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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use op_editor_core::MergeResolveState;
use op_git::{
    AuthStore, Commit, GitError, GitRepo, MergeOutcome, RepoStatus, SshKeyStore,
    WorktreeMergeReport,
};

/// The git repository bound to the currently-open document.
#[derive(Default)]
pub struct GitSession {
    /// The repository containing the open document, if any.
    repo: Option<GitRepo>,
    /// Absolute path of the tracked document.
    tracked_file: Option<PathBuf>,
    /// The user's credential store — used to authenticate network
    /// ops (pull / push). `None` when its config dir is unavailable.
    auth: Option<AuthStore>,
    /// The user's SSH-key store. `None` when `~/.ssh` is unavailable.
    ssh: Option<SshKeyStore>,
}

impl GitSession {
    /// An unbound session, with the credential + SSH-key stores
    /// loaded best-effort (a missing home directory leaves them
    /// `None` — network ops then use git's ambient auth).
    pub fn new() -> Self {
        Self {
            auth: AuthStore::user().ok(),
            ssh: SshKeyStore::user().ok(),
            ..Self::default()
        }
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

    /// Bind an externally-opened repository (the empty-state "Open"
    /// card), independent of the document's own location. The open
    /// document is tracked only when it lives under the repo's
    /// work-tree; otherwise the panel just reflects the repo's state.
    pub fn bind_repo(&mut self, repo: GitRepo, doc: Option<&Path>) {
        self.tracked_file = doc
            .filter(|p| p.starts_with(repo.workdir()))
            .map(|p| p.to_path_buf());
        self.repo = Some(repo);
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

    /// The bound repository with a resolved authentication
    /// environment applied — network ops (pull / push) run through
    /// this so a stored credential / SSH key is used. Falls back to
    /// the plain repo when no credential matches the remote.
    pub fn authed_repo(&self) -> Option<GitRepo> {
        let repo = self.repo.as_ref()?;
        match (&self.auth, &self.ssh) {
            (Some(auth), Some(ssh)) => Some(repo.with_auth_env(repo.auth_env(auth, ssh))),
            _ => Some(repo.clone()),
        }
    }

    /// The credential + SSH-key stores, when both are available —
    /// the host uses them to set up SSH auth for a remote.
    pub fn auth_stores(&self) -> Option<(&AuthStore, &SshKeyStore)> {
        Some((self.auth.as_ref()?, self.ssh.as_ref()?))
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

    /// Commit the currently-staged set with `message` — the index is
    /// exactly what the user assembled with the panel's per-file
    /// staging toggles, so nothing is force-staged here. A no-op
    /// (returns `Ok`) when the session is unbound.
    pub fn commit_staged(&self, message: &str) -> Result<(), GitError> {
        match self.repo.as_ref() {
            Some(repo) => {
                repo.commit(message)?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// Stage the tracked document — used to refresh its index blob
    /// after the editor saves it, so a commit captures the saved
    /// content rather than a stale staged blob. A no-op when unbound.
    pub fn stage_tracked(&self) -> Result<(), GitError> {
        let (Some(repo), Some(file)) = (self.repo.as_ref(), self.tracked_file.as_ref()) else {
            return Ok(());
        };
        repo.stage(&[file.as_path()])
    }

    /// Whether the tracked document has staged changes (its index blob
    /// differs from `HEAD`). After `stage_tracked`, `false` means the saved
    /// file matches the last commit — committing would create an empty
    /// milestone, so the caller skips it.
    pub fn tracked_has_staged_changes(&self) -> bool {
        let Some(repo) = self.repo.as_ref() else {
            return false;
        };
        match self.tracked_relpath() {
            Some(rel) => repo.is_path_staged(&rel).unwrap_or(false),
            None => false,
        }
    }

    /// The tracked document's path relative to the repository root,
    /// `/`-separated — the key form the Git panel's `changed_files`
    /// uses. `None` when unbound or the path is outside the repo.
    ///
    /// Both paths are canonicalized first: `repo.workdir()` comes
    /// from `git rev-parse` (symlinks resolved) while the tracked
    /// path is whatever the user opened, so a raw `strip_prefix`
    /// would fail whenever a symlinked directory (e.g. `/var` →
    /// `/private/var` on macOS) is in play.
    pub fn tracked_relpath(&self) -> Option<String> {
        let repo = self.repo.as_ref()?;
        let file = self.tracked_file.as_ref()?;
        let root = std::fs::canonicalize(repo.workdir()).ok()?;
        let file = std::fs::canonicalize(file).ok()?;
        let rel = file.strip_prefix(&root).ok()?;
        Some(rel.to_string_lossy().replace('\\', "/"))
    }

    /// Pull the bound repository's upstream branch.
    pub fn pull(&self) -> Result<MergeOutcome, GitError> {
        match self.repo.as_ref() {
            Some(repo) => repo.pull(),
            None => Ok(MergeOutcome::AlreadyUpToDate),
        }
    }

    /// Merge `other` into the current branch through the isolated
    /// worktree orchestrator. A conflicting `.op` file is offered to
    /// the structured node-level merge ([`op_opmerge`]); when that
    /// resolves cleanly its merged tree is written back so the merge
    /// can complete without manual conflict resolution. `None` when
    /// the session is unbound — there is no repository to merge in.
    pub fn merge_branch(&self, other: &str) -> Option<Result<WorktreeMergeReport, GitError>> {
        let repo = self.repo.as_ref()?;
        Some(
            repo.merge_branch_isolated(other, |path, base, ours, theirs| {
                if !path.ends_with(".op") {
                    return None;
                }
                // A whole-file add / delete (a missing ours or theirs
                // stage) is not a node-level content conflict — leave it
                // unresolved. A missing base (add/add) is fine.
                if ours.is_empty() || theirs.is_empty() {
                    return None;
                }
                let base = if base.is_empty() { "{}" } else { base };
                match op_opmerge::merge_op_documents(base, ours, theirs) {
                    Ok(result) if result.is_clean() => Some(result.merged_json()),
                    _ => None,
                }
            }),
        )
    }

    /// Re-run the branch merge applying the user's per-node ours /
    /// theirs choices from `state`. The resolver runs each `.op`
    /// file's conflicts through [`op_opmerge::resolve_op_merge`] with
    /// those choices — a fully-decided file resolves cleanly so the
    /// merge can complete. `None` when the session is unbound.
    pub fn merge_branch_resolved(
        &self,
        state: &MergeResolveState,
    ) -> Option<Result<WorktreeMergeReport, GitError>> {
        let repo = self.repo.as_ref()?;
        Some(
            repo.merge_branch_isolated(&state.branch, |path, base, ours, theirs| {
                if !path.ends_with(".op") {
                    return None;
                }
                if ours.is_empty() || theirs.is_empty() {
                    return None;
                }
                let file = state.files.iter().find(|f| f.path == path)?;
                let choices: HashMap<String, bool> = file
                    .conflicts
                    .iter()
                    .map(|c| (c.id.clone(), c.take_theirs))
                    .collect();
                let base = if base.is_empty() { "{}" } else { base };
                match op_opmerge::resolve_op_merge(base, ours, theirs, &choices) {
                    Ok(result) if result.is_clean() => Some(result.merged_json()),
                    _ => None,
                }
            }),
        )
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
        std::env::temp_dir().join(format!(
            "op-gitsession-{tag}-{}-{nanos}",
            std::process::id()
        ))
    }

    #[test]
    fn unbound_session_reports_nothing() {
        let session = GitSession::new();
        assert!(!session.is_bound());
        assert!(session.status().is_none());
        assert!(session.current_branch().is_none());
        assert!(session.recent_commits(10).is_empty());
        // A commit on an unbound session is a tolerated no-op.
        session.commit_staged("noop").expect("no-op commit");
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

        // Stage the document, then commit the staged set — the
        // session no longer force-stages, so staging is explicit.
        session.stage_tracked().expect("stage");
        assert_eq!(
            session.tracked_relpath().as_deref(),
            Some("design.op"),
            "the tracked path resolves relative to the repo root"
        );
        session.commit_staged("add design").expect("commit");
        assert!(session.status().expect("status").is_clean());
        assert_eq!(session.recent_commits(10).len(), 1);
        assert_eq!(session.current_branch().as_deref(), Some("main"));

        // Rebinding to `None` unbinds.
        session.rebind(None);
        assert!(!session.is_bound());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
