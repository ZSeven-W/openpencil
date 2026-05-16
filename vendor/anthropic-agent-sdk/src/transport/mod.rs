//! Transport layer for communicating with Claude Code CLI
//!
//! This module provides the transport abstraction and implementations for
//! communicating with the Claude Code CLI process.

pub mod subprocess;

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::error::Result;

/// Minimum supported Claude Code CLI version
pub const MIN_CLI_VERSION: &str = "2.0.60";

/// Transport trait for communicating with Claude Code
///
/// This trait defines the interface for sending and receiving messages
/// to/from the Claude Code CLI process.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Connect to the transport
    ///
    /// # Errors
    /// Returns error if connection fails
    async fn connect(&mut self) -> Result<()>;

    /// Write data to the transport
    ///
    /// # Arguments
    /// * `data` - String data to write (typically JSON)
    ///
    /// # Errors
    /// Returns error if write fails or transport is not ready
    async fn write(&mut self, data: &str) -> Result<()>;

    /// End the input stream (close stdin)
    ///
    /// # Errors
    /// Returns error if closing fails
    async fn end_input(&mut self) -> Result<()>;

    /// Read messages from the transport
    ///
    /// Returns a receiver that yields JSON values representing messages from Claude Code.
    /// This method spawns a background task to read messages, allowing concurrent writes.
    /// The receiver will be closed when the transport ends or encounters an error.
    fn read_messages(&mut self) -> mpsc::UnboundedReceiver<Result<serde_json::Value>>;

    /// Check if transport is ready for communication
    fn is_ready(&self) -> bool;

    /// Close the transport and clean up resources
    ///
    /// # Errors
    /// Returns error if cleanup fails
    async fn close(&mut self) -> Result<()>;
}

/// Check the Claude Code CLI version
///
/// Returns the version string if it meets minimum requirements.
///
/// # Errors
/// Returns `ClaudeError::CliVersionTooOld` if version is below minimum.
pub async fn check_claude_version(cli_path: &std::path::Path) -> crate::Result<String> {
    use tokio::process::Command;

    let output = Command::new(cli_path)
        .arg("--version")
        .output()
        .await
        .map_err(|e| crate::ClaudeError::connection(format!("Failed to get CLI version: {e}")))?;

    if !output.status.success() {
        return Err(crate::ClaudeError::connection("Failed to get CLI version"));
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    let version = version_str.trim();

    // Parse version (handle formats like "1.2.3" or "claude 1.2.3")
    let version_num = version
        .split_whitespace()
        .find(|s| s.starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or(version);

    if version_lt(version_num, MIN_CLI_VERSION) {
        return Err(crate::ClaudeError::cli_version_too_old(
            version_num,
            MIN_CLI_VERSION,
        ));
    }

    Ok(version_num.to_string())
}

/// Simple semver comparison (returns true if v1 < v2)
fn version_lt(v1: &str, v2: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };

    let v1_parts = parse(v1);
    let v2_parts = parse(v2);

    for i in 0..v1_parts.len().max(v2_parts.len()) {
        let p1 = v1_parts.get(i).copied().unwrap_or(0);
        let p2 = v2_parts.get(i).copied().unwrap_or(0);
        if p1 < p2 {
            return true;
        } else if p1 > p2 {
            return false;
        }
    }
    false
}

pub use subprocess::{PromptInput, SubprocessTransport};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_lt() {
        assert!(version_lt("1.0.0", "2.0.0"));
        assert!(version_lt("1.0.0", "1.1.0"));
        assert!(version_lt("1.0.0", "1.0.1"));
        assert!(!version_lt("2.0.0", "1.0.0"));
        assert!(!version_lt("1.0.0", "1.0.0"));
        assert!(version_lt("1.9.0", "1.10.0"));
    }
}
