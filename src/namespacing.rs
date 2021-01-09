#![feature(asm)]
use std::{
    process::ExitStatus,
    io,
};
use libc::{self, pid_t};

// Structures to mimic the layout of the stdlib Child, 
// in order to correctly transmute between our replacemet and the original
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
    inner: AnonPipe
}

#[derive(Debug)]
struct AnonPipe(FileDesc);

#[derive(Debug)]
struct FileDesc {
    fd: libc::c_int,
}


pub fn run_in_namespace<T>(f: T) -> io::Result<i32>
where T: FnOnce() -> ! {
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
                handle: ProcessReplacement { pid: child_pid, status:None },
                stdin: None,
                stdout: None,
                stderr: None,
            };
        0
        },
        None => {
            // TODO: namespace_init();
            f()
        }
    })
}


#[repr(align(8))]
#[repr(C)]
struct clone_args {
    flags: u64,        /* Flags bit mask */
    pidfd: u64,        /* Where to store PID file descriptor (pid_t *) */
    child_tid: u64,    /* Where to store child TID:u64, in child's memory (pid_t *) */
    parent_tid: u64,   /* Where to store child TID, in parent's memory (int *) */
    exit_signal: u64,  /* Signal to deliver to parent on child termination */
    stack: u64,        /* Pointer to lowest byte of stack */
    stack_size: u64,   /* Size of stack */
    tls: u64,          /* Location of new TLS */
}

// DAS IST V2 aber mein kernel is zu alt ...
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
            *libc::__errno_location() = - x;
            Err(io::Error::last_os_error())
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;

    #[test]
    fn test_transmute_sizes() {
        println!("{}, {}", std::mem::size_of::<std::process::Child>(), std::mem::size_of::<ChildReplacement>());
    }

    #[test]
    fn test_transmute() {
        let foo = std::process::Command::new("/bin/sleep")
            .arg("5s")
            .stdin(Stdio::piped()).spawn().unwrap();
        let foo: ChildReplacement = unsafe { std::mem::transmute(foo) };

        println!("{:?}", foo);
    }
}