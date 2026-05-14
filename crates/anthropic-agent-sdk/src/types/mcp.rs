//! MCP (Model Context Protocol) server configuration types
//!
//! This module provides configuration types for MCP servers that can be passed
//! to Claude Code CLI. For in-process SDK servers, enable the `rmcp` feature.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ============================================================================
// MCP Server Types
// ============================================================================

/// MCP stdio server configuration
///
/// Used to spawn an MCP server as a subprocess.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStdioServerConfig {
    /// Server type (stdio)
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub server_type: Option<String>,
    /// Command to execute
    pub command: String,
    /// Command arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    /// Environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// MCP SSE server configuration
///
/// Used to connect to an MCP server via Server-Sent Events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSseServerConfig {
    /// Server type (sse)
    #[serde(rename = "type")]
    pub server_type: String,
    /// Server URL
    pub url: String,
    /// HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// MCP HTTP server configuration
///
/// Used to connect to an MCP server via HTTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHttpServerConfig {
    /// Server type (http)
    #[serde(rename = "type")]
    pub server_type: String,
    /// Server URL
    pub url: String,
    /// HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

/// SDK MCP server configuration
///
/// Configuration for an in-process MCP server. The actual server instance
/// is created using rmcp and managed separately.
///
/// # Example
///
/// ```ignore
/// use anthropic_agent_sdk::types::SdkMcpServerConfig;
///
/// let config = SdkMcpServerConfig {
///     name: "my-tools".to_string(),
///     version: Some("1.0.0".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkMcpServerConfig {
    /// Server name (used as identifier)
    pub name: String,
    /// Server version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// MCP server configuration enum
///
/// Represents the different types of MCP server connections supported.
#[derive(Debug, Clone)]
pub enum McpServerConfig {
    /// Stdio-based MCP server (spawns subprocess)
    Stdio(McpStdioServerConfig),
    /// SSE-based MCP server (connects via Server-Sent Events)
    Sse(McpSseServerConfig),
    /// HTTP-based MCP server (connects via HTTP)
    Http(McpHttpServerConfig),
    /// SDK-based in-process MCP server (requires `rmcp` feature)
    Sdk(SdkMcpServerConfig),
}

/// MCP servers container
///
/// Specifies how MCP servers are configured for a session.
#[derive(Debug, Clone, Default)]
pub enum McpServers {
    /// No MCP servers
    #[default]
    None,
    /// Dictionary of MCP servers (inline configuration)
    Dict(HashMap<String, McpServerConfig>),
    /// Path to MCP servers configuration file
    Path(PathBuf),
}
