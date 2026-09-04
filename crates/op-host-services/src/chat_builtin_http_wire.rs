//! SSE transport + wire helpers for the built-in HTTP chat providers.
//!
//! The response pump, endpoint/base-URL normalization, and the
//! Anthropic / OpenAI SSE payload → `ChatDelta` parsers, carved off
//! `chat_builtin_http.rs` to keep both files under the 800-line cap.
//! `chat_builtin_http` re-exports this module's surface so existing
//! paths (`chat_builtin_http::map_openai_stop_reason`, …) are unchanged.
//!
//! The pure payload parsers (`parse_*_sse_data`, `map_*_stop_reason`,
//! `provider_endpoint`) moved to `op_ai::chat_sse` so the mobile FFI chat
//! pump shares them; this module re-exports them unchanged and keeps the
//! reqwest/tokio transport halves.

use futures::StreamExt;
use op_ai::chat_provider::{ChatDelta, StopReason};
use tokio::sync::mpsc;

pub use op_ai::chat_sse::{map_anthropic_stop_reason, map_openai_stop_reason};
pub(crate) use op_ai::chat_sse::{
    parse_anthropic_sse_data, parse_openai_sse_data, provider_endpoint,
};

use crate::chat_builtin_http::BuiltinHttpError;

// `apply_reasoning_wire_control` moved to `op_chat_agent::backoff` (pure
// code motion) so the shared agent loop and mobile hosts build request
// bodies through one entry point; re-exported unchanged.
pub use op_chat_agent::backoff::apply_reasoning_wire_control;

pub(crate) async fn pump_sse_response(
    resp: reqwest::Response,
    tx: &mpsc::Sender<ChatDelta>,
    parse: fn(&str) -> Option<ChatDelta>,
) -> Result<bool, BuiltinHttpError> {
    let mut stream = resp.bytes_stream();
    let mut buf = Vec::<u8>::new();
    let mut event_data = String::new();
    let mut emitted_done = false;

    while let Some(chunk) = stream.next().await {
        if tx.is_closed() {
            return Ok(true);
        }
        let bytes = chunk.map_err(|e| BuiltinHttpError::SseStream {
            message: e.to_string(),
        })?;
        buf.extend_from_slice(&bytes);
        while let Some(nl_pos) = buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = buf.drain(..=nl_pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            if line.is_empty() {
                if emit_sse_event(&mut event_data, tx, parse).await {
                    emitted_done = true;
                    break;
                }
                continue;
            }
            if let Some(data) = line.strip_prefix("data:") {
                if !event_data.is_empty() {
                    event_data.push('\n');
                }
                event_data.push_str(data.trim_start());
            }
        }
        if emitted_done {
            break;
        }
    }

    if !emitted_done && !buf.is_empty() {
        let line = String::from_utf8_lossy(&buf);
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        if let Some(data) = line.strip_prefix("data:") {
            if !event_data.is_empty() {
                event_data.push('\n');
            }
            event_data.push_str(data.trim_start());
        }
    }
    if !emitted_done && emit_sse_event(&mut event_data, tx, parse).await {
        emitted_done = true;
    }

    Ok(emitted_done)
}

async fn emit_sse_event(
    event_data: &mut String,
    tx: &mpsc::Sender<ChatDelta>,
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
    if tx.send(delta).await.is_err() {
        event_data.clear();
        return true;
    }
    if emitted_error {
        let _ = tx
            .send(ChatDelta::Done {
                stop_reason: StopReason::Aborted,
            })
            .await;
        event_data.clear();
        return true;
    }
    event_data.clear();
    emitted_done
}

pub(crate) fn normalize_provider_base_url(base_url: &str) -> Result<String, BuiltinHttpError> {
    let url =
        reqwest::Url::parse(base_url).map_err(|error| BuiltinHttpError::InvalidEndpointUrl {
            message: error.to_string(),
        })?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(BuiltinHttpError::EndpointUnsupportedScheme {
            scheme: url.scheme().to_string(),
        });
    }
    if url.host_str().is_none() {
        return Err(BuiltinHttpError::EndpointMissingHost);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(BuiltinHttpError::EndpointHasQueryOrFragment);
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}
