//! Git session — binds the open `.op` document to its repository.
//!
//! The Rust counterpart of the TS Electron app's `repo-session.ts`:
//! whenever the desktop opens / creates / saves-as a document, the
//! session re-detects whether that file lives inside a git
//! repository and, if so, binds an [`op_git::GitRepo`] handle plus
//! the tracked document path. The Git panel reads the session to
//! show branch / status / history for the open document and to
//! drive commits.
//!
//! An unsaved (pathless) document, or a document outside any git
//! repository, leaves the session simply unbound.

use std::path::{Path, PathBuf};

use op_git::{Commit, GitError, GitRepo, MergeOutcome, RepoStatus, WorktreeMergeReport};

/// The git repository bound to the currently-open document.
#[derive(Default)]
pub struct GitSession {
    /// The repository containing the open document, if any.
    repo: Option<GitRepo>,
    /// Absolute path of the tracked document.
    tracked_file: Option<PathBuf>,
}

impl GitSession {
    /// An unbound session.
    pub fn new() -> Self {
        Self::default()
    }

    /// Re-detect the repository for `path` — the document's path, or
    /// `None` for an unsaved document. Binds to the repo containing
    /// the file, or unbinds when there is no path / no repository.
    pub fn rebind(&mut self, path: Option<&Path>) {
        match path {
            Some(p) => {
                self.tracked_file = Some(p.to_path_buf());
                // A discovery error (e.g. `git` missing) leaves the
                // session unbound — version control is best-effort.
                self.repo = GitRepo::discover(p).ok().flatten();
            }
            None => {
                self.tracked_file = None;
                self.repo = None;
            }
        }
    }

    /// Current branch of the bound repository.
    pub fn current_branch(&self) -> Option<String> {
        self.repo
            .as_ref()
            .and_then(|r| r.current_branch().ok().flatten())
    }
}

/// Read / action surface consumed by the Git panel widget. Marked
/// `allow(dead_code)` until that panel increment wires it — the
/// methods are already covered by this module's tests.
#[allow(dead_code)]
impl GitSession {
    /// Whether a repository is bound.
    pub fn is_bound(&self) -> bool {
        self.repo.is_some()
    }

    /// The bound repository, if any.
    pub fn repo(&self) -> Option<&GitRepo> {
        self.repo.as_ref()
    }

    /// The tracked document's path.
    pub fn tracked_file(&self) -> Option<&Path> {
        self.tracked_file.as_deref()
    }

    /// Working-tree status of the bound repository — `None` when
    /// unbound or when `git status` fails.
    pub fn status(&self) -> Option<RepoStatus> {
        self.repo.as_ref().and_then(|r| r.status().ok())
    }

    /// The most recent `limit` commits of the bound repository.
    pub fn recent_commits(&self, limit: usize) -> Vec<Commit> {
        self.repo
            .as_ref()
            .and_then(|r| r.log(limit).ok())
            .unwrap_or_default()
    }

    /// Stage the tracked document and commit it with `message`.
    /// A no-op (returns `Ok`) when the session is unbound.
    pub fn commit_tracked(&self, message: &str) -> Result<(), GitError> {
        let (Some(repo), Some(file)) = (self.repo.as_ref(), self.tracked_file.as_ref()) else {
            return Ok(());
        };
        repo.stage(&[file.as_path()])?;
        repo.commit(message)?;
        Ok(())
    }

    /// Pull the bound repository's upstream branch.
    pub fn pull(&self) -> Result<MergeOutcome, GitError> {
        match self.repo.as_ref() {
            Some(repo) => repo.pull(),
            None => Ok(MergeOutcome::AlreadyUpToDate),
        }
    }

    /// Merge `other` into the current branch through the isolated
    /// worktree orchestrator. `None` when the session is unbound —
    /// there is no repository to merge in.
    pub fn merge_branch(&self, other: &str) -> Option<Result<WorktreeMergeReport, GitError>> {
        self.repo.as_ref().map(|repo| repo.merge_branch_isolated(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_available() -> bool {
        Command::new("git")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn unique_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("op-gitsession-{tag}-{}-{nanos}", std::process::id()))
    }

    #[test]
    fn unbound_session_reports_nothing() {
        let session = GitSession::new();
        assert!(!session.is_bound());
        assert!(session.status().is_none());
        assert!(session.current_branch().is_none());
        assert!(session.recent_commits(10).is_empty());
        // A commit on an unbound session is a tolerated no-op.
        session.commit_tracked("noop").expect("no-op commit");
    }

    #[test]
    fn rebind_binds_a_document_inside_a_repo() {
        if !git_available() {
            return;
        }
        let dir = unique_dir("bind");
        let repo = GitRepo::init(&dir).expect("git init");
        // A hermetic commit identity (no `op_git` config API yet).
        let git_config = |key: &str, value: &str| {
            Command::new("git")
                .arg("-C")
                .arg(repo.workdir())
                .args(["config", key, value])
                .output()
                .expect("git config");
        };
        git_config("user.email", "t@openpencil.dev");
        git_config("user.name", "OP Test");
        let doc = dir.join("design.op");
        std::fs::write(&doc, "{}").unwrap();

        let mut session = GitSession::new();
        // A path outside any repo → unbound.
        session.rebind(Some(&std::env::temp_dir()));
        // (temp_dir may or may not be in a repo; don't assert on it.)

        // Binding the document inside the repo.
        session.rebind(Some(&doc));
        assert!(session.is_bound());
        // The document is untracked until committed.
        let status = session.status().expect("status");
        assert!(!status.is_clean());

        // Commit through the session, then the tree is clean.
        session.commit_tracked("add design").expect("commit");
        assert!(session.status().expect("status").is_clean());
        assert_eq!(session.recent_commits(10).len(), 1);
        assert_eq!(session.current_branch().as_deref(), Some("main"));

        // Rebinding to `None` unbinds.
        session.rebind(None);
        assert!(!session.is_bound());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
