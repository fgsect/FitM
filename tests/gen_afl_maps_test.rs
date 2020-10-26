use fitm::AFLRun;
use std::fs;

mod common;

// init_run_test should check if a snapshot could be successfully be created.
// As the test does not have access to criu server responses or other logs it
// relies on the correct creation of various files

#[test]
fn create_outputs_test() {
    // pwd == root dir of repo
    common::setup();

    // creating the afl_client object manually would make the test even more
    // precise
    let afl_client: AFLRun = AFLRun::new(
        (0, 1),
        "tests/targets/echo_server".to_string(),
        1,
        "fitm-c0s1".to_string(),
        "".to_string(),
        false,
        false,
    );

    afl_client.init_run();

    std::fs::remove_dir_all(format!("active-state/{}", &afl_client.state_path))
        .expect(
            format!(
                "Could not remove '{}' in gen_afl_maps_test",
                &afl_client.state_path
            )
            .as_str(),
        );

    // populate in folder here
    let first = "a simple string";
    let second = "message 1, upcoming linebreak now:\nmessage 2";
    let third = "foo\tbar";
    fs::write("./saved-states/fitm-c0s1/outputs/first_case.txt", first)
        .expect("Could not write first input file");
    fs::write("./saved-states/fitm-c0s1/outputs/second_case.txt", second)
        .expect("Could not write second input file");
    fs::write("./saved-states/fitm-c0s1/outputs/third_case.txt", third)
        .expect("Could not write third input file");

    // tested function
    afl_client.gen_afl_maps();

    // break here and inspect `active-state/stdout-afl` to see breaking
    // forkserver
    common::teardown();
}
