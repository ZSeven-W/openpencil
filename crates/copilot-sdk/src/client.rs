// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! Copilot client for managing connections and sessions.
//!
//! The `Client` is the main entry point for the SDK.

use crate::error::{CopilotError, Result};
use crate::events::SessionEvent;
use crate::jsonrpc::{StdioJsonRpcClient, TcpJsonRpcClient};
use crate::process::{CopilotProcess, ProcessOptions};
use crate::session::Session;
use crate::types::{
    ClientOptions, ConnectionState, GetAuthStatusResponse, GetForegroundSessionResponse,
    GetStatusResponse, LogLevel, ModelInfo, PingResponse, ProviderConfig, QuotaResult,
    ResumeSessionConfig, SessionConfig, SessionLifecycleEvent, SessionMetadata,
    SetForegroundSessionResponse, StopError, TelemetryConfig, ToolsListResult,
    MIN_PROTOCOL_VERSION, SDK_PROTOCOL_VERSION,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{Mutex, RwLock};

// =============================================================================
// Helper Functions
// =============================================================================

/// Resolve CLI command for the current platform.
///
/// On Windows, .cmd/.bat files are npm wrappers that need special handling.
/// We resolve them to their underlying node.js scripts for proper pipe handling.
fn resolve_cli_command(cli_path: &Path, args: &[String]) -> (PathBuf, Vec<String>) {
    let path = cli_path.to_path_buf();
    let args_owned = args.to_vec();

    // Check if it's a Node.js script - run directly via node
    if crate::process::is_node_script(&path) {
        if let Some(node_path) = crate::process::find_node() {
            let mut full_args = vec![path.to_string_lossy().to_string()];
            full_args.extend(args_owned);
            return (node_path, full_args);
        }
    }

    #[cfg(windows)]
    {
        // On Windows, .cmd files are npm wrapper scripts that launch node.
        // Running them through cmd.exe causes pipe inheritance issues.
        // Instead, we find the underlying node.js script and run it directly.
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if ext_lower == "cmd" {
                // npm .cmd files have a corresponding node_modules structure
                // e.g., C:\Users\...\npm\copilot.cmd -> C:\Users\...\npm\node_modules\@github\copilot\npm-loader.js
                if let Some(parent) = path.parent() {
                    // Extract the package name from the .cmd filename
                    if let Some(stem) = path.file_stem() {
                        let stem_str = stem.to_string_lossy();

                        // Try to find the npm-loader.js in node_modules
                        // Common patterns: copilot -> @github/copilot, or package-name -> package-name
                        let possible_paths = vec![
                            parent
                                .join("node_modules/@github")
                                .join(&*stem_str)
                                .join("npm-loader.js"),
                            parent
                                .join("node_modules")
                                .join(&*stem_str)
                                .join("npm-loader.js"),
                            parent
                                .join("node_modules/@github")
                                .join(&*stem_str)
                                .join("index.js"),
                            parent
                                .join("node_modules")
                                .join(&*stem_str)
                                .join("index.js"),
                        ];

                        for loader_path in possible_paths {
                            if loader_path.exists() {
                                if let Some(node_path) = crate::process::find_node() {
                                    let mut full_args =
                                        vec![loader_path.to_string_lossy().to_string()];
                                    full_args.extend(args_owned);
                                    return (node_path, full_args);
                                }
                            }
                        }
                    }
                }

                // Fallback: use cmd /c if we can't find the loader
                let mut full_args = vec!["/c".to_string(), path.to_string_lossy().to_string()];
                full_args.extend(args_owned);
                return (PathBuf::from("cmd"), full_args);
            }

            // For .bat files, use cmd /c
            if ext_lower == "bat" {
                let mut full_args = vec!["/c".to_string(), path.to_string_lossy().to_string()];
                full_args.extend(args_owned);
                return (PathBuf::from("cmd"), full_args);
            }
        }

        // For non-absolute paths without extension, also use cmd /c for PATH resolution
        if !path.is_absolute() {
            let mut full_args = vec!["/c".to_string(), path.to_string_lossy().to_string()];
            full_args.extend(args_owned);
            return (PathBuf::from("cmd"), full_args);
        }
    }

    (path, args_owned)
}

fn spawn_cli_stderr_logger(stderr: tokio::process::ChildStderr) {
    tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!(target: "copilot_sdk::cli_stderr", "{line}");
        }
    });
}

/// Handler for client-level lifecycle events (session created, deleted, etc.).
pub type LifecycleHandler = Arc<dyn Fn(&SessionLifecycleEvent) + Send + Sync>;

/// Handle a tool.call request from the server.
async fn handle_tool_call(
    sessions: &RwLock<HashMap<String, Arc<Session>>>,
    params: &Value,
) -> Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidConfig("Missing sessionId".into()))?;

    let tool_name = params
        .get("toolName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidConfig("Missing toolName".into()))?;

    let arguments = normalize_tool_arguments(params);

    let session = sessions.read().await.get(session_id).cloned();

    let session = match session {
        Some(s) => s,
        None => {
            return Ok(json!({
                "result": {
                    "textResultForLlm": "Session not found",
                    "resultType": "failure",
                    "error": format!("Unknown session {}", session_id)
                }
            }));
        }
    };

    // Check if tool is registered
    if session.get_tool(tool_name).await.is_none() {
        return Ok(json!({
            "result": {
                "textResultForLlm": format!("Tool '{}' is not supported.", tool_name),
                "resultType": "failure",
                "error": format!("tool '{}' not supported", tool_name)
            }
        }));
    }

    // Invoke the tool handler
    match session.invoke_tool(tool_name, &arguments).await {
        Ok(result) => Ok(json!({ "result": result })),
        Err(e) => Ok(json!({
            "result": {
                "textResultForLlm": "Tool execution failed",
                "resultType": "failure",
                "error": e.to_string()
            }
        })),
    }
}

fn normalize_tool_arguments(params: &Value) -> Value {
    let raw = params
        .get("arguments")
        .or_else(|| params.get("argumentsJson"))
        .cloned()
        .unwrap_or(json!({}));

    match raw {
        Value::String(s) => serde_json::from_str(&s).unwrap_or(json!({})),
        Value::Null => json!({}),
        other => other,
    }
}

/// Handle a permission.request from the server.
async fn handle_permission_request(
    sessions: &RwLock<HashMap<String, Arc<Session>>>,
    params: &Value,
) -> Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidConfig("Missing sessionId".into()))?;

    // Permission request data may be nested in "permissionRequest" field
    let perm_data = params.get("permissionRequest").unwrap_or(params);

    let session = sessions.read().await.get(session_id).cloned();

    let session = match session {
        Some(s) => s,
        None => {
            // Default deny on unknown session
            return Ok(json!({
                "result": {
                    "kind": "denied-no-approval-rule-and-could-not-request-from-user"
                }
            }));
        }
    };

    // Build permission request
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

    // Collect extension data
    let mut extension_data = HashMap::new();
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

    let result = session.handle_permission_request(&request).await;

    // Build response
    let mut response = json!({
        "result": {
            "kind": result.kind
        }
    });

    if let Some(rules) = result.rules {
        response["result"]["rules"] = Value::Array(rules);
    }

    Ok(response)
}

/// Handle a userInput.request from the server.
async fn handle_user_input_request(
    sessions: &RwLock<HashMap<String, Arc<Session>>>,
    params: &Value,
) -> Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidConfig("Missing sessionId".into()))?;

    let session = sessions.read().await.get(session_id).cloned();

    let session = match session {
        Some(s) => s,
        None => {
            return Err(CopilotError::Protocol(format!(
                "Session not found for user input request: {session_id}"
            )));
        }
    };

    use crate::types::UserInputRequest;
    let request = UserInputRequest {
        question: params
            .get("question")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        choices: params.get("choices").and_then(|v| {
            v.as_array().map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
        }),
        allow_freeform: params.get("allowFreeform").and_then(|v| v.as_bool()),
    };

    let response = session.handle_user_input_request(&request).await?;
    Ok(serde_json::to_value(response).unwrap_or(json!({})))
}

async fn handle_hooks_invoke(
    sessions: &RwLock<HashMap<String, Arc<Session>>>,
    params: &Value,
) -> Result<Value> {
    let session_id = params
        .get("sessionId")
        .and_then(|v| v.as_str())
        .ok_or_else(|| CopilotError::InvalidConfig("Missing sessionId".into()))?;

    let session = sessions.read().await.get(session_id).cloned();

    let session = match session {
        Some(s) => s,
        None => {
            return Err(CopilotError::Protocol(format!(
                "Session not found for hooks invoke: {session_id}"
            )));
        }
    };

    let hook_type = params
        .get("hookType")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let input = params.get("input").cloned().unwrap_or(Value::Null);

    session.handle_hooks_invoke(hook_type, &input).await
}

fn parse_cli_url(url: &str) -> Result<(String, u16)> {
    let mut s = url.trim();
    if let Some((_, rest)) = s.split_once("://") {
        s = rest;
    }
    if let Some((host_port, _)) = s.split_once('/') {
        s = host_port;
    }

    if s.chars().all(|c| c.is_ascii_digit()) {
        let port: u16 = s.parse().map_err(|_| {
            CopilotError::InvalidConfig(format!("Invalid port in cli_url: {}", url))
        })?;
        return Ok(("localhost".to_string(), port));
    }

    if let Some((host, port_str)) = s.rsplit_once(':') {
        let host = host.trim();
        let port: u16 = port_str.trim().parse().map_err(|_| {
            CopilotError::InvalidConfig(format!("Invalid port in cli_url: {}", url))
        })?;
        if host.is_empty() {
            return Ok(("localhost".to_string(), port));
        }
        return Ok((host.to_string(), port));
    }

    Err(CopilotError::InvalidConfig(format!(
        "Invalid cli_url format (expected host:port or port): {}",
        url
    )))
}

fn parse_listening_port(line: &str) -> Option<u16> {
    let lower = line.to_lowercase();
    let idx = lower.find("listening on port")?;
    let after = &line[idx..];

    let mut digits = String::new();
    let mut in_digits = false;
    for ch in after.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            in_digits = true;
        } else if in_digits {
            break;
        }
    }
    digits.parse::<u16>().ok()
}

async fn detect_tcp_port_from_stdout(stdout: tokio::process::ChildStdout) -> Result<u16> {
    let mut lines = BufReader::new(stdout).lines();
    let port = tokio::time::timeout(Duration::from_secs(15), async {
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(port) = parse_listening_port(&line) {
                return Ok(port);
            }
        }
        Err(CopilotError::PortDetectionFailed)
    })
    .await
    .map_err(|_| CopilotError::Timeout(Duration::from_secs(15)))??;

    Ok(port)
}

enum RpcClient {
    Stdio(StdioJsonRpcClient),
    Tcp(TcpJsonRpcClient),
}

impl RpcClient {
    async fn stop(&self) {
        match self {
            RpcClient::Stdio(rpc) => rpc.stop().await,
            RpcClient::Tcp(rpc) => rpc.stop().await,
        }
    }

    async fn set_notification_handler<F>(&self, handler: F)
    where
        F: Fn(&str, &Value) + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        match self {
            RpcClient::Stdio(rpc) => {
                let handler = Arc::clone(&handler);
                rpc.set_notification_handler(move |method, params| {
                    (handler)(method, params);
                })
                .await;
            }
            RpcClient::Tcp(rpc) => {
                let handler = Arc::clone(&handler);
                rpc.set_notification_handler(move |method, params| {
                    (handler)(method, params);
                })
                .await;
            }
        }
    }

    async fn set_request_handler<F>(&self, handler: F)
    where
        F: Fn(&str, &Value) -> crate::jsonrpc::RequestHandlerFuture + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        match self {
            RpcClient::Stdio(rpc) => {
                let handler = Arc::clone(&handler);
                rpc.set_request_handler(move |method, params| (handler)(method, params))
                    .await;
            }
            RpcClient::Tcp(rpc) => {
                let handler = Arc::clone(&handler);
                rpc.set_request_handler(move |method, params| (handler)(method, params))
                    .await;
            }
        }
    }

    async fn invoke(&self, method: &str, params: Option<Value>) -> Result<Value> {
        match self {
            RpcClient::Stdio(rpc) => rpc.invoke(method, params).await,
            RpcClient::Tcp(rpc) => rpc.invoke(method, params).await,
        }
    }
}

// =============================================================================
// Client
// =============================================================================

/// Copilot client for managing connections and sessions.
///
/// The client manages the connection to the Copilot CLI server and provides
/// methods to create and manage conversation sessions.
///
/// # Example
///
/// ```no_run
/// use copilot_sdk::{Client, ClientOptions, SessionConfig};
///
/// #[tokio::main]
/// async fn main() -> copilot_sdk::Result<()> {
///     // Create client with options
///     let client = Client::new(ClientOptions::default())?;
///
///     // Start the client
///     client.start().await?;
///
///     // Create a session
///     let session = client.create_session(SessionConfig::default()).await?;
///
///     // Send a message and collect response
///     let response = session.send_and_collect("Hello!", None).await?;
///     println!("{}", response);
///
///     // Stop the client
///     client.stop().await;
///     Ok(())
/// }
/// ```
pub struct Client {
    options: ClientOptions,
    state: Arc<RwLock<ConnectionState>>,
    lifecycle: Mutex<()>,
    process: Mutex<Option<CopilotProcess>>,
    rpc: Arc<Mutex<Option<RpcClient>>>,
    sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
    lifecycle_handlers: Arc<RwLock<HashMap<u64, LifecycleHandler>>>,
    next_lifecycle_handler_id: AtomicU64,
    models_cache: Arc<Mutex<Option<Vec<ModelInfo>>>>,
    negotiated_protocol_version: Arc<Mutex<Option<u32>>>,
}

impl Client {
    /// Create a new Copilot client with the given options.
    pub fn new(options: ClientOptions) -> Result<Self> {
        let mut options = options;

        if options.cli_url.is_some() {
            options.use_stdio = false;
        }

        // Validate mutually exclusive options
        if options.cli_url.is_some() {
            if options.cli_path.is_some() {
                return Err(CopilotError::InvalidConfig(
                    "cli_url is mutually exclusive with cli_path".into(),
                ));
            }
            if options.port != 0 {
                return Err(CopilotError::InvalidConfig(
                    "cli_url is mutually exclusive with port".into(),
                ));
            }
        }
        if options.use_stdio && options.port != 0 {
            return Err(CopilotError::InvalidConfig(
                "port is only valid when use_stdio=false".into(),
            ));
        }
        if options.cli_url.is_some() && options.github_token.is_some() {
            return Err(CopilotError::InvalidConfig(
                "github_token cannot be used with cli_url (external server doesn't accept token)"
                    .into(),
            ));
        }
        if options.cli_url.is_some() && options.use_logged_in_user.is_some() {
            return Err(CopilotError::InvalidConfig(
                "use_logged_in_user cannot be used with cli_url (external server doesn't accept this option)".into(),
            ));
        }

        Ok(Self {
            options,
            state: Arc::new(RwLock::new(ConnectionState::Disconnected)),
            lifecycle: Mutex::new(()),
            process: Mutex::new(None),
            rpc: Arc::new(Mutex::new(None)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            lifecycle_handlers: Arc::new(RwLock::new(HashMap::new())),
            next_lifecycle_handler_id: AtomicU64::new(1),
            models_cache: Arc::new(Mutex::new(None)),
            negotiated_protocol_version: Arc::new(Mutex::new(None)),
        })
    }

    /// Create a client builder for fluent configuration.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    // =========================================================================
    // Connection Management
    // =========================================================================

    /// Start the client and connect to the server.
    pub async fn start(&self) -> Result<()> {
        let _guard = self.lifecycle.lock().await;

        let mut state = self.state.write().await;
        if *state == ConnectionState::Connected {
            return Ok(());
        }
        if *state != ConnectionState::Disconnected {
            return Err(CopilotError::InvalidConfig(
                "Client is already started".into(),
            ));
        }
        *state = ConnectionState::Connecting;
        drop(state);

        // Start CLI server process
        let result = self.start_cli_server().await;
        if let Err(e) = result {
            *self.state.write().await = ConnectionState::Error;
            return Err(e);
        }

        // Verify protocol version
        if let Err(e) = self.verify_protocol_version().await {
            *self.state.write().await = ConnectionState::Error;
            return Err(e);
        }

        // Set up event handlers
        self.setup_handlers().await?;

        *self.state.write().await = ConnectionState::Connected;
        Ok(())
    }

    /// Stop the client gracefully.
    pub async fn stop(&self) -> Vec<StopError> {
        let _guard = self.lifecycle.lock().await;
        let mut errors = Vec::new();

        let state = *self.state.read().await;
        if state == ConnectionState::Disconnected {
            self.sessions.write().await.clear();
            *self.rpc.lock().await = None;
            *self.process.lock().await = None;
            return errors;
        }

        // Best-effort destroy of all active sessions while still connected.
        let sessions: Vec<Arc<Session>> = self.sessions.read().await.values().cloned().collect();
        for session in sessions {
            if let Err(e) = session.destroy().await {
                errors.push(StopError {
                    message: format!("Failed to destroy session {}: {}", session.session_id(), e),
                    source: Some("session.destroy".into()),
                });
            }
        }
        self.sessions.write().await.clear();

        // Stop the RPC client
        if let Some(rpc) = self.rpc.lock().await.take() {
            rpc.stop().await;
        }

        // Stop the process
        if let Some(mut process) = self.process.lock().await.take() {
            let _ = process.terminate();
            let _ = process.wait().await;
        }

        *self.state.write().await = ConnectionState::Disconnected;
        *self.models_cache.lock().await = None;
        errors
    }

    /// Force stop the client immediately.
    pub async fn force_stop(&self) {
        let _guard = self.lifecycle.lock().await;

        self.sessions.write().await.clear();

        // Kill the process
        if let Some(mut process) = self.process.lock().await.take() {
            let _ = process.kill();
        }

        // Stop the RPC client
        if let Some(rpc) = self.rpc.lock().await.take() {
            rpc.stop().await;
        }

        *self.state.write().await = ConnectionState::Disconnected;
        *self.models_cache.lock().await = None;
    }

    /// Get the current connection state.
    pub async fn state(&self) -> ConnectionState {
        *self.state.read().await
    }

    // =========================================================================
    // Session Management
    // =========================================================================

    /// Create a new Copilot session.
    pub async fn create_session(&self, mut config: SessionConfig) -> Result<Arc<Session>> {
        self.ensure_connected().await?;

        // Apply BYOK from environment if enabled and not explicitly set
        if config.auto_byok_from_env && config.model.is_none() {
            config.model = ProviderConfig::model_from_env();
        }
        if config.auto_byok_from_env && config.provider.is_none() {
            config.provider = ProviderConfig::from_env();
        }

        // Build the request
        let params = serde_json::to_value(&config)?;

        // Send the request
        let result = self.invoke("session.create", Some(params)).await?;

        // Extract session ID
        let session_id = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CopilotError::Protocol("Missing sessionId in response".into()))?
            .to_string();

        // Extract workspace_path (for infinite sessions)
        let workspace_path = result
            .get("workspacePath")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Create session object
        let session = self
            .create_session_object(session_id.clone(), workspace_path)
            .await;

        // Register hooks from config if provided
        if let Some(hooks) = config.hooks.take() {
            if hooks.has_any() {
                session.register_hooks(hooks).await;
            }
        }

        // Store session
        self.sessions
            .write()
            .await
            .insert(session_id, Arc::clone(&session));

        Ok(session)
    }

    /// Resume an existing session.
    pub async fn resume_session(
        &self,
        session_id: &str,
        mut config: ResumeSessionConfig,
    ) -> Result<Arc<Session>> {
        self.ensure_connected().await?;

        // Apply BYOK from environment if enabled and not explicitly set
        if config.auto_byok_from_env && config.provider.is_none() {
            config.provider = ProviderConfig::from_env();
        }

        // Build the request
        let mut params = serde_json::to_value(&config)?;
        params["sessionId"] = json!(session_id);

        // Send the request
        let result = self.invoke("session.resume", Some(params)).await?;

        // Extract session ID from response
        let resumed_id = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .unwrap_or(session_id)
            .to_string();

        // Extract workspace_path (for infinite sessions)
        let workspace_path = result
            .get("workspacePath")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Create session object
        let session = self
            .create_session_object(resumed_id.clone(), workspace_path)
            .await;

        // Register hooks from config if provided
        if let Some(hooks) = config.hooks.take() {
            if hooks.has_any() {
                session.register_hooks(hooks).await;
            }
        }

        // Store session
        self.sessions
            .write()
            .await
            .insert(resumed_id, Arc::clone(&session));

        Ok(session)
    }

    /// List all available sessions.
    pub async fn list_sessions(&self) -> Result<Vec<SessionMetadata>> {
        self.ensure_connected().await?;

        let result = self.invoke("session.list", None).await?;

        let sessions: Vec<SessionMetadata> = result
            .get("sessions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Ok(sessions)
    }

    /// Delete a session.
    pub async fn delete_session(&self, session_id: &str) -> Result<()> {
        self.ensure_connected().await?;

        let params = json!({ "sessionId": session_id });
        let result = self.invoke("session.delete", Some(params)).await?;

        if let Some(success) = result.get("success").and_then(|v| v.as_bool()) {
            if !success {
                let msg = result
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();
                return Err(CopilotError::Protocol(format!(
                    "Failed to delete session: {}",
                    msg
                )));
            }
        }

        // Remove from local cache
        self.sessions.write().await.remove(session_id);

        Ok(())
    }

    /// Get the ID of the most recently used session.
    pub async fn get_last_session_id(&self) -> Result<Option<String>> {
        self.ensure_connected().await?;

        let result = self.invoke("session.getLastId", None).await?;

        Ok(result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    // =========================================================================
    // Server Communication
    // =========================================================================

    /// Send a ping to verify connection health.
    pub async fn ping(&self, message: Option<String>) -> Result<PingResponse> {
        self.ensure_connected().await?;

        let params = message.map(|m| json!({ "message": m }));
        let result = self.invoke("ping", params).await?;

        Ok(PingResponse {
            message: result
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            timestamp: result
                .get("timestamp")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            protocol_version: result
                .get("protocolVersion")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
        })
    }

    /// Get CLI status including version and protocol information.
    pub async fn get_status(&self) -> Result<GetStatusResponse> {
        self.ensure_connected().await?;

        let result = self.invoke("status.get", None).await?;
        serde_json::from_value(result)
            .map_err(|e| CopilotError::Protocol(format!("Failed to parse status response: {}", e)))
    }

    /// Get current authentication status.
    pub async fn get_auth_status(&self) -> Result<GetAuthStatusResponse> {
        self.ensure_connected().await?;

        let result = self.invoke("auth.getStatus", None).await?;
        serde_json::from_value(result).map_err(|e| {
            CopilotError::Protocol(format!("Failed to parse auth status response: {}", e))
        })
    }

    /// List available models with their metadata.
    ///
    /// Results are cached after the first call. Use [`clear_models_cache`] to force a refresh.
    ///
    /// If `on_list_models` was set via the builder, that callback is used instead of
    /// querying the CLI (useful for BYOK scenarios).
    ///
    /// # Errors
    /// Returns an error if not authenticated or if the request fails.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>> {
        // Check cache first
        {
            let cache = self.models_cache.lock().await;
            if let Some(cached) = &*cache {
                return Ok(cached.clone());
            }
        }

        // Check for custom model list provider
        if let Some(ref handler) = self.options.on_list_models {
            let models = handler().await?;
            *self.models_cache.lock().await = Some(models.clone());
            return Ok(models);
        }

        self.ensure_connected().await?;

        let result = self.invoke("models.list", None).await?;
        let models = result
            .get("models")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let models: Vec<ModelInfo> = serde_json::from_value(models).map_err(|e| {
            CopilotError::Protocol(format!("Failed to parse models response: {}", e))
        })?;

        // Store in cache
        *self.models_cache.lock().await = Some(models.clone());

        Ok(models)
    }

    /// List available tools with optional model-specific overrides.
    pub async fn tools_list(&self, model_id: Option<&str>) -> Result<ToolsListResult> {
        self.ensure_connected().await?;

        let params = model_id.map(|id| json!({ "modelId": id }));
        let result = self.invoke("tools.list", params).await?;
        serde_json::from_value(result)
            .map_err(|e| CopilotError::Protocol(format!("Failed to parse tools list: {}", e)))
    }

    /// Get account quota information.
    pub async fn get_quota(&self) -> Result<QuotaResult> {
        self.ensure_connected().await?;

        let result = self.invoke("account.get_quota", None).await?;
        serde_json::from_value(result)
            .map_err(|e| CopilotError::Protocol(format!("Failed to parse quota result: {}", e)))
    }

    /// Clear the cached models list, forcing a fresh fetch on next `list_models()` call.
    pub async fn clear_models_cache(&self) {
        *self.models_cache.lock().await = None;
    }

    /// Get the foreground session ID and workspace path.
    pub async fn get_foreground_session_id(&self) -> Result<GetForegroundSessionResponse> {
        self.ensure_connected().await?;

        let result = self.invoke("session.getForeground", None).await?;
        serde_json::from_value(result).map_err(|e| {
            CopilotError::Protocol(format!("Failed to parse foreground response: {}", e))
        })
    }

    /// Set the foreground session ID.
    pub async fn set_foreground_session_id(
        &self,
        session_id: &str,
    ) -> Result<SetForegroundSessionResponse> {
        self.ensure_connected().await?;

        let params = json!({ "sessionId": session_id });
        let result = self.invoke("session.setForeground", Some(params)).await?;
        serde_json::from_value(result).map_err(|e| {
            CopilotError::Protocol(format!("Failed to parse set foreground response: {}", e))
        })
    }

    // =========================================================================
    // Lifecycle Event Handling
    // =========================================================================

    /// Register a handler for client-level lifecycle events.
    ///
    /// Lifecycle events include session created, deleted, updated, foreground, and background.
    /// Returns an unsubscribe closure that removes the handler when called.
    pub async fn on<F>(&self, handler: F) -> impl FnOnce()
    where
        F: Fn(&SessionLifecycleEvent) + Send + Sync + 'static,
    {
        let id = self
            .next_lifecycle_handler_id
            .fetch_add(1, Ordering::SeqCst);
        self.lifecycle_handlers
            .write()
            .await
            .insert(id, Arc::new(handler));

        let handlers = Arc::clone(&self.lifecycle_handlers);
        move || {
            tokio::spawn(async move {
                handlers.write().await.remove(&id);
            });
        }
    }

    // =========================================================================
    // Internal Methods
    // =========================================================================

    /// Invoke a JSON-RPC method.
    pub(crate) async fn invoke(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let mut attempt = 0;

        loop {
            let result = {
                let rpc = self.rpc.lock().await;
                let rpc = rpc.as_ref().ok_or(CopilotError::NotConnected)?;
                rpc.invoke(method, params.clone()).await
            };

            match result {
                Ok(v) => return Ok(v),
                Err(e) => {
                    if attempt == 0
                        && *self.state.read().await == ConnectionState::Connected
                        && self.options.auto_restart
                        && self.should_restart_on_error(&e)
                    {
                        attempt += 1;
                        self.restart().await?;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Get a session by ID.
    pub async fn get_session(&self, session_id: &str) -> Option<Arc<Session>> {
        self.sessions.read().await.get(session_id).cloned()
    }

    /// Ensure client is connected.
    async fn ensure_connected(&self) -> Result<()> {
        match *self.state.read().await {
            ConnectionState::Connected => Ok(()),
            ConnectionState::Disconnected => {
                if self.options.auto_start {
                    self.start().await
                } else {
                    Err(CopilotError::NotConnected)
                }
            }
            ConnectionState::Error => {
                if self.options.auto_restart {
                    self.restart().await
                } else {
                    Err(CopilotError::NotConnected)
                }
            }
            ConnectionState::Connecting => Err(CopilotError::NotConnected),
        }
    }

    fn should_restart_on_error(&self, err: &CopilotError) -> bool {
        match err {
            CopilotError::ConnectionClosed | CopilotError::NotConnected => true,
            CopilotError::Transport(_) => true,
            CopilotError::ProcessExit(_) => true,
            CopilotError::JsonRpc { code, .. } => *code == -32801,
            _ => false,
        }
    }

    async fn restart(&self) -> Result<()> {
        self.force_stop().await;
        self.start().await
    }

    /// Start the CLI server process.
    async fn start_cli_server(&self) -> Result<()> {
        if let Some(cli_url) = &self.options.cli_url {
            let (host, port) = parse_cli_url(cli_url)?;
            let addr = format!("{}:{}", host, port);

            let rpc = TcpJsonRpcClient::connect(addr).await?;
            rpc.start().await?;

            *self.rpc.lock().await = Some(RpcClient::Tcp(rpc));
            return Ok(());
        }

        let cli_path = self
            .options
            .cli_path
            .clone()
            .or_else(crate::process::find_copilot_cli)
            .ok_or_else(|| {
                CopilotError::InvalidConfig("Could not find Copilot CLI executable".into())
            })?;

        let log_level = self.options.log_level.to_string();

        let mut args: Vec<String> = Vec::new();
        if let Some(extra_args) = &self.options.cli_args {
            args.extend(extra_args.iter().cloned());
        }

        // Add deny-tool arguments
        if let Some(deny_tools) = &self.options.deny_tools {
            for tool_spec in deny_tools {
                args.push("--deny-tool".to_string());
                args.push(tool_spec.clone());
            }
        }

        // Add allow-tool arguments
        if let Some(allow_tools) = &self.options.allow_tools {
            for tool_spec in allow_tools {
                args.push("--allow-tool".to_string());
                args.push(tool_spec.clone());
            }
        }

        // Add allow-all-tools flag
        if self.options.allow_all_tools {
            args.push("--allow-all-tools".to_string());
        }

        args.extend(["--server".to_string(), "--log-level".to_string(), log_level]);

        if self.options.use_stdio {
            args.push("--stdio".to_string());
        } else if self.options.port != 0 {
            args.extend(["--port".to_string(), self.options.port.to_string()]);
        }

        // Wire github_token auth: CLI flag for auth token env var
        if self.options.github_token.is_some() {
            args.push("--auth-token-env".to_string());
            args.push("COPILOT_SDK_AUTH_TOKEN".to_string());
        }

        // Wire use_logged_in_user: when false, pass --no-auto-login
        if let Some(false) = self.options.use_logged_in_user {
            args.push("--no-auto-login".to_string());
        }

        // Resolve command and arguments based on platform
        // On Windows, use cmd /c for PATH resolution if path is not absolute (for .cmd files)
        let (executable, full_args) = resolve_cli_command(&cli_path, &args);

        // Build process options
        let mut proc_options = ProcessOptions::new()
            .stdin(self.options.use_stdio)
            .stdout(true)
            .stderr(true);

        if let Some(ref dir) = self.options.cwd {
            proc_options = proc_options.working_dir(dir.clone());
        }

        // Add environment variables
        if let Some(ref env) = self.options.environment {
            for (key, value) in env {
                proc_options = proc_options.env(key, value);
            }
        }

        // Remove NODE_DEBUG to avoid debug output interfering with JSON-RPC
        proc_options = proc_options.env("NODE_DEBUG", "");

        // Wire github_token auth: pass via environment variable + CLI flag
        if let Some(ref token) = self.options.github_token {
            proc_options = proc_options.env("COPILOT_SDK_AUTH_TOKEN", token);
            args.push("--auth-token-env".to_string());
            args.push("COPILOT_SDK_AUTH_TOKEN".to_string());
        }

        // Wire use_logged_in_user: when false, pass --no-auto-login
        if let Some(false) = self.options.use_logged_in_user {
            args.push("--no-auto-login".to_string());
        }

        // Propagate telemetry configuration as environment variables
        if let Some(ref telemetry) = self.options.telemetry {
            if telemetry.otlp_endpoint.is_some() || telemetry.file_path.is_some() {
                proc_options = proc_options.env("COPILOT_OTEL_ENABLED", "true");
            }
            if let Some(ref endpoint) = telemetry.otlp_endpoint {
                proc_options = proc_options.env("OTEL_EXPORTER_OTLP_ENDPOINT", endpoint);
            }
            if let Some(ref path) = telemetry.file_path {
                proc_options = proc_options.env("COPILOT_OTEL_FILE_EXPORTER_PATH", path);
            }
            if let Some(ref exporter_type) = telemetry.exporter_type {
                proc_options = proc_options.env("COPILOT_OTEL_EXPORTER_TYPE", exporter_type);
            }
            if let Some(ref source_name) = telemetry.source_name {
                proc_options = proc_options.env("COPILOT_OTEL_SOURCE_NAME", source_name);
            }
            if let Some(capture) = telemetry.capture_content {
                if capture {
                    proc_options = proc_options
                        .env("OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT", "true");
                }
            }
        }

        let args_refs: Vec<&str> = full_args.iter().map(|s| s.as_str()).collect();
        let mut process = CopilotProcess::spawn(&executable, &args_refs, proc_options)?;

        if let Some(stderr) = process.take_stderr() {
            spawn_cli_stderr_logger(stderr);
        }

        let rpc = if self.options.use_stdio {
            let transport = process.take_transport().ok_or_else(|| {
                CopilotError::InvalidConfig("Failed to get transport from process".into())
            })?;
            let rpc = StdioJsonRpcClient::new(transport);
            rpc.start().await?;
            RpcClient::Stdio(rpc)
        } else {
            let stdout = process.take_stdout().ok_or_else(|| {
                CopilotError::InvalidConfig("Failed to capture stdout for port detection".into())
            })?;

            let detected_port = detect_tcp_port_from_stdout(stdout).await?;
            let addr = format!("127.0.0.1:{}", detected_port);
            let rpc = TcpJsonRpcClient::connect(addr).await?;
            rpc.start().await?;
            RpcClient::Tcp(rpc)
        };

        *self.process.lock().await = Some(process);
        *self.rpc.lock().await = Some(rpc);

        Ok(())
    }

    /// Verify that the server's protocol version is within the supported range
    /// and store the negotiated version.
    async fn verify_protocol_version(&self) -> Result<()> {
        // NOTE: We call the underlying RPC directly instead of ping() because ping() calls
        // ensure_connected(), but we haven't set state to Connected yet.
        let rpc = self.rpc.lock().await;
        let rpc = rpc.as_ref().ok_or(CopilotError::NotConnected)?;
        let result = rpc
            .invoke("ping", Some(serde_json::json!({ "message": null })))
            .await?;

        let server_version = result
            .get("protocolVersion")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        match server_version {
            None => {
                return Err(CopilotError::ProtocolMismatch {
                    min: MIN_PROTOCOL_VERSION,
                    max: SDK_PROTOCOL_VERSION,
                    actual: 0,
                });
            }
            Some(version) if !(MIN_PROTOCOL_VERSION..=SDK_PROTOCOL_VERSION).contains(&version) => {
                return Err(CopilotError::ProtocolMismatch {
                    min: MIN_PROTOCOL_VERSION,
                    max: SDK_PROTOCOL_VERSION,
                    actual: version,
                });
            }
            Some(version) => {
                *self.negotiated_protocol_version.lock().await = Some(version);
            }
        }

        Ok(())
    }

    /// Get the negotiated protocol version (set after successful start).
    pub async fn negotiated_protocol_version(&self) -> Option<u32> {
        *self.negotiated_protocol_version.lock().await
    }

    /// Set up notification and request handlers.
    async fn setup_handlers(&self) -> Result<()> {
        let rpc = self.rpc.lock().await;
        let rpc = rpc.as_ref().ok_or(CopilotError::NotConnected)?;

        // Clone Arc references for the handlers
        let sessions = Arc::clone(&self.sessions);
        let lifecycle_handlers = Arc::clone(&self.lifecycle_handlers);

        // Set up notification handler for session events and lifecycle events
        rpc.set_notification_handler(move |method, params| {
            if method == "session.event" {
                let sessions = Arc::clone(&sessions);
                let params = params.clone();

                // Spawn a task to handle the event
                tokio::spawn(async move {
                    if let Some(session_id) = params.get("sessionId").and_then(|v| v.as_str()) {
                        if let Some(session) = sessions.read().await.get(session_id) {
                            if let Some(event_data) = params.get("event") {
                                if let Ok(event) = SessionEvent::from_json(event_data) {
                                    session.dispatch_event(event).await;
                                }
                            }
                        }
                    }
                });
            } else if method == "session.lifecycle" {
                let lifecycle_handlers = Arc::clone(&lifecycle_handlers);
                let params = params.clone();

                tokio::spawn(async move {
                    if let Ok(event) = serde_json::from_value::<SessionLifecycleEvent>(params) {
                        let handlers = lifecycle_handlers.read().await;
                        for handler in handlers.values() {
                            handler(&event);
                        }
                    }
                });
            }
        })
        .await;

        // Clone Arc references for request handler
        let sessions_for_requests = Arc::clone(&self.sessions);

        // Protocol v2 backward-compatibility adapters.
        // v2 servers send tool.call / permission.request as RPC requests.
        // v3 servers send them as broadcast session events (handled in Session::handle_broadcast_event).
        // We always register v2 handlers; a v3 server will simply never send these requests.
        rpc.set_request_handler(move |method, params| {
            use crate::jsonrpc::JsonRpcError;

            let sessions = Arc::clone(&sessions_for_requests);
            let method = method.to_string();
            let params = params.clone();

            Box::pin(async move {
                let result = match method.as_str() {
                    "tool.call" => handle_tool_call(&sessions, &params).await,
                    "permission.request" => handle_permission_request(&sessions, &params).await,
                    "userInput.request" => handle_user_input_request(&sessions, &params).await,
                    "hooks.invoke" => handle_hooks_invoke(&sessions, &params).await,
                    _ => {
                        return Err(JsonRpcError::new(
                            -32601,
                            format!("Unknown method: {}", method),
                        ));
                    }
                };

                result.map_err(|e| JsonRpcError::new(-32000, e.to_string()))
            })
        })
        .await;

        Ok(())
    }

    /// Create a session object with the invoke function.
    async fn create_session_object(
        &self,
        session_id: String,
        workspace_path: Option<String>,
    ) -> Arc<Session> {
        let rpc = Arc::clone(&self.rpc);

        // Create the invoke function that captures the RPC client
        let invoke_fn = move |method: &str, params: Option<Value>| {
            let rpc = Arc::clone(&rpc);
            let method = method.to_string();

            Box::pin(async move {
                let rpc = rpc.lock().await;
                let rpc = rpc.as_ref().ok_or(CopilotError::NotConnected)?;
                rpc.invoke(&method, params).await
            }) as crate::session::InvokeFuture
        };

        Arc::new(Session::new(session_id, workspace_path, invoke_fn))
    }
}

// =============================================================================
// Client Builder
// =============================================================================

/// Builder for creating a Copilot client.
#[derive(Debug, Default)]
pub struct ClientBuilder {
    options: ClientOptions,
}

impl ClientBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the CLI executable path.
    pub fn cli_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.options.cli_path = Some(path.into());
        self
    }

    /// Set additional CLI arguments passed to the Copilot CLI.
    pub fn cli_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.options.cli_args = Some(args.into_iter().map(Into::into).collect());
        self
    }

    /// Add a single CLI argument passed to the Copilot CLI.
    pub fn cli_arg(mut self, arg: impl Into<String>) -> Self {
        self.options
            .cli_args
            .get_or_insert_with(Vec::new)
            .push(arg.into());
        self
    }

    /// Use stdio mode (default).
    pub fn use_stdio(mut self, use_stdio: bool) -> Self {
        self.options.use_stdio = use_stdio;
        self
    }

    /// Set the CLI URL for TCP mode.
    ///
    /// Supports: `"host:port"`, `"http://host:port"`, or `"port"` (defaults to localhost).
    pub fn cli_url(mut self, url: impl Into<String>) -> Self {
        self.options.cli_url = Some(url.into());
        self.options.use_stdio = false;
        self
    }

    /// Set port for TCP mode (ignored for stdio mode).
    ///
    /// Use `0` to let the CLI choose a random available port.
    pub fn port(mut self, port: u16) -> Self {
        self.options.port = port;
        self
    }

    /// Auto-start the connection on first use.
    pub fn auto_start(mut self, auto_start: bool) -> Self {
        self.options.auto_start = auto_start;
        self
    }

    /// Auto-restart the connection after a fatal failure.
    pub fn auto_restart(mut self, auto_restart: bool) -> Self {
        self.options.auto_restart = auto_restart;
        self
    }

    /// Set the log level.
    pub fn log_level(mut self, level: LogLevel) -> Self {
        self.options.log_level = level;
        self
    }

    /// Set the working directory.
    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.options.cwd = Some(dir.into());
        self
    }

    /// Add an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options
            .environment
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }

    /// Set a GitHub personal access token for authentication.
    pub fn github_token(mut self, token: impl Into<String>) -> Self {
        self.options.github_token = Some(token.into());
        self
    }

    /// Set whether to use the logged-in user for auth.
    pub fn use_logged_in_user(mut self, value: bool) -> Self {
        self.options.use_logged_in_user = Some(value);
        self
    }

    /// Add a single tool specification to deny.
    ///
    /// Passed as `--deny-tool` to the CLI. Takes precedence over allow options.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use copilot_sdk::Client;
    ///
    /// let client = Client::builder()
    ///     .deny_tool("shell(git push)")
    ///     .deny_tool("shell(git commit)")
    ///     .deny_tool("shell(rm)")
    ///     .build()?;
    /// # Ok::<(), copilot_sdk::CopilotError>(())
    /// ```
    pub fn deny_tool(mut self, tool_spec: impl Into<String>) -> Self {
        self.options
            .deny_tools
            .get_or_insert_with(Vec::new)
            .push(tool_spec.into());
        self
    }

    /// Set multiple tool specifications to deny.
    ///
    /// Passed as `--deny-tool` arguments to the CLI.
    pub fn deny_tools<I, S>(mut self, tool_specs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.options.deny_tools = Some(tool_specs.into_iter().map(Into::into).collect());
        self
    }

    /// Add a single tool specification to allow without manual approval.
    ///
    /// Passed as `--allow-tool` to the CLI.
    pub fn allow_tool(mut self, tool_spec: impl Into<String>) -> Self {
        self.options
            .allow_tools
            .get_or_insert_with(Vec::new)
            .push(tool_spec.into());
        self
    }

    /// Set multiple tool specifications to allow without manual approval.
    ///
    /// Passed as `--allow-tool` arguments to the CLI.
    pub fn allow_tools<I, S>(mut self, tool_specs: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.options.allow_tools = Some(tool_specs.into_iter().map(Into::into).collect());
        self
    }

    /// Set the telemetry configuration.
    pub fn telemetry(mut self, config: TelemetryConfig) -> Self {
        self.options.telemetry = Some(config);
        self
    }

    /// Set a custom model list provider for BYOK.
    ///
    /// When set, `list_models()` will call this handler instead of querying the CLI.
    pub fn on_list_models<F, Fut>(mut self, handler: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = std::result::Result<Vec<ModelInfo>, CopilotError>>
            + Send
            + 'static,
    {
        self.options.on_list_models = Some(Arc::new(move || Box::pin(handler())));
        self
    }

    /// Allow all tools without manual approval.
    ///
    /// Passes `--allow-all-tools` to the CLI. Use with `deny_tool()` to create
    /// an allowlist with specific exceptions.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use copilot_sdk::Client;
    ///
    /// // Allow everything except dangerous git operations and rm
    /// let client = Client::builder()
    ///     .allow_all_tools(true)
    ///     .deny_tool("shell(git push)")
    ///     .deny_tool("shell(git commit)")
    ///     .deny_tool("shell(rm)")
    ///     .build()?;
    /// # Ok::<(), copilot_sdk::CopilotError>(())
    /// ```
    pub fn allow_all_tools(mut self, allow: bool) -> Self {
        self.options.allow_all_tools = allow;
        self
    }

    /// Build the client.
    pub fn build(self) -> Result<Client> {
        Client::new(self.options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let client = Client::builder()
            .cli_path("/usr/bin/copilot")
            .cli_arg("--foo")
            .use_stdio(true)
            .log_level(LogLevel::Debug)
            .cwd("/tmp")
            .env("FOO", "bar")
            .build();

        assert!(client.is_ok());
    }

    #[test]
    fn test_client_builder_deny_allow_tools() {
        let client = Client::builder()
            .allow_all_tools(true)
            .deny_tool("shell(git push)")
            .deny_tool("shell(git commit)")
            .deny_tool("shell(rm)")
            .allow_tool("shell(ls)")
            .build()
            .unwrap();

        assert!(client.options.allow_all_tools);
        assert_eq!(
            client.options.deny_tools,
            Some(vec![
                "shell(git push)".to_string(),
                "shell(git commit)".to_string(),
                "shell(rm)".to_string(),
            ])
        );
        assert_eq!(
            client.options.allow_tools,
            Some(vec!["shell(ls)".to_string()])
        );
    }

    #[test]
    fn test_client_builder_deny_tools_batch() {
        let client = Client::builder()
            .deny_tools(vec!["shell(git push)", "shell(git add)"])
            .build()
            .unwrap();

        assert_eq!(
            client.options.deny_tools,
            Some(vec![
                "shell(git push)".to_string(),
                "shell(git add)".to_string(),
            ])
        );
    }

    #[test]
    fn test_client_mutually_exclusive_options() {
        let options = ClientOptions {
            cli_path: Some("/usr/bin/copilot".into()),
            cli_url: Some("http://localhost:8080".into()),
            ..Default::default()
        };
        assert!(matches!(
            Client::new(options),
            Err(CopilotError::InvalidConfig(_))
        ));

        let options = ClientOptions {
            cli_url: Some("localhost:8080".into()),
            port: 1234,
            ..Default::default()
        };
        assert!(matches!(
            Client::new(options),
            Err(CopilotError::InvalidConfig(_))
        ));

        let options = ClientOptions {
            use_stdio: true,
            port: 1234,
            ..Default::default()
        };
        assert!(matches!(
            Client::new(options),
            Err(CopilotError::InvalidConfig(_))
        ));

        // github_token + cli_url is invalid
        let options = ClientOptions {
            cli_url: Some("localhost:8080".into()),
            github_token: Some("ghp_abc123".into()),
            ..Default::default()
        };
        assert!(matches!(
            Client::new(options),
            Err(CopilotError::InvalidConfig(_))
        ));

        // use_logged_in_user + cli_url is invalid
        let options = ClientOptions {
            cli_url: Some("localhost:8080".into()),
            use_logged_in_user: Some(true),
            ..Default::default()
        };
        assert!(matches!(
            Client::new(options),
            Err(CopilotError::InvalidConfig(_))
        ));
    }

    #[tokio::test]
    async fn test_client_state_initial() {
        let client = Client::new(ClientOptions::default()).unwrap();
        assert_eq!(client.state().await, ConnectionState::Disconnected);
    }

    #[test]
    fn test_normalize_tool_arguments_object() {
        let params = json!({
            "arguments": { "n": 42 }
        });
        assert_eq!(normalize_tool_arguments(&params), json!({ "n": 42 }));
    }

    #[test]
    fn test_normalize_tool_arguments_string() {
        let params = json!({
            "arguments": "{\"n\":42}"
        });
        assert_eq!(normalize_tool_arguments(&params), json!({ "n": 42 }));
    }

    #[test]
    fn test_normalize_tool_arguments_fallback_arguments_json() {
        let params = json!({
            "argumentsJson": "{\"text\":\"hello\",\"shift\":-5}"
        });
        assert_eq!(
            normalize_tool_arguments(&params),
            json!({ "text": "hello", "shift": -5 })
        );
    }

    #[test]
    fn test_normalize_tool_arguments_invalid_json_string() {
        let params = json!({
            "arguments": "{not valid json"
        });
        assert_eq!(normalize_tool_arguments(&params), json!({}));
    }
}
