//! Branch listing, creation, deletion and switching.

use git2::{build::CheckoutBuilder, BranchType};

use crate::{GitError, GitRepo};

/// A local branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    /// Short branch name (`main`, `feature/x`).
    pub name: String,
    /// Whether this is the currently checked-out branch.
    pub is_current: bool,
}

impl GitRepo {
    /// The currently checked-out branch, or `None` on a detached `HEAD`.
    /// A fresh repo with no commits is still ON a branch (the unborn
    /// `main`), so it reports that branch name — not `None`.
    pub fn current_branch(&self) -> Result<Option<String>, GitError> {
        let repo = self.open()?;
        let head = match repo.head() {
            Ok(head) => head,
            // Unborn branch (no commits yet): `HEAD` does not resolve to a
            // commit, but it IS a symbolic ref to the branch the first
            // commit will create (`main`). Read that target so the panel
            // shows `main`, not a (wrong) "detached HEAD" — matching what
            // the subprocess `git status --branch` reported here.
            Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
                return Ok(unborn_head_branch(&repo));
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        // A detached `HEAD` points straight at a commit, not a branch.
        if !head.is_branch() {
            return Ok(None);
        }
        Ok(head.shorthand().map(str::to_string))
    }

    /// Every local branch, each flagged with whether it is current.
    pub fn branches(&self) -> Result<Vec<Branch>, GitError> {
        let current = self.current_branch()?;
        let repo = self.open()?;
        let mut result = Vec::new();
        for entry in repo.branches(Some(BranchType::Local))? {
            let (branch, _kind) = entry?;
            // `name()` is `Ok(None)` for a non-UTF-8 ref name; skip it
            // rather than fabricate a lossy name.
            if let Some(name) = branch.name()? {
                result.push(Branch {
                    is_current: current.as_deref() == Some(name),
                    name: name.to_string(),
                });
            }
        }
        Ok(result)
    }

    /// Create a branch `name` at the current `HEAD`, without
    /// switching to it.
    pub fn create_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.open()?;
        // Resolve `HEAD` to the commit the new branch should point at.
        let target = repo.head()?.peel_to_commit()?;
        // `force = false` — refuse to clobber an existing branch of the
        // same name, matching `git branch <name>`.
        repo.branch(name, &target, false)?;
        Ok(())
    }

    /// Delete branch `name`. Refuses (with [`GitError::Command`]) to
    /// delete a branch whose commits are not merged elsewhere, so
    /// work cannot be silently lost.
    pub fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.open()?;
        let mut branch = repo.find_branch(name, BranchType::Local)?;
        branch.delete()?;
        Ok(())
    }

    /// Switch the working tree to branch `name`.
    pub fn switch_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.open()?;
        // Point `HEAD` at the branch ref, then check its tree out into
        // the working directory. `force` mirrors the subprocess path's
        // tree-overwriting behaviour so the working tree always matches
        // the branch after a switch.
        let refname = format!("refs/heads/{name}");
        repo.set_head(&refname)?;
        let mut checkout = CheckoutBuilder::new();
        checkout.force();
        repo.checkout_head(Some(&mut checkout))?;
        Ok(())
    }

    /// Create branch `name` and switch to it in one step.
    pub fn create_and_switch_branch(&self, name: &str) -> Result<(), GitError> {
        let repo = self.open()?;
        // Create the branch at the current `HEAD` commit, then attach
        // `HEAD` to it. The new branch shares `HEAD`'s tree, so the
        // working tree needs no file changes — a safe checkout keeps any
        // uncommitted edits intact, matching `git switch --create`.
        let target = repo.head()?.peel_to_commit()?;
        repo.branch(name, &target, false)?;
        let refname = format!("refs/heads/{name}");
        repo.set_head(&refname)?;
        repo.checkout_head(Some(CheckoutBuilder::new().safe()))?;
        Ok(())
    }
}

/// The branch name a fresh (unborn-`HEAD`) repository will create on its
/// first commit — read from `HEAD`'s symbolic target (`refs/heads/main`
/// → `main`). `None` when `HEAD` is not a symbolic ref to a branch.
fn unborn_head_branch(repo: &git2::Repository) -> Option<String> {
    repo.find_reference("HEAD")
        .ok()
        .and_then(|r| r.symbolic_target().map(str::to_string))
        .and_then(|t| t.strip_prefix("refs/heads/").map(str::to_string))
}
