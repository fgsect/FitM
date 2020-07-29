use fitm::AFLRun;
mod common;

use std::env;

#[test]
fn init_run_test() {
    common::setup();
    println!("{:?}", env::current_dir().unwrap());

    let mut afl_client: AFLRun = AFLRun::new(
        (1, 0),
        "tests/targets/pseudoclient".to_string(),
        1,
        "".to_string(),
        "".to_string(),
        false,
        false,
    );
    afl_client.init_run();

    assert_eq!(4, 2 + 2);
    assert_ne!(5, 2 + 2);

    common::teardown();
}
