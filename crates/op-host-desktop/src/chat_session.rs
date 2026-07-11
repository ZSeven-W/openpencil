//! Desktop chat-session host glue.
//!
//! The transport-free turn worker, poll result, transcript folding, and tool
//! channel live in `op-editor-host-core::chat`. This module keeps desktop
//! provider routing plus UI-thread tool execution against `WidgetHostNative`.

use op_ai::chat_provider::ChatToolResult;
use op_editor_core::{ChatMessage, ChatRole, ChatState, EditorState};
pub use op_editor_host_core::chat::ChatSession;
#[cfg(test)]
pub use op_editor_host_core::chat::{apply_poll_to_message, ChatPoll};
use op_host_native::WidgetHostNative;

use op_host_services::design_agent_tools::execute_agent_tool;

// Turn launch + provider routing (split out at the 800-line cap).
// `launch_if_pending` and friends live in the sibling file; the
// re-exports keep every external `chat_session::` path stable.
#[path = "chat_session_launch.rs"]
mod launch;
#[cfg(test)]
pub(crate) use launch::builtin_provider_with_tools;
pub(crate) use launch::reconcile_starter_ghost;
pub use launch::{drain_new_chat_request, drain_stop_request, launch_if_pending};
pub(crate) use launch::{provider_for_selected_model, selected_cli_model_id};
// Sub-agent launcher (Task 3.1) reuses the design-toolset provider builder
// and the design-turn thinking policy.
pub(crate) use launch::launch_design::{
    builtin_provider_with_design_tools, design_turn_thinking_mode,
};

/// Pump the in-flight turn's deltas into the trailing assistant message, then
/// execute any pending canvas tool calls against the live editor state.
///
/// `running_tab` is the chat tab this turn is bound to (MT.3 session-per-tab):
/// the deltas land in THAT tab's transcript even after the user switches the
/// active tab, so a streaming run never corrupts the now-active (wrong) tab.
/// `None` (or a stale/out-of-range index) falls back to the active tab.
/// `agent_identity` stamps sub-agent output without coupling lower-level chat
/// data to the orchestrator's `AgentIdentity` type.
pub fn pump(
    host: &mut WidgetHostNative,
    current: &mut Option<ChatSession>,
    running_tab: Option<usize>,
    agent_identity: Option<(&str, &str)>,
    viewport_size: (f32, f32),
) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let poll = session.poll();
    let mut changed = false;
    if !poll.is_idle() {
        let messages = &mut host
            .editor_state_mut()
            .chat
            .run_tab_mut(running_tab)
            .messages;
        if let Some(index) = agent_message_index(messages, agent_identity) {
            let msg = &mut messages[index];
            if let Some((name, color)) = agent_identity {
                msg.agent_name = Some(name.to_string());
                msg.agent_color = Some(color.to_string());
            }
            op_editor_host_core::chat::apply_poll_to_message_with(
                msg,
                &poll,
                session.is_design_loop(),
            );
            changed = true;
        }
    }
    if drain_tool_requests(
        host.editor_state_mut(),
        session,
        running_tab,
        agent_identity,
    ) {
        changed = true;
        // Keep the design in view while it generates:
        //  - first fit when the first SIZED root lands ("the artboard sits
        //    roughly centered when output starts");
        //  - after that, refit ONLY when a growth batch pushed content out
        //    of the visible canvas — a design that still fits never yanks
        //    the user's own pan/zoom framing.
        if session.is_design_loop() {
            let (vw, vh) = viewport_size;
            let state = host.editor_state_mut();
            if !session.viewport_fitted() {
                let has_sized_root = state.active_children().iter().any(|node| {
                    op_editor_core::PenNodeExt::width_px(node).is_some()
                        && op_editor_core::PenNodeExt::height_px(node).is_some()
                });
                if has_sized_root {
                    op_host_services::design_session::fit_design_viewport_to_content(state, vw, vh);
                    session.mark_viewport_fitted();
                }
            } else if !op_host_services::design_session::design_content_fits_viewport(state, vw, vh)
            {
                op_host_services::design_session::fit_design_viewport_to_content(state, vw, vh);
            }
        }
    }
    if changed {
        host.mark_editor_state_dirty();
    }
    if poll.finished {
        // Backstop: a design loop that died early (429 / quota / abort)
        // never sent its finalize op — run the structural passes anyway so
        // the canvas isn't left with the mid-run debris a clean finish
        // would have repaired (measured: empty 68px TabBar shell + empty
        // MiniPlayer survived an aborted run, test0711-22).
        if let Some(session) = current.as_ref() {
            if session.is_design_loop() && !session.loop_finalized() {
                op_orchestrator::apply_loop_finalize(host.editor_state_mut());
                host.mark_editor_state_dirty();
            }
        }
        *current = None;
    }
    changed
}

fn agent_message_index(
    messages: &[ChatMessage],
    agent_identity: Option<(&str, &str)>,
) -> Option<usize> {
    let matching = |message: &&ChatMessage| {
        message.role == ChatRole::Assistant
            && match agent_identity {
                Some((name, color)) => {
                    message.agent_name.as_deref() == Some(name)
                        && message.agent_color.as_deref() == Some(color)
                }
                None => message.agent_color.is_none(),
            }
    };
    messages
        .iter()
        .enumerate()
        .rev()
        .find(|(_, message)| matching(message))
        .map(|(index, _)| index)
        .or_else(|| {
            messages
                .iter()
                .rposition(|message| message.role == ChatRole::Assistant)
        })
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
    agent_identity: Option<(&str, &str)>,
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
            if crate::design_loop_indicator::reveal_drain_pending_for_active_epoch() {
                session.defer_tool_request(req);
                continue;
            }
            op_orchestrator::apply_loop_finalize(state);
            session.mark_loop_finalized();
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
            if attach_tool_result_to_transcript_with(
                state.chat.run_tab_mut(running_tab),
                &req.name,
                &result,
                agent_identity,
            ) {
                changed = true;
            }
            let _ = req.ack.send(result);
            continue;
        }
        if req.name == op_host_services::chat_intent::APPLY_MODIFICATION_OP {
            let nodes = op_host_services::chat_canvas_tools::parse_design_modification_ops_arg(
                &req.args_json,
            );
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
        if attach_tool_result_to_transcript_with(
            state.chat.run_tab_mut(running_tab),
            &req.name,
            &result,
            agent_identity,
        ) {
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
#[cfg(test)]
fn attach_tool_result_to_transcript(
    chat: &mut ChatState,
    name: &str,
    result: &ChatToolResult,
) -> bool {
    attach_tool_result_to_transcript_with(chat, name, result, None)
}

fn attach_tool_result_to_transcript_with(
    chat: &mut ChatState,
    name: &str,
    result: &ChatToolResult,
    agent_identity: Option<(&str, &str)>,
) -> bool {
    let Some(index) = agent_message_index(&chat.messages, agent_identity) else {
        return false;
    };
    let msg = &mut chat.messages[index];
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

#[cfg(test)]
#[path = "chat_session_identity_tests.rs"]
mod identity_tests;

#[cfg(test)]
#[path = "chat_session_reveal_tests.rs"]
mod reveal_tests;
