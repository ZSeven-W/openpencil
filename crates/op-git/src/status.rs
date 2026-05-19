//! Working-tree status, staging, commit and restore.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::{stderr_of, GitError, GitRepo};

/// How a file differs from `HEAD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeState {
    /// Tracked file with content changes.
    Modified,
    /// Newly added file.
    Added,
    /// File removed from the tree.
    Deleted,
    /// File moved / renamed.
    Renamed,
    /// File not tracked by git.
    Untracked,
    /// File has unresolved merge conflicts.
    Conflicted,
}

/// One changed path in the working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStatus {
    /// Path relative to the repository root.
    pub path: String,
    /// What kind of change this is.
    pub state: ChangeState,
    /// Whether the change is staged in the index.
    pub staged: bool,
}

/// A snapshot of the repository's working-tree state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoStatus {
    /// Current branch name; `None` on a detached `HEAD`.
    pub branch: Option<String>,
    /// Changed paths (staged + unstaged + untracked + conflicted).
    pub files: Vec<FileStatus>,
    /// Commits the local branch is ahead of its upstream.
    pub ahead: u32,
    /// Commits the local branch is behind its upstream.
    pub behind: u32,
}

impl RepoStatus {
    /// Whether the working tree has any change at all.
    pub fn is_clean(&self) -> bool {
        self.files.is_empty()
    }

    /// Whether any tracked file has an unresolved conflict.
    pub fn has_conflicts(&self) -> bool {
        self.files.iter().any(|f| f.state == ChangeState::Conflicted)
    }
}

impl GitRepo {
    /// Snapshot the working tree — branch, changed files, ahead /
    /// behind counts.
    pub fn status(&self) -> Result<RepoStatus, GitError> {
        let raw = self.run(&["status", "--porcelain=v1", "--branch"])?;
        Ok(parse_status(&raw))
    }

    /// Stage `paths` (relative to the repo root or absolute).
    pub fn stage(&self, paths: &[&Path]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let mut args: Vec<&str> = vec!["add", "--"];
        let path_strs: Vec<&str> = paths.iter().filter_map(|p| p.to_str()).collect();
        args.extend(path_strs);
        self.run(&args)?;
        Ok(())
    }

    /// Stage every change in the working tree (`git add -A`).
    pub fn stage_all(&self) -> Result<(), GitError> {
        self.run(&["add", "-A"])?;
        Ok(())
    }

    /// Unstage `paths` — remove them from the index without touching
    /// the working tree. After the first commit `git restore --staged`
    /// resets each path's index entry to `HEAD`; before any commit
    /// there is no `HEAD`, so `git rm --cached` drops the staged
    /// addition instead (leaving the file untracked).
    pub fn unstage(&self, paths: &[&Path]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let has_head = self
            .run(&["rev-parse", "--verify", "--quiet", "HEAD"])
            .is_ok();
        for path in paths.iter().filter_map(|p| p.to_str()) {
            if has_head {
                self.run(&["restore", "--staged", "--", path])?;
            } else {
                self.run(&["rm", "--cached", "--quiet", "--", path])?;
            }
        }
        Ok(())
    }

    /// Stage a unified-diff `patch` into the index — `git apply
    /// --cached`, the mechanism behind per-hunk staging. `patch`
    /// must be a self-contained patch (file header + the chosen
    /// hunks). `--recount` lets git tolerate hunk line-count drift
    /// when only a subset of a file's hunks is applied.
    pub fn apply_cached(&self, patch: &str) -> Result<(), GitError> {
        let mut child = Command::new("git")
            .current_dir(&self.workdir)
            .args(["apply", "--cached", "--recount", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    GitError::GitNotFound
                } else {
                    GitError::Io(e.to_string())
                }
            })?;
        child
            .stdin
            .take()
            .ok_or_else(|| GitError::Io("git apply stdin unavailable".to_string()))?
            .write_all(patch.as_bytes())
            .map_err(|e| GitError::Io(e.to_string()))?;
        let output = child
            .wait_with_output()
            .map_err(|e| GitError::Io(e.to_string()))?;
        if !output.status.success() {
            return Err(GitError::Command {
                operation: "apply".to_string(),
                stderr: stderr_of(&output),
            });
        }
        Ok(())
    }

    /// Whether `path` has a change staged in the index. `git diff
    /// --cached` lists the path exactly when its index entry differs
    /// from `HEAD` (or, before the first commit, from the empty
    /// tree) — the authoritative answer, unlike a UI snapshot.
    pub fn is_path_staged(&self, path: &str) -> Result<bool, GitError> {
        let out = self.run(&["diff", "--cached", "--name-only", "--", path])?;
        Ok(!out.trim().is_empty())
    }

    /// Commit the staged changes with `message`. Returns the new
    /// commit's full hash. Fails with [`GitError::Command`] when
    /// there is nothing staged to commit.
    pub fn commit(&self, message: &str) -> Result<String, GitError> {
        self.run(&["commit", "-m", message])?;
        Ok(self.run(&["rev-parse", "HEAD"])?.trim().to_string())
    }

    /// Restore `path`'s working-tree content to its version at
    /// `commit` (a hash, tag, branch, or `"HEAD"`).
    ///
    /// This mirrors the TS engine's `restoreFileFromCommit`: it
    /// rewrites the working-tree file from the commit's blob and
    /// leaves the index untouched. Passing `"HEAD"` therefore
    /// discards uncommitted edits to the file; passing a historical
    /// commit hash rolls the file back to that revision.
    pub fn restore(&self, path: &Path, commit: &str) -> Result<(), GitError> {
        let Some(path) = path.to_str() else {
            return Ok(());
        };
        self.run(&["restore", "--source", commit, "--worktree", "--", path])?;
        Ok(())
    }
}

/// Parse `git status --porcelain=v1 --branch` output.
fn parse_status(raw: &str) -> RepoStatus {
    let mut branch = None;
    let mut ahead = 0;
    let mut behind = 0;
    let mut files = Vec::new();

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            let (b, a, be) = parse_branch_line(rest);
            branch = b;
            ahead = a;
            behind = be;
        } else if line.len() >= 3 {
            files.push(parse_file_line(line));
        }
    }

    RepoStatus {
        branch,
        files,
        ahead,
        behind,
    }
}

/// Parse the `## ` branch header — `main...origin/main [ahead 1, behind 2]`.
fn parse_branch_line(rest: &str) -> (Option<String>, u32, u32) {
    // The branch name runs up to `...` (upstream marker) or a space.
    let name_end = rest.find("...").or_else(|| rest.find(' ')).unwrap_or(rest.len());
    let name = rest[..name_end].trim();
    // A brand-new repo with no commits reports `No commits yet on main`.
    let branch = if name.is_empty() || name.contains("No commits yet") {
        rest.rsplit(' ').next().filter(|s| !s.is_empty()).map(str::to_string)
    } else {
        Some(name.to_string())
    };

    let mut ahead = 0;
    let mut behind = 0;
    if let (Some(open), Some(close)) = (rest.find('['), rest.find(']')) {
        if open < close {
            for part in rest[open + 1..close].split(',') {
                let part = part.trim();
                if let Some(n) = part.strip_prefix("ahead ") {
                    ahead = n.trim().parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix("behind ") {
                    behind = n.trim().parse().unwrap_or(0);
                }
            }
        }
    }
    (branch, ahead, behind)
}

/// Parse one `XY <path>` porcelain line into a [`FileStatus`].
fn parse_file_line(line: &str) -> FileStatus {
    let code = &line[..2];
    let mut path = line[3..].to_string();
    // Renames render as `old -> new`; keep the destination path.
    if let Some(idx) = path.find(" -> ") {
        path = path[idx + 4..].to_string();
    }
    let x = code.as_bytes()[0] as char;
    let y = code.as_bytes()[1] as char;

    let state = if code == "??" {
        ChangeState::Untracked
    } else if is_conflict(x, y) {
        ChangeState::Conflicted
    } else if x == 'A' || y == 'A' {
        ChangeState::Added
    } else if x == 'D' || y == 'D' {
        ChangeState::Deleted
    } else if x == 'R' || y == 'R' {
        ChangeState::Renamed
    } else {
        ChangeState::Modified
    };
    // The index column (`x`) carries the change when it is staged.
    let staged = x != ' ' && x != '?';

    FileStatus { path, state, staged }
}

/// Whether an `XY` porcelain code marks an unresolved merge conflict.
fn is_conflict(x: char, y: char) -> bool {
    x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_clean_branch_header() {
        let s = parse_status("## main\n");
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert!(s.is_clean());
        assert_eq!((s.ahead, s.behind), (0, 0));
    }

    #[test]
    fn parses_upstream_ahead_behind() {
        let s = parse_status("## main...origin/main [ahead 2, behind 3]\n");
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!((s.ahead, s.behind), (2, 3));
    }

    #[test]
    fn classifies_each_porcelain_code() {
        let raw = "## main\n\
                   M  staged.txt\n\
                   \u{20}M unstaged.txt\n\
                   ?? new.txt\n\
                   A  added.txt\n\
                   \u{20}D gone.txt\n\
                   UU conflict.txt\n";
        let s = parse_status(raw);
        assert_eq!(s.files.len(), 6);
        let by = |name: &str| s.files.iter().find(|f| f.path == name).unwrap().clone();
        assert_eq!(by("staged.txt").state, ChangeState::Modified);
        assert!(by("staged.txt").staged);
        assert_eq!(by("unstaged.txt").state, ChangeState::Modified);
        assert!(!by("unstaged.txt").staged);
        assert_eq!(by("new.txt").state, ChangeState::Untracked);
        assert_eq!(by("added.txt").state, ChangeState::Added);
        assert_eq!(by("gone.txt").state, ChangeState::Deleted);
        assert_eq!(by("conflict.txt").state, ChangeState::Conflicted);
        assert!(s.has_conflicts());
    }

    #[test]
    fn rename_keeps_the_destination_path() {
        let s = parse_status("## main\nR  old.txt -> new.txt\n");
        assert_eq!(s.files[0].path, "new.txt");
        assert_eq!(s.files[0].state, ChangeState::Renamed);
    }
}
