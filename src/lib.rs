use std::process::{Command, Child, Stdio};
use std::path::Path;
use std::fs;
use std::io;
use std::env;
use std::collections::VecDeque;
use std::os::unix::fs::OpenOptionsExt;

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
            let exit_on_dup = false;
            if exit_on_dup {
                std::process::exit(1);
            }
        }

        fs::create_dir(format!("states/{}", state_path))
            .expect("[-] Could not create state dir!");

        fs::create_dir(format!("states/{}/in", state_path))
            .expect("[-] Could not create in dir!");

        fs::create_dir(format!("states/{}/out", state_path))
            .expect("[-] Could not create out dir!");

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

    fn snapshot_run(&self) -> io::Result<Child> {
        let cur_input = fs::File::open(format!("states/{}/out/.cur_input",
            self.state_path)).unwrap();
        let stdout = fs::File::create(format!("states/{}/stdout",
            self.state_path)).unwrap();
        let stderr = fs::File::create(format!("states/{}/stderr",
        self.state_path)).unwrap();

        env::set_current_dir(format!("./states/{}", self.state_path)).unwrap();

        let ret = Command::new("setsid")
            .args(&[
                format!("stdbuf"),
                format!("-oL"),
                format!("../../AFLplusplus/afl-qemu-trace"),
                format!("../../{}", self.target_bin),
            ])
            .stdin(Stdio::from(cur_input))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
            .env("CRIU_SNAPSHOT_DIR", format!("{}/snapshot/",
                std::env::current_dir().unwrap().display()))
            .spawn();

        env::set_current_dir(&Path::new("../../")).unwrap();

        ret
    }

    // fn consolidation(&self) {
    //     return
    // }


}
pub fn run() {
    let cur_timeout = 5;
    let afl: AFLRun = AFLRun::new("fitm-c0s0".to_string(),
        "test/forkserver_test".to_string(), cur_timeout.to_string());

    fs::write(format!("states/{}/in/1", afl.state_path), "init case.")
        .expect("[-] Could not create initial test case!");

    let mut afl_child = afl.snapshot_run().expect("Failed to execute initial afl");

    afl_child.wait().unwrap_or_else(|x| {
        println!("Error while waiting for snapshot run: {}", x);
        std::process::exit(1);
    });

    // Are there no immutable queue implementations in the standard library?
    let mut queue: VecDeque<&AFLRun> = VecDeque::new();
    queue.push_back(&afl);
    while !queue.is_empty() {
        // consolidate previous runs here

        // kick off new run
        match queue.pop_front() {
            Some(afl) => {
                let mut child = afl.fuzz_run().expect("Failed to start fuzz run");
                child.wait().unwrap_or_else(|x| {
                    println!("[!] Error while waiting for fuzz run: {}", x);
                    std::process::exit(1);
                });
            }
            None => {
                println!("[*] Queue is empty, no more jobs to be done.")
            }
        }
    }

    println!("[*] Reached end of programm. Quitting.");




}
