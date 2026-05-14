//! Hook types for event handling

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// ============================================================================
// Hook Types
// ============================================================================

/// Hook event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Before a tool is used
    PreToolUse,
    /// After a tool is used
    PostToolUse,
    /// After a tool use fails
    PostToolUseFailure,
    /// When a notification is received
    Notification,
    /// When user submits a prompt
    UserPromptSubmit,
    /// When a session starts
    SessionStart,
    /// When a session ends
    SessionEnd,
    /// When conversation stops
    Stop,
    /// When a subagent starts
    SubagentStart,
    /// When a subagent stops
    SubagentStop,
    /// Before compacting the conversation
    PreCompact,
    /// When a permission is requested
    PermissionRequest,
}

// ============================================================================
// Hook Input Types
// ============================================================================

/// Base fields common to all hook inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseHookInput {
    /// Session identifier
    pub session_id: String,
    /// Path to the transcript file
    pub transcript_path: String,
    /// Current working directory
    pub cwd: String,
    /// Permission mode (if set)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
}

/// Input for `SubagentStart` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStartHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Agent identifier
    pub agent_id: String,
    /// Agent type (e.g., "knowledgeBuilder", "codeReviewer")
    pub agent_type: String,
}

/// Input for `SubagentStop` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentStopHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
    /// Agent identifier (removed in CLI 2.0.75, kept optional for backward compat)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Path to the agent's transcript (removed in CLI 2.0.75, kept optional for backward compat)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_transcript_path: Option<String>,
}

/// Input for `PreToolUse` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreToolUseHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Name of the tool being used
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
}

/// Input for `PostToolUse` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Name of the tool that was used
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Tool response/output
    pub tool_response: serde_json::Value,
    /// Tool use identifier (removed in CLI 2.0.75, kept optional for backward compat)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
}

/// Input for `PostToolUseFailure` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostToolUseFailureHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Name of the tool that failed
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Tool use identifier (removed in CLI 2.0.75, kept optional for backward compat)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    /// Error message
    pub error: String,
    /// Whether this was an interrupt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_interrupt: Option<bool>,
}

/// Input for `SessionStart` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStartHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Source of session start
    pub source: SessionStartSource,
}

/// Source of session start
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStartSource {
    /// Fresh startup
    Startup,
    /// Resumed session
    Resume,
    /// After clear
    Clear,
    /// After compact
    Compact,
}

/// Input for `SessionEnd` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEndHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Reason for session end
    pub reason: SessionEndReason,
}

/// Reason for session end
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    /// Session cleared
    Clear,
    /// User logged out
    Logout,
    /// User exited prompt input
    PromptInputExit,
    /// Other reason
    Other,
}

/// Input for Stop hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Whether stop hook is active
    pub stop_hook_active: bool,
}

/// Input for `UserPromptSubmit` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPromptSubmitHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// The submitted prompt
    pub prompt: String,
}

/// Input for Notification hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Notification message
    pub message: String,
    /// Optional title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Input for `PreCompact` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreCompactHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Trigger type
    pub trigger: CompactTrigger,
    /// Custom instructions for compaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,
}

/// Trigger type for compaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactTrigger {
    /// Manually triggered
    Manual,
    /// Automatically triggered
    Auto,
}

/// Input for `PermissionRequest` hook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRequestHookInput {
    /// Base hook fields
    #[serde(flatten)]
    pub base: BaseHookInput,
    /// Hook event name
    pub hook_event_name: String,
    /// Name of the tool requesting permission
    pub tool_name: String,
    /// Tool input parameters
    pub tool_input: serde_json::Value,
    /// Permission suggestions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_suggestions: Option<Vec<serde_json::Value>>,
}

/// Union type for all hook inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "hook_event_name")]
pub enum HookInput {
    /// Pre tool use input
    PreToolUse(PreToolUseHookInput),
    /// Post tool use input
    PostToolUse(PostToolUseHookInput),
    /// Post tool use failure input
    PostToolUseFailure(PostToolUseFailureHookInput),
    /// Notification input
    Notification(NotificationHookInput),
    /// User prompt submit input
    UserPromptSubmit(UserPromptSubmitHookInput),
    /// Session start input
    SessionStart(SessionStartHookInput),
    /// Session end input
    SessionEnd(SessionEndHookInput),
    /// Stop input
    Stop(StopHookInput),
    /// Subagent start input
    SubagentStart(SubagentStartHookInput),
    /// Subagent stop input
    SubagentStop(SubagentStopHookInput),
    /// Pre compact input
    PreCompact(PreCompactHookInput),
    /// Permission request input
    PermissionRequest(PermissionRequestHookInput),
}

// ============================================================================
// Hook Output and Decision Types
// ============================================================================

/// Hook decision
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HookDecision {
    /// Block the action
    Block,
}

/// Hook output
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookOutput {
    /// Decision to block or allow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<HookDecision>,
    /// System message to add
    #[serde(skip_serializing_if = "Option::is_none", rename = "systemMessage")]
    pub system_message: Option<String>,
    /// Hook-specific output data
    #[serde(skip_serializing_if = "Option::is_none", rename = "hookSpecificOutput")]
    pub hook_specific_output: Option<serde_json::Value>,
}

/// Context for hook callbacks
///
/// Provides session information and cancellation support to hook callbacks.
/// Equivalent to TypeScript SDK's `{ signal: AbortSignal }` context.
#[derive(Clone, Default)]
pub struct HookContext {
    /// Session ID from the system init message
    pub session_id: Option<String>,
    /// Current working directory from the system init message
    pub cwd: Option<String>,
    /// Cancellation token for aborting operations (like `AbortSignal` in JS)
    pub cancellation_token: Option<CancellationToken>,
}

impl std::fmt::Debug for HookContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookContext")
            .field("session_id", &self.session_id)
            .field("cwd", &self.cwd)
            .field(
                "cancellation_token",
                &self.cancellation_token.as_ref().map(|_| "<token>"),
            )
            .finish()
    }
}

impl HookContext {
    /// Create a new `HookContext` with session information
    #[must_use]
    pub fn new(
        session_id: Option<String>,
        cwd: Option<String>,
        cancellation_token: Option<CancellationToken>,
    ) -> Self {
        Self {
            session_id,
            cwd,
            cancellation_token,
        }
    }

    /// Check if cancellation has been requested
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token
            .as_ref()
            .is_some_and(tokio_util::sync::CancellationToken::is_cancelled)
    }
}

/// Hook matcher configuration
#[derive(Clone)]
pub struct HookMatcher {
    /// Matcher pattern (e.g., tool name like "Bash" or pattern like "Write|Edit")
    pub matcher: Option<String>,
    /// List of hook callbacks (using the trait-based approach)
    pub hooks: Vec<Arc<dyn crate::callbacks::HookCallback>>,
    /// Timeout for all hooks in this matcher (default: 60 seconds)
    ///
    /// If a hook exceeds this timeout, it will be cancelled and a default
    /// `HookOutput` will be returned. This prevents runaway callbacks from
    /// blocking the agent indefinitely.
    pub timeout: Option<std::time::Duration>,
}

impl std::fmt::Debug for HookMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookMatcher")
            .field("matcher", &self.matcher)
            .field("hooks", &format!("[{} callbacks]", self.hooks.len()))
            .field("timeout", &self.timeout)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_tool_use_hook_input_with_tool_use_id() {
        // CLI < 2.0.75 format with tool_use_id
        let json = serde_json::json!({
            "session_id": "test-session",
            "transcript_path": "/tmp/transcript.json",
            "cwd": "/home/user",
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": "ls"},
            "tool_response": {"output": "file.txt"},
            "tool_use_id": "tool_123"
        });

        let input: PostToolUseHookInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.tool_name, "Bash");
        assert_eq!(input.tool_use_id, Some("tool_123".to_string()));
    }

    #[test]
    fn test_post_tool_use_hook_input_without_tool_use_id() {
        // CLI >= 2.0.75 format without tool_use_id
        let json = serde_json::json!({
            "session_id": "test-session",
            "transcript_path": "/tmp/transcript.json",
            "cwd": "/home/user",
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash",
            "tool_input": {"command": "ls"},
            "tool_response": {"output": "file.txt"}
        });

        let input: PostToolUseHookInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.tool_name, "Bash");
        assert!(input.tool_use_id.is_none());
    }

    #[test]
    fn test_post_tool_use_failure_hook_input_with_tool_use_id() {
        let json = serde_json::json!({
            "session_id": "test-session",
            "transcript_path": "/tmp/transcript.json",
            "cwd": "/home/user",
            "hook_event_name": "PostToolUseFailure",
            "tool_name": "Bash",
            "tool_input": {"command": "invalid"},
            "tool_use_id": "tool_456",
            "error": "Command not found"
        });

        let input: PostToolUseFailureHookInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.error, "Command not found");
        assert_eq!(input.tool_use_id, Some("tool_456".to_string()));
    }

    #[test]
    fn test_post_tool_use_failure_hook_input_without_tool_use_id() {
        // CLI >= 2.0.75 format
        let json = serde_json::json!({
            "session_id": "test-session",
            "transcript_path": "/tmp/transcript.json",
            "cwd": "/home/user",
            "hook_event_name": "PostToolUseFailure",
            "tool_name": "Bash",
            "tool_input": {"command": "invalid"},
            "error": "Command not found"
        });

        let input: PostToolUseFailureHookInput = serde_json::from_value(json).unwrap();
        assert_eq!(input.error, "Command not found");
        assert!(input.tool_use_id.is_none());
    }

    #[test]
    fn test_subagent_stop_hook_input_with_agent_fields() {
        // CLI < 2.0.75 format with agent_id and agent_transcript_path
        let json = serde_json::json!({
            "session_id": "test-session",
            "transcript_path": "/tmp/transcript.json",
            "cwd": "/home/user",
            "hook_event_name": "SubagentStop",
            "stop_hook_active": true,
            "agent_id": "agent_789",
            "agent_transcript_path": "/tmp/agent_transcript.json"
        });

        let input: SubagentStopHookInput = serde_json::from_value(json).unwrap();
        assert!(input.stop_hook_active);
        assert_eq!(input.agent_id, Some("agent_789".to_string()));
        assert_eq!(
            input.agent_transcript_path,
            Some("/tmp/agent_transcript.json".to_string())
        );
    }

    #[test]
    fn test_subagent_stop_hook_input_without_agent_fields() {
        // CLI >= 2.0.75 format without agent_id and agent_transcript_path
        let json = serde_json::json!({
            "session_id": "test-session",
            "transcript_path": "/tmp/transcript.json",
            "cwd": "/home/user",
            "hook_event_name": "SubagentStop",
            "stop_hook_active": false
        });

        let input: SubagentStopHookInput = serde_json::from_value(json).unwrap();
        assert!(!input.stop_hook_active);
        assert!(input.agent_id.is_none());
        assert!(input.agent_transcript_path.is_none());
    }

    #[test]
    fn test_hook_output_serialization_omits_none() {
        let output = HookOutput::default();
        let json = serde_json::to_string(&output).unwrap();
        // Should be empty object when all fields are None
        assert_eq!(json, "{}");
    }

    #[test]
    fn test_hook_output_with_decision() {
        let output = HookOutput {
            decision: Some(HookDecision::Block),
            system_message: Some("Blocked for safety".to_string()),
            hook_specific_output: None,
        };

        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("\"decision\":\"block\""));
        assert!(json.contains("\"systemMessage\":\"Blocked for safety\""));
    }

    #[test]
    fn test_hook_event_serde() {
        let event = HookEvent::PreToolUse;
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(json, "\"PreToolUse\"");

        let parsed: HookEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, HookEvent::PreToolUse);
    }

    #[test]
    fn test_session_start_source_serde() {
        let source = SessionStartSource::Resume;
        let json = serde_json::to_string(&source).unwrap();
        assert_eq!(json, "\"resume\"");

        let parsed: SessionStartSource = serde_json::from_str("\"startup\"").unwrap();
        assert_eq!(parsed, SessionStartSource::Startup);
    }

    #[test]
    fn test_session_end_reason_serde() {
        let reason = SessionEndReason::PromptInputExit;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, "\"prompt_input_exit\"");

        let parsed: SessionEndReason = serde_json::from_str("\"clear\"").unwrap();
        assert_eq!(parsed, SessionEndReason::Clear);
    }

    #[test]
    fn test_compact_trigger_serde() {
        let trigger = CompactTrigger::Auto;
        let json = serde_json::to_string(&trigger).unwrap();
        assert_eq!(json, "\"auto\"");

        let parsed: CompactTrigger = serde_json::from_str("\"manual\"").unwrap();
        assert_eq!(parsed, CompactTrigger::Manual);
    }
}
