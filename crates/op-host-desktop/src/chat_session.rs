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

use op_host_services::design_agent_tools::execute_agent_tool;

// Turn launch + provider routing (split out at the 800-line cap).
// `launch_if_pending` and friends live in the sibling file; the
// re-exports keep every external `chat_session::` path stable.
#[path = "chat_session_launch.rs"]
mod launch;
#[cfg(test)]
pub(crate) use launch::builtin_provider_with_tools;
pub use launch::{drain_new_chat_request, drain_stop_request, launch_if_pending};
pub(crate) use launch::{provider_for_selected_model, selected_cli_model_id};
// Sub-agent launcher (Task 3.1) reuses the design-toolset provider builder.
pub(crate) use launch::launch_design::builtin_provider_with_design_tools;

/// Pump the in-flight turn's deltas into the trailing assistant message, then
/// execute any pending canvas tool calls against the live editor state.
///
/// `running_tab` is the chat tab this turn is bound to (MT.3 session-per-tab):
/// the deltas land in THAT tab's transcript even after the user switches the
/// active tab, so a streaming run never corrupts the now-active (wrong) tab.
/// `None` (or a stale/out-of-range index) falls back to the active tab.
pub fn pump(
    host: &mut WidgetHostNative,
    current: &mut Option<ChatSession>,
    running_tab: Option<usize>,
) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let poll = session.poll();
    let mut changed = false;
    if !poll.is_idle() {
        if let Some(msg) = host
            .editor_state_mut()
            .chat
            .run_tab_mut(running_tab)
            .messages
            .last_mut()
        {
            apply_poll_to_message(msg, &poll);
            changed = true;
        }
    }
    if drain_tool_requests(host.editor_state_mut(), session, running_tab) {
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
///
/// Canvas mutations write to the shared document (not a per-tab field), but a
/// tool-call card's result is recorded on the BOUND tab's transcript
/// (`running_tab`), so a tab switch mid-run doesn't drop the card on the wrong
/// tab.
fn drain_tool_requests(
    state: &mut EditorState,
    session: &mut ChatSession,
    running_tab: Option<usize>,
) -> bool {
    let requests = session.drain_tool_requests();
    if requests.is_empty() {
        return false;
    }
    let mut changed = false;
    for req in requests {
        // Intercept the reserved loop-finalize op (Track-1 Step 4): the
        // agentic design loop sends this at loop end so the host runs the
        // deterministic structural-quality backstop over the assembled live
        // document — the same whole-doc subset of the orchestrator's Class-A
        // passes the orchestrator runs per subtask. Runs on the UI thread (the
        // owner of the live `EditorState`), like every other tool mutation.
        if req.name == op_ai::chat_provider::LOOP_FINALIZE_OP {
            op_orchestrator::apply_loop_finalize(state);
            changed = true;
            let _ = req.ack.send(ChatToolResult {
                content: serde_json::json!({ "success": true }).to_string(),
                is_error: false,
            });
            continue;
        }
        // Intercept `spawn_agents`: parse the specs, stash them for the
        // host to launch after this (parent) pump, and ack immediately
        // (fire-and-forget). A SUB calling `spawn_agents` is refused —
        // only the top-level loop spawns. Keeps `pump`'s signature
        // unchanged; the launch happens in `app_handler` post-pump.
        if req.name == "spawn_agents" {
            let result = handle_spawn_agents(&req.args_json);
            if attach_tool_result_to_transcript(
                state.chat.run_tab_mut(running_tab),
                &req.name,
                &result,
            ) {
                changed = true;
            }
            let _ = req.ack.send(result);
            continue;
        }
        if req.name == op_host_services::chat_intent::APPLY_MODIFICATION_OP {
            let nodes = serde_json::from_str::<serde_json::Value>(&req.args_json)
                .ok()
                .and_then(|v| v.get("nodes").and_then(|n| n.as_array().cloned()))
                .unwrap_or_default();
            let (count, mutated) =
                op_host_services::chat_canvas_tools::apply_design_modification(state, &nodes);
            if mutated {
                changed = true;
            }
            let _ = req.ack.send(ChatToolResult {
                content: serde_json::json!({ "success": true, "count": count }).to_string(),
                is_error: false,
            });
            continue;
        }
        let (result, mutated) = execute_agent_tool(state, &req.name, &req.args_json);
        if mutated {
            changed = true;
        }
        if attach_tool_result_to_transcript(state.chat.run_tab_mut(running_tab), &req.name, &result)
        {
            changed = true;
        }
        let _ = req.ack.send(result);
    }
    changed
}

/// Handle a `spawn_agents` tool call from the design loop: parse the
/// specs, stash them for the host to launch after the parent pump, and
/// return the fire-and-forget ack.
///
/// - Top-level loop → stash N specs, ack `{spawned, agentIds}` (the
///   Phase-0 result shape; the host launches the sub-loops post-pump).
/// - A SUB calling `spawn_agents` again → refused (nested spawns no-op);
///   acks `{spawned: 0}` so the sub doesn't keep retrying.
/// - Parse error → an error result so the model can correct.
///
/// The ack rides the same `{success, data}` / `{success, error}` envelope
/// the design-tool surface uses (`execute_with_registry`) so the model
/// sees a consistent shape regardless of which path handled the call.
fn handle_spawn_agents(args_json: &str) -> ChatToolResult {
    use crate::sub_agent_session::{nested_spawn_active, parse_spawn_args, stash_pending_spawn};

    let specs = match parse_spawn_args(args_json) {
        Ok(specs) => specs,
        Err(msg) => {
            return ChatToolResult {
                content: serde_json::json!({ "success": false, "error": msg }).to_string(),
                is_error: true,
            };
        }
    };
    let n = specs.len();
    if !stash_pending_spawn(specs, nested_spawn_active()) {
        // Nested spawn — a sub-agent tried to spawn again. No-op.
        return ChatToolResult {
            content: serde_json::json!({
                "success": true,
                "data": {
                    "spawned": 0,
                    "agentIds": [],
                    "note": "nested spawn_agents ignored — only the top-level design loop spawns sub-agents"
                }
            })
            .to_string(),
            is_error: false,
        };
    }
    let agent_ids: Vec<String> = (0..n).map(|i| format!("agent-{i}")).collect();
    ChatToolResult {
        content: serde_json::json!({
            "success": true,
            "data": { "spawned": n, "agentIds": agent_ids }
        })
        .to_string(),
        is_error: false,
    }
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
