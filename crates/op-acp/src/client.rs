//! ACP connection — connect to a local (stdio) or remote (WebSocket)
//! agent and drive the initialize / session / prompt handshake.
//! Port of `pen-acp/src/client.ts`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_client_protocol::schema::ProtocolVersion;
use op_util::cli_output::BoundedTail;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::jsonrpc::{dispatch_inbound, JsonRpcEngine, NOTIFICATION_CAPACITY, OUTBOUND_CAPACITY};
use crate::protocol::{
    AcpStopReason, AgentCapabilities, AuthMethod, InitializeResult, NewSessionResult, PromptResult,
    SessionConfigOption, SessionConfigOptionValue, SessionNotification,
    SetSessionConfigOptionResponse, METHOD_INITIALIZE, METHOD_SESSION_CANCEL, METHOD_SESSION_NEW,
    METHOD_SESSION_PROMPT, METHOD_SESSION_SET_CONFIG_OPTION, PROTOCOL_VERSION,
};
#[cfg(feature = "remote")]
use crate::transport::MAX_INBOUND_FRAME_BYTES;
use crate::transport::{read_frame, write_frame};
use crate::types::{AcpAgentConfig, AcpAgentInfo, AcpError, ConnectionType};

/// Per-request timeout for the handshake calls.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
/// A remote TCP/TLS/WebSocket dial must not hold a worker forever.
#[cfg(feature = "remote")]
const REMOTE_DIAL_TIMEOUT: Duration = Duration::from_secs(15);
/// A prompt turn can run a long while — generous ceiling.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(600);
/// Best-effort protocol cancellation must not delay local teardown.
const CANCEL_QUEUE_TIMEOUT: Duration = Duration::from_secs(1);
/// Graceful process-tree termination before force-killing the group.
const PROCESS_SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

/// Byte budget for the retained stderr tail of a local agent. Fixed:
/// the drain task exists to keep the child's pipe from filling, so its
/// buffer must never grow with the child's output.
const STDERR_TAIL_CAP: usize = 16 * 1024;

/// Line budget paired with [`STDERR_TAIL_CAP`].
const STDERR_TAIL_LINES: usize = 256;

/// How long a failed handshake waits for the stderr drain to reach EOF
/// before quoting the agent. The child is killed first, so this is
/// normally one scheduler round; bounded so a wedged reader cannot hang
/// the connect path.
const STDERR_DRAIN_GRACE: Duration = Duration::from_secs(2);

/// One MCP server endpoint advertised to the agent in `session/new`
/// (`mcpServers[]`). Serialized as `{ name, type: "http", url,
/// headers: [] }` — the shape `claude-agent-acp` accepts (TS parity:
/// `apps/web/server/api/ai/agent.ts:513-521`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpHttpServer {
    /// Server name the agent prefixes tool ids with
    /// (`mcp__<name>__*`).
    pub name: String,
    /// HTTP endpoint, e.g. `http://127.0.0.1:3100/mcp`.
    pub url: String,
}

/// Extra `session/new` payload — MCP tool endpoints + the optional
/// `_meta.systemPrompt` override.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NewSessionOptions {
    /// MCP servers the agent should connect to for tools.
    pub mcp_servers: Vec<McpHttpServer>,
    /// Override the agent's default system prompt via
    /// `_meta.systemPrompt` (claude-agent-acp honors this; agents
    /// that don't simply ignore the unknown `_meta` key).
    pub system_prompt_meta: Option<String>,
}

/// A newly-created stable-v1 ACP session, including the configuration state
/// the agent advertised for model/mode/reasoning selectors.
#[derive(Debug, Clone)]
pub struct AcpSession {
    pub session_id: String,
    pub config_options: Vec<SessionConfigOption>,
}

/// A live ACP connection to one agent.
pub struct AcpConnection {
    engine: JsonRpcEngine,
    notifications: Option<mpsc::Receiver<SessionNotification>>,
    child: Option<Child>,
    tasks: Vec<JoinHandle<()>>,
    stderr_task: Option<JoinHandle<()>>,
    agent_info: AcpAgentInfo,
    protocol_version: ProtocolVersion,
    agent_capabilities: AgentCapabilities,
    auth_methods: Vec<AuthMethod>,
    /// The most recent lines a locally spawned agent wrote to stderr.
    /// `None` for remote (WebSocket) agents and for connections built
    /// over an arbitrary stream pair — neither has a stderr pipe.
    stderr_tail: Option<Arc<Mutex<BoundedTail>>>,
}

impl AcpConnection {
    /// Build a connection over an arbitrary async byte stream pair
    /// (stdio of a child, a test duplex, …). Spawns the reader +
    /// writer tasks; does NOT run the `initialize` handshake.
    pub fn new<R, W>(read: R, write: W, child: Option<Child>) -> AcpConnection
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
        let (notif_tx, notif_rx) = mpsc::channel::<SessionNotification>(NOTIFICATION_CAPACITY);
        let engine = JsonRpcEngine::new(out_tx);
        let pending = engine.pending();
        let reply_tx = engine.out_tx();
        let reader_engine = engine.clone();

        // Writer task — drain outbound frames onto the byte stream.
        let mut write = write;
        let writer = tokio::spawn(async move {
            while let Some(frame) = out_rx.recv().await {
                if write_frame(&mut write, &frame).await.is_err() {
                    break;
                }
            }
        });

        // Reader task — classify + dispatch every inbound frame until
        // EOF or a transport failure ends the stream.
        let reader = tokio::spawn(async move {
            let mut buf = BufReader::new(read);
            let failure = loop {
                match read_frame(&mut buf).await {
                    Ok(Some(value)) => {
                        if let Err(error) = dispatch_inbound(value, &pending, &notif_tx, &reply_tx)
                        {
                            break error;
                        }
                    }
                    Ok(None) => break AcpError::Closed,
                    Err(error) => break error,
                }
            };
            reader_engine.fail(failure);
        });

        AcpConnection {
            engine,
            notifications: Some(notif_rx),
            child,
            tasks: vec![writer, reader],
            stderr_task: None,
            agent_info: AcpAgentInfo::default(),
            protocol_version: ProtocolVersion::V1,
            agent_capabilities: AgentCapabilities::default(),
            auth_methods: Vec::new(),
            stderr_tail: None,
        }
    }

    /// The redacted, length-capped tail of a local agent's stderr, or
    /// `None` when it printed nothing (or has no stderr pipe at all).
    ///
    /// An ACP agent that dies during the handshake reports the reason
    /// on stderr — a missing API key, an unsupported flag, a broken
    /// install. That text used to be read and dropped line by line, so
    /// the connection failure surfaced as a bare timeout.
    pub fn stderr_tail(&self) -> Option<String> {
        let tail = self.stderr_tail.as_ref()?;
        let text = tail.lock().ok()?.text();
        op_util::cli_output::diagnostic_tail(&text)
    }

    /// Run the `initialize` handshake, recording the agent's identity.
    /// `fallback_name` is used when the agent reports none.
    pub async fn initialize(&mut self, fallback_name: &str) -> Result<(), AcpError> {
        let params = serde_json::json!({
            "protocolVersion": PROTOCOL_VERSION,
            "clientCapabilities": {},
            "clientInfo": { "name": "openpencil", "version": env!("CARGO_PKG_VERSION") }
        });
        let result = self
            .engine
            .call(METHOD_INITIALIZE, params, HANDSHAKE_TIMEOUT)
            .await?;
        let parsed: InitializeResult =
            serde_json::from_value(result).map_err(|e| AcpError::Protocol(e.to_string()))?;
        if parsed.protocol_version != ProtocolVersion::V1 {
            return Err(AcpError::Protocol(format!(
                "agent selected unsupported protocol version {}; OpenPencil supports stable v1",
                parsed.protocol_version.as_u16()
            )));
        }
        self.protocol_version = parsed.protocol_version;
        self.agent_capabilities = parsed.agent_capabilities;
        self.auth_methods = parsed.auth_methods;
        self.agent_info = match parsed.agent_info {
            Some(info) => AcpAgentInfo {
                name: if info.name.trim().is_empty() {
                    fallback_name.to_string()
                } else {
                    info.name
                },
                title: info.title,
                version: (!info.version.trim().is_empty()).then_some(info.version),
            },
            None => AcpAgentInfo {
                name: fallback_name.to_string(),
                title: None,
                version: None,
            },
        };
        Ok(())
    }

    /// Open a new session, retaining the agent's initial config options.
    pub async fn new_session(&self) -> Result<AcpSession, AcpError> {
        self.new_session_with(&NewSessionOptions::default()).await
    }

    /// Open a new session carrying MCP server endpoints + an optional
    /// `_meta.systemPrompt` override, returning the session id.
    ///
    /// Mirrors the TS host's `session/new` payload
    /// (`apps/web/server/api/ai/agent.ts:576-580`): `cwd` +
    /// `mcpServers` (HTTP endpoints the agent connects to for tools)
    /// + `_meta: { systemPrompt }` (claude-agent-acp honors it; other
    ///   agents ignore unknown `_meta`).
    pub async fn new_session_with(
        &self,
        options: &NewSessionOptions,
    ) -> Result<AcpSession, AcpError> {
        if !options.mcp_servers.is_empty() && !self.agent_capabilities.mcp_capabilities.http {
            return Err(AcpError::Protocol(
                "agent did not advertise agentCapabilities.mcpCapabilities.http; refusing to send an HTTP MCP endpoint"
                    .into(),
            ));
        }
        match self.new_session_once(options).await {
            Err(AcpError::Rpc { code: -32_000, .. }) => {
                self.authenticate_unambiguous().await?;
                self.new_session_once(options).await
            }
            result => result,
        }
    }

    async fn new_session_once(&self, options: &NewSessionOptions) -> Result<AcpSession, AcpError> {
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let servers: Vec<Value> = options
            .mcp_servers
            .iter()
            .map(|server| {
                // NOTE: claude-agent-acp expects `type: 'http' | 'sse'`
                // (not `transport`) — TS agent.ts:507 comment.
                serde_json::json!({
                    "name": server.name,
                    "type": "http",
                    "url": server.url,
                    "headers": [],
                })
            })
            .collect();
        let mut params = serde_json::json!({ "cwd": cwd, "mcpServers": servers });
        if let Some(prompt) = &options.system_prompt_meta {
            params["_meta"] = serde_json::json!({ "systemPrompt": prompt });
        }
        let result = self
            .engine
            .call(METHOD_SESSION_NEW, params, HANDSHAKE_TIMEOUT)
            .await?;
        let parsed: NewSessionResult =
            serde_json::from_value(result).map_err(|e| AcpError::Protocol(e.to_string()))?;
        Ok(AcpSession {
            session_id: parsed.session_id.to_string(),
            config_options: parsed.config_options.unwrap_or_default(),
        })
    }

    /// Change one stable-v1 session config option and retain the complete
    /// state returned by the agent. String values are select ids; booleans
    /// carry the required `type: "boolean"` discriminator.
    pub async fn set_session_config_option(
        &self,
        session_id: &str,
        config_id: &str,
        value: SessionConfigOptionValue,
    ) -> Result<Vec<SessionConfigOption>, AcpError> {
        let mut params = serde_json::json!({
            "sessionId": session_id,
            "configId": config_id,
        });
        let encoded = serde_json::to_value(value)
            .map_err(|e| AcpError::Protocol(format!("invalid session config value: {e}")))?;
        let object = encoded.as_object().ok_or_else(|| {
            AcpError::Protocol("session config value did not serialize as an object".into())
        })?;
        for (key, value) in object {
            params[key] = value.clone();
        }
        let result = self
            .engine
            .call(METHOD_SESSION_SET_CONFIG_OPTION, params, HANDSHAKE_TIMEOUT)
            .await?;
        let parsed: SetSessionConfigOptionResponse =
            serde_json::from_value(result).map_err(|e| AcpError::Protocol(e.to_string()))?;
        Ok(parsed.config_options)
    }

    /// Drive one prompt turn. Resolves when the agent finishes the
    /// turn; streamed output arrives on the notification channel.
    pub async fn prompt(&self, session_id: &str, text: &str) -> Result<AcpStopReason, AcpError> {
        let params = serde_json::json!({
            "sessionId": session_id,
            "prompt": [ { "type": "text", "text": text } ]
        });
        let result = self
            .engine
            .call(METHOD_SESSION_PROMPT, params, PROMPT_TIMEOUT)
            .await?;
        let parsed: PromptResult =
            serde_json::from_value(result).map_err(|e| AcpError::Protocol(e.to_string()))?;
        Ok(parsed.stop_reason)
    }

    /// Cancel all work for one session. ACP defines this as a notification,
    /// not a request; a conforming prompt resolves with `stopReason:
    /// "cancelled"` after receiving it.
    pub async fn cancel_session(&self, session_id: &str) -> Result<(), AcpError> {
        self.engine
            .notify(
                METHOD_SESSION_CANCEL,
                serde_json::json!({ "sessionId": session_id }),
                CANCEL_QUEUE_TIMEOUT,
            )
            .await
    }

    /// Take the `session/update` notification receiver — callable
    /// once; subsequent calls return `None`.
    pub fn take_notifications(&mut self) -> Option<mpsc::Receiver<SessionNotification>> {
        self.notifications.take()
    }

    /// The agent's identity from the `initialize` handshake.
    pub fn agent_info(&self) -> &AcpAgentInfo {
        &self.agent_info
    }

    /// Negotiated stable ACP protocol version.
    pub fn protocol_version(&self) -> ProtocolVersion {
        self.protocol_version
    }

    /// Capabilities retained from the initialize response.
    pub fn agent_capabilities(&self) -> &AgentCapabilities {
        &self.agent_capabilities
    }

    /// Authentication methods retained from the initialize response.
    pub fn auth_methods(&self) -> &[AuthMethod] {
        &self.auth_methods
    }
}

/// Connect to the agent described by `config`, running the
/// `initialize` handshake. Routes to the local or remote transport.
pub async fn connect_acp_agent(config: &AcpAgentConfig) -> Result<AcpConnection, AcpError> {
    match config.connection_type {
        ConnectionType::Local => connect_local(config).await,
        ConnectionType::Remote => connect_remote(config).await,
    }
}

/// Spawn a local agent process and connect over its stdio.
async fn connect_local(config: &AcpAgentConfig) -> Result<AcpConnection, AcpError> {
    let command = config
        .command
        .as_ref()
        .ok_or_else(|| AcpError::Config("local ACP agent requires a command".into()))?;

    let resolved = resolve_local_command(command, &config.env);
    let mut cmd = build_local_command(&resolved, &config.args);
    apply_local_environment(&mut cmd, &config.env);
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn().map_err(|e| AcpError::Spawn(e.to_string()))?;
    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| AcpError::Spawn("child has no stdin".into()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| AcpError::Spawn("child has no stdout".into()))?;
    // Drain stderr so the child never blocks on a full pipe — that is
    // why this task exists and it must keep reading to EOF. What
    // changed: the lines are now kept in a FIXED-CAPACITY tail instead
    // of being dropped, so a handshake failure can quote the agent
    // instead of guessing. The buffer never grows with the child's
    // output, so the anti-blocking guarantee is untouched.
    let stderr_tail = Arc::new(Mutex::new(BoundedTail::new(
        STDERR_TAIL_CAP,
        STDERR_TAIL_LINES,
    )));
    let stderr_drain = child.stderr.take().map(|stderr| {
        let capture = Arc::clone(&stderr_tail);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(mut buf) = capture.lock() {
                    buf.push_line(&line);
                }
            }
        })
    });

    let mut conn = AcpConnection::new(stdout, stdin, Some(child));
    conn.stderr_tail = Some(stderr_tail);
    conn.stderr_task = stderr_drain;
    match conn.initialize(&config.display_name).await {
        Ok(()) => Ok(conn),
        Err(error) => {
            // An agent that starts up broken dies at once, and its
            // stdout EOF is what fails the handshake — so the drain task
            // is racing that same instant and may not have been polled.
            // Measured 2026-08-07 under 16 concurrent connects: 5-8% of
            // failures came back with an empty tail, the same shape as
            // the CLI bridge's.
            //
            // `disconnect()` first, deliberately: this connection is
            // already doomed, and killing the child closes its stderr so
            // the drain hits EOF now instead of us waiting out the grace
            // on a child that is still running (the handshake-timeout
            // case). It aborts the reader/writer tasks, which is the
            // existing shutdown semantics; the drain task is NOT among
            // them, so joining it here does not fight `Drop`.
            conn.shutdown().await;
            Err(with_agent_output(error, conn.stderr_tail()))
        }
    }
}

/// Host environment keys that a local ACP child is allowed to inherit.
/// User-configured `config.env` entries are explicit and override this set.
fn local_env_allowed(key: &str) -> bool {
    // Windows environment keys are case-insensitive and are commonly exposed
    // as `Path`, `ComSpec`, and `SystemRoot`; normalize so the allowlist does
    // not accidentally remove the Node runtime needed by npm ACP shims.
    let key = key.to_ascii_uppercase();
    matches!(
        key.as_str(),
        "PATH"
            | "HOME"
            | "USER"
            | "LOGNAME"
            | "SHELL"
            | "TMPDIR"
            | "TMP"
            | "TEMP"
            | "LANG"
            | "LC_ALL"
            | "LC_CTYPE"
            | "TERM"
            | "XDG_CONFIG_HOME"
            | "XDG_CACHE_HOME"
            | "XDG_DATA_HOME"
            | "XDG_STATE_HOME"
            | "SSL_CERT_FILE"
            | "SSL_CERT_DIR"
            | "NODE_EXTRA_CA_CERTS"
            | "HTTP_PROXY"
            | "HTTPS_PROXY"
            | "ALL_PROXY"
            | "NO_PROXY"
            | "SYSTEMROOT"
            | "WINDIR"
            | "COMSPEC"
            | "PATHEXT"
            | "USERPROFILE"
            | "APPDATA"
            | "LOCALAPPDATA"
            | "PROGRAMDATA"
    ) || key.starts_with("LC_")
}

fn apply_local_environment(cmd: &mut Command, configured: &BTreeMap<String, String>) {
    cmd.env_clear();
    for (key, value) in std::env::vars().filter(|(key, _)| local_env_allowed(key)) {
        cmd.env(key, value);
    }
    for (key, value) in configured {
        cmd.env(key, value);
    }
}

fn effective_local_path(configured: &BTreeMap<String, String>) -> String {
    configured
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| value.clone())
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_default()
}

fn command_candidates(command: &str, path: &str) -> Vec<PathBuf> {
    let command_path = Path::new(command);
    if command_path.is_absolute() || command_path.components().count() > 1 {
        return vec![command_path.to_path_buf()];
    }
    let mut out = Vec::new();
    for dir in std::env::split_paths(path) {
        let base = dir.join(command);
        out.push(base.clone());
        #[cfg(windows)]
        for ext in ["exe", "cmd", "bat", "com", "ps1"] {
            out.push(base.with_extension(ext));
        }
    }
    #[cfg(unix)]
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        for dir in [".local/bin", ".npm-global/bin", ".bun/bin", ".volta/bin"] {
            out.push(home.join(dir).join(command));
        }
    }
    #[cfg(windows)]
    if let Some(appdata) = std::env::var_os("APPDATA") {
        let base = PathBuf::from(appdata).join("npm").join(command);
        for ext in ["exe", "cmd", "bat", "com", "ps1"] {
            out.push(base.with_extension(ext));
        }
    }
    #[cfg(unix)]
    {
        out.push(PathBuf::from("/usr/local/bin").join(command));
        out.push(PathBuf::from("/opt/homebrew/bin").join(command));
    }
    out
}

fn is_executable_candidate(candidate: &Path) -> bool {
    if !candidate.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        candidate
            .metadata()
            .is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn resolve_local_command(command: &str, configured: &BTreeMap<String, String>) -> String {
    let path = effective_local_path(configured);
    command_candidates(command, &path)
        .into_iter()
        .find(|candidate| is_executable_candidate(candidate))
        .unwrap_or_else(|| PathBuf::from(command))
        .to_string_lossy()
        .into_owned()
}

fn build_local_command(command: &str, args: &[String]) -> Command {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let extension = Path::new(command)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        if extension.eq_ignore_ascii_case("ps1") {
            let mut cmd = Command::new("powershell.exe");
            cmd.arg("-NoProfile")
                .arg("-NonInteractive")
                .arg("-File")
                .arg(command)
                .args(args)
                .creation_flags(CREATE_NO_WINDOW);
            cmd
        } else if extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat") {
            let mut cmd = Command::new("cmd.exe");
            cmd.arg("/d").arg("/c").arg(command).args(args);
            cmd.creation_flags(CREATE_NO_WINDOW);
            cmd
        } else {
            let mut cmd = Command::new(command);
            cmd.args(args).creation_flags(CREATE_NO_WINDOW);
            cmd
        }
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new(command);
        cmd.args(args);
        #[cfg(unix)]
        cmd.process_group(0);
        cmd
    }
}

/// Fold a local agent's own last words into a handshake failure.
///
/// The variant is preserved wherever it carries a message (callers
/// switch on it); `Closed` has no payload, so it becomes a `Transport`
/// error that can carry one. Without any output the error is returned
/// untouched rather than padded with an empty quote.
fn with_agent_output(error: AcpError, tail: Option<String>) -> AcpError {
    let Some(tail) = tail else {
        return error;
    };
    match error {
        AcpError::Config(message) => AcpError::Config(format!("{message}; agent said: {tail}")),
        AcpError::Spawn(message) => AcpError::Spawn(format!("{message}; agent said: {tail}")),
        AcpError::Transport(message) => {
            AcpError::Transport(format!("{message}; agent said: {tail}"))
        }
        AcpError::Protocol(message) => AcpError::Protocol(format!("{message}; agent said: {tail}")),
        AcpError::Rpc { code, message } => AcpError::Rpc {
            code,
            message: format!("{message}; agent said: {tail}"),
        },
        AcpError::Closed => {
            AcpError::Transport(format!("ACP connection closed; agent said: {tail}"))
        }
    }
}

#[cfg(not(feature = "remote"))]
async fn connect_remote(_config: &AcpAgentConfig) -> Result<AcpConnection, AcpError> {
    Err(AcpError::Config(
        "remote ACP agents need the `remote` feature".into(),
    ))
}

#[cfg(feature = "remote")]
fn remote_rustls_connector() -> tokio_tungstenite::Connector {
    let mut roots = rustls::RootCertStore::empty();
    let native = rustls_native_certs::load_native_certs();
    let _ = roots.add_parsable_certificates(native.certs);
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("ring supports default TLS protocol versions")
        .with_root_certificates(roots)
        .with_no_client_auth();
    tokio_tungstenite::Connector::Rustls(Arc::new(config))
}

#[cfg(feature = "remote")]
fn remote_websocket_config() -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
        max_message_size: Some(MAX_INBOUND_FRAME_BYTES),
        max_frame_size: Some(MAX_INBOUND_FRAME_BYTES),
        ..Default::default()
    }
}

/// Connect to a remote agent over a WebSocket endpoint.
#[cfg(feature = "remote")]
async fn connect_remote(config: &AcpAgentConfig) -> Result<AcpConnection, AcpError> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let url = config
        .url
        .as_ref()
        .ok_or_else(|| AcpError::Config("remote ACP agent requires a url".into()))?;
    let dial = tokio_tungstenite::connect_async_tls_with_config(
        url,
        Some(remote_websocket_config()),
        false,
        Some(remote_rustls_connector()),
    );
    let (ws, _resp) = tokio::time::timeout(REMOTE_DIAL_TIMEOUT, dial)
        .await
        .map_err(|_| AcpError::Transport("remote ACP WebSocket connect timed out".into()))?
        .map_err(|e| AcpError::Transport(e.to_string()))?;
    let (mut sink, mut stream) = ws.split();

    let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
    let (notif_tx, notif_rx) = mpsc::channel::<SessionNotification>(NOTIFICATION_CAPACITY);
    let engine = JsonRpcEngine::new(out_tx);
    let pending = engine.pending();
    let reply_tx = engine.out_tx();
    let reader_engine = engine.clone();

    // Writer task — each outbound frame is one WebSocket text message.
    let writer = tokio::spawn(async move {
        while let Some(frame) = out_rx.recv().await {
            let Ok(text) = serde_json::to_string(&frame) else {
                continue;
            };
            if sink.send(Message::Text(text)).await.is_err() {
                break;
            }
        }
    });
    // Reader task — each text message is one inbound frame.
    let reader = tokio::spawn(async move {
        let failure = loop {
            let message = match stream.next().await {
                Some(Ok(message)) => message,
                Some(Err(error)) => {
                    break AcpError::Transport(format!("remote ACP WebSocket: {error}"));
                }
                None => break AcpError::Closed,
            };
            let value = match message {
                Message::Text(text) => serde_json::from_str::<Value>(text.trim()),
                Message::Binary(bytes) => serde_json::from_slice::<Value>(&bytes),
                Message::Close(_) => break AcpError::Closed,
                _ => continue,
            };
            let value = match value {
                Ok(value) => value,
                Err(error) => {
                    break AcpError::Protocol(format!("invalid remote ACP JSON frame: {error}"));
                }
            };
            if let Err(error) = dispatch_inbound(value, &pending, &notif_tx, &reply_tx) {
                break error;
            }
        };
        reader_engine.fail(failure);
    });

    let mut conn = AcpConnection {
        engine,
        notifications: Some(notif_rx),
        child: None,
        tasks: vec![writer, reader],
        stderr_task: None,
        agent_info: AcpAgentInfo::default(),
        protocol_version: ProtocolVersion::V1,
        agent_capabilities: AgentCapabilities::default(),
        auth_methods: Vec::new(),
        // A WebSocket agent runs elsewhere — there is no stderr pipe.
        stderr_tail: None,
    };
    match conn.initialize(&config.display_name).await {
        Ok(()) => Ok(conn),
        Err(error) => {
            conn.shutdown().await;
            Err(error)
        }
    }
}

#[path = "client_lifecycle.rs"]
mod lifecycle;

#[cfg(test)]
#[path = "client_tests.rs"]
mod tests;
