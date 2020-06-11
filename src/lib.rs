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

/// AFLRun contains all the information for one specific fuzz run.
struct AFLRun {
    /// Path to the base directory of the state of the current fuzz run
    state_path: String,
    /// Binary that is being fuzzed
    target_bin: String,
    /// Timeout for this run
    /// TODO: probably should be dynamic based on how interesting this state is.
    timeout: String
}

/// Implementation of functions for an afl run
impl AFLRun {
    /// Create a new afl run instance
    fn new(state_path: String, target_bin: String, timeout: String) -> AFLRun {
        // If the new state directory already exists we may have old data there
        // so we optionally delete it
        if Path::new(&format!("states/{}", state_path)).exists() {
            println!("[!] states/{} already exists! Recreating..", state_path);
            let delete = true;
            if delete {
                // expect already panics so we don't need to exit manually
                fs::remove_dir(format!("states/{}", state_path))
                    .expect("[-] Could not remove duplicate state dir!");
            }
        }

        // Create the new directories and files to make afl feel at home
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

        // Create a dummy .cur_input because the file has to exist once criu 
        // restores the process
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o600)
            .open(format!("states/{}/out/.cur_input", state_path))
            .unwrap();

        AFLRun{ state_path, target_bin, timeout }
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn fuzz_run(&self) -> io::Result<Child> {
        // Spawn the afl run in a command
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

    /// Wrapper for the snapshot run
    fn init_run(&self) -> io::Result<Child> {
        // create the .cur_input so that criu snapshots a fd connected to
        // .cur_input
        let cur_input = File::open(format!("states/{}/out/.cur_input",
            self.state_path)).unwrap();
        self.snapshot_run(cur_input)
    }

    /// Start the target binary for the first time and run until the first recv
    /// which will trigger the snapshot
    fn snapshot_run(&self, stdin: File) -> io::Result<Child> {
        // Open a file for stdout and stderr to log to
        let stdout = File::create(format!("states/{}/stdout", self.state_path))
            .unwrap();
        let stderr = File::create(format!("states/{}/stderr", self.state_path))
            .unwrap();

        // Change into our state directory and fuzz from there
        env::set_current_dir(format!("./states/{}", self.state_path)).unwrap();

        // Start the initial snapshot run. We use our patched qemu to emulate
        // until the first recv of the target is hit. We have to use setsid to
        // circumvent the --shell-job problem of criu and stdbuf to have the
        // correct stdin, stdout and stderr file descriptors.
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

        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        ret
    }
}

/// Create the next iteration from a given state directory. If inc_server is set
/// we will increment the state for the server from fitm-cXsY to fitm-cXsY+1.
/// Otherwise we will increment the state for the client from fitm-cXsY to
/// fitm-cX+1sY
fn next_state_path(state_path: String, inc_server: bool) -> String {
    // Create a static regex to find the current state directory
    // TODO: Is the lazy_static macro even necessary?
    lazy_static! {
        static ref REGEX: Regex = Regex::new(r#"fitm-c([0-9])+s([0-9])+"#)
            .unwrap();
    }
    let caps: regex::Captures = REGEX.captures(&state_path).unwrap();
    // 0 is the whole capture, then 1st group, 2nd group, ...
    let mut server_int: u32 = caps.get(2).unwrap().as_str().parse().unwrap();
    let mut client_int: u32 = caps.get(1).unwrap().as_str().parse().unwrap();

    // If inc_server increment the server state else increment the client state
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
            "test/forkserver_test".to_string(), 5.to_string()
        );

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
    // this does not terminate atm as consolidate_poc does not yet minimize
    // anything
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
