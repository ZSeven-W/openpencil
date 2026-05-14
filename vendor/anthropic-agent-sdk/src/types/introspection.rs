//! Introspection types for querying Claude Code capabilities
//!
//! These types provide information about the current Claude Code session,
//! including available models, tools, and account information.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Session Information (from init message)
// ============================================================================

/// Tool information from the init message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,
    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Input schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
}

/// MCP server status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerStatus {
    /// Server name
    pub name: String,
    /// Connection status: "connected", "failed", "needs-auth", "pending"
    #[serde(default)]
    pub status: String,
    /// Error message if connection failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Available tools from this server
    #[serde(default)]
    pub tools: Vec<String>,
}

impl McpServerStatus {
    /// Check if the server is connected
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.status == "connected"
    }
}

/// Session initialization data captured from the init message
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Current model being used
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Available tools
    #[serde(default)]
    pub tools: Vec<ToolInfo>,
    /// Working directory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    /// MCP server statuses
    #[serde(default)]
    pub mcp_servers: Vec<McpServerStatus>,
    /// Additional raw data from init
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl SessionInfo {
    /// Create from init message data
    pub fn from_init_data(data: &serde_json::Value) -> Self {
        let model = data.get("model").and_then(|v| v.as_str()).map(String::from);

        let cwd = data.get("cwd").and_then(|v| v.as_str()).map(String::from);

        // CLI sends tools as array of strings: ["Task", "Bash", "Glob", ...]
        let tools = data
            .get("tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        // Handle string format (from CLI)
                        if let Some(name) = t.as_str() {
                            return Some(ToolInfo {
                                name: name.to_string(),
                                description: None,
                                input_schema: None,
                            });
                        }
                        // Handle object format (for future compatibility)
                        let name = t.get("name").and_then(|n| n.as_str())?;
                        Some(ToolInfo {
                            name: name.to_string(),
                            description: t
                                .get("description")
                                .and_then(|d| d.as_str())
                                .map(String::from),
                            input_schema: t.get("input_schema").cloned(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Extract all tool names for MCP tool matching
        let all_tools: Vec<String> = data
            .get("tools")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| t.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let mcp_servers = data
            .get("mcp_servers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|s| {
                        let name = s.get("name").and_then(|n| n.as_str())?;

                        // Extract MCP tools from main tools array
                        // Format: mcp__servername__toolname (hyphens preserved)
                        let server_prefix = format!("mcp__{name}__");
                        let server_tools: Vec<String> = all_tools
                            .iter()
                            .filter(|t| t.starts_with(&server_prefix))
                            .map(|t| t[server_prefix.len()..].to_string())
                            .collect();

                        Some(McpServerStatus {
                            name: name.to_string(),
                            status: s
                                .get("status")
                                .and_then(|c| c.as_str())
                                .unwrap_or("unknown")
                                .to_string(),
                            error: s.get("error").and_then(|e| e.as_str()).map(String::from),
                            tools: server_tools,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Collect extra fields
        let extra = data
            .as_object()
            .map(|obj| {
                obj.iter()
                    .filter(|(k, _)| {
                        !["model", "cwd", "tools", "mcp_servers"].contains(&k.as_str())
                    })
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            model,
            tools,
            cwd,
            mcp_servers,
            extra,
        }
    }

    /// Get tool names
    #[must_use]
    pub fn tool_names(&self) -> Vec<&str> {
        self.tools.iter().map(|t| t.name.as_str()).collect()
    }

    /// Check if a tool is available
    #[must_use]
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.iter().any(|t| t.name == name)
    }

    /// Get MCP server by name
    #[must_use]
    pub fn mcp_server(&self, name: &str) -> Option<&McpServerStatus> {
        self.mcp_servers.iter().find(|s| s.name == name)
    }

    /// Check if any MCP server has errors
    #[must_use]
    pub fn has_mcp_errors(&self) -> bool {
        self.mcp_servers.iter().any(|s| s.error.is_some())
    }

    /// Get the current permission mode from session
    ///
    /// Returns the permission mode string (e.g., "default", "plan", "acceptEdits", "bypassPermissions")
    #[must_use]
    pub fn permission_mode(&self) -> Option<&str> {
        self.extra.get("permissionMode").and_then(|v| v.as_str())
    }

    /// Check if session is in plan mode
    #[must_use]
    pub fn is_plan_mode(&self) -> bool {
        self.permission_mode() == Some("plan")
    }
}

// ============================================================================
// Account Information
// ============================================================================

/// Account information from OAuth credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    /// User email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// Account ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// Whether this is an OAuth account (vs API key)
    pub is_oauth: bool,
    /// Organization ID (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
}

// ============================================================================
// Supported Models
// ============================================================================

/// Information about a supported model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier
    pub id: String,
    /// Human-readable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Maximum context window
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Whether extended thinking is supported
    #[serde(default)]
    pub supports_thinking: bool,
}

/// Known Claude models (static list, can be updated)
impl ModelInfo {
    /// Get list of known Claude models
    #[must_use]
    pub fn known_models() -> Vec<Self> {
        vec![
            Self {
                id: "claude-sonnet-4-20250514".to_string(),
                name: Some("Claude Sonnet 4".to_string()),
                max_tokens: Some(200_000),
                supports_thinking: true,
            },
            Self {
                id: "claude-opus-4-20250514".to_string(),
                name: Some("Claude Opus 4".to_string()),
                max_tokens: Some(200_000),
                supports_thinking: true,
            },
            Self {
                id: "claude-3-5-sonnet-20241022".to_string(),
                name: Some("Claude 3.5 Sonnet".to_string()),
                max_tokens: Some(200_000),
                supports_thinking: false,
            },
            Self {
                id: "claude-3-5-haiku-20241022".to_string(),
                name: Some("Claude 3.5 Haiku".to_string()),
                max_tokens: Some(200_000),
                supports_thinking: false,
            },
            Self {
                id: "claude-3-opus-20240229".to_string(),
                name: Some("Claude 3 Opus".to_string()),
                max_tokens: Some(200_000),
                supports_thinking: false,
            },
        ]
    }
}

// ============================================================================
// Slash Commands
// ============================================================================

/// Information about an available slash command
///
/// Slash commands are custom user-defined commands that can be invoked
/// during a Claude Code session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    /// Command name (without the leading slash)
    pub name: String,
    /// Human-readable description of the command
    pub description: String,
    /// Hint for the command arguments (e.g., "<`file_path`>")
    #[serde(rename = "argumentHint")]
    pub argument_hint: String,
}

// ============================================================================
// Permission Denials
// ============================================================================

/// Information about a denied tool use
///
/// Returned in Result messages to indicate which tool uses were
/// denied by the permission system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SDKPermissionDenial {
    /// Name of the tool that was denied
    pub tool_name: String,
    /// ID of the tool use request that was denied
    pub tool_use_id: String,
    /// Input parameters that were passed to the tool
    pub tool_input: serde_json::Value,
}

// ============================================================================
// Model Usage Statistics
// ============================================================================

/// Per-model usage statistics returned in result messages
///
/// Provides detailed token usage and cost breakdown for each model
/// used during the conversation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelUsage {
    /// Number of input tokens consumed
    #[serde(rename = "inputTokens", default)]
    pub input_tokens: u64,
    /// Number of output tokens generated
    #[serde(rename = "outputTokens", default)]
    pub output_tokens: u64,
    /// Tokens read from cache
    #[serde(rename = "cacheReadInputTokens", default)]
    pub cache_read_input_tokens: u64,
    /// Tokens used to create cache
    #[serde(rename = "cacheCreationInputTokens", default)]
    pub cache_creation_input_tokens: u64,
    /// Number of web search requests made
    #[serde(rename = "webSearchRequests", default)]
    pub web_search_requests: u64,
    /// Total cost in USD
    #[serde(rename = "costUSD", default)]
    pub cost_usd: f64,
    /// Context window size used
    #[serde(rename = "contextWindow", default)]
    pub context_window: u64,
}

impl ModelUsage {
    /// Calculate total tokens (input + output)
    #[must_use]
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }

    /// Calculate effective input tokens (including cache)
    #[must_use]
    pub fn effective_input_tokens(&self) -> u64 {
        self.input_tokens
            .saturating_add(self.cache_read_input_tokens)
            .saturating_add(self.cache_creation_input_tokens)
    }
}
