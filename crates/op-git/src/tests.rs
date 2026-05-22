//! Integration tests — each runs against a throwaway repository
//! created with the real system `git`.
//!
//! `git` is a hard requirement of this crate, but the test harness
//! still guards on its presence so the suite is skipped (not failed)
//! on a machine without `git` installed.
//!
//! This module is the spine: it holds the shared test fixtures
//! (`TempRepo`, `clone_for_test`, the `git_available` guard) used by
//! the per-topic sibling test modules (`tests_repo`, `tests_merge`,
//! `tests_auth`). Each test file is kept under the 800-line cap.

#![cfg(test)]

use std::path::PathBuf;
use std::process::Command;

use crate::GitRepo;

/// Whether the system `git` executable is usable.
pub(crate) fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Whether `ssh-keygen` is usable. `-?` is an unknown flag — it
/// prints usage and exits non-zero *without* reading stdin, so the
/// process simply running (an `Ok` output) proves availability.
pub(crate) fn ssh_keygen_available() -> bool {
    Command::new("ssh-keygen").arg("-?").output().is_ok()
}

/// A unique temp directory path for one test.
pub(crate) fn unique_temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("op-git-test-{tag}-{}-{nanos}", std::process::id()))
}

/// A throwaway repository with a configured commit identity, removed
/// from disk when dropped.
pub(crate) struct TempRepo {
    pub(crate) dir: PathBuf,
    pub(crate) repo: GitRepo,
}

impl TempRepo {
    /// Create a fresh initialized repo, or `None` when `git` is absent.
    pub(crate) fn new(tag: &str) -> Option<Self> {
        if !git_available() {
            return None;
        }
        let dir = unique_temp_dir(tag);
        let repo = GitRepo::init(&dir).expect("git init");
        // A hermetic identity so `git commit` never depends on the
        // host's global git config.
        repo.run(&["config", "user.email", "test@openpencil.dev"])
            .expect("set user.email");
        repo.run(&["config", "user.name", "OP Test"])
            .expect("set user.name");
        Some(Self { dir, repo })
    }

    /// Write `content` to `name` in the working tree.
    pub(crate) fn write(&self, name: &str, content: &str) {
        std::fs::write(self.dir.join(name), content).expect("write file");
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Clone the bare repo at `remote` into a fresh temp dir and give it
/// a hermetic commit identity. Returns the clone's dir + handle.
pub(crate) fn clone_for_test(remote: &std::path::Path, tag: &str) -> (PathBuf, GitRepo) {
    let dir = unique_temp_dir(tag);
    let repo = GitRepo::clone(remote.to_str().unwrap(), &dir).expect("clone");
    repo.run(&["config", "user.email", "t@openpencil.dev"]).unwrap();
    repo.run(&["config", "user.name", "OP Test"]).unwrap();
    (dir, repo)
}
