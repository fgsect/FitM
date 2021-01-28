use crate::{FITMSnapshot, ACTIVE_STATE, CRIU_STDERR, CRIU_STDOUT};

use fs_extra::{self, dir::CopyOptions};
use json;
use std::{
    cmp::{max, min},
    fs::{self, create_dir_all},
    io::{self, ErrorKind, Write},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    str::FromStr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub fn pick_random<T>(rand: &mut RomuRand, input_vec: &[T], count: usize) -> Vec<T>
where
    T: Clone,
{
    let mut output_idx: Vec<usize> = Vec::new();
    // let mut output_vec: Vec<T> = vec![];

    if input_vec.len() <= count {
        return Vec::from(input_vec);
    }

    'genrand: loop {
        let rand_index = rand.below(input_vec.len() as _) as usize;
        for elem in output_idx.iter() {
            if rand_index == *elem {
                continue 'genrand;
            }
        }

        output_idx.push(rand_index as _);
        if output_idx.len() == count {
            break;
        }
    }
    output_idx.sort();
    output_idx
        .into_iter()
        .map(|x| input_vec[x].clone())
        .collect()
}

pub fn clear_out() {
    std::fs::remove_dir_all("out")
        .expect("[!] Could not remove old 'out' folder in utils::clear_out");
    create_dir_all("out").expect("[!] Could not recreate out folder in utils::clear_out");
}

pub fn parse_pid() -> io::Result<i32> {
    let pstree = Command::new("./criu/crit/crit-python3")
        .args(&[
            "decode".to_string(),
            "-i".to_string(),
            format!("{}/snapshot/pstree.img", ACTIVE_STATE),
        ])
        .output()
        .expect("[!] crit decode failed during utils::parse_pid");
    let pstree_string = String::from_utf8(pstree.stdout)
        .expect("[!] Failed to grab output from crit in utils::parse_pid");
    let pstree_json = json::parse(pstree_string.as_str()).unwrap();
    let pid = pstree_json["entries"][0]["pid"]
        .as_i32()
        .expect("[!] Could not transform json value into i32 in utils::parse_pid");
    Ok(pid.into())
}

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
        .expect("Writing failed (higher than /proc/sys/kernel/pid_max?)");
}

pub fn waitpid(snapshot_pid: libc::pid_t) -> io::Result<ExitStatus> {
    let mut status = 0 as libc::c_int;
    loop {
        let result = unsafe { libc::waitpid(snapshot_pid, &mut status, 0) };
        if result == -1 {
            let e = io::Error::last_os_error();
            if e.kind() != io::ErrorKind::Interrupted {
                return Err(e);
            }
        } else {
            break;
        }
    }
    // Casting the waitpid return value to automatically interpret ExitStatus flags
    Ok(unsafe { std::mem::transmute(status) })
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

pub fn get_filesize(path: &PathBuf) -> u64 {
    let metadata = fs::metadata(path)
        .expect("[!] Could not grab metadata for cur_file in utils::get_filesize");
    metadata.len()
}

/// Gets current nanoseconds since UNIX_EPOCH
#[inline]
pub fn current_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}

/// Gets current milliseconds since UNIX_EPOCH
#[inline]
pub fn current_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// see https://arxiv.org/pdf/2002.11331.pdf
#[derive(Copy, Clone, Debug, Default)]
pub struct RomuRand {
    x_state: u64,
    y_state: u64,
}

impl RomuRand {
    pub fn new(seed: u64) -> Self {
        let mut rand = Self::default();
        rand.set_seed(seed);
        rand
    }

    /// Creates a rand instance, pre-seeded with the current time in nanoseconds.
    /// Needs stdlib timer
    // #[cfg(feature = "std")]
    pub fn preseeded() -> Self {
        Self::new(current_nanos())
    }

    fn set_seed(&mut self, seed: u64) {
        self.x_state = seed ^ 0x12345;
        self.y_state = seed ^ 0x6789A;
    }

    #[inline]
    fn next(&mut self) -> u64 {
        let xp = self.x_state;
        self.x_state = 15241094284759029579u64.wrapping_mul(self.y_state);
        self.y_state = self.y_state.wrapping_sub(xp).rotate_left(27);
        xp
    }

    // Gets a value below the given 64 bit val (inclusive)
    pub fn below(&mut self, upper_bound_excl: u64) -> u64 {
        if upper_bound_excl <= 1 {
            return 0;
        }

        /*
        Modulo is biased - we don't want our fuzzing to be biased so let's do it
        right. See
        https://stackoverflow.com/questions/10984974/why-do-people-say-there-is-modulo-bias-when-using-a-random-number-generator
        */
        let mut unbiased_rnd: u64;
        loop {
            unbiased_rnd = self.next();
            if unbiased_rnd < (u64::MAX - (u64::MAX % upper_bound_excl)) {
                break;
            }
        }

        unbiased_rnd % upper_bound_excl
    }
}

/// The similarity of this output. 0 -> not similar, 1.0 -> very.
/// As improvement to JARO, we ignore very different lengths.
pub fn output_similarity(this_output: &[u8], other_output: &[u8]) -> f64 {
    let this_len = this_output.len();
    let other_len = other_output.len();

    if this_len > other_len * 2
        || this_len * 2 > other_len
        || (this_len > 512 && this_len != other_len)
    {
        return 0.0;
    }

    jaro(this_output, other_output)
}

/// Calculates the Jaro similarity between two strings. The returned value
/// is between 0.0 and 1.0 (higher value means more similar).
///
/// ```
/// use strsim::jaro;
///
/// assert!((0.392 - jaro("Friedrich Nietzsche", "Jean-Paul Sartre")).abs() <
///         0.001);
/// ```
///
pub fn jaro(a: &[u8], b: &[u8]) -> f64 {
    /*
    This is largely copied from
    https://nicolasdp.github.io/git/src/strsim/lib.rs.html
    Slightly patched to work on &[u8] instead of str.

    LICENSE:

    The MIT License (MIT)

    Copyright (c) 2015 Danny Guo
    Copyright (c) 2016 Titus Wormer <tituswormer@gmail.com>
    Copyright (c) 2018 Akash Kurdekar

    Permission is hereby granted, free of charge, to any person obtaining a copy
    of this software and associated documentation files (the "Software"), to deal
    in the Software without restriction, including without limitation the rights
    to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
    copies of the Software, and to permit persons to whom the Software is
    furnished to do so, subject to the following conditions:

    The above copyright notice and this permission notice shall be included in all
    copies or substantial portions of the Software.

    THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
    IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
    FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
    AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
    LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
    OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
    SOFTWARE.
    */
    if a == b {
        return 1.0;
    }

    let a_len = a.len();
    let b_len = b.len();
    if a_len == 0 && b_len == 0 {
        return 0.0;
    } else if a_len == 0 || b_len == 0 {
        return 0.0;
    } else if a_len == 1 && b_len == 1 && a[0] == b[0] {
        return 1.0;
    }

    let search_range = max(0, (max(a_len, b_len) / 2) - 1);

    let mut b_consumed = vec![false; b_len];

    let mut matches = 0.0;

    let mut transpositions = 0.0;
    let mut b_match_index = 0;

    for (i, a_char) in a.iter().enumerate() {
        let min_bound =
            // prevent integer wrapping
            if i > search_range {
                max(0, i - search_range)
            } else {
                0
            };

        let max_bound = min(b_len - 1, i + search_range);

        if min_bound > max_bound {
            continue;
        }

        for (j, b_char) in b.iter().enumerate() {
            if min_bound <= j && j <= max_bound && a_char == b_char && !b_consumed[j] {
                b_consumed[j] = true;
                matches += 1.0;

                if j < b_match_index {
                    transpositions += 1.0;
                }
                b_match_index = j;

                break;
            }
        }
    }

    let ret = if matches == 0.0 {
        0.0
    } else {
        (1.0 / 3.0)
            * ((matches / a_len as f64)
                + (matches / b_len as f64)
                + ((matches - transpositions) / matches))
    };

    //println!("JARO was {} for ({:?} <-> {:?})", ret, &a, &b);
    ret
}

#[cfg(test)]
mod tests {
    use crate::utils;
    use crate::utils::{latest_snapshot_time, parse_pid, pick_random, RomuRand};
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
    fn test_pick_random() {
        let mut rand = RomuRand::preseeded();
        let random_from_ten = pick_random(&mut rand, &vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9], 3);
        println!("Got {:?} from a range of 0..9", random_from_ten);
    }

    #[test]
    fn test_parse_pid() {
        println!("{:?}", parse_pid().unwrap());
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
