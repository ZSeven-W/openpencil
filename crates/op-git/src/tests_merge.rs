//! Merge tests: `pull` outcome classification, in-place `merge`,
//! and the isolated worktree-based merge orchestrator.

use std::process::Command;

use crate::tests::{clone_for_test, git_available, unique_temp_dir, TempRepo};
use crate::{ConflictKind, GitError, GitRepo, MergeOutcome};

#[test]
fn pull_classifies_up_to_date_then_fast_forward() {
    if !git_available() {
        return;
    }
    // A bare repo standing in for the "remote".
    let remote = unique_temp_dir("pull-remote");
    Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&remote)
        .output()
        .expect("init bare remote");

    // Clone A seeds the remote with an initial commit.
    let (a_dir, a) = clone_for_test(&remote, "pull-a");
    std::fs::write(a_dir.join("a.op"), "1").unwrap();
    a.stage_all().unwrap();
    a.commit("init").unwrap();
    a.run(&["push", "-u", "origin", "main"]).unwrap();

    // Clone B has nothing to pull → AlreadyUpToDate.
    let (b_dir, b) = clone_for_test(&remote, "pull-b");
    assert_eq!(b.pull().expect("pull"), MergeOutcome::AlreadyUpToDate);

    // A pushes a new commit; B's pull fast-forwards.
    std::fs::write(a_dir.join("a.op"), "2").unwrap();
    a.stage_all().unwrap();
    a.commit("update").unwrap();
    a.push().unwrap();
    assert_eq!(b.pull().expect("pull"), MergeOutcome::FastForward);

    for dir in [remote, a_dir, b_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn pull_refuses_to_fast_forward_over_a_dirty_tree() {
    if !git_available() {
        return;
    }
    let remote = unique_temp_dir("ff-dirty-remote");
    Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&remote)
        .output()
        .expect("init bare remote");

    let (a_dir, a) = clone_for_test(&remote, "ff-dirty-a");
    std::fs::write(a_dir.join("a.op"), "1").unwrap();
    a.stage_all().unwrap();
    a.commit("init").unwrap();
    a.run(&["push", "-u", "origin", "main"]).unwrap();

    let (b_dir, b) = clone_for_test(&remote, "ff-dirty-b");

    // A publishes a new commit — B *could* fast-forward.
    std::fs::write(a_dir.join("a.op"), "2").unwrap();
    a.stage_all().unwrap();
    a.commit("update").unwrap();
    a.push().unwrap();

    // B has an UNCOMMITTED edit to a tracked file. A fast-forward would
    // force-overwrite it, so the pull must refuse (WorkingTreeDirty)
    // rather than silently discard the local work — the data-loss
    // regression the libgit2 migration introduced + this guard fixes.
    std::fs::write(b_dir.join("a.op"), "local-uncommitted").unwrap();
    assert!(
        matches!(b.pull(), Err(GitError::WorkingTreeDirty)),
        "a fast-forward over a dirty tree must be refused, not forced"
    );
    assert_eq!(
        std::fs::read_to_string(b_dir.join("a.op")).unwrap(),
        "local-uncommitted",
        "the local uncommitted edit must survive the refused pull"
    );

    for dir in [remote, a_dir, b_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn pull_refuses_to_fast_forward_over_a_colliding_untracked_file() {
    if !git_available() {
        return;
    }
    let remote = unique_temp_dir("ff-untracked-remote");
    Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&remote)
        .output()
        .expect("init bare remote");

    let (a_dir, a) = clone_for_test(&remote, "ff-untracked-a");
    std::fs::write(a_dir.join("a.op"), "1").unwrap();
    a.stage_all().unwrap();
    a.commit("init").unwrap();
    a.run(&["push", "-u", "origin", "main"]).unwrap();

    let (b_dir, b) = clone_for_test(&remote, "ff-untracked-b");

    // A adds a NEW tracked file the fast-forward would bring down to B.
    std::fs::write(a_dir.join("new.op"), "from-remote").unwrap();
    a.stage_all().unwrap();
    a.commit("add new.op").unwrap();
    a.push().unwrap();

    // B has an UNTRACKED file at that same path. A forced fast-forward
    // would clobber it, so the pull must refuse — the untracked-overwrite
    // data-loss case (git's "untracked working tree files would be
    // overwritten by merge").
    std::fs::write(b_dir.join("new.op"), "local-untracked").unwrap();
    assert!(
        matches!(b.pull(), Err(GitError::WorkingTreeDirty)),
        "a fast-forward that would overwrite an untracked file must be refused"
    );
    assert_eq!(
        std::fs::read_to_string(b_dir.join("new.op")).unwrap(),
        "local-untracked",
        "the untracked file must survive the refused pull"
    );

    for dir in [remote, a_dir, b_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn pull_refuses_to_fast_forward_over_an_untracked_dir_replaced_by_a_file() {
    if !git_available() {
        return;
    }
    let remote = unique_temp_dir("ff-dir-remote");
    Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&remote)
        .output()
        .expect("init bare remote");

    let (a_dir, a) = clone_for_test(&remote, "ff-dir-a");
    std::fs::write(a_dir.join("a.op"), "1").unwrap();
    a.stage_all().unwrap();
    a.commit("init").unwrap();
    a.run(&["push", "-u", "origin", "main"]).unwrap();

    let (b_dir, b) = clone_for_test(&remote, "ff-dir-b");

    // A adds a tracked FILE named `data` the fast-forward would bring down.
    std::fs::write(a_dir.join("data"), "from-remote").unwrap();
    a.stage_all().unwrap();
    a.commit("add data file").unwrap();
    a.push().unwrap();

    // B has an untracked DIRECTORY at that path holding a local file. A
    // forced fast-forward would replace the directory with the incoming
    // file, destroying the local work — the file↔directory collision the
    // exact-path check missed; the pull must refuse.
    std::fs::create_dir(b_dir.join("data")).unwrap();
    std::fs::write(b_dir.join("data").join("local.op"), "local-work").unwrap();
    assert!(
        matches!(b.pull(), Err(GitError::WorkingTreeDirty)),
        "a fast-forward that replaces an untracked directory with a file must be refused"
    );
    assert_eq!(
        std::fs::read_to_string(b_dir.join("data").join("local.op")).unwrap(),
        "local-work",
        "the untracked directory's file must survive the refused pull"
    );

    for dir in [remote, a_dir, b_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn pull_classifies_a_divergent_merge() {
    if !git_available() {
        return;
    }
    let remote = unique_temp_dir("merge-remote");
    Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&remote)
        .output()
        .expect("init bare remote");

    // A seeds the remote with a base commit.
    let (a_dir, a) = clone_for_test(&remote, "merge-a");
    std::fs::write(a_dir.join("base.op"), "0").unwrap();
    a.stage_all().unwrap();
    a.commit("base").unwrap();
    a.run(&["push", "-u", "origin", "main"]).unwrap();

    // B clones at the base commit.
    let (b_dir, b) = clone_for_test(&remote, "merge-b");

    // A advances the remote with a change to one file.
    std::fs::write(a_dir.join("a-side.op"), "a").unwrap();
    a.stage_all().unwrap();
    a.commit("a side").unwrap();
    a.push().unwrap();

    // B commits a *different* file locally without pulling first —
    // now the local branch and the remote have diverged.
    std::fs::write(b_dir.join("b-side.op"), "b").unwrap();
    b.stage_all().unwrap();
    b.commit("b side").unwrap();

    // The pull cannot fast-forward; it must create a (clean,
    // non-conflicting) merge commit.
    assert_eq!(b.pull().expect("pull"), MergeOutcome::Merge);

    for dir in [remote, a_dir, b_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn pull_with_local_commits_and_unchanged_upstream_is_up_to_date() {
    if !git_available() {
        return;
    }
    let remote = unique_temp_dir("ahead-remote");
    Command::new("git")
        .args(["init", "--bare", "--initial-branch=main"])
        .arg(&remote)
        .output()
        .expect("init bare remote");

    let (dir, repo) = clone_for_test(&remote, "ahead-clone");
    std::fs::write(dir.join("base.op"), "0").unwrap();
    repo.stage_all().unwrap();
    repo.commit("base").unwrap();
    repo.run(&["push", "-u", "origin", "main"]).unwrap();

    // Commit locally WITHOUT pushing — the local branch is now ahead
    // of `origin/main`, which has not moved.
    std::fs::write(dir.join("local.op"), "1").unwrap();
    repo.stage_all().unwrap();
    repo.commit("local only").unwrap();

    // A pull has nothing to integrate (upstream is already an
    // ancestor of HEAD). It must report AlreadyUpToDate — NOT Merge.
    assert_eq!(repo.pull().expect("pull"), MergeOutcome::AlreadyUpToDate);

    for d in [remote, dir] {
        let _ = std::fs::remove_dir_all(d);
    }
}

#[test]
fn merge_fast_forwards_then_reports_up_to_date() {
    let Some(tr) = TempRepo::new("merge-ff") else {
        return;
    };
    tr.write("a.op", "1");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_and_switch_branch("feature").unwrap();
    tr.write("a.op", "2");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("feature work").unwrap();
    tr.repo.switch_branch("main").unwrap();

    // `main` has not moved since `base` → merging `feature` is a
    // fast-forward; merging it again is a no-op.
    assert_eq!(
        tr.repo.merge("feature").expect("merge"),
        MergeOutcome::FastForward
    );
    assert_eq!(
        tr.repo.merge("feature").expect("merge"),
        MergeOutcome::AlreadyUpToDate
    );
}

#[test]
fn merge_creates_a_commit_on_divergence() {
    let Some(tr) = TempRepo::new("merge-div") else {
        return;
    };
    tr.write("base.op", "0");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_and_switch_branch("feature").unwrap();
    tr.write("feature.op", "f");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("feature").unwrap();
    tr.repo.switch_branch("main").unwrap();
    tr.write("main.op", "m");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("main").unwrap();

    // Diverged, but the two branches touch different files → a
    // clean merge commit, no conflict, no merge left in progress.
    assert_eq!(
        tr.repo.merge("feature").expect("merge"),
        MergeOutcome::Merge
    );
    assert!(!tr.repo.is_merging());
}

#[test]
fn conflicting_merge_reports_conflict_then_aborts() {
    let Some(tr) = TempRepo::new("merge-conflict") else {
        return;
    };
    tr.write("doc.op", "base");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_and_switch_branch("feature").unwrap();
    tr.write("doc.op", "feature version");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("feature").unwrap();
    tr.repo.switch_branch("main").unwrap();
    tr.write("doc.op", "main version");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("main").unwrap();

    // Both branches changed `doc.op` → the merge conflicts.
    assert_eq!(
        tr.repo.merge("feature").expect("merge"),
        MergeOutcome::Conflict
    );
    assert!(tr.repo.is_merging());
    assert_eq!(
        tr.repo.conflicted_files().expect("conflicts"),
        vec!["doc.op".to_string()]
    );

    // A second merge — of *any* ref — must be refused while the
    // first merge is unresolved, rather than misreporting the
    // leftover conflict (or short-circuiting to a false success).
    let err = tr.repo.merge("feature").expect_err("must refuse");
    assert!(matches!(err, GitError::MergeInProgress));
    // `HEAD` itself would hit the up-to-date short-circuit — that
    // path must be refused too.
    let err_self = tr.repo.merge("HEAD").expect_err("must refuse");
    assert!(matches!(err_self, GitError::MergeInProgress));
    // `pull` must hit the same gate *before* any network fetch — in
    // this no-upstream repo, fetching first would instead surface a
    // confusing `@{u}` error, so `MergeInProgress` proves the gate
    // runs up front.
    let err_pull = tr.repo.pull().expect_err("pull must refuse");
    assert!(matches!(err_pull, GitError::MergeInProgress));

    // Aborting restores the clean pre-merge state.
    tr.repo.abort_merge().expect("abort");
    assert!(!tr.repo.is_merging());
    assert!(tr.repo.status().expect("status").is_clean());
}

/// Number of registered worktrees (the main tree counts as one).
fn worktree_count(repo: &GitRepo) -> usize {
    repo.run(&["worktree", "list"])
        .expect("worktree list")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count()
}

#[test]
fn worktree_merge_fast_forwards_into_a_clean_live_tree() {
    let Some(tr) = TempRepo::new("wt-merge-ff") else {
        return;
    };
    tr.write("base.op", "0");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_and_switch_branch("feature").unwrap();
    tr.write("feature.op", "f");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("feature").unwrap();
    tr.repo.switch_branch("main").unwrap();

    // `main` is a strict ancestor of `feature` → an exact ff.
    let report = tr
        .repo
        .merge_branch_isolated("feature", |_, _, _, _| None)
        .expect("merge");
    assert_eq!(report.outcome, MergeOutcome::FastForward);
    assert!(report.merged_commit.is_some());
    assert!(report.conflicts.is_empty());
    // The live tree advanced — `feature.op` is now present.
    assert!(tr.dir.join("feature.op").exists());
    assert!(!tr.repo.is_merging());
    // The throwaway worktree was cleaned up.
    assert_eq!(worktree_count(&tr.repo), 1);
}

#[test]
fn worktree_merge_creates_a_commit_on_divergence() {
    let Some(tr) = TempRepo::new("wt-merge-div") else {
        return;
    };
    tr.write("base.op", "0");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_and_switch_branch("feature").unwrap();
    tr.write("feature.op", "f");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("feature").unwrap();
    tr.repo.switch_branch("main").unwrap();
    tr.write("main.op", "m");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("main").unwrap();

    // Diverged but touching different files → a clean merge commit.
    let report = tr
        .repo
        .merge_branch_isolated("feature", |_, _, _, _| None)
        .expect("merge");
    assert_eq!(report.outcome, MergeOutcome::Merge);
    assert!(report.merged_commit.is_some());
    assert!(report.conflicts.is_empty());
    // Both files are present in the live tree and it is not merging.
    assert!(tr.dir.join("feature.op").exists());
    assert!(tr.dir.join("main.op").exists());
    assert!(!tr.repo.is_merging());
    assert_eq!(worktree_count(&tr.repo), 1);
}

#[test]
fn worktree_merge_quarantines_conflicts_and_keeps_the_live_tree_pristine() {
    let Some(tr) = TempRepo::new("wt-merge-conflict") else {
        return;
    };
    tr.write("doc.op", "base");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_and_switch_branch("feature").unwrap();
    tr.write("doc.op", "feature version");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("feature").unwrap();
    tr.repo.switch_branch("main").unwrap();
    tr.write("doc.op", "main version");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("main").unwrap();

    // Both branches changed `doc.op` → the merge conflicts, but the
    // conflict is quarantined to the throwaway worktree.
    let report = tr
        .repo
        .merge_branch_isolated("feature", |_, _, _, _| None)
        .expect("merge");
    assert_eq!(report.outcome, MergeOutcome::Conflict);
    assert!(report.merged_commit.is_none());
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(report.conflicts.files[0].path, "doc.op");
    assert_eq!(report.conflicts.files[0].kind, ConflictKind::BothModified);

    // The live tree is pristine — no `MERGE_HEAD`, no markers, and
    // `doc.op` still holds exactly the committed `main` content.
    assert!(!tr.repo.is_merging());
    assert!(tr.repo.status().expect("status").is_clean());
    assert_eq!(
        std::fs::read_to_string(tr.dir.join("doc.op")).unwrap(),
        "main version"
    );
    assert_eq!(worktree_count(&tr.repo), 1);
}

#[test]
fn worktree_merge_reports_up_to_date_for_an_ancestor() {
    let Some(tr) = TempRepo::new("wt-merge-utd") else {
        return;
    };
    tr.write("base.op", "0");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_branch("feature").unwrap();
    tr.write("main.op", "m");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("ahead").unwrap();

    // `feature` points at an ancestor of `main` → nothing to do, and
    // no worktree is created.
    let report = tr
        .repo
        .merge_branch_isolated("feature", |_, _, _, _| None)
        .expect("merge");
    assert_eq!(report.outcome, MergeOutcome::AlreadyUpToDate);
    assert!(report.merged_commit.is_none());
    assert_eq!(worktree_count(&tr.repo), 1);
}

#[test]
fn worktree_merge_refuses_a_dirty_live_tree() {
    let Some(tr) = TempRepo::new("wt-merge-dirty") else {
        return;
    };
    tr.write("base.op", "0");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_and_switch_branch("feature").unwrap();
    tr.write("feature.op", "f");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("feature").unwrap();
    tr.repo.switch_branch("main").unwrap();
    // An uncommitted change in the live tree.
    tr.write("scratch.op", "wip");

    let err = tr
        .repo
        .merge_branch_isolated("feature", |_, _, _, _| None)
        .expect_err("must refuse a dirty tree");
    assert!(matches!(err, GitError::WorkingTreeDirty));
    // Refused before any worktree was created.
    assert_eq!(worktree_count(&tr.repo), 1);
}

#[test]
fn worktree_merge_resolver_auto_completes_a_conflict() {
    let Some(tr) = TempRepo::new("wt-merge-resolve") else {
        return;
    };
    tr.write("doc.op", "base");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("base").unwrap();
    tr.repo.create_and_switch_branch("feature").unwrap();
    tr.write("doc.op", "feature version");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("feature").unwrap();
    tr.repo.switch_branch("main").unwrap();
    tr.write("doc.op", "main version");
    tr.repo.stage_all().unwrap();
    tr.repo.commit("main").unwrap();

    // The resolver supplies merged content for the conflicted `.op`
    // file — the otherwise-conflicting merge then completes cleanly.
    let report = tr
        .repo
        .merge_branch_isolated("feature", |path, _base, _ours, _theirs| {
            (path == "doc.op").then(|| "resolved".to_string())
        })
        .expect("merge");
    assert_eq!(report.outcome, MergeOutcome::Merge);
    assert!(report.conflicts.is_empty());
    assert!(!tr.repo.is_merging());
    assert_eq!(
        std::fs::read_to_string(tr.dir.join("doc.op")).unwrap(),
        "resolved",
        "the resolver's content landed in the live tree"
    );
    assert_eq!(worktree_count(&tr.repo), 1);
}
