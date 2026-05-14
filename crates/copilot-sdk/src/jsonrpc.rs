// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! JSON-RPC 2.0 client for the Copilot SDK.
//!
//! Provides bidirectional JSON-RPC communication over any transport.

use crate::error::{CopilotError, Result};
use crate::transport::{MessageFramer, MessageReader, MessageWriter, StdioTransport, Transport};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};

// =============================================================================
// JSON-RPC 2.0 Message Types
// =============================================================================

/// JSON-RPC request ID (can be string or integer).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    Num(i64),
    Str(String),
}

impl From<i64> for JsonRpcId {
    fn from(n: i64) -> Self {
        Self::Num(n)
    }
}

impl From<String> for JsonRpcId {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}

impl From<&str> for JsonRpcId {
    fn from(s: &str) -> Self {
        Self::Str(s.to_string())
    }
}

/// JSON-RPC 2.0 Request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonRpcId>,
}

impl JsonRpcRequest {
    /// Create a new request.
    pub fn new(method: impl Into<String>, params: Option<Value>, id: Option<JsonRpcId>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
            id,
        }
    }

    /// Create a notification (no id).
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self::new(method, params, None)
    }

    /// Check if this is a notification.
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC 2.0 Error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Create a new error.
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create an error with data.
    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Standard error codes.
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

/// JSON-RPC 2.0 Response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonRpcId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response.
    pub fn success(id: JsonRpcId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: JsonRpcId, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            result: None,
            error: Some(error),
        }
    }

    /// Check if this is an error response.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

// =============================================================================
// Handler Types
// =============================================================================

/// Handler for incoming notifications.
pub type NotificationHandler = Arc<dyn Fn(&str, &Value) + Send + Sync>;

/// Future returned by async request handlers.
pub type RequestHandlerFuture =
    Pin<Box<dyn std::future::Future<Output = std::result::Result<Value, JsonRpcError>> + Send>>;

/// Handler for incoming requests (returns result or error).
pub type RequestHandler = Arc<dyn Fn(&str, &Value) -> RequestHandlerFuture + Send + Sync>;

// =============================================================================
// Pending Request Tracking
// =============================================================================

struct PendingRequest {
    sender: oneshot::Sender<std::result::Result<Value, JsonRpcError>>,
}

// =============================================================================
// Shared State (for background task)
// =============================================================================

struct SharedState<T: Transport> {
    framer: Mutex<MessageFramer<T>>,
    running: AtomicBool,
    pending_requests: RwLock<HashMap<i64, PendingRequest>>,
    notification_handler: RwLock<Option<NotificationHandler>>,
    request_handler: RwLock<Option<RequestHandler>>,
}

// =============================================================================
// JSON-RPC Client
// =============================================================================

/// JSON-RPC 2.0 client with bidirectional communication.
///
/// Features:
/// - Send requests and await responses (with timeout)
/// - Send notifications (fire-and-forget)
/// - Handle incoming notifications via callback
/// - Handle incoming requests (server-to-client calls) via callback
/// - Background read loop with automatic dispatch
pub struct JsonRpcClient<T: Transport> {
    state: Arc<SharedState<T>>,
    next_id: AtomicI64,
    shutdown_tx: Mutex<Option<mpsc::Sender<()>>>,
}

impl<T: Transport + 'static> JsonRpcClient<T> {
    /// Create a new JSON-RPC client wrapping a transport.
    pub fn new(transport: T) -> Self {
        Self {
            state: Arc::new(SharedState {
                framer: Mutex::new(MessageFramer::new(transport)),
                running: AtomicBool::new(false),
                pending_requests: RwLock::new(HashMap::new()),
                notification_handler: RwLock::new(None),
                request_handler: RwLock::new(None),
            }),
            next_id: AtomicI64::new(1),
            shutdown_tx: Mutex::new(None),
        }
    }

    /// Start the background read loop.
    pub async fn start(&self) -> Result<()> {
        if self.state.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        // Clone Arc for the background task
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = async {
                        let mut framer = state.framer.lock().await;
                        framer.read_message().await
                    } => {
                        match result {
                            Ok(message_str) => {
                                if let Ok(message) = serde_json::from_str::<Value>(&message_str) {
                                    Self::dispatch_message(&state, message).await;
                                }
                            }
                            Err(CopilotError::ConnectionClosed) => {
                                state.running.store(false, Ordering::SeqCst);
                                // Fail all pending requests
                                let mut pending = state.pending_requests.write().await;
                                for (_, req) in pending.drain() {
                                    let _ = req.sender.send(Err(JsonRpcError::new(
                                        -32801,
                                        "Connection closed",
                                    )));
                                }
                                break;
                            }
                            Err(_) => {
                                // Continue on other errors if still running
                                if !state.running.load(Ordering::SeqCst) {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the client.
    pub async fn stop(&self) {
        self.state.running.store(false, Ordering::SeqCst);

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(()).await;
        }

        // Fail all pending requests
        let mut pending = self.state.pending_requests.write().await;
        for (_, req) in pending.drain() {
            let _ = req
                .sender
                .send(Err(JsonRpcError::new(-32801, "Connection closed")));
        }
    }

    /// Check if client is running.
    pub fn is_running(&self) -> bool {
        self.state.running.load(Ordering::SeqCst)
    }

    /// Set handler for incoming notifications.
    pub async fn set_notification_handler<F>(&self, handler: F)
    where
        F: Fn(&str, &Value) + Send + Sync + 'static,
    {
        *self.state.notification_handler.write().await = Some(Arc::new(handler));
    }

    /// Set handler for incoming requests.
    pub async fn set_request_handler<F>(&self, handler: F)
    where
        F: Fn(&str, &Value) -> RequestHandlerFuture + Send + Sync + 'static,
    {
        *self.state.request_handler.write().await = Some(Arc::new(handler));
    }

    /// Send a request and await response.
    pub async fn invoke(&self, method: &str, params: Option<Value>) -> Result<Value> {
        self.invoke_with_timeout(method, params, Duration::from_secs(30))
            .await
    }

    /// Send a request with custom timeout.
    pub async fn invoke_with_timeout(
        &self,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.state.pending_requests.write().await;
            pending.insert(id, PendingRequest { sender: tx });
        }

        // Build and send request
        let request = JsonRpcRequest::new(method, params, Some(JsonRpcId::Num(id)));
        let request_json = serde_json::to_string(&request)?;

        if let Err(e) = self.send_raw(&request_json).await {
            // Remove pending request on send failure
            self.state.pending_requests.write().await.remove(&id);
            return Err(e);
        }

        // Wait for response with timeout
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(result))) => Ok(result),
            Ok(Ok(Err(rpc_error))) => Err(CopilotError::JsonRpc {
                code: rpc_error.code,
                message: rpc_error.message,
                data: rpc_error.data,
            }),
            Ok(Err(_)) => {
                // Channel closed
                self.state.pending_requests.write().await.remove(&id);
                Err(CopilotError::ConnectionClosed)
            }
            Err(_) => {
                // Timeout
                self.state.pending_requests.write().await.remove(&id);
                Err(CopilotError::Timeout(timeout))
            }
        }
    }

    /// Send a notification (no response expected).
    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let request = JsonRpcRequest::notification(method, params);
        let request_json = serde_json::to_string(&request)?;
        self.send_raw(&request_json).await
    }

    /// Send a response to an incoming request.
    pub async fn send_response(&self, id: JsonRpcId, result: Value) -> Result<()> {
        let response = JsonRpcResponse::success(id, result);
        let response_json = serde_json::to_string(&response)?;
        self.send_raw(&response_json).await
    }

    /// Send an error response to an incoming request.
    pub async fn send_error_response(&self, id: JsonRpcId, error: JsonRpcError) -> Result<()> {
        let response = JsonRpcResponse::error(id, error);
        let response_json = serde_json::to_string(&response)?;
        self.send_raw(&response_json).await
    }

    /// Send a raw JSON-RPC message.
    async fn send_raw(&self, message: &str) -> Result<()> {
        let mut framer = self.state.framer.lock().await;
        framer.write_message(message).await
    }

    /// Dispatch an incoming message.
    async fn dispatch_message(state: &SharedState<T>, message: Value) {
        // Check if it's a response (has id and result/error, no method)
        if message.get("id").is_some()
            && !message.get("id").map(|v| v.is_null()).unwrap_or(true)
            && (message.get("result").is_some() || message.get("error").is_some())
            && message.get("method").is_none()
        {
            Self::handle_response(state, message).await;
            return;
        }

        // Check if it's a request or notification (has method)
        if message.get("method").is_some() {
            if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(message) {
                if request.is_notification() {
                    Self::handle_notification(state, &request).await;
                } else {
                    Self::handle_request(state, &request).await;
                }
            }
        }
    }

    /// Handle an incoming response.
    async fn handle_response(state: &SharedState<T>, message: Value) {
        // Parse response
        let response: JsonRpcResponse = match serde_json::from_value(message) {
            Ok(r) => r,
            Err(_) => return,
        };

        // Get the ID as i64
        let id = match &response.id {
            Some(JsonRpcId::Num(n)) => *n,
            _ => return, // We only use numeric IDs for outgoing requests
        };

        // Find and remove pending request
        let pending_req = {
            let mut pending = state.pending_requests.write().await;
            pending.remove(&id)
        };

        if let Some(req) = pending_req {
            let result = if let Some(error) = response.error {
                Err(error)
            } else {
                Ok(response.result.unwrap_or(Value::Null))
            };
            let _ = req.sender.send(result);
        }
    }

    /// Handle an incoming notification.
    async fn handle_notification(state: &SharedState<T>, request: &JsonRpcRequest) {
        let handler = state.notification_handler.read().await;
        if let Some(handler) = handler.as_ref() {
            let params = request.params.as_ref().unwrap_or(&Value::Null);
            handler(&request.method, params);
        }
    }

    /// Handle an incoming request.
    async fn handle_request(state: &SharedState<T>, request: &JsonRpcRequest) {
        let id = match &request.id {
            Some(id) => id.clone(),
            None => return, // Not a request
        };

        let handler = state.request_handler.read().await;
        let params = request.params.as_ref().unwrap_or(&Value::Null);

        let response = if let Some(handler) = handler.as_ref() {
            // Call the async handler and await result
            match handler(&request.method, params).await {
                Ok(result) => JsonRpcResponse::success(id, result),
                Err(error) => JsonRpcResponse::error(id, error),
            }
        } else {
            // No handler - respond with method not found
            JsonRpcResponse::error(
                id,
                JsonRpcError::new(
                    JsonRpcError::METHOD_NOT_FOUND,
                    format!("Method not found: {}", request.method),
                ),
            )
        };

        // Send response
        if let Ok(response_json) = serde_json::to_string(&response) {
            let mut framer = state.framer.lock().await;
            let _ = framer.write_message(&response_json).await;
        }
    }
}

// =============================================================================
// Stdio JSON-RPC Client (split read/write paths)
// =============================================================================

/// Shared state for the Stdio JSON-RPC client.
struct StdioSharedState {
    writer: Mutex<MessageWriter<tokio::process::ChildStdin>>,
    running: AtomicBool,
    pending_requests: RwLock<HashMap<i64, PendingRequest>>,
    notification_handler: RwLock<Option<NotificationHandler>>,
    request_handler: RwLock<Option<RequestHandler>>,
}

/// JSON-RPC client for stdio transports with separate read/write paths.
///
/// This client avoids the lock contention issue by using separate mutexes
/// for reading and writing.
pub struct StdioJsonRpcClient {
    state: Arc<StdioSharedState>,
    reader: Mutex<Option<MessageReader<tokio::process::ChildStdout>>>,
    next_id: AtomicI64,
    shutdown_tx: Mutex<Option<mpsc::Sender<()>>>,
}

impl StdioJsonRpcClient {
    /// Create a new stdio JSON-RPC client from a transport.
    pub fn new(transport: StdioTransport) -> Self {
        let (writer, reader) = transport.split();
        Self {
            state: Arc::new(StdioSharedState {
                writer: Mutex::new(MessageWriter::new(writer)),
                running: AtomicBool::new(false),
                pending_requests: RwLock::new(HashMap::new()),
                notification_handler: RwLock::new(None),
                request_handler: RwLock::new(None),
            }),
            reader: Mutex::new(Some(MessageReader::new(reader))),
            next_id: AtomicI64::new(1),
            shutdown_tx: Mutex::new(None),
        }
    }

    /// Start the background read loop.
    pub async fn start(&self) -> Result<()> {
        let reader = self.reader.lock().await.take().ok_or_else(|| {
            CopilotError::InvalidConfig("Reader already taken or client already started".into())
        })?;
        self.start_with_reader(reader).await
    }

    /// Start the background read loop with a specific reader.
    async fn start_with_reader(
        &self,
        mut reader: MessageReader<tokio::process::ChildStdout>,
    ) -> Result<()> {
        if self.state.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        // Clone state for the background task
        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = reader.read_message() => {
                        match result {
                            Ok(message_str) => {
                                if let Ok(message) = serde_json::from_str::<Value>(&message_str) {
                                    Self::dispatch_message(&state, message).await;
                                }
                            }
                            Err(CopilotError::ConnectionClosed) => {
                                state.running.store(false, Ordering::SeqCst);
                                // Fail all pending requests
                                let mut pending = state.pending_requests.write().await;
                                for (_, req) in pending.drain() {
                                    let _ = req.sender.send(Err(JsonRpcError::new(
                                        -32801,
                                        "Connection closed",
                                    )));
                                }
                                break;
                            }
                            Err(_) => {
                                // Continue on other errors if still running
                                if !state.running.load(Ordering::SeqCst) {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the client.
    pub async fn stop(&self) {
        self.state.running.store(false, Ordering::SeqCst);

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(()).await;
        }

        // Fail all pending requests
        let mut pending = self.state.pending_requests.write().await;
        for (_, req) in pending.drain() {
            let _ = req
                .sender
                .send(Err(JsonRpcError::new(-32801, "Connection closed")));
        }
    }

    /// Check if client is running.
    pub fn is_running(&self) -> bool {
        self.state.running.load(Ordering::SeqCst)
    }

    /// Set handler for incoming notifications.
    pub async fn set_notification_handler<F>(&self, handler: F)
    where
        F: Fn(&str, &Value) + Send + Sync + 'static,
    {
        *self.state.notification_handler.write().await = Some(Arc::new(handler));
    }

    /// Set handler for incoming requests.
    pub async fn set_request_handler<F>(&self, handler: F)
    where
        F: Fn(&str, &Value) -> RequestHandlerFuture + Send + Sync + 'static,
    {
        *self.state.request_handler.write().await = Some(Arc::new(handler));
    }

    /// Send a request and await response.
    pub async fn invoke(&self, method: &str, params: Option<Value>) -> Result<Value> {
        self.invoke_with_timeout(method, params, Duration::from_secs(30))
            .await
    }

    /// Send a request with custom timeout.
    pub async fn invoke_with_timeout(
        &self,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.state.pending_requests.write().await;
            pending.insert(id, PendingRequest { sender: tx });
        }

        // Build and send request
        let request = JsonRpcRequest::new(method, params, Some(JsonRpcId::Num(id)));
        let request_json = serde_json::to_string(&request)?;

        if let Err(e) = self.send_raw(&request_json).await {
            // Remove pending request on send failure
            self.state.pending_requests.write().await.remove(&id);
            return Err(e);
        }

        // Wait for response with timeout
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(result))) => Ok(result),
            Ok(Ok(Err(rpc_error))) => Err(CopilotError::JsonRpc {
                code: rpc_error.code,
                message: rpc_error.message,
                data: rpc_error.data,
            }),
            Ok(Err(_)) => {
                // Channel closed
                self.state.pending_requests.write().await.remove(&id);
                Err(CopilotError::ConnectionClosed)
            }
            Err(_) => {
                // Timeout
                self.state.pending_requests.write().await.remove(&id);
                Err(CopilotError::Timeout(timeout))
            }
        }
    }

    /// Send a notification (no response expected).
    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let request = JsonRpcRequest::notification(method, params);
        let request_json = serde_json::to_string(&request)?;
        self.send_raw(&request_json).await
    }

    /// Send a raw JSON-RPC message.
    async fn send_raw(&self, message: &str) -> Result<()> {
        let mut writer = self.state.writer.lock().await;
        writer.write_message(message).await
    }

    /// Dispatch an incoming message.
    async fn dispatch_message(state: &StdioSharedState, message: Value) {
        // Check if it's a response (has id and result/error, no method)
        if message.get("id").is_some()
            && !message.get("id").map(|v| v.is_null()).unwrap_or(true)
            && (message.get("result").is_some() || message.get("error").is_some())
            && message.get("method").is_none()
        {
            Self::handle_response(state, message).await;
            return;
        }

        // Check if it's a request or notification (has method)
        if message.get("method").is_some() {
            if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(message) {
                if request.is_notification() {
                    Self::handle_notification(state, &request).await;
                } else {
                    Self::handle_request(state, &request).await;
                }
            }
        }
    }

    /// Handle an incoming response.
    async fn handle_response(state: &StdioSharedState, message: Value) {
        // Parse response
        let response: JsonRpcResponse = match serde_json::from_value(message) {
            Ok(r) => r,
            Err(_) => return,
        };

        // Get the ID as i64
        let id = match &response.id {
            Some(JsonRpcId::Num(n)) => *n,
            _ => return,
        };

        // Find and remove pending request
        let pending_req = {
            let mut pending = state.pending_requests.write().await;
            pending.remove(&id)
        };

        if let Some(req) = pending_req {
            let result = if let Some(error) = response.error {
                Err(error)
            } else {
                Ok(response.result.unwrap_or(Value::Null))
            };
            let _ = req.sender.send(result);
        }
    }

    /// Handle an incoming notification.
    async fn handle_notification(state: &StdioSharedState, request: &JsonRpcRequest) {
        let handler = state.notification_handler.read().await;
        if let Some(handler) = handler.as_ref() {
            let params = request.params.as_ref().unwrap_or(&Value::Null);
            handler(&request.method, params);
        }
    }

    /// Handle an incoming request.
    async fn handle_request(state: &StdioSharedState, request: &JsonRpcRequest) {
        let id = match &request.id {
            Some(id) => id.clone(),
            None => return,
        };

        let handler = state.request_handler.read().await;
        let params = request.params.as_ref().unwrap_or(&Value::Null);

        let response = if let Some(handler) = handler.as_ref() {
            // Call the async handler and await result
            match handler(&request.method, params).await {
                Ok(result) => JsonRpcResponse::success(id.clone(), result),
                Err(error) => JsonRpcResponse::error(id.clone(), error),
            }
        } else {
            JsonRpcResponse::error(
                id.clone(),
                JsonRpcError::new(
                    JsonRpcError::METHOD_NOT_FOUND,
                    format!("Method not found: {}", request.method),
                ),
            )
        };

        // Send response
        if let Ok(response_json) = serde_json::to_string(&response) {
            let mut writer = state.writer.lock().await;
            let _ = writer.write_message(&response_json).await;
        }
    }
}

// =============================================================================
// TCP JSON-RPC Client (split read/write paths)
// =============================================================================

/// Shared state for the TCP JSON-RPC client.
struct TcpSharedState {
    writer: Mutex<MessageWriter<OwnedWriteHalf>>,
    running: AtomicBool,
    pending_requests: RwLock<HashMap<i64, PendingRequest>>,
    notification_handler: RwLock<Option<NotificationHandler>>,
    request_handler: RwLock<Option<RequestHandler>>,
}

/// JSON-RPC client for TCP transports with separate read/write paths.
pub struct TcpJsonRpcClient {
    state: Arc<TcpSharedState>,
    reader: Mutex<Option<MessageReader<OwnedReadHalf>>>,
    next_id: AtomicI64,
    shutdown_tx: Mutex<Option<mpsc::Sender<()>>>,
}

impl TcpJsonRpcClient {
    /// Connect to a TCP JSON-RPC server.
    pub async fn connect(addr: impl AsRef<str>) -> Result<Self> {
        let stream = TcpStream::connect(addr.as_ref())
            .await
            .map_err(CopilotError::Transport)?;
        Ok(Self::new(stream))
    }

    /// Create a new TCP JSON-RPC client from a connected socket.
    pub fn new(stream: TcpStream) -> Self {
        let (reader, writer) = stream.into_split();
        Self {
            state: Arc::new(TcpSharedState {
                writer: Mutex::new(MessageWriter::new(writer)),
                running: AtomicBool::new(false),
                pending_requests: RwLock::new(HashMap::new()),
                notification_handler: RwLock::new(None),
                request_handler: RwLock::new(None),
            }),
            reader: Mutex::new(Some(MessageReader::new(reader))),
            next_id: AtomicI64::new(1),
            shutdown_tx: Mutex::new(None),
        }
    }

    /// Start the background read loop.
    pub async fn start(&self) -> Result<()> {
        let reader = self.reader.lock().await.take().ok_or_else(|| {
            CopilotError::InvalidConfig("Reader already taken or client already started".into())
        })?;
        self.start_with_reader(reader).await
    }

    async fn start_with_reader(&self, mut reader: MessageReader<OwnedReadHalf>) -> Result<()> {
        if self.state.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown_tx.lock().await = Some(shutdown_tx);

        let state = Arc::clone(&self.state);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        break;
                    }
                    result = reader.read_message() => {
                        match result {
                            Ok(message_str) => {
                                if let Ok(message) = serde_json::from_str::<Value>(&message_str) {
                                    Self::dispatch_message(&state, message).await;
                                }
                            }
                            Err(CopilotError::ConnectionClosed) => {
                                state.running.store(false, Ordering::SeqCst);
                                let mut pending = state.pending_requests.write().await;
                                for (_, req) in pending.drain() {
                                    let _ = req.sender.send(Err(JsonRpcError::new(
                                        -32801,
                                        "Connection closed",
                                    )));
                                }
                                break;
                            }
                            Err(_) => {
                                if !state.running.load(Ordering::SeqCst) {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop the client.
    pub async fn stop(&self) {
        self.state.running.store(false, Ordering::SeqCst);

        if let Some(tx) = self.shutdown_tx.lock().await.take() {
            let _ = tx.send(()).await;
        }

        let mut pending = self.state.pending_requests.write().await;
        for (_, req) in pending.drain() {
            let _ = req
                .sender
                .send(Err(JsonRpcError::new(-32801, "Connection closed")));
        }
    }

    /// Check if client is running.
    pub fn is_running(&self) -> bool {
        self.state.running.load(Ordering::SeqCst)
    }

    /// Set handler for incoming notifications.
    pub async fn set_notification_handler<F>(&self, handler: F)
    where
        F: Fn(&str, &Value) + Send + Sync + 'static,
    {
        *self.state.notification_handler.write().await = Some(Arc::new(handler));
    }

    /// Set handler for incoming requests.
    pub async fn set_request_handler<F>(&self, handler: F)
    where
        F: Fn(&str, &Value) -> RequestHandlerFuture + Send + Sync + 'static,
    {
        *self.state.request_handler.write().await = Some(Arc::new(handler));
    }

    /// Send a request and await response.
    pub async fn invoke(&self, method: &str, params: Option<Value>) -> Result<Value> {
        self.invoke_with_timeout(method, params, Duration::from_secs(30))
            .await
    }

    /// Send a request with custom timeout.
    pub async fn invoke_with_timeout(
        &self,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.state.pending_requests.write().await;
            pending.insert(id, PendingRequest { sender: tx });
        }

        let request = JsonRpcRequest::new(method, params, Some(JsonRpcId::Num(id)));
        let request_json = serde_json::to_string(&request)?;

        if let Err(e) = self.send_raw(&request_json).await {
            self.state.pending_requests.write().await.remove(&id);
            return Err(e);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(Ok(result))) => Ok(result),
            Ok(Ok(Err(rpc_error))) => Err(CopilotError::JsonRpc {
                code: rpc_error.code,
                message: rpc_error.message,
                data: rpc_error.data,
            }),
            Ok(Err(_)) => {
                self.state.pending_requests.write().await.remove(&id);
                Err(CopilotError::ConnectionClosed)
            }
            Err(_) => {
                self.state.pending_requests.write().await.remove(&id);
                Err(CopilotError::Timeout(timeout))
            }
        }
    }

    /// Send a notification (no response expected).
    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        let request = JsonRpcRequest::notification(method, params);
        let request_json = serde_json::to_string(&request)?;
        self.send_raw(&request_json).await
    }

    async fn send_raw(&self, message: &str) -> Result<()> {
        let mut writer = self.state.writer.lock().await;
        writer.write_message(message).await
    }

    async fn dispatch_message(state: &TcpSharedState, message: Value) {
        if message.get("id").is_some()
            && !message.get("id").map(|v| v.is_null()).unwrap_or(true)
            && (message.get("result").is_some() || message.get("error").is_some())
            && message.get("method").is_none()
        {
            Self::handle_response(state, message).await;
            return;
        }

        if message.get("method").is_some() {
            if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(message) {
                if request.is_notification() {
                    Self::handle_notification(state, &request).await;
                } else {
                    Self::handle_request(state, &request).await;
                }
            }
        }
    }

    async fn handle_response(state: &TcpSharedState, message: Value) {
        let response: JsonRpcResponse = match serde_json::from_value(message) {
            Ok(r) => r,
            Err(_) => return,
        };

        let id = match &response.id {
            Some(JsonRpcId::Num(n)) => *n,
            _ => return,
        };

        let pending_req = {
            let mut pending = state.pending_requests.write().await;
            pending.remove(&id)
        };

        if let Some(req) = pending_req {
            let result = if let Some(error) = response.error {
                Err(error)
            } else {
                Ok(response.result.unwrap_or(Value::Null))
            };
            let _ = req.sender.send(result);
        }
    }

    async fn handle_notification(state: &TcpSharedState, request: &JsonRpcRequest) {
        let handler = state.notification_handler.read().await;
        if let Some(handler) = handler.as_ref() {
            let params = request.params.as_ref().unwrap_or(&Value::Null);
            handler(&request.method, params);
        }
    }

    async fn handle_request(state: &TcpSharedState, request: &JsonRpcRequest) {
        let id = match &request.id {
            Some(id) => id.clone(),
            None => return,
        };

        let handler = state.request_handler.read().await;
        let params = request.params.as_ref().unwrap_or(&Value::Null);

        let response = if let Some(handler) = handler.as_ref() {
            match handler(&request.method, params).await {
                Ok(result) => JsonRpcResponse::success(id.clone(), result),
                Err(error) => JsonRpcResponse::error(id.clone(), error),
            }
        } else {
            JsonRpcResponse::error(
                id.clone(),
                JsonRpcError::new(
                    JsonRpcError::METHOD_NOT_FOUND,
                    format!("Method not found: {}", request.method),
                ),
            )
        };

        if let Ok(response_json) = serde_json::to_string(&response) {
            let mut writer = state.writer.lock().await;
            let _ = writer.write_message(&response_json).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::MemoryTransport;
    use serde_json::json;

    #[test]
    fn test_json_rpc_request_serialization() {
        let request = JsonRpcRequest::new(
            "test_method",
            Some(json!({"key": "value"})),
            Some(JsonRpcId::Num(1)),
        );

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "test_method");
        assert_eq!(json["params"]["key"], "value");
        assert_eq!(json["id"], 1);
    }

    #[test]
    fn test_json_rpc_notification_serialization() {
        let request = JsonRpcRequest::notification("notify_method", Some(json!([1, 2, 3])));

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "notify_method");
        assert!(json.get("id").is_none());
    }

    #[test]
    fn test_json_rpc_response_success() {
        let response = JsonRpcResponse::success(JsonRpcId::Num(1), json!({"result": "ok"}));

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["result"]["result"], "ok");
        assert!(json.get("error").is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let response = JsonRpcResponse::error(
            JsonRpcId::Num(1),
            JsonRpcError::new(-32600, "Invalid Request"),
        );

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["error"]["code"], -32600);
        assert_eq!(json["error"]["message"], "Invalid Request");
    }

    #[test]
    fn test_json_rpc_id_from_i64() {
        let id: JsonRpcId = 42i64.into();
        assert_eq!(id, JsonRpcId::Num(42));
    }

    #[test]
    fn test_json_rpc_id_from_string() {
        let id: JsonRpcId = "test-id".into();
        assert_eq!(id, JsonRpcId::Str("test-id".to_string()));
    }

    #[test]
    fn test_json_rpc_error_constants() {
        assert_eq!(JsonRpcError::PARSE_ERROR, -32700);
        assert_eq!(JsonRpcError::INVALID_REQUEST, -32600);
        assert_eq!(JsonRpcError::METHOD_NOT_FOUND, -32601);
        assert_eq!(JsonRpcError::INVALID_PARAMS, -32602);
        assert_eq!(JsonRpcError::INTERNAL_ERROR, -32603);
    }

    #[test]
    fn test_request_is_notification() {
        let request = JsonRpcRequest::notification("method", None);
        assert!(request.is_notification());

        let request = JsonRpcRequest::new("method", None, Some(JsonRpcId::Num(1)));
        assert!(!request.is_notification());
    }

    #[tokio::test]
    async fn test_large_payload_64kb_boundary() {
        // Create a payload near 64KB (65536 bytes)
        let large_data = "x".repeat(65536 - 50); // account for JSON wrapper
        let msg =
            serde_json::json!({"jsonrpc": "2.0", "method": "test", "params": {"data": large_data}});
        let msg_str = serde_json::to_string(&msg).unwrap();

        // Write with framer
        let transport = MemoryTransport::new(Vec::new());
        let mut framer = MessageFramer::new(transport);
        framer.write_message(&msg_str).await.unwrap();

        // Read back from written data
        let written = framer.transport().written_data().to_vec();
        let transport2 = MemoryTransport::new(written);
        let mut framer2 = MessageFramer::new(transport2);
        let read_back = framer2.read_message().await.unwrap();
        assert_eq!(msg_str, read_back);
    }

    #[tokio::test]
    async fn test_large_payload_100kb() {
        let large_data = "y".repeat(100_000);
        let msg =
            serde_json::json!({"jsonrpc": "2.0", "method": "test", "params": {"data": large_data}});
        let msg_str = serde_json::to_string(&msg).unwrap();

        let transport = MemoryTransport::new(Vec::new());
        let mut framer = MessageFramer::new(transport);
        framer.write_message(&msg_str).await.unwrap();

        let written = framer.transport().written_data().to_vec();
        let transport2 = MemoryTransport::new(written);
        let mut framer2 = MessageFramer::new(transport2);
        let read_back = framer2.read_message().await.unwrap();
        assert_eq!(msg_str, read_back);
    }

    #[tokio::test]
    async fn test_multiple_large_messages_sequential() {
        let msg1_data = "a".repeat(50_000);
        let msg2_data = "b".repeat(80_000);
        let msg1 = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "test1", "params": {"data": msg1_data}});
        let msg2 = serde_json::json!({"jsonrpc": "2.0", "id": 2, "method": "test2", "params": {"data": msg2_data}});
        let msg1_str = serde_json::to_string(&msg1).unwrap();
        let msg2_str = serde_json::to_string(&msg2).unwrap();

        // Write both messages
        let transport = MemoryTransport::new(Vec::new());
        let mut framer = MessageFramer::new(transport);
        framer.write_message(&msg1_str).await.unwrap();
        framer.write_message(&msg2_str).await.unwrap();

        // Read both back
        let written = framer.transport().written_data().to_vec();
        let transport2 = MemoryTransport::new(written);
        let mut framer2 = MessageFramer::new(transport2);
        let read1 = framer2.read_message().await.unwrap();
        let read2 = framer2.read_message().await.unwrap();
        assert_eq!(msg1_str, read1);
        assert_eq!(msg2_str, read2);
    }
}
