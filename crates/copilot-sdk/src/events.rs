// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! Session event types for the Copilot SDK.
//!
//! Events are received from the Copilot CLI during a session. They include
//! assistant messages, tool executions, session lifecycle events, and more.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Nested Types (used within event data)
// =============================================================================

/// Handoff source type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandoffSourceType {
    Remote,
    Local,
}

/// System message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemMessageRole {
    System,
    Developer,
}

/// Repository info for handoff events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryInfo {
    pub owner: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// Attachment in user message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageAttachmentItem {
    #[serde(rename = "type")]
    pub attachment_type: super::AttachmentType,
    pub path: String,
    pub display_name: String,
}

/// Tool request in assistant message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolRequestItem {
    pub tool_call_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Tool execution result content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    pub content: String,
}

/// Tool execution error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Hook error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookError {
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
}

/// System message metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMessageMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, serde_json::Value>>,
}

// =============================================================================
// Event Data Types
// =============================================================================

/// Data for session.start event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStartData {
    #[serde(default)]
    pub session_id: String,
    #[serde(default)]
    pub version: f64,
    #[serde(default)]
    pub producer: String,
    #[serde(default)]
    pub copilot_version: String,
    #[serde(default)]
    pub start_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_model: Option<String>,
}

/// Data for session.resume event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResumeData {
    pub resume_time: String,
    pub event_count: f64,
}

/// Data for session.error event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionErrorData {
    pub error_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_call_id: Option<String>,
}

/// Data for session.idle event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionIdleData {}

/// Data for session.info event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfoData {
    pub info_type: String,
    pub message: String,
}

/// Data for session.model_change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionModelChangeData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_model: Option<String>,
    pub new_model: String,
}

/// Data for session.handoff event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHandoffData {
    pub handoff_time: String,
    pub source_type: HandoffSourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<RepositoryInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_session_id: Option<String>,
}

/// Data for session.truncation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTruncationData {
    pub token_limit: f64,
    pub pre_truncation_tokens_in_messages: f64,
    pub pre_truncation_messages_length: f64,
    pub post_truncation_tokens_in_messages: f64,
    pub post_truncation_messages_length: f64,
    pub tokens_removed_during_truncation: f64,
    pub messages_removed_during_truncation: f64,
    pub performed_by: String,
}

/// Data for user.message event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMessageData {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transformed_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attachments: Option<Vec<UserMessageAttachmentItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Data for pending_messages.modified event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PendingMessagesModifiedData {}

/// Data for assistant.turn_start event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantTurnStartData {
    pub turn_id: String,
}

/// Data for assistant.intent event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantIntentData {
    pub intent: String,
}

/// Data for assistant.reasoning event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantReasoningData {
    pub reasoning_id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_content: Option<String>,
}

/// Data for assistant.reasoning_delta event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantReasoningDeltaData {
    pub reasoning_id: String,
    pub delta_content: String,
}

/// Data for assistant.message event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessageData {
    pub message_id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_response_size_bytes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_requests: Option<Vec<ToolRequestItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
}

/// Data for assistant.message_delta event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessageDeltaData {
    pub message_id: String,
    pub delta_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_response_size_bytes: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
}

/// Data for assistant.turn_end event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantTurnEndData {
    pub turn_id: String,
}

/// Data for assistant.usage event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantUsageData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initiator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quota_snapshots: Option<HashMap<String, serde_json::Value>>,
}

/// Data for abort event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbortData {
    pub reason: String,
}

/// Data for tool.user_requested event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUserRequestedData {
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// Data for tool.execution_start event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionStartData {
    pub tool_call_id: String,
    pub tool_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
}

/// Data for tool.execution_partial_result event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionPartialResultData {
    pub tool_call_id: String,
    pub partial_output: String,
}

/// Data for tool.execution_complete event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionCompleteData {
    pub tool_call_id: String,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_user_requested: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ToolExecutionError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_telemetry: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_server_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_tool_name: Option<String>,
}

/// Data for custom_agent.started event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomAgentStartedData {
    pub tool_call_id: String,
    pub agent_name: String,
    pub agent_display_name: String,
    pub agent_description: String,
}

/// Data for custom_agent.completed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomAgentCompletedData {
    pub tool_call_id: String,
    pub agent_name: String,
}

/// Data for custom_agent.failed event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomAgentFailedData {
    pub tool_call_id: String,
    pub agent_name: String,
    pub error: String,
}

/// Data for custom_agent.selected event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomAgentSelectedData {
    pub agent_name: String,
    pub agent_display_name: String,
    pub tools: Vec<String>,
}

/// Data for hook.start event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookStartData {
    pub hook_invocation_id: String,
    pub hook_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
}

/// Data for hook.end event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEndData {
    pub hook_invocation_id: String,
    pub hook_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<HookError>,
}

/// Data for system.message event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemMessageEventData {
    pub content: String,
    pub role: SystemMessageRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<SystemMessageMetadata>,
}

/// Data for session.compaction_start event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCompactionStartData {}

/// Tokens used during compaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompactionTokensUsed {
    #[serde(default)]
    pub input: f64,
    #[serde(default)]
    pub output: f64,
    #[serde(default)]
    pub cached_input: f64,
}

/// Data for session.compaction_complete event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCompactionCompleteData {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_compaction_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_compaction_tokens: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pre_compaction_messages_length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_compaction_messages_length: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compaction_tokens_used: Option<CompactionTokensUsed>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages_removed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_removed: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_number: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checkpoint_path: Option<String>,
}

/// Shutdown type for session.shutdown event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShutdownType {
    Routine,
    Error,
}

/// Code changes reported in shutdown event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownCodeChanges {
    #[serde(default)]
    pub lines_added: f64,
    #[serde(default)]
    pub lines_removed: f64,
    #[serde(default)]
    pub files_modified: Vec<String>,
}

/// Data for session.shutdown event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionShutdownData {
    pub shutdown_type: ShutdownType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    #[serde(default)]
    pub total_premium_requests: f64,
    #[serde(default)]
    pub total_api_duration_ms: f64,
    #[serde(default)]
    pub session_start_time: f64,
    #[serde(default)]
    pub code_changes: ShutdownCodeChanges,
    #[serde(default)]
    pub model_metrics: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_model: Option<String>,
}

/// Data for session.snapshot_rewind event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshotRewindData {
    pub up_to_event_id: String,
    #[serde(default)]
    pub events_removed: f64,
}

/// Data for session.usage_info event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionUsageInfoData {
    #[serde(default)]
    pub token_limit: f64,
    #[serde(default)]
    pub current_tokens: f64,
    #[serde(default)]
    pub messages_length: f64,
}

/// Data for tool.execution_progress event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolExecutionProgressData {
    pub tool_call_id: String,
    pub progress_message: String,
}

/// Data for skill.invoked event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInvokedData {
    pub name: String,
    pub path: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<Vec<String>>,
}

// =============================================================================
// Session Event (Discriminated Union)
// =============================================================================

/// Data for `external_tool.requested` event (protocol v3 broadcast model).
///
/// In protocol v3, tool calls are broadcast as session events instead of
/// RPC requests. The SDK handles these internally and responds via
/// `session.tools.handlePendingToolCall` RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalToolRequestedData {
    /// Unique request ID for correlating the response.
    pub request_id: Option<String>,
    /// Name of the tool being requested.
    pub tool_name: Option<String>,
    /// Tool call ID for tracking.
    pub tool_call_id: Option<String>,
    /// Arguments to pass to the tool handler.
    pub arguments: Option<serde_json::Value>,
}

/// Data for `permission.requested` event (protocol v3 broadcast model).
///
/// In protocol v3, permission requests are broadcast as session events.
/// The SDK handles these internally and responds via
/// `session.permissions.handlePendingPermissionRequest` RPC.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequestedData {
    /// Unique request ID for correlating the response.
    pub request_id: Option<String>,
    /// The permission request details.
    pub permission_request: Option<serde_json::Value>,
}

/// Event data variants - the payload of each event type.
#[derive(Debug, Clone, Serialize)]
pub enum SessionEventData {
    SessionStart(SessionStartData),
    SessionResume(SessionResumeData),
    SessionError(SessionErrorData),
    SessionIdle(SessionIdleData),
    SessionInfo(SessionInfoData),
    SessionModelChange(SessionModelChangeData),
    SessionHandoff(SessionHandoffData),
    SessionTruncation(SessionTruncationData),
    UserMessage(UserMessageData),
    PendingMessagesModified(PendingMessagesModifiedData),
    AssistantTurnStart(AssistantTurnStartData),
    AssistantIntent(AssistantIntentData),
    AssistantReasoning(AssistantReasoningData),
    AssistantReasoningDelta(AssistantReasoningDeltaData),
    AssistantMessage(AssistantMessageData),
    AssistantMessageDelta(AssistantMessageDeltaData),
    AssistantTurnEnd(AssistantTurnEndData),
    AssistantUsage(AssistantUsageData),
    Abort(AbortData),
    ToolUserRequested(ToolUserRequestedData),
    ToolExecutionStart(ToolExecutionStartData),
    ToolExecutionPartialResult(ToolExecutionPartialResultData),
    ToolExecutionComplete(ToolExecutionCompleteData),
    ToolExecutionProgress(ToolExecutionProgressData),
    CustomAgentStarted(CustomAgentStartedData),
    CustomAgentCompleted(CustomAgentCompletedData),
    CustomAgentFailed(CustomAgentFailedData),
    CustomAgentSelected(CustomAgentSelectedData),
    HookStart(HookStartData),
    HookEnd(HookEndData),
    SystemMessage(SystemMessageEventData),
    SessionCompactionStart(SessionCompactionStartData),
    SessionCompactionComplete(SessionCompactionCompleteData),
    SessionShutdown(SessionShutdownData),
    SessionSnapshotRewind(SessionSnapshotRewindData),
    SessionUsageInfo(SessionUsageInfoData),
    SkillInvoked(SkillInvokedData),
    /// External tool requested (protocol v3 broadcast).
    ExternalToolRequested(ExternalToolRequestedData),
    /// Permission requested (protocol v3 broadcast).
    PermissionRequested(PermissionRequestedData),
    /// Unknown event - preserves raw JSON for forward compatibility.
    Unknown(serde_json::Value),
}

/// Raw session event as received from the CLI.
///
/// The event has common fields (id, timestamp, type) and a data payload
/// that varies based on the event type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawSessionEvent {
    pub id: String,
    pub timestamp: String,
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<bool>,
    pub data: serde_json::Value,
}

/// A parsed session event with typed data.
#[derive(Debug, Clone)]
pub struct SessionEvent {
    /// Unique event ID.
    pub id: String,
    /// ISO 8601 timestamp.
    pub timestamp: String,
    /// Original type string (e.g., "assistant.message").
    pub event_type: String,
    /// Parent event ID, if any.
    pub parent_id: Option<String>,
    /// Whether this event is ephemeral.
    pub ephemeral: Option<bool>,
    /// Typed event data.
    pub data: SessionEventData,
}

impl SessionEvent {
    /// Parse a session event from JSON.
    pub fn from_json(json: &serde_json::Value) -> Result<Self, serde_json::Error> {
        let raw: RawSessionEvent = serde_json::from_value(json.clone())?;
        Ok(Self::from_raw(raw))
    }

    /// Convert a raw event to a typed event.
    pub fn from_raw(raw: RawSessionEvent) -> Self {
        let data = parse_event_data(&raw.event_type, raw.data);
        Self {
            id: raw.id,
            timestamp: raw.timestamp,
            event_type: raw.event_type,
            parent_id: raw.parent_id,
            ephemeral: raw.ephemeral,
            data,
        }
    }

    // =========================================================================
    // Type checking helpers
    // =========================================================================

    /// Check if this is an assistant message event.
    pub fn is_assistant_message(&self) -> bool {
        matches!(self.data, SessionEventData::AssistantMessage(_))
    }

    /// Check if this is an assistant message delta event.
    pub fn is_assistant_message_delta(&self) -> bool {
        matches!(self.data, SessionEventData::AssistantMessageDelta(_))
    }

    /// Check if this is a session idle event.
    pub fn is_session_idle(&self) -> bool {
        matches!(self.data, SessionEventData::SessionIdle(_))
    }

    /// Check if this is a session error event.
    pub fn is_session_error(&self) -> bool {
        matches!(self.data, SessionEventData::SessionError(_))
    }

    /// Check if this is a terminal event (session ended).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.data,
            SessionEventData::SessionIdle(_) | SessionEventData::SessionError(_)
        )
    }

    // =========================================================================
    // Data extraction helpers
    // =========================================================================

    /// Get assistant message data if this is an assistant.message event.
    pub fn as_assistant_message(&self) -> Option<&AssistantMessageData> {
        match &self.data {
            SessionEventData::AssistantMessage(data) => Some(data),
            _ => None,
        }
    }

    /// Get assistant message delta data if this is an assistant.message_delta event.
    pub fn as_assistant_message_delta(&self) -> Option<&AssistantMessageDeltaData> {
        match &self.data {
            SessionEventData::AssistantMessageDelta(data) => Some(data),
            _ => None,
        }
    }

    /// Get session error data if this is a session.error event.
    pub fn as_session_error(&self) -> Option<&SessionErrorData> {
        match &self.data {
            SessionEventData::SessionError(data) => Some(data),
            _ => None,
        }
    }

    /// Get tool execution complete data if this is a tool.execution_complete event.
    pub fn as_tool_execution_complete(&self) -> Option<&ToolExecutionCompleteData> {
        match &self.data {
            SessionEventData::ToolExecutionComplete(data) => Some(data),
            _ => None,
        }
    }

    /// Extract the content from an assistant message or delta.
    pub fn content(&self) -> Option<&str> {
        match &self.data {
            SessionEventData::AssistantMessage(data) => Some(&data.content),
            SessionEventData::AssistantMessageDelta(data) => Some(&data.delta_content),
            _ => None,
        }
    }
}

/// Parse event data based on event type string.
fn parse_event_data(event_type: &str, data: serde_json::Value) -> SessionEventData {
    match event_type {
        "session.start" => serde_json::from_value(data)
            .map(SessionEventData::SessionStart)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.resume" => serde_json::from_value(data)
            .map(SessionEventData::SessionResume)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.error" => serde_json::from_value(data)
            .map(SessionEventData::SessionError)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.idle" => SessionEventData::SessionIdle(SessionIdleData {}),
        "session.info" => serde_json::from_value(data)
            .map(SessionEventData::SessionInfo)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.model_change" => serde_json::from_value(data)
            .map(SessionEventData::SessionModelChange)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.handoff" => serde_json::from_value(data)
            .map(SessionEventData::SessionHandoff)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.truncation" => serde_json::from_value(data)
            .map(SessionEventData::SessionTruncation)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "user.message" => serde_json::from_value(data)
            .map(SessionEventData::UserMessage)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "pending_messages.modified" => {
            SessionEventData::PendingMessagesModified(PendingMessagesModifiedData {})
        }
        "assistant.turn_start" => serde_json::from_value(data)
            .map(SessionEventData::AssistantTurnStart)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "assistant.intent" => serde_json::from_value(data)
            .map(SessionEventData::AssistantIntent)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "assistant.reasoning" => serde_json::from_value(data)
            .map(SessionEventData::AssistantReasoning)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "assistant.reasoning_delta" => serde_json::from_value(data)
            .map(SessionEventData::AssistantReasoningDelta)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "assistant.message" => serde_json::from_value(data)
            .map(SessionEventData::AssistantMessage)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "assistant.message_delta" => serde_json::from_value(data)
            .map(SessionEventData::AssistantMessageDelta)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "assistant.turn_end" => serde_json::from_value(data)
            .map(SessionEventData::AssistantTurnEnd)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "assistant.usage" => serde_json::from_value(data)
            .map(SessionEventData::AssistantUsage)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "abort" => serde_json::from_value(data)
            .map(SessionEventData::Abort)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "tool.user_requested" => serde_json::from_value(data)
            .map(SessionEventData::ToolUserRequested)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "tool.execution_start" => serde_json::from_value(data)
            .map(SessionEventData::ToolExecutionStart)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "tool.execution_partial_result" => serde_json::from_value(data)
            .map(SessionEventData::ToolExecutionPartialResult)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "tool.execution_complete" => serde_json::from_value(data)
            .map(SessionEventData::ToolExecutionComplete)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "tool.execution_progress" => serde_json::from_value(data)
            .map(SessionEventData::ToolExecutionProgress)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        // Primary wire names (subagent.*) + legacy aliases (custom_agent.*)
        "subagent.started" | "custom_agent.started" => serde_json::from_value(data)
            .map(SessionEventData::CustomAgentStarted)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "subagent.completed" | "custom_agent.completed" => serde_json::from_value(data)
            .map(SessionEventData::CustomAgentCompleted)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "subagent.failed" | "custom_agent.failed" => serde_json::from_value(data)
            .map(SessionEventData::CustomAgentFailed)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "subagent.selected" | "custom_agent.selected" => serde_json::from_value(data)
            .map(SessionEventData::CustomAgentSelected)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "hook.start" => serde_json::from_value(data)
            .map(SessionEventData::HookStart)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "hook.end" => serde_json::from_value(data)
            .map(SessionEventData::HookEnd)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "system.message" => serde_json::from_value(data)
            .map(SessionEventData::SystemMessage)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.compaction_start" => {
            SessionEventData::SessionCompactionStart(SessionCompactionStartData {})
        }
        "session.compaction_complete" => serde_json::from_value(data)
            .map(SessionEventData::SessionCompactionComplete)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.shutdown" => serde_json::from_value(data)
            .map(SessionEventData::SessionShutdown)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.snapshot_rewind" => serde_json::from_value(data)
            .map(SessionEventData::SessionSnapshotRewind)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "session.usage_info" => serde_json::from_value(data)
            .map(SessionEventData::SessionUsageInfo)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "skill.invoked" => serde_json::from_value(data)
            .map(SessionEventData::SkillInvoked)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "external_tool.requested" => serde_json::from_value(data)
            .map(SessionEventData::ExternalToolRequested)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        "permission.requested" => serde_json::from_value(data)
            .map(SessionEventData::PermissionRequested)
            .unwrap_or_else(|_| SessionEventData::Unknown(serde_json::Value::Null)),
        // Unknown event type - preserve raw data
        _ => SessionEventData::Unknown(data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_assistant_message() {
        let json = json!({
            "id": "evt_123",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "assistant.message",
            "data": {
                "messageId": "msg_456",
                "content": "Hello, world!"
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        assert_eq!(event.id, "evt_123");
        assert_eq!(event.event_type, "assistant.message");
        assert!(event.is_assistant_message());

        let msg = event.as_assistant_message().unwrap();
        assert_eq!(msg.message_id, "msg_456");
        assert_eq!(msg.content, "Hello, world!");
    }

    #[test]
    fn test_parse_assistant_message_delta() {
        let json = json!({
            "id": "evt_124",
            "timestamp": "2024-01-15T10:30:01Z",
            "type": "assistant.message_delta",
            "data": {
                "messageId": "msg_456",
                "deltaContent": "Hello"
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        assert!(event.is_assistant_message_delta());
        assert_eq!(event.content(), Some("Hello"));
    }

    #[test]
    fn test_parse_session_idle() {
        let json = json!({
            "id": "evt_125",
            "timestamp": "2024-01-15T10:30:02Z",
            "type": "session.idle",
            "data": {}
        });

        let event = SessionEvent::from_json(&json).unwrap();
        assert!(event.is_session_idle());
        assert!(event.is_terminal());
    }

    #[test]
    fn test_parse_external_tool_requested() {
        let json = json!({
            "id": "evt_125b",
            "timestamp": "2024-01-15T10:30:02Z",
            "type": "external_tool.requested",
            "data": {
                "requestId": "req_123",
                "toolName": "echo",
                "toolCallId": "call_456",
                "arguments": {
                    "text": "hello"
                }
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        match &event.data {
            SessionEventData::ExternalToolRequested(data) => {
                assert_eq!(data.request_id.as_deref(), Some("req_123"));
                assert_eq!(data.tool_name.as_deref(), Some("echo"));
                assert_eq!(data.tool_call_id.as_deref(), Some("call_456"));
                assert_eq!(data.arguments.as_ref().unwrap()["text"], "hello");
            }
            other => panic!("Expected ExternalToolRequested, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_permission_requested() {
        let json = json!({
            "id": "evt_125c",
            "timestamp": "2024-01-15T10:30:02Z",
            "type": "permission.requested",
            "data": {
                "requestId": "req_789",
                "permissionRequest": {
                    "kind": "tool_execution",
                    "toolCallId": "call_456",
                    "toolName": "shell"
                }
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        match &event.data {
            SessionEventData::PermissionRequested(data) => {
                assert_eq!(data.request_id.as_deref(), Some("req_789"));
                assert_eq!(
                    data.permission_request.as_ref().unwrap()["kind"],
                    "tool_execution"
                );
                assert_eq!(
                    data.permission_request.as_ref().unwrap()["toolName"],
                    "shell"
                );
            }
            other => panic!("Expected PermissionRequested, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_session_error() {
        let json = json!({
            "id": "evt_126",
            "timestamp": "2024-01-15T10:30:03Z",
            "type": "session.error",
            "data": {
                "errorType": "api_error",
                "message": "Rate limit exceeded"
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        assert!(event.is_session_error());
        assert!(event.is_terminal());

        let err = event.as_session_error().unwrap();
        assert_eq!(err.error_type, "api_error");
        assert_eq!(err.message, "Rate limit exceeded");
    }

    #[test]
    fn test_parse_tool_execution_complete() {
        let json = json!({
            "id": "evt_127",
            "timestamp": "2024-01-15T10:30:04Z",
            "type": "tool.execution_complete",
            "data": {
                "toolCallId": "call_789",
                "success": true,
                "result": {
                    "content": "Tool output"
                }
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        let tool = event.as_tool_execution_complete().unwrap();
        assert_eq!(tool.tool_call_id, "call_789");
        assert!(tool.success);
        assert_eq!(tool.result.as_ref().unwrap().content, "Tool output");
    }

    #[test]
    fn test_parse_unknown_event() {
        let json = json!({
            "id": "evt_128",
            "timestamp": "2024-01-15T10:30:05Z",
            "type": "future.unknown_event",
            "data": {
                "someField": "someValue"
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        assert_eq!(event.event_type, "future.unknown_event");
        assert!(matches!(event.data, SessionEventData::Unknown(_)));
    }

    #[test]
    fn test_parse_session_start() {
        let json = json!({
            "id": "evt_001",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "session.start",
            "data": {
                "sessionId": "sess_123",
                "version": 1.0,
                "producer": "copilot-cli",
                "copilotVersion": "1.0.0",
                "startTime": "2024-01-15T10:30:00Z"
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::SessionStart(data) = &event.data {
            assert_eq!(data.session_id, "sess_123");
            assert_eq!(data.producer, "copilot-cli");
        } else {
            panic!("Expected SessionStart");
        }
    }

    #[test]
    fn test_event_with_parent_id() {
        let json = json!({
            "id": "evt_129",
            "timestamp": "2024-01-15T10:30:06Z",
            "type": "assistant.message",
            "parentId": "evt_128",
            "ephemeral": true,
            "data": {
                "messageId": "msg_789",
                "content": "Nested message"
            }
        });

        let event = SessionEvent::from_json(&json).unwrap();
        assert_eq!(event.parent_id, Some("evt_128".to_string()));
        assert_eq!(event.ephemeral, Some(true));
    }

    #[test]
    fn test_parse_subagent_started() {
        let json = json!({
            "id": "evt_200",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "subagent.started",
            "data": {
                "toolCallId": "call_1",
                "agentName": "test-agent",
                "agentDisplayName": "Test Agent",
                "agentDescription": "A test agent"
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        assert!(matches!(
            event.data,
            SessionEventData::CustomAgentStarted(_)
        ));
        if let SessionEventData::CustomAgentStarted(data) = &event.data {
            assert_eq!(data.agent_name, "test-agent");
        }
    }

    #[test]
    fn test_parse_subagent_completed_legacy_alias() {
        // Verify legacy custom_agent.* wire names still work
        let json = json!({
            "id": "evt_201",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "custom_agent.completed",
            "data": {
                "toolCallId": "call_1",
                "agentName": "test-agent"
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        assert!(matches!(
            event.data,
            SessionEventData::CustomAgentCompleted(_)
        ));
    }

    #[test]
    fn test_parse_subagent_all_wire_names() {
        for wire_name in &["subagent.failed", "custom_agent.failed"] {
            let json = json!({
                "id": "evt_202",
                "timestamp": "2024-01-15T10:30:00Z",
                "type": wire_name,
                "data": {
                    "toolCallId": "call_1",
                    "agentName": "agent",
                    "error": "boom"
                }
            });
            let event = SessionEvent::from_json(&json).unwrap();
            assert!(
                matches!(event.data, SessionEventData::CustomAgentFailed(_)),
                "Failed to parse {wire_name}"
            );
        }
    }

    #[test]
    fn test_parse_session_compaction_start() {
        let json = json!({
            "id": "evt_300",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "session.compaction_start",
            "data": {}
        });
        let event = SessionEvent::from_json(&json).unwrap();
        assert!(matches!(
            event.data,
            SessionEventData::SessionCompactionStart(_)
        ));
    }

    #[test]
    fn test_parse_session_compaction_complete() {
        let json = json!({
            "id": "evt_301",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "session.compaction_complete",
            "data": {
                "success": true,
                "preCompactionTokens": 50000.0,
                "postCompactionTokens": 10000.0,
                "compactionTokensUsed": {
                    "input": 100.0,
                    "output": 200.0,
                    "cachedInput": 50.0
                },
                "summaryContent": "Session was compacted"
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::SessionCompactionComplete(data) = &event.data {
            assert!(data.success);
            assert_eq!(data.pre_compaction_tokens, Some(50000.0));
            assert_eq!(data.compaction_tokens_used.as_ref().unwrap().input, 100.0);
        } else {
            panic!("Expected SessionCompactionComplete");
        }
    }

    #[test]
    fn test_parse_session_shutdown() {
        let json = json!({
            "id": "evt_302",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "session.shutdown",
            "data": {
                "shutdownType": "routine",
                "totalPremiumRequests": 5.0,
                "totalApiDurationMs": 1200.0,
                "sessionStartTime": 1700000000.0,
                "codeChanges": {
                    "linesAdded": 10.0,
                    "linesRemoved": 3.0,
                    "filesModified": ["src/main.rs"]
                },
                "modelMetrics": {},
                "currentModel": "gpt-4"
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::SessionShutdown(data) = &event.data {
            assert_eq!(data.shutdown_type, ShutdownType::Routine);
            assert_eq!(data.current_model, Some("gpt-4".to_string()));
            assert_eq!(data.code_changes.lines_added, 10.0);
        } else {
            panic!("Expected SessionShutdown");
        }
    }

    #[test]
    fn test_parse_session_snapshot_rewind() {
        let json = json!({
            "id": "evt_303",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "session.snapshot_rewind",
            "data": {
                "upToEventId": "evt_100",
                "eventsRemoved": 5.0
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::SessionSnapshotRewind(data) = &event.data {
            assert_eq!(data.up_to_event_id, "evt_100");
            assert_eq!(data.events_removed, 5.0);
        } else {
            panic!("Expected SessionSnapshotRewind");
        }
    }

    #[test]
    fn test_parse_session_usage_info() {
        let json = json!({
            "id": "evt_304",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "session.usage_info",
            "data": {
                "tokenLimit": 100000.0,
                "currentTokens": 50000.0,
                "messagesLength": 42.0
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::SessionUsageInfo(data) = &event.data {
            assert_eq!(data.token_limit, 100000.0);
            assert_eq!(data.current_tokens, 50000.0);
        } else {
            panic!("Expected SessionUsageInfo");
        }
    }

    #[test]
    fn test_parse_tool_execution_progress() {
        let json = json!({
            "id": "evt_305",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "tool.execution_progress",
            "data": {
                "toolCallId": "call_100",
                "progressMessage": "Processing file 3 of 10..."
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::ToolExecutionProgress(data) = &event.data {
            assert_eq!(data.tool_call_id, "call_100");
            assert_eq!(data.progress_message, "Processing file 3 of 10...");
        } else {
            panic!("Expected ToolExecutionProgress");
        }
    }

    #[test]
    fn test_parse_skill_invoked() {
        let json = json!({
            "id": "evt_306",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "skill.invoked",
            "data": {
                "name": "code-review",
                "path": "/skills/code-review",
                "content": "Review this code",
                "allowedTools": ["read_file", "search"]
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::SkillInvoked(data) = &event.data {
            assert_eq!(data.name, "code-review");
            assert_eq!(data.allowed_tools.as_ref().unwrap().len(), 2);
        } else {
            panic!("Expected SkillInvoked");
        }
    }

    #[test]
    fn test_session_error_with_code_and_provider_call_id() {
        let json = json!({
            "id": "evt_err",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "session.error",
            "data": {
                "errorType": "provider_error",
                "message": "Rate limited",
                "code": 429.0,
                "providerCallId": "call-abc-123"
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::SessionError(data) = &event.data {
            assert_eq!(data.error_type, "provider_error");
            assert_eq!(data.code, Some(429.0));
            assert_eq!(data.provider_call_id.as_deref(), Some("call-abc-123"));
        } else {
            panic!("Expected SessionError");
        }
    }

    #[test]
    fn test_tool_execution_complete_with_mcp_fields() {
        let json = json!({
            "id": "evt_mcp",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "tool.execution_complete",
            "data": {
                "toolCallId": "call-1",
                "success": true,
                "mcpServerName": "my-server",
                "mcpToolName": "read_file"
            }
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::ToolExecutionComplete(data) = &event.data {
            assert_eq!(data.mcp_server_name.as_deref(), Some("my-server"));
            assert_eq!(data.mcp_tool_name.as_deref(), Some("read_file"));
        } else {
            panic!("Expected ToolExecutionComplete");
        }
    }

    #[test]
    fn test_session_start_data_optional_fields() {
        // All fields missing should still parse with defaults
        let json = json!({
            "id": "evt_start",
            "timestamp": "2024-01-15T10:30:00Z",
            "type": "session.start",
            "data": {}
        });
        let event = SessionEvent::from_json(&json).unwrap();
        if let SessionEventData::SessionStart(data) = &event.data {
            assert_eq!(data.session_id, "");
            assert_eq!(data.version, 0.0);
            assert_eq!(data.producer, "");
        } else {
            panic!("Expected SessionStart");
        }
    }

    #[test]
    fn test_unknown_event_type_handled_gracefully() {
        let json = json!({
            "id": "evt_unknown",
            "timestamp": "2025-01-01T00:00:00Z",
            "type": "some.future.event.type",
            "data": {"someField": "someValue"}
        });
        // Parsing an unknown event type should not panic
        let raw: RawSessionEvent = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(raw.event_type, "some.future.event.type");

        // It should also parse into a SessionEvent with Unknown data
        let event = SessionEvent::from_json(&json).unwrap();
        assert_eq!(event.event_type, "some.future.event.type");
        assert!(matches!(event.data, SessionEventData::Unknown(_)));
    }

    #[test]
    fn test_session_shutdown_event_parsed() {
        let json = json!({
            "id": "evt_shutdown",
            "timestamp": "2025-01-01T00:00:00Z",
            "type": "session.shutdown",
            "data": {
                "shutdownType": "routine",
                "reason": "user requested"
            }
        });
        let raw: RawSessionEvent = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(raw.event_type, "session.shutdown");

        let event = SessionEvent::from_json(&json).unwrap();
        assert_eq!(event.event_type, "session.shutdown");
    }

    #[test]
    fn test_session_usage_info_recognized() {
        let json = json!({
            "id": "evt_usage",
            "timestamp": "2025-01-01T00:00:00Z",
            "type": "session.usage_info",
            "data": {}
        });
        let raw: RawSessionEvent = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(raw.event_type, "session.usage_info");

        let event = SessionEvent::from_json(&json).unwrap();
        assert_eq!(event.event_type, "session.usage_info");
    }
}
