#![cfg(unix)]

use std::io::{BufRead, BufReader, Read};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::Duration;

use op_process_io::{
    spawn_null, terminate_process_tree, terminate_tokio_process_tree, wait_for_child_or,
    LineStreamChild, ProcessTree, WaitOutcome,
};
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::process::Command as TokioCommand;

#[test]
fn wait_for_child_or_reports_ready_before_process_exit() {
    let mut command = Command::new("sh");
    command.args(["-c", "sleep 5"]);
    let mut child = spawn_null(&mut command).expect("spawn child");

    let outcome = wait_for_child_or(&mut child, 3, Duration::from_millis(10), || Some("ready"))
        .expect("wait for readiness");

    assert_eq!(outcome, WaitOutcome::Ready("ready"));
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn wait_for_child_or_reports_early_exit_status() {
    let mut command = Command::new("sh");
    command.args(["-c", "exit 7"]);
    let mut child = spawn_null(&mut command).expect("spawn child");

    let outcome = wait_for_child_or::<()>(&mut child, 10, Duration::from_millis(10), || None)
        .expect("wait for child exit");

    let WaitOutcome::Exited(status) = outcome else {
        panic!("expected exited outcome");
    };
    assert_eq!(status.code(), Some(7));
}

#[tokio::test]
async fn line_stream_child_reads_stdout_lines() {
    let mut child = LineStreamChild::spawn(
        "sh",
        ["-c", "printf 'one\\ntwo\\n'"],
        std::iter::empty::<(&str, &str)>(),
    )
    .expect("spawn child");

    assert_eq!(
        child.next_line().await.expect("read line"),
        Some("one".into())
    );
    assert_eq!(
        child.next_line().await.expect("read line"),
        Some("two".into())
    );
    assert!(child.wait().await.expect("wait").success());
}

#[tokio::test]
async fn line_stream_child_feeds_stdin_and_closes_it() {
    let mut child = LineStreamChild::spawn(
        "sh",
        ["-c", "IFS= read -r line; printf '%s\\n' \"$line\""],
        std::iter::empty::<(&str, &str)>(),
    )
    .expect("spawn child");

    child.feed("hello\n").await.expect("feed stdin");
    child.close_stdin().await.expect("close stdin");

    assert_eq!(
        child.next_line().await.expect("read echoed line"),
        Some("hello".into())
    );
    assert!(child.wait().await.expect("wait").success());
}

#[tokio::test]
async fn kill_graceful_closes_stdin_before_forcing_exit() {
    let mut child = LineStreamChild::spawn(
        "sh",
        ["-c", "cat >/dev/null"],
        std::iter::empty::<(&str, &str)>(),
    )
    .expect("spawn child");

    let status = child
        .kill_graceful(Duration::from_secs(1))
        .await
        .expect("graceful kill");
    assert!(status.success());
}

#[tokio::test]
async fn kill_graceful_forces_exit_after_timeout() {
    let mut command = TokioCommand::new("sh");
    command
        .args(["-c", "trap '' TERM; sleep 30 & printf 'ready\\n'; wait"])
        .process_group(0);
    let mut child = LineStreamChild::spawn_command(command).expect("spawn child tree");

    assert_eq!(
        child.next_line().await.expect("read readiness line"),
        Some("ready".into())
    );

    let status = child
        .kill_graceful(Duration::from_millis(20))
        .await
        .expect("forced kill");
    assert!(!status.success());
    let eof = tokio::time::timeout(Duration::from_secs(1), child.next_line())
        .await
        .expect("descendant did not retain stdout after tree kill")
        .expect("read stdout after tree kill");
    assert_eq!(eof, None);
}

#[tokio::test]
async fn start_kill_terminates_the_verified_process_group() {
    let mut command = TokioCommand::new("sh");
    command
        .args(["-c", "sleep 30 & printf 'ready\\n'; wait"])
        .process_group(0);
    let mut child = LineStreamChild::spawn_command(command).expect("spawn child tree");

    assert_eq!(
        child.next_line().await.expect("read readiness line"),
        Some("ready".into())
    );
    child.start_kill().expect("kill child tree");

    let eof = tokio::time::timeout(Duration::from_secs(1), child.next_line())
        .await
        .expect("descendant did not release stdout after tree kill")
        .expect("read stdout after tree kill");
    assert_eq!(eof, None);
    assert!(!child.wait().await.expect("reap tree leader").success());
}

#[tokio::test]
async fn dropping_line_stream_child_terminates_descendants() {
    let mut command = TokioCommand::new("sh");
    command
        .args(["-c", "sleep 30 & printf 'ready\\n'; wait"])
        .process_group(0);
    let mut child = LineStreamChild::spawn_command(command).expect("spawn child tree");
    let mut lines = child.take_lines().expect("take stdout lines");

    assert_eq!(
        lines.next_line().await.expect("read readiness line"),
        Some("ready".into())
    );
    drop(child);

    let eof = tokio::time::timeout(Duration::from_secs(1), lines.next_line())
        .await
        .expect("descendant retained stdout after LineStreamChild drop")
        .expect("read stdout after LineStreamChild drop");
    assert_eq!(eof, None);
}

#[test]
fn terminate_process_tree_forces_and_reaps_a_stubborn_group() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "trap '' TERM; sleep 30 & printf 'ready\\n'; wait"])
        .process_group(0)
        .stdout(Stdio::piped());
    let mut child = command.spawn().expect("spawn blocking child tree");
    let stdout = child.stdout.take().expect("piped child stdout");
    let mut stdout = BufReader::new(stdout);
    let mut readiness = String::new();
    stdout
        .read_line(&mut readiness)
        .expect("read readiness line");
    assert_eq!(readiness, "ready\n");

    let status = terminate_process_tree(&mut child, Duration::from_millis(20))
        .expect("terminate and reap child tree");
    assert!(!status.success());

    let mut remainder = String::new();
    stdout
        .read_to_string(&mut remainder)
        .expect("read stdout EOF after tree kill");
    assert!(remainder.is_empty());
    assert!(child.try_wait().expect("poll reaped child").is_some());
}

#[tokio::test]
async fn terminate_tokio_process_tree_forces_and_reaps_a_stubborn_group() {
    let mut command = TokioCommand::new("sh");
    command
        .args(["-c", "trap '' TERM; sleep 30 & printf 'ready\\n'; wait"])
        .process_group(0)
        .stdout(Stdio::piped());
    let mut child = command.spawn().expect("spawn async child tree");
    let stdout = child.stdout.take().expect("piped child stdout");
    let mut stdout = TokioBufReader::new(stdout);
    let mut readiness = String::new();
    stdout
        .read_line(&mut readiness)
        .await
        .expect("read readiness line");
    assert_eq!(readiness, "ready\n");

    let status = terminate_tokio_process_tree(&mut child, Duration::from_millis(20))
        .await
        .expect("terminate and reap async child tree");

    assert!(!status.success());
    assert!(
        child.id().is_none(),
        "successful wait must clear the child pid"
    );
}

#[test]
fn process_tree_rejects_zero_pid() {
    let error = ProcessTree::from_pid(0).expect_err("zero pid must be rejected");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn post_reap_cleanup_is_a_noop_for_an_unverified_process_target() {
    let mut child = Command::new("sh")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("spawn child");
    let tree = ProcessTree::from_child(&child).expect("capture direct process target");
    assert!(child.wait().expect("reap child").success());

    tree.kill_after_leader_exit()
        .expect("post-reap cleanup must not signal an unverified numeric pid");
}

#[tokio::test]
async fn post_reap_cleanup_can_kill_a_verified_unix_process_group() {
    let mut command = TokioCommand::new("sh");
    command
        .args(["-c", "trap '' HUP; sleep 30 & printf 'ready\\n'; exit 0"])
        .process_group(0)
        .stdout(Stdio::piped());
    let mut child = command.spawn().expect("spawn wrapper and helper");
    let tree = ProcessTree::from_tokio_child(&child).expect("capture verified process group");
    let stdout = child.stdout.take().expect("piped child stdout");
    let mut stdout = TokioBufReader::new(stdout);
    let mut readiness = String::new();
    stdout
        .read_line(&mut readiness)
        .await
        .expect("read readiness line");
    assert_eq!(readiness, "ready\n");
    assert!(child.wait().await.expect("reap wrapper").success());

    tree.kill_after_leader_exit()
        .expect("verified Unix process group remains a safe cleanup target");
    let mut remainder = String::new();
    tokio::time::timeout(Duration::from_secs(1), stdout.read_line(&mut remainder))
        .await
        .expect("descendant retained stdout after process-group cleanup")
        .expect("read stdout EOF");
    assert!(remainder.is_empty());
}
