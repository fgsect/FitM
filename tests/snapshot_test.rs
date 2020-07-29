use fitm::AFLRun;
mod common;

use regex::Regex;
use std::env;

// This test should check if a snapshot could be successfully be created.
// As the test does not have access to criu server responses or other logs it relies on the correct creation of various files
// If snapshotting was successful can only be definitively tested by also restoring the process

#[test]
fn init_run_test() {
    // pwd == root dir of repo
    common::setup();

    let mut afl_client: AFLRun = AFLRun::new(
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
    assert_eq!(stdout, stdout_expected);
    assert_eq!(stderr, stderr_expected);

    common::teardown();
}
