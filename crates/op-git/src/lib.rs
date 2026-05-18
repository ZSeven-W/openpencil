//! System-`git`-backed version control for OpenPencil documents.
//!
//! This crate is the Rust counterpart of the TS Electron app's
//! in-app Git (`apps/desktop/git/`). It drives the user's installed
//! `git` executable through `std::process::Command` — the same
//! approach as the TS `git-sys.ts` backend — so no `libgit2` /
//! `git2` C dependency is pulled in.
//!
//! ## Scope
//!
//! The full TS surface spans repo lifecycle, branches, history,
//! remotes, merge orchestration, worktree merges, auth + SSH keys.
//! This module is the **foundation layer**: repo discovery / init,
//! working-tree status, staging, commit, restore, branch list /
//! create / delete / switch, and commit history / diff. Remote
//! operations, merge orchestration and credential handling land in
//! sibling modules in later increments.
//!
//! Every operation returns a [`GitError`] on failure — a missing
//! `git`, a non-repo path, or a non-zero `git` exit (with its
//! stderr) — and never panics.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod auth;
mod branch;
mod history;
mod merge;
mod remote;
mod ssh;
mod status;
mod worktree;

pub use auth::{AuthStore, Credential};
pub use branch::Branch;
pub use history::Commit;
pub use merge::{
    ConflictBag, ConflictKind, ConflictedFile, MergeOutcome, WorktreeMergeReport,
};
pub use remote::Remote;
pub use ssh::{SshKey, SshKeyStore};
pub use status::{ChangeState, FileStatus, RepoStatus};

/// An error from a git operation.
#[derive(Debug, thiserror::Error)]
pub enum GitError {
    /// The `git` executable could not be found on `PATH`.
    #[error("the `git` executable was not found — install Git to use version control")]
    GitNotFound,
    /// The path is not inside a git repository.
    #[error("not a git repository: {0}")]
    NotARepo(PathBuf),
    /// `git` ran but exited non-zero.
    #[error("git {operation} failed: {stderr}")]
    Command {
        /// The git subcommand that failed (`status`, `commit`, …).
        operation: String,
        /// Trimmed stderr from the failed invocation.
        stderr: String,
    },
    /// An operation that needs a clean state was attempted while a
    /// merge is still in progress (unresolved or uncommitted).
    #[error("a merge is already in progress — resolve or abort it first")]
    MergeInProgress,
    /// A merge was attempted while the working tree had uncommitted
    /// changes — the caller must commit or stash them first.
    #[error("the working tree has uncommitted changes — commit or stash them before merging")]
    WorkingTreeDirty,
    /// Spawning / waiting on the `git` process failed.
    #[error("git process error: {0}")]
    Io(String),
}

/// A handle to a git repository — its working-tree root directory.
#[derive(Debug, Clone)]
pub struct GitRepo {
    workdir: PathBuf,
}

/// The committer identity git would stamp on a new commit, read
/// from git config (repo, then global). Either field is `None` when
/// git config does not set it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Author {
    /// `user.name`.
    pub name: Option<String>,
    /// `user.email`.
    pub email: Option<String>,
}

/// Run `git <args>` in `dir`, mapping a missing executable to
/// [`GitError::GitNotFound`].
pub(crate) fn git_output(dir: &Path, args: &[&str]) -> Result<Output, GitError> {
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GitError::GitNotFound
            } else {
                GitError::Io(e.to_string())
            }
        })
}

/// Trimmed stderr text of a failed invocation.
pub(crate) fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).trim().to_string()
}

/// Write `bytes` to `path` for a file that holds secret material
/// (a credential store, a private key).
///
/// On Unix the file is created with owner-only (`0600`) permissions
/// *before* any content is written — so the secret is never, even
/// momentarily, world-readable. A naive `fs::write` followed by a
/// `chmod` leaves exactly that window open. A stale file at `path`
/// is removed first so the fresh file is created with the strict
/// mode rather than inheriting a looser one.
pub(crate) fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), GitError> {
    use std::io::Write;
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(GitError::Io(e.to_string())),
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|e| GitError::Io(e.to_string()))?;
    file.write_all(bytes).map_err(|e| GitError::Io(e.to_string()))?;
    Ok(())
}

impl GitRepo {
    /// Discover the git repository that contains `path` (a document
    /// file or a directory). Returns `Ok(None)` when `path` is not
    /// inside any repository — that is a normal state, not an error.
    pub fn discover(path: &Path) -> Result<Option<GitRepo>, GitError> {
        // A file's repo is found by probing its parent directory.
        let probe = if path.is_file() {
            path.parent().unwrap_or(path)
        } else {
            path
        };
        if !probe.exists() {
            return Ok(None);
        }
        let output = git_output(probe, &["rev-parse", "--show-toplevel"])?;
        if !output.status.success() {
            // git exits non-zero outside a work tree — "no repo here".
            return Ok(None);
        }
        let top = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if top.is_empty() {
            return Ok(None);
        }
        Ok(Some(GitRepo {
            workdir: PathBuf::from(top),
        }))
    }

    /// `git init` a repository at `dir` (creating `dir` if needed)
    /// and return a handle to it. The initial branch is named `main`.
    pub fn init(dir: &Path) -> Result<GitRepo, GitError> {
        std::fs::create_dir_all(dir).map_err(|e| GitError::Io(e.to_string()))?;
        let output = git_output(dir, &["init", "--initial-branch=main"])?;
        if !output.status.success() {
            return Err(GitError::Command {
                operation: "init".to_string(),
                stderr: stderr_of(&output),
            });
        }
        GitRepo::discover(dir)?.ok_or_else(|| GitError::NotARepo(dir.to_path_buf()))
    }

    /// The repository's working-tree root.
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// The committer identity git would use here — `user.name` /
    /// `user.email` resolved through the repo + global config. An
    /// unset field is `None`; this never errors (a missing `git`
    /// just yields an empty [`Author`]).
    pub fn author(&self) -> Author {
        Author {
            name: self.config_get("user.name"),
            email: self.config_get("user.email"),
        }
    }

    /// Read a single git config value, or `None` when it is unset.
    fn config_get(&self, key: &str) -> Option<String> {
        let output = git_output(&self.workdir, &["config", "--get", key]).ok()?;
        if !output.status.success() {
            return None;
        }
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!value.is_empty()).then_some(value)
    }

    /// Run `git <args>` in this repo, returning trimmed stdout on a
    /// zero exit. Shared by every operation in the sibling modules.
    pub(crate) fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let output = git_output(&self.workdir, args)?;
        if !output.status.success() {
            return Err(GitError::Command {
                operation: args.first().copied().unwrap_or("git").to_string(),
                stderr: stderr_of(&output),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[cfg(test)]
mod tests;
