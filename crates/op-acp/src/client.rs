//! ACP connection — connect to a local (stdio) or remote (WebSocket)
//! agent and drive the initialize / session / prompt handshake.
//! Port of `pen-acp/src/client.ts`.

use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::jsonrpc::{dispatch_inbound, JsonRpcEngine};
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

/// A live ACP connection to one agent.
pub struct AcpConnection {
    engine: JsonRpcEngine,
    notifications: Option<mpsc::UnboundedReceiver<SessionNotification>>,
    child: Option<Child>,
    tasks: Vec<JoinHandle<()>>,
    agent_info: AcpAgentInfo,
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
        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Value>();
        let (notif_tx, notif_rx) = mpsc::unbounded_channel::<SessionNotification>();
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
            let waiters: Vec<_> = pending.lock().unwrap().drain().collect();
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
        }
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
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string());
        let params = serde_json::json!({ "cwd": cwd, "mcpServers": [] });
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
    pub fn take_notifications(&mut self) -> Option<mpsc::UnboundedReceiver<SessionNotification>> {
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
    // Drain stderr so the child never blocks on a full pipe.
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(_)) = lines.next_line().await {}
        });
    }

    let mut conn = AcpConnection::new(stdout, stdin, Some(child));
    conn.initialize(&config.display_name).await?;
    Ok(conn)
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

    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Value>();
    let (notif_tx, notif_rx) = mpsc::unbounded_channel::<SessionNotification>();
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
        let waiters: Vec<_> = pending.lock().unwrap().drain().collect();
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
}
