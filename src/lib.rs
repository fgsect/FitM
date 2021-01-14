use std::io;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::{env, fs::File};
use std::{fmt, path::PathBuf};
use std::{fs, io::ErrorKind};

use crate::utils::cp_recursive;
use regex::Regex;
use std::thread::sleep;
use std::time::Duration;

pub mod utils;
// client_set: set of afl-showmap on client outputs that are relevant for us
// server_set: set of afl-showmap on server outputs that are relevant for us

pub const ORIGIN_STATE_CLIENT: &str = "fitm-gen2-state0";
pub const ORIGIN_STATE_SERVER: &str = "fitm-gen1-state0";
pub const ACTIVE_STATE: &str = "active-state";

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
    fn copy_fds_to_output_for(&self, gen: u32, state: usize) -> Result<(), io::Error> {
        let state_path = state_path_for(gen, state);
        // Make sure state dir outputs exists
        let _ = fs::create_dir_all(&format!("./saved-states/{}/outputs", state_path));
        for (i, entry) in
            fs::read_dir(&format!("./saved-states/{}/fd", self.state_path))?.enumerate()
        {
            let path = entry?.path();
            if path.is_file() {
                std::fs::copy(
                    path,
                    &format!("./saved-states/{}/outputs/initial{}", &state_path, i),
                )?;
            }
        }
        Ok(())
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
        let from = format!("{}/out", ACTIVE_STATE);
        let to = format!("./saved-states/{}/out_postrun", self.state_path);
        cp_recursive(from.as_str(), to.as_str());
        Ok(())
    }

    /// Needed for the two initial snapshots created based on the target
    /// binaries
    pub fn init_run(&self) -> Result<(), io::Error> {
        // Change into our state directory and generate the afl maps there
        env::set_current_dir(ACTIVE_STATE)?;

        // Open a file for stdout and stderr to log to
        let (stdout, stderr) = (fs::File::create("stdout")?, fs::File::create("stderr")?);

        // create the .cur_input so that criu snapshots a fd connected to
        // .cur_input
        let dev_null = "/dev/null";
        let stdin = fs::File::open(dev_null).unwrap();

        let snapshot_dir = format!("{}/snapshot", env::current_dir().unwrap().display());
        fs::create_dir(&snapshot_dir).expect("[-] Could not create snapshot dir!");

        let old = utils::get_latest_mod_time(snapshot_dir.as_str());

        // Start the initial snapshot run. We use our patched qemu to emulate
        // until the first recv of the target is hit. We have to use setsid to
        // circumvent the --shell-job problem of criu and stdbuf to have the
        // correct stdin, stdout and stderr file descriptors.
        Command::new("setsid")
            .args(&[
                format!("stdbuf"),
                format!("-oL"),
                format!("../fitm-qemu-trace"),
                format!("../{}", self.target_bin),
                format!("{}", dev_null),
            ])
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
            .env("CRIU_SNAPSHOT_DIR", &snapshot_dir)
            .env("AFL_NO_UI", "1")
            .spawn()
            .expect("[!] Could not spawn snapshot run")
            .wait()
            .expect("[!] Snapshot run failed");

        sleep(Duration::new(0, 50000000));

        // if there is a positive difference `new` is more recent than `old` meaning some file in the folder changed
        let new = utils::get_latest_mod_time(snapshot_dir.as_str());
        let _success = utils::positive_time_diff(&old, &new);

        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../")).unwrap();

        // With snapshot_run we move the state folder instead of copying it,
        // but in this initial case we need to use
        // the state folder shortly after running this function
        utils::mv_rename(ACTIVE_STATE, &format!("./saved-states/{}", self.state_path));

        Ok(())
    }

    /// Create a new snapshot based on a given snapshot
    /// @return: boolean indicating whether a new snapshot was create or not (true == new snapshot created)
    pub fn snapshot_run(&self, stdin_path: &str) -> Result<bool, io::Error> {
        let (stdout, stderr) = self.create_environment()?;

        let stdin_file = fs::File::open(stdin_path).unwrap();
        // Start the initial snapshot run. We use our patched qemu to emulate
        // until the first recv of the target is hit. We have to use setsid to
        // circumvent the --shell-job problem of criu and stdbuf to have the
        // correct stdin, stdout and stderr file descriptors.
        let snapshot_dir = format!("{}/snapshot", env::current_dir().unwrap().display());

        let old = utils::get_latest_mod_time(snapshot_dir.as_str());

        Command::new("setsid")
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
            .env("AFL_NO_UI", "1")
            .spawn()
            .expect("[!] Could not spawn snapshot run")
            .wait()
            .expect("[!] Snapshot run failed");

        sleep(Duration::new(0, 50000000));

        // if there is a positive difference new is more recent than old meaning some file in the folder changed
        let new = utils::get_latest_mod_time(snapshot_dir.as_str());
        let success = utils::positive_time_diff(&old, &new);

        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../")).unwrap();

        utils::mv_rename(ACTIVE_STATE, &format!("./saved-states/{}", self.state_path));

        Ok(success)
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn fuzz_run(&self, run_duration: &Duration) -> Result<(), io::Error> {
        // If not currently needed, all states should reside in `saved-state`.
        // Thus they need to be copied to be fuzzed
        // stdout is mutable so it can be read later
        let (stdout, stderr) = self.to_active()?;
        println!("==== [*] Start fuzzing {} ====", self.state_path);
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
            .spawn()?
            .wait()?;

        // wait for 50 millis
        sleep(Duration::from_millis(50));

        if !exit_status.success() {
            let info =
                "[!] Error during afl-fuzz execution. Please check latest statefolder for output";
            println!("{}", info);
            std::process::exit(1);
        }

        // Doesn't work since File has no copy trait and Stdio:from doesn't take ref :(
        // let mut stdout_content = String::new();
        // stdout.read_to_string(&mut stdout_content).unwrap();
        // println!("==== [*] AFL++ stdout: \n{}", stdout_content);

        // After finishing the run we go back into the base directory
        env::set_current_dir(&Path::new("../"))?;

        self.save_fuzz_results()?;

        println!("==== [*] Finished fuzzing {} ====", self.state_path);

        Ok(())
    }

    pub fn create_outputs_file(
        &self,
        entry_path: PathBuf,
        output_path: &str,
    ) -> Result<(), io::Error> {
        let (stdout, stderr) = self.to_active()?;

        let entry_file = fs::File::open(entry_path.clone()).expect("[!] Could not open queue file");
        println!("==== [*] Using input: {:?} ====", entry_path);
        if self.state_path == "fitm-gen2-state0" {
            sleep(Duration::from_millis(0));
        }
        let exit_status = Command::new("setsid")
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
            .expect("[!] Snapshot run failed");

        // No new states are discovered if this sleep is not there
        // Didn't investigate further.
        sleep(Duration::new(0, 50000000));

        if !exit_status.success() {
            let info =
                "[!] Error during create_outputs execution. Please check latest statefolder for output";
            println!("{}", info);
            std::process::exit(1);
        }

        // Move created outputs to a given folder
        // Probably saved states, as current active-state folder will be deleted with next to_active()
        for entry in fs::read_dir("./fd").expect("[!] Could not read populated fd folder") {
            let cur_file = entry.unwrap().file_name();
            let from = format!("./fd/{}", &cur_file.to_str().unwrap());
            let destination_path = Path::new(output_path).join(cur_file);
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

    /// Generate the maps provided by afl-showmap. This is used to filter out
    /// "interesting" new seeds i.e. seeds that will make the OTHER
    /// binary produce paths, which we haven't seen yet.
    pub fn gen_afl_maps(&self) -> Result<(), io::Error> {
        let (stdout, stderr) = self.to_active()?;

        // Execute afl-showmap from the state dir. We take all the possible
        // inputs for the OTHER binary that we created with a call to `send`.
        // We then save the generated maps inside `out/maps` where they are used
        // later.
        Command::new("../AFLplusplus/afl-showmap")
            .args(&[
                format!("-i"),
                format!("./out/main/queue"),
                format!("-o"),
                format!("./out/maps"),
                format!("-m"),
                format!("none"),
                format!("-U"),
                format!("--"),
                format!("bash"),
                format!("./restore.sh"),
                format!("@@"),
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("CRIU_SNAPSHOT_DIR", "./snapshot") // which folder a snapshot will be saved to
            // Ignore that sh is not instrumented
            .env("AFL_SKIP_BIN_CHECK", "1")
            // We want commandline output
            .env("AFL_NO_UI", "1")
            // Give criu forkserver up to a minute to spawn
            .env("AFL_FORKSRV_INIT_TMOUT", "60000")
            // Give me more output
            .env("AFL_DEBUG", "1")
            .spawn()?
            .wait()?;

        sleep(Duration::new(0, 50000000));

        // After spawning showmap command we go back into the base directory
        env::set_current_dir(&Path::new("../"))?;
        Ok(())
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
        );

        if afl.snapshot_run(input_path)? {
            Ok(Some(afl))
        } else {
            Ok(None)
        }
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn afl_cmin(&self, input_dir: &str, output_dir: &str) -> Result<(), io::Error> {
        let input_dir = build_create_absolute_path(input_dir)
            .expect("[!] Error while constructing absolute input_dir path");
        let output_dir = build_create_absolute_path(output_dir)
            .expect("[!] Error while constructing absolute output_dir path");

        let (stdout, stderr) = self.to_active()?;

        // state has to be activated at this point
        assert!(env::current_dir().unwrap().ends_with(ACTIVE_STATE));

        // Spawn the afl run in a command. This run is relative to the state dir
        // meaning we already are inside the directory. This prevents us from
        // accidentally using different resources than we expect.
        let exit_status = Command::new("../AFLplusplus/afl-cmin")
            .args(&[
                format!("-i"),
                format!("{}", input_dir),
                format!("-o"),
                format!("{}", output_dir),
                // No mem limit
                format!("-t"),
                format!("{}", self.timeout.as_millis()),
                format!("-m"),
                format!("none"),
                format!("-U"),
                format!("--"),
                format!("bash"),
                // Our restore script
                format!("./restore.sh"),
                // The fuzzer input file
                format!("@@"),
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
            .spawn()?
            .wait()?;

        sleep(Duration::new(0, 50000000));

        // We want to quit if cmin breaks (0) but not if it found a crash in the target (2)
        if exit_status.code().unwrap() != 0 && exit_status.code().unwrap() != 2 {
            let info =
                "[!] Error during afl-cmin execution. Please check latest statefolder for output";
            println!("{}", info);
            std::process::exit(1);
        }
        // After finishing the run we go back into the base directory
        env::set_current_dir(&Path::new("../")).unwrap();

        println!(
            "==== [*] Wrote cmin contents from {} to {} ====",
            input_dir, output_dir
        );
        Ok(())
    }
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

        snap.afl_cmin(&cmin_tmp_dir, &saved_state_dir)?;

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
        snap.afl_cmin(&cmin_tmp_dir, &cmin_post_exec)?;

        // TODO: Make sure the same bitmap never creates a new snapshop for this state (may exist from last round already)

        let outputs = format!("saved-states/{}/outputs", snap.state_path);
        snap.create_outputs(&cmin_post_exec, &outputs)?;

        let absolut_cmin_post_exec = build_create_absolute_path(&cmin_post_exec)
            .expect("[!] Error while constructing absolute input_dir path");
        for entry in fs::read_dir(&absolut_cmin_post_exec)? {
            let entry = entry?;
            if entry.path().is_file() {
                // get the next id: current start + amount of snapshots we created in the meantime
                let snap_option = snap.create_next_snapshot(
                    next_gen_id_start + next_own_snaps.len(),
                    entry.path().as_os_str().to_str().unwrap(),
                )?;
                match snap_option {
                    Some(new_snap) => next_own_snaps.push(new_snap),
                    None => (),
                }
            }
        }
    }

    Ok(next_own_snaps)
}

/// Originally proposed return value of process_stage()
/// @return: False, if we didn't advance to the next generation (no more output)
pub fn check_stage_advanced(next_inputs: &mut Vec<String>) -> bool {
    !next_inputs.is_empty()
}

// We begin running the client to the first send (gen == 0), then we fuzz the server (gen == 1), the fuzz the client (gen == 2), etc.
// So every odd numbered is a server
const fn is_client(gen_id: usize) -> bool {
    gen_id % 2 == 0
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

/// Run fitm
/// runtime indicates the time, after which the fuzzer switches to the next entry
pub fn run(
    base_path: &str,
    client_bin: &str,
    server_bin: &str,
    run_time: &Duration,
) -> Result<(), io::Error> {
    // A lot of timeout for now
    let run_timeout = Duration::from_secs(3);

    // set the directory to base_path for all of this criu madness to work.
    env::set_current_dir(base_path)?;

    // the folder contains inputs for each generation
    ensure_dir_exists(&generation_input_dir(0));
    ensure_dir_exists(&generation_input_dir(1));

    // Snapshot for gen2 (first client gen that's fuzzed) is created from this obj.
    let afl_client_snap: FITMSnapshot = FITMSnapshot::new(
        2,
        0,
        client_bin.to_string(),
        run_timeout,
        "".to_string(),
        false,
        false,
    );

    afl_client_snap.init_run()?;
    // Move ./fd files (hopefully just one) to ./outputs folder for gen 0, state 0
    // (to gen0-state0/outputs)
    // This is the (theoretical) state before the initial server run.
    // afl_client_snap.create_outputs("". "./saved-states/fitm-gen0-state0");
    let outputs_path_absolute =
        build_create_absolute_path(format!("./saved-states/fitm-gen0-state0/outputs").as_str())?;
    afl_client_snap
        .create_outputs_file(PathBuf::from("/dev/null"), outputs_path_absolute.as_str())?;
    // afl_client_snap.copy_fds_to_output_for(0, 0)?;

    let afl_server: FITMSnapshot = FITMSnapshot::new(
        1,
        0,
        server_bin.to_string(),
        run_timeout,
        "".to_string(),
        true,
        false,
    );
    afl_server.init_run()?;

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

        println!(
            "Fuzzing {} (gen {})",
            if is_client(current_gen) {
                "client"
            } else {
                "server"
            },
            current_gen
        );

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
