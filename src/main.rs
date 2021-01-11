use std::fs;
use std::path::Path;
use std::process;
use std::process::{Command, Stdio};
use std::{env, time::Duration};

fn main() {
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

    let idc = "AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES";
    let cpu = "AFL_SKIP_CPUFREQ";
    let debug = "AFL_DEBUG_CHILD_OUTPUT";
    // let debug = "AFL_QUIET";

    env::set_var(idc, "1");
    env::set_var(cpu, "1");
    env::set_var(debug, "1");

    if !Path::new("saved-states").exists() && fs::create_dir("saved-states").is_err() {
        println!("Could not create saved-states dir, aborting!");
        process::exit(0);
    }

    let criu_stdout = fs::File::create("criu_stdout").expect("[!] Could not create criu_stdout");
    let criu_stderr = fs::File::create("criu_stderr").expect("[!] Could not create criu_stderr");
    let foo = std::env::current_dir().unwrap();
    println!("cwd: {:?}", foo);
    let _criu_server = Command::new("/home/xcv/repos/FitM/criu/criu/criu")
        .args(&[
            format!("service"),
            format!("-v4"),
            format!("--address"),
            format!("/tmp/criu_service.socket"),
        ])
        .stdout(Stdio::from(criu_stdout))
        .stderr(Stdio::from(criu_stderr))
        .spawn()
        .expect("[!] Could not spawn criuserver");

    // TODO: use argv to fill these
    match fitm::run(
        ".",
        "./tests/targets/pseudoclient_simple",
        "./tests/targets/pseudoserver_simple",
        &Duration::from_secs(2),
    ) {
        Err(e) => println!("Error {:?}", e),
        _ => {}
    }
}
