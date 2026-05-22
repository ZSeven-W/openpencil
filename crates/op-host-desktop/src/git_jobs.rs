//! Background git operations.
//!
//! Network-bound git (currently `pull`) must not run on the winit
//! UI thread — a slow remote would freeze the window. These jobs
//! run on a worker thread and the runner drains the result on a
//! later frame, mirroring `model_discovery` / `update_check`.

use std::path::Path;
use std::sync::mpsc::{self, Receiver, TryRecvError};

use op_editor_core::{GitCommitSummary, GitDiffTarget, GitFileEntry, Locale};
use op_git::{ChangeState, FileStatus, GitError, GitRepo, MergeOutcome};

/// A plain-data snapshot of a repository, computed off the UI thread
/// for the Git panel — exactly the fields `GitPanelState` shows.
pub struct GitSnapshot {
    /// Whether the document is inside a git repository.
    pub in_repo: bool,
    /// Current branch.
    pub branch: Option<String>,
    /// All local branch names.
    pub branches: Vec<String>,
    /// Changed-file count.
    pub dirty_count: usize,
    /// Conflicted-file count.
    pub conflicted_count: usize,
    /// Whether a merge is in progress.
    pub merging: bool,
    /// Repo-relative paths with unresolved merge conflicts.
    pub conflicted_files: Vec<String>,
    /// Changed files in the working tree — the per-file staging list.
    pub changed_files: Vec<GitFileEntry>,
    /// Configured remotes as display strings — `name → url`.
    pub remotes: Vec<String>,
    /// Most-recent commits, newest first.
    pub recent_commits: Vec<GitCommitSummary>,
}

/// Collapse `git status` entries into one per path for the panel's
/// staging list — a path with both a staged and an unstaged change
/// shows once, marked staged.
fn build_changed_files(files: &[FileStatus]) -> Vec<GitFileEntry> {
    let mut out: Vec<GitFileEntry> = Vec::new();
    for f in files {
        let status = match f.state {
            ChangeState::Modified => 'M',
            ChangeState::Added => 'A',
            ChangeState::Deleted => 'D',
            ChangeState::Renamed => 'R',
            ChangeState::Untracked => '?',
            ChangeState::Conflicted => 'U',
        };
        match out.iter_mut().find(|e| e.path == f.path) {
            Some(entry) => entry.staged |= f.staged,
            None => out.push(GitFileEntry {
                path: f.path.clone(),
                staged: f.staged,
                status,
            }),
        }
    }
    out
}

/// A repository status query (`status` + `log` + branch) running on
/// a worker thread. `git status` scans the whole working tree, so on
/// a large repository it must never run on the UI thread.
pub struct GitStatusJob {
    rx: Option<Receiver<GitSnapshot>>,
}

impl GitStatusJob {
    /// Spawn the status query on a worker thread.
    pub fn spawn(repo: GitRepo) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(snapshot(&repo));
        });
        Self { rx: Some(rx) }
    }

    /// Whether the query worker is still running.
    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// Drain the snapshot once it lands.
    pub fn poll(&mut self) -> Option<GitSnapshot> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(snap) => {
                self.rx = None;
                Some(snap)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                None
            }
        }
    }
}

/// Compute a [`GitSnapshot`] — the blocking git work, run on a worker.
fn snapshot(repo: &GitRepo) -> GitSnapshot {
    let branch = repo.current_branch().ok().flatten();
    let branches = repo
        .branches()
        .unwrap_or_default()
        .into_iter()
        .map(|b| b.name)
        .collect();
    let status = repo.status().ok();
    let dirty_count = status.as_ref().map(|s| s.files.len()).unwrap_or(0);
    let conflicted_count = status
        .as_ref()
        .map(|s| {
            s.files
                .iter()
                .filter(|f| f.state == ChangeState::Conflicted)
                .count()
        })
        .unwrap_or(0);
    let recent_commits = repo
        .log(8)
        .unwrap_or_default()
        .into_iter()
        .map(|c| GitCommitSummary {
            short_hash: c.short_hash,
            summary: c.summary,
            author: c.author,
        })
        .collect();
    let merging = repo.is_merging();
    let conflicted_files = repo.conflicted_files().unwrap_or_default();
    let changed_files = status
        .as_ref()
        .map(|s| build_changed_files(&s.files))
        .unwrap_or_default();
    let remotes = repo
        .remotes()
        .unwrap_or_default()
        .into_iter()
        .map(|r| format!("{} → {}", r.name, r.url))
        .collect();
    GitSnapshot {
        in_repo: true,
        branch,
        branches,
        dirty_count,
        conflicted_count,
        merging,
        conflicted_files,
        changed_files,
        remotes,
        recent_commits,
    }
}

/// A `git pull` running on a worker thread.
pub struct GitPullJob {
    rx: Option<Receiver<Result<MergeOutcome, GitError>>>,
}

impl GitPullJob {
    /// Spawn `repo.pull()` on a worker thread. Returns immediately;
    /// `repo` is moved to the worker (clone the session's `GitRepo`
    /// before calling).
    pub fn spawn(repo: GitRepo) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(repo.pull());
        });
        Self { rx: Some(rx) }
    }

    /// Whether the pull worker is still running.
    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// Drain the pull result once it lands. Returns `Some` exactly
    /// once (when the worker resolves); `None` while in flight or
    /// after it has been drained.
    pub fn poll(&mut self) -> Option<Result<MergeOutcome, GitError>> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(result) => {
                self.rx = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                None
            }
        }
    }
}

/// A `git push` running on a worker thread — the network-bound
/// push must not block the winit UI thread.
pub struct GitPushJob {
    rx: Option<Receiver<Result<(), GitError>>>,
}

impl GitPushJob {
    /// Spawn `repo.push()` on a worker thread.
    pub fn spawn(repo: GitRepo) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(repo.push());
        });
        Self { rx: Some(rx) }
    }

    /// Whether the push worker is still running.
    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// Drain the push result once it lands.
    pub fn poll(&mut self) -> Option<Result<(), GitError>> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(result) => {
                self.rx = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                None
            }
        }
    }
}

/// A computed unified diff — the worker output of a [`GitDiffJob`].
pub struct GitDiffResult {
    /// Human label for the diff (a path, or a commit reference).
    pub title: String,
    /// The diff text, split into lines for per-line colouring.
    pub lines: Vec<String>,
    /// Repo-relative path when the diff is a single working-tree
    /// file (per-hunk staging applies); `None` otherwise.
    pub stage_path: Option<String>,
}

/// A `git diff` / `git show` running on a worker thread. A diff can
/// be large, so — like `git status` — it must never run on the UI
/// thread.
pub struct GitDiffJob {
    rx: Option<Receiver<GitDiffResult>>,
}

impl GitDiffJob {
    /// Spawn the diff computation for `target` on a worker thread.
    ///
    /// If a previous `GitDiffJob` is dropped (a second diff request
    /// supersedes it), its worker's `tx.send` simply fails and the
    /// result is discarded — the newest request always wins. The
    /// superseded `git` subprocess is short-lived (a single
    /// `diff` / `show`) and finishes harmlessly on its own.
    pub fn spawn(repo: GitRepo, target: GitDiffTarget, locale: Locale) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(compute_diff(&repo, &target, locale));
        });
        Self { rx: Some(rx) }
    }

    /// Whether the diff worker is still running.
    pub fn is_pending(&self) -> bool {
        self.rx.is_some()
    }

    /// Drain the diff once it lands.
    pub fn poll(&mut self) -> Option<GitDiffResult> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(result) => {
                self.rx = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                None
            }
        }
    }
}

/// Run the blocking `git diff` / `git show` for `target`.
fn compute_diff(repo: &GitRepo, target: &GitDiffTarget, locale: Locale) -> GitDiffResult {
    let (title, raw) = match target {
        GitDiffTarget::WorkingTree => (
            op_i18n::translate(locale, "git.panel.diffWorkingTree").to_string(),
            repo.diff(None),
        ),
        GitDiffTarget::Path(path) => (path.clone(), repo.diff(Some(Path::new(path)))),
        GitDiffTarget::Commit(rev) => (
            op_i18n::translate(locale, "git.panel.diffCommit").replace("{{rev}}", rev),
            repo.commit_diff(rev),
        ),
    };
    let text = match raw {
        Ok(text) => text,
        Err(err) => op_i18n::translate(locale, "git.panel.diffError")
            .replace("{{message}}", &err.to_string()),
    };
    let lines = text.lines().map(str::to_string).collect();
    // Only a single working-tree file's diff supports per-hunk
    // staging — a commit diff is historical, the whole-tree diff
    // spans many files.
    let stage_path = match target {
        GitDiffTarget::Path(path) => Some(path.clone()),
        _ => None,
    };
    GitDiffResult { title, lines, stage_path }
}
