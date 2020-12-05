use fitm::{origin_state, AFLRun, ORIGIN_STATE_TUPLE};
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
fn gen_afl_maps_test() {
    // pwd == root dir of repo
    common::setup();

    // creating the afl_client object manually would make the test even more
    // precise
    let afl_server: AFLRun = AFLRun::new(
        ORIGIN_STATE_TUPLE,
        "tests/targets/echo_server".to_string(),
        1,
        origin_state(true).to_string(),
        "".to_string(),
        true,
        false,
    );

    afl_server.init_run();

    std::fs::remove_dir_all(format!("active-state/{}", &afl_server.state_path)).expect(
        format!(
            "Could not remove '{}' in gen_afl_maps_test",
            &afl_server.state_path
        )
        .as_str(),
    );

    // populate in folder here
    let first = "a simple string";
    let second = "message 1, upcoming linebreak now:\nmessage 2";
    let third = "foo\tbar";
    fs::create_dir_all("./saved-states/fitm-server/out/main/queue/")
        .expect("Could not create queue folder");
    fs::write(
        "./saved-states/fitm-server/out/main/queue/first_case.txt",
        first,
    )
    .expect("Could not write first input file");
    fs::write(
        "./saved-states/fitm-server/out/main/queue/second_case.txt",
        second,
    )
    .expect("Could not write second input file");
    fs::write(
        "./saved-states/fitm-server/out/main/queue/third_case.txt",
        third,
    )
    .expect("Could not write third input file");

    // tested function
    afl_server
        .gen_afl_maps()
        .expect("Couldn't generate afl maps");

    let map1 = fs::read_to_string("./active-state/fitm-server/out/maps/first_case.txt");
    let map2 = fs::read_to_string("./active-state/fitm-server/out/maps/second_case.txt");
    let map3 = fs::read_to_string("./active-state/fitm-server/out/maps/third_case.txt");

    // Can't check for exact content since the addresses change depending on the compiler/architecture used for building the tested binary
    assert!(map1.unwrap().contains(":"));
    assert!(map2.unwrap().contains(":"));
    assert!(map3.unwrap().contains(":"));

    // break here and inspect `active-state/stdout-afl` to see breaking
    // forkserver
    common::teardown();
}
