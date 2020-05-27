use std::process::{Command, Output};
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

    fn run_q(&self, failure_msg: &str) -> Output{
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
}
pub fn run() -> Result<(), Box<dyn Error>>{

    let afl_run: AFLRun = AFLRun::new("/tmp/fitm-in".to_string(),
                "/tmp/fitm-out".to_string(),
                "none".to_string(),
                "/tmp/fitm-c0s0".to_string(),
                "~/repos/libxml2/xmllint".to_string(),
                "LETS_DO_THE_TIMEWARP_AGAIN".to_string());

    let output: Output = afl_run.run_q("failed to execute afl");

    let hello = output.stdout;
    match String::from_utf8(hello) {
        Ok(msg) =>     print!("{:?}", msg),
        Err(msg) => print!("Error while converting from bytes: {:?}", msg)
    }

    Ok(())
}