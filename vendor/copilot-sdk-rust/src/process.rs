// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! Process management for the Copilot SDK.
//!
//! Provides async subprocess spawning and management for the Copilot CLI.

use crate::error::{CopilotError, Result};
use crate::transport::StdioTransport;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::{Child, Command};

// =============================================================================
// Process Options
// =============================================================================

/// Options for spawning a subprocess.
#[derive(Debug, Clone)]
pub struct ProcessOptions {
    /// Working directory for the subprocess (None = inherit from parent).
    pub working_directory: Option<PathBuf>,

    /// Environment variables to set.
    pub environment: HashMap<String, String>,

    /// Whether to inherit the parent's environment variables.
    pub inherit_environment: bool,

    /// Whether to redirect stdin (pipe to subprocess).
    pub redirect_stdin: bool,

    /// Whether to redirect stdout (pipe from subprocess).
    pub redirect_stdout: bool,

    /// Whether to redirect stderr (pipe from subprocess).
    pub redirect_stderr: bool,
}

impl Default for ProcessOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessOptions {
    /// Create new process options with default values.
    pub fn new() -> Self {
        Self {
            working_directory: None,
            environment: HashMap::new(),
            inherit_environment: true,
            redirect_stdin: true,
            redirect_stdout: true,
            redirect_stderr: false,
        }
    }

    /// Set working directory.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(dir.into());
        self
    }

    /// Add environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.environment.insert(key.into(), value.into());
        self
    }

    /// Set whether to inherit parent environment.
    pub fn inherit_env(mut self, inherit: bool) -> Self {
        self.inherit_environment = inherit;
        self
    }

    /// Set stdin redirection.
    pub fn stdin(mut self, redirect: bool) -> Self {
        self.redirect_stdin = redirect;
        self
    }

    /// Set stdout redirection.
    pub fn stdout(mut self, redirect: bool) -> Self {
        self.redirect_stdout = redirect;
        self
    }

    /// Set stderr redirection.
    pub fn stderr(mut self, redirect: bool) -> Self {
        self.redirect_stderr = redirect;
        self
    }
}

// =============================================================================
// Copilot Process
// =============================================================================

/// A running Copilot CLI process.
pub struct CopilotProcess {
    child: Child,
    transport: Option<StdioTransport>,
    stdout: Option<tokio::process::ChildStdout>,
    stderr: Option<tokio::process::ChildStderr>,
}

impl CopilotProcess {
    /// Spawn a new Copilot CLI process.
    pub fn spawn(
        executable: impl AsRef<Path>,
        args: &[&str],
        options: ProcessOptions,
    ) -> Result<Self> {
        let executable = executable.as_ref();

        // Build command
        let mut cmd = Command::new(executable);
        cmd.args(args);

        // Set working directory
        if let Some(dir) = &options.working_directory {
            cmd.current_dir(dir);
        }

        // Set environment
        if !options.inherit_environment {
            cmd.env_clear();
        }
        for (key, value) in &options.environment {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdin(if options.redirect_stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        cmd.stdout(if options.redirect_stdout {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        cmd.stderr(if options.redirect_stderr {
            Stdio::piped()
        } else {
            Stdio::null()
        });

        // Spawn the process
        let mut child = cmd.spawn().map_err(CopilotError::ProcessStart)?;

        // Create transport from stdio handles
        let transport = if options.redirect_stdin && options.redirect_stdout {
            let stdin = child
                .stdin
                .take()
                .ok_or_else(|| CopilotError::InvalidConfig("Failed to capture stdin".into()))?;
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| CopilotError::InvalidConfig("Failed to capture stdout".into()))?;
            Some(StdioTransport::new(stdin, stdout))
        } else {
            None
        };

        // Capture stdout if redirected but not used for stdio transport.
        let stdout = if transport.is_none() && options.redirect_stdout {
            child.stdout.take()
        } else {
            None
        };

        // Capture stderr if redirected
        let stderr = if options.redirect_stderr {
            child.stderr.take()
        } else {
            None
        };

        Ok(Self {
            child,
            transport,
            stdout,
            stderr,
        })
    }

    /// Spawn the Copilot CLI with default options for stdio mode.
    pub fn spawn_stdio(cli_path: impl AsRef<Path>) -> Result<Self> {
        let options = ProcessOptions::new().stdin(true).stdout(true).stderr(false);

        Self::spawn(cli_path, &["--stdio"], options)
    }

    /// Take the transport (can only be called once).
    ///
    /// Returns the stdio transport for communication with the CLI.
    pub fn take_transport(&mut self) -> Option<StdioTransport> {
        self.transport.take()
    }

    /// Take stdout (can only be called once).
    pub fn take_stdout(&mut self) -> Option<tokio::process::ChildStdout> {
        self.stdout.take()
    }

    /// Get the process ID.
    pub fn id(&self) -> Option<u32> {
        self.child.id()
    }

    /// Check if the process is still running.
    pub async fn is_running(&mut self) -> bool {
        self.child.try_wait().ok().flatten().is_none()
    }

    /// Try to get the exit status without blocking.
    pub async fn try_wait(&mut self) -> Result<Option<i32>> {
        match self.child.try_wait() {
            Ok(Some(status)) => Ok(Some(status.code().unwrap_or(-1))),
            Ok(None) => Ok(None),
            Err(e) => Err(CopilotError::Transport(e)),
        }
    }

    /// Wait for the process to exit.
    pub async fn wait(&mut self) -> Result<i32> {
        let status = self.child.wait().await.map_err(CopilotError::Transport)?;
        Ok(status.code().unwrap_or(-1))
    }

    /// Request termination of the process.
    ///
    /// On Unix, this sends SIGTERM. On Windows, this kills the process.
    pub fn terminate(&mut self) -> Result<()> {
        // Use kill for cross-platform simplicity
        // A more sophisticated implementation could use SIGTERM on Unix
        self.kill()
    }

    /// Forcefully kill the process.
    pub fn kill(&mut self) -> Result<()> {
        self.child.start_kill().map_err(CopilotError::Transport)
    }

    /// Take stderr (can only be called once).
    pub fn take_stderr(&mut self) -> Option<tokio::process::ChildStderr> {
        self.stderr.take()
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Find an executable in the system PATH.
///
/// Returns the full path to the executable if found.
pub fn find_executable(name: &str) -> Option<PathBuf> {
    which::which(name).ok()
}

/// Check if a path looks like a Node.js script.
pub fn is_node_script(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext == "js" || ext == "mjs")
}

/// Get the system's Node.js executable path.
pub fn find_node() -> Option<PathBuf> {
    find_executable("node")
}

/// Find the Copilot CLI executable.
///
/// Searches for the Copilot CLI in common locations and the system PATH.
pub fn find_copilot_cli() -> Option<PathBuf> {
    // First, allow an explicit override to match the upstream SDKs.
    if let Ok(cli_path) = std::env::var("COPILOT_CLI_PATH") {
        let cli_path = cli_path.trim();
        if !cli_path.is_empty() {
            let path = PathBuf::from(cli_path);
            if path.exists() {
                return Some(path);
            }
        }
    }

    // First, try the system PATH
    if let Some(path) = find_executable("copilot") {
        return Some(path);
    }

    // On Windows, also try "copilot.cmd" and "copilot.exe"
    #[cfg(windows)]
    {
        if let Some(path) = find_executable("copilot.cmd") {
            return Some(path);
        }
        if let Some(path) = find_executable("copilot.exe") {
            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_options_builder() {
        let options = ProcessOptions::new()
            .working_dir("/tmp")
            .env("FOO", "bar")
            .inherit_env(false)
            .stdin(true)
            .stdout(true)
            .stderr(true);

        assert_eq!(options.working_directory, Some(PathBuf::from("/tmp")));
        assert_eq!(options.environment.get("FOO"), Some(&"bar".to_string()));
        assert!(!options.inherit_environment);
        assert!(options.redirect_stdin);
        assert!(options.redirect_stdout);
        assert!(options.redirect_stderr);
    }

    #[test]
    fn test_process_options_default() {
        let options = ProcessOptions::default();

        assert!(options.working_directory.is_none());
        assert!(options.environment.is_empty());
        assert!(options.inherit_environment);
        assert!(options.redirect_stdin);
        assert!(options.redirect_stdout);
        assert!(!options.redirect_stderr);
    }

    #[test]
    fn test_is_node_script() {
        assert!(is_node_script(Path::new("script.js")));
        assert!(is_node_script(Path::new("script.mjs")));
        assert!(is_node_script(Path::new("/path/to/script.js")));
        assert!(!is_node_script(Path::new("script.ts")));
        assert!(!is_node_script(Path::new("script")));
        assert!(!is_node_script(Path::new("script.py")));
    }

    #[test]
    fn test_find_node() {
        // This test just verifies the function doesn't panic
        // Whether it finds node depends on the system
        let _ = find_node();
    }

    #[test]
    fn test_find_copilot_cli() {
        // This test just verifies the function doesn't panic
        // Whether it finds copilot depends on the system
        let _ = find_copilot_cli();
    }
}
