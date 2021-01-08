use libc::{self};
use std::{
    env,
    ffi::CString,
    fs::{File, OpenOptions},
    io::{Error, Read, Seek, SeekFrom, Write},
    process::{id, Command, Stdio},
    time::Duration,
};

use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;

fn system(command: &str) -> i32 {
    let command = CString::new(command).unwrap();
    unsafe { libc::system(command.as_ptr()) }
}

fn main() {
    println!("PID: {}", id());
    println!("SID: {}", unsafe { libc::getsid(id() as _) });
    println!("UID: {}", unsafe { libc::getuid() });

    if let Some(val) = env::args().nth(1) {
        if val == "spawn" {
            let unshare_result =
                unsafe { libc::unshare(libc::CLONE_NEWPID | libc::CLONE_NEWNS | libc::CLONE_FS) };
            println!("UNSHARE: {}", unshare_result);

            let foo = Command::new("./target/debug/Namespace-testing")
                .stdin(Stdio::null())
                //.stdout(Stdio::from(File::create("stdout").unwrap()))
                //.stderr(Stdio::from(File::create("stderr").unwrap()))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("failed to spawn");
                
            println!("CHILD PID: {}", foo.id());
            let out = foo.wait_with_output().unwrap();
            println!("====== STDOUT =======\n{}", String::from_utf8(out.stdout).unwrap());
            println!("====== STDERR =======\n{}", String::from_utf8(out.stderr).unwrap());
        }
    }

    if id() == 1 {
        // remount proc
        unsafe {
            // WIP assume mounts are successful
            // libc::mount(b"none\0".as_ptr() as _, b"/\0".as_ptr() as _, 0 as _, libc::MS_REC | libc::MS_PRIVATE, 0 as _);
            libc::mount(
                b"none\0".as_ptr() as _,
                b"/proc\0".as_ptr() as _,
                0 as _,
                libc::MS_REC | libc::MS_PRIVATE,
                0 as _,
            );
            libc::mount(
                b"proc\0".as_ptr() as _,
                b"/proc\0".as_ptr() as _,
                b"proc\0".as_ptr() as _,
                libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
                0 as _,
            );

            // Were the init-process -- HEREBY I PROCLAIM THAT I'M A SESSION LEADER
            libc::setsid();
        }

        println!("NEW SID: {}", unsafe { libc::getsid(id() as _) });
        system("ps -aux");
        let criu_srv = Command::new("../../criu/criu/criu")
            .arg("service")
            .arg("-v4")
            .arg("--address")
            .arg("/tmp/criu_service.socket")
            .spawn()
            .unwrap();

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .mode(0o644)
            .open("/proc/sys/kernel/ns_last_pid")
            .expect("Failed to open ns_last_pid");

        println!("locking");
        unsafe {
            if libc::flock(file.as_raw_fd() as _, libc::LOCK_EX) != 0 {
                panic!("LOCKING FAILED")
            }
        }

        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        println!("last pid: [{}]", contents.trim());

        file.seek(SeekFrom::Start(0)).unwrap();
        unsafe { libc::ftruncate(file.as_raw_fd(), 0) };
        file.write(1336.to_string().as_bytes()).unwrap();

        println!("FORKING");

        let new_pid = unsafe { libc::fork() };
        if new_pid == 0 {
            drop(file);
            // I bim jetzt mein eigener session-leader
            eprintln!("setsid result: {}", unsafe { libc::setsid() });

            println!("CHILD SPEAKING I have pid {}", id());
            //std::thread::sleep(Duration::from_secs(1));
            for _ in 0..3 {
                std::thread::sleep(Duration::from_secs(1));
            }
            eprintln!("Returned from sleep");

            let mut file = File::create("out").unwrap();
            file.write(b"testtest123");
            return;
        } else {
            println!("PARENT REPORTING TARGET PID: 1337 resulting: {}", new_pid)
        }

        std::thread::sleep(Duration::from_millis(100));

        system("mkdir dump");
        let command = format!(
            "criu dump -t {} --images-dir dump/ --leave-running",
            new_pid
        );
        system(&command);


        // [DBG] CHECK CHILD-FDs
        let command = format!(
            "ls /proc/{}/fd",
            new_pid
        );
        system(&command);


        unsafe {
            if libc::flock(file.as_raw_fd(), libc::LOCK_UN) != 0 {
                panic!("UNLOCKING FAILED")
            }
        }
        drop(file);

        // env::set_current_dir("active-state").unwrap();
        // let (stdout, stderr) = (
        //     File::create("stdout-afl").unwrap(),
        //     File::create("stderr-afl").unwrap(),
        // );

        // system("../afl-init.sh");
        // let mut child = Command::new("./restore.sh").spawn().unwrap();
        // let mut afl = Command::new("../../../AFLplusplus/afl-fuzz")
        // .args(&[
        //     format!("-i"),
        //     format!("./in"),
        //     format!("-o"),
        //     format!("./out"),
        //     // No mem limit
        //     format!("-m"),
        //     format!("none"),
        //     // Fuzzing as main node
        //     format!("-M"),
        //     format!("main"),
        //     format!("-d"),
        //     // At what time to stop this afl run
        //     format!("-V"),
        //     format!("{}", 30),
        //     // Timeout per individual execution
        //     format!("-t"),
        //     format!("{}", 1000),
        //     format!("--"),
        //     format!("bash"),
        //     // Our restore script
        //     format!("./restore.sh"),
        //     // The fuzzer input file
        //     format!("@@"),
        // ])
        // .stdout(Stdio::from(stdout))
        // .stderr(Stdio::from(stderr))
        // // In case we already started the fuzz run earlier, resume it here.
        // .env("AFL_AUTORESUME", "1")
        // .env("CRIU_SNAPSHOT_DIR", "./snapshot")
        // // We launch sh first, which is (hopefully) not instrumented
        // .env("AFL_SKIP_BIN_CHECK", "1")
        // .env("AFL_NO_UI", "1")
        // // Give criu forkserver up to a minute to spawn
        // .env("AFL_FORKSRV_INIT_TMOUT", "60000")
        // .spawn().expect("Failed to spawn child");

        for _i in 0..1 {
            println!("");
            system("ps -aux");
        }

        std::thread::sleep(Duration::from_secs(10));
        // child.wait().unwrap();
        // let exit_status = afl.wait().expect("waitpid failed");
        // println!("AFL-EXIT: {}", exit_status);
    }
}
