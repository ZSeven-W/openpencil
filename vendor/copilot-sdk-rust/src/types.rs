// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! Core types for the Copilot SDK.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::CopilotError;

fn is_false(value: &bool) -> bool {
    !*value
}

// =============================================================================
// Protocol Version
// =============================================================================

/// Maximum protocol version this SDK supports.
/// This must match the version expected by the copilot-agent-runtime server.
pub const SDK_PROTOCOL_VERSION: u32 = 3;

/// Minimum protocol version this SDK can communicate with.
/// Servers reporting a version below this are rejected.
pub const MIN_PROTOCOL_VERSION: u32 = 2;

// =============================================================================
// Enums
// =============================================================================

/// Connection state of the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error,
}

/// System message mode for session configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemMessageMode {
    Append,
    Replace,
}

/// Attachment type for user messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttachmentType {
    File,
    Directory,
    Selection,
}

/// Log level for the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    None,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
    All,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::None => write!(f, "none"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
            LogLevel::All => write!(f, "all"),
        }
    }
}

// =============================================================================
// Tool Types
// =============================================================================

/// Binary result from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolBinaryResult {
    pub data: String,
    pub mime_type: String,
    #[serde(rename = "type")]
    pub result_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Result object returned from tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResultObject {
    pub text_result_for_llm: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_results_for_llm: Option<Vec<ToolBinaryResult>>,
    #[serde(default = "default_result_type")]
    pub result_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_log: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_telemetry: Option<HashMap<String, serde_json::Value>>,
}

fn default_result_type() -> String {
    "success".to_string()
}

impl ToolResultObject {
    /// Create a success result with text.
    pub fn text(result: impl Into<String>) -> Self {
        Self {
            text_result_for_llm: result.into(),
            binary_results_for_llm: None,
            result_type: "success".to_string(),
            error: None,
            session_log: None,
            tool_telemetry: None,
        }
    }

    /// Create an error result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            text_result_for_llm: String::new(),
            binary_results_for_llm: None,
            result_type: "error".to_string(),
            error: Some(message.into()),
            session_log: None,
            tool_telemetry: None,
        }
    }
}

/// Convenient alias for tool results.
pub type ToolResult = ToolResultObject;

/// Information about a tool invocation from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInvocation {
    pub session_id: String,
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

impl ToolInvocation {
    /// Get an argument by name, deserializing to the specified type.
    pub fn arg<T: serde::de::DeserializeOwned>(&self, name: &str) -> crate::Result<T> {
        let args = self
            .arguments
            .as_ref()
            .ok_or_else(|| crate::CopilotError::ToolError("No arguments provided".into()))?;

        let value = args
            .get(name)
            .ok_or_else(|| crate::CopilotError::ToolError(format!("Missing argument: {}", name)))?;

        serde_json::from_value(value.clone()).map_err(|e| {
            crate::CopilotError::ToolError(format!("Invalid argument '{}': {}", name, e))
        })
    }
}

// =============================================================================
// Permission Types
// =============================================================================

/// Permission request from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(flatten)]
    pub extension_data: HashMap<String, serde_json::Value>,
}

/// Result of a permission request (response to CLI).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestResult {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<serde_json::Value>>,
}

impl PermissionRequestResult {
    /// Create an approved permission result.
    pub fn approved() -> Self {
        Self {
            kind: "approved".to_string(),
            rules: None,
        }
    }

    /// Create a denied permission result.
    pub fn denied() -> Self {
        Self {
            kind: "denied-no-approval-rule-and-could-not-request-from-user".to_string(),
            rules: None,
        }
    }

    /// Returns true if the permission was approved.
    pub fn is_approved(&self) -> bool {
        self.kind == "approved"
    }

    /// Returns true if the permission was denied.
    pub fn is_denied(&self) -> bool {
        self.kind.starts_with("denied")
    }
}

// =============================================================================
// Configuration Types
// =============================================================================

/// System message configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMessageConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<SystemMessageMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Azure-specific provider options.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_version: Option<String>,
}

/// Provider configuration for BYOK (Bring Your Own Key).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConfig {
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub provider_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wire_api: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub azure: Option<AzureOptions>,
}

// Environment variable names for BYOK configuration
impl ProviderConfig {
    /// Environment variable for API key
    pub const ENV_API_KEY: &'static str = "COPILOT_SDK_BYOK_API_KEY";
    /// Environment variable for base URL
    pub const ENV_BASE_URL: &'static str = "COPILOT_SDK_BYOK_BASE_URL";
    /// Environment variable for provider type
    pub const ENV_PROVIDER_TYPE: &'static str = "COPILOT_SDK_BYOK_PROVIDER_TYPE";
    /// Environment variable for model
    pub const ENV_MODEL: &'static str = "COPILOT_SDK_BYOK_MODEL";

    /// Check if BYOK environment variables are configured.
    ///
    /// Returns true if `COPILOT_SDK_BYOK_API_KEY` is set and non-empty.
    pub fn is_env_configured() -> bool {
        std::env::var(Self::ENV_API_KEY)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
    }

    /// Load ProviderConfig from `COPILOT_SDK_BYOK_*` environment variables.
    ///
    /// Returns `Some(ProviderConfig)` if API key is set, `None` otherwise.
    ///
    /// Environment variables:
    /// - `COPILOT_SDK_BYOK_API_KEY` (required): API key for the provider
    /// - `COPILOT_SDK_BYOK_BASE_URL` (optional): Base URL (defaults to OpenAI)
    /// - `COPILOT_SDK_BYOK_PROVIDER_TYPE` (optional): Provider type (defaults to "openai")
    pub fn from_env() -> Option<Self> {
        if !Self::is_env_configured() {
            return None;
        }

        let api_key = std::env::var(Self::ENV_API_KEY).ok();
        let base_url = std::env::var(Self::ENV_BASE_URL)
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let provider_type = std::env::var(Self::ENV_PROVIDER_TYPE)
            .ok()
            .or_else(|| Some("openai".to_string()));

        Some(Self {
            base_url,
            provider_type,
            api_key,
            wire_api: None,
            bearer_token: None,
            azure: None,
        })
    }

    /// Load model from `COPILOT_SDK_BYOK_MODEL` environment variable.
    ///
    /// Returns `Some(model)` if set and non-empty, `None` otherwise.
    pub fn model_from_env() -> Option<String> {
        std::env::var(Self::ENV_MODEL)
            .ok()
            .filter(|v| !v.is_empty())
    }
}

// =============================================================================
// MCP Server Configuration
// =============================================================================

/// Configuration for a local/stdio MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpLocalServerConfig {
    pub tools: Vec<String>,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub server_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// Configuration for a remote MCP server (HTTP or SSE).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRemoteServerConfig {
    pub tools: Vec<String>,
    pub url: String,
    #[serde(default = "default_mcp_type", rename = "type")]
    pub server_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

fn default_mcp_type() -> String {
    "http".to_string()
}

/// MCP server configuration (either local or remote).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpServerConfig {
    Local(McpLocalServerConfig),
    Remote(McpRemoteServerConfig),
}

// =============================================================================
// Custom Agent Configuration
// =============================================================================

/// Configuration for a custom agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomAgentConfig {
    pub name: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infer: Option<bool>,
}

// =============================================================================
// Attachment Types
// =============================================================================

/// Attachment item for user messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageAttachment {
    #[serde(rename = "type")]
    pub attachment_type: AttachmentType,
    pub path: String,
    pub display_name: String,
}

// =============================================================================
// Tool Definition (SDK-side)
// =============================================================================

/// Tool definition for registration with a session.
///
/// Use the builder pattern to create tools:
/// ```no_run
/// use copilot_sdk::{Client, SessionConfig, Tool, ToolHandler, ToolResultObject};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() -> copilot_sdk::Result<()> {
/// let client = Client::builder().build()?;
/// client.start().await?;
///
/// let tool = Tool::new("get_weather")
///     .description("Get weather for a city")
///     .schema(serde_json::json!({
///         "type": "object",
///         "properties": { "city": { "type": "string" } },
///         "required": ["city"]
///     }));
///
/// let session = client.create_session(SessionConfig {
///     tools: vec![tool.clone()],
///     ..Default::default()
/// }).await?;
///
/// let handler: ToolHandler = Arc::new(|_name, args| {
///     let city = args.get("city").and_then(|v| v.as_str()).unwrap_or("unknown");
///     ToolResultObject::text(format!("Weather in {}: sunny", city))
/// });
/// session.register_tool_with_handler(tool, Some(handler)).await;
/// client.stop().await;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
    pub overrides_built_in_tool: bool,
    pub skip_permission: bool,
    // Handler is stored separately in Session since it's not Clone-friendly
}

impl Tool {
    /// Create a new tool with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            parameters_schema: serde_json::json!({}),
            overrides_built_in_tool: false,
            skip_permission: false,
        }
    }

    /// Set the tool description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Set the parameters JSON schema.
    pub fn schema(mut self, schema: serde_json::Value) -> Self {
        self.parameters_schema = schema;
        self
    }

    /// Add a parameter to the tool's JSON schema.
    ///
    /// Builds the schema incrementally using the builder pattern.
    pub fn parameter(
        mut self,
        name: impl Into<String>,
        param_type: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let name = name.into();

        // Ensure schema has the right shape
        if self.parameters_schema.get("type").is_none() {
            self.parameters_schema["type"] = serde_json::json!("object");
        }
        if self.parameters_schema.get("properties").is_none() {
            self.parameters_schema["properties"] = serde_json::json!({});
        }

        self.parameters_schema["properties"][&name] = serde_json::json!({
            "type": param_type.into(),
            "description": description.into(),
        });

        if required {
            if self.parameters_schema.get("required").is_none() {
                self.parameters_schema["required"] = serde_json::json!([]);
            }
            if let Some(arr) = self.parameters_schema["required"].as_array_mut() {
                arr.push(serde_json::json!(name));
            }
        }

        self
    }

    /// Derive the parameters JSON schema from a Rust type (requires the `schemars` feature).
    #[cfg(feature = "schemars")]
    pub fn typed_schema<T: schemars::JsonSchema>(mut self) -> Self {
        let schema = schemars::schema_for!(T);
        match serde_json::to_value(&schema) {
            Ok(value) => self.parameters_schema = value,
            Err(err) => {
                tracing::warn!("Failed to serialize schemars schema: {err}");
                self.parameters_schema = serde_json::json!({});
            }
        }
        self
    }

    /// Mark this tool as overriding a built-in tool.
    pub fn overrides_built_in_tool(mut self, value: bool) -> Self {
        self.overrides_built_in_tool = value;
        self
    }

    /// Skip permission checks for this tool.
    pub fn skip_permission(mut self, value: bool) -> Self {
        self.skip_permission = value;
        self
    }
}

impl std::fmt::Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("overrides_built_in_tool", &self.overrides_built_in_tool)
            .field("skip_permission", &self.skip_permission)
            .finish()
    }
}

// Serialization for sending tool definitions to the CLI
impl Serialize for Tool {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let field_count = 3
            + if self.overrides_built_in_tool { 1 } else { 0 }
            + if self.skip_permission { 1 } else { 0 };
        let mut state = serializer.serialize_struct("Tool", field_count)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("parameters", &self.parameters_schema)?;
        if self.overrides_built_in_tool {
            state.serialize_field("overridesBuiltInTool", &self.overrides_built_in_tool)?;
        }
        if self.skip_permission {
            state.serialize_field("skipPermission", &self.skip_permission)?;
        }
        state.end()
    }
}

// =============================================================================
// Infinite Session Configuration
// =============================================================================

/// Configuration for infinite sessions (automatic context compaction).
///
/// When enabled, the SDK will automatically manage conversation context to prevent
/// buffer exhaustion. Thresholds are expressed as fractions (0.0 to 1.0).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfiniteSessionConfig {
    /// Enable infinite sessions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    /// Threshold for background compaction (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_compaction_threshold: Option<f64>,
    /// Threshold for buffer exhaustion handling (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffer_exhaustion_threshold: Option<f64>,
}

impl InfiniteSessionConfig {
    /// Create an enabled infinite session config with default thresholds.
    pub fn enabled() -> Self {
        Self {
            enabled: Some(true),
            background_compaction_threshold: None,
            buffer_exhaustion_threshold: None,
        }
    }

    /// Create an infinite session config with custom thresholds.
    pub fn with_thresholds(background: f64, exhaustion: f64) -> Self {
        Self {
            enabled: Some(true),
            background_compaction_threshold: Some(background),
            buffer_exhaustion_threshold: Some(exhaustion),
        }
    }
}

// =============================================================================
// Session Hooks
// =============================================================================

/// Input for the pre-tool-use hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseHookInput {
    pub timestamp: i64,
    pub cwd: String,
    pub tool_name: String,
    pub tool_args: serde_json::Value,
}

/// Output for the pre-tool-use hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreToolUseHookOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_decision_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_args: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
}

/// Input for the post-tool-use hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostToolUseHookInput {
    pub timestamp: i64,
    pub cwd: String,
    pub tool_name: String,
    pub tool_args: serde_json::Value,
    pub tool_result: serde_json::Value,
}

/// Output for the post-tool-use hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostToolUseHookOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
}

/// Input for the user-prompt-submitted hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPromptSubmittedHookInput {
    pub timestamp: i64,
    pub cwd: String,
    pub prompt: String,
}

/// Output for the user-prompt-submitted hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPromptSubmittedHookOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
}

/// Input for the session-start hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartHookInput {
    pub timestamp: i64,
    pub cwd: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_prompt: Option<String>,
}

/// Output for the session-start hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartHookOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_config: Option<serde_json::Value>,
}

/// Input for the session-end hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEndHookInput {
    pub timestamp: i64,
    pub cwd: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Output for the session-end hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEndHookOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cleanup_actions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_summary: Option<String>,
}

/// Input for the error-occurred hook.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorOccurredHookInput {
    pub timestamp: i64,
    pub cwd: String,
    pub error: String,
    pub error_context: String,
    pub recoverable: bool,
}

/// Output for the error-occurred hook.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorOccurredHookOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suppress_output: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_handling: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_notification: Option<String>,
}

/// Handler types for session hooks.
pub type PreToolUseHandler = Arc<dyn Fn(PreToolUseHookInput) -> PreToolUseHookOutput + Send + Sync>;
pub type PostToolUseHandler =
    Arc<dyn Fn(PostToolUseHookInput) -> PostToolUseHookOutput + Send + Sync>;
pub type UserPromptSubmittedHandler =
    Arc<dyn Fn(UserPromptSubmittedHookInput) -> UserPromptSubmittedHookOutput + Send + Sync>;
pub type SessionStartHandler =
    Arc<dyn Fn(SessionStartHookInput) -> SessionStartHookOutput + Send + Sync>;
pub type SessionEndHandler = Arc<dyn Fn(SessionEndHookInput) -> SessionEndHookOutput + Send + Sync>;
pub type ErrorOccurredHandler =
    Arc<dyn Fn(ErrorOccurredHookInput) -> ErrorOccurredHookOutput + Send + Sync>;

/// Configuration for session hooks.
///
/// Hooks allow intercepting and modifying behavior at key points in the session lifecycle.
#[derive(Clone, Default)]
pub struct SessionHooks {
    pub on_pre_tool_use: Option<PreToolUseHandler>,
    pub on_post_tool_use: Option<PostToolUseHandler>,
    pub on_user_prompt_submitted: Option<UserPromptSubmittedHandler>,
    pub on_session_start: Option<SessionStartHandler>,
    pub on_session_end: Option<SessionEndHandler>,
    pub on_error_occurred: Option<ErrorOccurredHandler>,
}

impl SessionHooks {
    /// Returns true if any hook handler is registered.
    pub fn has_any(&self) -> bool {
        self.on_pre_tool_use.is_some()
            || self.on_post_tool_use.is_some()
            || self.on_user_prompt_submitted.is_some()
            || self.on_session_start.is_some()
            || self.on_session_end.is_some()
            || self.on_error_occurred.is_some()
    }
}

impl std::fmt::Debug for SessionHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionHooks")
            .field("on_pre_tool_use", &self.on_pre_tool_use.is_some())
            .field("on_post_tool_use", &self.on_post_tool_use.is_some())
            .field(
                "on_user_prompt_submitted",
                &self.on_user_prompt_submitted.is_some(),
            )
            .field("on_session_start", &self.on_session_start.is_some())
            .field("on_session_end", &self.on_session_end.is_some())
            .field("on_error_occurred", &self.on_error_occurred.is_some())
            .finish()
    }
}

// =============================================================================
// Session Configuration
// =============================================================================

/// Configuration for creating a new session.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_dir: Option<PathBuf>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_message: Option<SystemMessageConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_tools: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderConfig>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub streaming: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_agents: Option<Vec<CustomAgentConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_directories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestPermission")]
    pub request_permission: Option<bool>,
    /// Infinite session configuration for automatic context compaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infinite_sessions: Option<InfiniteSessionConfig>,

    /// Whether to request user input forwarding from the server.
    /// When true, `userInput.request` callbacks will be sent to the SDK.
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestUserInput")]
    pub request_user_input: Option<bool>,

    /// Reasoning effort level: "low", "medium", "high", or "xhigh".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    /// Working directory for the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,

    /// Client name identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,

    /// Agent to use for the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// Session hooks for pre/post tool use, session lifecycle, etc.
    #[serde(skip)]
    pub hooks: Option<SessionHooks>,

    /// If true and provider/model not explicitly set, load from `COPILOT_SDK_BYOK_*` env vars.
    ///
    /// Default: false (explicit configuration preferred over environment variables)
    #[serde(skip)]
    pub auto_byok_from_env: bool,
}

/// Configuration for resuming an existing session.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResumeSessionConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<ProviderConfig>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub streaming: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_agents: Option<Vec<CustomAgentConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_directories: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disabled_skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestPermission")]
    pub request_permission: Option<bool>,

    /// Whether to request user input forwarding from the server.
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestUserInput")]
    pub request_user_input: Option<bool>,

    /// Reasoning effort level: "low", "medium", "high", or "xhigh".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    /// Working directory for the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_directory: Option<String>,

    /// Client name identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,

    /// Agent to use for the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,

    /// If true, skip resuming and create a new session instead.
    #[serde(default, skip_serializing_if = "is_false")]
    pub disable_resume: bool,

    /// Infinite session configuration for resumed sessions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infinite_sessions: Option<InfiniteSessionConfig>,

    /// Session hooks for pre/post tool use, session lifecycle, etc.
    #[serde(skip)]
    pub hooks: Option<SessionHooks>,

    /// If true and provider not explicitly set, load from `COPILOT_SDK_BYOK_*` env vars.
    ///
    /// Default: false (explicit configuration preferred over environment variables)
    #[serde(skip)]
    pub auto_byok_from_env: bool,
}

/// Options for sending a message.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageOptions {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<UserMessageAttachment>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

impl From<&str> for MessageOptions {
    fn from(prompt: &str) -> Self {
        Self {
            prompt: prompt.to_string(),
            attachments: None,
            mode: None,
        }
    }
}

impl From<String> for MessageOptions {
    fn from(prompt: String) -> Self {
        Self {
            prompt,
            attachments: None,
            mode: None,
        }
    }
}

// =============================================================================
// Client Options
// =============================================================================

/// Options for creating a CopilotClient.
pub struct ClientOptions {
    pub cli_path: Option<PathBuf>,
    pub cli_args: Option<Vec<String>>,
    pub cwd: Option<PathBuf>,
    pub port: u16,
    pub use_stdio: bool,
    pub cli_url: Option<String>,
    pub log_level: LogLevel,
    pub auto_start: bool,
    pub auto_restart: bool,
    pub environment: Option<HashMap<String, String>>,
    /// GitHub personal access token for authentication.
    /// Cannot be used together with `cli_url`.
    pub github_token: Option<String>,
    /// Whether to use the logged-in user for auth.
    /// Defaults to true when github_token is empty. Cannot be used with `cli_url`.
    pub use_logged_in_user: Option<bool>,

    /// Tool specifications to deny (passed as `--deny-tool` arguments to the CLI).
    ///
    /// Each entry follows the CLI's tool specification format:
    /// - `"shell(git push)"` — deny a specific shell command
    /// - `"shell(git)"` — deny all git commands
    /// - `"shell(rm)"` — deny rm commands
    /// - `"shell"` — deny all shell commands
    /// - `"write"` — deny file write operations
    /// - `"MCP_SERVER(tool_name)"` — deny a specific MCP tool
    ///
    /// `--deny-tool` takes precedence over `--allow-tool` and `--allow-all-tools`.
    pub deny_tools: Option<Vec<String>>,

    /// Tool specifications to allow without manual approval
    /// (passed as `--allow-tool` arguments to the CLI).
    ///
    /// Each entry follows the same format as `deny_tools`.
    pub allow_tools: Option<Vec<String>>,

    /// If true, passes `--allow-all-tools` to the CLI.
    ///
    /// This allows Copilot to use any tool without asking for approval.
    /// Use `deny_tools` in combination to create an allowlist with exceptions.
    pub allow_all_tools: bool,

    /// OpenTelemetry configuration for tracing.
    pub telemetry: Option<TelemetryConfig>,

    /// Callback for custom model listing (BYOK).
    #[allow(clippy::type_complexity)]
    pub on_list_models: Option<
        Arc<
            dyn Fn() -> std::pin::Pin<
                    Box<
                        dyn std::future::Future<Output = Result<Vec<ModelInfo>, CopilotError>>
                            + Send,
                    >,
                > + Send
                + Sync,
        >,
    >,
}

impl Default for ClientOptions {
    fn default() -> Self {
        Self {
            cli_path: None,
            cli_args: None,
            cwd: None,
            port: 0,
            use_stdio: true,
            cli_url: None,
            log_level: LogLevel::Info,
            auto_start: true,
            auto_restart: true,
            environment: None,
            github_token: None,
            use_logged_in_user: None,
            deny_tools: None,
            allow_tools: None,
            allow_all_tools: false,
            telemetry: None,
            on_list_models: None,
        }
    }
}

impl std::fmt::Debug for ClientOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientOptions")
            .field("cli_path", &self.cli_path)
            .field("cli_args", &self.cli_args)
            .field("cwd", &self.cwd)
            .field("port", &self.port)
            .field("use_stdio", &self.use_stdio)
            .field("cli_url", &self.cli_url)
            .field("log_level", &self.log_level)
            .field("auto_start", &self.auto_start)
            .field("auto_restart", &self.auto_restart)
            .field("environment", &self.environment)
            .field("github_token", &self.github_token)
            .field("use_logged_in_user", &self.use_logged_in_user)
            .field("deny_tools", &self.deny_tools)
            .field("allow_tools", &self.allow_tools)
            .field("allow_all_tools", &self.allow_all_tools)
            .field("telemetry", &self.telemetry)
            .field(
                "on_list_models",
                &self.on_list_models.as_ref().map(|_| "Fn(...)"),
            )
            .finish()
    }
}

impl Clone for ClientOptions {
    fn clone(&self) -> Self {
        Self {
            cli_path: self.cli_path.clone(),
            cli_args: self.cli_args.clone(),
            cwd: self.cwd.clone(),
            port: self.port,
            use_stdio: self.use_stdio,
            cli_url: self.cli_url.clone(),
            log_level: self.log_level,
            auto_start: self.auto_start,
            auto_restart: self.auto_restart,
            environment: self.environment.clone(),
            github_token: self.github_token.clone(),
            use_logged_in_user: self.use_logged_in_user,
            deny_tools: self.deny_tools.clone(),
            allow_tools: self.allow_tools.clone(),
            allow_all_tools: self.allow_all_tools,
            telemetry: self.telemetry.clone(),
            on_list_models: self.on_list_models.clone(),
        }
    }
}

// =============================================================================
// Response Types
// =============================================================================

/// Metadata about a session.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMetadata {
    pub session_id: String,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub modified_time: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub is_remote: bool,
}

/// Response from a ping request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {
    pub message: String,
    pub timestamp: i64,
    #[serde(default)]
    pub protocol_version: Option<u32>,
}

/// Response from status.get request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetStatusResponse {
    pub version: String,
    pub protocol_version: u32,
}

/// Response from auth.getStatus request.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAuthStatusResponse {
    pub is_authenticated: bool,
    #[serde(default)]
    pub auth_type: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
    #[serde(default)]
    pub login: Option<String>,
    #[serde(default)]
    pub status_message: Option<String>,
}

/// Model capabilities - what the model supports.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCapabilities {
    #[serde(default)]
    pub supports: ModelSupports,
    #[serde(default)]
    pub limits: ModelLimits,
}

/// What features a model supports.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSupports {
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub reasoning_effort: bool,
}

/// Vision limits for a model.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelVisionLimits {
    #[serde(default)]
    pub supported_media_types: Vec<String>,
    #[serde(default)]
    pub max_prompt_images: u32,
    #[serde(default)]
    pub max_prompt_image_size: u64,
}

/// Model limits.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelLimits {
    #[serde(default)]
    pub max_prompt_tokens: Option<u32>,
    #[serde(default)]
    pub max_context_window_tokens: u32,
    #[serde(default)]
    pub vision: Option<ModelVisionLimits>,
}

/// Model policy state.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelPolicy {
    pub state: String,
    #[serde(default)]
    pub terms: String,
}

/// Model billing information.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelBilling {
    #[serde(default)]
    pub multiplier: f64,
}

/// Information about an available model.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub capabilities: ModelCapabilities,
    #[serde(default)]
    pub policy: Option<ModelPolicy>,
    #[serde(default)]
    pub billing: Option<ModelBilling>,
    #[serde(default)]
    pub supported_reasoning_efforts: Option<Vec<String>>,
    #[serde(default)]
    pub default_reasoning_effort: Option<String>,
}

// =============================================================================
// Selection Attachment Types
// =============================================================================

/// Position in a text document (line + character).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelectionPosition {
    #[serde(default)]
    pub line: f64,
    #[serde(default)]
    pub character: f64,
}

/// Range within a text document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SelectionRange {
    pub start: SelectionPosition,
    pub end: SelectionPosition,
}

/// Attachment representing a text selection in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectionAttachment {
    pub file_path: String,
    pub display_name: String,
    pub text: String,
    pub selection: SelectionRange,
}

// =============================================================================
// User Input Types
// =============================================================================

/// Request for user input from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInputRequest {
    pub question: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_freeform: Option<bool>,
}

/// Response to a user input request.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInputResponse {
    #[serde(default)]
    pub answer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub was_freeform: Option<bool>,
}

/// Context for a user input invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInputInvocation {
    pub session_id: String,
}

// =============================================================================
// Session Lifecycle Types
// =============================================================================

/// Session lifecycle event type constants.
pub mod session_lifecycle_event_types {
    pub const CREATED: &str = "session.created";
    pub const DELETED: &str = "session.deleted";
    pub const UPDATED: &str = "session.updated";
    pub const FOREGROUND: &str = "session.foreground";
    pub const BACKGROUND: &str = "session.background";
}

/// Metadata for session lifecycle events.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLifecycleEventMetadata {
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub modified_time: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
}

/// Session lifecycle event notification.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLifecycleEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub session_id: String,
    #[serde(default)]
    pub metadata: Option<SessionLifecycleEventMetadata>,
}

/// Response from session.getForeground.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetForegroundSessionResponse {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub workspace_path: Option<String>,
}

/// Response from session.setForeground.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetForegroundSessionResponse {
    #[serde(default)]
    pub success: bool,
    #[serde(default)]
    pub error: Option<String>,
}

// =============================================================================
// Stop Error
// =============================================================================

/// Error collected during client stop.
#[derive(Debug, Clone)]
pub struct StopError {
    pub message: String,
    pub source: Option<String>,
}

impl std::fmt::Display for StopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

// =============================================================================
// Session Mode
// =============================================================================

/// Session operation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionMode {
    Interactive,
    Plan,
    Autopilot,
}

// =============================================================================
// Set Model Options
// =============================================================================

/// Options for switching models mid-session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetModelOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

// =============================================================================
// Session Log Types
// =============================================================================

/// Log level for session log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionLogLevel {
    Error,
    Info,
    Warning,
}

/// Options for adding a log entry to a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<SessionLogLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<bool>,
}

/// Result from adding a session log entry.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogResult {
    pub event_id: String,
}

// =============================================================================
// Plan Data
// =============================================================================

/// Plan content data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

// =============================================================================
// Agent Info
// =============================================================================

/// Information about an available agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// =============================================================================
// Fleet Start Options
// =============================================================================

/// Options for starting a fleet of parallel agents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetStartOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
}

// =============================================================================
// Tools List Types
// =============================================================================

/// A tool definition as returned by the server.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: Option<serde_json::Value>,
}

/// Result from listing available tools.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsListResult {
    pub tools: Vec<ToolInfo>,
}

// =============================================================================
// Quota Types
// =============================================================================

/// A snapshot of quota usage.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSnapshot {
    #[serde(rename = "type")]
    pub quota_type: String,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub used: Option<u64>,
    #[serde(default)]
    pub remaining: Option<u64>,
    #[serde(default)]
    pub resets_at: Option<String>,
}

/// Result from getting account quota.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaResult {
    pub quotas: Vec<QuotaSnapshot>,
}

// =============================================================================
// Shell Exec Types
// =============================================================================

/// Signal to send to a shell process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellSignal {
    SIGINT,
    SIGKILL,
    SIGTERM,
}

/// Options for executing a shell command.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellExecOptions {
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
}

/// Result from executing a shell command.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellExecResult {
    pub process_id: String,
}

// =============================================================================
// Workspace File Types
// =============================================================================

/// Metadata about a workspace file.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceFile {
    pub path: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub modified_at: Option<String>,
}

// =============================================================================
// Telemetry Config
// =============================================================================

/// OpenTelemetry configuration for the Copilot CLI process.
#[derive(Debug, Clone, Default)]
pub struct TelemetryConfig {
    /// OTLP HTTP endpoint URL.
    pub otlp_endpoint: Option<String>,
    /// File path for JSON-lines trace output.
    pub file_path: Option<String>,
    /// Exporter type: "otlp-http" or "file".
    pub exporter_type: Option<String>,
    /// Instrumentation scope name.
    pub source_name: Option<String>,
    /// Whether to capture message content (prompts/responses).
    pub capture_content: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result_text() {
        let result = ToolResult::text("Hello, world!");
        assert_eq!(result.text_result_for_llm, "Hello, world!");
        assert_eq!(result.result_type, "success");
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult::error("Something went wrong");
        assert_eq!(result.result_type, "error");
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_permission_result() {
        let approved = PermissionRequestResult::approved();
        assert_eq!(approved.kind, "approved");
        assert!(approved.is_approved());
        assert!(!approved.is_denied());

        let denied = PermissionRequestResult::denied();
        assert!(denied.kind.starts_with("denied"));
        assert!(denied.is_denied());
        assert!(!denied.is_approved());
    }

    #[test]
    fn test_message_options_from_str() {
        let opts: MessageOptions = "Hello".into();
        assert_eq!(opts.prompt, "Hello");
    }

    #[test]
    fn test_session_config_default() {
        let config = SessionConfig::default();
        assert!(config.model.is_none());
        assert!(config.tools.is_empty());
    }

    #[test]
    fn test_session_config_serialization_with_new_fields() {
        let config = SessionConfig {
            session_id: Some("sess-1".into()),
            model: Some("gpt-4.1".into()),
            config_dir: Some(PathBuf::from("/tmp/copilot")),
            streaming: true,
            skill_directories: Some(vec!["skills".into()]),
            disabled_skills: Some(vec!["legacy_skill".into()]),
            request_permission: Some(true),
            ..Default::default()
        };

        let value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["sessionId"], "sess-1");
        assert_eq!(value["model"], "gpt-4.1");
        assert_eq!(value["configDir"], "/tmp/copilot");
        assert_eq!(value["streaming"], true);
        assert_eq!(value["skillDirectories"][0], "skills");
        assert_eq!(value["disabledSkills"][0], "legacy_skill");
        assert_eq!(value["requestPermission"], true);
    }

    #[test]
    fn test_tool_builder() {
        let tool = Tool::new("my_tool")
            .description("A test tool")
            .schema(serde_json::json!({"type": "object"}));

        assert_eq!(tool.name, "my_tool");
        assert_eq!(tool.description, "A test tool");
    }

    #[test]
    fn test_tool_serializes_parameters_field() {
        let tool = Tool::new("my_tool")
            .description("A test tool")
            .schema(serde_json::json!({"type": "object"}));

        let value = serde_json::to_value(&tool).unwrap();

        assert_eq!(value["parameters"]["type"], "object");
        assert!(value.get("parametersSchema").is_none());
    }

    #[test]
    fn test_user_input_request_roundtrip() {
        let req = UserInputRequest {
            question: "What color?".into(),
            choices: Some(vec!["red".into(), "blue".into()]),
            allow_freeform: Some(true),
        };
        let j = serde_json::to_value(&req).unwrap();
        assert_eq!(j["question"], "What color?");
        assert_eq!(j["choices"][0], "red");
        assert_eq!(j["allowFreeform"], true);

        let req2: UserInputRequest = serde_json::from_value(j).unwrap();
        assert_eq!(req2.question, "What color?");
    }

    #[test]
    fn test_user_input_response_roundtrip() {
        let resp = UserInputResponse {
            answer: "blue".into(),
            was_freeform: Some(true),
        };
        let j = serde_json::to_value(&resp).unwrap();
        assert_eq!(j["answer"], "blue");

        let resp2: UserInputResponse = serde_json::from_value(j).unwrap();
        assert_eq!(resp2.answer, "blue");
        assert_eq!(resp2.was_freeform, Some(true));
    }

    #[test]
    fn test_user_input_request_minimal() {
        let j = serde_json::json!({"question": "Yes or no?"});
        let req: UserInputRequest = serde_json::from_value(j).unwrap();
        assert_eq!(req.question, "Yes or no?");
        assert!(req.choices.is_none());
        assert!(req.allow_freeform.is_none());
    }

    #[test]
    fn test_session_lifecycle_event_from_json() {
        let j = serde_json::json!({
            "type": "session.created",
            "sessionId": "sess_123",
            "metadata": {
                "startTime": "2024-01-15T10:30:00Z",
                "modifiedTime": "2024-01-15T10:30:00Z",
                "summary": "Test session"
            }
        });
        let event: SessionLifecycleEvent = serde_json::from_value(j).unwrap();
        assert_eq!(event.event_type, session_lifecycle_event_types::CREATED);
        assert_eq!(event.session_id, "sess_123");
        assert_eq!(
            event.metadata.as_ref().unwrap().summary,
            Some("Test session".into())
        );
    }

    #[test]
    fn test_get_foreground_session_response() {
        let j = serde_json::json!({"sessionId": "sess_123", "workspacePath": "/tmp"});
        let resp: GetForegroundSessionResponse = serde_json::from_value(j).unwrap();
        assert_eq!(resp.session_id, Some("sess_123".into()));
        assert_eq!(resp.workspace_path, Some("/tmp".into()));
    }

    #[test]
    fn test_set_foreground_session_response() {
        let j = serde_json::json!({"success": true});
        let resp: SetForegroundSessionResponse = serde_json::from_value(j).unwrap();
        assert!(resp.success);
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_set_foreground_session_response_error() {
        let j = serde_json::json!({"success": false, "error": "not found"});
        let resp: SetForegroundSessionResponse = serde_json::from_value(j).unwrap();
        assert!(!resp.success);
        assert_eq!(resp.error, Some("not found".into()));
    }

    #[test]
    fn test_selection_attachment_roundtrip() {
        let att = SelectionAttachment {
            file_path: "src/main.rs".into(),
            display_name: "main.rs".into(),
            text: "fn main()".into(),
            selection: SelectionRange {
                start: SelectionPosition {
                    line: 1.0,
                    character: 0.0,
                },
                end: SelectionPosition {
                    line: 1.0,
                    character: 9.0,
                },
            },
        };
        let j = serde_json::to_value(&att).unwrap();
        assert_eq!(j["filePath"], "src/main.rs");
        assert_eq!(j["selection"]["start"]["line"], 1.0);
    }

    #[test]
    fn test_attachment_type_selection() {
        let j = serde_json::json!("selection");
        let at: AttachmentType = serde_json::from_value(j).unwrap();
        assert_eq!(at, AttachmentType::Selection);
    }

    #[test]
    fn test_stop_error_display() {
        let err = StopError {
            message: "timeout".into(),
            source: Some("rpc".into()),
        };
        assert_eq!(format!("{err}"), "timeout");
    }

    #[test]
    fn test_session_config_reasoning_effort() {
        let config = SessionConfig {
            reasoning_effort: Some("high".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["reasoningEffort"], "high");
    }

    #[test]
    fn test_session_config_working_directory() {
        let config = SessionConfig {
            working_directory: Some("/home/user/project".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["workingDirectory"], "/home/user/project");
    }

    #[test]
    fn test_resume_config_disable_resume() {
        let config = ResumeSessionConfig {
            disable_resume: true,
            ..Default::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["disableResume"], true);
    }

    #[test]
    fn test_resume_config_model() {
        let config = ResumeSessionConfig {
            model: Some("gpt-4".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        assert_eq!(json["model"], "gpt-4");
    }

    #[test]
    fn test_session_hooks_has_any() {
        let hooks = SessionHooks::default();
        assert!(!hooks.has_any());

        let hooks = SessionHooks {
            on_pre_tool_use: Some(Arc::new(|_| PreToolUseHookOutput::default())),
            ..Default::default()
        };
        assert!(hooks.has_any());
    }

    #[test]
    fn test_session_hooks_debug() {
        let hooks = SessionHooks {
            on_pre_tool_use: Some(Arc::new(|_| PreToolUseHookOutput::default())),
            ..Default::default()
        };
        let debug = format!("{:?}", hooks);
        assert!(debug.contains("on_pre_tool_use: true"));
        assert!(debug.contains("on_post_tool_use: false"));
    }

    #[test]
    fn test_pre_tool_use_hook_input_serde() {
        let json = serde_json::json!({
            "timestamp": 1234567890,
            "cwd": "/tmp",
            "toolName": "my_tool",
            "toolArgs": {"key": "value"}
        });
        let input: PreToolUseHookInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.timestamp, 1234567890);
        assert_eq!(input.tool_name, "my_tool");
    }

    #[test]
    fn test_pre_tool_use_hook_output_serde() {
        let output = PreToolUseHookOutput {
            permission_decision: Some("allow".into()),
            additional_context: Some("context".into()),
            ..Default::default()
        };
        let json = serde_json::to_value(&output).unwrap();
        assert_eq!(json["permissionDecision"], "allow");
        assert_eq!(json["additionalContext"], "context");
        assert!(json.get("suppressOutput").is_none());
    }

    #[test]
    fn test_session_end_hook_input_serde() {
        let json = serde_json::json!({
            "timestamp": 1234567890,
            "cwd": "/tmp",
            "reason": "complete",
            "finalMessage": "Done"
        });
        let input: SessionEndHookInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.reason, "complete");
        assert_eq!(input.final_message, Some("Done".into()));
    }

    #[test]
    fn test_error_occurred_hook_input_serde() {
        let json = serde_json::json!({
            "timestamp": 1234567890,
            "cwd": "/tmp",
            "error": "connection failed",
            "errorContext": "model_call",
            "recoverable": true
        });
        let input: ErrorOccurredHookInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.error_context, "model_call");
        assert!(input.recoverable);
    }

    #[test]
    fn test_hooks_not_serialized_in_config() {
        let config = SessionConfig {
            hooks: Some(SessionHooks {
                on_pre_tool_use: Some(Arc::new(|_| PreToolUseHookOutput::default())),
                ..Default::default()
            }),
            ..Default::default()
        };
        let json = serde_json::to_value(&config).unwrap();
        // hooks field should be skipped from serialization
        assert!(json.get("hooks").is_none());
    }

    #[test]
    fn test_session_mode_serialization() {
        assert_eq!(
            serde_json::to_string(&SessionMode::Interactive).unwrap(),
            "\"interactive\""
        );
        assert_eq!(
            serde_json::to_string(&SessionMode::Plan).unwrap(),
            "\"plan\""
        );
        assert_eq!(
            serde_json::to_string(&SessionMode::Autopilot).unwrap(),
            "\"autopilot\""
        );
    }

    #[test]
    fn test_shell_signal_serialization() {
        assert_eq!(
            serde_json::to_string(&ShellSignal::SIGINT).unwrap(),
            "\"SIGINT\""
        );
        assert_eq!(
            serde_json::to_string(&ShellSignal::SIGKILL).unwrap(),
            "\"SIGKILL\""
        );
    }

    #[test]
    fn test_tool_overrides_fields() {
        let tool = Tool::new("test")
            .overrides_built_in_tool(true)
            .skip_permission(true);
        let value = serde_json::to_value(&tool).unwrap();
        assert_eq!(value["overridesBuiltInTool"], true);
        assert_eq!(value["skipPermission"], true);
    }

    #[test]
    fn test_tool_overrides_fields_default() {
        let tool = Tool::new("test");
        let value = serde_json::to_value(&tool).unwrap();
        // Should not serialize false values
        assert!(value.get("overridesBuiltInTool").is_none());
        assert!(value.get("skipPermission").is_none());
    }

    #[test]
    fn test_session_config_new_fields() {
        let config = SessionConfig {
            client_name: Some("my-app".into()),
            agent: Some("code-reviewer".into()),
            ..Default::default()
        };
        let value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["clientName"], "my-app");
        assert_eq!(value["agent"], "code-reviewer");
    }

    #[test]
    fn test_log_options_serialization() {
        let opts = LogOptions {
            level: Some(SessionLogLevel::Warning),
            ephemeral: Some(true),
        };
        let value = serde_json::to_value(&opts).unwrap();
        assert_eq!(value["level"], "warning");
        assert_eq!(value["ephemeral"], true);
    }

    #[test]
    fn test_plan_data_serialization() {
        let plan = PlanData {
            content: Some("Step 1: Do things".into()),
            title: Some("My Plan".into()),
        };
        let value = serde_json::to_value(&plan).unwrap();
        assert_eq!(value["content"], "Step 1: Do things");
        assert_eq!(value["title"], "My Plan");
    }

    #[test]
    fn test_agent_info_deserialization() {
        let json = serde_json::json!({
            "name": "reviewer",
            "displayName": "Code Reviewer",
            "description": "Reviews code"
        });
        let agent: AgentInfo = serde_json::from_value(json).unwrap();
        assert_eq!(agent.name, "reviewer");
        assert_eq!(agent.display_name, Some("Code Reviewer".into()));
    }

    #[test]
    fn test_telemetry_config_default() {
        let config = TelemetryConfig::default();
        assert!(config.otlp_endpoint.is_none());
        assert!(config.file_path.is_none());
        assert!(config.capture_content.is_none());
    }

    #[test]
    fn test_set_model_options_serialization() {
        let opts = SetModelOptions {
            reasoning_effort: Some("high".into()),
        };
        let value = serde_json::to_value(&opts).unwrap();
        assert_eq!(value["reasoningEffort"], "high");
    }

    #[test]
    fn test_fleet_start_options_serialization() {
        let opts = FleetStartOptions {
            prompt: Some("Build and test the project".into()),
        };
        let value = serde_json::to_value(&opts).unwrap();
        assert_eq!(value["prompt"], "Build and test the project");
    }

    #[test]
    fn test_shell_exec_options_serialization() {
        let opts = ShellExecOptions {
            command: "ls -la".into(),
            cwd: Some("/tmp".into()),
            env: None,
        };
        let value = serde_json::to_value(&opts).unwrap();
        assert_eq!(value["command"], "ls -la");
        assert_eq!(value["cwd"], "/tmp");
    }

    #[test]
    fn test_quota_snapshot_deserialization() {
        let json = serde_json::json!({
            "type": "premium_requests",
            "limit": 1000,
            "used": 42,
            "remaining": 958,
            "resetsAt": "2026-04-01T00:00:00Z"
        });
        let quota: QuotaSnapshot = serde_json::from_value(json).unwrap();
        assert_eq!(quota.quota_type, "premium_requests");
        assert_eq!(quota.limit, Some(1000));
        assert_eq!(quota.used, Some(42));
    }

    #[test]
    fn test_workspace_file_deserialization() {
        let json = serde_json::json!({
            "path": "plan.md",
            "size": 1024,
            "modifiedAt": "2026-03-19T12:00:00Z"
        });
        let file: WorkspaceFile = serde_json::from_value(json).unwrap();
        assert_eq!(file.path, "plan.md");
        assert_eq!(file.size, Some(1024));
    }

    #[test]
    fn test_resume_session_config_new_fields() {
        let config = ResumeSessionConfig {
            client_name: Some("my-cli".into()),
            agent: Some("helper".into()),
            ..Default::default()
        };
        let value = serde_json::to_value(&config).unwrap();
        assert_eq!(value["clientName"], "my-cli");
        assert_eq!(value["agent"], "helper");
    }
}
