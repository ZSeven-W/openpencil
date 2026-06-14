//! Desktop chat-session host glue.
//!
//! The transport-free turn worker, poll result, transcript folding, and tool
//! channel live in `op-editor-host-core::chat`. This module keeps desktop
//! provider routing plus UI-thread tool execution against `WidgetHostNative`.

use op_ai::chat_provider::ChatToolResult;
use op_editor_core::{ChatState, EditorState};
#[cfg(test)]
pub use op_editor_host_core::chat::ChatPoll;
pub use op_editor_host_core::chat::{apply_poll_to_message, ChatSession};
use op_host_native::WidgetHostNative;

use crate::chat_canvas_tools::execute_chat_tool;

// Turn launch + provider routing (split out at the 800-line cap).
// `launch_if_pending` and friends live in the sibling file; the
// re-exports keep every external `chat_session::` path stable.
#[path = "chat_session_launch.rs"]
mod launch;
pub(crate) use launch::provider_for_selected_model;
#[cfg(test)]
pub(crate) use launch::{
    builtin_provider_with_tools, clear_fresh_starter_frame_for_design, selected_cli_model_id,
};
pub use launch::{drain_new_chat_request, drain_stop_request, launch_if_pending};

/// Pump the in-flight turn's deltas into the trailing assistant message, then
/// execute any pending canvas tool calls against the live editor state.
pub fn pump(host: &mut WidgetHostNative, current: &mut Option<ChatSession>) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let poll = session.poll();
    let mut changed = false;
    if !poll.is_idle() {
        if let Some(msg) = host.editor_state_mut().chat.messages.last_mut() {
            apply_poll_to_message(msg, &poll);
            changed = true;
        }
    }
    if drain_tool_requests(host.editor_state_mut(), session) {
        changed = true;
    }
    if changed {
        host.mark_editor_state_dirty();
    }
    if poll.finished {
        *current = None;
    }
    changed
}

/// Drain every pending canvas tool request from the in-flight turn and execute
/// it against the live `EditorState`.
fn drain_tool_requests(state: &mut EditorState, session: &mut ChatSession) -> bool {
    let requests = session.drain_tool_requests();
    if requests.is_empty() {
        return false;
    }
    let mut changed = false;
    for req in requests {
        if req.name == crate::chat_intent::APPLY_MODIFICATION_OP {
            let nodes = serde_json::from_str::<serde_json::Value>(&req.args_json)
                .ok()
                .and_then(|v| v.get("nodes").and_then(|n| n.as_array().cloned()))
                .unwrap_or_default();
            let (count, mutated) =
                crate::chat_canvas_tools::apply_design_modification(state, &nodes);
            if mutated {
                changed = true;
            }
            let _ = req.ack.send(ChatToolResult {
                content: serde_json::json!({ "success": true, "count": count }).to_string(),
                is_error: false,
            });
            continue;
        }
        let (result, mutated) = execute_chat_tool(state, &req.name, &req.args_json);
        if mutated {
            changed = true;
        }
        if attach_tool_result_to_transcript(&mut state.chat, &req.name, &result) {
            changed = true;
        }
        let _ = req.ack.send(result);
    }
    changed
}

/// Record an executed tool call's result on its transcript card.
fn attach_tool_result_to_transcript(
    chat: &mut ChatState,
    name: &str,
    result: &ChatToolResult,
) -> bool {
    let Some(msg) = chat
        .messages
        .last_mut()
        .filter(|m| m.role == op_editor_core::ChatRole::Assistant)
    else {
        return false;
    };
    for call in msg.tool_calls.iter_mut().rev() {
        if call.name != name {
            continue;
        }
        let Ok(mut envelope) = serde_json::from_str::<serde_json::Value>(&call.args) else {
            continue;
        };
        let Some(obj) = envelope.as_object_mut() else {
            continue;
        };
        if obj.get("status").and_then(serde_json::Value::as_str) != Some("running") {
            continue;
        }
        let result_value = serde_json::from_str::<serde_json::Value>(&result.content)
            .unwrap_or_else(|_| serde_json::Value::String(result.content.clone()));
        obj.insert("result".into(), result_value);
        let status = if result.is_error { "error" } else { "done" };
        obj.insert(
            "status".into(),
            serde_json::Value::String(status.to_string()),
        );
        call.args = envelope.to_string();
        return true;
    }
    false
}

#[cfg(test)]
#[path = "chat_session_tests.rs"]
mod tests;
