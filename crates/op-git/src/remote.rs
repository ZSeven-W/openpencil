//! Remote operations — clone, fetch, pull, push, remote config.
//!
//! The network operations (`clone` / `fetch` / `pull` / `push`)
//! depend on the ambient git credential / SSH setup; dedicated
//! credential + SSH-key handling lands in a later increment. They
//! are not unit-tested here (no network in tests); the remote-config
//! readers / writers are.

use std::path::{Path, PathBuf};

use crate::{git_output, stderr_of, GitError, GitRepo, MergeOutcome};

/// A configured git remote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    /// Remote name (`origin`, …).
    pub name: String,
    /// Fetch URL.
    pub url: String,
}

impl GitRepo {
    /// `git clone <url>` into `dir` and return a handle to the clone.
    /// `dir` must not already exist (git creates it).
    pub fn clone(url: &str, dir: &Path) -> Result<GitRepo, GitError> {
        // Run from `dir`'s parent so a relative target resolves; the
        // parent must exist for git to create `dir` inside it.
        let parent = dir.parent().unwrap_or_else(|| Path::new("."));
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| GitError::Io(e.to_string()))?;
        }
        let dir_str = dir.to_str().ok_or_else(|| GitError::Io("non-UTF-8 path".into()))?;
        let output = git_output(parent, &["clone", url, dir_str])?;
        if !output.status.success() {
            return Err(GitError::Command {
                operation: "clone".to_string(),
                stderr: stderr_of(&output),
            });
        }
        GitRepo::discover(dir)?.ok_or_else(|| GitError::NotARepo(PathBuf::from(dir)))
    }

    /// `git fetch` — update remote-tracking refs without touching the
    /// working tree.
    pub fn fetch(&self) -> Result<(), GitError> {
        self.run(&["fetch", "--all", "--prune"])?;
        Ok(())
    }

    /// Fetch and integrate the current branch's upstream, returning
    /// the [`MergeOutcome`] — exactly the TS `enginePull` model: a
    /// pull is a fetch followed by a merge of the remote-tracking
    /// ref. The fast-forward / merge / up-to-date decision is the
    /// shared, ancestry-based [`GitRepo::integrate`] classifier.
    pub fn pull(&self) -> Result<MergeOutcome, GitError> {
        // Stop *before* any network work: a pull during an unresolved
        // merge is refused by `integrate` anyway, and fetching first
        // would be wasted effort that can also hang or fail. `integrate`
        // keeps its own identical guard (it also backs `merge`).
        if self.is_merging() {
            return Err(GitError::MergeInProgress);
        }
        let before = self.run(&["rev-parse", "HEAD"])?.trim().to_string();
        self.fetch()?;
        // `@{u}` is the configured upstream tracking ref; pulling
        // without one configured is a genuine error.
        let upstream = self.run(&["rev-parse", "@{u}"])?.trim().to_string();
        self.integrate(&before, &upstream)
    }

    /// `git push` — publish the current branch to its upstream.
    pub fn push(&self) -> Result<(), GitError> {
        self.run(&["push"])?;
        Ok(())
    }

    /// Every configured remote with its fetch URL.
    pub fn remotes(&self) -> Result<Vec<Remote>, GitError> {
        let raw = self.run(&["remote", "-v"])?;
        Ok(parse_remotes(&raw))
    }

    /// The fetch URL of remote `name`, if it exists.
    pub fn remote_url(&self, name: &str) -> Result<Option<String>, GitError> {
        match self.run(&["remote", "get-url", name]) {
            Ok(url) => Ok(Some(url.trim().to_string())),
            // `git remote get-url` exits non-zero for an unknown
            // remote — that is "no such remote", not a hard error.
            Err(GitError::Command { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Point remote `name` at `url`, adding the remote when it does
    /// not exist yet.
    pub fn set_remote(&self, name: &str, url: &str) -> Result<(), GitError> {
        if self.remote_url(name)?.is_some() {
            self.run(&["remote", "set-url", name, url])?;
        } else {
            self.run(&["remote", "add", name, url])?;
        }
        Ok(())
    }
}

/// Parse `git remote -v` output. Each remote prints a `(fetch)` and
/// a `(push)` line; the `(fetch)` URL is kept, deduplicated by name.
fn parse_remotes(raw: &str) -> Vec<Remote> {
    let mut remotes: Vec<Remote> = Vec::new();
    for line in raw.lines() {
        // Format: `<name>\t<url> (fetch|push)`.
        if !line.contains("(fetch)") {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(name), Some(url)) = (parts.next(), parts.next()) else {
            continue;
        };
        if !remotes.iter().any(|r| r.name == name) {
            remotes.push(Remote {
                name: name.to_string(),
                url: url.to_string(),
            });
        }
    }
    remotes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fetch_urls_only_deduped() {
        let raw = "origin\tgit@github.com:ZSeven-W/openpencil.git (fetch)\n\
                   origin\tgit@github.com:ZSeven-W/openpencil.git (push)\n\
                   fork\thttps://example.com/x.git (fetch)\n\
                   fork\thttps://example.com/x.git (push)\n";
        let remotes = parse_remotes(raw);
        assert_eq!(remotes.len(), 2);
        assert_eq!(remotes[0].name, "origin");
        assert_eq!(remotes[0].url, "git@github.com:ZSeven-W/openpencil.git");
        assert_eq!(remotes[1].name, "fork");
    }

    #[test]
    fn empty_remote_output_is_empty() {
        assert!(parse_remotes("").is_empty());
    }
}
