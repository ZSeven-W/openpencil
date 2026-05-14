// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! Session management for the Copilot SDK.
//!
//! A session represents a conversation with the Copilot CLI.

use crate::error::{CopilotError, Result};
use crate::events::{SessionEvent, SessionEventData};
use crate::types::{
    AgentInfo, ErrorOccurredHookInput, FleetStartOptions, LogOptions, LogResult, MessageOptions,
    PermissionRequest, PermissionRequestResult, PlanData, PostToolUseHookInput,
    PreToolUseHookInput, SessionEndHookInput, SessionHooks, SessionMode, SessionStartHookInput,
    SetModelOptions, ShellExecOptions, ShellExecResult, ShellSignal, Tool, ToolResultObject,
    UserInputInvocation, UserInputRequest, UserInputResponse, UserPromptSubmittedHookInput,
    WorkspaceFile,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};

// =============================================================================
// Event Handler Types
// =============================================================================

/// Handler for session events.
pub type EventHandler = Arc<dyn Fn(&SessionEvent) + Send + Sync>;

/// Handler for permission requests.
pub type PermissionHandler =
    Arc<dyn Fn(&PermissionRequest) -> PermissionRequestResult + Send + Sync>;

/// Handler for tool invocations.
pub type ToolHandler = Arc<dyn Fn(&str, &Value) -> ToolResultObject + Send + Sync>;

/// Handler for user input requests.
pub type UserInputHandler =
    Arc<dyn Fn(&UserInputRequest, &UserInputInvocation) -> UserInputResponse + Send + Sync>;

/// Type alias for the invoke future.
pub type InvokeFuture = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send>>;

type InvokeFn = dyn Fn(&str, Option<Value>) -> InvokeFuture + Send + Sync;

// =============================================================================
// Event Subscription
// =============================================================================

/// A subscription to session events.
///
/// Events are delivered via the broadcast channel receiver.
pub struct EventSubscription {
    pub receiver: broadcast::Receiver<SessionEvent>,
}

impl EventSubscription {
    /// Receive the next event.
    pub async fn recv(&mut self) -> std::result::Result<SessionEvent, broadcast::error::RecvError> {
        self.receiver.recv().await
    }
}

// =============================================================================
// Registered Tool
// =============================================================================

/// A tool registered with the session, including its handler.
#[derive(Clone)]
pub struct RegisteredTool {
    /// Tool definition.
    pub tool: Tool,
    /// Handler for tool invocations.
    pub handler: Option<ToolHandler>,
}

// =============================================================================
// Session
// =============================================================================

/// Shared session state.
struct SessionState {
    /// Registered tools.
    tools: HashMap<String, RegisteredTool>,
    /// Permission handler.
    permission_handler: Option<PermissionHandler>,
    /// User input handler.
    user_input_handler: Option<UserInputHandler>,
    /// Session hooks.
    hooks: Option<SessionHooks>,
    /// Callback-based event handlers.
    event_handlers: HashMap<u64, EventHandler>,
    /// Next handler ID.
    next_handler_id: AtomicU64,
}

/// A Copilot conversation session.
///
/// Sessions maintain conversation state, handle events, and manage tool execution.
///
/// # Example
///
/// ```no_run
/// use copilot_sdk::{Client, SessionConfig, SessionEventData};
///
/// #[tokio::main]
/// async fn main() -> copilot_sdk::Result<()> {
/// let client = Client::builder().build()?;
/// client.start().await?;
/// let session = client.create_session(SessionConfig::default()).await?;
///
/// // Subscribe to events
/// let mut events = session.subscribe();
///
/// // Send a message
/// session.send("Hello!").await?;
///
/// // Process events
/// while let Ok(event) = events.recv().await {
///     match &event.data {
///         SessionEventData::AssistantMessage(msg) => println!("{}", msg.content),
///         SessionEventData::SessionIdle(_) => break,
///         _ => {}
///     }
/// }
/// client.stop().await;
/// # Ok(())
/// # }
/// ```
pub struct Session {
    /// Session ID.
    session_id: String,
    /// Workspace path for infinite sessions.
    workspace_path: Option<String>,
    /// Event broadcaster.
    event_tx: broadcast::Sender<SessionEvent>,
    /// Session state.
    state: Arc<RwLock<SessionState>>,
    /// JSON-RPC invoke function (injected by Client).
    invoke_fn: Arc<InvokeFn>,
}

impl Session {
    /// Create a new session.
    ///
    /// This is typically called by the Client when creating a session.
    pub fn new<F>(session_id: String, workspace_path: Option<String>, invoke_fn: F) -> Self
    where
        F: Fn(&str, Option<Value>) -> InvokeFuture + Send + Sync + 'static,
    {
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            session_id,
            workspace_path,
            event_tx,
            state: Arc::new(RwLock::new(SessionState {
                tools: HashMap::new(),
                permission_handler: None,
                user_input_handler: None,
                hooks: None,
                event_handlers: HashMap::new(),
                next_handler_id: AtomicU64::new(1),
            })),
            invoke_fn: Arc::new(invoke_fn),
        }
    }

    // =========================================================================
    // Session Properties
    // =========================================================================

    /// Get the session ID.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the workspace path for infinite sessions.
    ///
    /// Contains checkpoints/, plan.md, and files/ subdirectories.
    /// Returns None if infinite sessions are disabled.
    pub fn workspace_path(&self) -> Option<&str> {
        self.workspace_path.as_deref()
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    /// Subscribe to session events.
    ///
    /// Returns a receiver that will receive all session events.
    pub fn subscribe(&self) -> EventSubscription {
        EventSubscription {
            receiver: self.event_tx.subscribe(),
        }
    }

    /// Register a callback-based event handler.
    ///
    /// Returns an unsubscribe closure. Call it to remove the handler.
    /// Alternatively, use [`off`] with the internal handler ID.
    pub async fn on<F>(&self, handler: F) -> impl FnOnce()
    where
        F: Fn(&SessionEvent) + Send + Sync + 'static,
    {
        let mut state = self.state.write().await;
        let id = state.next_handler_id.fetch_add(1, Ordering::SeqCst);
        state.event_handlers.insert(id, Arc::new(handler));

        let state_ref = Arc::clone(&self.state);
        move || {
            tokio::spawn(async move {
                state_ref.write().await.event_handlers.remove(&id);
            });
        }
    }

    /// Unsubscribe a callback-based event handler.
    pub async fn off(&self, handler_id: u64) {
        let mut state = self.state.write().await;
        state.event_handlers.remove(&handler_id);
    }

    /// Dispatch an event to all subscribers.
    ///
    /// Broadcast request events (external_tool.requested, permission.requested) are handled
    /// internally before being forwarded to user handlers (protocol v3 model).
    ///
    /// This is called by the Client when events are received.
    pub async fn dispatch_event(&self, event: SessionEvent) {
        // Handle broadcast request events (protocol v3) before dispatching to user handlers.
        // Fire-and-forget: the response is sent asynchronously via RPC.
        self.handle_broadcast_event(&event).await;

        // Send to broadcast channel
        let _ = self.event_tx.send(event.clone());

        // Call registered handlers
        let state = self.state.read().await;
        for handler in state.event_handlers.values() {
            handler(&event);
        }
    }

    /// Handle broadcast request events by executing local handlers and responding via RPC.
    ///
    /// Implements the protocol v3 broadcast model where tool calls and permission requests
    /// are broadcast as session events to all clients.
    async fn handle_broadcast_event(&self, event: &SessionEvent) {
        match &event.data {
            SessionEventData::ExternalToolRequested(data) => {
                let request_id = match &data.request_id {
                    Some(id) => id.clone(),
                    None => return,
                };
                let tool_name = match &data.tool_name {
                    Some(name) => name.clone(),
                    None => return,
                };

                // Check if this session handles this tool
                if self.get_tool(&tool_name).await.is_none() {
                    return; // This client doesn't handle this tool; another client will.
                }

                let _tool_call_id = data.tool_call_id.clone().unwrap_or_default();
                let arguments = data.arguments.clone().unwrap_or(serde_json::json!({}));
                let session_id = self.session_id.clone();

                // Execute tool and respond via handlePendingToolCall RPC
                match self.invoke_tool(&tool_name, &arguments).await {
                    Ok(result) => {
                        // If the tool reported a failure with an error, send via top-level error
                        let params = if result.result_type == "failure"
                            || result.result_type == "error"
                        {
                            serde_json::json!({
                                "sessionId": session_id,
                                "requestId": request_id,
                                "error": result.error.unwrap_or_else(|| result.text_result_for_llm.clone()),
                            })
                        } else {
                            serde_json::json!({
                                "sessionId": session_id,
                                "requestId": request_id,
                                "result": {
                                    "textResultForLlm": result.text_result_for_llm,
                                    "resultType": result.result_type,
                                    "toolTelemetry": result.tool_telemetry.unwrap_or_default(),
                                }
                            })
                        };
                        let _ =
                            (self.invoke_fn)("session.tools.handlePendingToolCall", Some(params))
                                .await;
                    }
                    Err(e) => {
                        let params = serde_json::json!({
                            "sessionId": session_id,
                            "requestId": request_id,
                            "error": e.to_string(),
                        });
                        let _ =
                            (self.invoke_fn)("session.tools.handlePendingToolCall", Some(params))
                                .await;
                    }
                }
            }
            SessionEventData::PermissionRequested(data) => {
                let request_id = match &data.request_id {
                    Some(id) => id.clone(),
                    None => return,
                };
                let perm_data = match &data.permission_request {
                    Some(d) => d.clone(),
                    None => return,
                };

                let session_id = self.session_id.clone();

                // Build PermissionRequest from JSON
                use crate::types::PermissionRequest;
                let kind = perm_data
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let tool_call_id = perm_data
                    .get("toolCallId")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let mut extension_data = std::collections::HashMap::new();
                if let Some(obj) = perm_data.as_object() {
                    for (key, value) in obj {
                        if key != "kind" && key != "toolCallId" {
                            extension_data.insert(key.clone(), value.clone());
                        }
                    }
                }

                let request = PermissionRequest {
                    kind,
                    tool_call_id,
                    extension_data,
                };

                let result = self.handle_permission_request(&request).await;

                let mut perm_result_inner = serde_json::json!({
                    "kind": result.kind,
                });
                if let Some(rules) = &result.rules {
                    perm_result_inner["rules"] = serde_json::json!(rules);
                }
                let perm_result = serde_json::json!({
                    "sessionId": session_id,
                    "requestId": request_id,
                    "result": perm_result_inner,
                });

                let _ = (self.invoke_fn)(
                    "session.permissions.handlePendingPermissionRequest",
                    Some(perm_result),
                )
                .await;
            }
            _ => {} // Not a broadcast request event
        }
    }

    // =========================================================================
    // Messaging
    // =========================================================================

    /// Send a message to the session.
    ///
    /// Returns the message ID.
    pub async fn send(&self, options: impl Into<MessageOptions>) -> Result<String> {
        let options = options.into();
        let params = serde_json::json!({
            "sessionId": self.session_id,
            "prompt": options.prompt,
            "attachments": options.attachments,
            "mode": options.mode,
        });

        let result = (self.invoke_fn)("session.send", Some(params)).await?;

        result
            .get("messageId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| CopilotError::Protocol("Missing messageId in response".into()))
    }

    /// Abort the current message processing.
    pub async fn abort(&self) -> Result<()> {
        let params = serde_json::json!({
            "sessionId": self.session_id,
        });

        (self.invoke_fn)("session.abort", Some(params)).await?;
        Ok(())
    }

    /// Get all messages in the session.
    pub async fn get_messages(&self) -> Result<Vec<SessionEvent>> {
        let params = serde_json::json!({
            "sessionId": self.session_id,
        });

        let result = (self.invoke_fn)("session.getMessages", Some(params)).await?;

        let events: Vec<SessionEvent> = result
            .get("events")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| SessionEvent::from_json(v).ok())
                    .collect()
            })
            .or_else(|| {
                result
                    .get("messages")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| SessionEvent::from_json(v).ok())
                            .collect()
                    })
            })
            .ok_or_else(|| {
                CopilotError::Protocol("Missing events in getMessages response".into())
            })?;

        Ok(events)
    }

    // =========================================================================
    // Tool Management
    // =========================================================================

    /// Register a tool with this session.
    pub async fn register_tool(&self, tool: Tool) {
        self.register_tool_with_handler(tool, None).await;
    }

    /// Register a tool with a handler.
    pub async fn register_tool_with_handler(&self, tool: Tool, handler: Option<ToolHandler>) {
        let mut state = self.state.write().await;
        let name = tool.name.clone();
        state.tools.insert(name, RegisteredTool { tool, handler });
    }

    /// Register multiple tools.
    pub async fn register_tools(&self, tools: Vec<Tool>) {
        let mut state = self.state.write().await;
        for tool in tools {
            let name = tool.name.clone();
            state.tools.insert(
                name,
                RegisteredTool {
                    tool,
                    handler: None,
                },
            );
        }
    }

    /// Get a registered tool by name.
    pub async fn get_tool(&self, name: &str) -> Option<Tool> {
        let state = self.state.read().await;
        state.tools.get(name).map(|rt| rt.tool.clone())
    }

    /// Get all registered tools.
    pub async fn get_tools(&self) -> Vec<Tool> {
        let state = self.state.read().await;
        state.tools.values().map(|rt| rt.tool.clone()).collect()
    }

    /// Invoke a tool handler.
    pub async fn invoke_tool(&self, name: &str, arguments: &Value) -> Result<ToolResultObject> {
        let state = self.state.read().await;
        let registered = state
            .tools
            .get(name)
            .ok_or_else(|| CopilotError::ToolNotFound(name.to_string()))?;

        let handler = registered
            .handler
            .as_ref()
            .ok_or_else(|| CopilotError::ToolError(format!("No handler for tool: {}", name)))?;

        Ok(handler(name, arguments))
    }

    // =========================================================================
    // Permission Handling
    // =========================================================================

    /// Register a permission handler.
    pub async fn register_permission_handler<F>(&self, handler: F)
    where
        F: Fn(&PermissionRequest) -> PermissionRequestResult + Send + Sync + 'static,
    {
        let mut state = self.state.write().await;
        state.permission_handler = Some(Arc::new(handler));
    }

    /// Handle a permission request.
    ///
    /// Delegates to the registered permission handler, or denies by default
    /// if no handler is set.
    pub async fn handle_permission_request(
        &self,
        request: &PermissionRequest,
    ) -> PermissionRequestResult {
        let state = self.state.read().await;

        if let Some(handler) = &state.permission_handler {
            handler(request)
        } else {
            // Default: deny all permissions
            PermissionRequestResult::denied()
        }
    }

    // =========================================================================
    // User Input Handling
    // =========================================================================

    /// Register a handler for user input requests from the server.
    pub async fn register_user_input_handler<F>(&self, handler: F)
    where
        F: Fn(&UserInputRequest, &UserInputInvocation) -> UserInputResponse + Send + Sync + 'static,
    {
        let mut state = self.state.write().await;
        state.user_input_handler = Some(Arc::new(handler));
    }

    /// Handle a user input request from the server.
    pub async fn handle_user_input_request(
        &self,
        request: &UserInputRequest,
    ) -> Result<UserInputResponse> {
        let state = self.state.read().await;
        if let Some(handler) = &state.user_input_handler {
            let invocation = UserInputInvocation {
                session_id: self.session_id.clone(),
            };
            Ok(handler(request, &invocation))
        } else {
            Err(CopilotError::Protocol(
                "No user input handler registered".into(),
            ))
        }
    }

    /// Check if a user input handler is registered.
    pub async fn has_user_input_handler(&self) -> bool {
        let state = self.state.read().await;
        state.user_input_handler.is_some()
    }

    // =========================================================================
    // Hooks
    // =========================================================================

    /// Register session hooks.
    pub async fn register_hooks(&self, hooks: SessionHooks) {
        let mut state = self.state.write().await;
        state.hooks = Some(hooks);
    }

    /// Check if any hooks are registered.
    pub async fn has_hooks(&self) -> bool {
        let state = self.state.read().await;
        state.hooks.as_ref().is_some_and(|h| h.has_any())
    }

    /// Handle a `hooks.invoke` callback from the server.
    ///
    /// Dispatches to the appropriate hook handler based on `hook_type` and returns
    /// the serialized output JSON.
    pub async fn handle_hooks_invoke(&self, hook_type: &str, input: &Value) -> Result<Value> {
        let state = self.state.read().await;
        let hooks = match &state.hooks {
            Some(h) => h,
            None => return Ok(Value::Null),
        };

        match hook_type {
            "preToolUse" => {
                if let Some(handler) = &hooks.on_pre_tool_use {
                    let hook_input: PreToolUseHookInput = serde_json::from_value(input.clone())
                        .map_err(|e| {
                            CopilotError::Protocol(format!("Invalid preToolUse input: {}", e))
                        })?;
                    let output = handler(hook_input);
                    Ok(serde_json::to_value(output).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "postToolUse" => {
                if let Some(handler) = &hooks.on_post_tool_use {
                    let hook_input: PostToolUseHookInput = serde_json::from_value(input.clone())
                        .map_err(|e| {
                            CopilotError::Protocol(format!("Invalid postToolUse input: {}", e))
                        })?;
                    let output = handler(hook_input);
                    Ok(serde_json::to_value(output).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "userPromptSubmitted" => {
                if let Some(handler) = &hooks.on_user_prompt_submitted {
                    let hook_input: UserPromptSubmittedHookInput =
                        serde_json::from_value(input.clone()).map_err(|e| {
                            CopilotError::Protocol(format!(
                                "Invalid userPromptSubmitted input: {}",
                                e
                            ))
                        })?;
                    let output = handler(hook_input);
                    Ok(serde_json::to_value(output).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "sessionStart" => {
                if let Some(handler) = &hooks.on_session_start {
                    let hook_input: SessionStartHookInput = serde_json::from_value(input.clone())
                        .map_err(|e| {
                        CopilotError::Protocol(format!("Invalid sessionStart input: {}", e))
                    })?;
                    let output = handler(hook_input);
                    Ok(serde_json::to_value(output).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "sessionEnd" => {
                if let Some(handler) = &hooks.on_session_end {
                    let hook_input: SessionEndHookInput = serde_json::from_value(input.clone())
                        .map_err(|e| {
                            CopilotError::Protocol(format!("Invalid sessionEnd input: {}", e))
                        })?;
                    let output = handler(hook_input);
                    Ok(serde_json::to_value(output).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "errorOccurred" => {
                if let Some(handler) = &hooks.on_error_occurred {
                    let hook_input: ErrorOccurredHookInput = serde_json::from_value(input.clone())
                        .map_err(|e| {
                            CopilotError::Protocol(format!("Invalid errorOccurred input: {}", e))
                        })?;
                    let output = handler(hook_input);
                    Ok(serde_json::to_value(output).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            _ => Ok(Value::Null),
        }
    }

    // =========================================================================
    // Lifecycle
    // =========================================================================

    /// Destroy the session.
    pub async fn destroy(&self) -> Result<()> {
        let params = serde_json::json!({
            "sessionId": self.session_id,
        });

        (self.invoke_fn)("session.destroy", Some(params)).await?;
        Ok(())
    }

    // =========================================================================
    // Model Management
    // =========================================================================

    /// Get the current model for this session.
    pub async fn get_model(&self) -> Result<String> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        let result = (self.invoke_fn)("session.model.get_current", Some(params)).await?;
        result
            .get("modelId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| CopilotError::Protocol("Missing modelId in response".into()))
    }

    /// Switch to a different model mid-session.
    pub async fn set_model(&self, model: &str, options: Option<SetModelOptions>) -> Result<()> {
        let mut params = serde_json::json!({
            "sessionId": self.session_id,
            "modelId": model,
        });
        if let Some(opts) = options {
            if let Some(effort) = opts.reasoning_effort {
                params["reasoningEffort"] = serde_json::json!(effort);
            }
        }
        (self.invoke_fn)("session.model.switch_to", Some(params)).await?;
        Ok(())
    }

    // =========================================================================
    // Mode Management
    // =========================================================================

    /// Get the current session mode.
    pub async fn get_mode(&self) -> Result<SessionMode> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        let result = (self.invoke_fn)("session.mode.get", Some(params)).await?;
        let mode_str = result
            .get("mode")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CopilotError::Protocol("Missing mode in response".into()))?;
        serde_json::from_value(serde_json::json!(mode_str))
            .map_err(|e| CopilotError::Protocol(format!("Invalid mode '{}': {}", mode_str, e)))
    }

    /// Set the session mode (interactive, plan, or autopilot).
    pub async fn set_mode(&self, mode: SessionMode) -> Result<()> {
        let params = serde_json::json!({
            "sessionId": self.session_id,
            "mode": mode,
        });
        (self.invoke_fn)("session.mode.set", Some(params)).await?;
        Ok(())
    }

    // =========================================================================
    // Session Logging
    // =========================================================================

    /// Add a log entry to the session.
    pub async fn log(&self, message: &str, options: Option<LogOptions>) -> Result<LogResult> {
        let mut params = serde_json::json!({
            "sessionId": self.session_id,
            "message": message,
        });
        if let Some(opts) = options {
            if let Some(level) = opts.level {
                params["level"] = serde_json::to_value(level).unwrap_or_default();
            }
            if let Some(ephemeral) = opts.ephemeral {
                params["ephemeral"] = serde_json::json!(ephemeral);
            }
        }
        let result = (self.invoke_fn)("session.log", Some(params)).await?;
        let event_id = result
            .get("eventId")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(LogResult { event_id })
    }

    // =========================================================================
    // Plan Management
    // =========================================================================

    /// Read the current plan.
    pub async fn read_plan(&self) -> Result<Option<PlanData>> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        let result = (self.invoke_fn)("session.plan.read", Some(params)).await?;
        if result.is_null() || result.get("content").is_none() {
            return Ok(None);
        }
        serde_json::from_value(result)
            .map(Some)
            .map_err(|e| CopilotError::Protocol(format!("Failed to parse plan: {}", e)))
    }

    /// Update the session plan.
    pub async fn update_plan(&self, plan: &PlanData) -> Result<()> {
        let mut params = serde_json::to_value(plan)
            .map_err(|e| CopilotError::Protocol(format!("Failed to serialize plan: {}", e)))?;
        params["sessionId"] = serde_json::json!(self.session_id);
        (self.invoke_fn)("session.plan.update", Some(params)).await?;
        Ok(())
    }

    /// Delete the session plan.
    pub async fn delete_plan(&self) -> Result<()> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        (self.invoke_fn)("session.plan.delete", Some(params)).await?;
        Ok(())
    }

    // =========================================================================
    // Agent Management
    // =========================================================================

    /// List available agents.
    pub async fn list_agents(&self) -> Result<Vec<AgentInfo>> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        let result = (self.invoke_fn)("session.agent.list", Some(params)).await?;
        let agents = result
            .get("agents")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        serde_json::from_value(agents)
            .map_err(|e| CopilotError::Protocol(format!("Failed to parse agents: {}", e)))
    }

    /// Get the currently active agent.
    pub async fn get_current_agent(&self) -> Result<Option<AgentInfo>> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        let result = (self.invoke_fn)("session.agent.get_current", Some(params)).await?;
        if result.is_null() || result.get("name").is_none() {
            return Ok(None);
        }
        serde_json::from_value(result)
            .map(Some)
            .map_err(|e| CopilotError::Protocol(format!("Failed to parse agent: {}", e)))
    }

    /// Select (activate) a custom agent.
    pub async fn select_agent(&self, name: &str) -> Result<()> {
        let params = serde_json::json!({
            "sessionId": self.session_id,
            "name": name,
        });
        (self.invoke_fn)("session.agent.select", Some(params)).await?;
        Ok(())
    }

    /// Deselect the current custom agent.
    pub async fn deselect_agent(&self) -> Result<()> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        (self.invoke_fn)("session.agent.deselect", Some(params)).await?;
        Ok(())
    }

    // =========================================================================
    // Compaction
    // =========================================================================

    /// Trigger manual context compaction.
    pub async fn compact(&self) -> Result<()> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        (self.invoke_fn)("session.compaction.compact", Some(params)).await?;
        Ok(())
    }

    // =========================================================================
    // Fleet Management
    // =========================================================================

    /// Start a fleet of parallel agents.
    pub async fn start_fleet(&self, options: Option<FleetStartOptions>) -> Result<()> {
        let mut params = serde_json::json!({ "sessionId": self.session_id });
        if let Some(opts) = options {
            if let Some(prompt) = opts.prompt {
                params["prompt"] = serde_json::json!(prompt);
            }
        }
        (self.invoke_fn)("session.fleet.start", Some(params)).await?;
        Ok(())
    }

    // =========================================================================
    // Shell Operations
    // =========================================================================

    /// Execute a shell command in the session.
    pub async fn shell_exec(&self, options: ShellExecOptions) -> Result<ShellExecResult> {
        let mut params = serde_json::to_value(&options).map_err(|e| {
            CopilotError::Protocol(format!("Failed to serialize shell options: {}", e))
        })?;
        params["sessionId"] = serde_json::json!(self.session_id);
        let result = (self.invoke_fn)("session.shell.exec", Some(params)).await?;
        serde_json::from_value(result)
            .map_err(|e| CopilotError::Protocol(format!("Failed to parse shell result: {}", e)))
    }

    /// Kill a shell process.
    pub async fn shell_kill(&self, process_id: &str, signal: ShellSignal) -> Result<()> {
        let params = serde_json::json!({
            "sessionId": self.session_id,
            "processId": process_id,
            "signal": signal,
        });
        (self.invoke_fn)("session.shell.kill", Some(params)).await?;
        Ok(())
    }

    // =========================================================================
    // Workspace Operations
    // =========================================================================

    /// List files in the session workspace.
    pub async fn workspace_list_files(&self) -> Result<Vec<WorkspaceFile>> {
        let params = serde_json::json!({ "sessionId": self.session_id });
        let result = (self.invoke_fn)("session.workspace.list_files", Some(params)).await?;
        let files = result
            .get("files")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        serde_json::from_value(files)
            .map_err(|e| CopilotError::Protocol(format!("Failed to parse workspace files: {}", e)))
    }

    /// Read a file from the session workspace.
    pub async fn workspace_read_file(&self, path: &str) -> Result<String> {
        let params = serde_json::json!({
            "sessionId": self.session_id,
            "path": path,
        });
        let result = (self.invoke_fn)("session.workspace.read_file", Some(params)).await?;
        result
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| CopilotError::Protocol("Missing content in response".into()))
    }

    /// Create a file in the session workspace.
    pub async fn workspace_create_file(&self, path: &str, content: &str) -> Result<()> {
        let params = serde_json::json!({
            "sessionId": self.session_id,
            "path": path,
            "content": content,
        });
        (self.invoke_fn)("session.workspace.create_file", Some(params)).await?;
        Ok(())
    }
}

// =============================================================================
// Convenience methods for waiting on events
// =============================================================================

impl Session {
    /// Default timeout for waiting on session events (60 seconds).
    const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

    /// Wait for the session to become idle.
    ///
    /// Returns the last assistant message event, or None if no message was received.
    /// Uses the specified timeout, or 60 seconds if None.
    pub async fn wait_for_idle(&self, timeout: Option<Duration>) -> Result<Option<SessionEvent>> {
        let timeout = timeout.unwrap_or(Self::DEFAULT_TIMEOUT);
        let mut subscription = self.subscribe();
        let mut last_assistant_message: Option<SessionEvent> = None;

        let result = tokio::time::timeout(timeout, async {
            loop {
                match subscription.recv().await {
                    Ok(event) => match &event.data {
                        SessionEventData::AssistantMessage(_) => {
                            last_assistant_message = Some(event);
                        }
                        SessionEventData::AssistantMessageDelta(_) => {
                            // Deltas are intermediate; we track the full message
                        }
                        SessionEventData::SessionIdle(_) => {
                            break;
                        }
                        SessionEventData::SessionError(err) => {
                            return Err(CopilotError::Protocol(format!(
                                "Session error: {}",
                                err.message
                            )));
                        }
                        _ => {}
                    },
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err(CopilotError::ConnectionClosed);
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        // Continue - we missed some events but can recover
                    }
                }
            }
            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(last_assistant_message),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(CopilotError::Timeout(timeout)),
        }
    }

    /// Send a message and wait for the complete response.
    ///
    /// Returns the last `AssistantMessage` event, or `None` if session
    /// became idle without producing an assistant message.
    /// Uses the specified timeout, or 60 seconds if None.
    pub async fn send_and_wait(
        &self,
        options: impl Into<MessageOptions>,
        timeout: Option<Duration>,
    ) -> Result<Option<SessionEvent>> {
        self.send(options).await?;
        self.wait_for_idle(timeout).await
    }

    /// Send a message and wait for the response content as a string.
    ///
    /// Convenience method that collects all assistant message/delta content.
    /// Uses the specified timeout, or 60 seconds if None.
    pub async fn send_and_collect(
        &self,
        options: impl Into<MessageOptions>,
        timeout: Option<Duration>,
    ) -> Result<String> {
        let timeout = timeout.unwrap_or(Self::DEFAULT_TIMEOUT);
        self.send(options).await?;

        let mut subscription = self.subscribe();
        let mut content = String::new();

        let result = tokio::time::timeout(timeout, async {
            loop {
                match subscription.recv().await {
                    Ok(event) => match &event.data {
                        SessionEventData::AssistantMessage(msg) => {
                            content.push_str(&msg.content);
                        }
                        SessionEventData::AssistantMessageDelta(delta) => {
                            content.push_str(&delta.delta_content);
                        }
                        SessionEventData::SessionIdle(_) => {
                            break;
                        }
                        SessionEventData::SessionError(err) => {
                            return Err(CopilotError::Protocol(format!(
                                "Session error: {}",
                                err.message
                            )));
                        }
                        _ => {}
                    },
                    Err(broadcast::error::RecvError::Closed) => {
                        return Err(CopilotError::ConnectionClosed);
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                }
            }
            Ok(())
        })
        .await;

        match result {
            Ok(Ok(())) => Ok(content),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(CopilotError::Timeout(timeout)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    fn mock_invoke(_method: &str, _params: Option<Value>) -> InvokeFuture {
        Box::pin(async { Ok(serde_json::json!({"messageId": "test-msg-123"})) })
    }

    fn mock_invoke_with_events(method: &str, _params: Option<Value>) -> InvokeFuture {
        let method = method.to_string();
        Box::pin(async move {
            if method == "session.getMessages" {
                return Ok(serde_json::json!({
                    "events": [{
                        "id": "evt-1",
                        "timestamp": "2024-01-01T00:00:00Z",
                        "type": "session.idle",
                        "data": {}
                    }]
                }));
            }
            Ok(serde_json::json!({"messageId": "test-msg-123"}))
        })
    }

    #[tokio::test]
    async fn test_session_id() {
        let session = Session::new("test-session-123".to_string(), None, mock_invoke);
        assert_eq!(session.session_id(), "test-session-123");
    }

    #[tokio::test]
    async fn test_workspace_path() {
        let session = Session::new(
            "test".to_string(),
            Some("/tmp/workspace".to_string()),
            mock_invoke,
        );
        assert_eq!(session.workspace_path(), Some("/tmp/workspace"));
    }

    #[tokio::test]
    async fn test_register_tool() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        let tool = Tool::new("my_tool").description("A test tool");

        session.register_tool(tool.clone()).await;

        let retrieved = session.get_tool("my_tool").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "my_tool");
    }

    #[tokio::test]
    async fn test_register_tool_with_handler() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        let tool = Tool::new("echo").description("Echo tool");
        let handler: ToolHandler = Arc::new(|_name, args| {
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("empty");
            ToolResultObject::text(text)
        });

        session
            .register_tool_with_handler(tool, Some(handler))
            .await;

        let result = session
            .invoke_tool("echo", &serde_json::json!({"text": "hello"}))
            .await
            .unwrap();

        assert_eq!(result.text_result_for_llm, "hello");
    }

    #[tokio::test]
    async fn test_invoke_unknown_tool() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        let result = session.invoke_tool("unknown", &serde_json::json!({})).await;

        assert!(matches!(result, Err(CopilotError::ToolNotFound(_))));
    }

    #[tokio::test]
    async fn test_event_subscription() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        let mut sub1 = session.subscribe();
        let mut sub2 = session.subscribe();

        // Dispatch an event
        let event = SessionEvent::from_json(&serde_json::json!({
            "id": "evt-1",
            "timestamp": "2024-01-01T00:00:00Z",
            "type": "session.idle",
            "data": {}
        }))
        .unwrap();

        session.dispatch_event(event).await;

        // Both subscribers should receive it
        let received1 = sub1.recv().await.unwrap();
        let received2 = sub2.recv().await.unwrap();

        assert_eq!(received1.id, "evt-1");
        assert_eq!(received2.id, "evt-1");
    }

    #[tokio::test]
    async fn test_callback_handler() {
        let session = Session::new("test".to_string(), None, mock_invoke);
        let call_count = Arc::new(AtomicUsize::new(0));

        let count_clone = Arc::clone(&call_count);
        let unsubscribe = session
            .on(move |_event| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        // Dispatch events
        let event = SessionEvent::from_json(&serde_json::json!({
            "id": "evt-callback-1",
            "timestamp": "2024-01-01T00:00:00Z",
            "type": "session.idle",
            "data": {}
        }))
        .unwrap();

        session.dispatch_event(event).await;

        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Unsubscribe
        unsubscribe();
    }

    #[tokio::test]
    async fn test_permission_handler() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        // Default handler denies
        let request = PermissionRequest {
            kind: "tool_execution".to_string(),
            tool_call_id: Some("call-123".to_string()),
            extension_data: HashMap::new(),
        };
        let result = session.handle_permission_request(&request).await;
        assert!(result.kind.contains("denied"));

        // Register custom handler that approves
        session
            .register_permission_handler(|_req| PermissionRequestResult::approved())
            .await;

        let result = session.handle_permission_request(&request).await;
        assert_eq!(result.kind, "approved");
    }

    #[tokio::test]
    async fn test_dispatch_event_handles_external_tool_requested() {
        let rpc_calls = Arc::new(std::sync::Mutex::new(Vec::<(String, Value)>::new()));
        let rpc_calls_for_invoke = Arc::clone(&rpc_calls);
        let session = Session::new("test".to_string(), None, move |method, params| {
            let method = method.to_string();
            let params = params.unwrap_or(Value::Null);
            let rpc_calls = Arc::clone(&rpc_calls_for_invoke);
            Box::pin(async move {
                rpc_calls.lock().unwrap().push((method, params));
                Ok(serde_json::json!({}))
            })
        });

        session
            .register_tool_with_handler(
                Tool::new("echo").description("Echo tool"),
                Some(Arc::new(|_name, args| {
                    ToolResultObject::text(
                        args.get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("missing"),
                    )
                })),
            )
            .await;

        let mut subscription = session.subscribe();
        let event = SessionEvent::from_json(&serde_json::json!({
            "id": "evt-broadcast-tool",
            "timestamp": "2024-01-01T00:00:00Z",
            "type": "external_tool.requested",
            "data": {
                "requestId": "req-tool-1",
                "toolName": "echo",
                "toolCallId": "call-tool-1",
                "arguments": {
                    "text": "hello"
                }
            }
        }))
        .unwrap();

        session.dispatch_event(event).await;

        let forwarded = subscription.recv().await.unwrap();
        assert_eq!(forwarded.event_type, "external_tool.requested");

        let calls = rpc_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, "session.tools.handlePendingToolCall");
        assert_eq!(calls[0].1["sessionId"], "test");
        assert_eq!(calls[0].1["requestId"], "req-tool-1");
        assert_eq!(calls[0].1["result"]["textResultForLlm"], "hello");
        assert_eq!(calls[0].1["result"]["resultType"], "success");
    }

    #[tokio::test]
    async fn test_dispatch_event_handles_permission_requested() {
        let rpc_calls = Arc::new(std::sync::Mutex::new(Vec::<(String, Value)>::new()));
        let rpc_calls_for_invoke = Arc::clone(&rpc_calls);
        let seen_request = Arc::new(std::sync::Mutex::new(None::<PermissionRequest>));
        let seen_request_for_handler = Arc::clone(&seen_request);
        let session = Session::new("test".to_string(), None, move |method, params| {
            let method = method.to_string();
            let params = params.unwrap_or(Value::Null);
            let rpc_calls = Arc::clone(&rpc_calls_for_invoke);
            Box::pin(async move {
                rpc_calls.lock().unwrap().push((method, params));
                Ok(serde_json::json!({}))
            })
        });

        session
            .register_permission_handler(move |request| {
                *seen_request_for_handler.lock().unwrap() = Some(request.clone());
                PermissionRequestResult::approved()
            })
            .await;

        let mut subscription = session.subscribe();
        let event = SessionEvent::from_json(&serde_json::json!({
            "id": "evt-broadcast-permission",
            "timestamp": "2024-01-01T00:00:00Z",
            "type": "permission.requested",
            "data": {
                "requestId": "req-perm-1",
                "permissionRequest": {
                    "kind": "tool_execution",
                    "toolCallId": "call-perm-1",
                    "toolName": "shell",
                    "command": "ls"
                }
            }
        }))
        .unwrap();

        session.dispatch_event(event).await;

        let forwarded = subscription.recv().await.unwrap();
        assert_eq!(forwarded.event_type, "permission.requested");

        let request = seen_request.lock().unwrap().clone().unwrap();
        assert_eq!(request.kind, "tool_execution");
        assert_eq!(request.tool_call_id.as_deref(), Some("call-perm-1"));
        assert_eq!(request.extension_data["toolName"], "shell");
        assert_eq!(request.extension_data["command"], "ls");

        let calls = rpc_calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].0,
            "session.permissions.handlePendingPermissionRequest"
        );
        assert_eq!(calls[0].1["sessionId"], "test");
        assert_eq!(calls[0].1["requestId"], "req-perm-1");
        assert_eq!(calls[0].1["result"]["kind"], "approved");
    }

    #[tokio::test]
    async fn test_get_messages_with_events_field() {
        let session = Session::new("test".to_string(), None, mock_invoke_with_events);
        let messages = session.get_messages().await.unwrap();
        assert_eq!(messages.len(), 1);
        assert!(matches!(
            messages[0].data,
            crate::events::SessionEventData::SessionIdle(_)
        ));
    }

    #[tokio::test]
    async fn test_user_input_handler() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        session
            .register_user_input_handler(|req, _inv| {
                assert_eq!(req.question, "What color?");
                UserInputResponse {
                    answer: "blue".into(),
                    was_freeform: Some(true),
                }
            })
            .await;

        let request = UserInputRequest {
            question: "What color?".into(),
            choices: Some(vec!["red".into(), "blue".into()]),
            allow_freeform: Some(true),
        };

        let response = session.handle_user_input_request(&request).await.unwrap();
        assert_eq!(response.answer, "blue");
        assert_eq!(response.was_freeform, Some(true));
    }

    #[tokio::test]
    async fn test_user_input_no_handler_errors() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        let request = UserInputRequest {
            question: "?".into(),
            choices: None,
            allow_freeform: None,
        };

        let result = session.handle_user_input_request(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_register_hooks() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        assert!(!session.has_hooks().await);

        let hooks = crate::types::SessionHooks {
            on_pre_tool_use: Some(Arc::new(|input| {
                assert_eq!(input.tool_name, "my_tool");
                crate::types::PreToolUseHookOutput {
                    permission_decision: Some("allow".into()),
                    ..Default::default()
                }
            })),
            ..Default::default()
        };

        session.register_hooks(hooks).await;
        assert!(session.has_hooks().await);
    }

    #[tokio::test]
    async fn test_hooks_invoke_pre_tool_use() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        let hooks = crate::types::SessionHooks {
            on_pre_tool_use: Some(Arc::new(|_input| crate::types::PreToolUseHookOutput {
                permission_decision: Some("allow".into()),
                additional_context: Some("extra context".into()),
                ..Default::default()
            })),
            ..Default::default()
        };

        session.register_hooks(hooks).await;

        let input = serde_json::json!({
            "timestamp": 1234567890,
            "cwd": "/tmp",
            "toolName": "test_tool",
            "toolArgs": {"key": "value"}
        });

        let result = session
            .handle_hooks_invoke("preToolUse", &input)
            .await
            .unwrap();
        assert_eq!(
            result.get("permissionDecision").and_then(|v| v.as_str()),
            Some("allow")
        );
        assert_eq!(
            result.get("additionalContext").and_then(|v| v.as_str()),
            Some("extra context")
        );
    }

    #[tokio::test]
    async fn test_hooks_invoke_no_handler_returns_null() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        // No hooks registered at all
        let result = session
            .handle_hooks_invoke("preToolUse", &serde_json::json!({}))
            .await
            .unwrap();
        assert!(result.is_null());

        // Hooks registered but not for this type
        let hooks = crate::types::SessionHooks {
            on_session_start: Some(Arc::new(|_input| {
                crate::types::SessionStartHookOutput::default()
            })),
            ..Default::default()
        };
        session.register_hooks(hooks).await;

        let input = serde_json::json!({
            "timestamp": 1234567890,
            "cwd": "/tmp",
            "toolName": "test_tool",
            "toolArgs": {}
        });
        let result = session
            .handle_hooks_invoke("preToolUse", &input)
            .await
            .unwrap();
        assert!(result.is_null());
    }

    #[tokio::test]
    async fn test_hooks_invoke_unknown_type_returns_null() {
        let session = Session::new("test".to_string(), None, mock_invoke);

        let hooks = crate::types::SessionHooks {
            on_pre_tool_use: Some(Arc::new(|_| crate::types::PreToolUseHookOutput::default())),
            ..Default::default()
        };
        session.register_hooks(hooks).await;

        let result = session
            .handle_hooks_invoke("unknownHookType", &serde_json::json!({}))
            .await
            .unwrap();
        assert!(result.is_null());
    }
}
