// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! Error types for the Copilot SDK.

use std::time::Duration;
use thiserror::Error;

/// Main error type for the Copilot SDK.
#[derive(Debug, Error)]
pub enum CopilotError {
    /// Transport/IO error
    #[error("Transport error: {0}")]
    Transport(#[from] std::io::Error),

    /// Connection was closed unexpectedly
    #[error("Connection closed")]
    ConnectionClosed,

    /// Client is not connected
    #[error("Not connected")]
    NotConnected,

    /// JSON-RPC error from server
    #[error("JSON-RPC error {code}: {message}")]
    JsonRpc {
        code: i32,
        message: String,
        data: Option<serde_json::Value>,
    },

    /// Protocol version mismatch
    #[error("Protocol version mismatch: SDK supports versions {min}-{max}, but server reports version {actual}. Please update your SDK or server to ensure compatibility.")]
    ProtocolMismatch { min: u32, max: u32, actual: u32 },

    /// Protocol error (framing, invalid messages, etc.)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Request timed out
    #[error("Request timed out after {0:?}")]
    Timeout(Duration),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Session was already destroyed
    #[error("Session already destroyed")]
    SessionDestroyed,

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Failed to start CLI process
    #[error("Failed to start CLI: {0}")]
    ProcessStart(std::io::Error),

    /// CLI process exited unexpectedly
    #[error("CLI exited unexpectedly with code {0:?}")]
    ProcessExit(Option<i32>),

    /// Port detection failed in TCP mode
    #[error("Failed to detect CLI server port")]
    PortDetectionFailed,

    /// Client is shutting down
    #[error("Client is shutting down")]
    Shutdown,

    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Tool execution error
    #[error("Tool execution error: {0}")]
    ToolError(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Channel send error (internal)
    #[error("Internal channel error")]
    ChannelError,
}

/// Result type alias for Copilot SDK operations.
pub type Result<T> = std::result::Result<T, CopilotError>;

impl CopilotError {
    /// Create a JSON-RPC error from components.
    pub fn json_rpc(
        code: i32,
        message: impl Into<String>,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self::JsonRpc {
            code,
            message: message.into(),
            data,
        }
    }

    /// Create an invalid config error.
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Returns true if this error indicates the connection is no longer usable.
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            CopilotError::ConnectionClosed
                | CopilotError::ProcessExit(_)
                | CopilotError::Shutdown
                | CopilotError::ProtocolMismatch { .. }
        )
    }
}

// Convert oneshot channel errors to our error type
impl From<tokio::sync::oneshot::error::RecvError> for CopilotError {
    fn from(_: tokio::sync::oneshot::error::RecvError) -> Self {
        CopilotError::ConnectionClosed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CopilotError::ProtocolMismatch {
            min: 2,
            max: 3,
            actual: 1,
        };
        assert_eq!(
            err.to_string(),
            "Protocol version mismatch: SDK supports versions 2-3, but server reports version 1. Please update your SDK or server to ensure compatibility."
        );
    }

    #[test]
    fn test_is_fatal() {
        assert!(CopilotError::ConnectionClosed.is_fatal());
        assert!(CopilotError::Shutdown.is_fatal());
        assert!(!CopilotError::Timeout(Duration::from_secs(30)).is_fatal());
    }
}
