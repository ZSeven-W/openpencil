//! ACP `session/update` → [`ChatDelta`] adapter — the Rust analogue
//! of `pen-acp/src/event-adapter.ts` (`acpUpdateToSSE`), retargeted
//! from SSE strings to the OP chat panel's delta vocabulary.
//!
//! The turn terminator is the `session/prompt` *result*, not a
//! notification — so this adapter never emits `ChatDelta::Done`; the
//! `AcpProvider` emits it once `prompt()` returns.

use op_ai::chat_provider::ChatDelta;
use serde_json::Value;

use crate::protocol::{ContentBlock, SessionNotification, SessionUpdate};

/// Pull an error message out of a failed tool call's `content` blocks
/// — ACP places the text under `content[].content.text`.
fn extract_tool_error(content: &Value) -> Option<String> {
    let blocks = content.as_array()?;
    let mut found: Option<String> = None;
    for block in blocks {
        if let Some(text) = block
            .get("content")
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
        {
            found = Some(text.to_string());
        }
    }
    found
}

/// Map one ACP session-update notification to a [`ChatDelta`], or
/// `None` for updates the chat panel does not surface.
pub fn session_update_to_delta(note: &SessionNotification) -> Option<ChatDelta> {
    match &note.update {
        SessionUpdate::AgentMessageChunk {
            content: ContentBlock::Text { text },
        } => Some(ChatDelta::TextDelta(text.clone())),
        SessionUpdate::AgentThoughtChunk {
            content: ContentBlock::Text { text },
        } => Some(ChatDelta::Thinking(text.clone())),
        SessionUpdate::ToolCall {
            tool_call_id,
            title,
            raw_input,
        } => Some(ChatDelta::ToolUse {
            name: title.clone().unwrap_or_else(|| tool_call_id.clone()),
            args: raw_input.to_string(),
        }),
        SessionUpdate::ToolCallUpdate {
            status,
            content,
            raw_output,
            ..
        } => match status.as_deref() {
            Some("failed") => {
                let msg = extract_tool_error(content).unwrap_or_else(|| raw_output.to_string());
                Some(ChatDelta::Error(msg))
            }
            // `completed` is not a stream terminator — the turn ends
            // when `session/prompt` returns.
            _ => None,
        },
        // Non-text content / other update kinds carry nothing to show.
        SessionUpdate::AgentMessageChunk { .. }
        | SessionUpdate::AgentThoughtChunk { .. }
        | SessionUpdate::Other => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(update: Value) -> SessionNotification {
        serde_json::from_value(serde_json::json!({
            "sessionId": "s1",
            "update": update,
        }))
        .unwrap()
    }

    #[test]
    fn message_chunk_maps_to_text_delta() {
        let n = note(serde_json::json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "hello world" }
        }));
        assert_eq!(
            session_update_to_delta(&n),
            Some(ChatDelta::TextDelta("hello world".into()))
        );
    }

    #[test]
    fn thought_chunk_maps_to_thinking() {
        let n = note(serde_json::json!({
            "sessionUpdate": "agent_thought_chunk",
            "content": { "type": "text", "text": "hmm" }
        }));
        assert_eq!(
            session_update_to_delta(&n),
            Some(ChatDelta::Thinking("hmm".into()))
        );
    }

    #[test]
    fn tool_call_maps_to_tool_use() {
        let n = note(serde_json::json!({
            "sessionUpdate": "tool_call",
            "toolCallId": "tc1",
            "title": "snapshot_layout",
            "rawInput": { "page": 0 }
        }));
        match session_update_to_delta(&n) {
            Some(ChatDelta::ToolUse { name, .. }) => assert_eq!(name, "snapshot_layout"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn failed_tool_update_maps_to_error() {
        let n = note(serde_json::json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tc1",
            "status": "failed",
            "content": [ { "content": { "type": "text", "text": "permission denied" } } ]
        }));
        assert_eq!(
            session_update_to_delta(&n),
            Some(ChatDelta::Error("permission denied".into()))
        );
    }

    #[test]
    fn completed_tool_update_and_unknown_yield_nothing() {
        let completed = note(serde_json::json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "tc1",
            "status": "completed",
            "rawOutput": { "ok": true }
        }));
        assert_eq!(session_update_to_delta(&completed), None);
        let plan = note(serde_json::json!({ "sessionUpdate": "plan", "entries": [] }));
        assert_eq!(session_update_to_delta(&plan), None);
    }
}
