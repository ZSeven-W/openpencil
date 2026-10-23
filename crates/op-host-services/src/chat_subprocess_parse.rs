//! Generic stdout event parser for subprocess chat providers.

use op_ai::chat_provider::{ChatDelta, CliName, StopReason};

/// Parse a single CLI stdout line into a `ChatDelta`.
///
/// Known structured events are validated strictly. Plain text,
/// malformed JSON, and unknown event types remain visible as raw text
/// so custom stdout-only CLIs are still debuggable.
pub fn parse_line(line: &str) -> ChatDelta {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('{') {
        return raw_text(line);
    }
    let val: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return raw_text(line),
    };
    let ty = val.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match ty {
        "text" => match val.get("delta").and_then(|v| v.as_str()) {
            Some(s) => ChatDelta::TextDelta(s.to_string()),
            None => ChatDelta::Error(format!("malformed text event: {trimmed}")),
        },
        "thinking" => match val.get("delta").and_then(|v| v.as_str()) {
            Some(s) => ChatDelta::Thinking(s.to_string()),
            None => ChatDelta::Error(format!("malformed thinking event: {trimmed}")),
        },
        "tool_use" => match (val.get("name").and_then(|v| v.as_str()), val.get("args")) {
            (Some(name), Some(args)) => ChatDelta::ToolUse {
                name: name.to_string(),
                args: args.to_string(),
            },
            _ => ChatDelta::Error(format!("malformed tool_use event: {trimmed}")),
        },
        "done" => ChatDelta::Done {
            stop_reason: map_stop_reason(val.get("stop_reason").and_then(|v| v.as_str())),
        },
        "item.completed" => {
            let item = val.get("item");
            match (
                item.and_then(|i| i.get("type")).and_then(|v| v.as_str()),
                item.and_then(|i| i.get("text")).and_then(|v| v.as_str()),
            ) {
                (Some("agent_message"), Some(text)) => ChatDelta::TextDelta(text.to_string()),
                _ => ChatDelta::Thinking(String::new()),
            }
        }
        "turn.completed" => ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        },
        "thread.started" | "turn.started" | "item.started" | "item.updated" => {
            ChatDelta::Thinking(String::new())
        }
        "error" => match val.get("message").and_then(|v| v.as_str()) {
            Some(msg) => ChatDelta::Error(msg.to_string()),
            None => ChatDelta::Error(format!("malformed error event: {trimmed}")),
        },
        _ => raw_text(line),
    }
}

fn raw_text(line: &str) -> ChatDelta {
    ChatDelta::TextDelta(format!("{line}\n"))
}

fn map_stop_reason(value: Option<&str>) -> StopReason {
    match value.unwrap_or("") {
        "max_tokens" => StopReason::MaxTokens,
        "tool_use" => StopReason::ToolUse,
        "aborted" | "user_abort" => StopReason::Aborted,
        _ => StopReason::EndTurn,
    }
}

/// Per-CLI stdout parse: Codex skips unparsed lines (TS parity); generic
/// CLIs degrade them to raw text.
pub(crate) fn parse_cli_line(cli: Option<CliName>, line: &str) -> Option<ChatDelta> {
    match cli {
        Some(CliName::Codex) => crate::chat_subprocess_quirks::parse_codex_line(line),
        Some(CliName::GrokBuild) => crate::chat_grok_stream::parse_grok_stream_line(line),
        // dsh's whole stdout is the answer: every line is text, never an
        // event envelope.
        Some(CliName::Dsh) => Some(crate::chat_subprocess_dsh::parse_dsh_line(line)),
        Some(CliName::ClaudeCode) => {
            crate::chat_subprocess_claude_stream::parse_claude_stream_line(line)
        }
        Some(CliName::Kimi) => crate::chat_subprocess_kimi::parse_kimi_line(line),
        _ => Some(parse_line(line)),
    }
}
