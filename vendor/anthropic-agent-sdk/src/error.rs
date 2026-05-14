//! Error types for the Claude Agent SDK

use thiserror::Error;

/// Main error type for the Claude Agent SDK
#[derive(Error, Debug)]
pub enum ClaudeError {
    /// Claude Code CLI not found or not installed
    #[error("Claude Code CLI not found: {0}")]
    CliNotFound(String),

    /// Connection error when communicating with Claude Code
    #[error("Connection error: {0}")]
    Connection(String),

    /// Process execution error with exit code and stderr
    #[error("Process error (exit code {exit_code}): {message}")]
    Process {
        /// Error message
        message: String,
        /// Process exit code
        exit_code: i32,
        /// Standard error output
        stderr: Option<String>,
    },

    /// JSON decode error when parsing CLI output
    #[error("JSON decode error: {0}")]
    JsonDecode(#[from] serde_json::Error),

    /// JSON encode error
    #[error("JSON encode error: {0}")]
    JsonEncode(String),

    /// Message parse error with optional raw data
    #[error("Message parse error: {message}")]
    MessageParse {
        /// Error message
        message: String,
        /// Raw message data that failed to parse
        data: Option<serde_json::Value>,
    },

    /// Transport layer error
    #[error("Transport error: {0}")]
    Transport(String),

    /// Control protocol error
    #[error("Control protocol error: {0}")]
    ControlProtocol(String),

    /// Hook execution error
    #[error("Hook error: {0}")]
    Hook(String),

    /// MCP (Model Context Protocol) error
    #[error("MCP error: {0}")]
    Mcp(String),

    /// I/O error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Timeout error
    #[error("Timeout: {0}\nSuggestion: Increase timeout duration or check network connectivity")]
    Timeout(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// CLI version is below minimum required
    #[error(
        "Claude Code version {found} is below minimum required version {minimum}. Update with: npm update -g @anthropic-ai/claude-code"
    )]
    CliVersionTooOld {
        /// Version found
        found: String,
        /// Minimum version required
        minimum: String,
    },

    /// Control protocol request timed out
    #[error("Control request timed out after {timeout_secs} seconds: {request_type}")]
    ControlTimeout {
        /// Timeout duration in seconds
        timeout_secs: u64,
        /// Type of request that timed out
        request_type: String,
    },

    /// Client is not connected
    #[error("Not connected. Call connect() or create client before operations.")]
    NotConnected,

    /// Client is already connected
    #[error("Already connected. Close existing connection first.")]
    AlreadyConnected,

    /// Session mismatch - bound session differs from current session
    #[error("Session mismatch: expected {expected}, got {actual}")]
    SessionMismatch {
        /// Expected session ID (bound)
        expected: String,
        /// Actual session ID (current)
        actual: String,
    },

    /// Authentication error (OAuth/API key issues)
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Network error during API request
    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Result type alias for Claude SDK operations
pub type Result<T> = std::result::Result<T, ClaudeError>;

impl ClaudeError {
    /// Create a CLI not found error with a helpful message
    #[must_use]
    pub fn cli_not_found() -> Self {
        Self::CliNotFound(
            "Claude Code not found. Install with:\n\
             npm install -g @anthropic-ai/claude-code\n\
             \n\
             If already installed locally, try:\n\
             export PATH=\"$HOME/node_modules/.bin:$PATH\"\n\
             \n\
             Or specify the path when creating transport"
                .to_string(),
        )
    }

    /// Create a connection error
    pub fn connection(msg: impl Into<String>) -> Self {
        Self::Connection(msg.into())
    }

    /// Create a process error
    pub fn process(msg: impl Into<String>, exit_code: i32, stderr: Option<String>) -> Self {
        Self::Process {
            message: msg.into(),
            exit_code,
            stderr,
        }
    }

    /// Create a message parse error
    pub fn message_parse(msg: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        Self::MessageParse {
            message: msg.into(),
            data,
        }
    }

    /// Create a transport error
    pub fn transport(msg: impl Into<String>) -> Self {
        Self::Transport(msg.into())
    }

    /// Create a control protocol error
    pub fn control_protocol(msg: impl Into<String>) -> Self {
        Self::ControlProtocol(msg.into())
    }

    /// Alias for `control_protocol` - used by protocol handler
    pub fn protocol_error(msg: impl Into<String>) -> Self {
        Self::ControlProtocol(msg.into())
    }

    /// Create a JSON encode error
    pub fn json_encode(msg: impl Into<String>) -> Self {
        Self::JsonEncode(msg.into())
    }

    /// Create a JSON decode error from string (for custom messages)
    pub fn json_decode_msg(msg: impl Into<String>) -> Self {
        Self::MessageParse {
            message: msg.into(),
            data: None,
        }
    }

    /// Create a hook error
    pub fn hook(msg: impl Into<String>) -> Self {
        Self::Hook(msg.into())
    }

    /// Create an MCP error
    pub fn mcp(msg: impl Into<String>) -> Self {
        Self::Mcp(msg.into())
    }

    /// Create a timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }

    /// Create an invalid configuration error
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        Self::InvalidConfig(msg.into())
    }

    /// Create a CLI version too old error
    pub fn cli_version_too_old(found: impl Into<String>, minimum: impl Into<String>) -> Self {
        Self::CliVersionTooOld {
            found: found.into(),
            minimum: minimum.into(),
        }
    }

    /// Create a control timeout error
    pub fn control_timeout(timeout_secs: u64, request_type: impl Into<String>) -> Self {
        Self::ControlTimeout {
            timeout_secs,
            request_type: request_type.into(),
        }
    }

    /// Create a not connected error
    #[must_use]
    pub fn not_connected() -> Self {
        Self::NotConnected
    }

    /// Create an already connected error
    #[must_use]
    pub fn already_connected() -> Self {
        Self::AlreadyConnected
    }

    /// Create a session mismatch error
    pub fn session_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::SessionMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create an authentication error
    pub fn authentication(msg: impl Into<String>) -> Self {
        Self::AuthenticationError(msg.into())
    }

    /// Create a network error
    pub fn network(msg: impl Into<String>) -> Self {
        Self::NetworkError(msg.into())
    }
}
