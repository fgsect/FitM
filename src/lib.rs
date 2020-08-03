use std::collections::{BTreeSet, VecDeque};
use std::env;
use std::fmt;
use std::fs;
use std::io;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

extern crate fs_extra;
use fs_extra::dir::*;
use std::thread::sleep;
use std::time::Duration;

mod utils;
// client_set: set of afl-showmap on client outputs that are relevant for us
// server_set: set of afl-showmap on server outputs that are relevant for us

/// AFLRun contains all the information for one specific fuzz run.
#[derive(Clone)]
pub struct AFLRun {
    /// Path to the base directory of the state of the current fuzz run
    pub state_path: String,
    /// Binary that is being fuzzed
    pub target_bin: String,
    /// Path to the state the current state receives input from
    pub previous_state_path: String,
    /// Timeout for this run
    /// TODO: probably should be dynamic based on how interesting this state is.
    pub timeout: u32,
    // All the states that came out of the current state
    // child_states: Vec<(u32, u32)>
    /// Used to determine whether to increase first or second value of state
    /// tuple. Hope this is not too broken
    pub server: bool,
    /// State folder name of the state from which this object's snapshot was created
    /// Empty if created from binary
    pub base_state: String,
    /// Marks if this run is an initial state or not
    pub initial: bool,
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
            .finish()
    }
}

/// Implementation of functions for an afl run
impl AFLRun {
    /// Create a new afl run instance
    pub fn new(
        new_state: (u32, u32),
        target_bin: String,
        timeout: u32,
        previous_state_path: String,
        base_state: String,
        server: bool,
        from_snapshot: bool,
    ) -> AFLRun {
        let state_path = format!("fitm-c{}s{}", new_state.0, new_state.1);
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
        } else {
            fs::create_dir(format!("active-state/{}/snapshot", state_path))
                .expect("[-] Could not create snapshot dir!");
        };

        if base_state != "".to_string() {
            // copy old fd folder for new state
            let from = format!("./saved-states/{}/fd", base_state);
            let to = format!("./active-state/{}/", state_path);
            utils::copy(from, to);
        }

        let new_run = AFLRun {
            state_path,
            target_bin,
            timeout,
            previous_state_path,
            server,
            base_state,
            initial: false,
        };

        // We can write a tool in the future to parse this info
        // and print a visualization of the state order
        let path = format!("./active-state/{}/run-info", new_run.state_path);
        let mut file =
            fs::File::create(path).expect("[!] Could not create aflrun file");
        file.write(format!("{:?}", new_run).as_bytes())
            .expect("[!] Could not write to aflrun file");

        new_run
    }

    fn copy_base_state(&self) -> () {
        // Cleanstill existing base state folders in active-state
        let existing_path = format!("./active-state/{}", self.base_state);

        // remove_dir_all panics if the target does not exist.
        // To still catch errors if sth goes wrong a match is used here.
        match std::fs::remove_dir_all(existing_path.clone()) {
            Result::Ok(_) => {
                println!("[!] Successfully deleted path: {}", existing_path)
            }
            Result::Err(err) => println!(
                "[!] Error while deleting old base state folder: {}",
                err
            ),
        }

        // copy old snapshot folder for criu
        let from = format!("./saved-states/{}", self.base_state);
        let to = format!("./active-state/");

        // Check fs_extra docs for different copy options
        let options = CopyOptions::new();
        fs_extra::dir::copy(from, to, &options)
            .expect("[!] Could not copy base state dir from saved-states");
    }

    /// Needed for the two initial snapshots created based on the target binaries
    pub fn init_run(&self) -> () {
        utils::create_restore_sh(self);
        let dev_null = "/dev/null";
        // create the .cur_input so that criu snapshots a fd connected to
        // .cur_input
        let stdin = fs::File::open(dev_null).unwrap();

        // Change into our state directory and create the snapshot from there
        env::set_current_dir(format!("./active-state/{}", self.state_path))
            .unwrap();

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

        sleep(Duration::new(0, 20000000));
        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        // With snapshot_run we move the state folder, but in this initial case we need to use
        // the state folder shortly after running this function
        utils::copy(
            format!("./active-state/{}", self.state_path),
            String::from("./saved-states/"),
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
        env::set_current_dir(format!("./active-state/{}", self.state_path))
            .unwrap();

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
        let snapshot_dir =
            format!("{}/snapshot", env::current_dir().unwrap().display());

        let _ = Command::new("setsid")
            .args(&[
                format!("stdbuf"),
                format!("-oL"),
                format!("../../restore.sh"),
                format!("{}", self.base_state),
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
        sleep(Duration::new(0, 20000000));

        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        utils::mv(
            format!("./active-state/{}", self.state_path),
            String::from("./saved-states/"),
        );
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn fuzz_run(&self) -> () {
        // If not currently needed, all states should reside in `saved-state`.
        // Thus they need to be copied to be fuzzed
        utils::copy(
            format!("./saved-states/{}", self.state_path),
            String::from("./active-state/"),
        );

        utils::copy(
            String::from("./saved-states/fitm-c1s0"),
            String::from("./active-state/"),
        );
        utils::copy(
            String::from("./saved-states/fitm-c0s1"),
            String::from("./active-state/"),
        );

        // Create a copy of the state folder in `active-state`
        // from which the "to-be-fuzzed" state was snapshotted from,
        // otherwise criu can't restore
        if self.base_state != "".to_string() {
            self.copy_base_state();
        }
        utils::create_restore_sh(self);

        // Change into our state directory and create fuzz run from there
        env::set_current_dir(format!("./active-state/{}", self.state_path))
            .unwrap();

        // Open a file for stdout and stderr to log to
        let stdout = fs::File::create("stdout-afl").unwrap();
        let stderr = fs::File::create("stderr-afl").unwrap();
        // fs::File::create("stdout").unwrap();
        // fs::File::create("stderr").unwrap();

        // Spawn the afl run in a command. This run is relative to the state dir
        // meaning we already are inside the directory. This prevents us from
        // accidentally using different resources than we expect.
        Command::new("../../AFLplusplus/afl-fuzz")
            .args(&[
                format!("-i"),
                format!("./in"),
                format!("-o"),
                format!("./out"),
                format!("-m"),
                format!("none"),
                format!("-d"),
                format!("-V"),
                format!("{}", self.timeout),
                format!("--"),
                format!("sh"),
                format!("../../restore.sh"),
                format!("{}", self.state_path),
                format!("@@"),
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("CRIU_SNAPSHOT_DIR", "./snapshot")
            .env("AFL_SKIP_BIN_CHECK", "1")
            .env("AFL_NO_UI", "1")
            .spawn()
            .expect("[!] Error while spawning fuzz run")
            .wait()
            .expect("[!] Error while waiting for fuzz run");

        sleep(Duration::new(0, 20000000));

        if self.state_path == "fitm-c2s1" {
            println!("pass");
        }

        // After finishing the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        println!("==== [*] Generating outputs for: {} ====", self.state_path);
        self.create_outputs();
    }

    fn create_outputs(&self) -> () {
        // For consistency, change into necessary dir inside the function
        env::set_current_dir(format!("./active-state/{}", self.state_path))
            .unwrap();

        // For the binary that creates the seed we need to take input from the in folder
        let input_path = if self.previous_state_path == "".to_string() {
            "./in"
        } else {
            "./out/queue"
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
                format!("../../saved-states/{}/snapshot", self.state_path),
                format!("."),
            );
            utils::copy(
                format!("../../saved-states/{}/fd", self.state_path),
                format!("."),
            );

            // Open a file for stdout and stderr to log to
            // We need to do this inside the loop as the process gets restored multiple times
            let stdout = fs::File::create("stdout-afl").unwrap();
            let stderr = fs::File::create("stderr-afl").unwrap();
            fs::File::create("stdout").unwrap();
            fs::File::create("stderr").unwrap();

            let entry_path = entry_unwrapped.path();
            let entry_file = fs::File::open(entry_path.clone())
                .expect("[!] Could not open queue file");
            println!("using output: {:?}", entry_path);
            let _ = Command::new("setsid")
                .args(&[
                    format!("stdbuf"),
                    format!("-oL"),
                    format!("sh"),
                    format!("../../restore.sh"),
                    format!("{}", self.state_path),
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
            sleep(Duration::new(0, 20000000));

            for entry in fs::read_dir("./fd")
                .expect("[!] Could not read populated fd folder")
            {
                let cur_file = entry.unwrap().file_name();
                let from = format!("./fd/{}", &cur_file.to_str().unwrap());
                let to = format!(
                    "./outputs/{}_{}",
                    index,
                    &cur_file.to_str().unwrap()
                );
                fs::copy(from, to)
                    .expect("[!] Could not copy output file to outputs folder");
            }
        }

        // After creating the outputs we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();
    }

    /// Generate the maps provided by afl-showmap. This is used to filter out
    /// for "interesting" new seeds meaning seeds, that will make the OTHER
    /// binary produce paths, which we haven't seen yet.
    fn gen_afl_maps(&self) -> io::Result<Child> {
        // If not currently needed, all states should reside in `saved-state`.
        // Thus they need to be copied to be fuzzed
        utils::copy(
            format!("./saved-states/{}", self.previous_state_path),
            String::from("./active-state/"),
        );

        // Create a copy of the state folder in `active-state`
        // from which the "to-be-fuzzed" state was snapshotted from,
        // otherwise criu can't restore
        if self.base_state != "".to_string() {
            self.copy_base_state();
        }

        // Change into our state directory and generate the afl maps there
        env::set_current_dir(format!("./active-state/{}", self.state_path))
            .unwrap();

        // Open a file for stdout and stderr to log to
        let stdout = fs::File::create("stdout-afl").unwrap();
        let stderr = fs::File::create("stderr-afl").unwrap();
        fs::File::create("stdout").unwrap();
        fs::File::create("stderr").unwrap();

        // Execute afl-showmap from the state dir. We take all the possible
        // inputs for the OTHER binary that we created with a call to `send`.
        // We then save the generated maps inside `out/maps` where they are used
        // later.
        // For the first run fitm-c1s0 "previous_state_path" actually is the
        // upcoming state.
        let ret = Command::new("../../AFLplusplus/afl-showmap")
            .args(&[
                format!("-i"),
                format!("./outputs"),
                format!("-o"),
                format!("./out/maps"),
                format!("-m"),
                format!("none"),
                format!("-Q"),
                format!("--"),
                format!("sh"),
                format!("../../restore.sh"),
                format!("{}", self.previous_state_path),
                format!("@@"),
            ])
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("CRIU_SNAPSHOT_DIR", "./snapshot") // which folder a snapshot will be saved to
            .env("AFL_SKIP_BIN_CHECK", "1")
            .env("AFL_NO_UI", "1")
            .env("AFL_DEBUG", "1")
            .spawn();
        sleep(Duration::new(0, 20000000));

        // After spawning showmap command we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        ret
    }

    pub fn create_new_run(
        &self,
        new_state: (u32, u32),
        input: String,
        timeout: u32,
        from_snapshot: bool,
    ) -> AFLRun {
        let input_path: String =
            format!("active-state/{}/outputs/{}", self.state_path, input);

        // FIXME: This needs to be dependant on some input parameter
        let target_bin = if self.server {
            "tests/targets/pseudoclient".to_string()
        } else {
            "tests/targets/pseudoserver".to_string()
        };

        // Only mutate cur_state in this method. So next_state_path gets a
        // readable copy. We update cur_state here with a new tuple.
        // cur_state = next_state_path(cur_state, true);
        // We create a new state for the other binary that is not fuzzed by "self".
        // For this new state previous_state is "self". And base_state is self.previous
        // as we generated the maps on self.previous
        // and thus create the new state from that snapshot
        let afl = AFLRun::new(
            new_state,
            target_bin.to_string(),
            timeout,
            self.state_path.clone(),
            self.previous_state_path.clone(),
            !self.server,
            from_snapshot,
        );

        let seed_file_path =
            format!("active-state/{}/in/{}", afl.state_path, input);

        fs::copy(input_path, &seed_file_path)
            .expect("[!] Could not copy to new afl.state_path");

        // let seed_file = fs::File::open(seed_file_path)
        //     .expect("[!] Could not create input file");

        afl.snapshot_run(format!("in/{}", input));

        afl
    }
}

pub fn run() {
    let cur_timeout = 3;
    let mut cur_state: (u32, u32) = (1, 0);
    let mut client_maps: BTreeSet<String> = BTreeSet::new();

    let mut afl_client: AFLRun = AFLRun::new(
        (1, 0),
        "tests/targets/pseudoclient".to_string(),
        cur_timeout,
        // TODO: Need some extra handling for this previous_path value
        "".to_string(),
        "".to_string(),
        false,
        false,
    );

    let afl_server: AFLRun = AFLRun::new(
        (0, 1),
        "tests/targets/pseudoserver".to_string(),
        cur_timeout,
        "fitm-c1s0".to_string(),
        "".to_string(),
        true,
        false,
    );
    let mut queue: VecDeque<AFLRun> = VecDeque::new();

    fs::write(
        format!("active-state/{}/in/1", afl_client.state_path),
        "init case.",
    )
    .expect("[-] Could not create initial test case!");

    afl_client.initial = true;

    afl_server.init_run();
    afl_client.init_run();

    queue.push_back(afl_client);
    queue.push_back(afl_server);
    // this does not terminate atm as consolidate_poc does not yet minimize
    // anything
    while !queue.is_empty() {
        println!("QUEUE: {:?}", queue);
        // kick off new run
        let afl_current = queue.pop_front().unwrap();

        if !afl_current.initial {
            //clean active-state dir before doing anything
            std::fs::remove_dir_all("./active-state")
                .expect("[!] Can't delete ./active-state");
            std::fs::create_dir("./active-state")
                .expect("[!] Can't create ./active-state");

            println!(
                "==== [*] Starting the fuzz run of: {} ====",
                afl_current.state_path
            );
            afl_current.fuzz_run()
        }

        // TODO: Fancier solution? Is this correct?
        if afl_current.previous_state_path != "".to_string() {
            println!(
                "==== [*] Generating maps for: {} ====",
                afl_current.state_path
            );
            let mut child_map = afl_current
                .gen_afl_maps()
                .expect("[!] Failed to start the showmap run");

            child_map
                .wait()
                .expect("[!] Error while waiting for the showmap run");
        } else {
            // copy output of first run of binary 1 to in of first run of bin 2 as seed
            let from = format!("active-state/{}/fd", afl_current.state_path);
            // apparently fs_extra can not copy content of `from` into folder `[..]/in`
            for entry in fs::read_dir(from)
                .expect("[!] Could not read output of initial run")
            {
                let entry_path = entry.unwrap().path();
                let filename =
                    entry_path.file_name().unwrap().to_string_lossy();
                let upcoming_run = queue.front().unwrap();
                let to = format!(
                    "saved-states/{}/in/{}",
                    upcoming_run.state_path, filename
                );

                std::fs::copy(entry_path, to).unwrap();
            }
        }

        // consolidate previous runs here
        let path = format!("active-state/{}/out/maps", afl_current.state_path);

        for entry in fs::read_dir(path)
            .expect("[!] Could not read maps dir while consolidating")
        {
            let entry_path = entry.unwrap().path();
            let new_map = fs::read_to_string(entry_path.clone())
                .expect("[!] Could not read map file while consolidating");

            if !client_maps.contains(new_map.as_str()) {
                client_maps.insert(new_map);

                // Consolidating binary 1 will yield more runs on binary 2
                cur_state =
                    utils::next_state_path(cur_state, afl_current.server);

                let in_file = entry_path.file_name().unwrap().to_str().unwrap();

                // if afl_current == first binary, first run
                let next_run =
                    if afl_current.previous_state_path == "".to_string() {
                        let tmp = queue.pop_front().expect(
                            "[!] Could not get second afl_run from queue",
                        );

                        let from = format!(
                            "active-state/{}/fd/{}",
                            afl_current.state_path, in_file
                        );
                        let to = format!(
                            "saved-states/{}/in/{}",
                            tmp.state_path, in_file
                        );

                        fs::copy(from, to)
                            .expect("[!] Could not copy in file to new state");

                        queue.push_front(tmp.clone());

                        None
                    } else {
                        Some(afl_current.create_new_run(
                            cur_state,
                            String::from(in_file),
                            afl_current.timeout.into(),
                            true,
                        ))
                    };

                if let Some(next_run) = next_run {
                    queue.push_back(next_run);
                }
            }
        }
        queue.push_back(afl_current.clone());
    }

    println!("[*] Reached end of programm. Quitting.");
}
