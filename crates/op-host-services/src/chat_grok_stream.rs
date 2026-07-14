//! Parser for `grok --output-format streaming-json`.
//!
//! Grok Build emits newline-delimited JSON with `text`, `thought`, `end`, and
//! `error` events. Unknown lifecycle events are intentionally ignored: the
//! CLI documents the event set as non-exhaustive (for example auto-compaction
//! notices), and rendering raw JSON would leak protocol noise into chat.

use op_ai::chat_provider::{ChatDelta, StopReason};

/// Parse one Grok Build streaming-JSON line. `None` means the line is a banner
/// or an unknown lifecycle event and should not be shown to the user.
pub fn parse_grok_stream_line(line: &str) -> Option<ChatDelta> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let event_type = value.get("type").and_then(serde_json::Value::as_str)?;
    match event_type {
        "text" => required_string_delta(&value, "data", event_type, false),
        "thought" => required_string_delta(&value, "data", event_type, true),
        "end" => Some(ChatDelta::Done {
            stop_reason: grok_stop_reason(
                value
                    .get("stopReason")
                    .or_else(|| value.get("stop_reason"))
                    .and_then(serde_json::Value::as_str),
            ),
        }),
        "error" => Some(ChatDelta::Error(
            value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .filter(|message| !message.is_empty())
                .unwrap_or("Grok Build returned an unknown error.")
                .to_string(),
        )),
        _ => None,
    }
}

fn required_string_delta(
    value: &serde_json::Value,
    field: &str,
    event_type: &str,
    thinking: bool,
) -> Option<ChatDelta> {
    match value.get(field).and_then(serde_json::Value::as_str) {
        Some(text) if !text.is_empty() => Some(if thinking {
            ChatDelta::Thinking(text.to_string())
        } else {
            ChatDelta::TextDelta(text.to_string())
        }),
        Some(_) => None,
        None => Some(ChatDelta::Error(format!(
            "Malformed Grok Build {event_type} event: missing {field}."
        ))),
    }
}

fn grok_stop_reason(reason: Option<&str>) -> StopReason {
    match reason.unwrap_or_default().to_ascii_lowercase().as_str() {
        "maxtokens" | "max_tokens" | "max_tokens_reached" => StopReason::MaxTokens,
        "tooluse" | "tool_use" => StopReason::ToolUse,
        "aborted" | "cancelled" | "canceled" | "user_abort" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_documented_text_thought_and_end_events() {
        assert_eq!(
            parse_grok_stream_line(r#"{"type":"text","data":"Here's"}"#),
            Some(ChatDelta::TextDelta("Here's".to_string()))
        );
        assert_eq!(
            parse_grok_stream_line(
                r#"{"type":"thought","data":"Analyzing the directory structure..."}"#
            ),
            Some(ChatDelta::Thinking(
                "Analyzing the directory structure...".to_string()
            ))
        );
        assert_eq!(
            parse_grok_stream_line(r#"{"type":"end","stopReason":"EndTurn","sessionId":"abc123"}"#),
            Some(ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            })
        );
    }

    #[test]
    fn maps_terminal_reasons_and_errors() {
        assert_eq!(
            parse_grok_stream_line(r#"{"type":"end","stopReason":"MaxTokens"}"#),
            Some(ChatDelta::Done {
                stop_reason: StopReason::MaxTokens
            })
        );
        assert_eq!(
            parse_grok_stream_line(r#"{"type":"error","message":"not signed in"}"#),
            Some(ChatDelta::Error("not signed in".to_string()))
        );
    }

    #[test]
    fn skips_banners_and_future_lifecycle_events() {
        assert_eq!(parse_grok_stream_line("Checking for updates"), None);
        assert_eq!(
            parse_grok_stream_line(r#"{"type":"auto_compact_started"}"#),
            None
        );
    }
}
