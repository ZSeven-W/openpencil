//! Throwaway git worktrees for isolated merges.
//!
//! A normal `git merge` runs in the live working tree. For a design
//! tool that is dangerous: a conflicting merge writes `<<<<<<<`
//! markers straight into the open `.op` document, corrupting the
//! JSON the editor is rendering, and leaves the repo in a
//! `MERGE_HEAD` state the user did not ask for.
//!
//! [`MergeWorktree`] sidesteps that by running the merge in a
//! detached, throwaway worktree. The live tree is never touched: a
//! clean merge is fast-forwarded back deliberately, and a
//! conflicting one is reported as a [`crate::ConflictBag`] without
//! ever marking up the user's files. The worktree directory and its
//! git registration are removed when the handle drops.

use std::path::PathBuf;

use crate::{GitError, GitRepo};

/// A detached, throwaway worktree used to compute a merge in
/// isolation from the user's live working tree.
pub(crate) struct MergeWorktree {
    /// `GitRepo` handle rooted at the worktree directory.
    repo: GitRepo,
    /// The main repository — used to deregister the worktree.
    main: GitRepo,
    /// The worktree directory.
    dir: PathBuf,
}

impl MergeWorktree {
    /// Add a detached worktree at `dir` checked out at `commit`.
    /// `dir` must not already exist — `git` creates it.
    pub(crate) fn create(
        main: &GitRepo,
        dir: PathBuf,
        commit: &str,
    ) -> Result<MergeWorktree, GitError> {
        let dir_str = dir
            .to_str()
            .ok_or_else(|| GitError::Io("worktree path is not valid UTF-8".to_string()))?;
        main.run(&["worktree", "add", "--detach", dir_str, commit])?;
        Ok(MergeWorktree {
            repo: GitRepo {
                workdir: dir.clone(),
                auth_env: Vec::new(),
            },
            main: main.clone(),
            dir,
        })
    }

    /// The `GitRepo` handle rooted in the worktree — merge / status
    /// operations run against this, never the live tree.
    pub(crate) fn repo(&self) -> &GitRepo {
        &self.repo
    }
}

impl Drop for MergeWorktree {
    fn drop(&mut self) {
        // Deregister the worktree with git first so its admin files
        // under `.git/worktrees/` are cleaned up.
        if let Some(dir_str) = self.dir.to_str() {
            let _ = self.main.run(&["worktree", "remove", "--force", dir_str]);
        }
        // Belt-and-suspenders: if `git worktree remove` failed (e.g.
        // a half-created worktree), still drop the directory and
        // prune the now-dangling registration.
        let _ = std::fs::remove_dir_all(&self.dir);
        let _ = self.main.run(&["worktree", "prune"]);
    }
}
