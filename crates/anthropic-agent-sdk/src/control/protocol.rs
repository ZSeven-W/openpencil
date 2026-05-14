//! Control protocol implementation for bidirectional communication
//!
//! This module provides the protocol handler and message types for the control
//! protocol used in bidirectional communication with Claude Code CLI.
//!
//! # Overview
//!
//! The control protocol enables:
//! - Request/response communication
//! - Hook invocations from CLI to SDK
//! - Permission requests from CLI to SDK
//! - Protocol initialization and capability negotiation
//!
//! # Example: Basic Protocol Usage
//!
//! ```rust
//! use anthropic_agent_sdk::control::ProtocolHandler;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let handler = ProtocolHandler::new();
//!
//! // Create an initialization request
//! let init_req = handler.create_init_request();
//! assert_eq!(init_req.protocol_version, "1.0");
//!
//! // After receiving init response, mark as initialized
//! handler.set_initialized(true);
//!
//! // Create control requests
//! let interrupt_req = handler.create_interrupt_request();
//! let msg_req = handler.create_send_message_request("Hello!".to_string());
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Handling Hook Events
//!
//! ```rust
//! use anthropic_agent_sdk::control::ProtocolHandler;
//! use tokio::sync::mpsc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut handler = ProtocolHandler::new();
//!
//! // Set up hook channel
//! let (hook_tx, mut hook_rx) = mpsc::unbounded_channel();
//! handler.set_hook_channel(hook_tx);
//!
//! // When a hook event arrives, it will be sent to hook_rx
//! // You can then process it and send a response
//! tokio::spawn(async move {
//!     while let Some((hook_id, event)) = hook_rx.recv().await {
//!         println!("Received hook: {} {:?}", hook_id, event);
//!         // Process hook and create response...
//!     }
//! });
//! # Ok(())
//! # }
//! ```
//!
//! # Example: Serialization
//!
//! ```rust
//! use anthropic_agent_sdk::control::{ControlMessage, ProtocolHandler};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let handler = ProtocolHandler::new();
//! let request = handler.create_interrupt_request();
//! let message = ControlMessage::Request(request);
//!
//! // Serialize to JSON
//! let json = handler.serialize_message(&message)?;
//! assert!(json.ends_with('\n'));
//!
//! // Deserialize from JSON
//! let parsed = handler.deserialize_message(json.trim())?;
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::sync::{Mutex, mpsc, oneshot};

use crate::error::{ClaudeError, Result};
use crate::types::{HookEvent, PermissionRequest, PermissionResult, RequestId};

/// Control message envelope for all protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ControlMessage {
    /// Request from SDK to CLI
    #[serde(rename = "request")]
    Request(ControlRequest),
    /// Response from CLI to SDK
    #[serde(rename = "response")]
    Response(ControlResponse),
    /// Initialization request
    #[serde(rename = "init")]
    Init(InitRequest),
    /// Initialization response
    #[serde(rename = "init_response")]
    InitResponse(InitResponse),
}

/// Request from SDK to CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum ControlRequest {
    /// Interrupt the current operation
    #[serde(rename = "interrupt")]
    Interrupt {
        /// Unique request identifier
        id: RequestId,
    },
    /// Send a message to Claude
    #[serde(rename = "send_message")]
    SendMessage {
        /// Unique request identifier
        id: RequestId,
        /// Message content to send
        content: String,
    },
    /// Respond to a hook invocation
    #[serde(rename = "hook_response")]
    HookResponse {
        /// Unique request identifier
        id: RequestId,
        /// Hook event ID being responded to
        hook_id: String,
        /// Hook response data
        response: serde_json::Value,
    },
    /// Respond to a permission request
    #[serde(rename = "permission_response")]
    PermissionResponse {
        /// Unique request identifier
        id: RequestId,
        /// Permission request ID being responded to
        request_id: RequestId,
        /// Permission result (Allow/Deny)
        result: PermissionResult,
    },
    /// Set the model for subsequent messages
    #[serde(rename = "set_model")]
    SetModel {
        /// Unique request identifier
        id: RequestId,
        /// Model name or alias (e.g., "haiku", "sonnet", "opus", or full model ID)
        model: Option<String>,
    },
    /// Set the permission mode
    #[serde(rename = "set_permission_mode")]
    SetPermissionMode {
        /// Unique request identifier
        id: RequestId,
        /// Permission mode to use
        mode: String,
    },
    /// Set the maximum thinking tokens
    #[serde(rename = "set_max_thinking_tokens")]
    SetMaxThinkingTokens {
        /// Unique request identifier
        id: RequestId,
        /// Maximum thinking tokens (null to disable)
        max_thinking_tokens: Option<u32>,
    },
    /// Rewind files to a checkpoint
    ///
    /// Restores files to their state at the specified user message.
    /// Requires `enable_file_checkpointing: true` in options.
    #[serde(rename = "rewind_files")]
    RewindFiles {
        /// Unique request identifier
        id: RequestId,
        /// UUID of the user message checkpoint to rewind to
        user_message_uuid: String,
    },
}

/// Response from CLI to SDK
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ControlResponse {
    /// Successful response
    #[serde(rename = "success")]
    Success {
        /// Request ID this responds to
        id: RequestId,
        /// Optional response data
        data: Option<serde_json::Value>,
    },
    /// Error response
    #[serde(rename = "error")]
    Error {
        /// Request ID this responds to
        id: RequestId,
        /// Error message
        message: String,
        /// Error code
        code: Option<String>,
    },
    /// Hook invocation from CLI
    #[serde(rename = "hook")]
    Hook {
        /// Hook invocation ID
        id: String,
        /// Hook event details
        event: HookEvent,
    },
    /// Permission request from CLI
    #[serde(rename = "permission")]
    Permission {
        /// Permission request ID
        id: RequestId,
        /// Permission request details
        request: PermissionRequest,
    },
}

/// Initialization request sent from SDK to CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitRequest {
    /// Protocol version
    pub protocol_version: String,
    /// SDK version
    pub sdk_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
}

/// Client capabilities for negotiation
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Supports bidirectional communication
    pub bidirectional: bool,
    /// Supports hooks
    pub hooks: bool,
    /// Supports permissions
    pub permissions: bool,
    /// Supports interrupts
    pub interrupts: bool,
}

/// Initialization response from CLI to SDK
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitResponse {
    /// Protocol version accepted
    pub protocol_version: String,
    /// CLI version
    pub cli_version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Session ID for this connection
    pub session_id: String,
}

/// Server capabilities advertised by CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Supports streaming responses
    pub streaming: bool,
    /// Supports tool use
    pub tools: bool,
    /// Supports MCP servers
    pub mcp: bool,
}

/// Pending request awaiting response
struct PendingRequest {
    /// Response channel
    response_tx: oneshot::Sender<ControlResponse>,
}

/// Protocol handler for managing control protocol communication
pub struct ProtocolHandler {
    /// Request ID counter
    next_request_id: Arc<AtomicU64>,
    /// Pending requests awaiting responses
    pending_requests: Arc<Mutex<HashMap<RequestId, PendingRequest>>>,
    /// Initialized flag
    initialized: Arc<AtomicBool>,
    /// Hook callback channel
    hook_tx: Option<mpsc::UnboundedSender<(String, HookEvent)>>,
    /// Permission callback channel
    permission_tx: Option<mpsc::UnboundedSender<(RequestId, PermissionRequest)>>,
}

impl ProtocolHandler {
    /// Create a new protocol handler
    #[must_use]
    pub fn new() -> Self {
        Self {
            next_request_id: Arc::new(AtomicU64::new(1)),
            pending_requests: Arc::new(Mutex::new(HashMap::new())),
            initialized: Arc::new(AtomicBool::new(false)),
            hook_tx: None,
            permission_tx: None,
        }
    }

    /// Set hook callback channel
    pub fn set_hook_channel(&mut self, tx: mpsc::UnboundedSender<(String, HookEvent)>) {
        self.hook_tx = Some(tx);
    }

    /// Set permission callback channel
    pub fn set_permission_channel(
        &mut self,
        tx: mpsc::UnboundedSender<(RequestId, PermissionRequest)>,
    ) {
        self.permission_tx = Some(tx);
    }

    /// Check if protocol is initialized
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Set protocol as initialized (for cases where no handshake is needed)
    pub fn set_initialized(&self, value: bool) {
        self.initialized.store(value, Ordering::SeqCst);
    }

    /// Generate next request ID
    fn next_id(&self) -> RequestId {
        let id = self.next_request_id.fetch_add(1, Ordering::SeqCst);
        RequestId::new(format!("req-{id}"))
    }

    /// Create initialization request
    #[must_use]
    pub fn create_init_request(&self) -> InitRequest {
        InitRequest {
            protocol_version: "1.0".to_string(),
            sdk_version: crate::VERSION.to_string(),
            capabilities: ClientCapabilities {
                bidirectional: true,
                hooks: true,
                permissions: true,
                interrupts: true,
            },
        }
    }

    /// Handle initialization response
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if protocol version is unsupported.
    pub fn handle_init_response(&self, response: &InitResponse) -> Result<()> {
        // Validate protocol version
        if response.protocol_version != "1.0" {
            return Err(ClaudeError::protocol_error(format!(
                "Unsupported protocol version: {}",
                response.protocol_version
            )));
        }

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Send a request and wait for response
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if protocol is not initialized.
    pub async fn send_request(
        &self,
        request: ControlRequest,
    ) -> Result<oneshot::Receiver<ControlResponse>> {
        if !self.is_initialized() {
            return Err(ClaudeError::protocol_error(
                "Protocol not initialized - call init first",
            ));
        }

        let id = Self::get_request_id(&request);
        let (response_tx, response_rx) = oneshot::channel();

        let pending = PendingRequest { response_tx };

        {
            let mut pending_requests = self.pending_requests.lock().await;
            pending_requests.insert(id, pending);
        }

        Ok(response_rx)
    }

    /// Extract request ID from a control request
    fn get_request_id(request: &ControlRequest) -> RequestId {
        match request {
            ControlRequest::Interrupt { id }
            | ControlRequest::SendMessage { id, .. }
            | ControlRequest::HookResponse { id, .. }
            | ControlRequest::PermissionResponse { id, .. }
            | ControlRequest::SetModel { id, .. }
            | ControlRequest::SetPermissionMode { id, .. }
            | ControlRequest::SetMaxThinkingTokens { id, .. }
            | ControlRequest::RewindFiles { id, .. } => id.clone(),
        }
    }

    /// Handle incoming control response
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if hook or permission channel is closed.
    pub async fn handle_response(&self, response: ControlResponse) -> Result<()> {
        match &response {
            ControlResponse::Success { id, .. } | ControlResponse::Error { id, .. } => {
                let mut pending_requests = self.pending_requests.lock().await;
                if let Some(pending) = pending_requests.remove(id) {
                    let _ = pending.response_tx.send(response);
                }
                Ok(())
            }
            ControlResponse::Hook { id, event } => {
                if let Some(ref tx) = self.hook_tx {
                    tx.send((id.clone(), *event))
                        .map_err(|_| ClaudeError::protocol_error("Hook channel closed"))?;
                }
                Ok(())
            }
            ControlResponse::Permission { id, request } => {
                if let Some(ref tx) = self.permission_tx {
                    tx.send((id.clone(), request.clone()))
                        .map_err(|_| ClaudeError::protocol_error("Permission channel closed"))?;
                }
                Ok(())
            }
        }
    }

    /// Create interrupt request
    #[must_use]
    pub fn create_interrupt_request(&self) -> ControlRequest {
        ControlRequest::Interrupt { id: self.next_id() }
    }

    /// Create send message request
    #[must_use]
    pub fn create_send_message_request(&self, content: impl Into<String>) -> ControlRequest {
        ControlRequest::SendMessage {
            id: self.next_id(),
            content: content.into(),
        }
    }

    /// Create hook response
    #[must_use]
    pub fn create_hook_response(
        &self,
        hook_id: impl Into<String>,
        response: serde_json::Value,
    ) -> ControlRequest {
        ControlRequest::HookResponse {
            id: self.next_id(),
            hook_id: hook_id.into(),
            response,
        }
    }

    /// Create permission response
    #[must_use]
    pub fn create_permission_response(
        &self,
        request_id: RequestId,
        result: PermissionResult,
    ) -> ControlRequest {
        ControlRequest::PermissionResponse {
            id: self.next_id(),
            request_id,
            result,
        }
    }

    /// Create set model request
    #[must_use]
    pub fn create_set_model_request(&self, model: Option<String>) -> ControlRequest {
        ControlRequest::SetModel {
            id: self.next_id(),
            model,
        }
    }

    /// Create set permission mode request
    #[must_use]
    pub fn create_set_permission_mode_request(&self, mode: impl Into<String>) -> ControlRequest {
        ControlRequest::SetPermissionMode {
            id: self.next_id(),
            mode: mode.into(),
        }
    }

    /// Create set max thinking tokens request
    #[must_use]
    pub fn create_set_max_thinking_tokens_request(
        &self,
        max_thinking_tokens: Option<u32>,
    ) -> ControlRequest {
        ControlRequest::SetMaxThinkingTokens {
            id: self.next_id(),
            max_thinking_tokens,
        }
    }

    /// Create a rewind files request
    ///
    /// Rewinds files to their state at the specified checkpoint.
    /// The `user_message_uuid` comes from the `uuid` field on User messages
    /// when `enable_file_checkpointing` is enabled.
    #[must_use]
    pub fn create_rewind_files_request(
        &self,
        user_message_uuid: impl Into<String>,
    ) -> ControlRequest {
        ControlRequest::RewindFiles {
            id: self.next_id(),
            user_message_uuid: user_message_uuid.into(),
        }
    }

    /// Serialize control message to JSON
    ///
    /// # Errors
    ///
    /// Returns `JsonEncode` if serialization fails.
    pub fn serialize_message(&self, message: &ControlMessage) -> Result<String> {
        serde_json::to_string(message)
            .map(|s| format!("{s}\n"))
            .map_err(|e| ClaudeError::json_encode(format!("Failed to serialize message: {e}")))
    }

    /// Deserialize control message from JSON
    ///
    /// # Errors
    ///
    /// Returns `JsonDecodeMsg` if parsing fails.
    pub fn deserialize_message(&self, json: &str) -> Result<ControlMessage> {
        serde_json::from_str(json).map_err(|e| {
            ClaudeError::json_decode_msg(format!("Failed to deserialize message: {e}"))
        })
    }
}

impl Default for ProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolName;

    #[test]
    fn test_request_id_generation() {
        let handler = ProtocolHandler::new();
        let id1 = handler.next_id();
        let id2 = handler.next_id();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_init_request_creation() {
        let handler = ProtocolHandler::new();
        let init_req = handler.create_init_request();
        assert_eq!(init_req.protocol_version, "1.0");
        assert!(init_req.capabilities.bidirectional);
    }

    #[test]
    fn test_serialize_deserialize() {
        let handler = ProtocolHandler::new();
        let request = handler.create_interrupt_request();
        let message = ControlMessage::Request(request);

        let serialized = handler.serialize_message(&message).unwrap();
        let deserialized = handler.deserialize_message(serialized.trim()).unwrap();

        match deserialized {
            ControlMessage::Request(ControlRequest::Interrupt { .. }) => {}
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_deserialize_invalid_json() {
        let handler = ProtocolHandler::new();
        let result = handler.deserialize_message("not valid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_invalid_message_structure() {
        let handler = ProtocolHandler::new();
        let invalid = r#"{"type":"unknown_type"}"#;
        let result = handler.deserialize_message(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_missing_fields() {
        let handler = ProtocolHandler::new();
        let missing = r#"{"type":"request"}"#;
        let result = handler.deserialize_message(missing);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_response_with_missing_pending_request() {
        let handler = ProtocolHandler::new();
        handler.set_initialized(true);

        // Create a response for a request that was never sent
        let response = ControlResponse::Success {
            id: RequestId::new("non-existent-req"),
            data: None,
        };

        // Should not error, just ignore
        let result = handler.handle_response(response).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_hook_response_without_channel() {
        let handler = ProtocolHandler::new();

        // Try to handle hook response without setting up channel
        let response = ControlResponse::Hook {
            id: "hook-1".to_string(),
            event: HookEvent::PreToolUse,
        };

        // Should not error, just no-op
        let result = handler.handle_response(response).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_permission_response_without_channel() {
        let handler = ProtocolHandler::new();

        // Try to handle permission response without setting up channel
        let response = ControlResponse::Permission {
            id: RequestId::new("perm-1"),
            request: PermissionRequest {
                tool_name: ToolName::new("test"),
                tool_input: serde_json::json!({}),
                context: crate::types::ToolPermissionContext::new(vec![]),
            },
        };

        // Should not error, just no-op
        let result = handler.handle_response(response).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_response_with_wrong_version() {
        let handler = ProtocolHandler::new();

        let init_response = InitResponse {
            protocol_version: "999.0".to_string(),
            cli_version: "1.0.0".to_string(),
            capabilities: ServerCapabilities {
                streaming: true,
                tools: true,
                mcp: true,
            },
            session_id: "test".to_string(),
        };

        let result = handler.handle_init_response(&init_response);
        assert!(result.is_err());
        assert!(!handler.is_initialized());
    }

    #[tokio::test]
    async fn test_send_request_without_init() {
        let handler = ProtocolHandler::new();
        assert!(!handler.is_initialized());

        let request = handler.create_interrupt_request();
        let result = handler.send_request(request).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_all_request_types() {
        let handler = ProtocolHandler::new();

        // Test Interrupt
        let req = handler.create_interrupt_request();
        let msg = ControlMessage::Request(req);
        assert!(handler.serialize_message(&msg).is_ok());

        // Test SendMessage
        let req = handler.create_send_message_request("test".to_string());
        let msg = ControlMessage::Request(req);
        assert!(handler.serialize_message(&msg).is_ok());

        // Test HookResponse
        let req = handler.create_hook_response("hook-1".to_string(), serde_json::json!({}));
        let msg = ControlMessage::Request(req);
        assert!(handler.serialize_message(&msg).is_ok());

        // Test PermissionResponse
        let req = handler.create_permission_response(
            RequestId::new("req-1"),
            crate::types::PermissionResult::Allow(crate::types::PermissionResultAllow {
                updated_input: None,
                updated_permissions: None,
            }),
        );
        let msg = ControlMessage::Request(req);
        assert!(handler.serialize_message(&msg).is_ok());
    }

    #[test]
    fn test_serialize_all_response_types() {
        let handler = ProtocolHandler::new();

        // Test Success
        let resp = ControlResponse::Success {
            id: RequestId::new("req-1"),
            data: Some(serde_json::json!({"result": "ok"})),
        };
        let msg = ControlMessage::Response(resp);
        assert!(handler.serialize_message(&msg).is_ok());

        // Test Error
        let resp = ControlResponse::Error {
            id: RequestId::new("req-1"),
            message: "test error".to_string(),
            code: Some("ERR_TEST".to_string()),
        };
        let msg = ControlMessage::Response(resp);
        assert!(handler.serialize_message(&msg).is_ok());

        // Test Hook
        let resp = ControlResponse::Hook {
            id: "hook-1".to_string(),
            event: HookEvent::PreToolUse,
        };
        let msg = ControlMessage::Response(resp);
        assert!(handler.serialize_message(&msg).is_ok());

        // Test Permission
        let resp = ControlResponse::Permission {
            id: RequestId::new("perm-1"),
            request: PermissionRequest {
                tool_name: ToolName::new("test"),
                tool_input: serde_json::json!({}),
                context: crate::types::ToolPermissionContext::new(vec![]),
            },
        };
        let msg = ControlMessage::Response(resp);
        assert!(handler.serialize_message(&msg).is_ok());
    }

    #[test]
    fn test_get_request_id() {
        let interrupt = ControlRequest::Interrupt {
            id: RequestId::new("id1"),
        };
        assert_eq!(ProtocolHandler::get_request_id(&interrupt).as_str(), "id1");

        let send_msg = ControlRequest::SendMessage {
            id: RequestId::new("id2"),
            content: "test".to_string(),
        };
        assert_eq!(ProtocolHandler::get_request_id(&send_msg).as_str(), "id2");

        let hook_resp = ControlRequest::HookResponse {
            id: RequestId::new("id3"),
            hook_id: "hook".to_string(),
            response: serde_json::json!({}),
        };
        assert_eq!(ProtocolHandler::get_request_id(&hook_resp).as_str(), "id3");

        let perm_resp = ControlRequest::PermissionResponse {
            id: RequestId::new("id4"),
            request_id: RequestId::new("perm"),
            result: crate::types::PermissionResult::Allow(crate::types::PermissionResultAllow {
                updated_input: None,
                updated_permissions: None,
            }),
        };
        assert_eq!(ProtocolHandler::get_request_id(&perm_resp).as_str(), "id4");
    }
}
