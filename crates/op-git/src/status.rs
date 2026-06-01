//! Working-tree status, staging, commit and restore — in-process via
//! libgit2 (`git2`), so no system `git` binary is required.

use std::path::{Path, PathBuf};

use crate::{GitError, GitRepo};

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
        self.files
            .iter()
            .any(|f| f.state == ChangeState::Conflicted)
    }
}

impl GitRepo {
    /// Snapshot the working tree — branch, changed files, ahead /
    /// behind counts.
    pub fn status(&self) -> Result<RepoStatus, GitError> {
        let repo = self.open()?;

        let mut opts = git2::StatusOptions::new();
        opts.include_untracked(true)
            .recurse_untracked_dirs(true)
            .renames_head_to_index(true)
            .renames_index_to_workdir(true);
        let entries = repo.statuses(Some(&mut opts))?;

        let mut files = Vec::new();
        for entry in entries.iter() {
            let Some(path) = entry.path() else { continue };
            files.push(FileStatus {
                path: path.to_string(),
                state: classify(entry.status()),
                // Staged ⇔ the index differs from HEAD (any `INDEX_*` bit).
                staged: entry.status().intersects(
                    git2::Status::INDEX_NEW
                        | git2::Status::INDEX_MODIFIED
                        | git2::Status::INDEX_DELETED
                        | git2::Status::INDEX_RENAMED
                        | git2::Status::INDEX_TYPECHANGE,
                ),
            });
        }

        // `current_branch` resolves an unborn `HEAD` to its symbolic
        // target (`main`) too, so a fresh repo reports its branch rather
        // than a misleading detached state.
        let branch = self.current_branch().unwrap_or(None);
        let (ahead, behind) = ahead_behind(&repo);

        Ok(RepoStatus {
            branch,
            files,
            ahead,
            behind,
        })
    }

    /// Stage `paths` (relative to the repo root or absolute). Uses
    /// `add_all`, which stages additions, modifications AND deletions
    /// of the matched paths — mirroring `git add -- <path>`.
    pub fn stage(&self, paths: &[&Path]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let repo = self.open()?;
        let mut index = repo.index()?;
        let specs: Vec<PathBuf> = paths.iter().map(|p| self.rel_to_workdir(p)).collect();
        index.add_all(specs.iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    }

    /// Stage every change in the working tree (`git add -A`).
    pub fn stage_all(&self) -> Result<(), GitError> {
        let repo = self.open()?;
        let mut index = repo.index()?;
        // `add_all` over the whole tree picks up new + modified files;
        // `update_all` records deletions + modifications of already-tracked
        // files — together they equal `git add -A`.
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.update_all(["*"].iter(), None)?;
        index.write()?;
        Ok(())
    }

    /// Unstage `paths` — remove them from the index without touching
    /// the working tree. After the first commit each path's index entry
    /// is reset to `HEAD`; before any commit there is no `HEAD`, so the
    /// staged addition is dropped from the index instead (leaving the
    /// file untracked).
    pub fn unstage(&self, paths: &[&Path]) -> Result<(), GitError> {
        if paths.is_empty() {
            return Ok(());
        }
        let repo = self.open()?;
        let specs: Vec<PathBuf> = paths.iter().map(|p| self.rel_to_workdir(p)).collect();
        match repo.head() {
            Ok(head) => {
                let head_obj = head.peel(git2::ObjectType::Commit)?;
                repo.reset_default(Some(&head_obj), specs.iter())?;
            }
            // Unborn HEAD (no commit yet) — drop the staged additions.
            Err(_) => {
                let mut index = repo.index()?;
                for spec in &specs {
                    let _ = index.remove_path(spec);
                }
                index.write()?;
            }
        }
        Ok(())
    }

    /// Stage a unified-diff `patch` into the index — the mechanism
    /// behind per-hunk staging. `patch` must be a self-contained patch
    /// (file header + the chosen hunks); it is applied to the index
    /// only (the working tree is untouched).
    pub fn apply_cached(&self, patch: &str) -> Result<(), GitError> {
        let repo = self.open()?;
        let diff = git2::Diff::from_buffer(patch.as_bytes())?;
        repo.apply(&diff, git2::ApplyLocation::Index, None)?;
        Ok(())
    }

    /// Whether `path` has a change staged in the index — its index
    /// entry differs from `HEAD` (or, before the first commit, from the
    /// empty tree).
    pub fn is_path_staged(&self, path: &str) -> Result<bool, GitError> {
        let repo = self.open()?;
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(self.rel_str(Path::new(path)));
        let head_tree = repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        let diff = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
        Ok(diff.deltas().len() > 0)
    }

    /// Commit the staged changes with `message`. Returns the new
    /// commit's full hash. The committer identity comes from git config
    /// (`user.name` / `user.email`); an unborn `HEAD` produces the
    /// repository's first (parent-less) commit.
    pub fn commit(&self, message: &str) -> Result<String, GitError> {
        let repo = self.open()?;
        let signature = repo.signature()?;

        // Snapshot the staged index as a tree.
        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;

        // Parent = the current HEAD commit, if the branch has one yet.
        let parents: Vec<git2::Commit> = match repo.head() {
            Ok(head) => head.peel_to_commit().ok().into_iter().collect(),
            Err(_) => Vec::new(),
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();

        let oid = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parent_refs,
        )?;
        let commit = repo.find_commit(oid)?;
        repo.reset(commit.as_object(), git2::ResetType::Mixed, None)?;
        Ok(oid.to_string())
    }

    /// Restore `path`'s working-tree content to its version at `commit`
    /// (a hash, tag, branch, or `"HEAD"`), rewriting the working-tree
    /// file from the commit's blob and leaving the index untouched.
    pub fn restore(&self, path: &Path, commit: &str) -> Result<(), GitError> {
        let repo = self.open()?;
        let rel = self.rel_to_workdir(path);
        let Some(rel_str) = rel.to_str() else {
            return Ok(());
        };
        let object = repo.revparse_single(&format!("{commit}:{rel_str}"))?;
        let blob = object.peel_to_blob()?;
        let abs = self.workdir().join(&rel);
        std::fs::write(&abs, blob.content()).map_err(|e| GitError::Io(e.to_string()))?;
        Ok(())
    }

    /// Read `relpath`'s file content at `rev` (a hash, tag, branch, or a
    /// revspec like `<hash>^`). Returns `None` when the path does not exist
    /// at that revision (e.g. the file was added in that commit, or `rev`
    /// has no parent). Used by the commit-detail card's semantic diff.
    pub fn blob_at_commit(&self, rev: &str, relpath: &str) -> Result<Option<String>, GitError> {
        let repo = self.open()?;
        let result = match repo.revparse_single(&format!("{rev}:{relpath}")) {
            Ok(object) => {
                let blob = object.peel_to_blob()?;
                Ok(Some(String::from_utf8_lossy(blob.content()).into_owned()))
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        };
        result
    }

    /// A path made relative to the work-tree root — libgit2 index /
    /// pathspec APIs expect repo-relative paths, but callers pass
    /// absolute document paths.
    fn rel_to_workdir(&self, p: &Path) -> PathBuf {
        if let Ok(rel) = p.strip_prefix(self.workdir()) {
            return rel.to_path_buf();
        }

        let Some(rel) = canonical_relpath(self.workdir(), p) else {
            return p.to_path_buf();
        };
        rel
    }

    /// `rel_to_workdir` as a `String` for pathspec strings.
    fn rel_str(&self, p: &Path) -> String {
        self.rel_to_workdir(p).to_string_lossy().into_owned()
    }
}

fn canonical_relpath(root: &Path, path: &Path) -> Option<PathBuf> {
    let root = std::fs::canonicalize(root).ok()?;
    let path = canonicalize_path_or_parent(path)?;
    path.strip_prefix(root).map(Path::to_path_buf).ok()
}

fn canonicalize_path_or_parent(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok().or_else(|| {
        let parent = path.parent()?;
        let file_name = path.file_name()?;
        std::fs::canonicalize(parent)
            .ok()
            .map(|parent| parent.join(file_name))
    })
}

/// Map a libgit2 status bitset to the single [`ChangeState`] the panel
/// shows. Conflicts win; otherwise the first matching add / delete /
/// rename / untracked classification, falling back to modified.
fn classify(s: git2::Status) -> ChangeState {
    use git2::Status as St;
    if s.is_conflicted() {
        ChangeState::Conflicted
    } else if s.intersects(St::WT_NEW) && !s.intersects(St::INDEX_NEW) {
        ChangeState::Untracked
    } else if s.intersects(St::INDEX_NEW) {
        ChangeState::Added
    } else if s.intersects(St::INDEX_DELETED | St::WT_DELETED) {
        ChangeState::Deleted
    } else if s.intersects(St::INDEX_RENAMED | St::WT_RENAMED) {
        ChangeState::Renamed
    } else {
        ChangeState::Modified
    }
}

/// Ahead / behind commit counts of the current branch vs its configured
/// upstream — `(0, 0)` when detached, unborn, or with no upstream set.
fn ahead_behind(repo: &git2::Repository) -> (u32, u32) {
    let resolve = || -> Option<(usize, usize)> {
        let head = repo.head().ok()?;
        let local = head.target()?;
        let branch_name = head.shorthand()?;
        let upstream = repo
            .find_branch(branch_name, git2::BranchType::Local)
            .ok()?
            .upstream()
            .ok()?;
        let upstream_oid = upstream.get().target()?;
        repo.graph_ahead_behind(local, upstream_oid).ok()
    };
    resolve()
        .map(|(a, b)| (a as u32, b as u32))
        .unwrap_or((0, 0))
}
