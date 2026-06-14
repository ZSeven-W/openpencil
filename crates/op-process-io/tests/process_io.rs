#![cfg(unix)]

use std::process::Command;
use std::time::Duration;

use op_process_io::{spawn_null, wait_for_child_or, LineStreamChild, WaitOutcome};

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
    let mut child = LineStreamChild::spawn(
        "sh",
        ["-c", "while true; do sleep 1; done"],
        std::iter::empty::<(&str, &str)>(),
    )
    .expect("spawn child");

    let status = child
        .kill_graceful(Duration::from_millis(20))
        .await
        .expect("forced kill");
    assert!(!status.success());
}
