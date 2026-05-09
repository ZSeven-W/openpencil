//! git build helper.
#![allow(dead_code)]

use std::path::Path;
use std::process::{Command, Stdio};

const HALF_HASH_LENGTH: usize = 20;

/// Returns a 20 digit hash of the repository located in the current directory.
pub fn half_hash() -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.arg("rev-parse").arg("--short=20");
    let output = cmd.arg("HEAD").stderr(Stdio::inherit()).output().ok()?;
    if output.status.code() != Some(0) {
        None
    } else {
        // need to trim the string to remove newlines at the end.
        Some(String::from_utf8(output.stdout).unwrap().trim().to_string())
    }
}

pub fn trim_hash(hash: &str) -> String {
    hash[..HALF_HASH_LENGTH].into()
}

/// Run git with the given args in the given directory, print stderr to the current
/// process's terminal, and capture its stdout output.
/// Panics if the git command fails.
pub fn run<'a>(args: &[impl AsRef<str>], dir: impl Into<Option<&'a Path>>) -> Vec<u8> {
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();
    let (status, output) = _run2(&args, dir);
    if status == 0 {
        return output;
    }
    panic!("GIT command failed: git {}", args.join(" "));
}

/// Like run, but returns the status code _and_ the output or None if
/// there is no status code (for example the command was interrupted).
/// Panics if the git command could not be run at all.
pub fn _run2<'a>(args: &[impl AsRef<str>], dir: impl Into<Option<&'a Path>>) -> (i32, Vec<u8>) {
    let args: Vec<&str> = args.iter().map(|s| s.as_ref()).collect();

    let mut cmd = Command::new("git");
    cmd.args(&args).stderr(Stdio::inherit());

    if let Some(dir) = dir.into() {
        cmd.current_dir(dir);
    }

    let output = cmd.output().expect("running git failed, is it in PATH?");
    let status = output.status.code().expect("git command terminated");
    (status, output.stdout)
}
