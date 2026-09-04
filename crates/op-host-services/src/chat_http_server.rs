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
//!   running server on port 4096 (verify `GET /global/health`),
//!   else spawn one and kill it when the turn ends.
//! - `apps/web/server/opencode/server.ts` — spawn lifecycle, the
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
//! - Spawn reserves a concrete loopback port and retries a bounded
//!   address-in-use race; current OpenCode no longer treats `--port=0`
//!   as an ephemeral-port request.

use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, StopReason};
use tokio::sync::mpsc;

use crate::chat_runtime::{shared_runtime, BlockingRecvIter};
use crate::chat_spawn::find_binary;

#[path = "chat_http_server_error.rs"]
mod error;
pub use error::{format_opencode_error, OpenCodeError};
#[path = "chat_http_server_completion.rs"]
mod completion;
use completion::{cleanup_session, latest_assistant_text};
#[path = "chat_http_server_probe.rs"]
mod probe;
pub use probe::parse_server_url;
#[cfg(test)]
use probe::probe_server;
#[path = "chat_http_server_startup.rs"]
mod startup;
use startup::{resolve_opencode_server, ServerResolution};

/// TS `opencode-client.ts` reuses an existing server on the default
/// port before spawning its own.
const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:4096";
/// TS `chat.ts:617` — every turn opens a fresh session of this title.
const SESSION_TITLE: &str = "OpenPencil Chat";
/// TS `STREAM_TIMEOUT_MS` (chat.ts:675) — total SSE budget per turn.
const STREAM_TIMEOUT: Duration = Duration::from_secs(180);
/// TS chat.ts:708 — settle delay between SSE subscribe and prompt.
const SSE_SETTLE: Duration = Duration::from_millis(100);
const SERVER_EXIT_GRACE: Duration = Duration::from_secs(2);

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

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, None)
    }

    fn send_cancellable(
        &self,
        request: ChatRequest,
        cancel: Arc<AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        self.send_inner(request, Some(cancel))
    }
}

impl OpenCodeProvider {
    fn send_inner(
        &self,
        request: ChatRequest,
        cancel: Option<Arc<AtomicBool>>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let binary = self.binary.clone();
        let base_override = self.base_url_override.clone();
        // Prompt text = directive + history digest + user message.
        // The system prompt rides the dedicated `noReply` injection
        // (TS parity), so it is NOT folded into the prompt string.
        let mut prompt = request.user_message.clone();
        let mut directive = String::new();
        if let Some(d) = crate::chat_attachment::thinking_directive(request.thinking) {
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
                        crate::chat_attachment::attachment_to_base64(a)
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
                let _ = op_process_io::terminate_tokio_process_tree(&mut child, SERVER_EXIT_GRACE)
                    .await;
            }
        });
        match cancel {
            Some(cancel) => Box::new(BlockingRecvIter::cooperative(rx, cancel)),
            None => Box::new(BlockingRecvIter::new(rx)),
        }
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
        .use_rustls_tls()
        .timeout(Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => return fail(tx, format!("http client: {e}")).await,
    };
    // SSE client: no total timeout (the event stream outlives any
    // fixed request budget); connect failures still surface fast.
    let sse = match reqwest::Client::builder()
        .use_rustls_tls()
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
    } else {
        match resolve_opencode_server(tx, &ops, binary, DEFAULT_SERVER_URL, spawned).await {
            Ok(ServerResolution::Ready(url)) => url,
            Ok(ServerResolution::Cancelled) => return,
            Ok(ServerResolution::IdentityFailed) => {
                return fail(
                    tx,
                    "OpenCode server started but failed its /global/health identity check.".into(),
                )
                .await;
            }
            // `fail` writes into `op-ai`'s `ChatDelta::Error(String)` sink,
            // so the typed failure renders here at the boundary.
            Err(error) => return fail(tx, error.to_string()).await,
        }
    };

    // 2. Create a session for this conversation (TS chat.ts:616-624).
    // Keep the request independently owned so receiver cancellation can stop
    // this turn even while a reused server is still deciding the response.
    let create_ops = ops.clone();
    let create_base = base.clone();
    let mut create_task =
        tokio::spawn(async move { create_session(&create_ops, &create_base).await });
    let session_id = tokio::select! {
        biased;
        _ = tx.closed() => {
            if spawned.is_some() {
                // The caller tears down this integration-owned server next,
                // which also removes any session the request may have made.
                create_task.abort();
            } else {
                // A reused server outlives the turn. Let an accepted create
                // finish independently, then remove its temporary session.
                let cleanup_ops = ops.clone();
                let cleanup_base = base.clone();
                tokio::spawn(async move {
                    if let Ok(Ok(session_id)) = create_task.await {
                        cleanup_session(&cleanup_ops, &cleanup_base, &session_id, true).await;
                    }
                });
            }
            return;
        }
        result = &mut create_task => match result {
            Ok(Ok(id)) => id,
            Ok(Err(e)) => {
                return fail(tx, format!("Failed to create OpenCode session: {e}")).await;
            }
            Err(e) => {
                return fail(tx, format!("Failed to create OpenCode session: {e}")).await;
            }
        },
    };

    // Every branch after session creation must delete this integration-owned
    // session. Failed/canceled turns abort first so work cannot continue on a
    // reused port-4096 server after the OpenPencil receiver is gone.
    macro_rules! finish_session {
        ($abort_first:expr) => {{
            cleanup_session(&ops, &base, &session_id, $abort_first).await;
            return;
        }};
    }
    if tx.is_closed() {
        finish_session!(true);
    }

    // 3. Inject the system prompt as context, no AI reply
    // (TS chat.ts:626-636 — failure logs, never aborts the turn).
    let system = system_prompt.trim();
    if !system.is_empty() {
        let body = serde_json::json!({
            "noReply": true,
            "parts": [{ "type": "text", "text": system }],
        });
        let message_url = format!("{base}/session/{session_id}/message");
        let injection = tokio::select! {
            biased;
            _ = tx.closed() => finish_session!(true),
            result = post_json(&ops, &message_url, &body) => result,
        };
        if let Err(e) = injection {
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
    tokio::select! {
        biased;
        _ = tx.closed() => {
            sse_task.abort();
            finish_session!(true);
        }
        _ = tokio::time::sleep(SSE_SETTLE) => {}
    }

    // 6. Send the prompt (TS chat.ts:659-664, 710-716).
    // This bridge returns one text completion to the orchestrator. Disabling
    // tools prevents OpenCode from stranding the response in an unhandled
    // tool call.
    let mut prompt_payload = serde_json::json!({
        "parts": parts,
        "tools": { "*": false },
    });
    if let Some((provider_id, model_id)) = &parsed_model {
        prompt_payload["model"] = serde_json::json!({
            "providerID": provider_id,
            "modelID": model_id,
        });
    }
    let prompt_url = format!("{base}/session/{session_id}/prompt_async");
    let prompt_result = tokio::select! {
        biased;
        _ = tx.closed() => {
            sse_task.abort();
            finish_session!(true);
        }
        result = post_json(&ops, &prompt_url, &prompt_payload) => result,
    };
    if let Err(e) = prompt_result {
        eprintln!("[AI] OpenCode promptAsync error: {e}");
        sse_task.abort();
        fail(tx, e.to_string()).await;
        finish_session!(true);
    }

    // 7. Consume events until idle / error / timeout
    // (TS chat.ts:718-776).
    let deadline = tokio::time::Instant::now() + STREAM_TIMEOUT;
    let mut streamed_text = String::new();
    let mut canceled = false;
    let mut timed_out = false;
    let mut terminal_error = None;
    let mut tool_escape = None;
    loop {
        tokio::select! {
            biased;
            _ = tx.closed() => { canceled = true; break }
            // TS streamWithTimeout: the 180s budget ends the stream;
            // the fallback + empty checks still run after it.
            _ = tokio::time::sleep_until(deadline) => {
                timed_out = true;
                break;
            },
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
                            streamed_text.push_str(delta);
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
                    "message.part.updated" => {
                        let part = props.and_then(|p| p.get("part"));
                        let part_session = part
                            .and_then(|p| p.get("sessionID"))
                            .and_then(|v| v.as_str());
                        if part_session == Some(session_id.as_str())
                            && part.and_then(|p| p.get("type")).and_then(|v| v.as_str())
                                == Some("tool")
                        {
                            let name = part
                                .and_then(|p| p.get("tool"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("unknown");
                            tool_escape = Some(name.to_string());
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
                    "session.error"
                        if prop_session == Some(session_id.as_str()) || prop_session.is_none() =>
                    {
                        let err = props.and_then(|p| p.get("error"));
                        let msg = format_opencode_error(err);
                        eprintln!("[AI] OpenCode session error: {msg}");
                        terminal_error = Some(msg);
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
    sse_task.abort();
    if canceled {
        finish_session!(true);
    }

    if timed_out {
        fail(
            tx,
            format!(
                "OpenCode timed out after {} seconds before the session completed.",
                STREAM_TIMEOUT.as_secs()
            ),
        )
        .await;
        finish_session!(true);
    }
    if let Some(name) = tool_escape {
        fail(
            tx,
            format!(
                "OpenCode attempted the forbidden `{name}` tool during a text-only completion."
            ),
        )
        .await;
        finish_session!(true);
    }
    if let Some(error) = terminal_error {
        fail(tx, error).await;
        finish_session!(true);
    }

    // 8. Reconcile against the persisted assistant message even when SSE
    // produced text. OpenCode can reach idle after dropping a final delta;
    // emitting only the missing suffix keeps the completion exact.
    let messages_url = format!("{base}/session/{session_id}/message");
    let final_messages = tokio::select! {
        biased;
        _ = tx.closed() => finish_session!(true),
        result = get_json(&ops, &messages_url) => result,
    };
    if let Ok(messages) = final_messages {
        if let Some(final_text) = latest_assistant_text(&messages) {
            if streamed_text.is_empty() {
                if tx
                    .send(ChatDelta::TextDelta(final_text.clone()))
                    .await
                    .is_err()
                {
                    finish_session!(true);
                }
                streamed_text = final_text;
            } else if let Some(suffix) = final_text.strip_prefix(&streamed_text) {
                if !suffix.is_empty()
                    && tx
                        .send(ChatDelta::TextDelta(suffix.to_string()))
                        .await
                        .is_err()
                {
                    finish_session!(true);
                }
                streamed_text = final_text;
            } else if final_text != streamed_text {
                fail(
                    tx,
                    "OpenCode stream did not match its persisted final response.".into(),
                )
                .await;
                finish_session!(true);
            }
        }
    }

    // 9. Still nothing → terminal empty-response failure.
    if streamed_text.is_empty() {
        fail(
            tx,
            "OpenCode returned an empty response. The model may not have generated any output."
                .into(),
        )
        .await;
        finish_session!(true);
    }

    let delivered = tx
        .send(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        })
        .await
        .is_ok();
    cleanup_session(&ops, &base, &session_id, !delivered).await;
}

/// Parse an OpenCode model slug "providerID/modelID" into its parts
/// (TS chat.ts:509-513 — split on the FIRST slash only).
pub fn parse_opencode_model(model: &str) -> Option<(String, String)> {
    let idx = model.find('/')?;
    Some((model[..idx].to_string(), model[idx + 1..].to_string()))
}

/// Extract the JSON payload from one SSE line (`data: {...}`).
pub fn parse_sse_data_line(line: &str) -> Option<serde_json::Value> {
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
) -> Result<serde_json::Value, OpenCodeError> {
    let resp = client
        .post(url)
        .json(body)
        .send()
        .await
        .map_err(|e| OpenCodeError::Request(e.to_string()))?;
    read_json_response(resp).await
}

/// GET a JSON document with the same error mapping as [`post_json`].
async fn get_json(client: &reqwest::Client, url: &str) -> Result<serde_json::Value, OpenCodeError> {
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| OpenCodeError::Request(e.to_string()))?;
    read_json_response(resp).await
}

async fn read_json_response(resp: reqwest::Response) -> Result<serde_json::Value, OpenCodeError> {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        // Error bodies are OpenCode error objects when JSON — run
        // them through the TS formatter; otherwise show raw text.
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
            return Err(OpenCodeError::Provider(format_opencode_error(Some(&val))));
        }
        return Err(OpenCodeError::HttpStatus {
            status,
            body: text.trim().to_string(),
        });
    }
    if text.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| OpenCodeError::InvalidJson {
        message: e.to_string(),
    })
}

/// `POST /session` → session id (TS `session.create`).
async fn create_session(client: &reqwest::Client, base: &str) -> Result<String, OpenCodeError> {
    let body = serde_json::json!({ "title": SESSION_TITLE });
    let val = post_json(client, &format!("{base}/session"), &body).await?;
    val.get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| OpenCodeError::Provider(format_opencode_error(Some(&val))))
}

#[cfg(test)]
#[path = "chat_http_server_tests.rs"]
mod tests;
