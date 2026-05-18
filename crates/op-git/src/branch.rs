//! Branch listing, creation, deletion and switching.

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
    /// The currently checked-out branch, or `None` on a detached
    /// `HEAD` (or a fresh repo with no commits yet).
    pub fn current_branch(&self) -> Result<Option<String>, GitError> {
        let out = self.run(&["branch", "--show-current"])?;
        let name = out.trim();
        Ok(if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        })
    }

    /// Every local branch, each flagged with whether it is current.
    pub fn branches(&self) -> Result<Vec<Branch>, GitError> {
        let current = self.current_branch()?;
        let out = self.run(&["for-each-ref", "--format=%(refname:short)", "refs/heads"])?;
        Ok(out
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|name| Branch {
                is_current: current.as_deref() == Some(name),
                name: name.to_string(),
            })
            .collect())
    }

    /// Create a branch `name` at the current `HEAD`, without
    /// switching to it.
    pub fn create_branch(&self, name: &str) -> Result<(), GitError> {
        self.run(&["branch", name])?;
        Ok(())
    }

    /// Delete branch `name`. Refuses (with [`GitError::Command`]) to
    /// delete a branch whose commits are not merged elsewhere, so
    /// work cannot be silently lost.
    pub fn delete_branch(&self, name: &str) -> Result<(), GitError> {
        self.run(&["branch", "-d", name])?;
        Ok(())
    }

    /// Switch the working tree to branch `name`.
    pub fn switch_branch(&self, name: &str) -> Result<(), GitError> {
        self.run(&["switch", name])?;
        Ok(())
    }

    /// Create branch `name` and switch to it in one step.
    pub fn create_and_switch_branch(&self, name: &str) -> Result<(), GitError> {
        self.run(&["switch", "--create", name])?;
        Ok(())
    }
}
