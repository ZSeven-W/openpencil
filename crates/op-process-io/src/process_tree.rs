use std::io;
use std::process::{Child, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

use tokio::process::Child as TokioChild;

const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(10);
#[cfg(windows)]
const TASKKILL_TIMEOUT: Duration = Duration::from_secs(2);

/// A process-tree target captured while its leader is still alive.
///
/// On Unix, OpenPencil's CLI launcher places every child in a process group
/// whose id is the child pid. We verify that relationship before retaining a
/// group target; otherwise signals are restricted to the exact child pid so a
/// caller can never accidentally signal OpenPencil's own process group.
#[derive(Clone, Copy, Debug)]
pub struct ProcessTree {
    #[cfg(unix)]
    target: UnixTarget,
    #[cfg(windows)]
    pid: u32,
    #[cfg(not(any(unix, windows)))]
    pid: u32,
}

#[cfg(unix)]
#[derive(Clone, Copy, Debug)]
enum UnixTarget {
    Group(libc::pid_t),
    Process(libc::pid_t),
}

impl ProcessTree {
    /// Capture the safest available tree target for `pid`.
    pub fn from_pid(pid: u32) -> io::Result<Self> {
        if pid == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "process-tree leader pid must be non-zero",
            ));
        }

        #[cfg(unix)]
        {
            let pid = libc::pid_t::try_from(pid).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "process-tree leader pid exceeds pid_t",
                )
            })?;
            // SAFETY: getpgid/getpgrp are read-only libc queries. A failed
            // getpgid (for example, an already-exited child) deliberately
            // falls back to the exact pid rather than guessing a group id.
            let child_group = unsafe { libc::getpgid(pid) };
            let our_group = unsafe { libc::getpgrp() };
            let target = if child_group == pid && child_group != our_group {
                UnixTarget::Group(child_group)
            } else {
                UnixTarget::Process(pid)
            };
            Ok(Self { target })
        }

        #[cfg(windows)]
        {
            Ok(Self { pid })
        }

        #[cfg(not(any(unix, windows)))]
        {
            Ok(Self { pid })
        }
    }

    /// Capture a tree target from a blocking child handle.
    pub fn from_child(child: &Child) -> io::Result<Self> {
        Self::from_pid(child.id())
    }

    /// Capture a tree target from an async child handle.
    pub fn from_tokio_child(child: &TokioChild) -> io::Result<Self> {
        let pid = child.id().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "child has already exited and no longer has a pid",
            )
        })?;
        Self::from_pid(pid)
    }

    /// Ask the tree to terminate, allowing normal signal cleanup.
    pub fn terminate(self) -> io::Result<()> {
        self.signal(false)
    }

    /// Force the tree to exit.
    pub fn kill(self) -> io::Result<()> {
        self.signal(true)
    }

    /// Force any safely addressable descendants after the leader has exited.
    ///
    /// Only a verified Unix process group remains a stable target after its
    /// leader is reaped. Windows tree discovery requires the leader to still
    /// be alive, so this deliberately becomes a no-op there instead of
    /// risking `taskkill` against a reused numeric PID.
    pub fn kill_after_leader_exit(self) -> io::Result<()> {
        if self.can_signal_after_leader_exit() {
            self.kill()
        } else {
            Ok(())
        }
    }

    #[cfg(unix)]
    fn signal(self, force: bool) -> io::Result<()> {
        let signal = if force { libc::SIGKILL } else { libc::SIGTERM };
        // SAFETY: the target was captured from a positive child pid. Group
        // signaling is used only after verifying pgid == child pid and pgid !=
        // OpenPencil's own group; otherwise we signal the exact child pid.
        let result = unsafe {
            match self.target {
                UnixTarget::Group(group) => libc::killpg(group, signal),
                UnixTarget::Process(pid) => libc::kill(pid, signal),
            }
        };
        signal_result(result)
    }

    #[cfg(windows)]
    fn signal(self, force: bool) -> io::Result<()> {
        taskkill(self.pid, force)
    }

    #[cfg(not(any(unix, windows)))]
    fn signal(self, _force: bool) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("process-tree signaling is unsupported for pid {}", self.pid),
        ))
    }

    #[cfg(unix)]
    pub(crate) fn is_alive(self) -> io::Result<bool> {
        // Signal 0 checks whether the captured target still exists without
        // changing it. EPERM also means it exists but is not signalable.
        let result = unsafe {
            match self.target {
                UnixTarget::Group(group) => libc::killpg(group, 0),
                UnixTarget::Process(pid) => libc::kill(pid, 0),
            }
        };
        if result == 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::ESRCH) => Ok(false),
            Some(libc::EPERM) => Ok(true),
            _ => Err(error),
        }
    }

    #[cfg(not(unix))]
    pub(crate) fn is_alive(self) -> io::Result<bool> {
        // The std/tokio Child handle remains the source of truth off Unix.
        // Windows tree traversal is performed by taskkill when signaling.
        Ok(true)
    }

    #[cfg(unix)]
    pub(crate) fn covers_descendants(self) -> bool {
        matches!(self.target, UnixTarget::Group(_))
    }

    #[cfg(windows)]
    pub(crate) fn covers_descendants(self) -> bool {
        true
    }

    #[cfg(not(any(unix, windows)))]
    pub(crate) fn covers_descendants(self) -> bool {
        false
    }

    /// Whether the captured target remains safe to signal after the leader has
    /// exited and its child handle has observed that exit.
    ///
    /// A verified Unix process group can outlive its leader and still names
    /// exactly that group. Windows `taskkill /PID ... /T`, however, needs the
    /// live leader to discover its descendants. Retrying the numeric pid after
    /// the leader has been reaped can target an unrelated process after pid
    /// reuse.
    #[cfg(unix)]
    pub(crate) fn can_signal_after_leader_exit(self) -> bool {
        matches!(self.target, UnixTarget::Group(_))
    }

    #[cfg(not(unix))]
    pub(crate) fn can_signal_after_leader_exit(self) -> bool {
        false
    }

    /// Whether tree shutdown must be requested before any wait can reap the
    /// leader and make descendant discovery unsafe.
    #[cfg(windows)]
    pub(crate) fn requires_signal_before_wait(self) -> bool {
        self.covers_descendants()
    }

    #[cfg(not(windows))]
    pub(crate) fn requires_signal_before_wait(self) -> bool {
        false
    }
}

#[cfg(unix)]
fn signal_result(result: libc::c_int) -> io::Result<()> {
    if result == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ESRCH) {
        // Exiting between target capture and signal delivery is success for a
        // termination operation.
        Ok(())
    } else {
        Err(error)
    }
}

#[cfg(windows)]
fn taskkill(pid: u32, force: bool) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

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
    let deadline = Instant::now() + TASKKILL_TIMEOUT;
    let status = loop {
        if let Some(status) = taskkill.try_wait()? {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = taskkill.kill();
            let _ = taskkill.wait();
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("taskkill timed out for process tree {pid}"),
            ));
        }
        thread::sleep(WAIT_POLL_INTERVAL);
    };
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "taskkill failed for process tree {pid} with status {status}"
        )))
    }
}

pub(crate) fn combine_signal_results(
    tree: io::Result<()>,
    leader: io::Result<()>,
    tree_covers_descendants: bool,
) -> io::Result<()> {
    match (tree, leader) {
        (Ok(()), _) => Ok(()),
        (Err(_), Ok(())) if !tree_covers_descendants => Ok(()),
        (Err(tree_error), Ok(())) => Err(io::Error::other(format!(
            "failed to terminate process tree ({tree_error}); the leader exited but descendant cleanup is unverified"
        ))),
        (Err(tree_error), Err(leader_error)) => Err(io::Error::other(format!(
            "failed to terminate process tree ({tree_error}) and leader ({leader_error})"
        ))),
    }
}

/// Preserve a shutdown error, but only after the leader's wait has completed.
///
/// Tree and leader signaling are deliberately combined before this helper is
/// called. A tree signal can fail even though the exact leader accepted its
/// kill; returning that error before `wait` would leave a zombie on Unix and
/// an unreleased process handle on Windows.
pub(crate) fn combine_shutdown_and_reap(
    shutdown: io::Result<()>,
    reap: io::Result<ExitStatus>,
) -> io::Result<ExitStatus> {
    match (shutdown, reap) {
        (Ok(()), status) => status,
        (Err(shutdown_error), Ok(_)) => Err(shutdown_error),
        (Err(shutdown_error), Err(reap_error)) => Err(io::Error::other(format!(
            "process shutdown failed ({shutdown_error}) and leader reaping failed ({reap_error})"
        ))),
    }
}

pub(crate) fn shutdown_without_reap(shutdown: io::Result<()>) -> io::Result<ExitStatus> {
    match shutdown {
        Err(error) => Err(error),
        Ok(()) => Err(io::Error::other(
            "process shutdown did not accept a signal, so the live leader cannot be reaped",
        )),
    }
}

fn reap_blocking_after_shutdown(
    child: &mut Child,
    observed_status: Option<ExitStatus>,
    shutdown: io::Result<()>,
    signal_accepted: bool,
) -> io::Result<ExitStatus> {
    if observed_status.is_none() && !signal_accepted {
        return match child.try_wait() {
            Ok(Some(status)) => combine_shutdown_and_reap(shutdown, Ok(status)),
            Ok(None) => shutdown_without_reap(shutdown),
            Err(reap_error) => combine_shutdown_and_reap(shutdown, Err(reap_error)),
        };
    }
    let reap = match observed_status {
        Some(status) => Ok(status),
        None => child.wait(),
    };
    combine_shutdown_and_reap(shutdown, reap)
}

/// Force a blocking child and its descendants to exit without waiting.
pub fn kill_process_tree(child: &mut Child) -> io::Result<()> {
    let tree = ProcessTree::from_child(child);
    let tree_covers_descendants = tree.as_ref().is_ok_and(|tree| tree.covers_descendants());
    let tree = tree.and_then(ProcessTree::kill);
    let leader = child.kill();
    combine_signal_results(tree, leader, tree_covers_descendants)
}

/// Gracefully terminate a blocking child tree, then force and reap it when the
/// grace period expires.
pub fn terminate_process_tree(child: &mut Child, grace: Duration) -> io::Result<ExitStatus> {
    let tree = ProcessTree::from_child(child)?;
    let graceful_tree_result = tree.terminate();
    let deadline = Instant::now() + grace;
    let mut leader_status = None;

    loop {
        if leader_status.is_none() {
            leader_status = child.try_wait()?;
        }
        if let Some(status) = leader_status {
            if !tree.covers_descendants() {
                return Ok(status);
            }
            if !tree.can_signal_after_leader_exit() {
                graceful_tree_result?;
                return Ok(status);
            }
            if !tree.is_alive()? {
                return Ok(status);
            }
        }
        if Instant::now() >= deadline {
            break;
        }
        thread::sleep(WAIT_POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())));
    }

    // Only a verified Unix process group is safe to signal after its leader
    // has been reaped. Other targets return above as soon as leader exit is
    // observed, so this force signal cannot race pid reuse.
    let tree_result = tree.kill();
    let leader_result = if leader_status.is_none() {
        child.kill()
    } else {
        Ok(())
    };
    let signal_accepted = tree_result.is_ok() || leader_result.is_ok();
    let shutdown = combine_signal_results(tree_result, leader_result, tree.covers_descendants());
    reap_blocking_after_shutdown(child, leader_status, shutdown, signal_accepted)
}

/// Force an async child and its descendants to exit without waiting.
pub fn kill_tokio_process_tree(child: &mut TokioChild) -> io::Result<()> {
    let tree = ProcessTree::from_tokio_child(child);
    let tree_covers_descendants = tree.as_ref().is_ok_and(|tree| tree.covers_descendants());
    let tree = tree.and_then(ProcessTree::kill);
    let leader = child.start_kill();
    combine_signal_results(tree, leader, tree_covers_descendants)
}

/// Gracefully terminate an async child tree, then force and reap it when the
/// grace period expires.
pub async fn terminate_tokio_process_tree(
    child: &mut TokioChild,
    grace: Duration,
) -> io::Result<ExitStatus> {
    let tree = ProcessTree::from_tokio_child(child)?;
    let graceful_tree_result = tree.terminate();
    match tokio::time::timeout(grace, child.wait()).await {
        Ok(status) => {
            let status = status?;
            if tree.covers_descendants() {
                if tree.can_signal_after_leader_exit() {
                    if tree.is_alive()? {
                        tree.kill()?;
                    }
                } else {
                    graceful_tree_result?;
                }
            }
            Ok(status)
        }
        Err(_) => {
            let tree_result = tree.kill();
            let leader_result = child.start_kill();
            let signal_accepted = tree_result.is_ok() || leader_result.is_ok();
            let shutdown =
                combine_signal_results(tree_result, leader_result, tree.covers_descendants());
            if !signal_accepted {
                return match child.try_wait() {
                    Ok(Some(status)) => combine_shutdown_and_reap(shutdown, Ok(status)),
                    Ok(None) => shutdown_without_reap(shutdown),
                    Err(reap_error) => combine_shutdown_and_reap(shutdown, Err(reap_error)),
                };
            }
            let reap = child.wait().await;
            combine_shutdown_and_reap(shutdown, reap)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::combine_signal_results;
    #[cfg(unix)]
    use super::reap_blocking_after_shutdown;
    use std::io;
    #[cfg(unix)]
    use std::process::Command;

    #[test]
    fn leader_success_does_not_mask_descendant_tree_failure() {
        let result = combine_signal_results(Err(io::Error::other("tree failed")), Ok(()), true);

        let error = result.expect_err("descendant cleanup must remain an error");
        assert!(error
            .to_string()
            .contains("descendant cleanup is unverified"));
    }

    #[test]
    fn leader_success_is_enough_for_a_direct_process_target() {
        let result = combine_signal_results(
            Err(io::Error::other("direct signal raced exit")),
            Ok(()),
            false,
        );

        result.expect("direct leader termination covers a process-only target");
    }

    #[cfg(unix)]
    #[test]
    fn shutdown_error_is_reported_only_after_the_leader_is_reaped() {
        let mut child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn child");
        child.kill().expect("kill leader");

        let error = reap_blocking_after_shutdown(
            &mut child,
            None,
            Err(io::Error::other("tree signal failed")),
            true,
        )
        .expect_err("tree signal error must be retained");

        assert!(error.to_string().contains("tree signal failed"));
        assert!(child.try_wait().expect("poll reaped child").is_some());
    }

    #[cfg(unix)]
    #[test]
    fn two_signal_failures_do_not_wait_for_a_live_leader() {
        use std::time::{Duration, Instant};

        let mut child = Command::new("sleep").arg("1").spawn().expect("spawn child");
        let started = Instant::now();

        let error = reap_blocking_after_shutdown(
            &mut child,
            None,
            Err(io::Error::other("tree and leader signals failed")),
            false,
        )
        .expect_err("an unsignaled live child cannot be awaited");

        assert!(error.to_string().contains("tree and leader signals failed"));
        assert!(
            started.elapsed() < Duration::from_millis(250),
            "failed signals must not turn into an unbounded wait"
        );
        assert!(child.try_wait().expect("poll live child").is_none());
        child.kill().expect("clean up test child");
        child.wait().expect("reap test child");
    }

    #[cfg(unix)]
    #[test]
    fn two_signal_failures_still_reap_a_leader_that_just_exited() {
        let mut child = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn child");
        child.kill().expect("simulate concurrent leader exit");
        std::thread::sleep(std::time::Duration::from_millis(20));

        let error = reap_blocking_after_shutdown(
            &mut child,
            None,
            Err(io::Error::other("tree and leader signals failed")),
            false,
        )
        .expect_err("the original shutdown error must remain visible");

        assert!(error.to_string().contains("tree and leader signals failed"));
        assert!(child.try_wait().expect("poll reaped child").is_some());
    }
}
