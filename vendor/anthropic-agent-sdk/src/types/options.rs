//! Claude Agent configuration options

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use typed_builder::TypedBuilder;

use super::hooks::{HookEvent, HookMatcher};
use super::identifiers::ToolName;
use super::mcp::McpServers;
use super::permissions::{CanUseToolCallback, PermissionMode, SettingSource};

// ============================================================================
// System Prompt Types
// ============================================================================

/// System prompt preset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemPromptPreset {
    /// Prompt type (always "preset")
    #[serde(rename = "type")]
    pub prompt_type: String,
    /// Preset name (e.g., "`claude_code`")
    pub preset: String,
    /// Additional text to append to the preset
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<String>,
}

/// System prompt configuration
#[derive(Debug, Clone)]
pub enum SystemPrompt {
    /// Plain string system prompt
    String(String),
    /// Preset-based system prompt
    Preset(SystemPromptPreset),
    /// Load system prompt from a file (uses `--system-prompt-file` CLI flag)
    File(PathBuf),
}

// Implement conversions for SystemPrompt
impl From<String> for SystemPrompt {
    fn from(s: String) -> Self {
        SystemPrompt::String(s)
    }
}

impl From<&str> for SystemPrompt {
    fn from(s: &str) -> Self {
        SystemPrompt::String(s.to_string())
    }
}

impl From<SystemPromptPreset> for SystemPrompt {
    fn from(preset: SystemPromptPreset) -> Self {
        SystemPrompt::Preset(preset)
    }
}

impl From<PathBuf> for SystemPrompt {
    fn from(path: PathBuf) -> Self {
        SystemPrompt::File(path)
    }
}

impl SystemPrompt {
    /// Create a file-based system prompt
    #[must_use]
    pub fn from_file(path: impl Into<PathBuf>) -> Self {
        SystemPrompt::File(path.into())
    }
}

// ============================================================================
// Tools Configuration
// ============================================================================

/// Tools configuration - either a list of tool names or a preset
#[derive(Debug, Clone)]
pub enum ToolsConfig {
    /// Explicit list of tool names
    List(Vec<ToolName>),
    /// Use a preset (e.g., `claude_code`)
    Preset(ToolsPreset),
}

/// Tools preset configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsPreset {
    /// Type (always "preset")
    #[serde(rename = "type")]
    pub config_type: String,
    /// Preset name
    pub preset: String,
}

impl ToolsPreset {
    /// Create the Claude Code tools preset
    #[must_use]
    pub fn claude_code() -> Self {
        Self {
            config_type: "preset".to_string(),
            preset: "claude_code".to_string(),
        }
    }
}

impl ToolsConfig {
    /// Create a Claude Code preset
    #[must_use]
    pub fn claude_code_preset() -> Self {
        Self::Preset(ToolsPreset::claude_code())
    }

    /// Create from a list of tool names
    #[must_use]
    pub fn from_list(tools: Vec<ToolName>) -> Self {
        Self::List(tools)
    }
}

impl From<Vec<ToolName>> for ToolsConfig {
    fn from(tools: Vec<ToolName>) -> Self {
        Self::List(tools)
    }
}

impl From<ToolsPreset> for ToolsConfig {
    fn from(preset: ToolsPreset) -> Self {
        Self::Preset(preset)
    }
}

// ============================================================================
// Stderr Callback
// ============================================================================

use std::sync::Arc;

/// Callback for stderr output
///
/// This callback is invoked when the Claude CLI writes to stderr.
/// Useful for debugging and logging purposes.
pub type StderrCallback = Arc<dyn Fn(String) + Send + Sync>;

// ============================================================================
// Output Format
// ============================================================================

/// Output format configuration for structured outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputFormat {
    /// Format type (always "`json_schema`")
    #[serde(rename = "type")]
    pub format_type: String,
    /// JSON schema for the output
    pub schema: serde_json::Value,
}

impl OutputFormat {
    /// Create a new JSON schema output format
    #[must_use]
    pub fn json_schema(schema: serde_json::Value) -> Self {
        Self {
            format_type: "json_schema".to_string(),
            schema,
        }
    }
}

// ============================================================================
// Sandbox Settings
// ============================================================================

/// Network-specific configuration for sandbox mode
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkSandboxSettings {
    /// Allow processes to bind to local ports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_local_binding: Option<bool>,
    /// Unix socket paths that processes can access
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unix_sockets: Option<Vec<String>>,
    /// Allow access to all Unix sockets
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_all_unix_sockets: Option<bool>,
    /// HTTP proxy port for network requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_proxy_port: Option<u16>,
    /// SOCKS proxy port for network requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub socks_proxy_port: Option<u16>,
}

/// Configuration for ignoring specific sandbox violations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxIgnoreViolations {
    /// File path patterns to ignore violations for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<Vec<String>>,
    /// Network patterns to ignore violations for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Vec<String>>,
}

/// Configuration for sandbox behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SandboxSettings {
    /// Enable sandbox mode for command execution
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Auto-approve bash commands when sandbox is enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_allow_bash_if_sandboxed: Option<bool>,
    /// Commands that always bypass sandbox restrictions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_commands: Option<Vec<String>>,
    /// Allow the model to request running commands outside the sandbox
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_unsandboxed_commands: Option<bool>,
    /// Network-specific sandbox configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkSandboxSettings>,
    /// Configure which sandbox violations to ignore
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_violations: Option<SandboxIgnoreViolations>,
    /// Enable a weaker nested sandbox for compatibility
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_weaker_nested_sandbox: Option<bool>,
}

// ============================================================================
// Plugin Configuration
// ============================================================================

/// Configuration for loading plugins
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SdkPluginConfig {
    /// Local plugin from filesystem path
    #[serde(rename = "local")]
    Local {
        /// Path to the plugin directory
        path: String,
    },
}

// ============================================================================
// Beta Features
// ============================================================================

/// Available beta features
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SdkBeta {
    /// 1 million token context window (compatible with Claude Sonnet 4, Claude Sonnet 4.5)
    #[serde(rename = "context-1m-2025-08-07")]
    Context1M,
}

// ============================================================================
// Agent Definition
// ============================================================================

/// Agent definition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent description
    pub description: String,
    /// Agent system prompt
    pub prompt: String,
    /// Tools available to the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    /// Model to use for the agent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

// ============================================================================
// Claude Agent Options
// ============================================================================

/// Main options for Claude Agent SDK
#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Default, TypedBuilder)]
#[builder(
    builder_method(doc = "Create a new builder for ClaudeAgentOptions"),
    builder_type(doc = "Builder for ClaudeAgentOptions", vis = "pub"),
    build_method(doc = "Build the ClaudeAgentOptions")
)]
pub struct ClaudeAgentOptions {
    /// List of tools that Claude is allowed to use
    #[builder(default, setter(into))]
    pub allowed_tools: Vec<ToolName>,

    /// System prompt configuration
    #[builder(default, setter(strip_option, into))]
    pub system_prompt: Option<SystemPrompt>,

    /// Additional text to append to the system prompt (uses `--append-system-prompt` CLI flag)
    #[builder(default, setter(strip_option, into))]
    pub append_system_prompt: Option<String>,

    /// MCP server configurations
    #[builder(default)]
    pub mcp_servers: McpServers,

    /// Permission mode for tool execution
    #[builder(default, setter(strip_option))]
    pub permission_mode: Option<PermissionMode>,

    /// Whether to continue from the previous conversation
    #[builder(default)]
    pub continue_conversation: bool,

    /// Session ID to resume from
    #[builder(default, setter(strip_option, into))]
    pub resume: Option<super::identifiers::SessionId>,

    /// Maximum number of turns before stopping
    #[builder(default, setter(strip_option))]
    pub max_turns: Option<u32>,

    /// List of tools that Claude is not allowed to use
    #[builder(default, setter(into))]
    pub disallowed_tools: Vec<ToolName>,

    /// AI model to use
    #[builder(default, setter(strip_option, into))]
    pub model: Option<String>,

    /// Tool name to use for permission prompts
    #[builder(default, setter(strip_option, into))]
    pub permission_prompt_tool_name: Option<String>,

    /// Working directory for the CLI process
    #[builder(default, setter(strip_option, into))]
    pub cwd: Option<PathBuf>,

    /// Path to settings file
    #[builder(default, setter(strip_option, into))]
    pub settings: Option<PathBuf>,

    /// Additional directories to add to the context
    #[builder(default, setter(into))]
    pub add_dirs: Vec<PathBuf>,

    /// Environment variables for the CLI process
    #[builder(default)]
    pub env: HashMap<String, String>,

    /// Extra CLI arguments to pass
    #[builder(default)]
    pub extra_args: HashMap<String, Option<String>>,

    /// Maximum buffer size for JSON messages (default: 1MB)
    #[builder(default, setter(strip_option))]
    pub max_buffer_size: Option<usize>,

    /// Read timeout in seconds for waiting on CLI responses (default: 120s)
    /// Set higher for complex operations like subagent research.
    /// Set to 0 for no timeout.
    #[builder(default, setter(strip_option))]
    pub read_timeout_secs: Option<u64>,

    /// Callback for tool permission checks
    #[builder(default, setter(strip_option))]
    pub can_use_tool: Option<CanUseToolCallback>,

    /// Hook configurations
    #[builder(default, setter(strip_option))]
    pub hooks: Option<HashMap<HookEvent, Vec<HookMatcher>>>,

    /// User identifier
    #[builder(default, setter(strip_option, into))]
    pub user: Option<String>,

    /// Whether to include partial messages in stream
    #[builder(default)]
    pub include_partial_messages: bool,

    /// Whether to fork the session when resuming
    #[builder(default)]
    pub fork_session: bool,

    /// Custom session ID (must be valid UUID)
    /// Used with --resume or --continue and --fork-session
    #[builder(default, setter(strip_option, into))]
    pub session_id: Option<String>,

    /// Custom agent definitions
    #[builder(default, setter(strip_option))]
    pub agents: Option<HashMap<String, AgentDefinition>>,

    /// Setting sources to load
    #[builder(default, setter(strip_option))]
    pub setting_sources: Option<Vec<SettingSource>>,

    // ========================================================================
    // New options for TypeScript SDK parity
    // ========================================================================
    /// Maximum budget in USD for the query
    #[builder(default, setter(strip_option))]
    pub max_budget_usd: Option<f64>,

    /// Maximum tokens for thinking process
    #[builder(default, setter(strip_option))]
    pub max_thinking_tokens: Option<u32>,

    /// Model to use if primary fails
    #[builder(default, setter(strip_option, into))]
    pub fallback_model: Option<String>,

    /// Output format for structured outputs (JSON schema)
    #[builder(default, setter(strip_option))]
    pub output_format: Option<OutputFormat>,

    /// Sandbox configuration for command execution
    #[builder(default, setter(strip_option))]
    pub sandbox: Option<SandboxSettings>,

    /// Plugins to load from local paths
    #[builder(default, setter(strip_option))]
    pub plugins: Option<Vec<SdkPluginConfig>>,

    /// Beta features to enable
    #[builder(default, setter(strip_option))]
    pub betas: Option<Vec<SdkBeta>>,

    /// Enforce strict MCP configuration validation
    #[builder(default)]
    pub strict_mcp_config: bool,

    /// Enable file checkpointing for rewind support
    ///
    /// When enabled, tracks file changes made through Write/Edit/NotebookEdit tools,
    /// allowing programmatic rewind via `rewind_files()`. Requires `--replay-user-messages`
    /// to capture checkpoint UUIDs from user messages.
    #[builder(default)]
    pub enable_file_checkpointing: bool,

    /// Resume session at a specific message UUID
    #[builder(default, setter(strip_option, into))]
    pub resume_session_at: Option<String>,

    // ========================================================================
    // Additional TypeScript SDK parity options
    // ========================================================================
    /// Enable bypassing permissions (requires permissionMode: `BypassPermissions`)
    ///
    /// **WARNING**: This is dangerous and should only be used in controlled
    /// environments. When true, allows using `PermissionMode::BypassPermissions`.
    #[builder(default)]
    pub allow_dangerously_skip_permissions: bool,

    /// Path to custom Claude Code executable
    ///
    /// If not specified, uses the bundled or globally installed executable.
    #[builder(default, setter(strip_option, into))]
    pub path_to_claude_code_executable: Option<PathBuf>,

    /// Callback for stderr output
    ///
    /// Invoked when the Claude CLI writes to stderr. Useful for debugging.
    #[builder(default, setter(strip_option))]
    pub stderr: Option<StderrCallback>,

    /// Tools configuration
    ///
    /// Either a list of tool names or a preset (e.g., `ToolsConfig::claude_code_preset()`).
    #[builder(default, setter(strip_option))]
    pub tools: Option<ToolsConfig>,
}

impl ClaudeAgentOptions {
    /// Maximum allowed turns
    pub const MAX_ALLOWED_TURNS: u32 = 1000;
}

#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for ClaudeAgentOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeAgentOptions")
            .field("allowed_tools", &self.allowed_tools)
            .field("system_prompt", &self.system_prompt)
            .field("mcp_servers", &self.mcp_servers)
            .field("permission_mode", &self.permission_mode)
            .field("continue_conversation", &self.continue_conversation)
            .field("resume", &self.resume)
            .field("max_turns", &self.max_turns)
            .field("disallowed_tools", &self.disallowed_tools)
            .field("model", &self.model)
            .field(
                "permission_prompt_tool_name",
                &self.permission_prompt_tool_name,
            )
            .field("cwd", &self.cwd)
            .field("settings", &self.settings)
            .field("add_dirs", &self.add_dirs)
            .field("env", &self.env)
            .field("extra_args", &self.extra_args)
            .field("max_buffer_size", &self.max_buffer_size)
            .field(
                "can_use_tool",
                &self.can_use_tool.as_ref().map(|_| "<callback>"),
            )
            .field(
                "hooks",
                &self
                    .hooks
                    .as_ref()
                    .map(|h| format!("[{} hook types]", h.len())),
            )
            .field("user", &self.user)
            .field("include_partial_messages", &self.include_partial_messages)
            .field("fork_session", &self.fork_session)
            .field("session_id", &self.session_id)
            .field("agents", &self.agents)
            .field("setting_sources", &self.setting_sources)
            // New fields for TypeScript SDK parity
            .field("max_budget_usd", &self.max_budget_usd)
            .field("max_thinking_tokens", &self.max_thinking_tokens)
            .field("fallback_model", &self.fallback_model)
            .field("output_format", &self.output_format)
            .field("sandbox", &self.sandbox)
            .field(
                "plugins",
                &self
                    .plugins
                    .as_ref()
                    .map(|p| format!("[{} plugins]", p.len())),
            )
            .field("betas", &self.betas)
            .field("strict_mcp_config", &self.strict_mcp_config)
            .field("resume_session_at", &self.resume_session_at)
            // Additional TypeScript SDK parity options
            .field(
                "allow_dangerously_skip_permissions",
                &self.allow_dangerously_skip_permissions,
            )
            .field(
                "path_to_claude_code_executable",
                &self.path_to_claude_code_executable,
            )
            .field("stderr", &self.stderr.as_ref().map(|_| "<callback>"))
            .field(
                "tools",
                &self.tools.as_ref().map(|t| match t {
                    ToolsConfig::List(l) => format!("[{} tools]", l.len()),
                    ToolsConfig::Preset(p) => format!("preset:{}", p.preset),
                }),
            )
            .finish()
    }
}
