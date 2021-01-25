use libc::{self, pid_t};
use std::process::ExitStatus;
use std::{
    ffi::CString,
    fmt::Debug,
    io::{self, Write},
};

fn mount(
    src: &str,
    target: &str,
    fstype: Option<&str>,
    flags: u64,
    data: Option<&str>,
) -> io::Result<()> {
    let src = CString::new(src)?;
    let target = CString::new(target)?;

    let fstype_buf;
    let fstype_ptr = match fstype {
        Some(val) => {
            fstype_buf = CString::new(val)?;
            fstype_buf.as_ptr()
        }
        None => 0 as _,
    };

    let data_buf;
    let data_ptr = match data {
        Some(val) => {
            data_buf = CString::new(val)?;
            data_buf.as_ptr()
        }
        None => 0 as _,
    };

    if -1
        == unsafe {
            libc::mount(
                src.as_ptr(),
                target.as_ptr(),
                fstype_ptr,
                flags,
                data_ptr as _,
            )
        }
    {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

unsafe fn sys_clone(flags: libc::c_int) -> io::Result<Option<libc::pid_t>> {
    let ret: pid_t = libc::syscall(libc::SYS_clone, flags as libc::c_int, 0, 0, 0, 0) as _;

    match ret {
        0 => Ok(None),
        x if x > 0 => Ok(Some(x)),
        x => {
            *libc::__errno_location() = -x;
            Err(io::Error::last_os_error())
        }
    }
}

pub struct NamespaceContext {
    pub init_fn: Box<dyn FnOnce()>,
}

impl NamespaceContext {
    pub fn new() -> Self {
        NamespaceContext {
            init_fn: Box::new(|| {
                // mount("none","/", None, libc::MS_REC | libc::MS_PRIVATE, None); // Make / private (meaning changes wont propagate to the default namespace)
                mount("none", "/proc", None, libc::MS_REC | libc::MS_PRIVATE, None)
                    .expect("mounting proc private failed");
                mount(
                    "proc",
                    "/proc",
                    Some("proc"),
                    libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
                    None,
                )
                .expect("proc remounting failed");

                unsafe { libc::setsid() };

                let _ = io::stdout().flush();
                // std::process::Command::new("stat").arg("/proc/self/ns/pid").status().unwrap();
                // std::process::Command::new("ps").arg("-aux").status().unwrap();
            }),
        }
    }

    pub fn execute<T, E>(self, f: T) -> io::Result<Namespace>
    where
        T: FnOnce() -> Result<i32, E>,
        E: Debug,
    {
        // Clone-process with namespaceing flags
        // let clone_result = unsafe {
        //     let args = clone_args {
        //         flags: (libc::CLONE_NEWPID | libc::CLONE_NEWNS) as _,
        //         pidfd: 0,
        //         parent_tid: 0,
        //         child_tid: 0,
        //         exit_signal: libc::SIGCHLD as _,
        //         stack: 0,
        //         stack_size: 0,
        //         tls: 0,
        //     };
        //     clone3(&args)?
        // };

        let clone_result =
            unsafe { sys_clone(libc::CLONE_NEWPID | libc::CLONE_NEWNS | libc::SIGCHLD)? };

        Ok(match clone_result {
            Some(child_pid) => Namespace {
                init_pid: child_pid,
                status: None,
            },
            None => {
                (self.init_fn)();
                let res = f();
                std::thread::sleep(std::time::Duration::from_millis(5));
                std::process::Command::new("sync")
                    .arg("-f")
                    .status()
                    .unwrap();
                let _ = io::stdout().flush();
                match res {
                    Ok(val) => std::process::exit(val),
                    Err(e) => panic!("Namespace call failed with error {:?}", e),
                }
            }
        })
    }
}

pub struct Namespace {
    pub init_pid: pid_t,
    pub status: Option<ExitStatus>,
}

impl Namespace {
    pub fn wait(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.status {
            return Ok(status);
        }

        let mut status = 0 as libc::c_int;
        loop {
            let result = unsafe { libc::waitpid(self.init_pid, &mut status, 0) };
            if result == -1 {
                let e = io::Error::last_os_error();
                if e.kind() != io::ErrorKind::Interrupted {
                    return Err(e);
                }
            } else {
                break;
            }
        }
        // Casting the waitpid return value to automatically interpret ExistStatus flags
        let exit_status: ExitStatus = unsafe { std::mem::transmute(status) };
        self.status = Some(exit_status);
        Ok(exit_status)
    }
}

/*
#[repr(align(8))]
#[repr(C)]
struct clone_args {
    flags: u64,       /* Flags bit mask */
    pidfd: u64,       /* Where to store PID file descriptor (pid_t *) */
    child_tid: u64,   /* Where to store child TID:u64, in child's memory (pid_t *) */
    parent_tid: u64,  /* Where to store child TID, in parent's memory (int *) */
    exit_signal: u64, /* Signal to deliver to parent on child termination */
    stack: u64,       /* Pointer to lowest byte of stack */
    stack_size: u64,  /* Size of stack */
    tls: u64,         /* Location of new TLS */
}
*/

// Das ist V2 aber mein kernel is zu alt ...
#[repr(align(8))]
#[repr(C)]
#[allow(dead_code)]
struct clone_argsV2 {
    flags: u64,        /* Flags bit mask */
    pidfd: u64,        /* Where to store PID file descriptor (pid_t *) */
    child_tid: u64,    /* Where to store child TID:u64, in child's memory (pid_t *) */
    parent_tid: u64,   /* Where to store child TID, in parent's memory (int *) */
    exit_signal: u64,  /* Signal to deliver to parent on child termination */
    stack: u64,        /* Pointer to lowest byte of stack */
    stack_size: u64,   /* Size of stack */
    tls: u64,          /* Location of new TLS */
    set_tid: u64,      /* Pointer to a pid_t array */
    set_tid_size: u64, /* Number of elements in set_tid */
}

/*
unsafe fn clone3(args: &clone_args) -> io::Result<Option<i32>> {
    let ret: pid_t = libc::syscall(
        libc::SYS_clone3,
        args as *const _,
        std::mem::size_of::<clone_args>(),
    ) as _;
    // asm!(
    //     "syscall",
    //     in("rax") libc::SYS_clone3,
    //     in("rdi") args as *const _,
    //     in("rsi") std::mem::size_of::<clone_args>(),
    //     out("rdx") _,
    //     out("rcx") _,
    //     out("r11") _,
    //     lateout("rax") ret,
    // );

    match ret {
        0 => Ok(None),
        x if x > 0 => Ok(Some(x)),
        x => {
            *libc::__errno_location() = -x;
            Err(io::Error::last_os_error())
        }
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::os::unix::{fs::OpenOptionsExt, io::AsRawFd};
    use std::path::Path;
    use std::process::{id, Command, Stdio};

    use crate::FITMSnapshot;

    fn system(command: &str) -> i32 {
        let command = CString::new(command).unwrap();
        unsafe { libc::system(command.as_ptr()) }
    }

    #[test]
    fn test_namespacing1() {
        let mut child = NamespaceContext::new()
            .execute(|| -> io::Result<i32> {
                println!("PID: {}", id());
                println!("SID: {}", unsafe { libc::getsid(id() as _) });
                println!("UID: {}", unsafe { libc::getuid() });

                // TODO spawn this process with a specific PID
                Command::new("setsid")
                    .arg("bash")
                    .arg("-c")
                    .arg("sleep 5s; echo 'hi'")
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;

                system("ps -aux");
                system("criu dump -t 2 -v -o dump.log --images-dir dump/ --leave-running");
                system("chown 1000:1000 dump/dump.log");

                std::thread::sleep(std::time::Duration::from_secs(10));
                Ok(10)
            })
            .unwrap();

        let ret = child.wait().unwrap();
        println!("{:?}", ret);
    }

    #[test]
    fn test_snapshot_init() {
        NamespaceContext::new()
            .execute(|| -> io::Result<i32> {
                println!("PID: {}", id());
                println!("SID: {}", unsafe { libc::getsid(id() as _) });
                println!("UID: {}", unsafe { libc::getuid() });

                let _criu_srv = Command::new("criu")
                    .args(&[
                        "service",
                        "-v4",
                        "--address",
                        "/tmp/criu_service.socket",
                        "-o",
                        "dump.log",
                        "-vv",
                    ])
                    .spawn()?;

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

                let (stdout, stderr) = (File::create("stdout-afl")?, File::create("stderr-afl")?);

                let stdin_path = "/dev/null";
                let stdin_file = File::open(stdin_path)?;

                file.seek(SeekFrom::Start(0)).unwrap();
                unsafe { libc::ftruncate(file.as_raw_fd(), 0) };
                file.write(311336.to_string().as_bytes())?;

                let mut child = Command::new("setsid")
                    .args(&[
                        format!("stdbuf"),
                        format!("-oL"),
                        format!("./fitm-qemu-trace"),
                        format!("{}", "tests/targets/pseudoserver_simple"),
                        format!("{}", "/dev/null"),
                    ])
                    .stdin(Stdio::from(stdin_file))
                    .stdout(Stdio::from(stdout))
                    .stderr(Stdio::from(stderr))
                    .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
                    .env("FITM_CREATE_OUTPUTS", "1")
                    .env("CRIU_SNAPSHOT_DIR", "./snapshot")
                    .env("AFL_NO_UI", "1")
                    .spawn()
                    .expect("[!] Could not spawn snapshot run");

                println!("CHILD PID: {}", child.id());
                unsafe {
                    if libc::flock(file.as_raw_fd(), libc::LOCK_UN) != 0 {
                        panic!("UNLOCKING FAILED")
                    }
                }
                drop(file);

                child
                    .wait()
                    .expect("[!] Could not wait for child to finish");
                std::process::exit(1);
            })
            .unwrap()
            .wait()
            .unwrap();
    }

    #[test]
    fn test_snapshot_init2() {
        if !Path::new("saved-states").exists() && std::fs::create_dir("saved-states").is_err() {
            println!("Could not create saved-states dir, aborting!");
            std::process::exit(0);
        }

        let afl_server_snap: FITMSnapshot = FITMSnapshot::new(
            1,
            0,
            "tests/targets/pseudoserver_simple".to_string(),
            std::time::Duration::from_secs(5),
            "".to_string(),
            true,
            false,
            None,
        );

        afl_server_snap.init_run(false, true, &[""]).unwrap();

        std::fs::write("./saved-states/fitm-gen1-state0/in/testinp", "ulullulul")
            .expect("failed to create test-input");

        afl_server_snap
            .create_outputs(
                "./saved-states/fitm-gen1-state0/in",
                "./saved-states/fitm-gen1-state0/outputs",
            )
            .unwrap();
    }
}
