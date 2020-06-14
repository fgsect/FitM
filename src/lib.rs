use std::process::{Command, Child, Stdio};
use std::path::Path;
use std::fs;
use std::io;
use std::env;
use std::collections::{VecDeque, BTreeSet};
use std::os::unix::fs::OpenOptionsExt;
use rand::random;

// client_set: set of afl-showmap on client outputs that are relevant for us
// server_set: set of afl-showmap on server outputs that are relevant for us

/// AFLRun contains all the information for one specific fuzz run.
struct AFLRun {
    /// Path to the base directory of the state of the current fuzz run
    state_path: String,
    /// Binary that is being fuzzed
    target_bin: String,
    /// Path to the state the current state receives input from
    previous_state_path: String,
    /// Timeout for this run
    /// TODO: probably should be dynamic based on how interesting this state is.
    timeout: String,
    // All the states that came out of the current state
    // child_states: Vec<(u32, u32)>
    /// Used to determine wether to increase first or second value of state tuple
    /// Hope this is not too broken
    server: bool
}

/// Implementation of functions for an afl run
impl AFLRun {
    /// Create a new afl run instance
    fn new(state_path: String, target_bin: String, timeout: String,
            previous_state_path: String, server: bool) -> AFLRun {
        // If the new state directory already exists we may have old data there
        // so we optionally delete it
        if Path::new(&format!("states/{}", state_path)).exists() {
            println!("[!] states/{} already exists! Recreating..", state_path);
            let delete = true;
            if delete {
                // expect already panics so we don't need to exit manually
                fs::remove_dir(format!("states/{}", state_path))
                    .expect("[-] Could not remove duplicate state dir!");
            }
        }

        // Create the new directories and files to make afl feel at home
        fs::create_dir(format!("states/{}", state_path))
            .expect("[-] Could not create state dir!");

        fs::create_dir(format!("states/{}/in", state_path))
            .expect("[-] Could not create in dir!");

        fs::create_dir(format!("states/{}/out", state_path))
            .expect("[-] Could not create out dir!");

        fs::create_dir(format!("states/{}/out/maps", state_path))
            .expect("[-] Could not create out/maps dir!");

        fs::create_dir(format!("states/{}/fd", state_path))
            .expect("[-] Could not create fd dir!");

        fs::create_dir(format!("states/{}/snapshot", state_path))
            .expect("[-] Could not create snapshot dir!");

        // Create a dummy .cur_input because the file has to exist once criu
        // restores the process
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o600)
            .open(format!("states/{}/out/.cur_input", state_path))
            .unwrap();

        AFLRun{ 
            state_path,
            target_bin,
            timeout,
            previous_state_path,
            server
        }
    }

    /// Wrapper for the snapshot run and to start the create the initial 
    /// snapshot of the binary
    fn init_run(&self) -> io::Result<Child> {
        // create the .cur_input so that criu snapshots a fd connected to
        // .cur_input
        let cur_input = fs::File::open(format!("states/{}/out/.cur_input",
            self.state_path)).unwrap();
        self.snapshot_run(cur_input)
    }

    /// Start the target binary for the first time and run until the first recv
    /// which will trigger the snapshot
    fn snapshot_run(&self, stdin: fs::File) -> io::Result<Child> {
        // Change into our state directory and create the snapshot from there
        env::set_current_dir(format!("./states/{}", self.state_path)).unwrap();

        // Open a file for stdout and stderr to log to
        let stdout = fs::File::create("stdout").unwrap();
        let stderr = fs::File::create("stderr").unwrap();

        // Start the initial snapshot run. We use our patched qemu to emulate
        // until the first recv of the target is hit. We have to use setsid to
        // circumvent the --shell-job problem of criu and stdbuf to have the
        // correct stdin, stdout and stderr file descriptors.
        let ret = Command::new("setsid")
            .args(&[
                format!("stdbuf"),
                format!("-oL"),
                format!("../../AFLplusplus/afl-qemu-trace"),
                format!("../../{}", self.target_bin),
            ])
            .stdin(Stdio::from(stdin))
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .env("LETS_DO_THE_TIMEWARP_AGAIN", "1")
            .env("CRIU_SNAPSHOT_DIR", "./snapshot")
            .env("AFL_NO_UI", "1")
            .spawn();

        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        ret
    }

    /// Start a single fuzz run in afl which gets restored from an earlier
    /// snapshot. Because we use sh and the restore script we have to skip the
    /// bin check
    fn fuzz_run(&self) -> io::Result<Child> {
        // Change into our state directory and create fuzz run from there
        env::set_current_dir(format!("./states/{}", self.state_path)).unwrap();

        // Spawn the afl run in a command. This run is relative to the state dir
        // meaning we already are inside the directory. This prevents us from
        // accidentally using different resources than we expect.
        let ret = Command::new("../../AFLplusplus/afl-fuzz")
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
                format!("@@")
            ])
            .env("CRIU_SNAPSHOT_DIR", "./snapshot")
            .env("AFL_SKIP_BIN_CHECK", "1")
            .env("AFL_NO_UI", "1")
            .spawn();

        // After spawning the run we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        ret
    }

    /// Generate the maps provided by afl-showmap. This is used to filter out 
    /// for "interesting" new seeds meaning seeds, that will make the OTHER 
    /// binary produce paths, which we haven't seen yet.
    fn gen_afl_maps(&self) -> io::Result<Child> {
        // Change into our state directory and generate the afl maps there
        env::set_current_dir(format!("./states/{}", self.state_path)).unwrap();

        // Create stdout, stderr and .cur_input to truncate the files so that 
        // criu can restore the processes
        fs::File::create("stdout").unwrap();
        fs::File::create("stderr").unwrap();

        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o600)
            .open("./out/.cur_input")
            .unwrap();

        // Execute afl-showmap from the state dir. We take all the possible 
        // inputs for the OTHER binary that we created with a call to `send`.
        // We then save the generated maps inside `out/maps` where they are used
        // later.
        // For the first run fitm-c1s0 "previous_state_path" actually is the upcoming state.
        let ret = Command::new("../../AFLplusplus/afl-showmap")
            .args(&[
                format!("-i"),
                format!("./fd"),
                format!("-o"),
                format!("./out/maps"),
                format!("-m"),
                format!("none"),
                format!("-Q"),
                format!("--"),
                format!("sh"),
                format!("../../restore.sh"),
                format!("{}", self.previous_state_path),
                format!("@@")
            ])
            .env("CRIU_SNAPSHOT_DIR", "./snapshot")
            .env("AFL_SKIP_BIN_CHECK", "1")
            .env("AFL_NO_UI", "1")
            .env("AFL_DEBUG", "1")
            .spawn();

        // After spawning showmap command we go back into the base directory
        env::set_current_dir(&Path::new("../../")).unwrap();

        ret
    }


    fn create_from_run(&self, new_state: (u32, u32), input: String,
            target_bin: String, server: bool) -> AFLRun {
        let cur_timeout = 1;
        let input_path: String = format!("states/{}/fd/{}",
            self.state_path, input);

        // Only mutate cur_state in this method. So next_state_path gets a
        // readable copy. We update cur_state here with a new tuple.
        // cur_state = next_state_path(cur_state, true);
        let afl = AFLRun::new(
            format!("fitm-c{}s{}", new_state.0, new_state.1),
            target_bin.to_string(),
            cur_timeout.to_string(),
            // FIXME: Wrong path
            format!("fitm-c{}s{}", new_state.0, new_state.1),
            server
        );

        let seed_file_path = format!("states/{}/in/{}", afl.state_path,
            random::<u16>());

        fs::copy(input_path, &seed_file_path)
            .expect("[!] Could not copy to new afl.state_path");

        let seed_file = fs::File::open(seed_file_path)
            .expect("[!] Could not create input file");

        let mut child = afl.snapshot_run(seed_file)
            .expect("Failed to start snapshot run");

        child.wait().expect("[!] Error while waiting for snapshot run");

        afl
    }
}

/// Create the next iteration from a given state directory. If inc_server is set
/// we will increment the state for the server from fitm-cXsY to fitm-cXsY+1.
/// Otherwise we will increment the state for the client from fitm-cXsY to
/// fitm-cX+1sY
fn next_state_path(state_path: (u32, u32), cur_is_server: bool) -> (u32, u32) {
    // If inc_server increment the server state else increment the client state
    if cur_is_server {
        ((state_path.0)+1, state_path.1)
    } else {
        (state_path.0, (state_path.1)+1)
    }

}



pub fn run() {
    let cur_timeout = 1;
    let mut cur_state: (u32, u32) = (1, 0);
    let mut client_maps: BTreeSet<String> = BTreeSet::new();

    let aflClient: AFLRun = AFLRun::new(
        "fitm-c1s0".to_string(),
        "test/pseudoclient".to_string(),
        cur_timeout.to_string(),
        // TODO: Need some extra handling for this previous_path value
        "fitm-c0s1".to_string(),
        false
    );

    let aflServer: AFLRun = AFLRun::new(
        "fitm-c0s1".to_string(),
        "test/pseudoserver".to_string(),
        cur_timeout.to_string(),
        "fitm-c1s0".to_string(),
        true
    );
    let mut queue: VecDeque<AFLRun> = VecDeque::new();

    fs::write(format!("states/{}/in/1", aflClient.state_path), "init case.")
        .expect("[-] Could not create initial test case!");

    let mut afl_server_child = aflServer.init_run().expect("Failed to execute initial afl");

    afl_server_child.wait().unwrap_or_else(|x| {
        println!("Error while waiting for snapshot run: {}", x);
        std::process::exit(1);
    });

    let mut afl_client_child = aflClient.init_run().expect("Failed to execute initial afl");

    afl_client_child.wait().unwrap_or_else(|x| {
        println!("Error while waiting for snapshot run: {}", x);
        std::process::exit(1);
    });

    queue.push_back(aflClient);
    queue.push_back(aflServer);
    // this does not terminate atm as consolidate_poc does not yet minimize
    // anything
    while !queue.is_empty() {
        // kick off new run
        let aflCurrent = queue.pop_front()
            .expect("[*] Queue is empty, no more jobs to be done");
        println!("[*] Starting the fuzz run of: {}", aflCurrent.state_path);
        let mut childFuzz = aflCurrent.fuzz_run().expect("[!] Failed to start fuzz run");
        childFuzz.wait().expect("[!] Error while waiting for fuzz run");
        let _tmp = aflCurrent.state_path.clone();

        // TODO: Fancier solution? Is this correct?
        println!("[*] Generating maps for: {}", aflCurrent.state_path);
        let mut childMap = aflCurrent.gen_afl_maps().expect("[!] Failed to start the showmap run");
        childMap.wait().expect("[!] Error while waiting for the showmap run");
        // consolidate previous runs here
        let path = format!("states/{}/out/maps", aflCurrent.state_path);
        for entry in fs::read_dir(path)
            .expect("[!] Could not read maps dir while consolidating") {
            let entry_path = entry.unwrap().path();
            let new_map = fs::read_to_string(entry_path.clone())
                .expect("[!] Could not read map file while consolidating");
            if !client_maps.contains(new_map.as_str()){
                client_maps.insert(new_map);

                // Consolidating binary 1 will yield more runs on binary 2
                cur_state = next_state_path(cur_state, aflCurrent.server);
                let state_path = format!("fitm-c{}s{}", cur_state.0, cur_state.1);
                let target_bin = if aflCurrent.server{
                    "test/pseudoclient".to_string()
                } else {
                    "test/pseudoserver".to_string()
                };

                let next_run = if aflCurrent.previous_state_path == "fitm-c0s1".to_string() {
                    queue.pop_front()
                        .expect("[!] Could not get first server aflRun from qeuue")

                } else {
                    AFLRun::new(
                        state_path,
                        target_bin,
                        cur_timeout.to_string(),
                        "fitm-c1s0".to_string(),
                        !aflCurrent.server
                    )
                };

                let in_file = entry_path.file_name().unwrap().to_str().unwrap();
                let from = format!("states/{}/fd/{}", aflCurrent.state_path, in_file);
                fs::copy(from, format!("states/{}/in/{}", next_run.state_path, in_file));

                queue.push_back(next_run);



            }
        }
    }

    println!("[*] Reached end of programm. Quitting.");
}
