//! Integration tests — each runs against a throwaway repository
//! created with the real system `git`.
//!
//! `git` is a hard requirement of this crate, but the test harness
//! still guards on its presence so the suite is skipped (not failed)
//! on a machine without `git` installed.

use std::path::PathBuf;
use std::process::Command;

use crate::{
    AuthStore, ChangeState, ConflictKind, Credential, GitError, GitRepo, MergeOutcome, SshKeyStore,
};

/// Whether the system `git` executable is usable.
fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Whether `ssh-keygen` is usable. `-?` is an unknown flag — it
/// prints usage and exits non-zero *without* reading stdin, so the
/// process simply running (an `Ok` output) proves availability.
fn ssh_keygen_available() -> bool {
    Command::new("ssh-keygen").arg("-?").output().is_ok()
}

/// A unique temp directory path for one test.
fn unique_temp_dir(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("op-git-test-{tag}-{}-{nanos}", std::process::id()))
}

/// A throwaway repository with a configured commit identity, removed
/// from disk when dropped.
struct TempRepo {
    dir: PathBuf,
    repo: GitRepo,
}

impl TempRepo {
    /// Create a fresh initialized repo, or `None` when `git` is absent.
    fn new(tag: &str) -> Option<Self> {
        if !git_available() {
            return None;
        }
        let dir = unique_temp_dir(tag);
        let repo = GitRepo::init(&dir).expect("git init");
        // A hermetic identity so `git commit` never depends on the
        // host's global git config.
        repo.run(&["config", "user.email", "test@openpencil.dev"])
            .expect("set user.email");
        repo.run(&["config", "user.name", "OP Test"])
            .expect("set user.name");
        Some(Self { dir, repo })
    }

    /// Write `content` to `name` in the working tree.
    fn write(&self, name: &str, content: &str) {
        std::fs::write(self.dir.join(name), content).expect("write file");
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Clone the bare repo at `remote` into a fresh temp dir and give it
/// a hermetic commit identity. Returns the clone's dir + handle.
fn clone_for_test(remote: &std::path::Path, tag: &str) -> (PathBuf, GitRepo) {
    let dir = unique_temp_dir(tag);
    let repo = GitRepo::clone(remote.to_str().unwrap(), &dir).expect("clone");
    repo.run(&["config", "user.email", "t@openpencil.dev"]).unwrap();
    repo.run(&["config", "user.name", "OP Test"]).unwrap();
    (dir, repo)
}

#[test]
fn discover_finds_a_repo_and_rejects_a_bare_dir() {
    if !git_available() {
        return;
    }
    // A plain temp dir is not inside any repo.
    let plain = unique_temp_dir("discover-plain");
    std::fs::create_dir_all(&plain).unwrap();
    assert!(GitRepo::discover(&plain).expect("discover ok").is_none());
    let _ = std::fs::remove_dir_all(&plain);

    // An initialized repo is discovered — including from a file path
    // nested inside it.
    let Some(tr) = TempRepo::new("discover-repo") else {
        return;
    };
    tr.write("design.op", "{}");
    let from_file = GitRepo::discover(&tr.dir.join("design.op"))
        .expect("discover ok")
        .expect("repo found from a file path");
    assert!(from_file.workdir().exists());
}

#[test]
fn status_tracks_untracked_then_clean_after_commit() {
    let Some(tr) = TempRepo::new("status") else {
        return;
    };
    // Fresh repo, no files → clean.
    assert!(tr.repo.status().expect("status").is_clean());

    // A new file shows as untracked.
    tr.write("design.op", "{\"v\":1}");
    let st = tr.repo.status().expect("status");
    assert_eq!(st.files.len(), 1);
    assert_eq!(st.files[0].state, ChangeState::Untracked);

    // Stage + commit → working tree clean again.
    tr.repo.stage_all().expect("stage");
    let hash = tr.repo.commit("add design").expect("commit");
    assert_eq!(hash.len(), 40, "commit returns a full hash");
    assert!(tr.repo.status().expect("status").is_clean());
}

#[test]
fn commit_shows_up_in_the_log() {
    let Some(tr) = TempRepo::new("log") else {
        return;
    };
    // No commits yet → empty log, not an error.
    assert!(tr.repo.log(10).expect("log").is_empty());

    tr.write("a.op", "1");
    tr.repo.stage_all().expect("stage");
    tr.repo.commit("first design").expect("commit");
    tr.write("a.op", "2");
    tr.repo.stage_all().expect("stage");
    tr.repo.commit("tweak design").expect("commit");

    let log = tr.repo.log(10).expect("log");
    assert_eq!(log.len(), 2);
    // Newest first.
    assert_eq!(log[0].summary, "tweak design");
    assert_eq!(log[1].summary, "first design");
    assert_eq!(log[0].author, "OP Test");
    assert!(log[0].timestamp > 0);
}

#[test]
fn branch_create_list_and_switch() {
    let Some(tr) = TempRepo::new("branch") else {
        return;
    };
    tr.write("a.op", "1");
    tr.repo.stage_all().expect("stage");
    tr.repo.commit("init").expect("commit");

    assert_eq!(tr.repo.current_branch().expect("current"), Some("main".into()));

    tr.repo.create_branch("feature").expect("create branch");
    let branches = tr.repo.branches().expect("branches");
    assert_eq!(branches.len(), 2);
    assert!(branches.iter().any(|b| b.name == "feature" && !b.is_current));
    assert!(branches.iter().any(|b| b.name == "main" && b.is_current));

    tr.repo.switch_branch("feature").expect("switch");
    assert_eq!(
        tr.repo.current_branch().expect("current"),
        Some("feature".into())
    );
}

#[test]
fn restore_reverts_a_modified_file() {
    let Some(tr) = TempRepo::new("restore") else {
        return;
    };
    tr.write("a.op", "committed");
    tr.repo.stage_all().expect("stage");
    tr.repo.commit("init").expect("commit");

    // Modify, then restore back to the committed (HEAD) content.
    tr.write("a.op", "scratch edit");
    assert!(!tr.repo.status().expect("status").is_clean());
    tr.repo
        .restore(std::path::Path::new("a.op"), "HEAD")
        .expect("restore");
    assert!(tr.repo.status().expect("status").is_clean());
    let content = std::fs::read_to_string(tr.dir.join("a.op")).unwrap();
    assert_eq!(content, "committed");
}

#[test]
fn restore_rolls_a_file_back_to_an_older_commit() {
    let Some(tr) = TempRepo::new("restore-commit") else {
        return;
    };
    tr.write("a.op", "version one");
    tr.repo.stage_all().expect("stage");
    let v1 = tr.repo.commit("v1").expect("commit");
    tr.write("a.op", "version two");
    tr.repo.stage_all().expect("stage");
    tr.repo.commit("v2").expect("commit");

    // Restore the working file to its content at the v1 commit —
    // matches the TS engine's `restoreFileFromCommit`.
    tr.repo
        .restore(std::path::Path::new("a.op"), &v1)
        .expect("restore from commit");
    assert_eq!(
        std::fs::read_to_string(tr.dir.join("a.op")).unwrap(),
        "version one"
    );
}

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
    assert_eq!(tr.repo.merge("feature").expect("merge"), MergeOutcome::FastForward);
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
    assert_eq!(tr.repo.merge("feature").expect("merge"), MergeOutcome::Merge);
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
    assert_eq!(tr.repo.merge("feature").expect("merge"), MergeOutcome::Conflict);
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

#[test]
fn ssh_key_generate_list_load_and_delete() {
    if !ssh_keygen_available() {
        return;
    }
    let dir = unique_temp_dir("ssh");
    let store = SshKeyStore::at(&dir);
    assert!(store.list().expect("list empty").is_empty());

    let key = store.generate("id_test", "op@test").expect("generate");
    assert_eq!(key.name, "id_test");
    assert!(key.public_key.starts_with("ssh-ed25519 "));
    assert!(key.public_key.contains("op@test"));
    assert!(key.private_path.is_file());

    let listed = store.list().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0], key);

    // Re-generating an existing key name is refused (no overwrite).
    assert!(store.generate("id_test", "x").is_err());

    store.delete("id_test").expect("delete");
    assert!(store.list().expect("list after delete").is_empty());
    // Deleting a missing key is a tolerated no-op.
    store.delete("id_test").expect("delete is idempotent");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn auth_store_set_get_replace_remove_and_persist() {
    let dir = unique_temp_dir("auth");
    let path = dir.join("git-auth.json");
    let store = AuthStore::at(&path);

    // Empty store.
    assert!(store.hosts().expect("hosts").is_empty());
    assert!(store.get("github.com").expect("get").is_none());

    // Store an HTTPS credential.
    let https = Credential::Https {
        username: "octocat".to_string(),
        token: "ghp_secret".to_string(),
    };
    store.set("github.com", https.clone()).expect("set https");
    assert_eq!(store.get("github.com").expect("get"), Some(https));

    // A second host with an SSH credential; both must survive a
    // reload from a fresh handle on the same file.
    store
        .set(
            "git.example.com",
            Credential::Ssh {
                key_name: "id_work".to_string(),
            },
        )
        .expect("set ssh");
    let reloaded = AuthStore::at(&path);
    assert_eq!(
        reloaded.hosts().expect("hosts"),
        vec!["git.example.com".to_string(), "github.com".to_string()]
    );

    // Re-setting a host replaces its credential.
    store
        .set(
            "github.com",
            Credential::Ssh {
                key_name: "id_personal".to_string(),
            },
        )
        .expect("replace");
    assert_eq!(
        store.get("github.com").expect("get"),
        Some(Credential::Ssh {
            key_name: "id_personal".to_string()
        })
    );

    // Remove — and removing an absent host is a tolerated no-op.
    store.remove("github.com").expect("remove");
    assert!(store.get("github.com").expect("get").is_none());
    store.remove("github.com").expect("remove is idempotent");
    assert_eq!(
        store.hosts().expect("hosts"),
        vec!["git.example.com".to_string()]
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn auth_store_file_is_created_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let dir = unique_temp_dir("auth-perms");
    let path = dir.join("git-auth.json");
    let store = AuthStore::at(&path);
    store
        .set(
            "host",
            Credential::Ssh {
                key_name: "k".to_string(),
            },
        )
        .expect("set");
    // The store holds secrets — it must never be world-readable,
    // not even momentarily during the write.
    let mode = std::fs::metadata(&path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "credential store must be owner-only (0600)");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn ssh_key_store_rejects_directory_escaping_names() {
    // No `ssh-keygen` needed: name validation runs first, before any
    // filesystem work or subprocess.
    let store = SshKeyStore::at(unique_temp_dir("ssh-reject"));
    for bad in ["..", ".", "", "../escape", "sub/key", "/abs/key", "a\\b"] {
        assert!(
            store.generate(bad, "c").is_err(),
            "generate must reject `{bad}`"
        );
        assert!(store.load(bad).is_err(), "load must reject `{bad}`");
        assert!(store.delete(bad).is_err(), "delete must reject `{bad}`");
        assert!(
            store
                .import(std::path::Path::new("/nonexistent"), bad)
                .is_err(),
            "import must reject `{bad}`"
        );
    }
}

#[test]
fn ssh_key_import_copies_the_pair() {
    if !ssh_keygen_available() {
        return;
    }
    // Generate a key in a "source" store, then import it into a
    // separate store under a new name.
    let src_dir = unique_temp_dir("ssh-src");
    let src = SshKeyStore::at(&src_dir);
    let original = src.generate("orig", "src@test").expect("generate");

    let dst_dir = unique_temp_dir("ssh-dst");
    let dst = SshKeyStore::at(&dst_dir);
    let imported = dst
        .import(&original.private_path, "imported")
        .expect("import");

    assert_eq!(imported.name, "imported");
    // The public key is carried over verbatim.
    assert_eq!(imported.public_key, original.public_key);
    assert!(dst_dir.join("imported").is_file());
    assert!(dst_dir.join("imported.pub").is_file());

    for dir in [src_dir, dst_dir] {
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[test]
fn author_reads_the_configured_identity() {
    let Some(tr) = TempRepo::new("author") else {
        return;
    };
    let author = tr.repo.author();
    assert_eq!(author.name.as_deref(), Some("OP Test"));
    assert_eq!(author.email.as_deref(), Some("test@openpencil.dev"));
}

#[test]
fn set_remote_adds_then_updates() {
    let Some(tr) = TempRepo::new("remote") else {
        return;
    };
    assert!(tr.repo.remotes().expect("remotes").is_empty());

    tr.repo
        .set_remote("origin", "https://example.com/a.git")
        .expect("add remote");
    assert_eq!(
        tr.repo.remote_url("origin").expect("url").as_deref(),
        Some("https://example.com/a.git")
    );

    // A second call updates the existing remote rather than failing.
    tr.repo
        .set_remote("origin", "https://example.com/b.git")
        .expect("update remote");
    let remotes = tr.repo.remotes().expect("remotes");
    assert_eq!(remotes.len(), 1);
    assert_eq!(remotes[0].url, "https://example.com/b.git");

    // An unknown remote resolves to `None`, not an error.
    assert!(tr.repo.remote_url("nope").expect("url").is_none());
}

#[test]
fn create_and_switch_branch_in_one_step() {
    let Some(tr) = TempRepo::new("create-switch") else {
        return;
    };
    tr.write("a.op", "1");
    tr.repo.stage_all().expect("stage");
    tr.repo.commit("init").expect("commit");

    tr.repo
        .create_and_switch_branch("wip")
        .expect("create + switch");
    assert_eq!(tr.repo.current_branch().expect("current"), Some("wip".into()));
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
    let report = tr.repo.merge_branch_isolated("feature").expect("merge");
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
    let report = tr.repo.merge_branch_isolated("feature").expect("merge");
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
    let report = tr.repo.merge_branch_isolated("feature").expect("merge");
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
    let report = tr.repo.merge_branch_isolated("feature").expect("merge");
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
        .merge_branch_isolated("feature")
        .expect_err("must refuse a dirty tree");
    assert!(matches!(err, GitError::WorkingTreeDirty));
    // Refused before any worktree was created.
    assert_eq!(worktree_count(&tr.repo), 1);
}
