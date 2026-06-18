//! `.op` candidate enumeration for the tracked-file picker — lists the
//! repository's `.op` files with their commit history stats (TS
//! `gitClient.listCandidates` / `GitCandidateFileInfo`).

use std::collections::HashMap;
use std::path::Path;

use crate::{GitError, GitRepo};

/// One `.op` candidate file with its history stats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateOpFile {
    /// Absolute path.
    pub path: String,
    /// Path relative to the repo work-tree root (POSIX separators).
    pub relative_path: String,
    /// Number of commits that touched this file.
    pub milestone_count: u32,
    /// Author timestamp (Unix seconds) of the most recent commit touching
    /// it, or `None` when it has no history yet.
    pub last_commit_secs: Option<i64>,
    /// First line of that commit's message, if any.
    pub last_commit_message: Option<String>,
}

/// How far back the per-file history scan walks (keeps the picker snappy on
/// large repos; the picker only needs a count + the latest commit).
const SCAN_LIMIT: usize = 300;

impl GitRepo {
    /// Enumerate the `.op` files in the work tree with their history stats.
    /// Walks the working directory for `*.op` files (skipping `.git`), then a
    /// single bounded revwalk attributes commits to the files they touched.
    pub fn candidate_op_files(&self) -> Result<Vec<CandidateOpFile>, GitError> {
        let repo = self.open()?;
        let workdir = self.workdir().to_path_buf();

        // 1. Find every `.op` file under the work tree.
        let mut files: HashMap<String, CandidateOpFile> = HashMap::new();
        collect_op_files(&workdir, &workdir, &mut files);
        if files.is_empty() {
            return Ok(Vec::new());
        }

        // 2. One bounded revwalk; per commit, diff against its first parent
        //    and credit each changed `.op` path (newest commit seen first, so
        //    the first credit per file is its latest commit).
        if let Ok(mut walk) = repo.revwalk() {
            walk.set_sorting(git2::Sort::TIME).ok();
            if walk.push_head().is_ok() {
                for oid in walk.flatten().take(SCAN_LIMIT) {
                    let Ok(commit) = repo.find_commit(oid) else {
                        continue;
                    };
                    let new_tree = commit.tree().ok();
                    let old_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
                    let Some(new_tree) = new_tree else { continue };
                    let Ok(diff) = repo.diff_tree_to_tree(old_tree.as_ref(), Some(&new_tree), None)
                    else {
                        continue;
                    };
                    let secs = commit.author().when().seconds();
                    let summary = commit.summary().ok().flatten().map(str::to_string);
                    for delta in diff.deltas() {
                        let Some(path) = delta.new_file().path().and_then(Path::to_str) else {
                            continue;
                        };
                        let rel = path.replace('\\', "/");
                        if let Some(f) = files.get_mut(&rel) {
                            f.milestone_count += 1;
                            if f.last_commit_secs.is_none() {
                                f.last_commit_secs = Some(secs);
                                f.last_commit_message = summary.clone();
                            }
                        }
                    }
                }
            }
        }

        Ok(files.into_values().collect())
    }

    /// Clear this repository's local commit-author identity by removing the
    /// `user.name` / `user.email` keys from the repo-LOCAL config (TS overflow
    /// "清除提交作者"). Opens the `Local` config level explicitly so it never
    /// touches the user's global `~/.gitconfig` — `Config::remove` on the
    /// merged snapshot would delete from whichever level holds the key, which
    /// could be the global identity used by every other repo. Best-effort: a
    /// key that isn't set locally is not an error.
    pub fn unset_local_author(&self) -> Result<(), GitError> {
        let repo = self.open()?;
        if let Ok(mut local) = repo
            .config()
            .and_then(|c| c.open_level(git2::ConfigLevel::Local))
        {
            let _ = local.remove("user.name");
            let _ = local.remove("user.email");
        }
        Ok(())
    }
}

/// Recursively collect `*.op` files under `dir`, keyed by repo-relative path.
fn collect_op_files(dir: &Path, root: &Path, out: &mut HashMap<String, CandidateOpFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        if path.is_dir() {
            collect_op_files(&path, root, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("op") {
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            out.insert(
                rel.clone(),
                CandidateOpFile {
                    path: path.to_string_lossy().into_owned(),
                    relative_path: rel,
                    milestone_count: 0,
                    last_commit_secs: None,
                    last_commit_message: None,
                },
            );
        }
    }
}
