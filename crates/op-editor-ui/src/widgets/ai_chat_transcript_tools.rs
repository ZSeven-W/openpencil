use op_editor_core::chat::ChatToolCall;
use serde_json::Value;

use super::ai_chat_transcript::wrap_units;

/// Format tool-call process cards with the same high-level semantics
/// surfaced by the TS chat: source, status, result, and call args.
pub(crate) fn tool_lines(calls: &[ChatToolCall], budget: u32, default_status: &str) -> Vec<String> {
    const MAX_ARG_LINES: usize = 8;
    let mut lines = Vec::new();
    for c in calls {
        lines.push(format!("→ {}", c.name));
        let card = ToolCardFields::from_args(&c.args, default_status);
        if let Some(source) = card.source.as_deref() {
            lines.push(format!("  Source: {source}"));
        }
        lines.push(format!("  Status: {}", card.status));
        if let Some(result) = card.result.as_deref() {
            lines.push(format!("  Result: {result}"));
        }
        if !card.args.is_empty() {
            push_wrapped_tool_line(&mut lines, "Args", &card.args, budget, MAX_ARG_LINES);
        }
    }
    lines
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolCardFields {
    source: Option<String>,
    status: String,
    result: Option<String>,
    args: String,
}

impl ToolCardFields {
    fn from_args(raw_args: &str, default_status: &str) -> Self {
        let compact: String = raw_args.split_whitespace().collect::<Vec<_>>().join(" ");
        let Ok(value) = serde_json::from_str::<Value>(&compact) else {
            return Self {
                source: None,
                status: default_status.to_string(),
                result: None,
                args: compact,
            };
        };
        let Some(obj) = value.as_object() else {
            return Self {
                source: None,
                status: default_status.to_string(),
                result: None,
                args: compact_json(&value),
            };
        };

        let source = obj
            .get("source")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty() && *s != "lead")
            .map(str::to_string);
        let result = obj.get("result").and_then(result_summary);
        let status = obj
            .get("status")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| status_from_result(obj.get("result")))
            .unwrap_or_else(|| default_status.to_string());
        let args = obj.get("args").map(compact_json).unwrap_or(compact);

        Self {
            source,
            status,
            result,
            args,
        }
    }
}

fn compact_json(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
}

fn status_from_result(result: Option<&Value>) -> Option<String> {
    match result?.get("success").and_then(Value::as_bool) {
        Some(true) => Some("done".into()),
        Some(false) => Some("error".into()),
        None => None,
    }
}

fn result_summary(result: &Value) -> Option<String> {
    if result.get("success").and_then(Value::as_bool) == Some(false) {
        return result
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some(compact_json(result)));
    }
    result
        .get("data")
        .map(compact_json)
        .or_else(|| Some(compact_json(result)))
}

fn push_wrapped_tool_line(
    lines: &mut Vec<String>,
    label: &str,
    text: &str,
    budget: u32,
    max_lines: usize,
) {
    let wrapped = wrap_units(text, budget.saturating_sub(8).max(1));
    let shown = wrapped.len().min(max_lines);
    for (i, line) in wrapped[..shown].iter().enumerate() {
        if i == 0 {
            lines.push(format!("  {label}: {line}"));
        } else {
            lines.push(format!("  {line}"));
        }
    }
    if wrapped.len() > shown {
        lines.push("  …".to_string());
    }
}
