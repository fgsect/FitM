use std::fs;
use std::path::Path;
use std::process;
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
    // let debug = "AFL_DEBUG_CHILD_OUTPUT";
    let debug = "AFL_QUIET";
    env::set_var(debug, "1");

    env::set_var(idc, "1");
    env::set_var(cpu, "1");

    if !Path::new("saved-states").exists() && fs::create_dir("saved-states").is_err() {
        println!("Could not create saved-states dir, aborting!");
        process::exit(0);
    }

    let foo = std::env::current_dir().unwrap();
    println!("cwd: {:?}", foo);

    // TODO: use argv to fill these
    // Paths are relative to ACTIVE_DIR
    match fitm::run(
        "../tests/targets/LightFTP/Source/Release/fftp",
        &["../tests/targets/LightFTP/fftp.conf"],
        &[],//("QEMU_STRACE", "1")],
        "/usr/bin/ftp",
        &["127.0.0.1", "2200"],
        &[("QEMU_STRACE", "1")],
        &Duration::from_secs(5 * 60),
        false,
    ) {
        Err(e) => println!("Error {:?}", e),
        _ => {}
    };
}
