use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::{env, time::Duration};
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize)]
struct RunArgs {
    /// The client target binary
    client: String,
    client_args: Vec<String>,
    client_envs: Vec<(String, String)>,
    /// The client target binary
    server: String,
    server_args: Vec<String>,
    server_envs: Vec<(String, String)>,
    /// run time in secs
    run_time: u64,
    // Still needs an echo binary or a binary producing a short output, as client
    // Just fuzzes the client for 100 millis.
    /// Enable protocol discovery (server_only)
    server_only: bool,
}

fn is_root() {
    match env::var("USER") {
        Ok(_) => {}
        Err(_) => {
            println!(
            "{} {} {}",
            "Please execute FitM as root as it is needed for criu.",
            "For reference please visit",
            "https://criu.org/Self_dump#Difficulties"
            );
            process::exit(1);
        }
    }
}

fn setup_env() {
    let idc = "AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES";
    let cpu = "AFL_SKIP_CPUFREQ";
    // let debug = "AFL_DEBUG_CHILD_OUTPUT";
    let debug = "AFL_QUIET";

    env::set_var(idc, "1");
    env::set_var(cpu, "1");
    env::set_var(debug, "1");
}

#[allow(dead_code)]
fn load_args(path: PathBuf) -> RunArgs {
    match fs::read_to_string(path) {
        Ok(args_json) => match serde_json::from_str(&args_json) {
            Ok(run_args) => run_args,
            Err(e) => panic!("[!] Error parsing fitm-args.json: {:?}", e)
        },
        Err(e) =>
            panic!("[!] Error reading fitm-args.json: {:?}", e)
    }
}

fn ensure_saved_states() {
    if !Path::new("saved-states").exists() && fs::create_dir("saved-states").is_err() {
        println!("Could not create saved-states dir, aborting!");
        process::exit(0);
    };
}

fn main() {
    is_root();

    setup_env();

    ensure_saved_states();

    println!("cwd: {:?}", std::env::current_dir().unwrap());

    // Need to change internal use of arrays to vecs for this to work. Disabled until changed.
    // let config_path: PathBuf = std::env::args().nth(1).expect("No config path given").into();
    // let _args = load_args(config_path);

    // TODO: use argv to fill these
    // Paths are relative to ACTIVE_DIR
    match fitm::run(
        "../tests/targets/live555/testProgs/testRTSPClient",
        &["rtsp://127.0.0.1:8554/wavAudioTest"],
        &[("QEMU_STRACE", "1")],
        "../tests/targets/live555/testProgs/testOnDemandRTSPServer",
        &[""],
        &[("QEMU_STRACE", "1"), ("INIT_RECV_SKIP", "1")],
        &Duration::from_secs(1),
        false,
    ) {
        Err(e) => println!("Error {:?}", e),
        _ => {}
    };
}
