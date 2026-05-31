//! Branch merging, the shared integration classifier, and
//! merge-conflict handling.
//!
//! Backed by in-process `libgit2` (`git2`) rather than the system
//! `git` executable: the merge analysis, fast-forward, merge-commit
//! creation, conflict inspection and abort all run against the
//! `git2::Repository` opened fresh for each call. The worktree-merge
//! orchestration still goes through [`MergeWorktree`] so a
//! conflicting merge is computed in a throwaway worktree and never
//! marks up the live `.op` document.

use std::path::{Path, PathBuf};

use git2::{build::CheckoutBuilder, Oid, Repository, RepositoryState, ResetType};

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
        let repo = self.open()?;
        let before = repo.head()?.peel_to_commit()?.id().to_string();
        let target = repo
            .revparse_single(refname)?
            .peel_to_commit()?
            .id()
            .to_string();
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
            let repo = self.open()?;
            let target_oid = Oid::from_str(target)?;
            fast_forward(&repo, target_oid)?;
            return Ok(MergeOutcome::FastForward);
        }
        // Diverged histories — a merge commit is required.
        let repo = self.open()?;
        let before_oid = Oid::from_str(before)?;
        let target_oid = Oid::from_str(target)?;
        merge_commit_or_conflict(&repo, before_oid, target_oid)
    }

    /// Whether commit `ancestor` is an ancestor of commit
    /// `descendant` (a commit is its own ancestor).
    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> bool {
        // `git2::Repository::graph_descendant_of(descendant, ancestor)`
        // is the libgit2 equivalent of `merge-base --is-ancestor`, but
        // it returns `false` when the two commits are equal — so the
        // "a commit is its own ancestor" case is handled explicitly to
        // preserve the subprocess behaviour.
        if ancestor == descendant {
            return true;
        }
        let Ok(repo) = self.open() else {
            return false;
        };
        let (Ok(anc), Ok(desc)) = (Oid::from_str(ancestor), Oid::from_str(descendant)) else {
            return false;
        };
        repo.graph_descendant_of(desc, anc).unwrap_or(false)
    }

    /// Whether a merge is currently in progress (`MERGE_HEAD` exists).
    pub fn is_merging(&self) -> bool {
        let Ok(repo) = self.open() else {
            return false;
        };
        // `RepositoryState::Merge` is set whenever `MERGE_HEAD` is
        // present; fall back to a direct ref probe in case the state
        // read is ambiguous.
        repo.state() == RepositoryState::Merge || repo.find_reference("MERGE_HEAD").is_ok()
    }

    /// Abort an in-progress merge, restoring the pre-merge state.
    pub fn abort_merge(&self) -> Result<(), GitError> {
        let repo = self.open()?;
        // `git merge --abort` is `git reset --hard` followed by a
        // clear of the merge metadata. A hard reset to `HEAD` throws
        // away the conflicted working-tree + index content, then
        // `cleanup_state` removes `MERGE_HEAD` / `MERGE_MSG`.
        let head = repo.head()?.peel_to_commit()?;
        let mut checkout = CheckoutBuilder::new();
        checkout.force();
        repo.reset(head.as_object(), ResetType::Hard, Some(&mut checkout))?;
        repo.cleanup_state()?;
        Ok(())
    }

    /// Repo-relative paths with unresolved merge conflicts.
    pub fn conflicted_files(&self) -> Result<Vec<String>, GitError> {
        let repo = self.open()?;
        let index = repo.index()?;
        let mut paths = Vec::new();
        for conflict in index.conflicts()? {
            let conflict = conflict?;
            // The path is identical across the three stages; take it
            // from whichever stage exists (a delete/modify conflict is
            // missing one of `our` / `their`).
            if let Some(path) = conflict_path(&conflict) {
                if !paths.contains(&path) {
                    paths.push(path);
                }
            }
        }
        Ok(paths)
    }

    /// Mark `path` resolved by staging its current (resolved)
    /// content into the index — the libgit2 equivalent of `git add`
    /// of a once-conflicted file, which also clears the path's
    /// conflict entry (moving it to the index's resolve-undo section).
    pub fn mark_resolved(&self, path: &Path) -> Result<(), GitError> {
        let repo = self.open()?;
        let mut index = repo.index()?;
        // `Index::add_path` needs a path relative to the work tree.
        let rel = self.repo_relative(path);
        index.add_path(&rel)?;
        index.write()?;
        Ok(())
    }

    /// Finalize an in-progress merge once every conflict is resolved
    /// and staged, keeping git's generated merge message.
    pub fn complete_merge(&self) -> Result<(), GitError> {
        let repo = self.open()?;
        // The two parents: `HEAD` (ours) and `MERGE_HEAD` (theirs).
        let ours = repo.head()?.peel_to_commit()?;
        let merge_head_oid =
            repo.find_reference("MERGE_HEAD")?
                .target()
                .ok_or_else(|| GitError::Command {
                    operation: "commit".to_string(),
                    stderr: "MERGE_HEAD does not resolve to a commit".to_string(),
                })?;
        let theirs = repo.find_commit(merge_head_oid)?;

        // Write the (resolved) index out to a tree; a lingering
        // conflict makes `write_tree` fail, which is the correct
        // refusal to commit an unresolved merge.
        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;

        // Keep git's generated merge message (`MERGE_MSG`) when present,
        // matching `git commit --no-edit`; otherwise synthesize one.
        let message =
            read_merge_msg(&repo).unwrap_or_else(|| format!("Merge commit '{}'", theirs.id()));

        let sig = repo.signature()?;
        repo.commit(Some("HEAD"), &sig, &sig, &message, &tree, &[&ours, &theirs])?;
        repo.cleanup_state()?;
        Ok(())
    }

    /// The three index-stage blobs of a conflicted `path` —
    /// `(base, ours, theirs)` from merge stages 1 / 2 / 3 of the
    /// index. A stage is `None` when it does not exist (an add/add
    /// conflict has no base stage). Valid only while a merge is in
    /// progress with `path` unresolved.
    pub fn conflict_stages(&self, path: &str) -> ConflictStages {
        // Any failure to open the repo / read the index yields the
        // empty (all-`None`) stage set, matching the subprocess
        // version's "`git show` failed → `None`" behaviour.
        let Ok(repo) = self.open() else {
            return ConflictStages::default();
        };
        let Ok(index) = repo.index() else {
            return ConflictStages::default();
        };
        let stage = |n: i32| stage_blob(&repo, &index, path, n);
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
        let repo = self.open()?;
        let head = repo.head()?.peel_to_commit()?.id().to_string();
        let target = repo
            .revparse_single(other)?
            .peel_to_commit()?
            .id()
            .to_string();
        drop(repo);

        // Ancestry short-circuits — no worktree needed, no mutation.
        if head == target || self.is_ancestor(&target, &head) {
            return Ok(WorktreeMergeReport::up_to_date());
        }

        // Bringing a clean merge back fast-forwards the live tree,
        // which a dirty tree cannot accept — refuse up front.
        if !self.status().map(|s| s.is_clean()).unwrap_or(false) {
            return Err(GitError::WorkingTreeDirty);
        }

        let head_oid = Oid::from_str(&head)?;
        let target_oid = Oid::from_str(&target)?;

        // Compute the merge in a detached worktree pinned at HEAD.
        let worktree = MergeWorktree::create(self, merge_worktree_dir(), &head)?;
        let wgit = worktree.repo();

        // Fast-forward: exact, never conflicts. The worktree would
        // simply land on `target`, so the live branch can advance
        // straight onto `target`.
        if self.is_ancestor(&head, &target) {
            let live = self.open()?;
            fast_forward(&live, target_oid)?;
            return Ok(WorktreeMergeReport::clean(
                MergeOutcome::FastForward,
                target,
            ));
        }

        // Diverged histories — a real merge commit, or conflicts. Run
        // the merge inside the worktree so any markers stay quarantined.
        let wrepo = wgit.open()?;
        let merged = match merge_in_worktree(&wrepo, head_oid, target_oid)? {
            WorktreeMergeResult::Clean(merged) => merged,
            WorktreeMergeResult::Conflicts => {
                let bag = collect_conflicts(wgit)?;
                if bag.is_empty() {
                    // `merge_in_worktree` reported conflicts but the
                    // index shows none — treat as a non-conflict
                    // failure and surface it rather than swallow it.
                    let _ = abort_in_worktree(&wrepo);
                    return Err(GitError::Command {
                        operation: "merge".to_string(),
                        stderr: "merge halted without a recorded conflict".to_string(),
                    });
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
                        let abs = wgit.workdir().join(&file.path);
                        std::fs::write(&abs, content).map_err(|e| GitError::Io(e.to_string()))?;
                        // Stage the resolved content into the worktree
                        // index, which also clears the conflict entry.
                        let mut index = wrepo.index()?;
                        index.add_path(Path::new(&file.path))?;
                        index.write()?;
                    }
                }
                // Anything still unmerged after that?
                let residue = collect_conflicts(wgit)?;
                if !residue.is_empty() {
                    // Abort the worktree's half-merge; the worktree drop
                    // then removes the directory entirely. The live tree
                    // was never touched.
                    let _ = abort_in_worktree(&wrepo);
                    return Ok(WorktreeMergeReport::conflicted(residue));
                }
                // Every conflict was structurally auto-resolved —
                // complete the merge in the worktree.
                complete_in_worktree(&wrepo, head_oid, target_oid)?
            }
        };

        // The worktree built (or fast-forwarded to) a commit on top of
        // `head`; the live branch can fast-forward onto it exactly.
        let live = self.open()?;
        let merged_oid = Oid::from_str(&merged)?;
        fast_forward(&live, merged_oid)?;
        Ok(WorktreeMergeReport::clean(MergeOutcome::Merge, merged))
        // `worktree` drops here → the throwaway worktree is removed.
    }

    /// `path` made relative to the repository's work tree. An absolute
    /// path under the work tree is stripped to its relative remainder;
    /// an already-relative path is returned unchanged.
    fn repo_relative(&self, path: &Path) -> PathBuf {
        path.strip_prefix(self.workdir())
            .map(Path::to_path_buf)
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

/// Fast-forward `repo`'s current `HEAD` to commit `oid`: check the
/// target tree out into the working directory + index, then move the
/// ref. An attached branch keeps its name (the branch ref advances);
/// a detached `HEAD` (as in a throwaway worktree) is re-pointed at the
/// commit directly.
///
/// The checkout is **SAFE**, not forced. libgit2 updates the work tree
/// to the target but REFUSES — returning `GIT_ECONFLICT` *without
/// applying any change* — when an update would overwrite local work: a
/// modified tracked file, or an untracked file / directory in the way
/// (every collision shape, honoring `core.ignorecase`). That refusal is
/// mapped to [`GitError::WorkingTreeDirty`] so a fast-forward can never
/// silently discard the user's edits — exactly the guard the subprocess
/// `git pull` gave with "local changes / untracked working tree files
/// would be overwritten by merge".
fn fast_forward(repo: &Repository, oid: Oid) -> Result<(), GitError> {
    let commit = repo.find_commit(oid)?;
    let mut checkout = CheckoutBuilder::new(); // SAFE by default
    match repo.checkout_tree(commit.as_object(), Some(&mut checkout)) {
        Ok(()) => {}
        Err(e) if e.code() == git2::ErrorCode::Conflict => {
            return Err(GitError::WorkingTreeDirty);
        }
        Err(e) => return Err(e.into()),
    }
    match repo.head() {
        Ok(mut head_ref) if head_ref.is_branch() => {
            head_ref.set_target(oid, "fast-forward")?;
        }
        // Detached HEAD (worktree) or no current branch — point HEAD
        // straight at the commit.
        _ => {
            repo.set_head_detached(oid)?;
        }
    }
    Ok(())
}

/// Run a real merge of `target` into `before` against `repo`'s live
/// `HEAD`, producing a merge commit on a clean merge or leaving the
/// repository in its conflicted merging state (returning
/// [`MergeOutcome::Conflict`]) otherwise.
fn merge_commit_or_conflict(
    repo: &Repository,
    before: Oid,
    target: Oid,
) -> Result<MergeOutcome, GitError> {
    let annotated = repo.find_annotated_commit(target)?;
    let mut checkout = CheckoutBuilder::new();
    checkout.safe();
    repo.merge(&[&annotated], None, Some(&mut checkout))?;

    let mut index = repo.index()?;
    if index.has_conflicts() {
        // Conflicts left in the working tree + index. Leave the merge
        // in progress (matching `git merge` halting on conflicts) so
        // the caller can inspect / abort / resolve it.
        return Ok(MergeOutcome::Conflict);
    }

    // Clean merge — write the merged index out as the commit's tree
    // and create the two-parent merge commit, then clear the merge
    // metadata so the tree is no longer mid-merge.
    let tree_oid = index.write_tree()?;
    let tree = repo.find_tree(tree_oid)?;
    let ours = repo.find_commit(before)?;
    let theirs = repo.find_commit(target)?;
    let sig = repo.signature()?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Merge commit '{target}'"),
        &tree,
        &[&ours, &theirs],
    )?;
    repo.cleanup_state()?;
    Ok(MergeOutcome::Merge)
}

/// The result of computing a merge inside a throwaway worktree.
enum WorktreeMergeResult {
    /// A clean merge — the merge commit's hash.
    Clean(String),
    /// The merge halted on conflicts left in the worktree index.
    Conflicts,
}

/// Run a merge of `target` into the worktree's detached `HEAD`
/// (pinned at `head`). On a clean merge it commits the result and
/// returns its hash; on conflicts it leaves the worktree mid-merge
/// and returns [`WorktreeMergeResult::Conflicts`].
fn merge_in_worktree(
    wrepo: &Repository,
    head: Oid,
    target: Oid,
) -> Result<WorktreeMergeResult, GitError> {
    let annotated = wrepo.find_annotated_commit(target)?;
    let mut checkout = CheckoutBuilder::new();
    checkout.safe();
    wrepo.merge(&[&annotated], None, Some(&mut checkout))?;

    if wrepo.index()?.has_conflicts() {
        return Ok(WorktreeMergeResult::Conflicts);
    }
    let merged = commit_worktree_merge(wrepo, head, target)?;
    Ok(WorktreeMergeResult::Clean(merged))
}

/// Commit a fully-resolved worktree merge (every conflict already
/// staged) — the path taken once the structured resolver has cleared
/// the last conflict. Returns the merge commit's hash.
fn complete_in_worktree(wrepo: &Repository, head: Oid, target: Oid) -> Result<String, GitError> {
    commit_worktree_merge(wrepo, head, target)
}

/// Write the worktree's (resolved) index out as a tree and create the
/// two-parent merge commit on the detached `HEAD`, then clear the
/// merge metadata. Returns the new commit's hash.
fn commit_worktree_merge(wrepo: &Repository, head: Oid, target: Oid) -> Result<String, GitError> {
    let mut index = wrepo.index()?;
    let tree_oid = index.write_tree()?;
    let tree = wrepo.find_tree(tree_oid)?;
    let ours = wrepo.find_commit(head)?;
    let theirs = wrepo.find_commit(target)?;
    let sig = wrepo.signature()?;
    let merged = wrepo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Merge commit '{target}'"),
        &tree,
        &[&ours, &theirs],
    )?;
    wrepo.cleanup_state()?;
    Ok(merged.to_string())
}

/// Abort a worktree's half-finished merge — hard-reset to its `HEAD`
/// and clear the merge metadata so the worktree is no longer mid-merge.
fn abort_in_worktree(wrepo: &Repository) -> Result<(), GitError> {
    let head = wrepo.head()?.peel_to_commit()?;
    let mut checkout = CheckoutBuilder::new();
    checkout.force();
    wrepo.reset(head.as_object(), ResetType::Hard, Some(&mut checkout))?;
    wrepo.cleanup_state()?;
    Ok(())
}

/// Read git's generated `MERGE_MSG`, the message `git commit --no-edit`
/// keeps for a merge commit. `None` when it is absent or unreadable.
fn read_merge_msg(repo: &Repository) -> Option<String> {
    let path = repo.path().join("MERGE_MSG");
    let text = std::fs::read_to_string(path).ok()?;
    let trimmed = text.trim_end();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// The blob content of merge stage `n` (1 = base, 2 = ours, 3 =
/// theirs) for `path` in `index`, decoded as UTF-8 (lossy). `None`
/// when that stage does not exist for the path.
fn stage_blob(repo: &Repository, index: &git2::Index, path: &str, n: i32) -> Option<String> {
    let entry = index.get_path(Path::new(path), n)?;
    let blob = repo.find_blob(entry.id).ok()?;
    Some(String::from_utf8_lossy(blob.content()).into_owned())
}

/// The repo-relative path of a conflict — taken from whichever of its
/// three stage entries exists (a delete/modify conflict is missing one).
fn conflict_path(conflict: &git2::IndexConflict) -> Option<String> {
    let entry = conflict
        .our
        .as_ref()
        .or(conflict.their.as_ref())
        .or(conflict.ancestor.as_ref())?;
    Some(String::from_utf8_lossy(&entry.path).into_owned())
}

/// Classify a conflict from which of its three stage entries are
/// present — the libgit2 counterpart of the porcelain `XY` codes the
/// subprocess version read (`UU` / `AA` / `DU`-`UD`-`DD` / other).
fn conflict_kind(conflict: &git2::IndexConflict) -> ConflictKind {
    let has_ancestor = conflict.ancestor.is_some();
    let has_our = conflict.our.is_some();
    let has_their = conflict.their.is_some();
    match (has_ancestor, has_our, has_their) {
        // Both sides changed a file that existed at the base — `UU`.
        (true, true, true) => ConflictKind::BothModified,
        // Both sides added the path with no common base — `AA`.
        (false, true, true) => ConflictKind::BothAdded,
        // One side deleted while the other kept/changed it — `DU` /
        // `UD` / `DD`.
        (_, false, true) | (_, true, false) => ConflictKind::DeleteModify,
        // Anything else (including a stageless record) — `Other`.
        _ => ConflictKind::Other,
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

/// Build a [`ConflictBag`] from a conflicted worktree's index. Only
/// the index's unmerged (conflict) entries are collected, in the order
/// the conflict iterator yields them.
///
/// `.op` documents carry their three merge-stage blobs so the caller
/// can run a structured node-level merge; other files do not.
fn collect_conflicts(repo: &GitRepo) -> Result<ConflictBag, GitError> {
    let git = repo.open()?;
    let index = git.index()?;
    let mut files = Vec::new();
    for conflict in index.conflicts()? {
        let conflict = conflict?;
        let Some(path) = conflict_path(&conflict) else {
            continue;
        };
        let kind = conflict_kind(&conflict);
        // `.op` documents carry their three merge-stage blobs so the
        // caller can run a structured node-level merge.
        let stages = path.ends_with(".op").then(|| repo.conflict_stages(&path));
        files.push(ConflictedFile { path, kind, stages });
    }
    Ok(ConflictBag { files })
}
