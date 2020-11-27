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
        (1, 0),
        "tests/targets/echo_server".to_string(),
        1,
        "".to_string(),
        "fitm-client".to_string(),
        false,
        false,
    );

    // tested function
    afl_client.init_run();

    // populate in folder here
    let first = "a simple string";
    let second = "message 1, upcoming linebreak now:\nmessage 2";
    let third = "foo\tbar";
    fs::write("./active-state/fitm-client/in/first_case.txt", first)
        .expect("Could not write first input file");
    fs::write("./active-state/fitm-client/in/second_case.txt", second)
        .expect("Could not write second input file");
    fs::write("./active-state/fitm-client/in/third_case.txt", third)
        .expect("Could not write third input file");

    afl_client.create_outputs();

    for path in fs::read_dir("./active-state/fitm-client/outputs")
        .expect("Couldn't read outputs dir")
    {
        let file_path = path.as_ref().unwrap().path();
        let file_content = std::fs::read_to_string(&file_path)
            .expect(format!("{} file missing", &file_path.display()).as_str());
        // holy cow
        let mut file_name = path.unwrap().file_name().into_string().unwrap();
        file_name.truncate(2);
        match file_name.as_str() {
            "0_" => assert_eq!(file_content, first),
            "1_" => assert_eq!(file_content, second),
            "2_" => assert_eq!(file_content, third),
            _ => assert_eq!(0, 1),
        }
    }

    common::teardown();
}
