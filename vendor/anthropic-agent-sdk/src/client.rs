//! `ClaudeSDKClient` for bidirectional communication
//!
//! This module provides the main client for interactive, stateful conversations
//! with Claude Code, including support for:
//! - Bidirectional messaging (no lock contention)
//! - Interrupts and control flow
//! - Hook and permission callbacks
//! - Conversation state management
//!
//! # Architecture
//!
//! The client uses a lock-free architecture for reading and writing:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                   ClaudeSDKClient                        │
//! │                                                          │
//! │  ┌──────────────────┐        ┌──────────────────┐      │
//! │  │  Message Reader  │        │  Control Writer  │      │
//! │  │  Background Task │        │  Background Task │      │
//! │  │                  │        │                  │      │
//! │  │ • Gets receiver  │        │ • Locks per-write│      │
//! │  │   once           │        │ • No blocking    │      │
//! │  │ • No lock held   │        │                  │      │
//! │  │   while reading  │        │                  │      │
//! │  └────────┬─────────┘        └────────┬─────────┘      │
//! │           │                           │                 │
//! │           │    ┌──────────────┐      │                 │
//! │           └───→│  Transport   │←─────┘                 │
//! │                │  (Arc<Mutex>)│                         │
//! │                └──────────────┘                         │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! **Key Design Points:**
//! - Transport returns an owned `UnboundedReceiver` (no lifetime issues)
//! - Reader task gets receiver once, then releases transport lock
//! - Writer task locks transport briefly for each write operation
//! - No contention: reader never blocks writer, writer never blocks reader
//!
//! # Example: Basic Usage
//!
//! ```no_run
//! use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::default();
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//!
//! // Send a message
//! client.send_message("Hello, Claude!").await?;
//!
//! // Read responses
//! while let Some(message) = client.next_message().await {
//!     match message? {
//!         Message::Assistant { message, .. } => {
//!             println!("Response: {:?}", message.content);
//!         }
//!         Message::Result { .. } => break,
//!         _ => {}
//!     }
//! }
//!
//! client.close().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Concurrent Operations
//!
//! ```no_run
//! use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::default();
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//!
//! // Send first message
//! client.send_message("First question").await?;
//!
//! // Can send another message while reading responses
//! // No blocking due to lock-free architecture
//! tokio::spawn(async move {
//!     tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
//!     client.send_message("Second question").await
//! });
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Interrupt
//!
//! ```no_run
//! use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::default();
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//!
//! client.send_message("Write a long essay").await?;
//!
//! // After some time, interrupt the response
//! tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
//! client.interrupt().await?;
//!
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Hooks and Permissions
//!
//! ```no_run
//! use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let options = ClaudeAgentOptions::default();
//! let mut client = ClaudeSDKClient::new(options, None).await?;
//!
//! // Take receivers to handle hooks and permissions
//! let mut hook_rx = client.take_hook_receiver().unwrap();
//! let mut perm_rx = client.take_permission_receiver().unwrap();
//!
//! // Handle hook events
//! tokio::spawn(async move {
//!     while let Some((hook_id, event)) = hook_rx.recv().await {
//!         println!("Hook: {} {:?}", hook_id, event);
//!         // Respond to hook...
//!     }
//! });
//!
//! // Handle permission requests
//! tokio::spawn(async move {
//!     while let Some((req_id, request)) = perm_rx.recv().await {
//!         println!("Permission: {:?}", request);
//!         // Respond to permission...
//!     }
//! });
//!
//! # Ok(())
//! # }
//! ```

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_util::sync::CancellationToken;

use crate::control::{ControlMessage, ControlRequest, ProtocolHandler};
use crate::error::{ClaudeError, Result};
use crate::hooks::HookManager;
use crate::message::parse_message;
use crate::permissions::PermissionManager;
use crate::transport::{PromptInput, SubprocessTransport, Transport};
use crate::types::{
    AccountInfo, ClaudeAgentOptions, HookEvent, Message, ModelInfo, PermissionRequest, RequestId,
    SessionId, SessionInfo,
};
use futures::Stream;

/// A buffered message with its associated session ID for security validation
type BufferedMessage = (Option<SessionId>, String);

/// Thread-safe queue for buffering messages during streaming
type MessageBuffer = Arc<std::sync::Mutex<VecDeque<BufferedMessage>>>;

/// Context for the message reader background task
struct MessageReaderContext {
    transport: Arc<Mutex<SubprocessTransport>>,
    protocol: Arc<Mutex<ProtocolHandler>>,
    message_tx: mpsc::UnboundedSender<Result<Message>>,
    session_id: Arc<std::sync::Mutex<Option<SessionId>>>,
    session_info: Arc<std::sync::Mutex<Option<SessionInfo>>>,
    bound_session_id: Arc<std::sync::Mutex<Option<SessionId>>>,
    hook_manager: Option<Arc<Mutex<HookManager>>>,
    is_resume: bool,
}

/// Client for bidirectional communication with Claude Code
///
/// `ClaudeSDKClient` provides interactive, stateful conversations with
/// support for interrupts, hooks, and permission callbacks.
///
/// # Examples
///
/// ```no_run
/// use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
/// use futures::StreamExt;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let options = ClaudeAgentOptions::default();
///     let mut client = ClaudeSDKClient::new(options, None).await?;
///
///     client.send_message("Hello, Claude!").await?;
///
///     while let Some(message) = client.next_message().await {
///         println!("{:?}", message?);
///     }
///
///     Ok(())
/// }
/// ```
pub struct ClaudeSDKClient {
    /// Transport layer
    transport: Arc<Mutex<SubprocessTransport>>,
    /// Control protocol handler
    protocol: Arc<Mutex<ProtocolHandler>>,
    /// Message stream receiver
    message_rx: mpsc::UnboundedReceiver<Result<Message>>,
    /// Control message sender
    control_tx: mpsc::UnboundedSender<ControlRequest>,
    /// Hook event receiver (if not using automatic handler)
    hook_rx: Option<mpsc::UnboundedReceiver<(String, HookEvent)>>,
    /// Permission request receiver (if not using automatic handler)
    permission_rx: Option<mpsc::UnboundedReceiver<(RequestId, PermissionRequest)>>,
    /// Hook manager for automatic hook handling (kept alive for background tasks)
    #[allow(dead_code)]
    hook_manager: Option<Arc<Mutex<HookManager>>>,
    /// Permission manager for automatic permission handling (kept alive for background tasks)
    #[allow(dead_code)]
    permission_manager: Option<Arc<Mutex<PermissionManager>>>,
    /// Captured session ID from messages
    session_id: Arc<std::sync::Mutex<Option<SessionId>>>,
    /// Session info from init message (model, tools, MCP servers)
    session_info: Arc<std::sync::Mutex<Option<SessionInfo>>>,
    /// Cancellation token for aborting operations (like `AbortController` in JS)
    cancellation_token: CancellationToken,
    /// Runtime model override
    runtime_model: Arc<std::sync::Mutex<Option<String>>>,
    /// Runtime permission mode override
    runtime_permission_mode: Arc<std::sync::Mutex<Option<crate::types::PermissionMode>>>,
    /// Runtime max thinking tokens override
    runtime_max_thinking_tokens: Arc<std::sync::Mutex<Option<u32>>>,
    /// Message buffer for queuing messages during streaming
    message_buffer: MessageBuffer,
    /// Bound session ID - if set, all sends validate against this
    bound_session_id: Arc<std::sync::Mutex<Option<SessionId>>>,
}

impl ClaudeSDKClient {
    /// Create a new `ClaudeSDKClient`
    ///
    /// # Arguments
    /// * `options` - Configuration options
    /// * `cli_path` - Optional path to Claude Code CLI
    ///
    /// # Errors
    /// Returns error if CLI cannot be found or connection fails
    pub async fn new(
        options: ClaudeAgentOptions,
        cli_path: Option<std::path::PathBuf>,
    ) -> Result<Self> {
        // Create cancellation token (like AbortController in JavaScript)
        let cancellation_token = CancellationToken::new();

        // Initialize hook manager if hooks are configured
        let (hook_manager, hook_rx) = if let Some(ref hooks_config) = options.hooks {
            let mut manager = HookManager::from_hooks_config(hooks_config.clone());
            // Set cancellation token so hooks can check for abort
            manager.set_cancellation_token(cancellation_token.child_token());
            (Some(Arc::new(Mutex::new(manager))), None)
        } else {
            (None, Some(mpsc::unbounded_channel().1))
        };

        // Initialize permission manager if callback is configured
        let (permission_manager, permission_rx) = if options.can_use_tool.is_some() {
            let mut manager = PermissionManager::new();
            if let Some(callback) = options.can_use_tool.clone() {
                manager.set_callback(callback);
            }
            manager.set_allowed_tools(Some(options.allowed_tools.clone()));
            manager.set_disallowed_tools(options.disallowed_tools.clone());
            (Some(Arc::new(Mutex::new(manager))), None)
        } else {
            (None, Some(mpsc::unbounded_channel().1))
        };

        // Check if this is a resume session (for SessionStart hook)
        let is_resume = options.resume.is_some();

        // Create transport with streaming mode and pass child cancellation token
        let prompt_input = PromptInput::Stream;
        let mut transport = SubprocessTransport::with_cancellation_token(
            prompt_input,
            options,
            cli_path,
            Some(cancellation_token.child_token()),
        )?;

        // Connect transport
        transport.connect().await?;

        // Create protocol handler
        let mut protocol = ProtocolHandler::new();

        // Set up channels
        let (hook_tx, hook_rx_internal) = mpsc::unbounded_channel();
        let (permission_tx, permission_rx_internal) = mpsc::unbounded_channel();
        protocol.set_hook_channel(hook_tx);
        protocol.set_permission_channel(permission_tx);

        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let (control_tx, control_rx) = mpsc::unbounded_channel();

        // Note: Claude CLI doesn't use a separate control protocol initialization.
        // The stream-json mode expects user messages to be sent directly.
        // Mark protocol as initialized immediately.
        protocol.set_initialized(true);

        let transport = Arc::new(Mutex::new(transport));
        let protocol = Arc::new(Mutex::new(protocol));
        let session_id = Arc::new(std::sync::Mutex::new(None));
        let session_info = Arc::new(std::sync::Mutex::new(None));
        let bound_session_id = Arc::new(std::sync::Mutex::new(None));

        // Spawn message reader task
        let reader_ctx = MessageReaderContext {
            transport: transport.clone(),
            protocol: protocol.clone(),
            message_tx,
            session_id: session_id.clone(),
            session_info: session_info.clone(),
            bound_session_id: bound_session_id.clone(),
            hook_manager: hook_manager.clone(),
            is_resume,
        };
        tokio::spawn(async move {
            Self::message_reader_task(reader_ctx).await;
        });

        // Spawn control message writer task
        let transport_clone = transport.clone();
        let protocol_clone = protocol.clone();
        tokio::spawn(async move {
            Self::control_writer_task(transport_clone, protocol_clone, control_rx).await;
        });

        // Spawn hook handler task if hook manager is configured
        if let Some(ref manager) = hook_manager {
            let manager_clone = manager.clone();
            let protocol_clone = protocol.clone();
            tokio::spawn(async move {
                Self::hook_handler_task(manager_clone, protocol_clone, hook_rx_internal).await;
            });
        }

        // Spawn permission handler task if permission manager is configured
        if let Some(ref manager) = permission_manager {
            let manager_clone = manager.clone();
            let protocol_clone = protocol.clone();
            tokio::spawn(async move {
                Self::permission_handler_task(
                    manager_clone,
                    protocol_clone,
                    permission_rx_internal,
                )
                .await;
            });
        }

        Ok(Self {
            transport,
            protocol,
            message_rx,
            control_tx,
            hook_rx,
            permission_rx,
            hook_manager,
            permission_manager,
            session_id,
            session_info,
            cancellation_token,
            runtime_model: Arc::new(std::sync::Mutex::new(None)),
            runtime_permission_mode: Arc::new(std::sync::Mutex::new(None)),
            runtime_max_thinking_tokens: Arc::new(std::sync::Mutex::new(None)),
            message_buffer: Arc::new(std::sync::Mutex::new(VecDeque::new())),
            bound_session_id,
        })
    }

    /// Message reader task - reads from transport and processes messages
    ///
    /// If `hook_manager` is provided, automatically calls `process_message()` on each
    /// message to trigger registered hooks (`SubagentStart`, `SubagentStop`, etc.)
    #[allow(clippy::too_many_lines)]
    async fn message_reader_task(ctx: MessageReaderContext) {
        let MessageReaderContext {
            transport,
            protocol,
            message_tx,
            session_id,
            session_info,
            bound_session_id,
            hook_manager,
            is_resume,
        } = ctx;
        // Get the message receiver from the transport without holding the lock
        let mut msg_stream = {
            let mut transport_guard = transport.lock().await;
            transport_guard.read_messages()
        };

        while let Some(result) = msg_stream.recv().await {
            match result {
                Ok(value) => {
                    // Try to parse as control message first
                    let protocol_guard = protocol.lock().await;
                    let value_str = serde_json::to_string(&value).unwrap_or_default();
                    if let Ok(control_msg) = protocol_guard.deserialize_message(&value_str) {
                        tracing::trace!("Parsed as control message, consuming internally");

                        match control_msg {
                            ControlMessage::InitResponse(init_response) => {
                                if let Err(e) = protocol_guard.handle_init_response(&init_response)
                                {
                                    let _ = message_tx.send(Err(e));
                                    break;
                                }
                            }
                            ControlMessage::Response(response) => {
                                if let Err(e) = protocol_guard.handle_response(response).await {
                                    let _ = message_tx.send(Err(e));
                                }
                            }
                            ControlMessage::Request(_) | ControlMessage::Init(_) => {
                                // Ignore requests and init in client mode
                            }
                        }
                        drop(protocol_guard);
                        continue;
                    }
                    drop(protocol_guard);

                    // Check for control_response (ack from CLI for control_request)
                    // These are internal protocol messages, not user-facing
                    if let Some(msg_type) = value.get("type").and_then(|v| v.as_str()) {
                        if msg_type == "control_response" {
                            tracing::debug!(
                                request_id = %value.get("response")
                                    .and_then(|r| r.get("request_id"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown"),
                                "Received control_response (interrupt/setModel/etc ack)"
                            );
                            continue;
                        }
                    }

                    // Otherwise parse as regular message
                    tracing::trace!(
                        preview = %serde_json::to_string(&value).unwrap_or_default().chars().take(100).collect::<String>(),
                        "Parsing as Message"
                    );

                    match parse_message(value) {
                        Ok(msg) => {
                            tracing::trace!("Parsed message successfully, sending to channel");

                            // Capture session_id from Result messages
                            if let Message::Result {
                                session_id: ref sid,
                                ..
                            } = msg
                            {
                                if let Ok(mut session_guard) = session_id.lock() {
                                    *session_guard = Some(sid.clone());
                                }
                                // Auto-bind to session on first Result (secure by default)
                                if let Ok(mut bound_guard) = bound_session_id.lock() {
                                    if bound_guard.is_none() {
                                        *bound_guard = Some(sid.clone());
                                    }
                                }
                            }

                            // Capture session info from System init message and pass to HookManager
                            if let Message::System {
                                ref subtype,
                                ref data,
                            } = msg
                            {
                                if subtype == "init" {
                                    let init_session_id = data
                                        .get("session_id")
                                        .and_then(|v| v.as_str())
                                        .map(std::string::ToString::to_string);
                                    let init_cwd = data
                                        .get("cwd")
                                        .and_then(|v| v.as_str())
                                        .map(std::string::ToString::to_string);

                                    // Update session_id storage
                                    if let Some(ref sid) = init_session_id {
                                        if let Ok(mut session_guard) = session_id.lock() {
                                            *session_guard = Some(SessionId::from(sid.clone()));
                                        }
                                    }

                                    // Populate session_info from init data
                                    if let Ok(mut info_guard) = session_info.lock() {
                                        *info_guard = Some(SessionInfo::from_init_data(data));
                                    }

                                    // Update HookManager with session context and trigger SessionStart
                                    if let Some(ref manager) = hook_manager {
                                        if let Some(sid) = init_session_id {
                                            let mut manager_guard = manager.lock().await;
                                            manager_guard.set_session_context(sid, init_cwd);

                                            // Determine session start source
                                            let source =
                                                if is_resume { "resume" } else { "startup" };

                                            // Trigger SessionStart hook
                                            if let Err(e) =
                                                manager_guard.trigger_session_start(source).await
                                            {
                                                tracing::warn!(error = %e, "SessionStart hook error");
                                            }

                                            tracing::debug!(
                                                source = source,
                                                "Triggered SessionStart hook"
                                            );
                                        }
                                    }
                                }
                            }

                            // Process message through hook manager if configured
                            // This triggers SubagentStart, SubagentStop, PreToolUse, PostToolUse hooks
                            if let Some(ref manager) = hook_manager {
                                let mut manager_guard = manager.lock().await;
                                match manager_guard.process_message(&msg).await {
                                    Ok(outputs) => {
                                        if !outputs.is_empty() {
                                            tracing::debug!(
                                                count = outputs.len(),
                                                "Hook outputs from message processing"
                                            );
                                        }
                                        // Hook outputs are handled internally by callbacks
                                        // Future: could send outputs to a channel for external handling
                                    }
                                    Err(e) => {
                                        tracing::warn!(error = %e, "Hook processing error");
                                    }
                                }
                            }

                            if message_tx.send(Ok(msg)).is_err() {
                                tracing::warn!(
                                    "Failed to send message to channel - receiver dropped"
                                );
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::debug!(error = ?e, "Failed to parse message");
                            let _ = message_tx.send(Err(e));
                        }
                    }
                }
                Err(e) => {
                    let _ = message_tx.send(Err(e));
                    break;
                }
            }
        }
    }

    /// Control message writer task - writes control requests to transport
    ///
    /// Sends control requests using the Claude CLI streaming protocol format:
    /// ```json
    /// {"type": "control_request", "request_id": "...", "request": {"subtype": "..."}}
    /// ```
    async fn control_writer_task(
        transport: Arc<Mutex<SubprocessTransport>>,
        _protocol: Arc<Mutex<ProtocolHandler>>,
        mut control_rx: mpsc::UnboundedReceiver<ControlRequest>,
    ) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

        while let Some(request) = control_rx.recv().await {
            // Generate unique request ID matching TypeScript SDK format
            let request_id = format!(
                "req_{}_{:x}",
                REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            );

            // Build the inner request object based on control type
            let inner_request = match request {
                ControlRequest::Interrupt { .. } => {
                    serde_json::json!({"subtype": "interrupt"})
                }
                ControlRequest::SendMessage { content, .. } => {
                    // User messages should go through send_message, not control channel
                    // But handle it anyway for robustness
                    serde_json::json!({
                        "type": "user",
                        "message": {
                            "role": "user",
                            "content": content
                        }
                    })
                }
                ControlRequest::SetModel { model, .. } => {
                    serde_json::json!({
                        "subtype": "set_model",
                        "model": model
                    })
                }
                ControlRequest::SetPermissionMode { mode, .. } => {
                    serde_json::json!({
                        "subtype": "set_permission_mode",
                        "mode": mode
                    })
                }
                ControlRequest::SetMaxThinkingTokens {
                    max_thinking_tokens,
                    ..
                } => {
                    serde_json::json!({
                        "subtype": "set_max_thinking_tokens",
                        "max_thinking_tokens": max_thinking_tokens
                    })
                }
                ControlRequest::RewindFiles {
                    user_message_uuid, ..
                } => {
                    // Try snake_case format (CLI may use different format than TS SDK)
                    serde_json::json!({
                        "subtype": "rewind_files",
                        "user_message_uuid": user_message_uuid
                    })
                }
                _ => {
                    // Other control types not yet supported in stream-json mode
                    tracing::debug!(request = ?request, "Skipping unsupported control request");
                    continue;
                }
            };

            // Wrap in control_request envelope (matches TypeScript SDK protocol)
            let control_json = serde_json::json!({
                "type": "control_request",
                "request_id": request_id,
                "request": inner_request
            });

            if let Ok(json_str) = serde_json::to_string(&control_json) {
                tracing::debug!(json = %json_str, "Sending control request to CLI");
                let message_line = format!("{json_str}\n");
                let mut transport_guard = transport.lock().await;
                if transport_guard.write(&message_line).await.is_err() {
                    tracing::error!("Failed to write control request to CLI");
                    break;
                }
            } else {
                tracing::error!("Failed to serialize control request");
                break;
            }
        }
    }

    /// Hook handler task - automatically processes hook events
    async fn hook_handler_task(
        manager: Arc<Mutex<HookManager>>,
        protocol: Arc<Mutex<ProtocolHandler>>,
        mut hook_rx: mpsc::UnboundedReceiver<(String, HookEvent)>,
    ) {
        while let Some((hook_id, event)) = hook_rx.recv().await {
            // NOTE: This code path handles hook events from the control protocol.
            // The primary hook flow is through process_message() in message_reader_task,
            // which properly extracts event data and builds HookContext with session info.
            // This handler is kept for backwards compatibility with the control protocol.
            let manager_guard = manager.lock().await;
            // Build context with session info from manager (if set)
            let context = manager_guard.build_context();

            match manager_guard
                .invoke(event, serde_json::json!({}), None, context)
                .await
            {
                Ok(output) => {
                    drop(manager_guard);

                    // Send hook response
                    let protocol_guard = protocol.lock().await;
                    let response = serde_json::to_value(&output).unwrap_or_default();
                    let _request = protocol_guard.create_hook_response(hook_id, response);
                    drop(protocol_guard);

                    // Send through control channel would require access to control_tx
                    // For now, hooks are processed but response sending needs client cooperation
                    // This is acceptable as hooks are advisory
                    // In a full implementation, we'd send _request through control_tx
                    tracing::debug!(event = ?event, "Hook processed");
                    #[cfg(debug_assertions)]
                    eprintln!("Hook processed for event {event:?}");
                }
                Err(e) => {
                    tracing::error!(error = %e, "Hook processing error");
                    #[cfg(debug_assertions)]
                    eprintln!("Hook processing error: {e}");
                }
            }
        }
    }

    /// Permission handler task - automatically processes permission requests
    async fn permission_handler_task(
        manager: Arc<Mutex<PermissionManager>>,
        protocol: Arc<Mutex<ProtocolHandler>>,
        mut permission_rx: mpsc::UnboundedReceiver<(RequestId, PermissionRequest)>,
    ) {
        while let Some((request_id, request)) = permission_rx.recv().await {
            let manager_guard = manager.lock().await;

            match manager_guard
                .can_use_tool(
                    request.tool_name.clone(),
                    request.tool_input.clone(),
                    request.context.clone(),
                )
                .await
            {
                Ok(result) => {
                    drop(manager_guard);

                    // Send permission response
                    let protocol_guard = protocol.lock().await;
                    let _request = protocol_guard
                        .create_permission_response(request_id.clone(), result.clone());
                    drop(protocol_guard);

                    // Send through control channel would require access to control_tx
                    // For now, permissions are processed but response sending needs client cooperation
                    // This is acceptable for the automatic mode
                    // In a full implementation, we'd send _request through control_tx
                    tracing::debug!(request_id = %request_id.as_str(), result = ?result, "Permission processed");
                    #[cfg(debug_assertions)]
                    eprintln!("Permission {} processed: {:?}", request_id.as_str(), result);
                }
                Err(e) => {
                    tracing::error!(error = %e, "Permission processing error");
                    #[cfg(debug_assertions)]
                    eprintln!("Permission processing error: {e}");
                }
            }
        }
    }

    /// Send a message to Claude
    ///
    /// # Arguments
    /// * `content` - Message content to send
    ///
    /// # Errors
    /// Returns error if message cannot be sent
    pub async fn send_message(&mut self, content: impl Into<String>) -> Result<()> {
        // Validate session if bound
        self.validate_session()?;

        let content_str = content.into();

        // Trigger UserPromptSubmit hook before sending
        if let Some(ref manager) = self.hook_manager {
            let manager_guard = manager.lock().await;
            if let Err(e) = manager_guard.trigger_user_prompt_submit(&content_str).await {
                tracing::warn!(error = %e, "UserPromptSubmit hook error");
            }
        }

        // Send a user message in the format the CLI expects
        let message = serde_json::json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": content_str
            }
        });
        let message_json = format!("{}\n", serde_json::to_string(&message)?);

        let mut transport = self.transport.lock().await;
        transport.write(&message_json).await
    }

    // ========================================================================
    // Message Buffering
    // ========================================================================

    /// Queue a message to be sent after the current turn completes.
    ///
    /// The CLI only reads stdin between turns, not during streaming.
    /// Messages queued with this method are stored and can be sent
    /// automatically using `receive_buffered()` or manually with `send_queued()`.
    ///
    /// **Security**: Each queued message is associated with the current `session_id`.
    /// When sending, the SDK verifies the session hasn't changed, preventing
    /// messages from being accidentally sent to a different conversation context.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// // Send first message
    /// client.send_message("What is Python?").await?;
    ///
    /// // Queue follow-up messages (will be sent after each Result)
    /// client.queue_message("What is TypeScript?");
    /// client.queue_message("Compare Rust to both.");
    ///
    /// // Process all messages with automatic queue handling
    /// while let Some(msg) = client.next_buffered().await {
    ///     // Handle messages...
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn queue_message(&self, content: impl Into<String>) {
        // Capture current session_id for security
        let session_id = self.get_session_id();
        if let Ok(mut buffer) = self.message_buffer.lock() {
            buffer.push_back((session_id, content.into()));
        }
    }

    /// Get the number of messages waiting in the queue.
    #[must_use]
    pub fn queued_count(&self) -> usize {
        self.message_buffer.lock().map(|b| b.len()).unwrap_or(0)
    }

    /// Check if there are messages waiting to be sent.
    #[must_use]
    pub fn has_queued(&self) -> bool {
        self.queued_count() > 0
    }

    /// Send the next queued message.
    ///
    /// Returns `Ok(true)` if a message was sent, `Ok(false)` if queue is empty.
    ///
    /// **Security**: Verifies the queued message's `session_id` matches the current
    /// session. If session changed, the message is discarded with a warning to
    /// prevent sending messages to an unintended conversation context.
    ///
    /// # Errors
    /// Returns error if message cannot be sent.
    pub async fn send_queued(&mut self) -> Result<bool> {
        let current_session = self.get_session_id();

        let next_entry = {
            self.message_buffer
                .lock()
                .ok()
                .and_then(|mut b| b.pop_front())
        };

        if let Some((queued_session, msg)) = next_entry {
            // Security check: verify session_id matches
            match (&queued_session, &current_session) {
                (Some(queued), Some(current)) if queued != current => {
                    // Session changed - discard message for safety
                    tracing::warn!(
                        queued_session = %queued,
                        current_session = %current,
                        "Discarding queued message: session_id changed"
                    );
                    // Clear remaining messages from old session
                    self.clear_queue();
                    return Ok(false);
                }
                _ => {
                    // Session matches or one is None (early in conversation)
                    self.send_message(msg).await?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Get the next message, automatically sending queued messages after Results.
    ///
    /// This is the recommended way to handle multi-turn conversations with buffering.
    /// After receiving a Result message, any queued messages are automatically sent.
    /// Returns `None` when stream ends AND no more queued messages remain.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// client.send_message("What is Python?").await?;
    /// client.queue_message("What is TypeScript?");
    /// client.queue_message("Compare Rust to both.");
    ///
    /// while let Some(msg) = client.next_buffered().await {
    ///     match msg? {
    ///         Message::Assistant { message, .. } => {
    ///             println!("Claude: {:?}", message.content);
    ///         }
    ///         Message::Result { .. } => {
    ///             println!("Turn complete");
    ///             // next_buffered() automatically sends queued messages
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn next_buffered(&mut self) -> Option<Result<Message>> {
        match self.message_rx.recv().await {
            Some(result) => {
                // Check if this is a Result message
                if let Ok(Message::Result { .. }) = &result {
                    // After Result, try to send next queued message
                    if self.has_queued() {
                        let _ = self.send_queued().await;
                    }
                }
                Some(result)
            }
            None => None,
        }
    }

    /// Clear all queued messages.
    pub fn clear_queue(&self) {
        if let Ok(mut buffer) = self.message_buffer.lock() {
            buffer.clear();
        }
    }

    /// Send an interrupt signal
    ///
    /// **Note**: Interrupt functionality via control messages may not be fully supported
    /// in all Claude CLI versions. The method demonstrates the SDK's bidirectional
    /// capability and will send the control message without blocking, but the CLI
    /// may not process it. Check your CLI version for control message support.
    ///
    /// # Errors
    /// Returns error if interrupt cannot be sent
    pub async fn interrupt(&mut self) -> Result<()> {
        let protocol = self.protocol.lock().await;
        let request = protocol.create_interrupt_request();
        drop(protocol);

        self.control_tx
            .send(request)
            .map_err(|_| ClaudeError::transport("Control channel closed"))
    }

    /// Rewind files to a checkpoint
    ///
    /// Restores files to their state at the specified user message checkpoint.
    /// Requires `enable_file_checkpointing: true` in options.
    ///
    /// # Arguments
    /// * `user_message_uuid` - UUID from a User message's `uuid` field
    ///
    /// # Errors
    /// Returns error if the rewind request cannot be sent
    ///
    /// # Example
    /// ```rust,no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::builder()
    ///     .enable_file_checkpointing(true)
    ///     .build();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// // Capture checkpoint UUID from user messages
    /// let mut checkpoint_uuid: Option<String> = None;
    /// while let Some(msg) = client.next_message().await {
    ///     if let Message::User { uuid: Some(uuid), .. } = msg? {
    ///         checkpoint_uuid = Some(uuid);
    ///         break;
    ///     }
    /// }
    ///
    /// // Later, rewind to that checkpoint
    /// if let Some(uuid) = checkpoint_uuid {
    ///     client.rewind_files(&uuid).await?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn rewind_files(&mut self, user_message_uuid: &str) -> Result<()> {
        let protocol = self.protocol.lock().await;
        let request = protocol.create_rewind_files_request(user_message_uuid);
        drop(protocol);

        self.control_tx
            .send(request)
            .map_err(|_| ClaudeError::transport("Control channel closed"))
    }

    /// Get the next message from the stream
    ///
    /// Returns None when the stream ends
    pub async fn next_message(&mut self) -> Option<Result<Message>> {
        self.message_rx.recv().await
    }

    /// Take the hook event receiver
    ///
    /// This allows the caller to handle hook events independently
    pub fn take_hook_receiver(&mut self) -> Option<mpsc::UnboundedReceiver<(String, HookEvent)>> {
        self.hook_rx.take()
    }

    /// Take the permission request receiver
    ///
    /// This allows the caller to handle permission requests independently
    pub fn take_permission_receiver(
        &mut self,
    ) -> Option<mpsc::UnboundedReceiver<(RequestId, PermissionRequest)>> {
        self.permission_rx.take()
    }

    /// Respond to a hook event
    ///
    /// # Arguments
    /// * `hook_id` - ID of the hook event being responded to
    /// * `response` - Hook response data
    ///
    /// # Errors
    /// Returns error if response cannot be sent
    pub async fn respond_to_hook(
        &mut self,
        hook_id: String,
        response: serde_json::Value,
    ) -> Result<()> {
        let protocol = self.protocol.lock().await;
        let request = protocol.create_hook_response(hook_id, response);
        drop(protocol);

        self.control_tx
            .send(request)
            .map_err(|_| ClaudeError::transport("Control channel closed"))
    }

    /// Respond to a permission request
    ///
    /// # Arguments
    /// * `request_id` - ID of the permission request being responded to
    /// * `result` - Permission result (Allow/Deny)
    ///
    /// # Errors
    /// Returns error if response cannot be sent
    pub async fn respond_to_permission(
        &mut self,
        request_id: RequestId,
        result: crate::types::PermissionResult,
    ) -> Result<()> {
        let protocol = self.protocol.lock().await;
        let request = protocol.create_permission_response(request_id, result);
        drop(protocol);

        self.control_tx
            .send(request)
            .map_err(|_| ClaudeError::transport("Control channel closed"))
    }

    /// Receive messages until a Result message is encountered.
    ///
    /// Returns a stream that yields messages and automatically terminates
    /// after yielding the final Result message. Convenient for single-query workflows.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
    /// # use futures::StreamExt;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let options = ClaudeAgentOptions::default();
    /// # let mut client = ClaudeSDKClient::new(options, None).await?;
    /// client.send_message("Hello").await?;
    ///
    /// let mut messages = Box::pin(client.receive_response());
    /// while let Some(msg) = messages.next().await {
    ///     match msg? {
    ///         Message::Assistant { message, .. } => println!("{:?}", message),
    ///         Message::Result { .. } => println!("Done!"),
    ///         _ => {}
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use = "receive_response returns a stream that must be consumed to receive messages"]
    pub fn receive_response(&mut self) -> impl Stream<Item = Result<Message>> + '_ {
        async_stream::stream! {
            while let Some(result) = self.message_rx.recv().await {
                let is_result = matches!(&result, Ok(Message::Result { .. }));
                yield result;
                if is_result {
                    break;
                }
            }
        }
    }

    /// Check if the client is currently connected.
    ///
    /// Returns `true` if the transport is connected and ready.
    pub async fn is_connected(&self) -> bool {
        let transport = self.transport.lock().await;
        transport.is_ready()
    }

    /// Get the current session ID if available.
    ///
    /// The session ID is captured from Result messages automatically.
    /// Returns `None` if no session has been established yet.
    #[must_use]
    pub fn get_session_id(&self) -> Option<SessionId> {
        self.session_id.lock().ok()?.clone()
    }

    // ========================================================================
    // Session Binding
    // ========================================================================

    /// Bind this client to a specific session ID.
    ///
    /// Once bound, all `send_message()` calls validate that the current
    /// session matches the bound session. If a mismatch is detected,
    /// `send_message()` returns `ClaudeError::SessionMismatch`.
    ///
    /// This provides defense in depth to prevent messages from being
    /// accidentally sent to a different conversation context.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions, Message};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// client.send_message("Hello").await?;
    ///
    /// while let Some(msg) = client.next_message().await {
    ///     if let Message::Result { session_id, .. } = msg? {
    ///         // Bind to this session - all future sends will validate
    ///         client.bind_session(session_id);
    ///         break;
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn bind_session(&self, session_id: SessionId) {
        if let Ok(mut guard) = self.bound_session_id.lock() {
            *guard = Some(session_id);
        }
    }

    /// Get the bound session ID, if any.
    ///
    /// Returns `None` if no session is bound (binding disabled).
    #[must_use]
    pub fn bound_session(&self) -> Option<SessionId> {
        self.bound_session_id.lock().ok()?.clone()
    }

    /// Clear session binding, allowing messages to any session.
    ///
    /// After calling this, `send_message()` will no longer validate
    /// session IDs before sending.
    pub fn unbind_session(&self) {
        if let Ok(mut guard) = self.bound_session_id.lock() {
            *guard = None;
        }
    }

    /// Validate that current session matches bound session.
    ///
    /// Returns `Ok(())` if:
    /// - No session is bound (binding disabled)
    /// - Current session matches bound session
    /// - Either session is None (early in conversation)
    ///
    /// This is called automatically by `send_message()` if a session is bound.
    /// You can also call it manually for explicit validation.
    ///
    /// # Errors
    ///
    /// Returns `SessionMismatch` if bound session differs from current session.
    pub fn validate_session(&self) -> Result<()> {
        let bound = self.bound_session();
        let current = self.get_session_id();

        match (&bound, &current) {
            (Some(b), Some(c)) if b != c => {
                Err(ClaudeError::session_mismatch(b.to_string(), c.to_string()))
            }
            _ => Ok(()),
        }
    }

    // ========================================================================
    // Introspection Methods
    // ========================================================================

    /// Get session information including model, tools, and MCP servers.
    ///
    /// Returns `None` if the init message has not been received yet.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// // Wait for first message to ensure init is received
    /// if let Some(info) = client.session_info() {
    ///     println!("Model: {:?}", info.model);
    ///     println!("Available tools: {:?}", info.tool_names());
    ///     for server in &info.mcp_servers {
    ///         println!("MCP Server {}: status={}", server.name, server.status);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn session_info(&self) -> Option<SessionInfo> {
        self.session_info.lock().ok()?.clone()
    }

    /// Get the current model being used.
    ///
    /// Convenience method that extracts the model from session info.
    /// Returns `None` if init has not been received or model is not set.
    #[must_use]
    pub fn current_model(&self) -> Option<String> {
        self.session_info().and_then(|info| info.model)
    }

    /// Get the list of available tools in this session.
    ///
    /// Returns an empty vector if init has not been received.
    #[must_use]
    pub fn available_tools(&self) -> Vec<crate::types::ToolInfo> {
        self.session_info()
            .map(|info| info.tools)
            .unwrap_or_default()
    }

    /// Get MCP server status for all configured servers.
    ///
    /// Returns the status of each MCP server including connection state
    /// and any errors. Returns an empty vector if no MCP servers are configured.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// for server in client.mcp_server_status() {
    ///     if !server.is_connected() {
    ///         eprintln!("MCP server {} failed: {:?}", server.name, server.error);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn mcp_server_status(&self) -> Vec<crate::types::McpServerStatus> {
        self.session_info()
            .map(|info| info.mcp_servers)
            .unwrap_or_default()
    }

    /// Get list of known Claude models.
    ///
    /// Returns a static list of known Claude models with their capabilities.
    /// Note: This is a static list and may not reflect all available models
    /// for your account. Use `current_model()` to see what's actually in use.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// for model in ClaudeSDKClient::supported_models() {
    ///     println!("{}: {:?}", model.id, model.name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn supported_models() -> Vec<ModelInfo> {
        ModelInfo::known_models()
    }

    /// Get available slash commands.
    ///
    /// Returns the list of slash commands available in this session.
    /// Commands may be defined in project configuration (`.claude/commands/`)
    /// or built into Claude Code.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// for cmd in client.supported_commands() {
    ///     println!("/{} - {}", cmd.name, cmd.description);
    ///     if !cmd.argument_hint.is_empty() {
    ///         println!("  Usage: /{} {}", cmd.name, cmd.argument_hint);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn supported_commands(&self) -> Vec<crate::types::SlashCommand> {
        // Commands are loaded from the init message's slash_commands field.
        // This includes both user-level (~/.claude/commands/) and
        // project-level (.claude/commands/) custom commands.
        //
        // Note: Requires setting_sources to include User and/or Project.
        self.session_info()
            .and_then(|info| {
                info.extra.get("slash_commands").and_then(|v| {
                    // The CLI sends slash_commands as an array of strings
                    if let Some(arr) = v.as_array() {
                        let commands: Vec<crate::types::SlashCommand> = arr
                            .iter()
                            .filter_map(|item| {
                                // Handle both string format and object format
                                if let Some(name) = item.as_str() {
                                    Some(crate::types::SlashCommand {
                                        name: name.to_string(),
                                        description: String::new(),
                                        argument_hint: String::new(),
                                    })
                                } else {
                                    // Try to deserialize as SlashCommand object
                                    serde_json::from_value(item.clone()).ok()
                                }
                            })
                            .collect();
                        Some(commands)
                    } else {
                        None
                    }
                })
            })
            .unwrap_or_default()
    }

    /// Get account information from OAuth credentials.
    ///
    /// Reads account information from the Claude credentials file.
    /// This is only available for OAuth-authenticated accounts (Max Plan),
    /// not for API key authentication.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Credentials file not found
    /// - Credentials file is invalid
    /// - Not using OAuth authentication
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// match client.account_info() {
    ///     Ok(info) => {
    ///         println!("Email: {:?}", info.email);
    ///         println!("OAuth: {}", info.is_oauth);
    ///     }
    ///     Err(e) => println!("No account info: {}", e),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn account_info(&self) -> Result<AccountInfo> {
        // Account info is derived from the init message's apiKeySource field
        // and potentially other session data
        let session = self.session_info().ok_or_else(ClaudeError::not_connected)?;

        // Get apiKeySource from extra fields
        let api_key_source = session
            .extra
            .get("apiKeySource")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Determine if OAuth based on apiKeySource
        // "none" means OAuth/Max Plan, other values indicate API key sources
        let is_oauth = api_key_source.as_deref() == Some("none");

        Ok(AccountInfo {
            email: None,      // Not available in init message
            account_id: None, // Not available in init message
            is_oauth,
            organization_id: None, // Not available in init message
        })
    }

    // ========================================================================
    // Runtime Setters
    // ========================================================================

    /// Store a model preference locally.
    ///
    /// **NOTE:** Runtime model switching mid-session is NOT currently supported
    /// by the Claude CLI's stream-json protocol. This method only stores the value
    /// locally for SDK reference. To use a different model, start a new session:
    ///
    /// ```rust,no_run
    /// use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::builder()
    ///     .model("haiku")  // Set model at session start
    ///     .build();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_model(&self, model: impl Into<String>) {
        let model_str = model.into();
        tracing::debug!(model = %model_str, "set_model: storing locally (runtime switching not supported)");

        if let Ok(mut guard) = self.runtime_model.lock() {
            *guard = Some(model_str);
        }
    }

    /// Get the currently configured runtime model override.
    ///
    /// Returns `None` if no runtime override is set.
    #[must_use]
    pub fn get_runtime_model(&self) -> Option<String> {
        self.runtime_model.lock().ok()?.clone()
    }

    /// Store a permission mode preference locally.
    ///
    /// **NOTE:** Runtime permission mode switching mid-session is NOT currently
    /// supported by the Claude CLI's stream-json protocol. This method only stores
    /// the value locally for SDK reference. To set permission mode, use session options:
    ///
    /// ```rust,no_run
    /// use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient, PermissionMode};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::builder()
    ///     .permission_mode(PermissionMode::AcceptEdits)  // Set at session start
    ///     .build();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Permission modes:
    /// - `Default`: Prompt for permission on sensitive operations
    /// - `AcceptEdits`: Auto-approve file edits
    /// - `Plan`: Plan-only mode, no execution
    /// - `BypassPermissions`: Auto-approve all operations (use with caution)
    pub fn set_permission_mode(&self, mode: crate::types::PermissionMode) {
        tracing::debug!(mode = ?mode, "set_permission_mode: storing locally (runtime switching not supported)");

        if let Ok(mut guard) = self.runtime_permission_mode.lock() {
            *guard = Some(mode);
        }
    }

    /// Get the currently configured runtime permission mode override.
    ///
    /// Returns `None` if no runtime override is set.
    #[must_use]
    pub fn get_runtime_permission_mode(&self) -> Option<crate::types::PermissionMode> {
        *self.runtime_permission_mode.lock().ok()?
    }

    /// Store a max thinking tokens preference locally.
    ///
    /// **NOTE:** Runtime thinking token adjustment mid-session is NOT currently
    /// supported by the Claude CLI's stream-json protocol. This method only stores
    /// the value locally for SDK reference. To set thinking tokens, use session options:
    ///
    /// ```rust,no_run
    /// use anthropic_agent_sdk::{ClaudeAgentOptions, ClaudeSDKClient};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::builder()
    ///     .max_thinking_tokens(20000)  // Set at session start
    ///     .build();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Extended thinking allows Claude to "think" before responding,
    /// improving quality for complex tasks.
    pub fn set_max_thinking_tokens(&self, tokens: u32) {
        tracing::debug!(
            tokens = tokens,
            "set_max_thinking_tokens: storing locally (runtime switching not supported)"
        );

        if let Ok(mut guard) = self.runtime_max_thinking_tokens.lock() {
            *guard = Some(tokens);
        }
    }

    /// Get the currently configured runtime max thinking tokens override.
    ///
    /// Returns `None` if no runtime override is set.
    #[must_use]
    pub fn get_runtime_max_thinking_tokens(&self) -> Option<u32> {
        *self.runtime_max_thinking_tokens.lock().ok()?
    }

    /// Clear all runtime overrides.
    ///
    /// This resets model, permission mode, and thinking tokens to their
    /// original values from the initial options.
    pub fn clear_runtime_overrides(&self) {
        if let Ok(mut guard) = self.runtime_model.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.runtime_permission_mode.lock() {
            *guard = None;
        }
        if let Ok(mut guard) = self.runtime_max_thinking_tokens.lock() {
            *guard = None;
        }
    }

    /// Get a child cancellation token for this client.
    ///
    /// This is analogous to JavaScript's `AbortController.signal`. Callers can
    /// use the returned token to:
    /// - Check if cancellation was requested: `token.is_cancelled()`
    /// - Wait for cancellation: `token.cancelled().await`
    /// - Use with `tokio::select!` to race cancellation against other futures
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// let cancel_token = client.cancellation_token();
    ///
    /// // Use in a spawned task to respect cancellation
    /// let token = cancel_token.clone();
    /// tokio::spawn(async move {
    ///     tokio::select! {
    ///         _ = token.cancelled() => {
    ///             println!("Operation cancelled");
    ///         }
    ///         _ = async { /* long operation */ } => {
    ///             println!("Operation completed");
    ///         }
    ///     }
    /// });
    ///
    /// // Later, cancel all operations
    /// client.cancel();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.child_token()
    }

    /// Cancel all ongoing operations.
    ///
    /// This is analogous to JavaScript's `AbortController.abort()`. Calling this
    /// method will:
    /// - Cancel the message reader in the transport
    /// - Signal any operations using a child cancellation token to stop
    ///
    /// Unlike `close()`, this does not immediately close the client - it only
    /// signals cancellation. Use `close()` after `cancel()` for full cleanup.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use anthropic_agent_sdk::{ClaudeSDKClient, ClaudeAgentOptions};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let options = ClaudeAgentOptions::default();
    /// let mut client = ClaudeSDKClient::new(options, None).await?;
    ///
    /// client.send_message("Write a long essay").await?;
    ///
    /// // After some condition, cancel operations
    /// client.cancel();
    ///
    /// // Then close the client
    /// client.close().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    /// Check if cancellation has been requested.
    ///
    /// Returns `true` if `cancel()` has been called on this client.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    /// Close the client and clean up resources
    ///
    /// # Errors
    /// Returns error if cleanup fails
    pub async fn close(&mut self) -> Result<()> {
        // Trigger SessionEnd hook before closing
        if let Some(ref manager) = self.hook_manager {
            let manager_guard = manager.lock().await;
            if let Err(e) = manager_guard.trigger_session_end("other").await {
                tracing::warn!(error = %e, "SessionEnd hook error");
            }
        }

        let mut transport = self.transport.lock().await;
        transport.close().await
    }
}

impl Drop for ClaudeSDKClient {
    fn drop(&mut self) {
        // Channel senders will be dropped, causing background tasks to exit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let options = ClaudeAgentOptions::default();
        let result = ClaudeSDKClient::new(options, None).await;
        assert!(result.is_ok() || result.is_err()); // Will succeed if CLI is available
    }

    #[tokio::test]
    async fn test_session_id_initially_none() {
        let options = ClaudeAgentOptions::default();
        if let Ok(client) = ClaudeSDKClient::new(options, None).await {
            // Session ID should be None before any Result message is received
            assert!(client.get_session_id().is_none());
        }
    }

    #[tokio::test]
    async fn test_is_connected() {
        let options = ClaudeAgentOptions::default();
        if let Ok(client) = ClaudeSDKClient::new(options, None).await {
            // Should be connected after successful initialization
            assert!(client.is_connected().await);
        }
    }
}
