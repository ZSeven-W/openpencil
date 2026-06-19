//! OpenCode chat transport — drives the `opencode` CLI through its
//! local HTTP server + SSE event stream. Verbatim port of the TS
//! reference path:
//!
//! - `apps/web/server/api/ai/chat.ts::streamViaOpenCode` (598-830) —
//!   session create, system-prompt injection via `noReply`, SSE
//!   subscribe-before-prompt, `message.part.delta` text / reasoning
//!   deltas, `session.idle` / `session.error` terminators, the
//!   `session/{id}/message` empty-response fallback, and
//!   `formatOpenCodeError`'s label + nested-JSON mapping (454-506).
//! - `apps/web/server/utils/opencode-client.ts` — reuse an already
//!   running server on port 4096 (probe `GET /config/providers`),
//!   else spawn one and kill it when the turn ends.
//! - `apps/web/server/opencode/server.ts` — spawn argv
//!   (`serve --hostname=127.0.0.1 --port=0`), the
//!   `OPENCODE_CONFIG_CONTENT` env, and the
//!   `opencode server listening on <url>` stdout handshake.
//!
//! Wire shapes were verified against a live `opencode` 1.15.0 server
//! (2026-06-11): listening line, `POST /session`,
//! `POST /session/{id}/message` (noReply), `GET /event` SSE envelope
//! (`data: {"type":…,"properties":…}`), `session.idle` /
//! `session.error` payloads, and the messages-fallback shape.
//!
//! Documented divergences from TS:
//! - The thinking / effort chips ride an in-band directive line and
//!   prior turns ride the history digest (both established
//!   beyond-TS-baseline behaviors shared with `chat_subprocess.rs`;
//!   TS sends neither — its OpenCode turn is system + last message).
//! - JSON control requests carry a 120s client timeout (TS disables
//!   fetch timeouts entirely; a hung local server would wedge the
//!   turn forever with nothing but Stop to recover).
//! - An empty system prompt skips the `noReply` injection instead of
//!   posting an empty text part.

use std::sync::Arc;
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};
use op_web_daemon::chat_spawn::{build_command, find_binary};

/// TS `opencode-client.ts` reuses an existing server on the default
/// port before spawning its own.
const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:4096";
/// TS `chat.ts:617` — every turn opens a fresh session of this title.
const SESSION_TITLE: &str = "OpenPencil Chat";
/// TS `STREAM_TIMEOUT_MS` (chat.ts:675) — total SSE budget per turn.
const STREAM_TIMEOUT: Duration = Duration::from_secs(180);
/// TS chat.ts:708 — settle delay between SSE subscribe and prompt.
const SSE_SETTLE: Duration = Duration::from_millis(100);
/// Probe timeout for the existing-server check. TS runs the probe
/// with fetch timeouts disabled (connection-refused still fails
/// fast); the explicit cap here only guards a wedged listener.
const PROBE_TIMEOUT: Duration = Duration::from_secs(3);

/// TS `server.ts` listen timeout: 5s, 15s on Windows
/// (`opencode-client.ts:92`).
fn listen_timeout() -> Duration {
    if cfg!(windows) {
        Duration::from_secs(15)
    } else {
        Duration::from_secs(5)
    }
}

/// `ChatProvider` impl backed by the OpenCode CLI's HTTP server.
pub struct OpenCodeProvider {
    binary: String,
    /// Pre-resolved server base URL — skips both the port-4096 probe
    /// and the spawn path. Used by tests (scripted mock server) and
    /// available for a future settings override.
    base_url_override: Option<String>,
    label: String,
}

impl OpenCodeProvider {
    /// Standard construction: resolve the `opencode` binary now;
    /// attach-or-spawn the server per send.
    pub fn new() -> Self {
        Self {
            binary: find_binary("opencode"),
            base_url_override: None,
            label: "OpenCode".into(),
        }
    }

    /// Bind the provider to an explicit server base URL (no probe, no
    /// spawn). Tests drive a scripted mock server through this.
    #[allow(dead_code)]
    pub fn with_base_url(url: impl Into<String>) -> Self {
        Self {
            binary: "opencode".into(),
            base_url_override: Some(url.into()),
            label: "OpenCode".into(),
        }
    }
}

impl Default for OpenCodeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatProvider for OpenCodeProvider {
    fn provider_label(&self) -> &str {
        &self.label
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let binary = self.binary.clone();
        let base_override = self.base_url_override.clone();
        // Prompt text = directive + history digest + user message.
        // The system prompt rides the dedicated `noReply` injection
        // (TS parity), so it is NOT folded into the prompt string.
        let mut prompt = request.user_message.clone();
        let mut directive = String::new();
        if let Some(d) = op_web_daemon::chat_attachment::thinking_directive(request.thinking) {
            directive.push_str(d);
        }
        if request.effort != EffortLevel::Low {
            if !directive.is_empty() {
                directive.push(' ');
            }
            directive.push_str(&format!(
                "Apply {} reasoning effort.",
                request.effort.as_str()
            ));
        }
        if !directive.is_empty() {
            prompt = format!("{directive}\n\n{prompt}");
        }
        let digest = op_ai::chat_history::history_digest(
            &request.history,
            op_ai::chat_history::DEFAULT_DIGEST_CHARS,
        );
        if !digest.is_empty() {
            prompt = format!("{digest}\n\n{prompt}");
        }
        // Parts array: image attachments as data URLs ahead of the
        // text part (TS chat.ts:650-657 — incl. the `type:"image"`
        // shape and the "Analyze these images." empty-prompt
        // fallback).
        let mut parts: Vec<serde_json::Value> = request
            .attachments
            .iter()
            .map(|a| {
                serde_json::json!({
                    "type": "image",
                    "url": format!(
                        "data:{};base64,{}",
                        a.media_type,
                        op_web_daemon::chat_attachment::attachment_to_base64(a)
                    ),
                })
            })
            .collect();
        let text = if prompt.is_empty() {
            "Analyze these images.".to_string()
        } else {
            prompt
        };
        parts.push(serde_json::json!({ "type": "text", "text": text }));
        // Model override: "providerID/modelID" slugs split on the
        // first slash; unparseable selections warn + send without an
        // override (TS chat.ts:642-647).
        let model = request.model_id().map(str::to_string);
        let parsed_model = model.as_deref().and_then(parse_opencode_model);
        if let (Some(m), None) = (&model, &parsed_model) {
            eprintln!(
                "[AI] OpenCode: could not parse model string \"{m}\", sending without model override"
            );
        }
        let system_prompt = request.system_prompt.clone();

        let (tx, rx) = mpsc::channel::<ChatDelta>(64);
        shared_runtime().spawn(async move {
            let mut spawned: Option<tokio::process::Child> = None;
            run_opencode_turn(
                &tx,
                &binary,
                base_override,
                &system_prompt,
                parts,
                parsed_model,
                &mut spawned,
            )
            .await;
            // TS finally: releaseOpencodeServer — only servers we
            // spawned for this turn are killed; a pre-existing 4096
            // server is left alone.
            if let Some(mut child) = spawned {
                let _ = child.start_kill();
                let _ = child.wait().await;
            }
        });
        Box::new(BlockingRecvIter::new(rx))
    }
}

/// Send `Error + Done{Aborted}` — the terminal failure shape.
async fn fail(tx: &mpsc::Sender<ChatDelta>, msg: String) {
    let _ = tx.send(ChatDelta::Error(msg)).await;
    let _ = tx
        .send(ChatDelta::Done {
            stop_reason: StopReason::Aborted,
        })
        .await;
}

/// One full OpenCode turn. `spawned` receives the server child when
/// this turn had to start one, so the caller can kill it in every
/// exit path.
async fn run_opencode_turn(
    tx: &mpsc::Sender<ChatDelta>,
    binary: &str,
    base_override: Option<String>,
    system_prompt: &str,
    parts: Vec<serde_json::Value>,
    parsed_model: Option<(String, String)>,
    spawned: &mut Option<tokio::process::Child>,
) {
    // Control-plane client. TS disables fetch timeouts; the 120s cap
    // is a documented divergence so a wedged server can't hang the
    // turn forever.
    let ops = match reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => return fail(tx, format!("http client: {e}")).await,
    };
    // SSE client: no total timeout (the event stream outlives any
    // fixed request budget); connect failures still surface fast.
    let sse = match reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => return fail(tx, format!("http client: {e}")).await,
    };

    // 1. Resolve a server: explicit override → as-is; else probe the
    // default port; else spawn `opencode serve` (TS getOpencodeClient).
    let base = if let Some(base) = base_override {
        base
    } else if probe_server(&ops, DEFAULT_SERVER_URL).await {
        DEFAULT_SERVER_URL.to_string()
    } else {
        match spawn_opencode_server(binary).await {
            Ok((url, child)) => {
                *spawned = Some(child);
                url
            }
            Err(e) => return fail(tx, e).await,
        }
    };

    // 2. Create a session for this conversation (TS chat.ts:616-624).
    let session_id = match create_session(&ops, &base).await {
        Ok(id) => id,
        Err(e) => return fail(tx, format!("Failed to create OpenCode session: {e}")).await,
    };

    // 3. Inject the system prompt as context, no AI reply
    // (TS chat.ts:626-636 — failure logs, never aborts the turn).
    let system = system_prompt.trim();
    if !system.is_empty() {
        let body = serde_json::json!({
            "noReply": true,
            "parts": [{ "type": "text", "text": system }],
        });
        if let Err(e) =
            post_json(&ops, &format!("{base}/session/{session_id}/message"), &body).await
        {
            eprintln!("[AI] OpenCode system prompt injection failed: {e}");
        }
    }

    // 4. Subscribe to the event stream BEFORE sending the prompt —
    // events emitted before the SSE connection exists are lost
    // (TS chat.ts:666-705).
    let (events_tx, mut events_rx) = mpsc::channel::<serde_json::Value>(256);
    let sse_task = {
        let url = format!("{base}/event");
        tokio::spawn(async move {
            let resp = match sse.get(&url).send().await {
                Ok(r) => r,
                Err(_) => return,
            };
            let mut stream = resp.bytes_stream();
            let mut buf: Vec<u8> = Vec::new();
            use futures::StreamExt;
            while let Some(chunk) = stream.next().await {
                let Ok(bytes) = chunk else { break };
                buf.extend_from_slice(&bytes);
                while let Some(nl) = buf.iter().position(|&b| b == b'\n') {
                    let line: Vec<u8> = buf.drain(..=nl).collect();
                    let line = String::from_utf8_lossy(&line);
                    if let Some(val) = parse_sse_data_line(line.trim_end()) {
                        if events_tx.send(val).await.is_err() {
                            return; // turn finished — stop reading
                        }
                    }
                }
            }
        })
    };

    // 5. Give the SSE connection a moment to establish (TS 100ms).
    tokio::time::sleep(SSE_SETTLE).await;

    // 6. Send the prompt (TS chat.ts:659-664, 710-716).
    let mut prompt_payload = serde_json::json!({ "parts": parts });
    if let Some((provider_id, model_id)) = &parsed_model {
        prompt_payload["model"] = serde_json::json!({
            "providerID": provider_id,
            "modelID": model_id,
        });
    }
    if let Err(e) = post_json(
        &ops,
        &format!("{base}/session/{session_id}/prompt_async"),
        &prompt_payload,
    )
    .await
    {
        eprintln!("[AI] OpenCode promptAsync error: {e}");
        sse_task.abort();
        return fail(tx, e).await;
    }

    // 7. Consume events until idle / error / timeout
    // (TS chat.ts:718-776).
    let deadline = tokio::time::Instant::now() + STREAM_TIMEOUT;
    let mut emitted_text = false;
    let mut canceled = false;
    loop {
        tokio::select! {
            biased;
            _ = tx.closed() => { canceled = true; break }
            // TS streamWithTimeout: the 180s budget ends the stream;
            // the fallback + empty checks still run after it.
            _ = tokio::time::sleep_until(deadline) => break,
            ev = events_rx.recv() => {
                let Some(val) = ev else { break }; // SSE stream ended
                let Some(ty) = val.get("type").and_then(|v| v.as_str()) else { continue };
                let props = val.get("properties");
                let prop_session = props
                    .and_then(|p| p.get("sessionID"))
                    .and_then(|v| v.as_str());
                match ty {
                    "message.part.delta" => {
                        let field = props.and_then(|p| p.get("field")).and_then(|v| v.as_str());
                        let delta = props
                            .and_then(|p| p.get("delta"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if prop_session == Some(session_id.as_str()) && field == Some("text") {
                            if tx.send(ChatDelta::TextDelta(delta.to_string())).await.is_err() {
                                canceled = true;
                                break;
                            }
                            emitted_text = true;
                        }
                        // Forward reasoning deltas as thinking chunks.
                        if prop_session == Some(session_id.as_str())
                            && field == Some("reasoning")
                            && tx.send(ChatDelta::Thinking(delta.to_string())).await.is_err()
                        {
                            canceled = true;
                            break;
                        }
                    }
                    // Session went idle — response complete.
                    "session.idle" => {
                        if prop_session == Some(session_id.as_str()) {
                            break;
                        }
                    }
                    // Session error: ours, or one with no session id.
                    "session.error" => {
                        if prop_session == Some(session_id.as_str()) || prop_session.is_none() {
                            let err = props.and_then(|p| p.get("error"));
                            let msg = format_opencode_error(err);
                            eprintln!("[AI] OpenCode session error: {msg}");
                            if tx.send(ChatDelta::Error(msg)).await.is_err() {
                                canceled = true;
                            }
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    sse_task.abort();
    if canceled {
        return;
    }

    // 8. Fallback: no streamed text → read the session messages
    // directly (TS chat.ts:778-802; failures fall through to the
    // empty-response error).
    if !emitted_text {
        if let Ok(messages) = get_json(&ops, &format!("{base}/session/{session_id}/message")).await
        {
            if let Some(items) = messages.as_array() {
                let assistant = items.iter().rev().find(|m| {
                    m.get("info")
                        .and_then(|i| i.get("role"))
                        .and_then(|v| v.as_str())
                        == Some("assistant")
                });
                if let Some(parts) = assistant
                    .and_then(|m| m.get("parts"))
                    .and_then(|p| p.as_array())
                {
                    for part in parts {
                        if part.get("type").and_then(|v| v.as_str()) == Some("text") {
                            if let Some(text) = part
                                .get("text")
                                .and_then(|v| v.as_str())
                                .filter(|t| !t.is_empty())
                            {
                                if tx
                                    .send(ChatDelta::TextDelta(text.to_string()))
                                    .await
                                    .is_err()
                                {
                                    return;
                                }
                                emitted_text = true;
                            }
                        }
                    }
                }
            }
        }
    }

    // 9. Still nothing → the TS empty-response error (chat.ts:804-811).
    if !emitted_text {
        let _ = tx
            .send(ChatDelta::Error(
                "OpenCode returned an empty response. The model may not have generated any output."
                    .into(),
            ))
            .await;
    }

    let _ = tx
        .send(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        })
        .await;
}

/// Probe an already-running server (TS `client.config.providers()`).
async fn probe_server(client: &reqwest::Client, base: &str) -> bool {
    match tokio::time::timeout(
        PROBE_TIMEOUT,
        client.get(format!("{base}/config/providers")).send(),
    )
    .await
    {
        Ok(Ok(resp)) => resp.status().is_success(),
        _ => false,
    }
}

/// Spawn `opencode serve --hostname=127.0.0.1 --port=0` and wait for
/// the `opencode server listening on <url>` stdout line (TS
/// `server.ts::createOpencodeServer`). Returns the announced base URL
/// plus the child handle (caller kills it when the turn ends).
async fn spawn_opencode_server(binary: &str) -> Result<(String, tokio::process::Child), String> {
    let args: Vec<String> = vec![
        "serve".into(),
        "--hostname=127.0.0.1".into(),
        "--port=0".into(),
    ];
    let mut cmd = build_command(binary, &args);
    // TS spawns with the full parent env plus the inline config
    // override (server.ts:40-43).
    cmd.env("OPENCODE_CONFIG_CONTENT", "{}");
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn {binary} serve: {e}"))?;

    // Collect stderr into the diagnostic buffer the TS error message
    // includes ("Server output: …").
    let output_buf: Arc<std::sync::Mutex<String>> = Arc::default();
    if let Some(stderr) = child.stderr.take() {
        let buf = Arc::clone(&output_buf);
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(mut b) = buf.lock() {
                    if b.len() < 16 * 1024 {
                        b.push_str(&line);
                        b.push('\n');
                    }
                }
            }
        });
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "opencode serve: no stdout".to_string())?;

    let timeout = listen_timeout();
    let buf_for_scan = Arc::clone(&output_buf);
    let scan = async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(url) = parse_server_url(&line) {
                // Keep draining stdout for the server's lifetime so
                // it can't block on a full pipe.
                tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
                return Some(url);
            }
            if let Ok(mut b) = buf_for_scan.lock() {
                if b.len() < 16 * 1024 {
                    b.push_str(&line);
                    b.push('\n');
                }
            }
        }
        None
    };
    match tokio::time::timeout(timeout, scan).await {
        Ok(Some(url)) => Ok((url, child)),
        Ok(None) => {
            // stdout ended — the server exited before announcing.
            let _ = child.start_kill();
            let status = child.wait().await.ok();
            let code = status
                .and_then(|s| s.code())
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".into());
            let output = output_buf.lock().map(|b| b.clone()).unwrap_or_default();
            let mut msg = format!("Server exited with code {code}");
            if !output.trim().is_empty() {
                msg.push_str(&format!("\nServer output: {output}"));
            }
            Err(msg)
        }
        Err(_) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            Err(format!(
                "Timeout waiting for server to start after {}ms",
                timeout.as_millis()
            ))
        }
    }
}

/// Parse the `opencode server listening on <url>` stdout line
/// (TS server.ts:55-62: `startsWith('opencode server listening')` +
/// `/on\s+(https?:\/\/[^\s]+)/`).
pub(crate) fn parse_server_url(line: &str) -> Option<String> {
    if !line.starts_with("opencode server listening") {
        return None;
    }
    line.split_whitespace()
        .find(|tok| tok.starts_with("http://") || tok.starts_with("https://"))
        .map(str::to_string)
}

/// Parse an OpenCode model slug "providerID/modelID" into its parts
/// (TS chat.ts:509-513 — split on the FIRST slash only).
pub(crate) fn parse_opencode_model(model: &str) -> Option<(String, String)> {
    let idx = model.find('/')?;
    Some((model[..idx].to_string(), model[idx + 1..].to_string()))
}

/// Extract the JSON payload from one SSE line (`data: {...}`).
pub(crate) fn parse_sse_data_line(line: &str) -> Option<serde_json::Value> {
    let rest = line.strip_prefix("data:")?.trim_start();
    if rest.is_empty() {
        return None;
    }
    serde_json::from_str(rest).ok()
}

/// POST a JSON body; non-2xx and transport failures map to the
/// formatted error string the TS path would surface.
async fn post_json(
    client: &reqwest::Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let resp = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    read_json_response(resp).await
}

/// GET a JSON document with the same error mapping as [`post_json`].
async fn get_json(client: &reqwest::Client, url: &str) -> Result<serde_json::Value, String> {
    let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
    read_json_response(resp).await
}

async fn read_json_response(resp: reqwest::Response) -> Result<serde_json::Value, String> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        // Error bodies are OpenCode error objects when JSON — run
        // them through the TS formatter; otherwise show raw text.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
            return Err(format_opencode_error(Some(&val)));
        }
        return Err(format!("http {status}: {}", text.trim()));
    }
    if text.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| format!("invalid JSON response: {e}"))
}

/// `POST /session` → session id (TS `session.create`).
async fn create_session(client: &reqwest::Client, base: &str) -> Result<String, String> {
    let body = serde_json::json!({ "title": SESSION_TITLE });
    let val = post_json(client, &format!("{base}/session"), &body).await?;
    val.get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| format_opencode_error(Some(&val)))
}

/// Error name → user-friendly label mapping
/// (TS `OPENCODE_ERROR_LABELS`).
fn opencode_error_label(name: &str) -> &str {
    match name {
        "APIError" => "API error",
        "ProviderAuthError" => "Authentication failed",
        "UnknownError" => "Unknown error",
        "MessageOutputLengthError" => "Response too long",
        "MessageAbortedError" => "Request aborted",
        "StructuredOutputError" => "Output format error",
        "ContextOverflowError" => "Context too long",
        other => other,
    }
}

/// Extract a human-readable message from an OpenCode error object —
/// verbatim port of TS `formatOpenCodeError` (chat.ts:470-506):
/// structured `{ name, data: { message } }` errors get a label plus
/// nested-JSON message extraction; plain `{ message }` passes
/// through; everything else falls back to truncated JSON.
pub(crate) fn format_opencode_error(error: Option<&serde_json::Value>) -> String {
    let Some(error) = error else {
        return "Unknown error".into();
    };
    if error.is_null() {
        return "Unknown error".into();
    }
    if let Some(s) = error.as_str() {
        return s.to_string();
    }

    let name = error.get("name").and_then(|v| v.as_str());
    let data_message = error
        .get("data")
        .and_then(|d| d.get("message"))
        .and_then(|v| v.as_str());
    if let (Some(name), Some(message)) = (name, data_message) {
        let label = opencode_error_label(name);
        let mut msg = message.to_string();
        // Try to extract a nested error message from JSON embedded in
        // the message string, e.g.
        // 'Unauthorized: {"error":{"code":"invalid_api_key","message":"invalid access token"}}'.
        // TS only unwraps when the JSON starts past index 0.
        if let Some(json_start) = msg.find('{').filter(|&i| i > 0) {
            if let Ok(nested) = serde_json::from_str::<serde_json::Value>(&msg[json_start..]) {
                let nested_msg = nested
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|v| v.as_str())
                    .or_else(|| nested.get("message").and_then(|v| v.as_str()));
                if let Some(nested_msg) = nested_msg {
                    let prefix = msg[..json_start]
                        .trim_end()
                        .trim_end_matches(':')
                        .trim()
                        .to_string();
                    msg = if prefix.is_empty() {
                        nested_msg.to_string()
                    } else {
                        format!("{prefix}: {nested_msg}")
                    };
                }
            }
        }
        return format!("{label} — {msg}");
    }

    // Plain { message } object.
    if let Some(message) = error.get("message").and_then(|v| v.as_str()) {
        return message.to_string();
    }

    // Fallback: truncated JSON.
    let json = error.to_string();
    if json.chars().count() > 200 {
        let truncated: String = json.chars().take(200).collect();
        format!("{truncated}…")
    } else {
        json
    }
}

#[cfg(test)]
#[path = "chat_http_server_tests.rs"]
mod tests;
