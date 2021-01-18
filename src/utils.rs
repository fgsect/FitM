use fs_extra;
use fs_extra::dir::CopyOptions;
use std::fs;
use std::process::{Child, Command, Stdio};

use crate::{FITMSnapshot, ACTIVE_STATE, CRIU_STDERR, CRIU_STDOUT, TARGET_EXITCODE};

use std::io::{self, ErrorKind, Read, Write};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

pub fn mv(from: &str, to: &str) {
    let options = CopyOptions::new();
    fs_extra::dir::move_dir(&from, &to, &options)
        .expect(format!("utils::mv failed to move '{}' to '{}'", from, to).as_str());
}

pub fn mv_rename(from: &str, to: &str) {
    cp_recursive(from, to);

    match std::fs::remove_dir_all(from) {
        Ok(_) => (),
        Err(e) if e.kind() == ErrorKind::Other => {
            // retry since this usually is a problem within remove_dir_all
            std::fs::remove_dir_all(from)
                .expect("[!] Error while calling remove_dir_all() again in utils:mv_rename");
        }
        Err(e) => println!(
            "[!] Could not remove '{}' in utils::mv_rename: {:?}",
            from, e
        ),
    };
}

pub fn copy(from: &str, to: &str) {
    let options = CopyOptions::new();
    fs_extra::dir::copy(&from, &to, &options)
        .expect(format!("utils::copy failed to copy '{}' to '{}'", from, to).as_str());
}

pub fn cp_recursive(from: &str, to: &str) {
    // std::fs::rename can not rename filled dirs -.-

    // preserve is needed because otherwise file permissions change through copying
    Command::new("cp")
        .args(&["--preserve", "-r", from, to])
        .spawn()
        .expect("[!] Could not spawn cp cmd")
        .wait()
        .expect("[!] Failed to wait for cp");

    Command::new("sync").status().unwrap();
}

pub fn copy_overwrite(from: &str, to: &str) {
    let mut options = CopyOptions::new();
    options.overwrite = true;
    fs_extra::dir::copy(&from, &to, &options)
        .expect(format!("utils::copy failed to copy '{}' to '{}'", from, to).as_str());
}

pub fn copy_ignore(from: &str, to: &str) {
    let options = CopyOptions::new();
    match fs_extra::dir::copy(&from, &to, &options) {
        Err(e) => println!("Ignored error in copy: {:?}", e),
        _ => (),
    }
}

//#[allow(dead_code)]
pub fn rm(dir: &str) {
    Command::new("rm")
        .args(&["-rf", dir])
        .spawn()
        .expect("[!] Could not start removing dir/file")
        .wait()
        .expect(format!("[!] Removing dir/file {} failed.", dir).as_str());
}

fn cp_stdfiles(base_state: &str) {
    // stdout
    fs::copy(
        format!("./saved-states/{}/stdout", base_state),
        format!("{}/stdout", ACTIVE_STATE),
    )
    .expect("[!] Could not copy old stdout file to active-state");

    // stderr
    fs::copy(
        format!("./saved-states/{}/stderr", base_state),
        format!("{}/stderr", ACTIVE_STATE),
    )
    .expect("[!] Could not copy old stdout file to active-state");
}

pub fn copy_snapshot_base(base_state: &str) -> () {
    // copy old snapshot folder for criu
    let old_snapshot = format!("./saved-states/{}/snapshot", base_state);
    let new_snapshot = format!("{}", ACTIVE_STATE);

    cp_recursive(old_snapshot.as_str(), new_snapshot.as_str());

    // copy old pipes file so restore.sh knows which pipes are open
    let old_pipes = format!("./saved-states/{}/pipes", base_state);
    let new_pipes = format!("{}/pipes", ACTIVE_STATE);
    fs::copy(old_pipes, new_pipes).expect("[!] Could not copy old pipes file to new state-dir");

    // copy old fd folder for new state
    let from = format!("./saved-states/{}/fd", base_state);
    let to = format!("{}", ACTIVE_STATE);
    copy(&from, &to);

    // copy old stdout/err since they are part of the process' state
    cp_stdfiles(base_state);
}

pub fn create_restore_sh(afl: &FITMSnapshot) {
    Command::new("python3")
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
pub fn next_state_path(state_path: (u32, u32), cur_is_server: bool) -> (u32, u32) {
    // If inc_server increment the server state else increment the client state
    if cur_is_server {
        ((state_path.0) + 1, state_path.1)
    } else {
        (state_path.0, (state_path.1) + 1)
    }
}

/// @param snapshot_dir: str of path pointing to a dir with depth 1
/// @return: the most recent timestamp of a successfull criu worker exiting in the criu server log
pub fn latest_snapshot_time(criu_stderr: &str) -> f64 {
    let mut timestamp_cleaned = "0";
    let server_log =
        fs::read_to_string(criu_stderr).expect("[!] Could not read criu_stderr in count_snapshots");
    let lines: Vec<&str> = server_log.split("\n").collect();
    for line in lines {
        // timestamp has constant length - remove it
        let splits: Vec<&str> = line.split(" ").collect();
        // Relevant lines look like this: "(00.055739) Worker(pid 43750) exited with 0"
        if splits.contains(&"Worker(pid") {
            if splits.last().unwrap() == &"0" {
                let timestamp = splits.first().unwrap();
                let timestamp_cleaned_new = timestamp.trim_start_matches("(").trim_end_matches(")");
                if timestamp_cleaned_new > timestamp_cleaned {
                    timestamp_cleaned = timestamp_cleaned_new;
                }
            } else {
                panic!("[!] Criu server failed to create new snapshot. Check active-state dir.")
            }
        }
    }
    f64::from_str(timestamp_cleaned).expect("[!] Error parsing timestamp str to float")
}

/// @return: a boolean indicating if there is a positivie time difference between old and new
pub fn positive_time_diff(old: &SystemTime, new: &SystemTime) -> bool {
    let diff = new
        .duration_since(*old)
        .expect("[!] duration_since failed to retrieve duration. System clock may have drifted");
    println!("time diff: {:?}", diff);
    if diff > Duration::from_secs(0) {
        true
    } else {
        false
    }
}

/// Sets the PID-counter to a specific target
/// Assumes no other processes are concurrently spawning/accessing PID-counter
/// The generated PIDs are not checked against target
pub fn advance_pid(target: u64) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("/proc/sys/kernel/ns_last_pid")
        .expect("Failed to open ns_last_pid");

    file.write((target - 1).to_string().as_bytes())
        .expect("Writing failed");
}

pub fn spawn_criu(criu_path: &str, socket_path: &str) -> io::Result<Child> {
    let criu_stdout = fs::File::create(CRIU_STDOUT).expect("[!] Could not create criu_stdout");
    let criu_stderr = fs::File::create(CRIU_STDERR).expect("[!] Could not create criu_stderr");
    Command::new(criu_path)
        .args(&[
            format!("service"),
            format!("-v4"),
            format!("--address"),
            format!("{}", socket_path),
            format!("--display-stats"),
        ])
        .stdout(Stdio::from(criu_stdout))
        .stderr(Stdio::from(criu_stderr))
        .spawn()
}

pub fn read_exitcode() -> Result<i32, io::Error> {
    let mut exitcode_file = fs::File::open(format!("{}/{}", ACTIVE_STATE, TARGET_EXITCODE))
        .expect("[!] Could not open target-exitcode in utils::read_exitcode");
    let mut buffer = String::new();
    exitcode_file
        .read_to_string(&mut buffer)
        .expect("[!] Could not read target-exitcode in utils::read_exitcode");
    // Added this since rust seemed to read the null byte as a character(?) and broke on parsing
    buffer.retain(|c| c.is_digit(10));
    Ok(buffer
        .parse::<i32>()
        .expect("[!] Could not parse exitcode in utils::read_exitcode"))
}

#[cfg(test)]
mod tests {
    use crate::utils;
    use crate::utils::latest_snapshot_time;
    use std::fs;
    use std::path::Path;

    fn setup(root_folder: &str, from_path: &str, from_content_path: &str, content: &str) {
        // setup - require user interaction so we don't delete anything by
        // default Creates necessary files/folders under /tmp
        fs_extra::dir::create(root_folder, false)
            .expect("rust_unittest folder already exists, please remove to make this test run");
        fs_extra::dir::create_all(from_path, true).expect("Could not create test folder");
        fs::write(from_content_path, content).expect("Could not write to 'from' content.txt");
    }

    fn teardown(root_folder: &String) {
        // Remove all files created during the test
        std::fs::remove_dir_all(root_folder).expect("Could not remove rust_unittest folder");
    }

    fn paths_exist(root_folder: &String, to_content_path: &String) -> bool {
        // Returns true if all three of the given paths exists
        let bool_1 = Path::new(&format!("{}/foo", root_folder)).exists();
        let bool_2 = Path::new(&format!("{}/bar", root_folder)).exists();
        let bool_3 = Path::new(to_content_path).exists();
        bool_1 && bool_2 && bool_3
    }

    fn check_is_dir(to_path: &String) -> bool {
        // Returns true if the given path points to a directory
        let metadata = fs::metadata(to_path).expect("Could not find copy 'to' folder");
        metadata.file_type().is_dir()
    }

    #[test]
    fn test_latest_snapshot_time() {
        let count = latest_snapshot_time("criu_stderr");
        assert_eq!(
            count, 10.672444,
            "Update the expected value if you actually want to test the function"
        );
    }

    #[test]
    fn test_copy() {
        // Test whether utils::copy() copies recursively to a given path,
        // using the original folders name as target name
        let root_folder = String::from("/tmp/rust_unittest");
        let from_path = format!("{}/foo/bar", root_folder);
        let to_path = format!("{}", root_folder);
        let from_content_path = format!("{}/foo/bar/content.txt", root_folder);
        let to_content_path = format!("{}/bar/content.txt", root_folder);
        let content = "A simple string.";

        setup(&root_folder, &from_path, &from_content_path, content);

        // tested function
        utils::copy(&from_path, &root_folder);

        // Check that the 'from' path does not exist anymore, but the 'to' path
        // does
        assert_eq!(Path::new(&from_path).exists(), true);
        assert!(paths_exist(&root_folder, &to_content_path));

        // Check 'to' path is still a directory
        assert!(check_is_dir(&to_path));

        // Check that the content of the copied folder still exists
        let result_content = std::fs::read_to_string(to_content_path)
            .expect("Could not read from expected content.txt");

        assert_eq!(result_content, "A simple string.");

        // teardown
        teardown(&root_folder);
    }

    #[test]
    fn test_mv() {
        // Check that utils::mv moves a folder to a new destination
        let root_folder = String::from("/tmp/rust_unittest");
        let from_path = format!("{}/foo/bar", root_folder);
        let to_path = format!("{}", root_folder);
        let from_content_path = format!("{}/foo/bar/content.txt", root_folder);
        let to_content_path = format!("{}/bar/content.txt", root_folder);
        let content = "A simple string.";

        setup(&root_folder, &from_path, &from_content_path, content);

        // tested function
        utils::mv(&from_path, &to_path);

        // Check that the 'from' path does not exist anymore, but the 'to' path
        // does
        assert_eq!(Path::new(&from_path).exists(), false);
        assert!(paths_exist(&root_folder, &to_content_path));

        // Check 'to' path is still a directory
        assert!(check_is_dir(&to_path));

        // Check that the content of the copied folder still exists
        let result_content = std::fs::read_to_string(to_content_path)
            .expect("Could not read from expected content.txt");

        assert_eq!(result_content, "A simple string.");

        // teardown
        teardown(&root_folder);
    }

    #[test]
    fn test_remove_dir_all() {
        let root_folder = String::from("/tmp/rust_unittest");
        let path = format!("{}/foo/bar", root_folder);
        let content_path = format!("{}/foo/bar/content.txt", root_folder);
        let content = "A simple string.";

        setup(&root_folder, &path, &content_path, content);

        let foo_path = format!("{}/foo", root_folder);

        // tested function
        std::fs::remove_dir_all(&foo_path).expect("Tested remove_dir_all failed");

        assert_eq!(Path::new(&foo_path).exists(), false);

        teardown(&root_folder);
    }
}
