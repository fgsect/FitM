use std::process::{Command, Output, Child, ExitStatus};
use std::error::Error;

struct AFLRun {
    in_path: String,
    out_path: String,
    mem_limit: String,
    rst_path: String,
    server_bin: String,
    snapshot_env: String
}

impl AFLRun {
    fn new(in_path: String,
           out_path: String,
           mem_limit: String,
           rst_path: String,
           server_bin: String,
           snapshot_env: String) -> AFLRun{
        AFLRun{ in_path, out_path, mem_limit, rst_path, server_bin, snapshot_env }
    }

    fn run_restore(&self, failure_msg: &str) -> Output{
        Command::new("AFLplusplus/afl-fuzz")
            .args(&[format!("-i {}", self.in_path),
                format!("-o {}", self.out_path),
                format!(" -m {} ", self.mem_limit),
                format!("-d"),
                format!("-r {}", self.rst_path),
                format!("--"),
                format!("{}", self.server_bin),
                format!("@@")])
            .env(&self.snapshot_env, "")
            .output()
            .expect(failure_msg)
    }

    fn run_qemu(&self, failure_msg: &str) -> Child{
        Command::new("AFLplusplus/afl-fuzz")
            .args(&["-i", &self.in_path,
                "-o", &self.out_path,
                "-m", &self.mem_limit,
                "-d",
                "-Q",
                "--", &self.server_bin,
                "@@"])
            .env(&self.snapshot_env, "")
            .spawn().ok()
            .expect(failure_msg)
    }
}
pub fn run() -> Result<(), Box<dyn Error>>{

    let afl: AFLRun = AFLRun::new("fitm-in".to_string(),
                "fitm-out".to_string(),
                "none".to_string(),
                "fitm-c0s0".to_string(),
                "test/fsrv_test".to_string(),
                "LETS_DO_THE_TIMEWARP_AGAIN".to_string());

    let mut afl_child = afl.run_qemu("failed to execute afl");

    let the_status = afl_child.wait()
        .ok().expect("Couldn't wait for process.");
    // Output some exit information.
    // match the_status {
    //     ExitStatus(x) => println!("Exited with status {}", x),
    // };
    // tmp match {
    //     Ok(Child) => println!("Spawned AFL"),
    //     Err(Child) => println!("Error spawning AFL")
    // }

    // let hello = output.stdout;
    // print!("{}", String::from_utf8_lossy(&hello));

    Ok(())
}