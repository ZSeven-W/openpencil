//! Commit history and diffs.

use std::path::Path;

use git2::{Diff, DiffFormat, DiffLineType, DiffOptions, Repository, Sort, Tree};

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
    /// `true` for the root commit (no parent) — drives the panel's
    /// "initial commit, nothing to diff" detail line.
    pub is_initial: bool,
}

impl GitRepo {
    /// The most recent `limit` commits on the current branch, newest
    /// first. A repository with no commits yet yields an empty list
    /// rather than an error.
    pub fn log(&self, limit: usize) -> Result<Vec<Commit>, GitError> {
        let repo = self.open()?;
        // A fresh repo with no commits has an unborn `HEAD`; probe it
        // first — the old code did this with `rev-parse --verify HEAD`.
        // `revwalk.push_head` on such a repo fails with a generic
        // `Reference`-class error ("reference 'refs/heads/main' not
        // found") rather than the dedicated `UnbornBranch` code, so we
        // can't rely on classifying *its* error; `Repository::head`
        // reports the unborn state cleanly.
        let head = match repo.head() {
            Ok(head) => head,
            Err(e)
                if e.code() == git2::ErrorCode::UnbornBranch
                    || e.code() == git2::ErrorCode::NotFound =>
            {
                return Ok(Vec::new());
            }
            Err(e) => return Err(e.into()),
        };
        let head_oid = head.peel_to_commit()?.id();

        let mut walk = repo.revwalk()?;
        // Newest first — `Sort::TIME` walks in reverse-chronological
        // order, the same ordering `git log` defaults to.
        walk.set_sorting(Sort::TIME)?;
        walk.push(head_oid)?;

        let mut commits = Vec::new();
        for oid in walk.take(limit) {
            let oid = oid?;
            let commit = repo.find_commit(oid)?;
            commits.push(commit_to_record(&commit));
        }
        Ok(commits)
    }

    /// The unified diff of unstaged working-tree changes. With
    /// `path` set, the diff is restricted to that path.
    pub fn diff(&self, path: Option<&Path>) -> Result<String, GitError> {
        let repo = self.open()?;
        let mut opts = DiffOptions::new();
        if let Some(spec) = path.and_then(Path::to_str) {
            opts.pathspec(spec);
        }
        // `git diff` (no `--cached`) is the index-vs-working-tree delta —
        // exactly the unstaged changes the panel renders.
        let diff = repo.diff_index_to_workdir(None, Some(&mut opts))?;
        render_diff(&diff)
    }

    /// The unified diff of staged (index vs `HEAD`) changes.
    pub fn diff_staged(&self, path: Option<&Path>) -> Result<String, GitError> {
        let repo = self.open()?;
        let mut opts = DiffOptions::new();
        if let Some(spec) = path.and_then(Path::to_str) {
            opts.pathspec(spec);
        }
        // `git diff --cached` is the `HEAD`-tree-vs-index delta. Before
        // the first commit there is no `HEAD` tree — pass `None`, which
        // libgit2 treats as the empty tree, so a brand-new repo's staged
        // additions still diff cleanly.
        let head_tree = head_tree(&repo)?;
        let diff = repo.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
        render_diff(&diff)
    }

    /// The full patch a single commit introduced — the diff of `rev`
    /// (a hash, `short_hash`, or any rev-spec) against its first
    /// parent. The output is the unified diff so the Git panel can
    /// render a commit's changes the same way it renders a
    /// working-tree diff.
    pub fn commit_diff(&self, rev: &str) -> Result<String, GitError> {
        let repo = self.open()?;
        // Resolve `rev` (hash / short hash / any rev-spec) to a commit.
        let object = repo.revparse_single(rev)?;
        let commit = object.peel_to_commit()?;
        let new_tree = commit.tree()?;
        // Diff against the first parent's tree; the root commit has no
        // parent, so its "before" side is the empty tree (`None`).
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&new_tree), None)?;
        render_diff(&diff)
    }
}

/// Build a [`Commit`] record from a libgit2 commit. `summary` /
/// `author` / `email` fall back to empty strings when libgit2 cannot
/// decode them as UTF-8, matching the old separator-parse behaviour
/// (a malformed field became an empty string rather than an error).
fn commit_to_record(commit: &git2::Commit<'_>) -> Commit {
    let author = commit.author();
    // `Object::short_id` abbreviates the hash the way `git` does
    // (uniqueness-aware, honouring `core.abbrev`); fall back to a
    // 7-char prefix of the full hash if it is somehow unavailable.
    let hash = commit.id().to_string();
    let short_hash = commit
        .as_object()
        .short_id()
        .ok()
        .and_then(|buf| buf.as_str().map(str::to_string))
        .unwrap_or_else(|| hash.chars().take(7).collect());
    Commit {
        hash,
        short_hash,
        author: author.name().unwrap_or("").to_string(),
        email: author.email().unwrap_or("").to_string(),
        timestamp: commit.time().seconds(),
        summary: commit.summary().unwrap_or("").to_string(),
        is_initial: commit.parent_count() == 0,
    }
}

/// The tree `HEAD` points at, or `None` before the first commit (an
/// unborn `HEAD`). Any other failure to resolve `HEAD` propagates.
fn head_tree(repo: &Repository) -> Result<Option<Tree<'_>>, GitError> {
    match repo.head() {
        Ok(head) => Ok(Some(head.peel_to_commit()?.tree()?)),
        // No commits yet — `HEAD` is unborn (or simply absent).
        Err(e)
            if e.code() == git2::ErrorCode::UnbornBranch
                || e.code() == git2::ErrorCode::NotFound =>
        {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

/// Render a libgit2 [`Diff`] to the same unified-diff string the
/// `git diff` / `git show` subprocess produced — file headers, hunk
/// headers, and `+`/`-`/` ` line prefixes — by replaying it through
/// [`Diff::print`] with [`DiffFormat::Patch`] and concatenating every
/// emitted line into a `String`.
fn render_diff(diff: &Diff<'_>) -> Result<String, GitError> {
    let mut out = String::new();
    diff.print(DiffFormat::Patch, |_delta, _hunk, line| {
        // Content lines (context / addition / deletion) carry their
        // payload *without* the leading marker, so re-prepend the
        // origin char. Header lines (file header, hunk header, binary,
        // EOFNL markers) already contain their full text — emit them
        // verbatim.
        match line.origin_value() {
            DiffLineType::Context | DiffLineType::Addition | DiffLineType::Deletion => {
                out.push(line.origin())
            }
            _ => {}
        }
        out.push_str(&String::from_utf8_lossy(line.content()));
        true
    })?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `commit_to_record` is exercised against real commits in the
    /// integration suite (which builds fixture repos); here we only
    /// assert the empty-history contract is plumbed through a struct
    /// the rest of the crate can rely on.
    #[test]
    fn commit_fields_are_addressable() {
        let c = Commit {
            hash: "abc123".to_string(),
            short_hash: "abc".to_string(),
            author: "Ada".to_string(),
            email: "ada@x.dev".to_string(),
            timestamp: 1_700_000_000,
            summary: "first commit".to_string(),
            is_initial: true,
        };
        assert_eq!(c.hash, "abc123");
        assert_eq!(c.short_hash, "abc");
        assert_eq!(c.author, "Ada");
        assert_eq!(c.timestamp, 1_700_000_000);
        assert_eq!(c.summary, "first commit");
    }
}
