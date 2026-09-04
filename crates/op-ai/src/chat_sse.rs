//! Transport-free SSE payload parsers for the built-in HTTP chat providers.
//!
//! One `data:` event payload (Anthropic `/v1/messages` or an
//! OpenAI-compatible `/chat/completions` stream) maps to at most one
//! [`ChatDelta`]. These helpers moved here from
//! `op-host-services/src/chat_builtin_http_wire.rs` (which re-exports them
//! unchanged) so the mobile FFI chat pump (`op-engine-ffi`) and the desktop /
//! daemon transports parse the wire protocol from ONE implementation instead
//! of drifting copies. Byte-level transports (reqwest response pumps, retry
//! ladders) stay host-side — this module is pure `&str -> Option<ChatDelta>`.

use serde_json::Value;

use crate::chat_provider::{ChatDelta, StopReason};

/// Join a provider base URL and an API path without doubling either side.
/// A base that already ends with the full path is used as-is; an
/// Anthropic-style base ending in `/v1` absorbs the `/v1` of
/// `/v1/messages`.
pub fn provider_endpoint(base_url: &str, path: &str) -> String {
    let base = base_url.trim().trim_end_matches('/');
    if base.ends_with(path) {
        return base.to_string();
    }
    if path == "/v1/messages" && base.ends_with("/v1") {
        return format!("{base}/messages");
    }
    format!("{base}{path}")
}

/// Parse one OpenAI-compatible SSE `data:` payload.
pub fn parse_openai_sse_data(data: &str) -> Option<ChatDelta> {
    let data = data.trim();
    if data == "[DONE]" {
        return Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        });
    }
    let value: Value = serde_json::from_str(data).ok()?;
    if value.get("error").is_some() {
        // A provider-controlled HTTP-200 SSE event can reflect request headers
        // or credentials in its message. Preserve only the error boundary.
        return Some(ChatDelta::Error(
            "OpenAI-compatible provider reported a stream error".into(),
        ));
    }
    let choice = value.get("choices")?.as_array()?.first()?;
    if let Some(delta) = choice.get("delta") {
        // Some OpenAI-compatible providers put reasoning and the first
        // content token in the SAME delta. This classic parser returns one
        // event, so content must win; preferring reasoning here used to drop
        // the only script-bearing token and could leave orchestration empty.
        if let Some(content) = delta
            .get("content")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            return Some(ChatDelta::TextDelta(content.to_string()));
        }
        if let Some(reasoning) = delta
            .get("reasoning_content")
            .or_else(|| delta.get("reasoning"))
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            return Some(ChatDelta::Thinking(reasoning.to_string()));
        }
    }
    choice
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(|reason| ChatDelta::Done {
            stop_reason: map_openai_stop_reason(reason),
        })
}

/// Parse one Anthropic Messages-API SSE `data:` payload.
pub fn parse_anthropic_sse_data(data: &str) -> Option<ChatDelta> {
    let value: Value = serde_json::from_str(data.trim()).ok()?;
    match value.get("type").and_then(Value::as_str).unwrap_or("") {
        "content_block_delta" => {
            let delta = value.get("delta")?;
            match delta.get("type").and_then(Value::as_str).unwrap_or("") {
                "text_delta" => delta
                    .get("text")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| ChatDelta::TextDelta(s.to_string())),
                "thinking_delta" => delta
                    .get("thinking")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                    .map(|s| ChatDelta::Thinking(s.to_string())),
                _ => None,
            }
        }
        "message_delta" => value
            .pointer("/delta/stop_reason")
            .and_then(Value::as_str)
            .map(|reason| ChatDelta::Done {
                stop_reason: map_anthropic_stop_reason(reason),
            }),
        "message_stop" => Some(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        }),
        "error" => Some(ChatDelta::Error(
            "Anthropic provider reported a stream error".into(),
        )),
        _ => None,
    }
}

pub fn map_anthropic_stop_reason(reason: &str) -> StopReason {
    match reason {
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "aborted" | "user_abort" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

pub fn map_openai_stop_reason(reason: &str) -> StopReason {
    match reason {
        "length" => StopReason::MaxTokens,
        "tool_calls" | "function_call" => StopReason::ToolUse,
        "content_filter" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_endpoint_joins_without_doubling() {
        assert_eq!(
            provider_endpoint("https://api.deepseek.com", "/chat/completions"),
            "https://api.deepseek.com/chat/completions"
        );
        assert_eq!(
            provider_endpoint(
                "https://api.deepseek.com/chat/completions",
                "/chat/completions"
            ),
            "https://api.deepseek.com/chat/completions"
        );
        assert_eq!(
            provider_endpoint("https://api.anthropic.com/v1", "/v1/messages"),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn openai_payloads_map_to_deltas() {
        assert!(matches!(
            parse_openai_sse_data(r#"{"choices":[{"delta":{"content":"hi"}}]}"#),
            Some(ChatDelta::TextDelta(s)) if s == "hi"
        ));
        assert!(matches!(
            parse_openai_sse_data(r#"{"choices":[{"delta":{"reasoning_content":"mull"}}]}"#),
            Some(ChatDelta::Thinking(s)) if s == "mull"
        ));
        assert!(matches!(
            parse_openai_sse_data("[DONE]"),
            Some(ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            })
        ));
        assert!(matches!(
            parse_openai_sse_data(r#"{"error":{"message":"secret-echo"}}"#),
            Some(ChatDelta::Error(s)) if !s.contains("secret-echo")
        ));
    }

    #[test]
    fn anthropic_payloads_map_to_deltas() {
        assert!(matches!(
            parse_anthropic_sse_data(
                r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}"#
            ),
            Some(ChatDelta::TextDelta(s)) if s == "hi"
        ));
        assert!(matches!(
            parse_anthropic_sse_data(r#"{"type":"message_stop"}"#),
            Some(ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            })
        ));
        assert!(matches!(
            parse_anthropic_sse_data(
                r#"{"type":"message_delta","delta":{"stop_reason":"max_tokens"}}"#
            ),
            Some(ChatDelta::Done {
                stop_reason: StopReason::MaxTokens
            })
        ));
    }
}
