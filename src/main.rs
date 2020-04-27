#![allow(non_snake_case)]

use std::{env, io, fs, process, thread};
use std::process::Command;

use std::io::Read;

use std::os::unix::net::UnixDatagram;
use std::os::unix::io::AsRawFd;

use std::path::Path;

fn pipe_name(fd: usize) -> std::path::PathBuf {
    match fs::read_link(format!("/proc/self/fd/{}", fd)) {
        Ok(v) => v,
        Err(_) => {
            println!("Couldn't open fd: {}", fd);
            process::exit(1);
        }
    }
}

fn main() {
    let is_root = match env::var("USER") {
        Ok(v) => v == "root",
        Err(_) => false,
    };

    if !is_root {
        println!(
            "{} {} {}",
            "Please execute FitM as root as it is needed for criu.", 
            "For reference please visit",
            "https://criu.org/Self_dump#Difficulties"
        );
        process::exit(1);
    }
   
	let socket = Path::new("test.sock");
	
	let mut stream = match UnixDatagram::bind(&socket) {
		Err(e) => panic!("Server is not running {}", e),
		Ok(s) => s,
	};

    let fd = stream.as_raw_fd();

	thread::spawn(move || {
		Command::new("criu")
				 .arg("swrk")
 				 .arg(&fd.to_string())
                 .spawn()
				 .unwrap();
	});

	let mut buffer = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();

    handle.read_to_string(&mut buffer);
}
