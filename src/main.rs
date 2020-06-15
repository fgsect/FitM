use std::process;
use std::env;
use std::path::Path;
use std::fs;

fn main() {
    match env::var("USER") {
        Ok(_) => {},
        Err(_) => {
            println!(
                "{} {} {}",
                "Please execute FitM as root as it is needed for criu.",
                "For reference please visit",
                "https://criu.org/Self_dump#Difficulties"
            );
            process::exit(1);
        },
    }

    let idc   = "AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES";
    let cpu   = "AFL_SKIP_CPUFREQ";
    let child = "AFL_DEBUG_CHILD_OUTPUT";

    env::set_var(idc,   "1");
    env::set_var(cpu,   "1");
    env::set_var(child, "1");

    if !Path::new("active-state").exists() && fs::create_dir("active-state").is_err() {
        println!("Could not create the states dir, aborting!");
        process::exit(0);
    }

    if !Path::new("saved-states").exists() && fs::create_dir("saved-states").is_err() {
        println!("Could not create saved-states dir, aborting!");
        process::exit(0);
    }

    fitm::run();
}
