use std::process::{Command, Child, Stdio};
use std::path::Path;
use std::fs::File;
use std::fs;
use std::io;
use std::env;
use std::collections::VecDeque;
use std::os::unix::fs::OpenOptionsExt;
use rand::random;

use lazy_static::lazy_static;
use regex::Regex;

struct AFLRun {
    state_path: String,
    target_bin: String,
    timeout: String
}

impl AFLRun {
    fn new(state_path: String, target_bin: String, timeout: String) -> AFLRun {
        if Path::new(&format!("states/{}", state_path)).exists() {
            println!("[!] states/{} already exists! Recreating..", state_path);
            let delete = true;
            if delete {
                fs::remove_dir(format!("states/{}", state_path))
                    .expect("[-] Could not remove duplicate state dir!");
            }
            // expect already panics so we don't need to exit manually
        }

        fs::create_dir(format!("states/{}", state_path))
            .expect("[-] Could not create state dir!");

        fs::create_dir(format!("states/{}/in", state_path))
            .expect("[-] Could not create in dir!");

        fs::create_dir(format!("states/{}/out", state_path))
            .expect("[-] Could not create out dir!");

        fs::create_dir(format!("states/{}/fd", state_path))
            .expect("[-] Could not create fd dir!");

        fs::create_dir(format!("states/{}/snapshot", state_path))
            .expect("[-] Could not create snapshot dir!");

        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o600)
            .open(format!("states/{}/out/.cur_input", state_path))
            .unwrap();

        AFLRun{ state_path, target_bin, timeout }
    }

    fn fuzz_run(&self) -> io::Result<Child> {
        Command::new("AFLplusplus/afl-fuzz")
            .args(&[
                format!("-i"),
                format!("states/{}/in", self.state_path),
                format!("-o"),
                format!("states/{}/out", self.state_path),
                format!("-m"),
                format!("none"),
                format!("-d"),
                format!("-V"),
                format!("{}", self.timeout),
                format!("--"),
                format!("sh"),
                format!("restore.sh"),
                format!("states/{}/snapshot", self.state_path),
                format!("@@")
            ])
            .env("CRIU_SNAPSHOT_DIR", format!("{}/states/{}/snapshot/",
                std::env::current_dir().unwrap().display(), self.state_path))
            .env("AFL_SKIP_BIN_CHECK", "1")
            .spawn()
    }

    fn init_run(&self) -> io::Result<Child> {
        let cur_input = File::open(format!("states/{}/out/.cur_input",
            self.state_path)).unwrap();
        self.snapshot_run(cur_input)
    }

    // In consolidation mode we want to have rather
    // fine grained controller over the input of the run
    fn snapshot_run(&self, stdin: File) -> io::Result<Child> {
        let stdout = File::create(format!("states/{}/stdout", self.state_path))
            .unwrap();
        let stderr = File::create(format!("states/{}/stderr", self.state_path))
            .unwrap();

        env::set_current_dir(format!("./states/{}", self.state_path)).unwrap();

        let ret = Command::new("setsid")
            .args(&[
                format!("stdbuf"),
                format!("-oL"),
                format!("../../AFLplusplus/afl-qemu-trace"),
                format!("../../{}", self.target_bin),
            ])
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
            .env("CRIU_SNAPSHOT_DIR", format!("{}/snapshot/",
                std::env::current_dir().unwrap().display()))
            .spawn();

        env::set_current_dir(&Path::new("../../")).unwrap();

        ret
    }
}

// Take a string like: fitm-c0s0 and turn it into fitm-c1s0 or fitm-c0s1
fn next_state_path(state_path: String, inc_server: bool) -> String {
    lazy_static! {
        static ref REGEX: Regex = Regex::new(r#"fitm-c([0-9])+s([0-9])+"#)
            .unwrap();
    }
    let caps: regex::Captures = REGEX.captures(&state_path).unwrap();
    // 0 is the whole capture, then 1st group, 2nd group, ...
    let mut server_int: u32 = caps.get(2).unwrap().as_str().parse().unwrap();
    let mut client_int: u32 = caps.get(1).unwrap().as_str().parse().unwrap();

    if inc_server {
        server_int += 1;
    } else {
        client_int += 1;
    }

    format!("fitm-c{}s{}", client_int, server_int)
}

fn consolidate_poc(previous_run: &AFLRun) -> VecDeque<AFLRun> {
    let mut previous_state: String = previous_run.state_path.clone();
    let mut new_runs: VecDeque<AFLRun> = VecDeque::new();
    let queue_folder: String = format!("states/{}/out/queue", 
        &previous_run.state_path);

    for entry in fs::read_dir(queue_folder)
            .expect("[!] read_dir on previous_run.state_path failed") {
        let entry = entry
            .expect("[!] Could not read entry from previous_run.state_path");
        let path = entry.path();

        // skip dirs, only create a new run for each input file
        if path.is_dir() {
            continue
        }

        let afl = AFLRun::new(
        next_state_path(previous_state, true),
        "test/forkserver_test".to_string(), 5.to_string());

        previous_state = afl.state_path.clone();
        let seed_file_path = format!("states/{}/in/{}", afl.state_path,
            random::<u16>());
        fs::copy(path, &seed_file_path)
            .expect("[!] Could not copy to new afl.state_path");

        let seed_file = File::open(seed_file_path)
            .expect("[!] Could not create input file");

        let mut child = afl.snapshot_run(seed_file)
            .expect("Failed to start snapshot run");

        child.wait().expect("[!] Error while waiting for snapshot run");
        new_runs.push_back(afl);
    }

    new_runs
}

pub fn run() {
    let cur_timeout = 60;
    let afl: AFLRun = AFLRun::new("fitm-c0s0".to_string(),
        "test/forkserver_test".to_string(), cur_timeout.to_string());
    let mut queue: VecDeque<AFLRun> = VecDeque::new();

    fs::write(format!("states/{}/in/1", afl.state_path), "init case.")
        .expect("[-] Could not create initial test case!");

    let mut afl_child = afl.init_run().expect("Failed to execute initial afl");

    afl_child.wait().unwrap_or_else(|x| {
        println!("Error while waiting for snapshot run: {}", x);
        std::process::exit(1);
    });

    queue.push_back(afl);
    // this does not terminate atm as consolidate_poc does not yet minimize anything
    while !queue.is_empty() {
        // kick off new run
        let afl = queue.pop_front()
            .expect("[*] Queue is empty, no more jobs to be done");
        let mut child = afl.fuzz_run().expect("[!] Failed to start fuzz run");
        child.wait().expect("[!] Error while waiting for fuzz run");
        let _tmp = afl.state_path.clone();
        // consolidate previous runs here
        let mut new_runs: VecDeque<AFLRun> = consolidate_poc(&afl);
        queue.append(&mut new_runs);
    }

    println!("[*] Reached end of programm. Quitting.");
}
