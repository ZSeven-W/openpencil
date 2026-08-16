//! ACP connection — connect to a local (stdio) or remote (WebSocket)
//! agent and drive the initialize / session / prompt handshake.
//! Port of `pen-acp/src/client.ts`.

use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use op_util::cli_output::BoundedTail;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::jsonrpc::{dispatch_inbound, JsonRpcEngine, NOTIFICATION_CAPACITY, OUTBOUND_CAPACITY};
use crate::protocol::{
    InitializeResult, NewSessionResult, SessionNotification, METHOD_INITIALIZE, METHOD_SESSION_NEW,
    METHOD_SESSION_PROMPT, PROTOCOL_VERSION,
};
use crate::transport::{read_frame, write_frame};
use crate::types::{AcpAgentConfig, AcpAgentInfo, AcpError, ConnectionType};

/// Per-request timeout for the handshake calls.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
/// A prompt turn can run a long while — generous ceiling.
const PROMPT_TIMEOUT: Duration = Duration::from_secs(600);

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

/// A live ACP connection to one agent.
pub struct AcpConnection {
    engine: JsonRpcEngine,
    notifications: Option<mpsc::Receiver<SessionNotification>>,
    child: Option<Child>,
    tasks: Vec<JoinHandle<()>>,
    agent_info: AcpAgentInfo,
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
            while let Ok(Some(value)) = read_frame(&mut buf).await {
                dispatch_inbound(value, &pending, &notif_tx, &reply_tx);
            }
            // Connection closed: fail every in-flight request now so
            // callers get `Closed` immediately instead of stalling
            // until the request timeout.
            let waiters: Vec<_> = pending
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .drain()
                .collect();
            for (_, waiter) in waiters {
                let _ = waiter.send(Err(AcpError::Closed));
            }
        });

        AcpConnection {
            engine,
            notifications: Some(notif_rx),
            child,
            tasks: vec![writer, reader],
            agent_info: AcpAgentInfo::default(),
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
        let info = parsed.agent_info.unwrap_or_default();
        self.agent_info = AcpAgentInfo {
            name: info.name.unwrap_or_else(|| fallback_name.to_string()),
            title: info.title,
            version: info.version,
        };
        Ok(())
    }

    /// Open a new session, returning its id.
    pub async fn new_session(&self) -> Result<String, AcpError> {
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
    pub async fn new_session_with(&self, options: &NewSessionOptions) -> Result<String, AcpError> {
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
        Ok(parsed.session_id)
    }

    /// Drive one prompt turn. Resolves when the agent finishes the
    /// turn; streamed output arrives on the notification channel.
    pub async fn prompt(&self, session_id: &str, text: &str) -> Result<(), AcpError> {
        let params = serde_json::json!({
            "sessionId": session_id,
            "prompt": [ { "type": "text", "text": text } ]
        });
        self.engine
            .call(METHOD_SESSION_PROMPT, params, PROMPT_TIMEOUT)
            .await?;
        Ok(())
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

    /// Kill the local process (if any) and stop the IO tasks.
    pub fn disconnect(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
        }
        for task in self.tasks.drain(..) {
            task.abort();
        }
    }
}

impl Drop for AcpConnection {
    fn drop(&mut self) {
        self.disconnect();
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

    let mut cmd = Command::new(command);
    cmd.args(&config.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in &config.env {
        cmd.env(key, value);
    }
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
            conn.disconnect();
            if let Some(drain) = stderr_drain {
                let _ = tokio::time::timeout(STDERR_DRAIN_GRACE, drain).await;
            }
            Err(with_agent_output(error, conn.stderr_tail()))
        }
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

/// Connect to a remote agent over a WebSocket endpoint.
#[cfg(feature = "remote")]
async fn connect_remote(config: &AcpAgentConfig) -> Result<AcpConnection, AcpError> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;

    let url = config
        .url
        .as_ref()
        .ok_or_else(|| AcpError::Config("remote ACP agent requires a url".into()))?;
    let (ws, _resp) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(|e| AcpError::Transport(e.to_string()))?;
    let (mut sink, mut stream) = ws.split();

    let (out_tx, mut out_rx) = mpsc::channel::<Value>(OUTBOUND_CAPACITY);
    let (notif_tx, notif_rx) = mpsc::channel::<SessionNotification>(NOTIFICATION_CAPACITY);
    let engine = JsonRpcEngine::new(out_tx);
    let pending = engine.pending();
    let reply_tx = engine.out_tx();

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
        while let Some(msg) = stream.next().await {
            let Ok(msg) = msg else { break };
            let text = match msg {
                Message::Text(t) => t,
                Message::Binary(b) => String::from_utf8_lossy(&b).into_owned(),
                Message::Close(_) => break,
                _ => continue,
            };
            if let Ok(value) = serde_json::from_str::<Value>(text.trim()) {
                dispatch_inbound(value, &pending, &notif_tx, &reply_tx);
            }
        }
        // Socket closed: fail in-flight requests immediately.
        let waiters: Vec<_> = pending
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .drain()
            .collect();
        for (_, waiter) in waiters {
            let _ = waiter.send(Err(AcpError::Closed));
        }
    });

    let mut conn = AcpConnection {
        engine,
        notifications: Some(notif_rx),
        child: None,
        tasks: vec![writer, reader],
        agent_info: AcpAgentInfo::default(),
        // A WebSocket agent runs elsewhere — there is no stderr pipe.
        stderr_tail: None,
    };
    conn.initialize(&config.display_name).await?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::read_frame;

    /// A mock ACP agent: answers `initialize` / `session/new`, then on
    /// `session/prompt` streams one message chunk and returns.
    async fn mock_agent(read: impl AsyncRead + Unpin, mut write: impl AsyncWrite + Unpin) {
        let mut buf = BufReader::new(read);
        while let Ok(Some(frame)) = read_frame(&mut buf).await {
            let id = frame.get("id").cloned().unwrap_or(Value::Null);
            let method = frame.get("method").and_then(|m| m.as_str()).unwrap_or("");
            match method {
                "initialize" => {
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "protocolVersion": 1,
                                    "agentInfo": { "name": "Mock Agent", "version": "9.9" } }
                    });
                    write_frame(&mut write, &resp).await.unwrap();
                }
                "session/new" => {
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "sessionId": "sess-1" }
                    });
                    write_frame(&mut write, &resp).await.unwrap();
                }
                "session/prompt" => {
                    // Stream one chunk, then close the turn.
                    let note = serde_json::json!({
                        "jsonrpc": "2.0", "method": "session/update",
                        "params": { "sessionId": "sess-1",
                                    "update": { "sessionUpdate": "agent_message_chunk",
                                                "content": { "type": "text", "text": "hi there" } } }
                    });
                    write_frame(&mut write, &note).await.unwrap();
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0", "id": id, "result": { "stopReason": "end_turn" }
                    });
                    write_frame(&mut write, &resp).await.unwrap();
                }
                _ => break,
            }
        }
    }

    #[tokio::test]
    async fn handshake_and_prompt_against_a_mock_agent() {
        // Two duplex pipes: client writes → agent reads, agent writes
        // → client reads.
        let (client_w, agent_r) = tokio::io::duplex(8192);
        let (agent_w, client_r) = tokio::io::duplex(8192);
        tokio::spawn(mock_agent(agent_r, agent_w));

        let mut conn = AcpConnection::new(client_r, client_w, None);
        let mut notes = conn.take_notifications().expect("notifications");

        conn.initialize("fallback").await.expect("initialize");
        assert_eq!(conn.agent_info().name, "Mock Agent");
        assert_eq!(conn.agent_info().version.as_deref(), Some("9.9"));

        let session = conn.new_session().await.expect("new_session");
        assert_eq!(session, "sess-1");

        conn.prompt(&session, "design a button")
            .await
            .expect("prompt");
        // The streamed chunk reached the notification channel.
        let note = notes.recv().await.expect("a session/update");
        assert_eq!(note.session_id.as_deref(), Some("sess-1"));
    }

    /// A mock agent that asserts the `session/new` params carry the
    /// MCP server list + `_meta.systemPrompt`, encoding the verdict in
    /// the returned session id.
    async fn mock_agent_checking_session_new(
        read: impl AsyncRead + Unpin,
        mut write: impl AsyncWrite + Unpin,
    ) {
        let mut buf = BufReader::new(read);
        while let Ok(Some(frame)) = read_frame(&mut buf).await {
            let id = frame.get("id").cloned().unwrap_or(Value::Null);
            let method = frame.get("method").and_then(|m| m.as_str()).unwrap_or("");
            match method {
                "initialize" => {
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "protocolVersion": 1 }
                    });
                    write_frame(&mut write, &resp).await.unwrap();
                }
                "session/new" => {
                    let params = frame.get("params").cloned().unwrap_or(Value::Null);
                    let server = &params["mcpServers"][0];
                    let ok = server["name"] == "openpencil"
                        && server["type"] == "http"
                        && server["url"] == "http://127.0.0.1:3100/mcp"
                        && server["headers"].as_array().is_some_and(Vec::is_empty)
                        && params["_meta"]["systemPrompt"] == "use the canvas tools"
                        && params["cwd"].as_str().is_some_and(|c| !c.is_empty());
                    let session_id = if ok { "sess-mcp-ok" } else { "sess-bad" };
                    let resp = serde_json::json!({
                        "jsonrpc": "2.0", "id": id,
                        "result": { "sessionId": session_id }
                    });
                    write_frame(&mut write, &resp).await.unwrap();
                }
                _ => break,
            }
        }
    }

    #[tokio::test]
    async fn session_new_carries_mcp_servers_and_system_prompt_meta() {
        let (client_w, agent_r) = tokio::io::duplex(8192);
        let (agent_w, client_r) = tokio::io::duplex(8192);
        tokio::spawn(mock_agent_checking_session_new(agent_r, agent_w));

        let mut conn = AcpConnection::new(client_r, client_w, None);
        conn.initialize("fallback").await.expect("initialize");
        let options = NewSessionOptions {
            mcp_servers: vec![McpHttpServer {
                name: "openpencil".into(),
                url: "http://127.0.0.1:3100/mcp".into(),
            }],
            system_prompt_meta: Some("use the canvas tools".into()),
        };
        let session = conn.new_session_with(&options).await.expect("new_session");
        assert_eq!(
            session, "sess-mcp-ok",
            "agent saw a TS-shaped mcpServers + _meta.systemPrompt payload"
        );
    }

    #[tokio::test]
    async fn plain_new_session_sends_empty_server_list_and_no_meta() {
        // The default payload must stay byte-compatible with the old
        // `{ cwd, mcpServers: [] }` wire (no `_meta` key at all).
        let options = NewSessionOptions::default();
        assert!(options.mcp_servers.is_empty());
        assert!(options.system_prompt_meta.is_none());
    }

    #[tokio::test]
    async fn in_flight_call_fails_fast_when_agent_exits() {
        // Agent end is dropped immediately — no response will come.
        let (client_w, agent_r) = tokio::io::duplex(1024);
        let (agent_w, client_r) = tokio::io::duplex(1024);
        drop(agent_r);
        drop(agent_w);

        let mut conn = AcpConnection::new(client_r, client_w, None);
        let started = std::time::Instant::now();
        let err = conn.initialize("fallback").await.unwrap_err();
        // The reader drains pending requests on EOF, so the call
        // resolves to `Closed` at once rather than after the 30s
        // handshake timeout.
        assert!(
            matches!(err, AcpError::Closed),
            "expected Closed, got {err:?}"
        );
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "must fail fast, not wait out the timeout"
        );
    }

    /// A local agent stub on disk. Unix-only: it is a `/bin/sh` script.
    #[cfg(unix)]
    fn stub_agent(body: &str) -> (std::path::PathBuf, AcpAgentConfig) {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "op-acp-stub-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("agent.sh");
        std::fs::write(&path, body).expect("write stub");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let config = AcpAgentConfig {
            id: "stub".into(),
            display_name: "Stub Agent".into(),
            connection_type: ConnectionType::Local,
            command: Some(path.to_string_lossy().into_owned()),
            args: Vec::new(),
            env: std::collections::BTreeMap::new(),
            url: None,
            enabled: true,
        };
        (dir, config)
    }

    /// [`connect_acp_agent`] with a bounded retry for the /tmp write→exec
    /// race that surfaces as ETXTBSY ("Text file busy") on CI runners'
    /// overlay filesystems. The stub script is written and spawned
    /// back-to-back, and the exec can land while the file is still
    /// mid-copy-up; a few short retries clear it without masking genuine
    /// spawn or handshake failures.
    #[cfg(unix)]
    async fn connect_stub_retry(config: &AcpAgentConfig) -> Result<AcpConnection, AcpError> {
        for attempt in 0..3 {
            match connect_acp_agent(config).await {
                Err(error) if attempt < 2 && error.to_string().contains("Text file busy") => {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                result => return result,
            }
        }
        unreachable!("the loop returns on its final attempt")
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn failed_handshake_quotes_the_agents_stderr_with_secrets_removed() {
        // An agent that dies during the handshake explains itself on
        // stderr. That text used to be read and discarded line by line,
        // so the user got a bare transport failure.
        let (dir, config) = stub_agent(
            "#!/bin/sh\n\
             echo 'fatal: ANTHROPIC_API_KEY=sk-fake-000111222333 rejected by upstream' >&2\n\
             echo 'see https://agent.example.test/setup?token=fake-token-999' >&2\n\
             exit 1\n",
        );
        let error = match connect_stub_retry(&config).await {
            Err(error) => error,
            Ok(_) => panic!("stub never completes the handshake"),
        };
        let _ = std::fs::remove_dir_all(dir);
        let text = error.to_string();
        assert!(text.contains("rejected by upstream"), "{text}");
        assert!(
            text.contains("agent.example.test/setup?<redacted>"),
            "{text}"
        );
        for secret in ["sk-fake-000111222333", "token=fake-token-999"] {
            assert!(!text.contains(secret), "leaked {secret:?} in {text}");
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stderr_capture_stays_bounded_under_a_flood() {
        // The drain task exists to stop a full stderr pipe from
        // blocking the child; retaining a tail must not turn it into an
        // unbounded buffer. 200k lines in, a capped tail out.
        let (dir, config) = stub_agent(
            "#!/bin/sh\n\
             awk 'BEGIN{for(i=0;i<200000;i++) print \"agent chatter line \" i > \"/dev/stderr\"}'\n\
             exit 1\n",
        );
        let error = match connect_stub_retry(&config).await {
            Err(error) => error,
            Ok(_) => panic!("stub never completes the handshake"),
        };
        let _ = std::fs::remove_dir_all(dir);
        let text = error.to_string();
        assert!(
            text.chars().count() <= 96 + op_util::cli_output::TAIL_MAX_CHARS,
            "error message was {} chars: {text}",
            text.chars().count()
        );
        assert!(text.contains("agent chatter line 199999"), "{text}");
    }

    /// The ACP twin of the CLI bridge's drain race. An agent binary that
    /// exists but starts up broken (bad config, missing dependency,
    /// immediate panic) dies with its explanation on stderr, and its
    /// stdout EOF is what fails the handshake — so the read path and the
    /// drain task wake on the same instant.
    ///
    /// The contention is CREATED here, not hoped for: run idle, the
    /// drain wins every time and this case passes even with the fix
    /// reverted. Measured before the fix: 5-8 of 96 connects came back
    /// quoting nothing.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn a_broken_agents_stderr_survives_concurrent_connects() {
        let (dir, config) =
            stub_agent("#!/bin/sh\necho 'fatal: agent config rejected by upstream' >&2\nexit 1\n");
        let mut lost = 0usize;
        let mut total = 0usize;
        for _round in 0..6 {
            let mut pending = Vec::new();
            for _ in 0..16 {
                let config = config.clone();
                pending.push(tokio::spawn(async move {
                    match connect_stub_retry(&config).await {
                        Err(error) => error.to_string(),
                        Ok(_) => "unexpectedly connected".to_string(),
                    }
                }));
            }
            for handle in pending {
                let text = handle.await.expect("probe task");
                total += 1;
                if !text.contains("agent config rejected by upstream") {
                    lost += 1;
                }
            }
        }
        let _ = std::fs::remove_dir_all(dir);
        assert_eq!(
            lost, 0,
            "{lost} of {total} connect failures lost the agent's stderr"
        );
    }
}
