use fitm::AFLRun;
mod common;

use fs_extra::dir::CopyOptions;
use regex::Regex;
use std::env;
use std::fs::{remove_file, File};
use std::io::Write;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

// init_run_test should check if a snapshot could be successfully be created.
// As the test does not have access to criu server responses or other logs it relies on the correct creation of various files

#[test]
fn init_run_test() {
    // pwd == root dir of repo
    common::setup();

    // creating the afl_client object manually would make the test even more precise
    let afl_client: AFLRun = AFLRun::new(
        (1, 0),
        "tests/targets/pseudoclient".to_string(),
        1,
        "".to_string(),
        "".to_string(),
        false,
        false,
    );

    // tested function
    afl_client.init_run();

    // relevant files
    let pipes = std::fs::read_to_string("./active-state/fitm-c1s0/pipes")
        .expect("Pipes file missing");
    let run_info = std::fs::read_to_string("./active-state/fitm-c1s0/run-info")
        .expect("run-info file missing");
    let stdout = std::fs::read_to_string("./active-state/fitm-c1s0/stdout")
        .expect("stdout file missing");
    let stderr = std::fs::read_to_string("./active-state/fitm-c1s0/stderr")
        .expect("stderr file missing");

    // expected outputs
    let run_info_expected = "AFLRun { state_path: \"fitm-c1s0\", previous_state_path: \"\", base_state: \"\", target_bin: \"tests/targets/pseudoclient\", timeout: 1, server: false, initial: false }";

    // the regex matches e.g. "pipe:[123456]\npipe:[7890]\n"
    // \d{3,6} - 3 to 6 decimal digits
    let pipes_regex: Regex =
        Regex::new(r"^pipe:\[\d{3,6}]\npipe:\[\d{3,6}]\n$").unwrap();

    let stdout_expected = "client sent: R\n";
    let stderr_expected = "";

    // required assertions
    assert!(pipes_regex.is_match(&pipes));
    assert_eq!(run_info, run_info_expected);
    assert_eq!(
        stdout, stdout_expected,
        "Stdout expectation did not match. \
        Check whether pseudoclient_simple.c test target is compiled."
    );
    assert_eq!(stderr, stderr_expected);

    common::teardown();
}

// create_new_run_test checks if the method with the same name works correctly and
// the produced snapshot can be restored using restore.sh

#[test]
fn create_new_run_test() {
    // pwd == root dir of repo
    common::setup();

    // We need this folder as AFLRun::new copies the fd folder from there
    let base_state = "fitm-c1s0";
    fs_extra::dir::create_all(
        format!("./saved-states/{}/fd", base_state),
        false,
    )
    .expect("Could not create dummy fd folder");

    // creating the afl_client object manually would make the test even more precise
    // previous_state needs to be the same as base_state as create_new_run would normally generate
    // new AFLRuns for the opposite binary for the one currently fuzzed.
    // So if bin 1 was just fuzzed, consolidated and produced new outputs (and thus new paths in bin 2),
    // then create_new_run would produce new AFLRuns based on binary 2.
    let afl_client: AFLRun = AFLRun::new(
        (1, 0),
        "tests/targets/snapshot_creation".to_string(),
        1,
        "fitm-c1s0".to_string(),
        base_state.to_string(),
        false,
        false,
    );

    // required input for tested function
    let input_filepath = "input.txt";
    let mut stdin = File::create(format!(
        "./active-state/{}/in/{}",
        afl_client.state_path, input_filepath
    ))
    .expect("Could not create input file");
    stdin.write_all(b"a random teststring").unwrap();

    afl_client.init_run();

    let stdout = std::fs::read_to_string("./active-state/fitm-c1s0/stdout")
        .expect("stdout file missing");
    let stderr = std::fs::read_to_string("./active-state/fitm-c1s0/stderr")
        .expect("stderr file missing");
    let stdout_expected = "00\n";
    let stderr_expected = "";
    assert_eq!(stdout, stdout_expected);
    assert_eq!(stderr, stderr_expected);

    let outputs_file = "foo.out";
    let _ = File::create(format!(
        "./active-state/{}/outputs/{}",
        afl_client.state_path, outputs_file
    ))
    .expect("Couldn't create dummy output file");

    // let input = format!("./in/{}", input_filepath);
    // tested function
    let new_run =
        afl_client.create_new_run((2, 0), outputs_file.to_string(), 1, true);

    assert_eq!(new_run.state_path, "fitm-c2s0");
    // As long as target_bin selection in create_new_run is hardcoded,
    // this is what's expected at this point
    assert_eq!(new_run.target_bin, "tests/targets/pseudoserver");
    assert_eq!(new_run.previous_state_path, "fitm-c1s0");
    assert_eq!(new_run.timeout, 1);
    // afl_client was a client run, so the following run needs to be a server run
    assert_eq!(new_run.server, true);
    assert_eq!(new_run.base_state, "fitm-c1s0");
    assert_eq!(new_run.initial, false);

    let options = CopyOptions::new();
    fs_extra::dir::copy(
        "./saved-states/fitm-c2s0",
        "./active-state/",
        &options,
    )
    .expect("[!] Could not copy snapshot dir from previous state");

    // Restore the snapshotted process, because only by doing so can we be sure that the snapshot actually worked
    env::set_current_dir(format!("./active-state/{}", new_run.state_path))
        .unwrap();
    let _ = Command::new("sh")
        .args(&[
            format!("../../restore.sh"),
            format!("{}", new_run.state_path),
            "in/foo.out".to_string(),
        ])
        .spawn()
        .expect("[!] Could not spawn snapshot run")
        .wait()
        .expect("[!] Snapshot run failed");
    // fokn sleep seems necessary everywhere - w/o the process is not done printing before the assert is done
    sleep(Duration::new(0, 20000000));

    env::set_current_dir("../../").unwrap();

    let stdout = std::fs::read_to_string("./active-state/fitm-c2s0/stdout")
        .expect("stdout file missing");
    let stderr = std::fs::read_to_string("./active-state/fitm-c2s0/stderr")
        .expect("stderr file missing");
    let stdout_expected = "Success\nRestored\nOK\n01\n02\nSuccess\nRestored\nForkserver not started, since SHM_ENV_VAR env variable is missing\nOK\n03\n";
    let stderr_expected = "";
    assert_eq!(
        stdout, stdout_expected,
        "Stdout expectation did not match. \
        Check whether snapshot_creation.c test target is compiled."
    );
    assert_eq!(stderr, stderr_expected);

    // teardown
    remove_file(format!(
        "./active-state/{}/in/{}",
        afl_client.state_path, input_filepath
    ))
    .expect("Could not clean up input file");
    common::teardown();
}
