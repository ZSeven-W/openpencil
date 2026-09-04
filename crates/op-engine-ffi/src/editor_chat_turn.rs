//! One streaming HTTP chat turn against a built-in (API-key) provider.
//!
//! The mobile twin of `op-host-services`' plain streaming path
//! (`chat_builtin_http::run_openai_chat` / `run_anthropic_chat`): POST the
//! provider's streaming endpoint and convert SSE payloads into
//! [`ChatDelta`]s through the shared parsers in [`op_ai::chat_sse`]. The
//! deliberate deltas from desktop: no canvas-tool agent loop, no skill
//! preamble, no retry/backoff ladder — a failed request surfaces one honest
//! error in the transcript instead. TLS is rustls with the bundled
//! webpki roots: mobile OSes expose no `/etc/ssl/certs`, so a
//! native-cert root store would be empty and every HTTPS dial would fail
//! (the exact failure mode fixed for collaboration in
//! `op-collab-relay-client`).

use std::fmt;
use std::sync::mpsc::Sender;
use std::time::Duration;

use futures::StreamExt;
use op_ai::chat_provider::{ChatDelta, ChatHistoryRole, StopReason};
use op_ai::chat_sse::{parse_anthropic_sse_data, parse_openai_sse_data, provider_endpoint};
use op_editor_core::BuiltinAgentKind;
use serde_json::json;

/// Same deadlines as the desktop provider client: a hung endpoint must
/// surface as an error, not an eternal "Thinking…" bubble.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
/// A provider may omit newlines forever or split one event across many
/// `data:` lines. Bound both buffers at the same hard ceiling as the codegen
/// response collector so malformed SSE cannot grow mobile memory without
/// limit before the outer pipeline sees a delta.
const MAX_SSE_EVENT_BYTES: usize = 2 * 1024 * 1024;

/// Everything one worker task needs to run a turn. A value snapshot — the
/// worker never touches the live editor state.
pub(crate) struct BuiltinChatTurn {
    pub kind: BuiltinAgentKind,
    pub api_key: String,
    pub model: String,
    /// Trimmed base URL (provider default when the config left it empty).
    pub base_url: String,
    pub system_prompt: String,
    pub history: Vec<(ChatHistoryRole, String)>,
    pub prompt: String,
    pub max_output_tokens: u32,
}

/// Typed failures for the mobile chat transport. Wording mirrors
/// `op_host_services::chat_builtin_http::BuiltinHttpError` so the same
/// failure reads the same in a transcript on every host.
#[derive(Debug)]
pub(crate) enum MobileChatTurnError {
    ClientBuild {
        message: String,
    },
    HttpStatus {
        label: &'static str,
        status: u16,
    },
    Timeout {
        label: &'static str,
        message: String,
    },
    Transport {
        label: &'static str,
        message: String,
    },
    SseStream {
        message: String,
    },
}

impl fmt::Display for MobileChatTurnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MobileChatTurnError::ClientBuild { message } => {
                write!(f, "Provider HTTP client is unavailable: {message}")
            }
            MobileChatTurnError::HttpStatus { label, status } => {
                write!(f, "{label} http {status}")
            }
            MobileChatTurnError::Timeout { label, message } => {
                write!(f, "{label} request timed out: {message}")
            }
            MobileChatTurnError::Transport { label, message } => {
                write!(f, "{label} request failed: {message}")
            }
            MobileChatTurnError::SseStream { message } => write!(f, "sse stream: {message}"),
        }
    }
}

impl std::error::Error for MobileChatTurnError {}

/// Run one turn to completion, streaming deltas into `tx`. Every exit path
/// ends with a terminal [`ChatDelta::Done`] so the pump always retires the
/// bubble — a swallowed error is exactly the "stuck at Thinking…" bug this
/// module exists to prevent.
pub(crate) async fn run_builtin_turn(turn: BuiltinChatTurn, tx: Sender<ChatDelta>) {
    let emitted_done = match run_streaming_request(&turn, &tx).await {
        Ok(done) => done,
        Err(error) => {
            let _ = tx.send(ChatDelta::Error(error.to_string()));
            let _ = tx.send(ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            });
            return;
        }
    };
    if !emitted_done {
        let _ = tx.send(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        });
    }
}

/// Provider HTTP client with the shared mobile timeouts. Used by the plain
/// chat turn and the design loop so both dial with identical posture.
pub(crate) fn build_mobile_chat_client() -> Result<reqwest::Client, MobileChatTurnError> {
    reqwest::Client::builder()
        .use_rustls_tls()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| MobileChatTurnError::ClientBuild {
            message: error.to_string(),
        })
}

async fn run_streaming_request(
    turn: &BuiltinChatTurn,
    tx: &Sender<ChatDelta>,
) -> Result<bool, MobileChatTurnError> {
    let client = build_mobile_chat_client()?;
    let (label, request, parse): (_, _, fn(&str) -> Option<ChatDelta>) = match turn.kind {
        BuiltinAgentKind::OpenAiCompat => {
            let url = provider_endpoint(&turn.base_url, "/chat/completions");
            let mut messages = Vec::new();
            if !turn.system_prompt.trim().is_empty() {
                messages.push(json!({ "role": "system", "content": turn.system_prompt }));
            }
            for (role, text) in &turn.history {
                messages.push(json!({ "role": role.as_str(), "content": text }));
            }
            messages.push(json!({ "role": "user", "content": turn.prompt }));
            let body = json!({
                "model": turn.model,
                "stream": true,
                "max_tokens": turn.max_output_tokens,
                "messages": messages,
            });
            (
                "openai-compatible",
                client.post(&url).bearer_auth(&turn.api_key).json(&body),
                parse_openai_sse_data,
            )
        }
        BuiltinAgentKind::Anthropic => {
            let url = provider_endpoint(&turn.base_url, "/v1/messages");
            let mut messages: Vec<serde_json::Value> = turn
                .history
                .iter()
                .map(|(role, text)| json!({ "role": role.as_str(), "content": text }))
                .collect();
            messages.push(json!({ "role": "user", "content": turn.prompt }));
            let mut body = json!({
                "model": turn.model,
                "max_tokens": turn.max_output_tokens,
                "stream": true,
                "messages": messages,
            });
            if !turn.system_prompt.trim().is_empty() {
                body.as_object_mut()
                    .expect("anthropic request body is object")
                    .insert("system".into(), json!(turn.system_prompt));
            }
            (
                "anthropic",
                client
                    .post(&url)
                    .header("x-api-key", &turn.api_key)
                    .header("anthropic-version", "2023-06-01")
                    .json(&body),
                parse_anthropic_sse_data,
            )
        }
    };
    let response = request.send().await.map_err(|error| {
        if error.is_timeout() {
            MobileChatTurnError::Timeout {
                label,
                message: error.to_string(),
            }
        } else {
            MobileChatTurnError::Transport {
                label,
                message: error.to_string(),
            }
        }
    })?;
    if !response.status().is_success() {
        // Provider error bodies are untrusted (they can echo request
        // headers); report only the status, same as desktop.
        return Err(MobileChatTurnError::HttpStatus {
            label,
            status: response.status().as_u16(),
        });
    }
    pump_sse_response(response, tx, parse).await
}

/// Split the SSE byte stream into `data:` events and forward each parsed
/// delta. Synchronous-`Sender` twin of
/// `op_host_services::chat_builtin_http_wire::pump_sse_response` — keep the
/// two line-splitting loops in step when touching either.
pub(crate) async fn pump_sse_response(
    response: reqwest::Response,
    tx: &Sender<ChatDelta>,
    parse: fn(&str) -> Option<ChatDelta>,
) -> Result<bool, MobileChatTurnError> {
    let mut stream = response.bytes_stream();
    let mut buf = Vec::<u8>::new();
    let mut event_data = String::new();
    let mut emitted_done = false;

    while let Some(chunk) = stream.next().await {
        let bytes = chunk.map_err(|error| MobileChatTurnError::SseStream {
            message: error.to_string(),
        })?;
        buf.extend_from_slice(&bytes);
        while let Some(nl_pos) = buf.iter().position(|&b| b == b'\n') {
            ensure_sse_size(nl_pos)?;
            let line: Vec<u8> = buf.drain(..=nl_pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                if emit_sse_event(&mut event_data, tx, parse) {
                    emitted_done = true;
                    break;
                }
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                append_sse_data(&mut event_data, data.trim_start())?;
            }
        }
        // Only the unterminated suffix remains. Complete short lines above
        // do not count against each other; one never-terminated line does.
        ensure_sse_size(buf.len())?;
        if emitted_done {
            break;
        }
    }

    if !emitted_done && !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(data) = line.strip_prefix("data:") {
            append_sse_data(&mut event_data, data.trim_start())?;
        }
    }
    if !emitted_done && emit_sse_event(&mut event_data, tx, parse) {
        emitted_done = true;
    }

    Ok(emitted_done)
}

fn append_sse_data(event_data: &mut String, data: &str) -> Result<(), MobileChatTurnError> {
    let separator = usize::from(!event_data.is_empty());
    let next_len = event_data
        .len()
        .checked_add(separator)
        .and_then(|length| length.checked_add(data.len()))
        .ok_or_else(sse_size_error)?;
    ensure_sse_size(next_len)?;
    if separator != 0 {
        event_data.push('\n');
    }
    event_data.push_str(data);
    Ok(())
}

fn ensure_sse_size(size: usize) -> Result<(), MobileChatTurnError> {
    if size <= MAX_SSE_EVENT_BYTES {
        Ok(())
    } else {
        Err(sse_size_error())
    }
}

fn sse_size_error() -> MobileChatTurnError {
    MobileChatTurnError::SseStream {
        message: format!("event exceeded the {MAX_SSE_EVENT_BYTES}-byte safety limit"),
    }
}

/// Forward one buffered event. Returns true when the turn is over (a
/// terminal delta was sent or the pump's receiver is gone).
fn emit_sse_event(
    event_data: &mut String,
    tx: &Sender<ChatDelta>,
    parse: fn(&str) -> Option<ChatDelta>,
) -> bool {
    let data = event_data.trim();
    if data.is_empty() {
        event_data.clear();
        return false;
    }
    let Some(delta) = parse(data) else {
        event_data.clear();
        return false;
    };
    let emitted_done = matches!(delta, ChatDelta::Done { .. });
    let emitted_error = matches!(delta, ChatDelta::Error(_));
    if tx.send(delta).is_err() {
        event_data.clear();
        return true;
    }
    if emitted_error {
        let _ = tx.send(ChatDelta::Done {
            stop_reason: StopReason::Aborted,
        });
        event_data.clear();
        return true;
    }
    event_data.clear();
    emitted_done
}

#[cfg(test)]
#[path = "editor_chat_turn_tests.rs"]
mod tests;
