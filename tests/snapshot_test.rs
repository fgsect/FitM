use fitm::FITMSnapshot;
mod common;

use crate::common::teardown;
use std::time::Duration;

static SERVER_BIN: &str = "./tests/targets/pseudoserver_simple";
#[allow(dead_code)]
static CLIENT_BIN: &str = "./tests/targets/pseudoclient_simple";

#[test]
fn repeated_cmin_test_() {
    common::setup();

    let server0: FITMSnapshot = FITMSnapshot::new(
        1,
        0,
        SERVER_BIN.to_string(),
        Duration::from_secs(2),
        "".to_string(),
        true,
        false,
        None,
    );

    server0
        .init_run(false, true)
        .expect("[!] Init run on server0 failed");

    // =========== snapshot on gen1 =============
    let file_name = "tmp-input-0";
    std::fs::write(file_name, "R").expect("[!] Writing Test payload to tmp file failed");

    let input_path = std::path::Path::new(file_name)
        .canonicalize()
        .expect("[!] Could not canonicalize tmp-input path");

    let server1 = server0
        .create_next_snapshot(0, input_path.to_str().unwrap())
        .expect("[!] Create_next_snapshot for server0 failed")
        .unwrap();

    // =========== snapshot on gen3 =============
    let file_name = "tmp-input-1";
    std::fs::write(file_name, "ACK!").expect("[!] Writing Test payload to tmp file failed");

    let input_path = std::path::Path::new(file_name)
        .canonicalize()
        .expect("[!] Could not canonicalize tmp-input path");

    let _server2 = server1
        .create_next_snapshot(0, input_path.to_str().unwrap())
        .expect("[!] Create_next_snapshot for server0 failed");

    teardown();
}
