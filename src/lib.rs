use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::{env, fs::File};
use std::{fmt, path::PathBuf};

use fs_extra::dir::*;
use regex::Regex;
use std::thread::sleep;
use std::time::Duration;

pub mod utils;
// client_set: set of afl-showmap on client outputs that are relevant for us
// server_set: set of afl-showmap on server outputs that are relevant for us

pub const ORIGIN_STATE_CLIENT: &str = "fitm-gen0-state0";
pub const ORIGIN_STATE_SERVER: &str = "fitm-gen1-state0";

/// AFLRun contains all the information for one specific fuzz run.
#[derive(Clone)]
pub struct AFLRun {
    /// The fitm generation (starting with 0 for the initial client)
    pub generation: u32,
    /// The state id, unique in one generation
    pub state_id: u32,
    /// Path to the base directory of the state of the current fuzz run
    pub state_path: String,
    /// Binary that is being fuzzed
    pub target_bin: String,
    /// Path to the state the current state receives input from
    pub previous_state_path: String,
    /// Timeout for this run
    /// TODO: probably should be dynamic based on how interesting this state
    /// is.
    pub timeout: u32,
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
    pub active_dir: &'static str,
}

impl fmt::Debug for AFLRun {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AFLRun")
            .field("state_path", &self.state_path)
            .field("previous_state_path", &self.previous_state_path)
            .field("base_state", &self.base_state)
            .field("target_bin", &self.target_bin)
            .field("timeout", &self.timeout)
            .field("server", &self.server)
            .field("initial", &self.initial)
            .field("active_dir", &self.active_dir)
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

/// Implementation of functions for an afl run
/// Createing a new AFLRun will create the necessary directory in active-state
impl AFLRun {
    /// Create a new afl run instance
    pub fn new(
        generation: u32,
        state_id: u32,
        target_bin: String,
        timeout: u32,
        previous_state_path: String,
        base_state: String,
        server: bool,
        from_snapshot: bool,
    ) -> AFLRun {
        let active_dir = origin_state(server);

        let state_path = format!("fitm-gen{}-state{}", generation, state_id);

        // If the new state directory already exists we may have old data there
        // so we optionally delete it
        if Path::new(&format!("active-state/{}", state_path)).exists() {
            println!(
                "[!] active-state/{} already exists! Recreating..",
                state_path
            );
            let delete = true;
            if delete {
                // expect already panics so we don't need to exit manually
                fs::remove_dir(format!("active-state/{}", state_path))
                    .expect("[-] Could not remove duplicate state dir!");
            }
        }

        // Create the new directories and files to make afl feel at home
        fs::create_dir(format!("active-state/{}", state_path))
            .expect("[-] Could not create state dir!");

        fs::create_dir(format!("active-state/{}/in", state_path))
            .expect("[-] Could not create in dir!");

        fs::create_dir(format!("active-state/{}/out", state_path))
            .expect("[-] Could not create out dir!");

        fs::create_dir(format!("active-state/{}/outputs", state_path))
            .expect("[-] Could not create outputs dir!");

        fs::create_dir(format!("active-state/{}/out/maps", state_path))
            .expect("[-] Could not create out/maps dir!");

        let fd_path = format!("active-state/{}/fd", state_path);
        fs::create_dir(fd_path.clone()).expect("[-] Could not create fd dir!");

        if from_snapshot {
            utils::copy_snapshot_base(&base_state, &state_path);

            if base_state != "".to_string() {
                // copy old fd folder for new state
                let from = format!("./saved-states/{}/fd", base_state);
                let to = format!("./active-state/{}/", state_path);
                utils::copy(&from, &to);
            }
        } else {
            fs::create_dir(format!("active-state/{}/snapshot", state_path))
                .expect("[-] Could not create snapshot dir!");
        };

        let new_run = AFLRun {
            generation,
            state_id,
            state_path,
            target_bin,
            timeout,
            previous_state_path,
            server,
            base_state,
            initial: false,
            active_dir: active_dir,
        };

        // We can write a tool in the future to parse this info
        // and print a visualization of the state order
        let path = format!("./active-state/{}/run-info", new_run.state_path);
        let mut file = fs::File::create(path).expect("[!] Could not create aflrun file");
        file.write(format!("{:?}", new_run).as_bytes())
            .expect("[!] Could not write to aflrun file");

        new_run
    }

    /// Copies everything in ./fd to ./outputs/
    /// this is used on the initial client state to generate intitial inputs for the first server run
    fn copy_fds_to_output(&self) -> Result<(), io::Error> {
        for (i, entry) in
            fs::read_dir(&format!("./saved-states/{}/fd", self.state_path))?.enumerate()
        {
            let path = entry?.path();
            if path.is_file() {
                std::fs::copy(
                    path,
                    &format!("./saved-states/{}/outputs/initial{}", self.state_path, i),
                )?;
            }
        }
        Ok(())
    }

    /// Copies everything in ./fd to ./outputs/
    /// this is used on the initial client state to generate intitial inputs for the first server run
    fn copy_queue_to(&self, dst: &Path, active: bool) -> Result<(), io::Error> {
        for (i, entry) in fs::read_dir(&format!(
            "./{}/{}/out/main/queue",
            if active {
                "active-state"
            } else {
                "saved-states"
            },
            self.state_path
        ))?
        .enumerate()
        {
            let path = entry?.path();
            let name = path.file_name().unwrap();
            if path.is_file() {
                std::fs::copy(&path, dst.join(&name))?;
            }
        }
        Ok(())
    }

    fn copy_base_state(&self) -> () {
        // Cleanstill existing base state folders in active-state
        let existing_path = format!("./active-state/{}", self.active_dir);

        // remove_dir_all panics if the target does not exist.
        // To still catch errors if sth goes wrong a match is used here.
        match std::fs::remove_dir_all(existing_path.clone()) {
            Result::Ok(_) => println!("[!] Successfully deleted path: {}", existing_path),
            Result::Err(err) => println!("[!] Error while deleting old base state folder: {}", err),
        }

        // copy old snapshot folder for criu
        let from = format!("./saved-states/{}", self.active_dir);
        let to = format!("./active-state");

        // Check fs_extra docs for different copy options
        let mut options = CopyOptions::new();
        options.overwrite = true;
        fs_extra::dir::copy(from, to, &options)
            .expect("[!] Could not copy base state dir from saved-states");
    }

    /// Needed for the two initial snapshots created based on the target
    /// binaries
    pub fn init_run(&self) -> () {
        let dev_null = "/dev/null";
        // create the .cur_input so that criu snapshots a fd connected to
        // .cur_input
        let stdin = fs::File::open(dev_null).unwrap();

        // Change into our state directory and create the snapshot from there
        env::set_current_dir(format!("./active-state/{}", self.state_path)).unwrap();

        // Open a file for stdout and stderr to log to
        let stdout = fs::File::create("stdout").unwrap();
        let stderr = fs::File::create("stderr").unwrap();

        // Start the initial snapshot run. We use our patched qemu to emulate
        // until the first recv of the target is hit. We have to use setsid to
        // circumvent the --shell-job problem of criu and stdbuf to have the
        // correct stdin, stdout and stderr file descriptors.
        let _ = Command::new("setsid")
            .args(&[
                format!("stdbuf"),
                format!("-oL"),
                format!("../../AFLplusplus/afl-qemu-trace"),
                format!("../../{}", self.target_bin),
                format!("{}", dev_null),
            ])
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
            .env("FITM_CREATE_OUTPUTS", "1")
            .env("CRIU_SNAPSHOT_DIR", "./snapshot")
            .env("AFL_NO_UI", "1")
            .spawn()
            .expect("[!] Could not spawn snapshot run")
            .wait()
            .expect("[!] Snapshot run failed");

        sleep(Duration::new(0, 50000000));
        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        // With snapshot_run we move the state folder instead of copying it,
        // but in this initial case we need to use
        // the state folder shortly after running this function
        utils::copy(
            &format!("./active-state/{}", self.state_path),
            &format!("./saved-states"),
        );
    }

    /// Create a new snapshot based on a given snapshot
    pub fn snapshot_run(&self, stdin: String) -> () {
        // Create a copy of the state folder in `active-state`
        // from which the "to-be-fuzzed" state was snapshotted from,
        // otherwise criu can't restore
        if self.base_state != "".to_string() {
            self.copy_base_state();
        }
        utils::create_restore_sh(self);

        // Change into our state directory and create the snapshot from there
        env::set_current_dir(format!("./active-state/{}", self.state_path)).unwrap();

        let stdin_file = fs::File::open(stdin.clone()).unwrap();
        // Open a file for stdout and stderr to log to
        let stdout = fs::File::create("stdout-afl").unwrap();
        let stderr = fs::File::create("stderr-afl").unwrap();
        fs::File::create("stdout").unwrap();
        fs::File::create("stderr").unwrap();

        // Start the initial snapshot run. We use our patched qemu to emulate
        // until the first recv of the target is hit. We have to use setsid to
        // circumvent the --shell-job problem of criu and stdbuf to have the
        // correct stdin, stdout and stderr file descriptors.
        let snapshot_dir = format!("{}/snapshot", env::current_dir().unwrap().display());

        let _ = Command::new("setsid")
            .args(&[
                format!("stdbuf"),
                format!("-oL"),
                format!("./restore.sh"),
                stdin,
            ])
            .stdin(Stdio::from(stdin_file))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
            .env("CRIU_SNAPSHOT_DIR", snapshot_dir)
            .env("AFL_NO_UI", "1")
            .spawn()
            .expect("[!] Could not spawn snapshot run")
            .wait()
            .expect("[!] Snapshot run failed");
        sleep(Duration::new(0, 50000000));

        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        utils::mv(
            &format!("./active-state/{}", self.state_path),
            &format!("./saved-states"),
        );
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn fuzz_run(&self) -> Result<(), io::Error> {
        // If not currently needed, all states should reside in `saved-state`.
        // Thus they need to be copied to be fuzzed
        let (stdout, stderr) = self.to_active()?;

        // Spawn the afl run in a command. This run is relative to the state dir
        // meaning we already are inside the directory. This prevents us from
        // accidentally using different resources than we expect.
        Command::new("../../AFLplusplus/afl-fuzz")
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
                format!("{}", self.timeout),
                // Timeout per individual execution
                format!("-t"),
                format!("1000"),
                format!("--"),
                format!("sh"),
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

        // After finishing the run we go back into the base directory
        env::set_current_dir(&Path::new("../../"))?;

        // println!("==== [*] Generating outputs for: {} ====", self.state_path);
        //self.create_outputs();

        Ok(())
    }

    pub fn create_outputs(&self) -> () {
        utils::create_restore_sh(self);

        // For consistency, change into necessary dir inside the function
        env::set_current_dir(format!("./active-state/{}", self.state_path)).unwrap();

        // For the binary that creates the seed we need to take input from the
        // in folder
        let input_path = if self.previous_state_path == "".to_string() {
            "./in"
        } else {
            "./out/main/queue"
        };

        for (index, entry) in fs::read_dir(input_path)
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

            std::fs::remove_dir_all(String::from("./snapshot"))
                .expect("[!] Error deleting old snapshot folder");
            std::fs::remove_dir_all(String::from("./fd"))
                .expect("[!] Error deleting old fd folder");
            utils::copy(
                &format!("../../saved-states/{}/snapshot", self.state_path),
                &format!("."),
            );
            utils::copy(
                &format!("../../saved-states/{}/fd", self.state_path),
                &format!("."),
            );

            // Open a file for stdout and stderr to log to
            // We need to do this inside the loop as the process gets restored
            // multiple times
            let stdout = fs::File::create("stdout-afl").unwrap();
            let stderr = fs::File::create("stderr-afl").unwrap();
            fs::File::create("stdout").unwrap();
            fs::File::create("stderr").unwrap();

            let entry_path = entry_unwrapped.path();
            let entry_file =
                fs::File::open(entry_path.clone()).expect("[!] Could not open queue file");
            println!("using output: {:?}", entry_path);
            let _ = Command::new("setsid")
                .args(&[
                    format!("stdbuf"),
                    format!("-oL"),
                    format!("sh"),
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

            for entry in fs::read_dir("./fd").expect("[!] Could not read populated fd folder") {
                let cur_file = entry.unwrap().file_name();
                let from = format!("./fd/{}", &cur_file.to_str().unwrap());
                let to = format!("./outputs/{}_{}", index, &cur_file.to_str().unwrap());
                fs::copy(from, to).expect("[!] Could not copy output file to outputs folder");
            }
        }

        // After creating the outputs we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();
    }

    /// Copies the state from saved-states to active-state
    /// Returns a tuple of (stdout, stderr)
    pub fn to_active(&self) -> Result<(File, File), io::Error> {
        // If not currently needed, all states should reside in `saved-state`.
        // Thus they need to be copied to be fuzzed
        let _ = fs::remove_dir_all(&format!("./active-state/{}", self.state_path));
        utils::copy_ignore(
            &format!("./saved-states/{}", self.state_path),
            &format!("./active-state"),
        );

        // Create a copy of the state folder in `active-state`
        // from which the "to-be-fuzzed" state was snapshotted from,
        // otherwise criu can't restore
        if self.base_state != "".to_string() {
            self.copy_base_state();
        }

        utils::create_restore_sh(self);
        // Change into our state directory and generate the afl maps there
        env::set_current_dir(format!("./active-state/{}", self.state_path))?;

        // Open a file for stdout and stderr to log to
        let stdout = fs::File::create("stdout-afl")?;
        let stderr = fs::File::create("stderr-afl")?;
        fs::File::create("stdout")?;
        fs::File::create("stderr")?;
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
        // For the first run fitm-c1s0 "previous_state_path" actually is the
        // upcoming state.
        Command::new("../../AFLplusplus/afl-showmap")
            .args(&[
                format!("-i"),
                format!("./out/main/queue"),
                format!("-o"),
                format!("./out/maps"),
                format!("-m"),
                format!("none"),
                format!("-U"),
                format!("--"),
                format!("sh"),
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
        env::set_current_dir(&Path::new("../../"))?;
        Ok(())
    }

    pub fn create_new_run(
        &self,
        generation: u32,
        state_id: u32,
        input: String,
        timeout: u32,
        from_snapshot: bool,
    ) -> AFLRun {
        let input_path: String = format!("active-state/{}/outputs/{}", self.state_path, input);

        // Only mutate cur_state in this method. So next_state_path gets a
        // readable copy. We update cur_state here with a new tuple.
        // cur_state = next_state_path(cur_state, true);
        // We create a new state for the other binary that is not fuzzed by
        // "self". For this new state previous_state is "self". And
        // base_state is self.previous as we generated the maps on
        // self.previous and thus create the new state from that
        // snapshot
        let afl = AFLRun::new(
            generation,
            state_id,
            self.target_bin.to_string(),
            timeout,
            self.state_path.clone(),
            self.previous_state_path.clone(),
            self.server,
            from_snapshot,
        );

        let seed_file_path = format!("active-state/{}/in/{}", afl.state_path, input);

        fs::copy(input_path, &seed_file_path).expect("[!] Could not copy to new afl.state_path");

        // let seed_file = fs::File::open(seed_file_path)
        //     .expect("[!] Could not create input file");

        afl.snapshot_run(format!("in/{}", input));

        afl
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn afl_cmin(&self, input_dir: &str, output_dir: &str) -> Result<(), io::Error> {
        // Make sure we use absolute paths
        let cwd = env::current_dir().unwrap();
        let cwd = cwd.to_str().unwrap();
        let input_dir = format!("{}/{}", &cwd, &input_dir);
        let output_dir = format!("{}/{}", &cwd, &output_dir);

        let (stdout, stderr) = self.to_active()?;
        // state has to be activated at this point
        assert!(env::current_dir().unwrap().ends_with(&self.state_path));

        // fs::File::create("stdout").unwrap();
        // fs::File::create("stderr").unwrap();

        // Spawn the afl run in a command. This run is relative to the state dir
        // meaning we already are inside the directory. This prevents us from
        // accidentally using different resources than we expect.
        Command::new("../../AFLplusplus/afl-cmin")
            .args(&[
                format!("-i"),
                format!("{}", input_dir),
                format!("-o"),
                format!("{}", output_dir),
                // No mem limit
                format!("-m"),
                format!("none"),
                format!("-U"),
                format!("--"),
                format!("sh"),
                // Our restore script
                format!("./restore.sh"),
                // The fuzzer input file
                format!("@@"),
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("CRIU_SNAPSHOT_DIR", "./snapshot")
            // We launch sh first, which is (hopefully) not instrumented
            .env("AFL_SKIP_BIN_CHECK", "1")
            .env("AFL_NO_UI", "1")
            // Give criu forkserver up to a minute to spawn
            .env("AFL_FORKSRV_INIT_TMOUT", "60000")
            .env("AFL_DEBUG_CHILD_OUTPUT", "1")
            .env("AFL_DEBUG", "1")
            .spawn()?
            .wait()?;

        sleep(Duration::new(0, 50000000));

        // After finishing the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        /*
        utils::copy("./active_state/cmin_tmp", output_dir);
        utils::rm("./active_state/cmin_tmp");
        */

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
    current_snaps: &Vec<AFLRun>,
    current_inputs: &Vec<PathBuf>,
    run_time: Duration,
) -> Result<Vec<AFLRun>, io::Error> {
    let next_own_snaps: Vec<AFLRun> = vec![];

    for snap in current_snaps {
        /*
        state:
        - snapshot (already read stdin)
            - fds
        - afl directory (queue)
        - outputs directory (queue->cmin->single run)


        criu_restore(input_fd)
            dup2(input_fd, 0)
            close(input_fd)
            close(0)
            criu_snapshot()
            dup2(input_fd, 0)

        */

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
        let _ = std::fs::remove_dir_all(&format!("./saved-states/{}/in", snap.state_path));

        let saved_state_dir = &format!("saved-states/{}/in", snap.state_path);
        let output_dir = Path::new(&saved_state_dir).as_os_str().to_str().unwrap();

        snap.afl_cmin(&cmin_tmp_dir, &output_dir)?;

        snap.fuzz_run()?; //TODO: inputs)?;

        let _ = std::fs::remove_dir_all(&cmin_tmp_dir);
        snap.copy_queue_to(&Path::new(&cmin_tmp_dir), true)?;

        /// Replace the old stored queue with the new cminned queue
        let _ = std::fs::remove_dir_all(&format!(
            "./saved-states/{}/out/main/queue",
            snap.state_path
        ));
        snap.afl_cmin(
            &cmin_tmp_dir,
            &format!("./saved-states/{}/out/main/queue", snap.state_path),
        )?;

        /*
        // Python pseudocode for the next steps:
        for queue_entry in minimized_queue:
            output = snap.restore().input(queue_entry).run_to_recv().output()
            if output:
                next_gen_valid = True
                next_inputs.append(output)

        # Get all snapshots for the n+1 server run (later)
        # This could also be done at a later time.
        for queue_entry in minimized_queue:
            next_own_snaps.append(
                snap.restore().input(queue_entry).run_to_recv().snapshot()
            )


        */
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

/// Gets the correct binary for the passed gen_id (server or client bin)
const fn bin_for_gen<'a>(gen_id: usize, server_bin: &'a str, client_bin: &'a str) -> &'a str {
    if is_client(gen_id) {
        client_bin
    } else {
        server_bin
    }
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
        // return all files in outpus
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
    run_time: Duration,
) -> Result<(), io::Error> {
    let cur_timeout = 10;

    // set the directory to base_path for all of this criu madness to work.
    let dir_prev = env::current_dir()?;
    env::set_current_dir(base_path)?;

    // the folder contains inputs for each generation
    ensure_dir_exists(&generation_input_dir(0));
    ensure_dir_exists(&generation_input_dir(1));

    let afl_client: AFLRun = AFLRun::new(
        0,
        0,
        client_bin.to_string(),
        cur_timeout,
        // TODO: Need some extra handling for this previous_path value
        "".to_string(),
        "".to_string(),
        false,
        false,
    );
    afl_client.init_run();
    // Probably: Move ./fd files (hopefully just one) to ./outputs folder
    afl_client.copy_fds_to_output()?;

    let afl_server: AFLRun = AFLRun::new(
        1,
        0,
        server_bin.to_string(),
        cur_timeout,
        "".to_string(),
        "".to_string(),
        true,
        false,
    );
    afl_server.init_run();

    // We need initial outputs from the client, else something went wrong
    assert_ne!(input_file_list_for_gen(1)?.len(), 0);

    let mut generation_snaps: Vec<Vec<AFLRun>> = vec![];
    generation_snaps.push(vec![afl_client]);
    generation_snaps.push(vec![afl_server]);

    let mut current_gen = 0;

    loop {
        current_gen = current_gen + 1;
        if generation_snaps[current_gen].len() == 0 {
            println!(
                "No for snapshots (yet) for gen {}, restarting with initial server",
                current_gen
            );
            // Restart with gen 1 -> the client at gen 0 doesn not accept input.
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

        let mut next_snaps = process_stage(
            &generation_snaps[current_gen],
            &input_file_list_for_gen(current_gen)?,
            run_time,
        )?;

        generation_snaps[next_own_gen].append(&mut next_snaps);
    }
}
