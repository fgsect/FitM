use fs_extra::dir::CopyOptions;
use std::fs;
use std::process::Command;

use crate::AFLRun;

pub fn mv(from: String, to: String) {
    Command::new("mv")
        .args(&[from.clone(), to.clone()])
        .spawn()
        .expect("[!] Could not start moving dirs")
        .wait()
        .expect(
            format!("[!] Moving dir failed To: {} From: {}", to, from).as_str(),
        );
}

pub fn copy(from: String, to: String) {
    Command::new("cp")
        .args(&[String::from("-r"), from.clone(), to.clone()])
        .spawn()
        .expect("[!] Could not start copying dirs")
        .wait()
        .expect(
            format!("[!] Copying dir failed To: {} From: {}", to, from)
                .as_str(),
        );
}

pub fn rm(dir: String) {
    Command::new("rm")
        .args(&["-rf", dir.clone().as_str()])
        .spawn()
        .expect("[!] Could not start removing dir/file")
        .wait()
        .expect(format!("[!] Removing dir/file {} failed.", dir).as_str());
}

pub fn copy_snapshot_base(base_state: &String, state_path: &String) -> () {
    // copy old snapshot folder for criu
    let old_snapshot = format!("./saved-states/{}/snapshot", base_state);
    let new_snapshot = format!("./active-state/{}/", state_path);

    // Check fs_extra docs for different copy options
    let options = CopyOptions::new();
    fs_extra::dir::copy(old_snapshot, new_snapshot, &options)
        .expect("[!] Could not copy snapshot dir from previous state");

    // copy old pipes file so restore.sh knows which pipes are open
    let old_pipes = format!("./saved-states/{}/pipes", base_state);
    let new_pipes = format!("./active-state/{}/pipes", state_path);
    fs::copy(old_pipes, new_pipes)
        .expect("[!] Could not copy old pipes file to new state-dir");
}

pub fn create_restore_sh(afl: &AFLRun) {
    let _ = Command::new("python3")
        .args(&[
            "create_restore.py".to_string(),
            afl.base_state.to_string(),
            afl.state_path.to_string(),
        ])
        .spawn()
        .expect("[!] Could not spawn create_restore.py")
        .wait()
        .expect("[!] Could not create restore.sh with python");
}

/// Create the next iteration from a given state directory. If inc_server is set
/// we will increment the state for the server from fitm-cXsY to fitm-cXsY+1.
/// Otherwise we will increment the state for the client from fitm-cXsY to
/// fitm-cX+1sY
pub fn next_state_path(
    state_path: (u32, u32),
    cur_is_server: bool,
) -> (u32, u32) {
    // If inc_server increment the server state else increment the client state
    if cur_is_server {
        ((state_path.0) + 1, state_path.1)
    } else {
        (state_path.0, (state_path.1) + 1)
    }
}
