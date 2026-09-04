//! Shared process IO primitives for OpenPencil native crates.

use std::ffi::OsStr;
use std::io;
use std::process::{Child, Command as StdCommand, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{ChildStderr, ChildStdin, ChildStdout, Command as TokioCommand};

mod process_tree;

use process_tree::{combine_shutdown_and_reap, combine_signal_results, shutdown_without_reap};
pub use process_tree::{
    kill_process_tree, kill_tokio_process_tree, terminate_process_tree,
    terminate_tokio_process_tree, ProcessTree,
};

/// Result of polling a spawned child while waiting for an external
/// readiness signal.
#[derive(Debug, PartialEq, Eq)]
pub enum WaitOutcome<T> {
    Ready(T),
    Exited(ExitStatus),
    TimedOut,
}

/// Apply the detached daemon stdio policy used by CLI-launched
/// OpenPencil processes.
pub fn null_stdio(command: &mut StdCommand) -> &mut StdCommand {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
}

/// Spawn a std child with stdin/stdout/stderr connected to null.
pub fn spawn_null(command: &mut StdCommand) -> io::Result<Child> {
    null_stdio(command).spawn()
}

/// Poll `probe` while also noticing if `child` exits first.
pub fn wait_for_child_or<T>(
    child: &mut Child,
    attempts: usize,
    interval: Duration,
    mut probe: impl FnMut() -> Option<T>,
) -> io::Result<WaitOutcome<T>> {
    for _ in 0..attempts {
        if let Some(value) = probe() {
            return Ok(WaitOutcome::Ready(value));
        }
        if let Some(status) = child.try_wait()? {
            return Ok(WaitOutcome::Exited(status));
        }
        thread::sleep(interval);
    }
    Ok(WaitOutcome::TimedOut)
}

/// Poll until `still_up` becomes false, returning whether it stopped
/// within the allotted attempts.
pub fn wait_until_false(
    attempts: usize,
    interval: Duration,
    mut still_up: impl FnMut() -> bool,
) -> bool {
    for _ in 0..attempts {
        if !still_up() {
            return true;
        }
        thread::sleep(interval);
    }
    false
}

/// Async stdout line stream for a piped child process.
pub type LineStream = Lines<BufReader<ChildStdout>>;

/// Tokio child wrapper with piped stdin/stdout/stderr.
pub struct LineStreamChild {
    child: tokio::process::Child,
    tree: Option<ProcessTree>,
    stdin: Option<ChildStdin>,
    lines: Option<LineStream>,
    stderr: Option<ChildStderr>,
}

impl LineStreamChild {
    /// Spawn `program` with piped stdio and the supplied args/envs.
    pub fn spawn<P, A, E, K, V>(program: P, args: A, envs: E) -> io::Result<Self>
    where
        P: AsRef<OsStr>,
        A: IntoIterator,
        A::Item: AsRef<OsStr>,
        E: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let mut command = TokioCommand::new(program);
        command.args(args);
        command.envs(envs);
        Self::spawn_command(command)
    }

    /// Spawn a preconfigured tokio command after forcing piped stdio.
    pub fn spawn_command(mut command: TokioCommand) -> io::Result<Self> {
        pipe_stdio(&mut command);
        command.kill_on_drop(true);
        let mut child = command.spawn()?;
        let tree = Some(ProcessTree::from_tokio_child(&child)?);
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("child stdout was not piped"))?;
        Ok(Self {
            stdin: child.stdin.take(),
            lines: Some(BufReader::new(stdout).lines()),
            stderr: child.stderr.take(),
            tree,
            child,
        })
    }

    /// Write bytes to stdin without adding a newline.
    pub async fn feed(&mut self, text: impl AsRef<[u8]>) -> io::Result<()> {
        let Some(stdin) = &mut self.stdin else {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "child stdin is closed",
            ));
        };
        stdin.write_all(text.as_ref()).await
    }

    /// Close stdin, signaling EOF to children that read from it.
    pub async fn close_stdin(&mut self) -> io::Result<()> {
        if let Some(mut stdin) = self.stdin.take() {
            stdin.shutdown().await?;
        }
        Ok(())
    }

    /// Read the next stdout line.
    pub async fn next_line(&mut self) -> io::Result<Option<String>> {
        match &mut self.lines {
            Some(lines) => lines.next_line().await,
            None => Ok(None),
        }
    }

    /// Borrow the active stdout line stream.
    pub fn lines(&mut self) -> Option<&mut LineStream> {
        self.lines.as_mut()
    }

    /// Move the stdout line stream out for select loops that must
    /// still operate on the child concurrently.
    pub fn take_lines(&mut self) -> Option<LineStream> {
        self.lines.take()
    }

    /// Move stderr out so callers can drain or capture it separately.
    pub fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.stderr.take()
    }

    /// Start platform termination without waiting for reaping.
    ///
    /// A returned error can still mean that the leader accepted its kill but
    /// descendant-tree signaling failed. Callers that use this low-level API
    /// must therefore perform a bounded [`Self::wait`] after either result;
    /// [`Self::kill_graceful`] owns that distinction for the common case.
    pub fn start_kill(&mut self) -> io::Result<()> {
        self.start_kill_outcome().0
    }

    fn start_kill_outcome(&mut self) -> (io::Result<()>, bool) {
        let tree_was_signaled = self.tree.is_some();
        let tree_covers_descendants = self.tree.is_some_and(ProcessTree::covers_descendants);
        let tree_result = self.tree.map_or(Ok(()), ProcessTree::kill);
        let leader_result = self.child.start_kill();
        let signal_accepted = (tree_was_signaled && tree_result.is_ok()) || leader_result.is_ok();
        (
            combine_signal_results(tree_result, leader_result, tree_covers_descendants),
            signal_accepted,
        )
    }

    /// Wait for process exit.
    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        let status = self.child.wait().await;
        if status.is_ok() {
            self.tree = None;
        }
        status
    }

    /// Close stdin and wait for the process to exit, killing it if it
    /// ignores EOF beyond `budget`.
    pub async fn kill_graceful(&mut self, budget: Duration) -> io::Result<ExitStatus> {
        // Windows taskkill discovers descendants from the live leader pid. Ask
        // it to terminate the full tree before closing stdin can let that
        // leader exit and before `wait` can reap it; signaling the numeric pid
        // afterwards would risk pid reuse. Verified Unix process groups remain
        // safe to inspect after leader exit.
        let pre_wait_tree_result = self
            .tree
            .filter(|tree| tree.requires_signal_before_wait())
            .map(ProcessTree::terminate);
        let _ = self.close_stdin().await;
        match tokio::time::timeout(budget, self.child.wait()).await {
            Ok(status) => {
                let status = status?;
                if let Some(tree) = self.tree.take() {
                    if tree.covers_descendants() {
                        if tree.can_signal_after_leader_exit() {
                            if tree.is_alive()? {
                                tree.kill()?;
                            }
                        } else if let Some(tree_result) = pre_wait_tree_result {
                            tree_result?;
                        }
                    }
                }
                Ok(status)
            }
            Err(_) => {
                let (shutdown, signal_accepted) = self.start_kill_outcome();
                if !signal_accepted {
                    return match self.child.try_wait() {
                        Ok(Some(status)) => {
                            self.tree = None;
                            combine_shutdown_and_reap(shutdown, Ok(status))
                        }
                        Ok(None) => shutdown_without_reap(shutdown),
                        Err(reap_error) => combine_shutdown_and_reap(shutdown, Err(reap_error)),
                    };
                }
                let reap = self.child.wait().await;
                if reap.is_ok() {
                    self.tree = None;
                }
                combine_shutdown_and_reap(shutdown, reap)
            }
        }
    }
}

impl Drop for LineStreamChild {
    fn drop(&mut self) {
        if self.child.id().is_some() {
            if let Some(tree) = self.tree.take() {
                let _ = tree.kill();
            }
            let _ = self.child.start_kill();
        }
    }
}

/// Apply the piped stdio policy used by line-stream subprocesses.
pub fn pipe_stdio(command: &mut TokioCommand) -> &mut TokioCommand {
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
}
