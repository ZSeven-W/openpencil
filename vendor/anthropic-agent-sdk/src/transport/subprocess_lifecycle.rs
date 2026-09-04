//! Claude CLI process-tree termination and direct-child reaping.

use std::io;
use std::process::ExitStatus;
use std::time::Duration;

use tokio::process::Child;

/// Reap a process after synchronous Drop has already signaled its tree. A
/// plain OS thread keeps this reliable even when Drop runs outside Tokio or
/// while the async runtime itself is shutting down.
pub(super) fn reap_cli_process_in_background(mut child: Child) {
    match child.try_wait() {
        Ok(Some(_)) | Err(_) => return,
        Ok(None) => {}
    }
    let _ = std::thread::Builder::new()
        .name("claude-cli-reaper".into())
        .spawn(move || {
            loop {
                match child.try_wait() {
                    Ok(Some(_)) | Err(_) => break,
                    Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                }
            }
        });
}

/// Gracefully terminate a dedicated Claude CLI process group, then force any
/// survivors and reap the direct child. The group signal is best-effort; the
/// direct-child kill remains the portable fallback.
pub(super) async fn terminate_cli_process(
    child: &mut Child,
    pid: Option<u32>,
    grace: Duration,
) -> io::Result<ExitStatus> {
    let _ = graceful_cli_tree(pid);
    match tokio::time::timeout(grace, child.wait()).await {
        Ok(status) => {
            let status = status?;
            // A Unix leader can exit while a tool process remains in its
            // dedicated group. Windows cannot safely address the old PID
            // after wait() because it may already have been reused; its tree
            // termination request must happen before the leader is reaped.
            #[cfg(unix)]
            let _ = force_cli_tree(pid);
            Ok(status)
        }
        Err(_) => {
            let tree_result = force_cli_tree(pid);
            let leader_result = child.start_kill();
            if tree_result.is_err() && leader_result.is_err() {
                return Err(io::Error::other(format!(
                    "failed to terminate Claude process tree ({}) and leader ({})",
                    tree_result.expect_err("checked error"),
                    leader_result.expect_err("checked error")
                )));
            }
            child.wait().await
        }
    }
}

/// On Windows, ask `taskkill /T` to snapshot and terminate descendants while
/// the leader PID is still live. `taskkill` cannot provide durable ownership
/// like a Job Object, so a leader that exits spontaneously before this call
/// remains a residual platform limitation. Unix keeps its existing short
/// natural-exit grace and returns `false` here.
#[cfg(windows)]
pub(super) fn request_tree_shutdown_before_wait(pid: Option<u32>) -> bool {
    pid.is_some() && graceful_cli_tree(pid).is_ok()
}

#[cfg(not(windows))]
pub(super) fn request_tree_shutdown_before_wait(_pid: Option<u32>) -> bool {
    false
}

#[cfg(unix)]
fn graceful_cli_tree(pid: Option<u32>) -> io::Result<()> {
    signal_unix_process_group(pid, libc::SIGTERM)
}

#[cfg(unix)]
pub(super) fn force_cli_tree(pid: Option<u32>) -> io::Result<()> {
    signal_unix_process_group(pid, libc::SIGKILL)
}

#[cfg(unix)]
fn signal_unix_process_group(pid: Option<u32>, signal: libc::c_int) -> io::Result<()> {
    let Some(pid) = pid else {
        return Ok(());
    };
    let pid = libc::pid_t::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Claude pid exceeds pid_t"))?;
    // SAFETY: build_command places the child in a new process group whose id
    // is its positive pid. We never derive this target from user input.
    let result = unsafe { libc::killpg(pid, signal) };
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(windows)]
fn graceful_cli_tree(pid: Option<u32>) -> io::Result<()> {
    taskkill_cli_tree(pid, false)
}

#[cfg(windows)]
pub(super) fn force_cli_tree(pid: Option<u32>) -> io::Result<()> {
    taskkill_cli_tree(pid, true)
}

#[cfg(windows)]
fn taskkill_cli_tree(pid: Option<u32>, force: bool) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    let Some(pid) = pid else {
        return Ok(());
    };
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = Command::new("taskkill");
    command
        .arg("/PID")
        .arg(pid.to_string())
        .arg("/T")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW);
    if force {
        command.arg("/F");
    }
    let mut taskkill = command.spawn()?;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let status = loop {
        if let Some(status) = taskkill.try_wait()? {
            break status;
        }
        if std::time::Instant::now() >= deadline {
            let _ = taskkill.kill();
            let _ = taskkill.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("taskkill timed out for Claude process tree {pid}"),
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "taskkill failed for Claude process tree {pid} with status {status}"
        )))
    }
}

#[cfg(not(any(unix, windows)))]
fn graceful_cli_tree(_pid: Option<u32>) -> io::Result<()> {
    Ok(())
}

#[cfg(not(any(unix, windows)))]
pub(super) fn force_cli_tree(_pid: Option<u32>) -> io::Result<()> {
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    use crate::Transport;
    use crate::transport::subprocess::{PromptInput, SubprocessTransport};
    use crate::types::ClaudeAgentOptions;

    fn process_exists(pid: u32) -> bool {
        let Ok(pid) = libc::pid_t::try_from(pid) else {
            return false;
        };
        // SAFETY: signal 0 only probes existence of the positive test pid.
        let result = unsafe { libc::kill(pid, 0) };
        result == 0 || io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }

    async fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            if predicate() {
                return true;
            }
            if tokio::time::Instant::now() >= deadline {
                return false;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn spawn_fake_cli_tree() -> (tempfile::TempDir, SubprocessTransport, u32, u32) {
        let temp = tempfile::tempdir().expect("temporary fake CLI directory");
        let script = temp.path().join("claude");
        let descendant_file = temp.path().join("descendant.pid");
        std::fs::write(
            &script,
            r#"#!/bin/sh
(
  trap '' TERM
  while :; do sleep 1; done
) &
printf '%s\n' "$!" > "$CLAUDE_TEST_DESCENDANT_FILE"
wait
"#,
        )
        .expect("write fake Claude CLI");
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).unwrap();

        let mut options = ClaudeAgentOptions::default();
        options.env.insert(
            "CLAUDE_TEST_DESCENDANT_FILE".into(),
            descendant_file.to_string_lossy().into_owned(),
        );
        let mut transport =
            SubprocessTransport::new(PromptInput::String("ignored".into()), options, Some(script))
                .expect("build fake transport");
        transport.connect().await.expect("spawn fake Claude CLI");
        let leader = transport.process_pid.expect("leader pid");
        let mut descendant = None;
        assert!(
            wait_until(Duration::from_secs(5), || {
                descendant = std::fs::read_to_string(&descendant_file)
                    .ok()
                    .and_then(|value| value.trim().parse::<u32>().ok());
                descendant.is_some()
            })
            .await,
            "fake CLI must report its descendant pid"
        );
        let descendant = descendant.expect("waited for a complete descendant pid");
        assert!(process_exists(leader));
        assert!(process_exists(descendant));

        (temp, transport, leader, descendant)
    }

    async fn assert_tree_gone(leader: u32, descendant: u32, context: &str) {
        assert!(
            wait_until(Duration::from_secs(2), || !process_exists(leader)).await,
            "{context} must reap the direct Claude child"
        );
        assert!(
            wait_until(Duration::from_secs(2), || !process_exists(descendant)).await,
            "{context} must terminate Claude descendants"
        );
    }

    #[tokio::test]
    async fn dropping_active_transport_kills_tree_and_reaps_leader() {
        let (_temp, mut transport, leader, descendant) = spawn_fake_cli_tree().await;
        let _messages = transport.read_messages();

        drop(transport);

        assert_tree_gone(leader, descendant, "transport Drop").await;
    }

    #[tokio::test]
    async fn canceling_close_still_kills_tree_and_reaps_leader() {
        let (_temp, mut transport, leader, descendant) = spawn_fake_cli_tree().await;
        let close_task = tokio::spawn(async move { transport.close().await });
        // `close` is now inside its graceful child wait. Canceling that await
        // used to drop a local Child after it had been taken out of the
        // transport, losing the process-group id and orphaning descendants.
        tokio::time::sleep(Duration::from_millis(50)).await;
        close_task.abort();
        let join_error = close_task.await.expect_err("close task should be canceled");
        assert!(join_error.is_cancelled());

        assert_tree_gone(leader, descendant, "canceled close").await;
    }
}
