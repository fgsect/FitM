use std::process;
use std::env;

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

    println!("Welcome to FitM!");

    if let Err(e) = fitm::run() {
        eprintln!("Application error: {}", e);

        process::exit(1);
    }
}
