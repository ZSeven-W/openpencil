//! Branch merging, the shared integration classifier, and
//! merge-conflict handling.

use std::path::{Path, PathBuf};

use crate::worktree::MergeWorktree;
use crate::{GitError, GitRepo};

/// How an integration — a `merge` or the merge half of a `pull` —
/// resolved. Mirrors the outcome set the TS engine's
/// `engineBranchMerge` / `enginePull` report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Nothing to integrate — the target was already contained in
    /// the current branch (target unchanged, or the local branch is
    /// ahead of it).
    AlreadyUpToDate,
    /// The branch was fast-forwarded onto the target.
    FastForward,
    /// The histories had diverged; a merge commit was created.
    Merge,
    /// The merge stopped with conflicts left in the working tree.
    Conflict,
}

impl GitRepo {
    /// Merge `refname` (a branch, tag, or commit) into the current
    /// branch, classifying the outcome.
    pub fn merge(&self, refname: &str) -> Result<MergeOutcome, GitError> {
        let before = self.run(&["rev-parse", "HEAD"])?.trim().to_string();
        let target = self.run(&["rev-parse", refname])?.trim().to_string();
        self.integrate(&before, &target)
    }

    /// Integrate the already-resolved commit `target` into `before`
    /// (the current `HEAD`). Shared by [`GitRepo::merge`] and
    /// [`GitRepo::pull`].
    ///
    /// The decision is made entirely from commit ancestry, never
    /// from the post-integration `HEAD` shape:
    ///
    /// - `target` is already an ancestor of `before` (the branch
    ///   already contains it — including when the local branch is
    ///   *ahead*) → `AlreadyUpToDate`, no git mutation.
    /// - `before` is an ancestor of `target` → a fast-forward is
    ///   exact and sufficient.
    /// - neither is an ancestor of the other → the histories
    ///   diverged → a real merge commit (or a conflict).
    pub(crate) fn integrate(&self, before: &str, target: &str) -> Result<MergeOutcome, GitError> {
        // Refuse to start a new integration while a merge is still
        // unresolved. Otherwise the leftover conflict state would be
        // misattributed to *this* call, and the `AlreadyUpToDate`
        // short-circuits below would report success on a tree that
        // is mid-merge. The caller must `abort_merge` or
        // `complete_merge` first.
        if self.is_merging() {
            return Err(GitError::MergeInProgress);
        }
        if before == target {
            return Ok(MergeOutcome::AlreadyUpToDate);
        }
        // `target` already contained in `before` — local is ahead of
        // (or level with) the target. Merging it would be a no-op;
        // report it honestly rather than as a merge.
        if self.is_ancestor(target, before) {
            return Ok(MergeOutcome::AlreadyUpToDate);
        }
        // `before` is an ancestor of `target` — fast-forward.
        if self.is_ancestor(before, target) {
            self.run(&["merge", "--ff-only", target])?;
            return Ok(MergeOutcome::FastForward);
        }
        // Diverged histories — a merge commit is required.
        match self.run(&["merge", "--no-edit", target]) {
            Ok(_) => Ok(MergeOutcome::Merge),
            Err(err @ GitError::Command { .. }) => {
                // A merge that halts on conflicts leaves conflict
                // markers in the tree rather than failing cleanly.
                if self.status().map(|s| s.has_conflicts()).unwrap_or(false) {
                    Ok(MergeOutcome::Conflict)
                } else {
                    Err(err)
                }
            }
            Err(err) => Err(err),
        }
    }

    /// Whether commit `ancestor` is an ancestor of commit
    /// `descendant` (a commit is its own ancestor).
    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> bool {
        // `merge-base --is-ancestor` exits 0 when true, 1 when false;
        // a non-zero exit surfaces as `Err` from `run`.
        self.run(&["merge-base", "--is-ancestor", ancestor, descendant])
            .is_ok()
    }

    /// Whether a merge is currently in progress (`MERGE_HEAD` exists).
    pub fn is_merging(&self) -> bool {
        self.run(&["rev-parse", "--verify", "--quiet", "MERGE_HEAD"])
            .is_ok()
    }

    /// Abort an in-progress merge, restoring the pre-merge state.
    pub fn abort_merge(&self) -> Result<(), GitError> {
        self.run(&["merge", "--abort"])?;
        Ok(())
    }

    /// Repo-relative paths with unresolved merge conflicts.
    pub fn conflicted_files(&self) -> Result<Vec<String>, GitError> {
        let raw = self.run(&["diff", "--name-only", "--diff-filter=U"])?;
        Ok(raw
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect())
    }

    /// Mark `path` resolved by staging its current (resolved)
    /// content — `git add` of a once-conflicted file.
    pub fn mark_resolved(&self, path: &Path) -> Result<(), GitError> {
        self.stage(&[path])
    }

    /// Finalize an in-progress merge once every conflict is resolved
    /// and staged, keeping git's generated merge message.
    pub fn complete_merge(&self) -> Result<(), GitError> {
        self.run(&["commit", "--no-edit"])?;
        Ok(())
    }

    /// The three index-stage blobs of a conflicted `path` —
    /// `(base, ours, theirs)` from merge stages 1 / 2 / 3 of the
    /// index. A stage is `None` when it does not exist (an add/add
    /// conflict has no base stage). Valid only while a merge is in
    /// progress with `path` unresolved.
    pub fn conflict_stages(&self, path: &str) -> ConflictStages {
        let stage = |n: u8| self.run(&["show", &format!(":{n}:{path}")]).ok();
        ConflictStages {
            base: stage(1),
            ours: stage(2),
            theirs: stage(3),
        }
    }

    /// Merge `other` into the current branch through a throwaway
    /// worktree so the live working tree is never marked up.
    ///
    /// This is the orchestrator the in-app Git uses for branch
    /// merges: a normal `git merge` would write conflict markers
    /// straight into the open `.op` document. Here the merge is
    /// computed in a detached [`MergeWorktree`]; only a *clean*
    /// result is fast-forwarded back into the live tree, and a
    /// conflicting one is reported as a [`ConflictBag`] with the
    /// live tree left pristine.
    ///
    /// The live working tree must be clean — the caller commits or
    /// stashes first; otherwise [`GitError::WorkingTreeDirty`] is
    /// returned before any worktree is created.
    ///
    /// `resolve` is a structured-merge hook: when the worktree merge
    /// conflicts, it is called per conflicted file with
    /// `(path, base, ours, theirs)` blob contents and may return
    /// `Some(resolved)` to auto-resolve that file. When *every*
    /// conflict is resolved this way the merge is completed and
    /// fast-forwarded back like a clean merge; otherwise the
    /// still-conflicted residue is reported. Pass `|_, _, _, _| None`
    /// for the plain (no structured resolution) behaviour.
    pub fn merge_branch_isolated(
        &self,
        other: &str,
        resolve: impl Fn(&str, &str, &str, &str) -> Option<String>,
    ) -> Result<WorktreeMergeReport, GitError> {
        // A live merge already in progress would be misattributed.
        if self.is_merging() {
            return Err(GitError::MergeInProgress);
        }
        let head = self.run(&["rev-parse", "HEAD"])?.trim().to_string();
        let target = self.run(&["rev-parse", other])?.trim().to_string();

        // Ancestry short-circuits — no worktree needed, no mutation.
        if head == target || self.is_ancestor(&target, &head) {
            return Ok(WorktreeMergeReport::up_to_date());
        }

        // Bringing a clean merge back fast-forwards the live tree,
        // which a dirty tree cannot accept — refuse up front.
        if !self.status().map(|s| s.is_clean()).unwrap_or(false) {
            return Err(GitError::WorkingTreeDirty);
        }

        // Compute the merge in a detached worktree pinned at HEAD.
        let worktree = MergeWorktree::create(self, merge_worktree_dir(), &head)?;
        let wrepo = worktree.repo();

        // Fast-forward: exact, never conflicts.
        if self.is_ancestor(&head, &target) {
            wrepo.run(&["merge", "--ff-only", &target])?;
            let merged = wrepo.run(&["rev-parse", "HEAD"])?.trim().to_string();
            self.run(&["merge", "--ff-only", &merged])?;
            return Ok(WorktreeMergeReport::clean(
                MergeOutcome::FastForward,
                merged,
            ));
        }

        // Diverged histories — a real merge commit, or conflicts.
        match wrepo.run(&["merge", "--no-edit", &target]) {
            Ok(_) => {
                let merged = wrepo.run(&["rev-parse", "HEAD"])?.trim().to_string();
                // The worktree built a merge commit on top of `head`;
                // the live branch can fast-forward onto it exactly.
                self.run(&["merge", "--ff-only", &merged])?;
                Ok(WorktreeMergeReport::clean(MergeOutcome::Merge, merged))
            }
            Err(err @ GitError::Command { .. }) => {
                let bag = collect_conflicts(wrepo)?;
                if bag.is_empty() {
                    // A non-conflict failure (e.g. an unrelated
                    // history) — surface it rather than swallow it.
                    return Err(err);
                }
                // Offer each conflicted file to the structured
                // resolver; write back + stage whatever it resolves.
                for file in &bag.files {
                    let Some(stages) = &file.stages else {
                        continue;
                    };
                    let resolved = resolve(
                        &file.path,
                        stages.base.as_deref().unwrap_or(""),
                        stages.ours.as_deref().unwrap_or(""),
                        stages.theirs.as_deref().unwrap_or(""),
                    );
                    if let Some(content) = resolved {
                        let abs = wrepo.workdir().join(&file.path);
                        std::fs::write(&abs, content).map_err(|e| GitError::Io(e.to_string()))?;
                        wrepo.stage(&[abs.as_path()])?;
                    }
                }
                // Anything still unmerged after that?
                let residue = collect_conflicts(wrepo)?;
                if residue.is_empty() {
                    // Every conflict was structurally auto-resolved —
                    // complete the merge and fast-forward it back.
                    wrepo.complete_merge()?;
                    let merged = wrepo.run(&["rev-parse", "HEAD"])?.trim().to_string();
                    self.run(&["merge", "--ff-only", &merged])?;
                    return Ok(WorktreeMergeReport::clean(MergeOutcome::Merge, merged));
                }
                // Abort the worktree's half-merge; the worktree drop
                // then removes the directory entirely. The live tree
                // was never touched.
                let _ = wrepo.abort_merge();
                Ok(WorktreeMergeReport::conflicted(residue))
            }
            Err(err) => Err(err),
        }
        // `worktree` drops here → the throwaway worktree is removed.
    }
}

/// How a single path conflicts in a merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// Both sides changed the file's content.
    BothModified,
    /// Both sides added the path with differing content.
    BothAdded,
    /// One side deleted the file while the other changed it.
    DeleteModify,
    /// An unmerged state outside the common cases above.
    Other,
}

/// The three index-stage blobs of a conflicted file — `base` is
/// merge-stage 1, `ours` stage 2, `theirs` stage 3. A stage is
/// `None` when it does not exist (an add/add conflict has no base).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConflictStages {
    /// The merge-base revision of the file.
    pub base: Option<String>,
    /// The current branch's revision.
    pub ours: Option<String>,
    /// The merged-in branch's revision.
    pub theirs: Option<String>,
}

/// One conflicted path left by a worktree merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictedFile {
    /// Repo-relative path.
    pub path: String,
    /// How the path conflicts.
    pub kind: ConflictKind,
    /// The three merge-stage blobs — populated for `.op` documents
    /// so the caller can run a structured node-level merge; `None`
    /// for other files.
    pub stages: Option<ConflictStages>,
}

/// The set of unresolved conflicts a worktree merge produced.
///
/// File-granular today; node-level (per-`.op`-node) conflict
/// detail is the deeper, still-pending increment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConflictBag {
    /// Conflicted paths, repository order.
    pub files: Vec<ConflictedFile>,
}

impl ConflictBag {
    /// Whether the bag holds no conflicts.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Number of conflicted paths.
    pub fn len(&self) -> usize {
        self.files.len()
    }
}

/// The outcome of a worktree-isolated branch merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeMergeReport {
    /// How the merge resolved.
    pub outcome: MergeOutcome,
    /// Conflicts left by the merge — empty unless `outcome` is
    /// [`MergeOutcome::Conflict`].
    pub conflicts: ConflictBag,
    /// The merged commit hash on a clean merge; `None` otherwise.
    pub merged_commit: Option<String>,
}

impl WorktreeMergeReport {
    /// A no-op report — the target was already integrated.
    fn up_to_date() -> Self {
        Self {
            outcome: MergeOutcome::AlreadyUpToDate,
            conflicts: ConflictBag::default(),
            merged_commit: None,
        }
    }

    /// A clean merge / fast-forward report.
    fn clean(outcome: MergeOutcome, merged_commit: String) -> Self {
        Self {
            outcome,
            conflicts: ConflictBag::default(),
            merged_commit: Some(merged_commit),
        }
    }

    /// A conflicting-merge report — the live tree was left pristine.
    fn conflicted(conflicts: ConflictBag) -> Self {
        Self {
            outcome: MergeOutcome::Conflict,
            conflicts,
            merged_commit: None,
        }
    }
}

/// A unique throwaway-worktree directory under the system temp dir.
fn merge_worktree_dir() -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("op-git-merge-{}-{nanos}", std::process::id()))
}

/// Build a [`ConflictBag`] from a conflicted worktree's porcelain
/// status. Only unmerged (`U`-coded) entries are collected.
///
/// `--porcelain=v1 -z` is used deliberately: it emits NUL-terminated
/// records with byte-exact, *unquoted* paths, so a path containing
/// spaces — or leading / trailing whitespace — survives intact (the
/// space-delimited, sometimes-quoted default format would not).
fn collect_conflicts(repo: &GitRepo) -> Result<ConflictBag, GitError> {
    let raw = repo.run(&["status", "--porcelain=v1", "-z"])?;
    let mut files = Vec::new();
    let mut records = raw.split('\0');
    while let Some(record) = records.next() {
        // `XY <path>` — two status codes, a space, then the path.
        // The trailing split element after the final NUL is empty.
        if record.len() < 4 {
            continue;
        }
        let xy = record.as_bytes();
        // Rename / copy entries carry a second NUL-delimited field
        // (the original path). Consume it so it is not misread as a
        // standalone status record on the next iteration.
        if xy[0] == b'R' || xy[0] == b'C' || xy[1] == b'R' || xy[1] == b'C' {
            let _ = records.next();
        }
        // Unmerged porcelain codes — see `git status` docs. Byte 3
        // onward is the path; bytes 0..3 are pure ASCII, so the
        // slice always falls on a char boundary.
        let kind = match &record[..2] {
            "UU" => ConflictKind::BothModified,
            "AA" => ConflictKind::BothAdded,
            "DD" | "DU" | "UD" => ConflictKind::DeleteModify,
            "AU" | "UA" => ConflictKind::Other,
            // Not an unmerged entry — skip it.
            _ => continue,
        };
        let path = record[3..].to_string();
        // `.op` documents carry their three merge-stage blobs so the
        // caller can run a structured node-level merge.
        let stages = path.ends_with(".op").then(|| repo.conflict_stages(&path));
        files.push(ConflictedFile { path, kind, stages });
    }
    Ok(ConflictBag { files })
}
