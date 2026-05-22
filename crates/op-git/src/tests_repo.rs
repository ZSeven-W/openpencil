//! Local-repository tests: repo discovery, working-tree status,
//! commit + log, branch create / list / switch, file restore,
//! committer identity, and remote-URL configuration.

use crate::tests::{git_available, unique_temp_dir, TempRepo};
use crate::ChangeState;

#[test]
fn discover_finds_a_repo_and_rejects_a_bare_dir() {
    if !git_available() {
        return;
    }
    // A plain temp dir is not inside any repo.
    let plain = unique_temp_dir("discover-plain");
    std::fs::create_dir_all(&plain).unwrap();
    assert!(crate::GitRepo::discover(&plain).expect("discover ok").is_none());
    let _ = std::fs::remove_dir_all(&plain);

    // An initialized repo is discovered — including from a file path
    // nested inside it.
    let Some(tr) = TempRepo::new("discover-repo") else {
        return;
    };
    tr.write("design.op", "{}");
    let from_file = crate::GitRepo::discover(&tr.dir.join("design.op"))
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
