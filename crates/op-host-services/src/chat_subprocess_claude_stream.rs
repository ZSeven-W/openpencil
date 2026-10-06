//! Claude Code `--output-format stream-json` events for the subprocess
//! transport. The generic parser only knows the `text` / `thinking` /
//! `done` envelope; Claude's `system` / `assistant` / `result` lines used
//! to degrade to raw text, so a generation turn received JSON lines as
//! its script and failed with `expecting ';'`.

use op_ai::chat_provider::{ChatDelta, StopReason};

/// Parse one Claude Code stream-json line. `None` means the line carries
/// nothing for the transcript (init, rate-limit, echoed user turns).
pub fn parse_claude_stream_line(line: &str) -> Option<ChatDelta> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('{') {
        return Some(crate::chat_subprocess_parse::parse_line(line));
    }
    let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return Some(crate::chat_subprocess_parse::parse_line(line));
    };
    match val.get("type").and_then(serde_json::Value::as_str) {
        Some("system" | "rate_limit_event" | "user") => None,
        Some("assistant") => assistant_delta(&val),
        Some("result") => Some(result_delta(&val)),
        _ => Some(crate::chat_subprocess_parse::parse_line(line)),
    }
}

fn assistant_delta(val: &serde_json::Value) -> Option<ChatDelta> {
    let blocks = val.get("message")?.get("content")?.as_array()?;
    let mut text = String::new();
    let mut thinking = String::new();
    for block in blocks {
        match block.get("type").and_then(serde_json::Value::as_str) {
            Some("text") => {
                if let Some(t) = block.get("text").and_then(serde_json::Value::as_str) {
                    text.push_str(t);
                }
            }
            Some("thinking") => {
                if let Some(t) = block.get("thinking").and_then(serde_json::Value::as_str) {
                    thinking.push_str(t);
                }
            }
            _ => {}
        }
    }
    if !text.is_empty() {
        Some(ChatDelta::TextDelta(text))
    } else if !thinking.is_empty() {
        Some(ChatDelta::Thinking(thinking))
    } else {
        None
    }
}

/// The `result` line repeats the final text; the assistant line already
/// delivered it, so only the outcome is surfaced here.
fn result_delta(val: &serde_json::Value) -> ChatDelta {
    let is_error = val
        .get("is_error")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if is_error {
        let message = val
            .get("errors")
            .and_then(serde_json::Value::as_array)
            .map(|errors| {
                errors
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .filter(|s| !s.is_empty())
            .or_else(|| {
                val.get("result")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "claude exited with an error result".to_string());
        return ChatDelta::Error(message);
    }
    let stop_reason = match val.get("subtype").and_then(serde_json::Value::as_str) {
        Some("error_max_turns") => StopReason::MaxTokens,
        _ => StopReason::EndTurn,
    };
    ChatDelta::Done { stop_reason }
}

#[cfg(test)]
mod tests {
    use super::parse_claude_stream_line;
    use op_ai::chat_provider::{ChatDelta, StopReason};

    #[test]
    fn init_and_rate_limit_lines_carry_nothing() {
        assert!(parse_claude_stream_line(r#"{"type":"system","subtype":"init"}"#).is_none());
        assert!(parse_claude_stream_line(r#"{"type":"rate_limit_event"}"#).is_none());
        assert!(parse_claude_stream_line(r#"{"type":"user","message":{}}"#).is_none());
    }

    #[test]
    fn assistant_text_blocks_become_one_text_delta() {
        let line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"const a"},{"type":"text","text":" = 1;"}]}}"#;
        match parse_claude_stream_line(line) {
            Some(ChatDelta::TextDelta(t)) => assert_eq!(t, "const a = 1;"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn result_line_is_done_not_duplicated_text() {
        let line =
            r#"{"type":"result","subtype":"success","is_error":false,"result":"const a = 1;"}"#;
        match parse_claude_stream_line(line) {
            Some(ChatDelta::Done { stop_reason }) => assert_eq!(stop_reason, StopReason::EndTurn),
            other => panic!("expected done, got {other:?}"),
        }
    }

    #[test]
    fn error_result_surfaces_the_message() {
        let line = r#"{"type":"result","subtype":"error_during_execution","is_error":true,"errors":["boom"]}"#;
        match parse_claude_stream_line(line) {
            Some(ChatDelta::Error(m)) => assert_eq!(m, "boom"),
            other => panic!("expected error, got {other:?}"),
        }
    }

    #[test]
    fn unknown_lines_fall_back_to_the_generic_parser() {
        match parse_claude_stream_line(r#"{"type":"text","delta":"hi"}"#) {
            Some(ChatDelta::TextDelta(t)) => assert_eq!(t, "hi"),
            other => panic!("expected generic text, got {other:?}"),
        }
    }
}
