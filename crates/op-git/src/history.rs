//! Commit history and diffs.

use std::path::Path;

use crate::{GitError, GitRepo};

/// One commit in the repository history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    /// Full 40-hex commit hash.
    pub hash: String,
    /// Abbreviated hash (as `git` chose to shorten it).
    pub short_hash: String,
    /// Author display name.
    pub author: String,
    /// Author email.
    pub email: String,
    /// Author timestamp, Unix seconds.
    pub timestamp: i64,
    /// First line of the commit message.
    pub summary: String,
}

/// Field separator for the `git log` pretty format — the ASCII unit
/// separator (`0x1f`), which never appears in commit metadata.
const SEP: char = '\u{1f}';

impl GitRepo {
    /// The most recent `limit` commits on the current branch, newest
    /// first. A repository with no commits yet yields an empty list
    /// rather than an error.
    pub fn log(&self, limit: usize) -> Result<Vec<Commit>, GitError> {
        // `git log` errors on a commit-less repo; probe `HEAD` first.
        if self
            .run(&["rev-parse", "--verify", "--quiet", "HEAD"])
            .is_err()
        {
            return Ok(Vec::new());
        }
        let limit_arg = limit.to_string();
        let format = format!(
            "--pretty=format:%H{SEP}%h{SEP}%an{SEP}%ae{SEP}%at{SEP}%s"
        );
        let raw = self.run(&["log", "-n", &limit_arg, &format])?;
        Ok(parse_log(&raw))
    }

    /// The unified diff of unstaged working-tree changes. With
    /// `path` set, the diff is restricted to that path.
    pub fn diff(&self, path: Option<&Path>) -> Result<String, GitError> {
        match path.and_then(Path::to_str) {
            Some(path) => self.run(&["diff", "--", path]),
            None => self.run(&["diff"]),
        }
    }

    /// The unified diff of staged (index vs `HEAD`) changes.
    pub fn diff_staged(&self, path: Option<&Path>) -> Result<String, GitError> {
        match path.and_then(Path::to_str) {
            Some(path) => self.run(&["diff", "--cached", "--", path]),
            None => self.run(&["diff", "--cached"]),
        }
    }

    /// The full patch a single commit introduced — `git show` of
    /// `rev` (a hash, `short_hash`, or any rev-spec). The output
    /// carries the commit metadata header followed by the unified
    /// diff, so the Git panel can render a commit's changes the same
    /// way it renders a working-tree diff.
    pub fn commit_diff(&self, rev: &str) -> Result<String, GitError> {
        // `--stat` first gives a per-file summary, then the patch;
        // `git` writes uncoloured output to a pipe so no `--no-color`
        // is needed (matching `diff` above).
        self.run(&["show", "--stat", "-p", rev])
    }
}

/// Parse the separator-delimited `git log` output into commits.
fn parse_log(raw: &str) -> Vec<Commit> {
    raw.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(SEP).collect();
            if fields.len() < 6 {
                return None;
            }
            Some(Commit {
                hash: fields[0].to_string(),
                short_hash: fields[1].to_string(),
                author: fields[2].to_string(),
                email: fields[3].to_string(),
                timestamp: fields[4].trim().parse().unwrap_or(0),
                summary: fields[5].to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_separator_delimited_log() {
        let raw = format!(
            "abc123{SEP}abc{SEP}Ada{SEP}ada@x.dev{SEP}1700000000{SEP}first commit\n\
             def456{SEP}def{SEP}Bo{SEP}bo@x.dev{SEP}1700000100{SEP}second: a, b"
        );
        let commits = parse_log(&raw);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].hash, "abc123");
        assert_eq!(commits[0].author, "Ada");
        assert_eq!(commits[0].timestamp, 1_700_000_000);
        // A summary containing commas survives — the separator is 0x1f.
        assert_eq!(commits[1].summary, "second: a, b");
    }

    #[test]
    fn skips_malformed_lines() {
        let raw = format!("only{SEP}three{SEP}fields\ngarbage");
        assert!(parse_log(&raw).is_empty());
    }

    #[test]
    fn empty_log_is_empty() {
        assert!(parse_log("").is_empty());
    }
}
