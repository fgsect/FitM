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

    let idc = "AFL_I_DONT_CARE_ABOUT_MISSING_CRASHES";
    let cpu = "AFL_SKIP_CPUFREQ";

    env::set_var(idc, "1");
    env::set_var(cpu, "1");

    if !Path::new("states").exists() {
        match fs::create_dir("states"){
         Ok(_) => (),
         Err(_) => {
            println!("Could not create the states dir, aborting!");
            process::exit(0);
         }
        }
    }

    let path = env::current_dir().unwrap();
    println!("The current directory is {}", path.display());

    println!("Welcome to FitM!");

    if let Err(e) = fitm::run() {
        eprintln!("Application error: {}", e);

        process::exit(1);
    }
}
