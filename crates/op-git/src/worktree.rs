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
//!
//! The implementation drives libgit2 in-process (`git2`) — no system
//! `git` subprocess. A linked worktree is created with
//! [`git2::Repository::worktree`], detached onto the requested commit
//! by opening the worktree repo and `set_head_detached` + a forced
//! `checkout_head`, and deregistered on drop via
//! [`git2::Worktree::prune`] with `working_tree` enabled so libgit2
//! removes both the `.git/worktrees/<name>` admin files and the
//! worktree directory itself.

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
    /// libgit2's registration name for this linked worktree (the
    /// directory basename). Used to look the worktree up again on
    /// drop so it can be pruned.
    name: String,
}

impl MergeWorktree {
    /// Add a detached worktree at `dir` checked out at `commit`.
    /// `dir` must not already exist — libgit2 creates it.
    pub(crate) fn create(
        main: &GitRepo,
        dir: PathBuf,
        commit: &str,
    ) -> Result<MergeWorktree, GitError> {
        // The worktree registration name. libgit2 keys its admin
        // files under `.git/worktrees/<name>`; deriving it from the
        // directory basename keeps it unique (the dir is itself a
        // unique temp path) and lets `Drop` find the worktree again.
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| GitError::Io("worktree path has no valid basename".to_string()))?
            .to_string();

        let main_repo = main.open()?;

        // Resolve the committish (a full HEAD sha in practice, but
        // accept any revspec) to the commit it names. The detached
        // HEAD is pinned onto exactly this commit below.
        let commit_oid = main_repo
            .revparse_single(commit)
            .map_err(|e| GitError::Command {
                operation: "worktree add".to_string(),
                stderr: e.message().to_string(),
            })?
            .peel_to_commit()
            .map_err(|e| GitError::Command {
                operation: "worktree add".to_string(),
                stderr: e.message().to_string(),
            })?
            .id();

        // Create the linked worktree. With no `reference` set in the
        // options libgit2 checks it out at the main repo's HEAD; we
        // re-point it onto `commit` (detached) immediately after.
        let opts = git2::WorktreeAddOptions::new();
        let worktree =
            main_repo
                .worktree(&name, &dir, Some(&opts))
                .map_err(|e| GitError::Command {
                    operation: "worktree add".to_string(),
                    stderr: e.message().to_string(),
                })?;

        // Detach the new worktree's HEAD onto the requested commit and
        // force the working tree to match — the libgit2 equivalent of
        // `git worktree add --detach <dir> <commit>`.
        let wt_repo =
            git2::Repository::open_from_worktree(&worktree).map_err(|e| GitError::Command {
                operation: "worktree add".to_string(),
                stderr: e.message().to_string(),
            })?;
        wt_repo
            .set_head_detached(commit_oid)
            .map_err(|e| GitError::Command {
                operation: "worktree add".to_string(),
                stderr: e.message().to_string(),
            })?;
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        wt_repo
            .checkout_head(Some(&mut checkout))
            .map_err(|e| GitError::Command {
                operation: "worktree add".to_string(),
                stderr: e.message().to_string(),
            })?;

        Ok(MergeWorktree {
            repo: GitRepo {
                workdir: dir.clone(),
                auth_env: Vec::new(),
            },
            main: main.clone(),
            dir,
            name,
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
        // Deregister the worktree with libgit2 first so its admin
        // files under `.git/worktrees/` are cleaned up. `prune` with
        // `working_tree` also recursively removes the worktree
        // directory itself.
        if let Ok(main_repo) = self.main.open() {
            if let Ok(worktree) = main_repo.find_worktree(&self.name) {
                let mut opts = git2::WorktreePruneOptions::new();
                // Prune even though the worktree is still valid (it
                // exists on disk) and recursively remove its working
                // directory — this is a throwaway tree by design.
                opts.valid(true).working_tree(true);
                let _ = worktree.prune(Some(&mut opts));
            }
        }
        // Belt-and-suspenders: if the prune failed or only removed the
        // admin files (e.g. a half-created worktree, or a libgit2
        // build that left the directory behind), still drop the
        // directory so no stale temp tree lingers.
        let _ = std::fs::remove_dir_all(&self.dir);
        // Final sweep: drop any now-dangling registration whose
        // working tree no longer exists on disk.
        if let Ok(main_repo) = self.main.open() {
            if let Ok(worktree) = main_repo.find_worktree(&self.name) {
                if worktree.is_prunable(None).unwrap_or(false) {
                    let _ = worktree.prune(None);
                }
            }
        }
    }
}
