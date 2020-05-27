#![allow(non_snake_case)]
use std::process;

fn main() {
    println!("Welcome to FitM!");

    if let Err(e) = fitm::run() {
        eprintln!("Application error: {}", e);

        process::exit(1);
    }
}
