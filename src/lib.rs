use std::process::{Command, Child, Stdio};
use std::path::Path;
use std::fs;
use std::io;

struct AFLRun {
    state_path: String,
    target_bin: String,
}

impl AFLRun {
    fn new(state_path: String, target_bin: String) -> AFLRun {
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

        fs::File::create(format!("states/{}/out/.cur_input", state_path))
            .expect("[-] Could not create cur_input file!");

        AFLRun{ state_path, target_bin }
    }

    fn fuzz_run(&self) -> io::Result<Child> {
        Command::new("AFLplusplus/afl-fuzz")
            .args(&[
                format!("-i"),
                format!("states/{}/in", self.state_path),
                format!("-o"),
                format!("states/{}/out", self.state_path),
                format!("-m none"),
                format!("-d"),
                format!("-r"),
                format!("states/{}/snapshot", self.state_path),
                format!("--"),
                format!("sh"),
                format!("../restore.sh"),
                format!("{}", self.state_path)
            ])
            .env("CRIU_SNAPSHOT_DIR", format!("{}/states/{}/snapshot/", 
                std::env::current_dir().unwrap().display(), self.state_path))
            .spawn()
    }

    fn init_run(&self) -> io::Result<Child> {
        fs::write(format!("states/{}/in/1", self.state_path), "init case.")
            .expect("[-] Could not create initial test case!");
        let cur_input = fs::File::open(format!("states/{}/out/.cur_input",
            self.state_path)).unwrap();
        let stdout = fs::File::create(format!("states/{}/stdout",
            self.state_path)).unwrap();
        let stderr = fs::File::create(format!("states/{}/stderr",
        self.state_path)).unwrap();
        Command::new("setsid")
            .args(&[
                format!("stdbuf"),
                format!("-oL"),
                format!("AFLplusplus/afl-qemu-trace"),
                format!("{}", self.target_bin),
            ])
            .stdin(Stdio::from(cur_input))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
            .spawn()
    }

    // fn consolidation(&self) {
    //     return
    // }
}
pub fn run() {

    let afl: AFLRun = AFLRun::new("fitm-c0s0".to_string(),
        "test/forkserver_test".to_string());

    let mut afl_child = afl.init_run().expect("Failed to execute initial afl");

    afl_child.wait().unwrap_or_else(|x| {
        println!("Error while waiting for init run: {}", x);
        std::process::exit(1);
    });

    fs::remove_file(format!("states/{}/out/.cur_input", afl.state_path))
        .expect("[-] Could not remove cur_input file!");

    afl_child = afl.fuzz_run().expect("Failed to start fuzz run");

    afl_child.wait().unwrap_or_else(|x| {
        println!("Error while waiting for fuzz run: {}", x);
        std::process::exit(1);
    });
}
