use fitm::{AFLRun, origin_state, ORIGIN_STATE_TUPLE};
use std::fs;

mod common;

// init_run_test should check if a snapshot could be successfully be created.
// As the test does not have access to criu server responses or other logs it
// relies on the correct creation of various files

/*

    fitm-client -> snapshot() at initial recv
        --> outputs send stuff
    fitm-server -> snapshot() at initial recv (server should not send earlier (for now))

    fuzz fitm-server -> c0s1(fitm-client[send stuff])
        --> outputs c0s1stuff[testcase][u8]

    for testcase in c0s1stuff
        fuzz fitm-client -> c1s1(c0s1[testcase])
    
    fitm-client: origin_state(client)
    fitm-server: origin_state(server), necessary for criu right now
        - server_run0 (c0s1)
            - client_run0 (c1s1)
                - server_run0 (c1s2)
                - server_run1 (c1s3) < base state here <<----.
                    - client_run0 (c2s3)                     |
                        - server_run0 (c2s5)  ---------------^ << counter for c, s are global
                    - client_run1 (c3s3)
                - server_run2 (c1s4)
                    - client_run1 (c2s4)

    numbers are continouus

    Scripted client, wants to CWD, DELE, MODE

    FTP Example
    Base snapshot: 
    fitm-client: sent CWD, rady to recv
    fitm-server: ready to recv

    step 1: fuzz the server (fitm-server).
    Client => CWD
    server: CWD, CWX, DWD, FXX, PORT, ...
    if new testcase: snapshot(c0s1..c0sn)

    step 2: fuzz the client (fitm-client).
    Server => [CWD, PORT]
    client: Unknown command: XOXO -> DELE, CWD with what it expected -> PLZ send file, PORT -> Exit

    step 3: fuzz all servers (c0s1)


    */
#[test]
fn create_outputs_test() {
    // pwd == root dir of repo
    common::setup();

    // creating the afl_client object manually would make the test even more
    // precise
    let afl_client: AFLRun = AFLRun::new(
        ORIGIN_STATE_TUPLE,
        "tests/targets/echo_server".to_string(),
        1,
        origin_state(true).to_string(),
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
    fs::write("./saved-states/fitm-server/outputs/first_case.txt", first)
        .expect("Could not write first input file");
    fs::write("./saved-states/fitm-server/outputs/second_case.txt", second)
        .expect("Could not write second input file");
    fs::write("./saved-states/fitm-server/outputs/third_case.txt", third)
        .expect("Could not write third input file");

    // tested function
    afl_client.gen_afl_maps().expect("Couldn't generate afl maps");

    // break here and inspect `active-state/stdout-afl` to see breaking
    // forkserver
    common::teardown();
}
