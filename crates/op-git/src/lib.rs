//! Git-backed version control for OpenPencil documents.
//!
//! This crate is the Rust counterpart of the TS Electron app's
//! in-app Git (`apps/desktop/git/`). EVERY operation runs through
//! in-process **libgit2** (`git2`, vendored), so the shipped binary
//! carries its own git engine — there is no `std::process::Command`
//! and no dependency on a system `git` executable at runtime. (The
//! former subprocess backend failed under macOS TCC when the child
//! process touched a sandboxed directory, and broke entirely on
//! machines without git installed.)
//!
//! ## Scope
//!
//! Repo lifecycle (discover / init / clone), working-tree status,
//! staging, commit, restore, branches, commit history / diff, remotes
//! and network ops (fetch / pull / push with credential callbacks), and
//! merge orchestration incl. worktree-isolated merges. Credential and
//! SSH-key storage live in `auth` / `ssh` (plain file I/O).
//!
//! Every operation returns a [`GitError`] on failure — a non-repo
//! path, or a libgit2 error mapped onto [`GitError::Command`] — and
//! never panics.

use std::path::{Path, PathBuf};

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
    ConflictBag, ConflictKind, ConflictStages, ConflictedFile, MergeOutcome, WorktreeMergeReport,
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

impl GitError {
    /// Stable `op-i18n` key for this error variant. `op-git` stays
    /// locale-free (no `op-i18n` dependency); the desktop host
    /// translates this key when surfacing the error in a dialog.
    /// The `Display` impl remains the English fallback.
    pub fn i18n_key(&self) -> &'static str {
        match self {
            GitError::GitNotFound => "git.gitError.notFound",
            GitError::NotARepo(_) => "git.gitError.notARepo",
            GitError::Command { .. } => "git.gitError.commandFailed",
            GitError::MergeInProgress => "git.gitError.mergeInProgress",
            GitError::WorkingTreeDirty => "git.gitError.workingTreeDirty",
            GitError::Io(_) => "git.gitError.io",
        }
    }

    /// The variant's technical detail — the repository path, the
    /// failing command's stderr, or the IO error text. Empty for
    /// variants that carry no payload. The host substitutes this
    /// into the `{{detail}}` slot of the translated message so the
    /// localized dialog still shows the actionable git output.
    pub fn i18n_detail(&self) -> String {
        match self {
            GitError::NotARepo(path) => path.display().to_string(),
            // Keep the failing subcommand — `commit` / `push` / … —
            // alongside its stderr so the dialog stays actionable.
            GitError::Command { operation, stderr } => {
                format!("git {operation}: {stderr}")
            }
            GitError::Io(text) => text.clone(),
            GitError::GitNotFound | GitError::MergeInProgress | GitError::WorkingTreeDirty => {
                String::new()
            }
        }
    }
}

/// A libgit2 error maps onto the existing [`GitError::Command`] variant
/// (a non-zero op with its message) so the public error surface — the
/// host's `i18n_key` matches + dialogs — stays unchanged across the
/// subprocess → libgit2 migration.
impl From<git2::Error> for GitError {
    fn from(e: git2::Error) -> Self {
        GitError::Command {
            operation: "libgit2".to_string(),
            stderr: e.message().to_string(),
        }
    }
}

/// A handle to a git repository — its working-tree root directory.
#[derive(Debug, Clone)]
pub struct GitRepo {
    workdir: PathBuf,
    /// Extra environment applied to every `git` invocation — set by
    /// [`GitRepo::with_auth_env`] so a network op (pull / push /
    /// fetch) authenticates with a stored credential or SSH key.
    /// Empty for an un-authenticated handle (git then uses its
    /// ambient credential helpers / ssh-agent).
    auth_env: Vec<(String, String)>,
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
    file.write_all(bytes)
        .map_err(|e| GitError::Io(e.to_string()))?;
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
        match git2::Repository::discover(probe) {
            Ok(repo) => Ok(Some(GitRepo {
                // The work-tree root. A bare repo has none — fall back to
                // the `.git` path so the handle is still well-formed.
                workdir: repo
                    .workdir()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| repo.path().to_path_buf()),
                auth_env: Vec::new(),
            })),
            // Not inside any repository is a normal state, not an error.
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Initialize a repository at `dir` (creating `dir` if needed) and
    /// return a handle to it. The initial branch is named `main`.
    pub fn init(dir: &Path) -> Result<GitRepo, GitError> {
        std::fs::create_dir_all(dir).map_err(|e| GitError::Io(e.to_string()))?;
        let mut opts = git2::RepositoryInitOptions::new();
        opts.initial_head("main");
        git2::Repository::init_opts(dir, &opts)?;
        GitRepo::discover(dir)?.ok_or_else(|| GitError::NotARepo(dir.to_path_buf()))
    }

    /// The repository's working-tree root.
    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    /// Open the in-process libgit2 handle for this repository. Opening
    /// is cheap (it just reads `.git`), so every operation opens fresh —
    /// keeping [`GitRepo`] itself a plain `Clone + Send` `{workdir,
    /// auth_env}` that the background pull / push / clone jobs can move
    /// across threads (a `git2::Repository` is neither `Clone` nor `Send`).
    pub(crate) fn open(&self) -> Result<git2::Repository, GitError> {
        git2::Repository::open(&self.workdir).map_err(Into::into)
    }

    /// A handle to the same repository whose `git` invocations carry
    /// `env` — set by [`GitRepo::auth_env`] so a network op runs with
    /// a stored credential / SSH key. An empty `env` is a no-op.
    pub fn with_auth_env(&self, env: Vec<(String, String)>) -> GitRepo {
        GitRepo {
            workdir: self.workdir.clone(),
            auth_env: env,
        }
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

    /// Read a single git config value (repo config, falling back to the
    /// global/system config the way `git config --get` does), or `None`
    /// when it is unset.
    fn config_get(&self, key: &str) -> Option<String> {
        let repo = self.open().ok()?;
        let cfg = repo.config().ok()?;
        // `Config::get_string` already walks repo → global → system.
        cfg.get_string(key).ok().filter(|v| !v.is_empty())
    }

    /// Test-only porcelain escape hatch: run `git <args>` in this repo
    /// and return trimmed stdout. Used ONLY by the integration-test
    /// fixtures to *set up* repositories (seed commits, branches,
    /// remotes) the quick way; the shipped library is pure libgit2 with
    /// no subprocess, so this is gated out of every non-test build.
    #[cfg(test)]
    pub(crate) fn run(&self, args: &[&str]) -> Result<String, GitError> {
        let output = std::process::Command::new("git")
            .current_dir(&self.workdir)
            .args(args)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    GitError::GitNotFound
                } else {
                    GitError::Io(e.to_string())
                }
            })?;
        if !output.status.success() {
            return Err(GitError::Command {
                operation: args.first().copied().unwrap_or("git").to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

// Integration tests — `tests` is the spine (shared fixtures); the
// per-topic test files are flat sibling modules, each under the
// 800-line file cap.
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_auth;
#[cfg(test)]
mod tests_merge;
#[cfg(test)]
mod tests_repo;
