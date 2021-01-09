use libc::{self, pid_t};
use std::{
    ffi::CString,
    io,
    process::{Child, ExitStatus},
};

// Structures to mimic the layout of the stdlib Child,
// in order to correctly transmute between our structure and the original
// since we we can't construct Child by hand
// TESTED for rust-toolchain nightly-1.51 for x86_64-unknown-linux-gnu
#[derive(Debug)]
struct ChildReplacement {
    handle: ProcessReplacement,
    stdin: Option<ChildPipe>,
    stdout: Option<ChildPipe>,
    stderr: Option<ChildPipe>,
}

#[derive(Debug)]
struct ProcessReplacement {
    pid: pid_t,
    status: Option<ExitStatus>,
}

#[derive(Debug)]
struct ChildPipe {
    inner: AnonPipe,
}

#[derive(Debug)]
struct AnonPipe(FileDesc);

#[derive(Debug)]
#[rustc_layout_scalar_valid_range_start(0)]
// libstd/os/raw/mod.rs assures me that every libstd-supported platform has a
// 32-bit c_int. Below is -2, in two's complement, but that only works out
// because c_int is 32 bits.
#[rustc_layout_scalar_valid_range_end(0xFF_FF_FF_FE)]
struct FileDesc {
    fd: libc::c_int,
}

pub fn run_in_namespace<T>(f: T) -> io::Result<Child>
where
    T: FnOnce() -> !,
{
    // Clone-process with namespaceing flags
    let clone_result = unsafe {
        let args = clone_args {
            flags: (libc::CLONE_NEWPID | libc::CLONE_NEWNS) as _,
            pidfd: 0,
            parent_tid: 0,
            child_tid: 0,
            exit_signal: libc::SIGCHLD as _,
            stack: 0,
            stack_size: 0,
            tls: 0,
        };
        clone3(&args)?
    };

    Ok(match clone_result {
        Some(child_pid) => {
            let child = ChildReplacement {
                handle: ProcessReplacement {
                    pid: child_pid,
                    status: None,
                },
                stdin: None,
                stdout: None,
                stderr: None,
            };

            let child: Child = unsafe { std::mem::transmute(child) };
            child
        }
        None => {
            // TODO: namespace_init();
            f()
        }
    })
}

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

// Das ist V2 aber mein kernel is zu alt ...
#[repr(align(8))]
#[repr(C)]
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

unsafe fn clone3(args: &clone_args) -> io::Result<Option<i32>> {
    let ret: pid_t;
    println!("{}", std::mem::size_of::<clone_args>());
    asm!(
        "syscall",
        in("rax") libc::SYS_clone3,
        in("rdi") args as *const _,
        in("rsi") std::mem::size_of::<clone_args>(),
        out("rdx") _,
        out("rcx") _,
        out("r11") _,
        lateout("rax") ret,
    );

    match ret {
        0 => Ok(None),
        x if x > 0 => Ok(Some(x)),
        x => {
            *libc::__errno_location() = -x;
            Err(io::Error::last_os_error())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{id, Child, Command, Stdio};

    #[test]
    fn test_transmute_sizes() {
        println!(
            "{}, {}",
            std::mem::size_of::<Child>(),
            std::mem::size_of::<ChildReplacement>()
        );
    }

    #[test]
    fn test_transmute() {
        let foo = Command::new("/bin/sleep")
            .arg("5s")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let foo: ChildReplacement = unsafe { std::mem::transmute(foo) };

        println!("{:?}", foo);
    }

    fn system(command: &str) -> i32 {
        let command = CString::new(command).unwrap();
        unsafe { libc::system(command.as_ptr()) }
    }

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

    #[test]
    fn test_namespacing1() {
        run_in_namespace(|| {
            // mount("none","/", None, libc::MS_REC | libc::MS_PRIVATE, None); // Make / private (meaning changes wont propagate to the default namespace)
            mount("none", "/proc", None, libc::MS_REC | libc::MS_PRIVATE, None).expect("mounting proc private failed");
            mount(
                "proc",
                "/proc",
                Some("proc"),
                libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
                None,
            ).expect("proc remounting failed");

            println!("PID: {}", id());
            println!("SID: {}", unsafe { libc::getsid(id() as _) });
            println!("UID: {}", unsafe { libc::getuid() });

            system("ps -aux");
            std::process::exit(0);
        })
        .unwrap();
    }
}
