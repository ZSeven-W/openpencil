//! Hook system for intercepting agent events
//!
//! This module provides the hook system that allows users to intercept
//! and respond to various events in the agent lifecycle.
//!
//! # Architecture
//!
//! The hook system mirrors the TypeScript SDK design:
//! - Hooks are registered by event type (`PreToolUse`, `PostToolUse`, etc.)
//! - Each event type can have multiple matchers with patterns
//! - Matchers filter by tool name (e.g., "Bash", "Write|Edit", "*")
//! - Callbacks receive typed input and return `HookOutput`
//!
//! # Example
//!
//! ```no_run
//! use anthropic_agent_sdk::hooks::{HookManager, HookMatcherBuilder};
//! use anthropic_agent_sdk::types::{HookEvent, HookOutput, HookContext};
//! use std::collections::HashMap;
//!
//! let mut manager = HookManager::new();
//!
//! // Register a PreToolUse hook for Bash commands
//! let hook = HookManager::callback(|input, tool_name, ctx| async move {
//!     // ctx contains session_id, cwd, and cancellation_token
//!     println!("Tool: {:?}, Input: {:?}, Session: {:?}", tool_name, input, ctx.session_id);
//!     Ok(HookOutput::default())
//! });
//!
//! manager.register_for_event(
//!     HookEvent::PreToolUse,
//!     HookMatcherBuilder::new(Some("Bash")).add_hook(hook).build(),
//! );
//! ```

use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::callbacks::{FnHookCallback, HookCallback};
use crate::error::Result;
use crate::types::{
    ContentBlock, ContentValue, HookContext, HookDecision, HookEvent, HookMatcher, HookOutput,
    Message,
};

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert `ContentValue` to a string representation for error messages
fn content_to_string(content: Option<&ContentValue>) -> String {
    match content {
        Some(ContentValue::String(s)) => s.clone(),
        Some(ContentValue::Blocks(blocks)) => {
            // Try to extract text from blocks
            blocks
                .iter()
                .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        }
        None => String::new(),
    }
}

// ============================================================================
// Pending Tool Use Tracking
// ============================================================================

/// Tracks a tool use for matching with its result
#[derive(Debug, Clone)]
pub struct PendingToolUse {
    /// Tool name
    pub tool_name: String,
    /// Tool input
    pub tool_input: serde_json::Value,
    /// Whether this is a Task (subagent) tool
    pub is_task: bool,
}

// ============================================================================
// Hook Manager
// ============================================================================

/// Hook manager for registering and invoking hooks by event type
///
/// Mirrors the TypeScript SDK structure:
/// ```typescript
/// hooks: Partial<Record<HookEvent, HookCallbackMatcher[]>>
/// ```
pub struct HookManager {
    /// Hooks registered by event type
    hooks_by_event: HashMap<HookEvent, Vec<HookMatcher>>,
    /// Pending tool uses awaiting results (`tool_use_id` -> info)
    pending_tools: HashMap<String, PendingToolUse>,
    /// Session context for constructing hook inputs
    session_id: Option<String>,
    /// Current working directory
    cwd: Option<String>,
    /// Cancellation token for aborting operations
    cancellation_token: Option<CancellationToken>,
    /// Current subagent context (`parent_tool_use_id` of active subagent)
    /// When set, we're processing messages from inside a subagent.
    /// When a message with `parent_tool_use_id=None` arrives after this is set,
    /// the subagent has completed and we trigger `SubagentStop`.
    current_subagent: Option<String>,
}

impl HookManager {
    /// Create a new hook manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            hooks_by_event: HashMap::new(),
            pending_tools: HashMap::new(),
            session_id: None,
            cwd: None,
            cancellation_token: None,
            current_subagent: None,
        }
    }

    /// Create from options hooks configuration
    ///
    /// This is the primary constructor matching TypeScript SDK pattern.
    #[must_use]
    pub fn from_hooks_config(config: HashMap<HookEvent, Vec<HookMatcher>>) -> Self {
        Self {
            hooks_by_event: config,
            pending_tools: HashMap::new(),
            session_id: None,
            cwd: None,
            cancellation_token: None,
            current_subagent: None,
        }
    }

    /// Set session context (called when system init message is received)
    pub fn set_session_context(&mut self, session_id: String, cwd: Option<String>) {
        self.session_id = Some(session_id);
        self.cwd = cwd;
    }

    /// Set cancellation token for aborting operations
    pub fn set_cancellation_token(&mut self, token: CancellationToken) {
        self.cancellation_token = Some(token);
    }

    /// Build a `HookContext` with current session information
    #[must_use]
    pub fn build_context(&self) -> HookContext {
        HookContext::new(
            self.session_id.clone(),
            self.cwd.clone(),
            self.cancellation_token.clone(),
        )
    }

    /// Register a hook matcher for a specific event type
    pub fn register_for_event(&mut self, event: HookEvent, matcher: HookMatcher) {
        self.hooks_by_event.entry(event).or_default().push(matcher);
    }

    /// Register a hook with a matcher (legacy API - registers for all events)
    #[deprecated(note = "Use register_for_event() instead")]
    pub fn register(&mut self, matcher: HookMatcher) {
        // For backward compatibility, register for PreToolUse
        self.register_for_event(HookEvent::PreToolUse, matcher);
    }

    /// Default timeout for hook callbacks (60 seconds, matching TypeScript SDK)
    pub const DEFAULT_HOOK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

    /// Invoke hooks for a specific event type
    ///
    /// # Arguments
    /// * `event` - The hook event type
    /// * `event_data` - Event data (JSON value matching `HookInput` structure)
    /// * `tool_name` - Optional tool name for filtering
    /// * `context` - Hook context
    ///
    /// # Returns
    /// Combined hook output from all matching hooks
    ///
    /// # Timeout Behavior
    /// Each hook matcher has a configurable timeout (default: 60 seconds).
    /// If a hook exceeds its timeout, it will be cancelled and a default
    /// `HookOutput` will be used, allowing the agent to continue.
    ///
    /// # Errors
    ///
    /// Propagates errors from hook callback execution.
    pub async fn invoke(
        &self,
        event: HookEvent,
        event_data: serde_json::Value,
        tool_name: Option<String>,
        context: HookContext,
    ) -> Result<HookOutput> {
        let mut output = HookOutput::default();

        // Get hooks for this event type
        let Some(matchers) = self.hooks_by_event.get(&event) else {
            return Ok(output);
        };

        // Find matching hooks
        for matcher in matchers {
            if Self::matches(matcher.matcher.as_ref(), tool_name.as_ref()) {
                let timeout = matcher.timeout.unwrap_or(Self::DEFAULT_HOOK_TIMEOUT);

                // Invoke each hook callback with timeout
                for hook in &matcher.hooks {
                    let hook_future =
                        hook.call(event_data.clone(), tool_name.clone(), context.clone());

                    let result = match tokio::time::timeout(timeout, hook_future).await {
                        Ok(hook_result) => hook_result?,
                        Err(_elapsed) => {
                            // Hook timed out - log warning and continue with default output
                            tracing::warn!(
                                event = ?event,
                                tool_name = ?tool_name,
                                timeout_secs = timeout.as_secs(),
                                "Hook callback timed out, continuing with default output"
                            );

                            // Continue with default output (don't block the agent)
                            HookOutput::default()
                        }
                    };

                    // Merge hook results
                    if result.decision.is_some() {
                        output.decision = result.decision;
                    }
                    if result.system_message.is_some() {
                        output.system_message = result.system_message;
                    }
                    if result.hook_specific_output.is_some() {
                        output.hook_specific_output = result.hook_specific_output;
                    }

                    // If decision is Block, stop processing
                    if matches!(output.decision, Some(HookDecision::Block)) {
                        return Ok(output);
                    }
                }
            }
        }

        Ok(output)
    }

    // ========================================================================
    // Message Processing
    // ========================================================================

    /// Process a message and invoke applicable hooks
    ///
    /// This method extracts hook events from messages and invokes registered
    /// hooks. It tracks tool uses to match them with results.
    ///
    /// # Events extracted:
    /// - `ToolUse` in Assistant message → `PreToolUse` (+ `SubagentStart` for Task)
    /// - `ToolResult` in User message → `PostToolUse` or `PostToolUseFailure` (+ `SubagentStop` for Task)
    /// - `Result` message → captures `session_id`
    ///
    /// # Returns
    /// Vector of hook outputs from all invoked hooks
    ///
    /// # Errors
    ///
    /// Propagates errors from hook invocation.
    #[allow(clippy::too_many_lines)]
    pub async fn process_message(&mut self, msg: &Message) -> Result<Vec<HookOutput>> {
        let mut outputs = Vec::new();
        let context = self.build_context();

        // Extract parent_tool_use_id from message for subagent tracking
        let parent_tool_use_id = match msg {
            Message::Assistant {
                parent_tool_use_id, ..
            }
            | Message::User {
                parent_tool_use_id, ..
            } => parent_tool_use_id.clone(),
            _ => None,
        };

        // Check for SubagentStop: if we were inside a subagent and now we're not
        if let Some(ref current) = self.current_subagent {
            if parent_tool_use_id.is_none() {
                // We've exited subagent context - trigger SubagentStop
                let agent_id = current.clone();

                let subagent_input = serde_json::json!({
                    "hook_event_name": "SubagentStop",
                    "session_id": self.session_id.as_deref().unwrap_or(""),
                    "cwd": self.cwd.as_deref().unwrap_or(""),
                    "transcript_path": "",
                    "stop_hook_active": false,
                    "agent_id": agent_id,
                    "agent_transcript_path": "",
                });

                let output = self
                    .invoke(
                        HookEvent::SubagentStop,
                        subagent_input,
                        Some("Task".to_string()),
                        context.clone(),
                    )
                    .await?;

                if output.decision.is_some()
                    || output.system_message.is_some()
                    || output.hook_specific_output.is_some()
                {
                    outputs.push(output);
                }

                // Clear subagent context and remove from pending
                self.pending_tools.remove(&agent_id);
                self.current_subagent = None;
            }
        }

        // Update subagent context tracking
        if let Some(ref ptui) = parent_tool_use_id {
            // Check if this is from a Task tool (subagent)
            if self.pending_tools.get(ptui).is_some_and(|p| p.is_task) {
                self.current_subagent = Some(ptui.clone());
            }
        }

        match msg {
            Message::Assistant { message, .. } => {
                // Extract ToolUse blocks
                for block in &message.content {
                    if let ContentBlock::ToolUse { id, name, input } = block {
                        // Track for later PostToolUse matching
                        self.pending_tools.insert(
                            id.clone(),
                            PendingToolUse {
                                tool_name: name.clone(),
                                tool_input: input.clone(),
                                is_task: name == "Task",
                            },
                        );

                        // Build PreToolUse input
                        let hook_input = serde_json::json!({
                            "hook_event_name": "PreToolUse",
                            "session_id": self.session_id.as_deref().unwrap_or(""),
                            "cwd": self.cwd.as_deref().unwrap_or(""),
                            "transcript_path": "",
                            "tool_name": name,
                            "tool_input": input,
                        });

                        // Invoke PreToolUse hooks
                        let output = self
                            .invoke(
                                HookEvent::PreToolUse,
                                hook_input,
                                Some(name.clone()),
                                context.clone(),
                            )
                            .await?;

                        if output.decision.is_some()
                            || output.system_message.is_some()
                            || output.hook_specific_output.is_some()
                        {
                            outputs.push(output);
                        }

                        // For Task tool, also invoke SubagentStart
                        if name == "Task" {
                            let subagent_input = serde_json::json!({
                                "hook_event_name": "SubagentStart",
                                "session_id": self.session_id.as_deref().unwrap_or(""),
                                "cwd": self.cwd.as_deref().unwrap_or(""),
                                "transcript_path": "",
                                "agent_id": id,
                                "agent_type": input.get("subagent_type").and_then(|v| v.as_str()).unwrap_or("unknown"),
                            });

                            let output = self
                                .invoke(
                                    HookEvent::SubagentStart,
                                    subagent_input,
                                    Some(name.clone()),
                                    context.clone(),
                                )
                                .await?;

                            if output.decision.is_some()
                                || output.system_message.is_some()
                                || output.hook_specific_output.is_some()
                            {
                                outputs.push(output);
                            }
                        }
                    }
                }
            }

            Message::User { message, .. } => {
                // Check for ToolResult in content blocks
                if let Some(crate::types::UserContent::Blocks(blocks)) = &message.content {
                    for block in blocks {
                        if let ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            is_error,
                        } = block
                        {
                            // Look up pending tool use
                            if let Some(pending) = self.pending_tools.remove(tool_use_id) {
                                let is_failure = is_error.unwrap_or(false);

                                if is_failure {
                                    // PostToolUseFailure
                                    let hook_input = serde_json::json!({
                                        "hook_event_name": "PostToolUseFailure",
                                        "session_id": self.session_id.as_deref().unwrap_or(""),
                                        "cwd": self.cwd.as_deref().unwrap_or(""),
                                        "transcript_path": "",
                                        "tool_name": pending.tool_name,
                                        "tool_input": pending.tool_input,
                                        "tool_use_id": tool_use_id,
                                        "error": content_to_string(content.as_ref()),
                                    });

                                    let output = self
                                        .invoke(
                                            HookEvent::PostToolUseFailure,
                                            hook_input,
                                            Some(pending.tool_name.clone()),
                                            context.clone(),
                                        )
                                        .await?;

                                    if output.decision.is_some()
                                        || output.system_message.is_some()
                                        || output.hook_specific_output.is_some()
                                    {
                                        outputs.push(output);
                                    }
                                } else {
                                    // PostToolUse
                                    let hook_input = serde_json::json!({
                                        "hook_event_name": "PostToolUse",
                                        "session_id": self.session_id.as_deref().unwrap_or(""),
                                        "cwd": self.cwd.as_deref().unwrap_or(""),
                                        "transcript_path": "",
                                        "tool_name": pending.tool_name,
                                        "tool_input": pending.tool_input,
                                        "tool_response": content,
                                        "tool_use_id": tool_use_id,
                                    });

                                    let output = self
                                        .invoke(
                                            HookEvent::PostToolUse,
                                            hook_input,
                                            Some(pending.tool_name.clone()),
                                            context.clone(),
                                        )
                                        .await?;

                                    if output.decision.is_some()
                                        || output.system_message.is_some()
                                        || output.hook_specific_output.is_some()
                                    {
                                        outputs.push(output);
                                    }

                                    // For Task tool, also invoke SubagentStop
                                    if pending.is_task {
                                        let subagent_input = serde_json::json!({
                                            "hook_event_name": "SubagentStop",
                                            "session_id": self.session_id.as_deref().unwrap_or(""),
                                            "cwd": self.cwd.as_deref().unwrap_or(""),
                                            "transcript_path": "",
                                            "stop_hook_active": false,
                                            "agent_id": tool_use_id,
                                            "agent_transcript_path": "",
                                        });

                                        let output = self
                                            .invoke(
                                                HookEvent::SubagentStop,
                                                subagent_input,
                                                Some(pending.tool_name.clone()),
                                                context.clone(),
                                            )
                                            .await?;

                                        if output.decision.is_some()
                                            || output.system_message.is_some()
                                            || output.hook_specific_output.is_some()
                                        {
                                            outputs.push(output);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Message::Result { session_id, .. } => {
                // Capture session_id for context
                self.session_id = Some(session_id.to_string());

                // Trigger Stop hook
                let stop_input = serde_json::json!({
                    "hook_event_name": "Stop",
                    "session_id": self.session_id.as_deref().unwrap_or(""),
                    "cwd": self.cwd.as_deref().unwrap_or(""),
                    "transcript_path": "",
                    "stop_hook_active": false,
                });

                let output = self
                    .invoke(HookEvent::Stop, stop_input, None, context.clone())
                    .await?;

                if output.decision.is_some()
                    || output.system_message.is_some()
                    || output.hook_specific_output.is_some()
                {
                    outputs.push(output);
                }
            }

            Message::System { subtype, data } => {
                // Handle compact_boundary for PreCompact hook
                if subtype == "compact_boundary" {
                    let trigger = data
                        .get("compact_metadata")
                        .and_then(|m| m.get("trigger"))
                        .and_then(|t| t.as_str())
                        .unwrap_or("auto");

                    let precompact_input = serde_json::json!({
                        "hook_event_name": "PreCompact",
                        "session_id": self.session_id.as_deref().unwrap_or(""),
                        "cwd": self.cwd.as_deref().unwrap_or(""),
                        "transcript_path": "",
                        "trigger": trigger,
                        "custom_instructions": null,
                    });

                    let output = self
                        .invoke(
                            HookEvent::PreCompact,
                            precompact_input,
                            None,
                            context.clone(),
                        )
                        .await?;

                    if output.decision.is_some()
                        || output.system_message.is_some()
                        || output.hook_specific_output.is_some()
                    {
                        outputs.push(output);
                    }
                }
            }

            Message::StreamEvent { .. } => {}
        }

        Ok(outputs)
    }

    /// Check if there are any hooks registered for a given event
    #[must_use]
    pub fn has_hooks_for(&self, event: HookEvent) -> bool {
        self.hooks_by_event
            .get(&event)
            .is_some_and(|v| !v.is_empty())
    }

    // ========================================================================
    // Explicit Hook Triggers
    // ========================================================================

    /// Trigger `SessionStart` hook
    ///
    /// Called when the session is initialized (after receiving system init message).
    ///
    /// # Arguments
    /// * `source` - Source of session start: "startup", "resume", "clear", or "compact"
    ///
    /// # Errors
    ///
    /// Propagates errors from hook callback execution.
    pub async fn trigger_session_start(&self, source: &str) -> Result<Vec<HookOutput>> {
        let mut outputs = Vec::new();
        let context = self.build_context();

        let session_start_input = serde_json::json!({
            "hook_event_name": "SessionStart",
            "session_id": self.session_id.as_deref().unwrap_or(""),
            "cwd": self.cwd.as_deref().unwrap_or(""),
            "transcript_path": "",
            "source": source,
        });

        let output = self
            .invoke(HookEvent::SessionStart, session_start_input, None, context)
            .await?;

        if output.decision.is_some()
            || output.system_message.is_some()
            || output.hook_specific_output.is_some()
        {
            outputs.push(output);
        }

        Ok(outputs)
    }

    /// Trigger `SessionEnd` hook
    ///
    /// Called when the session ends.
    ///
    /// # Arguments
    /// * `reason` - Reason for session end: "clear", "logout", "`prompt_input_exit`", or "other"
    ///
    /// # Errors
    ///
    /// Propagates errors from hook callback execution.
    pub async fn trigger_session_end(&self, reason: &str) -> Result<Vec<HookOutput>> {
        let mut outputs = Vec::new();
        let context = self.build_context();

        let session_end_input = serde_json::json!({
            "hook_event_name": "SessionEnd",
            "session_id": self.session_id.as_deref().unwrap_or(""),
            "cwd": self.cwd.as_deref().unwrap_or(""),
            "transcript_path": "",
            "reason": reason,
        });

        let output = self
            .invoke(HookEvent::SessionEnd, session_end_input, None, context)
            .await?;

        if output.decision.is_some()
            || output.system_message.is_some()
            || output.hook_specific_output.is_some()
        {
            outputs.push(output);
        }

        Ok(outputs)
    }

    /// Trigger `UserPromptSubmit` hook
    ///
    /// Called before a user message is sent to the agent.
    ///
    /// # Arguments
    /// * `prompt` - The user's prompt text
    ///
    /// # Returns
    /// Hook outputs that may contain `additionalContext` in `hook_specific_output`
    ///
    /// # Errors
    ///
    /// Propagates errors from hook callback execution.
    pub async fn trigger_user_prompt_submit(&self, prompt: &str) -> Result<Vec<HookOutput>> {
        let mut outputs = Vec::new();
        let context = self.build_context();

        let user_prompt_input = serde_json::json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": self.session_id.as_deref().unwrap_or(""),
            "cwd": self.cwd.as_deref().unwrap_or(""),
            "transcript_path": "",
            "prompt": prompt,
        });

        let output = self
            .invoke(
                HookEvent::UserPromptSubmit,
                user_prompt_input,
                None,
                context,
            )
            .await?;

        if output.decision.is_some()
            || output.system_message.is_some()
            || output.hook_specific_output.is_some()
        {
            outputs.push(output);
        }

        Ok(outputs)
    }

    /// Trigger Notification hook
    ///
    /// Called when a notification is received.
    ///
    /// # Arguments
    /// * `message` - The notification message
    /// * `title` - Optional notification title
    ///
    /// # Errors
    ///
    /// Propagates errors from hook callback execution.
    pub async fn trigger_notification(
        &self,
        message: &str,
        title: Option<&str>,
    ) -> Result<Vec<HookOutput>> {
        let mut outputs = Vec::new();
        let context = self.build_context();

        let notification_input = serde_json::json!({
            "hook_event_name": "Notification",
            "session_id": self.session_id.as_deref().unwrap_or(""),
            "cwd": self.cwd.as_deref().unwrap_or(""),
            "transcript_path": "",
            "message": message,
            "title": title,
        });

        let output = self
            .invoke(HookEvent::Notification, notification_input, None, context)
            .await?;

        if output.decision.is_some()
            || output.system_message.is_some()
            || output.hook_specific_output.is_some()
        {
            outputs.push(output);
        }

        Ok(outputs)
    }

    /// Trigger `PermissionRequest` hook
    ///
    /// Called when a permission is requested for a tool.
    ///
    /// # Arguments
    /// * `tool_name` - Name of the tool requesting permission
    /// * `tool_input` - The tool's input parameters
    /// * `suggestions` - Optional permission suggestions
    ///
    /// # Errors
    ///
    /// Propagates errors from hook callback execution.
    pub async fn trigger_permission_request(
        &self,
        tool_name: &str,
        tool_input: serde_json::Value,
        suggestions: Option<Vec<serde_json::Value>>,
    ) -> Result<Vec<HookOutput>> {
        let mut outputs = Vec::new();
        let context = self.build_context();

        let permission_input = serde_json::json!({
            "hook_event_name": "PermissionRequest",
            "session_id": self.session_id.as_deref().unwrap_or(""),
            "cwd": self.cwd.as_deref().unwrap_or(""),
            "transcript_path": "",
            "tool_name": tool_name,
            "tool_input": tool_input,
            "permission_suggestions": suggestions,
        });

        let output = self
            .invoke(
                HookEvent::PermissionRequest,
                permission_input,
                Some(tool_name.to_string()),
                context,
            )
            .await?;

        if output.decision.is_some()
            || output.system_message.is_some()
            || output.hook_specific_output.is_some()
        {
            outputs.push(output);
        }

        Ok(outputs)
    }

    // ========================================================================
    // Pattern Matching
    // ========================================================================

    /// Check if a matcher matches a tool name
    ///
    /// # Security Note
    /// This uses simple pattern matching with pipe-separated alternatives.
    /// For production use with untrusted patterns, consider using a proper
    /// glob or regex library with safety guarantees (e.g., `globset` crate).
    fn matches(matcher: Option<&String>, tool_name: Option<&String>) -> bool {
        match (matcher, tool_name) {
            (None, _) => true, // No matcher = match all
            (Some(pattern), Some(name)) => {
                // Simple wildcard matching
                if pattern == "*" {
                    return true;
                }
                // Exact match or simple pipe-separated pattern
                // Note: This doesn't handle edge cases like pipe characters in tool names
                pattern == name || pattern.split('|').any(|p| p == name)
            }
            (Some(_), None) => false,
        }
    }

    // ========================================================================
    // Callback Helpers
    // ========================================================================

    /// Create a hook callback from a closure
    ///
    /// # Example
    ///
    /// ```no_run
    /// use anthropic_agent_sdk::{HookManager, HookOutput, HookContext};
    ///
    /// let hook = HookManager::callback(|_input, tool_name, ctx| async move {
    ///     // Check cancellation and access session info
    ///     if ctx.is_cancelled() { return Ok(HookOutput::default()); }
    ///     println!("Tool: {:?}, Session: {:?}", tool_name, ctx.session_id);
    ///     Ok(HookOutput::default())
    /// });
    /// ```
    pub fn callback<F, Fut>(f: F) -> Arc<dyn HookCallback>
    where
        F: Fn(serde_json::Value, Option<String>, HookContext) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<HookOutput>> + Send + 'static,
    {
        Arc::new(FnHookCallback::new(
            move |event_data, tool_name, context| Box::pin(f(event_data, tool_name, context)),
        ))
    }
}

impl Default for HookManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating hook matchers
pub struct HookMatcherBuilder {
    matcher: Option<String>,
    hooks: Vec<Arc<dyn HookCallback>>,
    timeout: Option<std::time::Duration>,
}

impl HookMatcherBuilder {
    /// Create a new hook matcher builder
    ///
    /// # Arguments
    /// * `pattern` - Matcher pattern (None for all, or specific tool name/pattern)
    pub fn new(pattern: Option<impl Into<String>>) -> Self {
        Self {
            matcher: pattern.map(std::convert::Into::into),
            hooks: Vec::new(),
            timeout: None,
        }
    }

    /// Add a hook callback
    #[must_use]
    pub fn add_hook(mut self, hook: Arc<dyn HookCallback>) -> Self {
        self.hooks.push(hook);
        self
    }

    /// Set timeout for all hooks in this matcher
    ///
    /// Default is 60 seconds if not specified.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use anthropic_agent_sdk::hooks::HookMatcherBuilder;
    /// use std::time::Duration;
    ///
    /// let matcher = HookMatcherBuilder::new(Some("Bash"))
    ///     .timeout(Duration::from_secs(30))
    ///     .build();
    /// ```
    #[must_use]
    pub fn timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the hook matcher
    #[must_use]
    pub fn build(self) -> HookMatcher {
        HookMatcher {
            matcher: self.matcher,
            hooks: self.hooks,
            timeout: self.timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hook_manager() {
        let mut manager = HookManager::new();

        // Register a hook for PreToolUse
        let hook = HookManager::callback(|_event_data, _tool_name, _context| async {
            Ok(HookOutput::default())
        });

        let matcher = HookMatcherBuilder::new(Some("*")).add_hook(hook).build();
        manager.register_for_event(HookEvent::PreToolUse, matcher);

        // Invoke hook
        let result = manager
            .invoke(
                HookEvent::PreToolUse,
                serde_json::json!({}),
                Some("test".to_string()),
                HookContext::default(),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hook_manager_by_event() {
        let mut manager = HookManager::new();

        // Register hooks for different events - ctx provides session info
        let pre_hook = HookManager::callback(|_data, _tool, ctx| async move {
            // Verify ctx fields are accessible (use references to avoid move)
            let _ = (&ctx.session_id, &ctx.cwd, ctx.is_cancelled());
            Ok(HookOutput {
                system_message: Some("pre".to_string()),
                ..Default::default()
            })
        });
        let post_hook = HookManager::callback(|_data, _tool, ctx| async move {
            let _ = &ctx.cancellation_token;
            Ok(HookOutput {
                system_message: Some("post".to_string()),
                ..Default::default()
            })
        });

        manager.register_for_event(
            HookEvent::PreToolUse,
            HookMatcherBuilder::new(None::<String>)
                .add_hook(pre_hook)
                .build(),
        );
        manager.register_for_event(
            HookEvent::PostToolUse,
            HookMatcherBuilder::new(None::<String>)
                .add_hook(post_hook)
                .build(),
        );

        // PreToolUse should return "pre"
        let result = manager
            .invoke(
                HookEvent::PreToolUse,
                serde_json::json!({}),
                None,
                HookContext::default(),
            )
            .await
            .unwrap();
        assert_eq!(result.system_message, Some("pre".to_string()));

        // PostToolUse should return "post"
        let result = manager
            .invoke(
                HookEvent::PostToolUse,
                serde_json::json!({}),
                None,
                HookContext::default(),
            )
            .await
            .unwrap();
        assert_eq!(result.system_message, Some("post".to_string()));

        // SubagentStart should return empty (no hooks registered)
        let result = manager
            .invoke(
                HookEvent::SubagentStart,
                serde_json::json!({}),
                None,
                HookContext::default(),
            )
            .await
            .unwrap();
        assert!(result.system_message.is_none());
    }

    #[test]
    fn test_matcher_wildcard() {
        assert!(HookManager::matches(
            Some("*".to_string()).as_ref(),
            Some("any_tool".to_string()).as_ref()
        ));
        assert!(HookManager::matches(
            None,
            Some("any_tool".to_string()).as_ref()
        ));
    }

    #[test]
    fn test_matcher_specific() {
        assert!(HookManager::matches(
            Some("Bash".to_string()).as_ref(),
            Some("Bash".to_string()).as_ref()
        ));
        assert!(!HookManager::matches(
            Some("Bash".to_string()).as_ref(),
            Some("Write".to_string()).as_ref()
        ));
    }

    #[test]
    fn test_matcher_pattern() {
        assert!(HookManager::matches(
            Some("Write|Edit".to_string()).as_ref(),
            Some("Write".to_string()).as_ref()
        ));
        assert!(HookManager::matches(
            Some("Write|Edit".to_string()).as_ref(),
            Some("Edit".to_string()).as_ref()
        ));
        assert!(!HookManager::matches(
            Some("Write|Edit".to_string()).as_ref(),
            Some("Bash".to_string()).as_ref()
        ));
    }

    // ========================================================================
    // Security: Timeout Tests
    // ========================================================================

    #[tokio::test]
    async fn test_hook_timeout_prevents_blocking() {
        let mut manager = HookManager::new();

        // Create a hook that would block forever without timeout
        let slow_hook = HookManager::callback(|_data, _tool, _ctx| async move {
            // Simulate a very slow operation
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            Ok(HookOutput {
                system_message: Some("should never see this".to_string()),
                ..Default::default()
            })
        });

        // Register with a very short timeout (100ms)
        let matcher = HookMatcherBuilder::new(Some("*"))
            .timeout(std::time::Duration::from_millis(100))
            .add_hook(slow_hook)
            .build();
        manager.register_for_event(HookEvent::PreToolUse, matcher);

        // Should complete quickly (timeout) and return default output
        let start = std::time::Instant::now();
        let result = manager
            .invoke(
                HookEvent::PreToolUse,
                serde_json::json!({}),
                Some("test".to_string()),
                HookContext::default(),
            )
            .await;

        let elapsed = start.elapsed();

        // Verify timeout was enforced (should be ~100ms, not 10s)
        assert!(elapsed < std::time::Duration::from_secs(1));
        assert!(result.is_ok());

        // Timed-out hook returns default output (no system_message)
        let output = result.unwrap();
        assert!(output.system_message.is_none());
        assert!(output.decision.is_none());
    }

    #[tokio::test]
    async fn test_hook_custom_timeout_respected() {
        let mut manager = HookManager::new();

        // Hook that takes 200ms
        let medium_hook = HookManager::callback(|_data, _tool, _ctx| async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            Ok(HookOutput {
                system_message: Some("completed".to_string()),
                ..Default::default()
            })
        });

        // Register with 500ms timeout - should complete
        let matcher = HookMatcherBuilder::new(Some("*"))
            .timeout(std::time::Duration::from_millis(500))
            .add_hook(medium_hook)
            .build();
        manager.register_for_event(HookEvent::PreToolUse, matcher);

        let result = manager
            .invoke(
                HookEvent::PreToolUse,
                serde_json::json!({}),
                Some("test".to_string()),
                HookContext::default(),
            )
            .await
            .unwrap();

        // Hook should complete within timeout
        assert_eq!(result.system_message, Some("completed".to_string()));
    }

    #[tokio::test]
    async fn test_hook_default_timeout_is_60_seconds() {
        // Verify the constant is set correctly
        assert_eq!(
            HookManager::DEFAULT_HOOK_TIMEOUT,
            std::time::Duration::from_secs(60)
        );

        // Verify builder without timeout uses None (which means default)
        let matcher = HookMatcherBuilder::new(Some("*")).build();
        assert!(matcher.timeout.is_none());
    }

    #[tokio::test]
    async fn test_fast_hook_unaffected_by_timeout() {
        let mut manager = HookManager::new();

        // Fast hook that completes immediately
        let fast_hook = HookManager::callback(|_data, _tool, _ctx| async move {
            Ok(HookOutput {
                system_message: Some("fast".to_string()),
                ..Default::default()
            })
        });

        // Even with short timeout, fast hook completes
        let matcher = HookMatcherBuilder::new(Some("*"))
            .timeout(std::time::Duration::from_millis(100))
            .add_hook(fast_hook)
            .build();
        manager.register_for_event(HookEvent::PreToolUse, matcher);

        let result = manager
            .invoke(
                HookEvent::PreToolUse,
                serde_json::json!({}),
                Some("test".to_string()),
                HookContext::default(),
            )
            .await
            .unwrap();

        assert_eq!(result.system_message, Some("fast".to_string()));
    }

    // ========================================================================
    // Security: Cancellation Token Tests
    // ========================================================================

    #[tokio::test]
    async fn test_hook_context_cancellation() {
        let token = CancellationToken::new();
        let ctx = HookContext::new(
            Some("session-1".to_string()),
            Some("/tmp".to_string()),
            Some(token.clone()),
        );

        // Initially not cancelled
        assert!(!ctx.is_cancelled());

        // Cancel the token
        token.cancel();

        // Now should be cancelled
        assert!(ctx.is_cancelled());
    }

    #[tokio::test]
    async fn test_hook_receives_cancellation_token() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let mut manager = HookManager::new();
        let was_cancelled = Arc::new(AtomicBool::new(false));
        let was_cancelled_clone = was_cancelled.clone();

        // Hook that checks cancellation status
        let hook = HookManager::callback(move |_data, _tool, ctx| {
            let was_cancelled = was_cancelled_clone.clone();
            async move {
                // Store whether we received a cancellation token
                if ctx.cancellation_token.is_some() {
                    was_cancelled.store(true, Ordering::SeqCst);
                }
                Ok(HookOutput::default())
            }
        });

        let matcher = HookMatcherBuilder::new(Some("*")).add_hook(hook).build();
        manager.register_for_event(HookEvent::PreToolUse, matcher);

        // Invoke with cancellation token
        let token = CancellationToken::new();
        let ctx = HookContext::new(None, None, Some(token));

        let _ = manager
            .invoke(
                HookEvent::PreToolUse,
                serde_json::json!({}),
                Some("test".to_string()),
                ctx,
            )
            .await;

        assert!(was_cancelled.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_hook_context_default_has_no_token() {
        let ctx = HookContext::default();
        assert!(ctx.cancellation_token.is_none());
        assert!(!ctx.is_cancelled()); // No token means not cancelled
    }
}
