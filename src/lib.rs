use std::fs::{self, DirEntry, File};
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fmt};

use crate::namespacing::NamespaceContext;
use crate::utils::{advance_pid, cp_recursive, get_filesize, spawn_criu};
use regex::Regex;
use std::fs::remove_dir_all;
use std::thread::sleep;
use std::time::Duration;

pub mod namespacing;
pub mod utils;
// client_set: set of afl-showmap on client outputs that are relevant for us
// server_set: set of afl-showmap on server outputs that are relevant for us

pub const ORIGIN_STATE_CLIENT: &str = "fitm-gen2-state0";
pub const ORIGIN_STATE_SERVER: &str = "fitm-gen1-state0";
pub const ACTIVE_STATE: &str = "active-state";
pub const SAVED_STATES: &str = "saved-states";

pub const CRIU_STDOUT: &str = "criu_stdout";
pub const CRIU_STDERR: &str = "criu_stderr";

/// FITMSnapshot contains all the information for one specific snapshot and fuzz run.
#[derive(Clone)]
pub struct FITMSnapshot {
    /// The fitm generation (starting with 0 for the initial client)
    pub generation: u32,
    /// The state id, unique in one generation
    pub state_id: usize,
    /// Path to the base directory of the state of the current fuzz run
    pub state_path: String,
    /// Binary that is being fuzzed
    pub target_bin: String,
    /// Timeout for this run
    /// TODO: probably should be dynamic based on how interesting this state
    /// is.
    pub timeout: Duration,
    // All the states that came out of the current state
    // child_states: Vec<(u32, u32)>
    /// Used to determine whether to increase first or second value of state
    /// tuple. Hope this is not too broken
    pub server: bool,
    /// State folder name of the state from which this object's snapshot was
    /// created Empty if created from binary
    pub base_state: String,
    /// Marks if this run is an initial state or not
    pub initial: bool,
    /// Name of the corresponding acitve dir
    pub origin_state: &'static str,
    /// Pid of the snapshotted process
    pub pid: Option<i32>,
}

impl fmt::Debug for FITMSnapshot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FITMSnapshot")
            .field("state_path", &self.state_path)
            .field("base_state", &self.base_state)
            .field("target_bin", &self.target_bin)
            .field("timeout", &self.timeout)
            .field("server", &self.server)
            .field("initial", &self.initial)
            .field("origin_state", &self.origin_state)
            .finish()
    }
}

/// Returns the origin state for client or server
pub fn origin_state(is_server: bool) -> &'static str {
    if is_server {
        ORIGIN_STATE_SERVER
    } else {
        ORIGIN_STATE_CLIENT
    }
}

fn state_path_for(gen: u32, state_id: usize) -> String {
    format!("fitm-gen{}-state{}", gen, state_id)
}

/// Implementation of functions for an afl run
/// Createing a new FITMSnapshot will create the necessary directory in active-state
impl FITMSnapshot {
    /// Create a new afl run instance
    pub fn new(
        generation: u32,
        state_id: usize,
        target_bin: String,
        timeout: Duration,
        base_state: String,
        server: bool,
        from_snapshot: bool,
        pid: Option<i32>,
    ) -> FITMSnapshot {
        let origin_state = origin_state(server);

        let state_path = state_path_for(generation, state_id);

        // Make sure there is no old active_state folder
        match std::fs::remove_dir_all(ACTIVE_STATE) {
            Ok(_) => (),
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            Err(e) => println!("[!] Error while removing {}: {:?}", ACTIVE_STATE, e),
        };

        // Create the new directories and files to make afl feel at home
        fs::create_dir(ACTIVE_STATE).expect("[-] Could not create state dir!");

        fs::create_dir(format!("{}/in", ACTIVE_STATE)).expect("[-] Could not create in dir!");

        fs::create_dir(format!("{}/out", ACTIVE_STATE)).expect("[-] Could not create out dir!");

        fs::create_dir(format!("{}/outputs", ACTIVE_STATE))
            .expect("[-] Could not create outputs dir!");

        fs::create_dir(format!("{}/out/maps", ACTIVE_STATE))
            .expect("[-] Could not create out/maps dir!");

        let fd_path = format!("{}/fd", ACTIVE_STATE);
        fs::create_dir(fd_path.clone()).expect("[-] Could not create fd dir!");

        if from_snapshot {
            // Grab old snapshot from which we want to create a new one here

            if base_state != "".to_string() {
                utils::copy_snapshot_base(&base_state);
            }
        };

        let new_run = FITMSnapshot {
            generation,
            state_id,
            state_path,
            target_bin,
            timeout,
            server,
            base_state,
            initial: false,
            origin_state: origin_state,
            pid,
        };

        // We can write a tool in the future to parse this info
        // and print a visualization of the state order
        let path = format!("{}/run-info", ACTIVE_STATE);
        let mut file = fs::File::create(path).expect("[!] Could not create FITMSnapshot file");
        file.write(format!("{:?}", new_run).as_bytes())
            .expect("[!] Could not write to FITMSnapshot file");

        new_run
    }

    /// Copies everything in ./fd to ./outputs/ of a specified state path.
    /// this is used on the initial client state to generate intitial inputs for the first server run
    fn copy_fds_to_output_for(&self, gen: u32, state: usize) {
        let state_path = state_path_for(gen, state);
        // Make sure state dir outputs exists
        let _ = fs::create_dir_all(&format!("./saved-states/{}/outputs", state_path));
        for (i, entry) in fs::read_dir(&format!("./{}/fd", ACTIVE_STATE))
            .expect("[!] Could not find fd folder in copy_fds_to_output_for")
            .enumerate()
        {
            let path = entry
                .expect("[!] Could not find entry in fd folder in copy_fds_to_output_for")
                .path();
            if path.is_file() {
                let to = &format!("./saved-states/{}/outputs/initial{}", &state_path, i);
                std::fs::copy(&path, &to).expect(
                    format!(
                        "[!] Could not copy {:?} to {} in copy_fds_to_output_for",
                        path,
                        to.as_str()
                    )
                    .as_str(),
                );
            }
        }
    }

    /// Copies everything in ./fd to ./outputs/
    /// this is used on the initial client state to generate intitial inputs for the first server run
    fn copy_queue_to(&self, dst: &Path, active: bool) -> Result<(), io::Error> {
        fs::create_dir_all(dst)?;
        // used string as type because format!().as_str offended the borrow checker,
        // even if the string is saved in a tmp variable first
        let from = if active {
            "active-state/out/main/queue".to_string()
        } else {
            format!("saved-states/{}/out/main/queue", self.state_path)
        };
        for (_, entry) in fs::read_dir(from)?.enumerate() {
            let path = entry?.path();
            let name = path.file_name().unwrap();
            if path.is_file() {
                std::fs::copy(&path, dst.join(&name))?;
            }
        }
        Ok(())
    }

    fn save_fuzz_results(&self) -> Result<(), io::Error> {
        let postrun = "out_postrun";
        let out = format!("{}/out", ACTIVE_STATE);
        let out_postrun = format!("{}/{}", ACTIVE_STATE, postrun);

        // cp will copy out into out_postrun on the second and third copy because the destination already exists
        // thus we need src and dst to be the same name
        if PathBuf::from(out_postrun.as_str()).is_dir() {
            remove_dir_all(out_postrun.as_str())
                .expect("[!] Error while removing out_postrun in save_fuzz_results");
        }
        cp_recursive(out.as_str(), out_postrun.as_str());

        // Don't copy INTO out_postrun, if you do the folders won't get merged by cp
        let to = format!("./saved-states/{}", self.state_path);
        cp_recursive(out_postrun.as_str(), to.as_str());
        Ok(())
    }

    /// Needed for the two initial snapshots created based on the target
    /// binaries
    pub fn init_run(
        &self,
        create_outputs: bool,
        create_snapshot: bool,
        cli_args: &[&str],
    ) -> Result<Option<i32>, io::Error> {
        ensure_dir_exists(ACTIVE_STATE);

        // Start the initial snapshot run. We use our patched qemu to emulate
        // until the first recv of the target is hit. We have to use setsid to
        // circumvent the --shell-job problem of criu and stdbuf to have the
        // correct stdin, stdout and stderr file descriptors.
        let closure_exit = NamespaceContext::new()
            .execute(|| -> io::Result<i32> {
                spawn_criu("./criu/criu/criu", "/tmp/criu_service.socket")
                    .expect("[!] Could not spawn criuserver");

                // Change into our state directory and generate the afl maps there
                env::set_current_dir(ACTIVE_STATE)
                    .expect("[!] Could not change into active_state during init_run");

                let snapshot_dir = format!("{}/snapshot", env::current_dir().unwrap().display());
                fs::create_dir(&snapshot_dir).expect("[-] Could not create snapshot dir!");

                // Force the target PID to be in the Order of ~16k (high, but not hither than a normal pid_max)
                advance_pid(1 << 14);

                // Open a file for stdout and stderr to log to
                let (stdout, stderr) = (fs::File::create("stdout")?, fs::File::create("stderr")?);

                // create the .cur_input so that criu snapshots a fd connected to
                // .cur_input
                let dev_null = "/dev/null";
                let stdin = fs::File::open(dev_null).unwrap();

                let mut command = Command::new("setsid");
                command
                    .args(&["stdbuf", "-oL", "../fitm-qemu-trace", &self.target_bin])
                    .args(cli_args)
                    .stdin(Stdio::from(stdin))
                    .stdout(Stdio::from(stdout))
                    .stderr(Stdio::from(stderr))
                    // .env("CRIU_SNAPSHOT_DIR", &snapshot_dir)
                    .env("CRIU_SNAPSHOT_OUT_DIR", &snapshot_dir)
                    //.env("QEMU_STRACE", "1")
                    .env("AFL_NO_UI", "1");

                if create_outputs {
                    command.env("FITM_CREATE_OUTPUTS", "1");
                }

                if create_snapshot {
                    command.env("LETS_DO_THE_TIMEWARP_AGAIN", "1");
                }

                let exit_status = command
                    .spawn()
                    .expect("[!] Could not spawn snapshot run")
                    .wait()
                    .expect("[!] Snapshot run failed");

                Ok(exit_status.code().unwrap())
            })
            .expect("[!] Namespace creation failed")
            .wait()
            .expect("[!] Namespace wait failed")
            .code()
            .unwrap();

        let mut pid = None;
        if create_snapshot {
            if closure_exit == 42 {
                // With snapshot_run we move the state folder instead of copying it,
                // but in this initial case we need to use
                // the state folder shortly after running this function
                pid = Some(utils::parse_pid().unwrap());
                utils::mv_rename(ACTIVE_STATE, &format!("./saved-states/{}", self.state_path));
            } else {
                panic!(
                    "[!] Snapshot in init_run failed. Check latest active-state folder for clues."
                );
            }
        }

        if create_outputs {
            self.copy_fds_to_output_for(0, 0);

            remove_dir_all(ACTIVE_STATE)
                .expect("[!] Could not remove active_state during init_run");
        }

        Ok(pid)
    }

    /// Create a new snapshot based on a given snapshot
    /// @return: boolean indicating whether a new snapshot was create or not (true == new snapshot created)
    pub fn snapshot_run(&self, stdin_path: &str) -> Result<bool, io::Error> {
        println!(
            "==== [*] Running snapshot run for input: \"{}\" ====",
            stdin_path
        );
        let _ = io::stdout().flush();

        let exit_code = NamespaceContext::new()
            .execute(|| -> io::Result<i32> {
                spawn_criu("./criu/criu/criu", "/tmp/criu_service.socket")
                    .expect("[!] Could not spawn criuserver");

                let (stdout, stderr) = self.create_environment()?;
                let stdin_file = fs::File::open(stdin_path).unwrap();
                let snapshot_dir = format!("{}/snapshot", env::current_dir().unwrap().display());

                let next_snapshot_dir =
                    format!("{}/next_snapshot", env::current_dir().unwrap().display());
                fs::create_dir(&next_snapshot_dir).expect("[-] Could not create snapshot dir!");

                let _restore = Command::new("setsid")
                    .args(&[
                        format!("stdbuf"),
                        format!("-oL"),
                        format!("./restore.sh"),
                        stdin_path.to_string(),
                    ])
                    .stdin(Stdio::from(stdin_file))
                    .stdout(Stdio::from(stdout))
                    .stderr(Stdio::from(stderr))
                    .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
                    .env("CRIU_SNAPSHOT_DIR", &snapshot_dir)
                    .env("CRIU_SNAPSHOT_OUT_DIR", &next_snapshot_dir)
                    .env("AFL_NO_UI", "1")
                    .spawn()
                    .expect("[!] Could not spawn snapshot run")
                    .wait()
                    .expect("[!] Snapshot restore failed");

                let exit_status =
                    utils::waitpid(self.pid.unwrap()).expect("[!] Snapshot run failed");
                Ok(exit_status.code().unwrap())
            })
            .expect("[!] Namespace creation failed")
            .wait()
            .expect("[!] Namespace wait failed")
            .code()
            .unwrap();

        let _next_snapshot_path = format!(
            "{}/{}/next_snapshot",
            env::current_dir().unwrap().display(),
            ACTIVE_STATE
        );

        let success = exit_code == 42;
        if success {
            fs::remove_dir_all(&format!("./{}/snapshot", ACTIVE_STATE))
                .expect("Failed to remove old snapshot");
            fs::rename(
                &format!("./{}/next_snapshot", ACTIVE_STATE),
                &format!("./{}/snapshot", ACTIVE_STATE),
            )
            .expect("Failed to move folder");
            fs::create_dir(&format!("./{}/next_snapshot", ACTIVE_STATE))
                .expect("Failed to reinitialize ./next_snapshot");
            utils::mv_rename(ACTIVE_STATE, &format!("./saved-states/{}", self.state_path));
        }

        Ok(success)
    }

    fn found_crashes(&self) -> bool {
        let iter: Vec<DirEntry> = fs::read_dir(format!("{}/out/main/crashes", ACTIVE_STATE))
            .expect("[!]")
            .map(|entry| entry.unwrap())
            .collect();
        if iter.len() > 0 {
            true
        } else {
            false
        }
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn fuzz_run(&self, run_duration: &Duration) -> Result<(), io::Error> {
        // If not currently needed, all states should reside in `saved-state`.
        // Thus they need to be copied to be fuzzed
        // stdout is mutable so it can be read later
        let exit_status = NamespaceContext::new()
            .execute(|| -> io::Result<i32> {
                let (stdout, stderr) = self.to_active()?;
                println!(
                    "==== [*] Start fuzzing {} ({:?}) ====",
                    self.state_path,
                    PathBuf::from(&self.target_bin).file_name().unwrap()
                );
                // Spawn the afl run in a command. This run is relative to the state dir
                // meaning we already are inside the directory. This prevents us from
                // accidentally using different resources than we expect.

                let exit_status = Command::new("../AFLplusplus/afl-fuzz")
                    .args(&[
                        format!("-i"),
                        format!("./in"),
                        format!("-o"),
                        format!("./out"),
                        // No mem limit
                        format!("-m"),
                        format!("none"),
                        // Fuzzing as main node
                        format!("-M"),
                        format!("main"),
                        format!("-d"),
                        // At what time to stop this afl run
                        format!("-V"),
                        format!("{}", run_duration.as_secs()),
                        // Timeout per individual execution
                        format!("-t"),
                        format!("{}", self.timeout.as_millis()),
                        format!("--"),
                        format!("bash"),
                        // Our restore script
                        format!("./restore.sh"),
                        // The fuzzer input file
                        format!("@@"),
                    ])
                    .stdout(Stdio::from(stdout))
                    .stderr(Stdio::from(stderr))
                    // In case we already started the fuzz run earlier, resume it here.
                    .env("AFL_AUTORESUME", "1")
                    .env("CRIU_SNAPSHOT_DIR", "./snapshot")
                    // We launch sh first, which is (hopefully) not instrumented
                    .env("AFL_SKIP_BIN_CHECK", "1")
                    .env("AFL_NO_UI", "1")
                    // Give criu forkserver up to a minute to spawn
                    .env("AFL_FORKSRV_INIT_TMOUT", "60000")
                    .env("FITM_CREATE_OUTPUTS", "1")
                    .env("AFL_COMPCOV_LEVEL", "2")
                    .spawn()?
                    .wait()?;

                Ok(exit_status.code().unwrap())
            })
            .expect("[!] Namespace creation failed")
            .wait()
            .expect("[!] Namespace wait failed")
            .code()
            .unwrap();

        if self.state_path == "fitm-gen4-state0" {
            sleep(Duration::from_millis(0));
        }

        if exit_status != 0 {
            let info =
                "[!] Error during afl-fuzz execution. Please check latest statefolder for output";
            println!("{}", info);
            std::process::exit(1);
        }
        println!("==== [*] Finished fuzzing {} ====", self.state_path);

        if self.found_crashes() {
            println!(
                "==== [*] Crashes present after fuzzing {} ====",
                self.state_path
            );
        }

        // Doesn't work since File has no copy trait and Stdio:from doesn't take ref :(
        // let mut stdout_content = String::new();
        // stdout.read_to_string(&mut stdout_content).unwrap();
        // println!("==== [*] AFL++ stdout: \n{}", stdout_content);
        self.save_fuzz_results()?;

        Ok(())
    }

    pub fn create_outputs_file(
        &self,
        entry_path: PathBuf,
        output_path: &str,
    ) -> Result<(), io::Error> {
        let exit_status = NamespaceContext::new()
            .execute(|| -> io::Result<i32> {
                let (stdout, stderr) = self.to_active()?;

                let entry_file =
                    fs::File::open(entry_path.clone()).expect("[!] Could not open queue file");
                println!("==== [*] Using input: {:?} ====", entry_path);

                let _restore_status = Command::new("setsid")
                    .args(&[
                        format!("stdbuf"),
                        format!("-oL"),
                        format!("bash"),
                        format!("./restore.sh"),
                        String::from(entry_path.clone().to_str().unwrap()),
                    ])
                    .stdin(Stdio::from(entry_file))
                    .stdout(Stdio::from(stdout.try_clone().unwrap()))
                    .stderr(Stdio::from(stderr.try_clone().unwrap()))
                    .env("FITM_CREATE_OUTPUTS", "1")
                    .env("AFL_NO_UI", "1")
                    .spawn()
                    .expect("[!] Could not spawn snapshot run")
                    .wait()
                    .expect("[!] Snapshot restore failed");

                let exit_status =
                    utils::waitpid(self.pid.unwrap()).expect("[!] Snapshot run failed");
                Ok(exit_status.code().unwrap())
            })
            .expect("[!] Namespace creation failed")
            .wait()
            .expect("[!] Namespace wait failed")
            .code()
            .unwrap();

        if exit_status != 0 {
            let info =
                "[!] Error during create_outputs execution. Please check latest statefolder for output";
            println!("{}", info);
            std::process::exit(1);
        }

        env::set_current_dir(ACTIVE_STATE)?;
        // Move created outputs to a given folder
        // Probably saved states, as current active-state folder will be deleted with next to_active()
        for entry in fs::read_dir("./fd").expect("[!] Could not read populated fd folder") {
            let dir_entry = entry.unwrap();
            let file_name = &dir_entry.file_name();

            // skip empty outputs --> easier debugging
            if get_filesize(&dir_entry.path()) == 0 {
                continue;
            }

            let from = format!("./fd/{}", &file_name.to_str().unwrap());
            let destination_path = Path::new(output_path).join(file_name);
            let to = destination_path
                .to_str()
                .expect("[!] Couldn't convert destination_path to str");
            // append index to not overwrite fd-files
            let to = format!(
                "{}-{}",
                to,
                entry_path.file_name().unwrap().to_str().unwrap()
            );
            fs::copy(from, to).expect("[!] Could not copy output file to outputs folder");
        }
        if self.state_path == "fitm-gen2-state0" {
            sleep(Duration::from_millis(0));
        }
        // After creating the outputs we go back into the base directory
        env::set_current_dir(&Path::new("../")).unwrap();

        Ok(())
    }

    pub fn create_outputs(&self, input_path: &str, output_path: &str) -> Result<(), io::Error> {
        // Work with absolute paths
        let input_path = build_create_absolute_path(input_path)
            .expect("[!] Error while constructing absolute input_dir path");
        let output_path = build_create_absolute_path(output_path)
            .expect("[!] Error while constructing absolute output_dir path");

        println!(
            "==== [*] Creating outputs for state: {} ====",
            self.state_path
        );
        let _ = io::stdout().flush();

        // Iterate through all entries of given folder and create output for each
        for (_, entry) in fs::read_dir(input_path)
            .expect(&format!(
                "[!] Could not read queue of state: {}",
                self.state_path
            ))
            .enumerate()
        {
            let entry_unwrapped = entry.unwrap();
            if entry_unwrapped.file_type().unwrap().is_dir() {
                continue;
            }
            let entry_path = entry_unwrapped.path();

            self.create_outputs_file(entry_path, output_path.as_str())?;
        }

        Ok(())
    }

    pub fn create_environment(&self) -> Result<(File, File), io::Error> {
        utils::create_restore_sh(self);
        // Change into our state directory and generate the afl maps there
        env::set_current_dir(ACTIVE_STATE)?;

        // Open a file for stdout and stderr to log to
        let (stdout, stderr) = (
            fs::File::create("stdout-afl")?,
            fs::File::create("stderr-afl")?,
        );

        Ok((stdout, stderr))
    }

    /// Copies the state from saved-states to active-state
    /// Returns a tuple of (stdout, stderr)
    /// We have to copy to an active state, because each state can only be restored once in CRIU
    /// Initial indicates which file handles (stdout, stderr) are returned
    pub fn to_active(&self) -> Result<(File, File), io::Error> {
        // If not currently needed, all states should reside in `saved-state`.
        // Thus they need to be copied to be fuzzed
        // clear active-state first to make sure fuzzed state folder ends up
        // as "active-state" and not within "active-state"
        match std::fs::remove_dir_all(ACTIVE_STATE) {
            Ok(_) => (),
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            Err(e) => println!("[!] Error while removing {}: {:?}", ACTIVE_STATE, e),
        };

        utils::cp_recursive(&format!("./saved-states/{}", self.state_path), ACTIVE_STATE);

        let (stdout, stderr) = self.create_environment()?;

        Ok((stdout, stderr))
    }

    pub fn create_next_snapshot(
        &self,
        state_id: usize,
        input_path: &str,
    ) -> Result<Option<FITMSnapshot>, io::Error> {
        let afl = FITMSnapshot::new(
            self.generation + 2,
            state_id,
            self.target_bin.to_string(),
            self.timeout,
            self.state_path.clone(),
            self.server,
            true,
            self.pid,
        );

        if afl.snapshot_run(input_path)? {
            println!(
                "==== [*] New snapshot: {} with input {} ====",
                afl.state_path, input_path
            );
            Ok(Some(afl))
        } else {
            println!(
                "==== [*] No snapshot: {} with input {} ====",
                afl.state_path, input_path
            );
            Ok(None)
        }
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn afl_cmin(
        &self,
        input_dir: &str,
        output_dir: &str,
        keep_traces: bool,
    ) -> Result<(), io::Error> {
        let input_dir = build_create_absolute_path(input_dir)
            .expect("[!] Error while constructing absolute input_dir path");
        let output_dir = build_create_absolute_path(output_dir)
            .expect("[!] Error while constructing absolute output_dir path");
        
        // Make sure we always have at least dummy input (even if the other side finished)
        let mut dummy_file = File::create(&format!("{}/dummy", &input_dir))?;
        dummy_file.write_all(b"dummy")?;

        // Spawn the afl run in a command. This run is relative to the state dir
        // meaning we already are inside the directory. This prevents us from
        // accidentally using different resources than we expect.

        let exit_status = NamespaceContext::new()
            .execute(|| -> io::Result<i32> {
                let (stdout, stderr) = self.to_active()?;
                // state has to be activated at this point
                assert!(env::current_dir().unwrap().ends_with(ACTIVE_STATE));

                let mut command = Command::new("../AFLplusplus/afl-cmin");
                command
                    .args(&[
                        "-i",
                        &input_dir,
                        "-o",
                        &output_dir,
                        // No mem limit
                        "-t",
                        &format!("{}", self.timeout.as_millis()),
                        "-m",
                        "none",
                        "-U",
                        "--",
                        "bash",
                        // Our restore script
                        "./restore.sh",
                        // The fuzzer input file
                        "@@",
                    ])
                    .stdout(Stdio::from(stdout))
                    .stderr(Stdio::from(stderr))
                    .env("CRIU_SNAPSHOT_DIR", "./snapshot")
                    // We launch sh first, which is (hopefully) not instrumented.
                    // Also, we cannot restore a snapshot more than once.
                    // In afl++ 3.01 cmin, this option will run the bin only once.
                    .env("AFL_SKIP_BIN_CHECK", "1")
                    .env("AFL_NO_UI", "1")
                    // Give criu forkserver up to a minute to spawn
                    .env("AFL_FORKSRV_INIT_TMOUT", "60000")
                    .env("AFL_DEBUG_CHILD_OUTPUT", "1")
                    .env("AFL_DEBUG", "1")
                    .env("FITM_CREATE_OUTPUTS", "1");

                // Don't keep traces BEFORE fuzzing, only afterwards.
                if keep_traces {
                    // afl-cmin will keep the showmap traces in `.traces` after each run
                    command.env("AFL_KEEP_TRACES", "1");
                }

                let mut child = command.spawn()?;
                let exit_status = child.wait()?;
                Ok(exit_status.code().unwrap())
            })
            .expect("[!] Namespace creation failed")
            .wait()
            .expect("[!] Namespace wait failed")
            .code()
            .unwrap();

        // We want to quit if cmin breaks (0) but not if it found a crash in the target (2)
        if exit_status != 0 && exit_status != 2 {
            let info =
                "[!] Error during afl-cmin execution. Please check latest statefolder for output";
            println!("{}", info);
            std::process::exit(1);
        }

        if fs::read_dir(&output_dir).unwrap().next().is_none() {
            println!("Cmin minimized to 0 testcases. Bug in cmin? Check active-dir.");
            std::process::exit(-1);
        }

        println!(
            "==== [*] Wrote cmin contents from {} to {} ====",
            input_dir, output_dir
        );
        Ok(())
    }
}

fn cpy_trace(trace_file: &str, state_path: &str) -> Result<(), io::Error> {
    // Copy the .trace to the new snapshot dir
    let to = format!("./saved-states/{}/snapshot_map", state_path);
    println!("saving trace_file: {} to: {}", &trace_file, &to);
    fs::copy(&trace_file, &to).expect(
        format!(
            "[!] cpy_trace failed to copy trace_file: {} to: {}",
            trace_file,
            to.as_str()
        )
        .as_str(),
    );

    Ok(())
}

/// Run afl_fuzz for each snapshot with all inputs for the current gen
/// @param current_snaps: list of snapshots for this stage
/// @param current_inputs: path to inputs for this stage
/// @return: upcoming snaps for the next generation based on current snaps (client->client, server->server)
pub fn process_stage(
    current_snaps: &Vec<FITMSnapshot>,
    current_inputs: &Vec<PathBuf>,
    next_gen_id_start: usize,
    run_time: &Duration,
) -> Result<Vec<FITMSnapshot>, io::Error> {
    let mut next_own_snaps: Vec<FITMSnapshot> = vec![];

    for snap in current_snaps {
        let cmin_tmp_dir = format!("cmin-tmp");

        // remove old tmp if it exists, then recreate
        let _ = std::fs::remove_dir_all(&cmin_tmp_dir);
        std::fs::create_dir_all(&cmin_tmp_dir)?;

        // Copy all current_inputs to cmin dir
        for (i, input) in current_inputs.iter().enumerate() {
            std::fs::copy(&input, &format!("{}/imported{}", &cmin_tmp_dir, i))?;
        }

        // Copy all queue items to cmin dir (doesn't necessary exist yet)
        let _ = snap.copy_queue_to(&Path::new(&cmin_tmp_dir), false);

        // cmin all files to the in dir
        let saved_state_dir = &format!("saved-states/{}/in", snap.state_path);
        let _ = std::fs::remove_dir_all(&saved_state_dir);

        // don't keep traces here
        snap.afl_cmin(&cmin_tmp_dir, &saved_state_dir, false)?;

        // afl_cmin exports minimized input to saved-states/$state/in
        // fuzz_run activates saved-states/$state and uses ./in as input
        snap.fuzz_run(&run_time)?;

        // current output to cmin-tmp
        let _ = std::fs::remove_dir_all(&cmin_tmp_dir);
        snap.copy_queue_to(&Path::new(&cmin_tmp_dir), true)
            .expect(format!("[!] copy_queue_to failed for snap: {}", snap.state_path).as_str());

        // Replace the old stored queue with the new, cminned queue
        let cmin_post_exec = format!("saved-states/{}/out/main/queue", snap.state_path);
        let _ = std::fs::remove_dir_all(&cmin_post_exec);

        // keep traces for snapshot creation
        snap.afl_cmin(&cmin_tmp_dir, &cmin_post_exec, true)?;

        // TODO: Make sure the same bitmap never creates a new snapshop for this state (may exist from last round already)

        let outputs = format!("saved-states/{}/outputs", snap.state_path);
        snap.create_outputs(&cmin_post_exec, &outputs)?;

        let absolut_cmin_post_exec = build_create_absolute_path(&cmin_post_exec)
            .expect("[!] Error while constructing absolute input_dir path");
        for entry in fs::read_dir(&absolut_cmin_post_exec)? {
            let entry = entry?;
            if entry.path().is_file() {
                // get the next id: current start + amount of snapshots we created in the meantime
                let state_id = next_gen_id_start + next_own_snaps.len();

                let trace_file = format!(
                    "{}/.traces/{}",
                    &absolut_cmin_post_exec,
                    entry.file_name().into_string().unwrap()
                );

                match get_traces().unwrap() {
                    // If we have seen the current trace before we don't want to create a new snapshot for this input
                    Some(traces) => {
                        let cur_trace = fs::read_to_string(&trace_file)
                            .expect("[!] Could not read current trace_file in process_stage");
                        if traces.iter().any(|trace| trace == cur_trace.as_str()) {
                            println!("==== [*] Skipping snapshot run for input (duplicate trace): {:?} ====", entry.path());
                            continue;
                        }
                    }
                    _ => (),
                }
                let snap_option = snap
                    .create_next_snapshot(state_id, entry.path().as_os_str().to_str().unwrap())?;
                match snap_option {
                    Some(new_snap) => {
                        cpy_trace(trace_file.as_str(), &new_snap.state_path)?;

                        // Commit this fresly-baked snapshot to our vec.
                        next_own_snaps.push(new_snap);
                    }
                    None => (),
                }
            }
        }

        fs::remove_dir_all(format!("{}/.traces", &absolut_cmin_post_exec))
            .expect("[!] Could not remove .traces after saving program maps");
    }

    Ok(next_own_snaps)
}

/// Originally proposed return value of process_stage()
/// @return: False, if we didn't advance to the next generation (no more output)
pub fn check_stage_advanced(next_inputs: &mut Vec<String>) -> bool {
    !next_inputs.is_empty()
}

// Get the (non-minimized) input dir to the generation with id gen_id
fn generation_input_dir(gen_id: usize) -> String {
    format!("./generation_inputs/{}", gen_id)
}

// Make sure the given folder exists
fn ensure_dir_exists(dir: &str) {
    fs_extra::dir::create_all(dir, false).expect("Could not create dir");
}

// Constructs an absolute path from a relative one. Creates the directory if it doesn't exist yet
fn build_create_absolute_path(relative: &str) -> Result<String, io::Error> {
    let canonicalized_string = PathBuf::from(relative).canonicalize();
    let os_string = match canonicalized_string {
        Ok(val) => val.into_os_string(),
        Err(_e) => {
            ensure_dir_exists(relative);
            PathBuf::from(relative)
                .canonicalize()
                .unwrap()
                .into_os_string()
        }
    };
    let absolute_str = String::from(
        os_string
            .to_str()
            .expect("[!] Could not convert os_string to str"),
    );
    Ok(absolute_str)
}

/// Naming scheme:
/// genX-stateY
/// X: identifies the generation, gen_id here
/// Y: as the generation already identifies which binary is currently fuzzed Y just iterates
/// the snapshots of this generation. Starts with 0 for each X
/// @param gen_id: The generation for which to get all inputs
/// @return: List of paths, one path per output per state
fn input_file_list_for_gen(gen_id: usize) -> Result<Vec<PathBuf>, io::Error> {
    // should match above naming scheme
    // Look for the last state's output to get the input.
    let gen_path = Regex::new(&format!("fitm-gen{}-state\\d+", gen_id - 1)).unwrap();
    // Using shell like globs would make this much easier: https://docs.rs/globset/0.4.6/globset/
    Ok(fs::read_dir("./saved-states/")?
        .into_iter()
        // Ignore errors
        .filter_map(|x| x.ok())
        // First, find all legit gen{gen_id}-state dirs
        .filter(|entry| {
            entry.path().is_dir() && gen_path.find(entry.path().to_str().unwrap()).is_some()
        })
        // return all files in outputs
        .map(|entry| entry.path().join("outputs").read_dir().unwrap())
        // We now have an iterator of directories of files, flatten to iterator of files
        .flatten()
        // Ignore more errors
        .filter_map(|x| x.ok())
        // read all files, return the strings
        .filter(|x| x.path().is_file())
        .map(|x| x.path())
        .collect())
}

// We are currently not sure if checking only current gen or all gens for duplicate traces is better
// Problem: Server & Client may indefinitely bounce "passwd" and "wrong passwd" back and forth
// without realizing that no new path has been found.
pub fn get_traces() -> io::Result<Option<Vec<String>>> {
    // should match naming scheme explained at `input_file_list_for_gen`
    let snapshot_regex = Regex::new("fitm-gen\\d+-state\\d+").unwrap();
    // Collect all snapshot folders in saved-states
    let states_iter = fs::read_dir(SAVED_STATES)
        .expect(&format!(
            "[!] Could not read_dir {} in get_traces.",
            SAVED_STATES
        ))
        .into_iter()
        .filter_map(|dir| dir.ok())
        .filter(|dir_entry| {
            dir_entry.path().is_dir()
                && snapshot_regex
                    .find(dir_entry.path().to_str().unwrap())
                    .is_some()
        });

    // Collect iterator of paths to all trace files
    let traces_iter = states_iter
        .map(|dir_entry| dir_entry.path().join("snapshot_map"))
        .filter(|snapshot_path| snapshot_path.is_file());

    // Return content of each file as vec
    let traces_vec: Vec<String> = traces_iter
        .map(|path| {
            fs::read_to_string(path)
                .expect("[!] Error while reading snapshot_map files in get_traces")
        })
        .collect();
    if traces_vec.len() > 0 {
        Ok(Some(traces_vec))
    } else {
        Ok(None)
    }
}

/// Run fitm
/// runtime indicates the time, after which the fuzzer switches to the next entry
pub fn run(
    client_bin: &str,
    client_args: &[&str],
    server_bin: &str,
    server_args: &[&str],
    run_time: &Duration,
) -> Result<(), io::Error> {
    // A lot of timeout for now
    let run_timeout = Duration::from_secs(3);

    // the folder contains inputs for each generation
    ensure_dir_exists(&generation_input_dir(0));
    ensure_dir_exists(&generation_input_dir(1));

    // Snapshot for gen2 (first client gen that's fuzzed) is created from this obj.
    let mut afl_client_snap: FITMSnapshot = FITMSnapshot::new(
        2,
        0,
        client_bin.to_string(),
        run_timeout,
        "".to_string(),
        false,
        false,
        None,
    );

    // first create a snapshot, without outputs
    afl_client_snap.pid = afl_client_snap.init_run(false, true, client_args)?;
    // Move ./fd files (hopefully just one) to ./outputs folder for gen 0, state 0
    // (to gen0-state0/outputs)
    // we just need tmp to create outputs
    // something fails if we don't use this tmp object
    let tmp = FITMSnapshot::new(
        2,
        0,
        client_bin.to_string(),
        run_timeout,
        "".to_string(),
        false,
        false,
        None,
    );
    tmp.init_run(true, false, client_args)?;

    let mut afl_server: FITMSnapshot = FITMSnapshot::new(
        1,
        0,
        server_bin.to_string(),
        run_timeout,
        "".to_string(),
        true,
        false,
        None,
    );
    afl_server.pid = afl_server.init_run(false, true, server_args)?;

    // We need initial outputs from the client, else something went wrong
    assert_ne!(input_file_list_for_gen(1)?.len(), 0);

    let mut generation_snaps: Vec<Vec<FITMSnapshot>> = vec![];
    // Gen 0 client doesn't need a snapshot (it's the run from binary start to initial recv)
    generation_snaps.push(vec![]);
    // Gen 1 server is the initial server snapshot at recv, awaiting gen 0's output as input
    generation_snaps.push(vec![afl_server]);
    // Gen 2 client is the initial client snapshot, awaiting gen 1's output (server response) as input
    generation_snaps.push(vec![afl_client_snap]);

    let mut current_gen = 0;

    loop {
        current_gen = current_gen + 1;
        if generation_snaps[current_gen].len() == 0 {
            println!(
                "No snapshots (yet) for gen {}, restarting with gen 1 (initial server)",
                current_gen
            );
            // Restart with gen 1 -> the client at gen 0 does not accept input.
            current_gen = 1;
        }

        println!("Fuzzing Gen {}", current_gen);

        // outputs of current gen (i.e. client) --> inputs[current_gen+1] (i.e. server)
        let next_other_gen = current_gen + 1;
        // snapshots based on current_gen (i.e. client) --> snaps[current_gen+2] (client)
        let next_own_gen = current_gen + 2;
        // Make sure we have vecs for the next client and server generations
        if next_other_gen == generation_snaps.len() {
            generation_snaps.push(vec![])
        }
        if next_own_gen == generation_snaps.len() {
            generation_snaps.push(vec![])
        }

        // In each generation, IDs are simply numbered
        let next_gen_id_start = generation_snaps[next_own_gen].len();
        let mut next_snaps = process_stage(
            &generation_snaps[current_gen],
            &input_file_list_for_gen(current_gen)?,
            next_gen_id_start,
            &run_time,
        )?;

        generation_snaps[next_own_gen].append(&mut next_snaps);
    }
}
