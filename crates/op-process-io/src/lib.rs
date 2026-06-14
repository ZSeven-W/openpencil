//! Shared process IO primitives for OpenPencil native crates.

use std::ffi::OsStr;
use std::io;
use std::process::{Child, Command as StdCommand, ExitStatus, Stdio};
use std::thread;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{ChildStderr, ChildStdin, ChildStdout, Command as TokioCommand};

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
        let mut child = command.spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("child stdout was not piped"))?;
        Ok(Self {
            stdin: child.stdin.take(),
            lines: Some(BufReader::new(stdout).lines()),
            stderr: child.stderr.take(),
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
    pub fn start_kill(&mut self) -> io::Result<()> {
        self.child.start_kill()
    }

    /// Wait for process exit.
    pub async fn wait(&mut self) -> io::Result<ExitStatus> {
        self.child.wait().await
    }

    /// Close stdin and wait for the process to exit, killing it if it
    /// ignores EOF beyond `budget`.
    pub async fn kill_graceful(&mut self, budget: Duration) -> io::Result<ExitStatus> {
        let _ = self.close_stdin().await;
        match tokio::time::timeout(budget, self.child.wait()).await {
            Ok(status) => status,
            Err(_) => {
                self.child.start_kill()?;
                self.child.wait().await
            }
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
