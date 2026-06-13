//! Background chat-turn runner.
//!
//! `ChatProvider::send()` hands back a *blocking* iterator — draining
//! it on the UI thread would freeze the window for the whole LLM
//! turn. `ChatSession` drains the turn on a dedicated worker thread
//! and exposes a non-blocking [`ChatSession::poll`] the winit event
//! loop pumps each frame, appending deltas to the in-flight assistant
//! message.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, ChatToolResult};
use op_editor_core::{ChatMessage, ChatState, ChatToolCall, EditorState};
use op_host_native::WidgetHostNative;

use crate::chat_canvas_tools::{execute_chat_tool, ChatToolRequest};

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

/// One in-flight chat turn. The worker thread owns the provider and
/// drains `provider.send()` into the channel; [`poll`] consumes
/// whatever is ready without blocking.
pub struct ChatSession {
    rx: Receiver<ChatDelta>,
    /// Canvas tool-call requests from the builtin agent loop. `None`
    /// for providers without tool execution. Drained by [`pump`] each
    /// frame — the worker blocks on each request's ack, mirroring the
    /// design session's command channel.
    tool_rx: Option<Receiver<ChatToolRequest>>,
    finished: bool,
}

/// Result of a single non-blocking [`ChatSession::poll`].
pub struct ChatPoll {
    /// Answer-text fragments (`TextDelta`) accumulated since the last
    /// poll. Empty when nothing new arrived.
    pub text: String,
    /// Reasoning fragments (`Thinking`) accumulated since the last
    /// poll — kept separate from `text` so the chat panel can render
    /// them in their own collapsible block.
    pub thinking: String,
    /// Tool invocations (`ToolUse`) seen this poll.
    pub tool_calls: Vec<ChatToolCall>,
    /// First error seen this poll, if any. When set the caller
    /// should surface it as the assistant message body.
    pub error: Option<String>,
    /// True once the turn's terminal `Done` arrived, or the worker
    /// thread / channel closed.
    pub finished: bool,
}

impl ChatPoll {
    /// True when this poll carried no new content and the turn has
    /// not ended — the caller can skip touching the transcript.
    fn is_idle(&self) -> bool {
        self.text.is_empty()
            && self.thinking.is_empty()
            && self.tool_calls.is_empty()
            && self.error.is_none()
            && !self.finished
    }
}

/// Fold one [`ChatPoll`] into the trailing assistant `message`. An
/// error replaces the visible body; otherwise answer text + thinking
/// accumulate and tool calls append. A finished poll clears the
/// `streaming` flag so the panel stops the streaming animation.
pub fn apply_poll_to_message(message: &mut ChatMessage, poll: &ChatPoll) {
    if let Some(err) = &poll.error {
        message.content = format!("error: {err}");
    } else {
        message.content.push_str(&poll.text);
    }
    message.thinking.push_str(&poll.thinking);
    if poll.tool_calls.iter().any(tool_call_defaults_open) {
        message.tools_collapsed = false;
    }
    message.tool_calls.extend(poll.tool_calls.iter().cloned());
    if poll.finished {
        message.streaming = false;
    }
}

fn tool_call_defaults_open(call: &ChatToolCall) -> bool {
    if let Some(level) = tool_level_from_args(&call.args) {
        return matches!(level.as_str(), "modify" | "delete" | "orchestrate");
    }
    matches!(
        call.name.as_str(),
        "update_node"
            | "replace_node"
            | "move_node"
            | "set_variables"
            | "set_themes"
            | "load_theme_preset"
            | "rename_page"
            | "reorder_page"
            | "batch_design"
            | "set_design_md"
            | "export_design_md"
            | "delete_node"
            | "remove_page"
    )
}

fn tool_level_from_args(args: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(args)
        .ok()?
        .get("level")?
        .as_str()
        .map(str::to_string)
}

impl ChatSession {
    /// Spawn a worker that drains `provider.send(req)` into a
    /// channel. Returns immediately — the LLM turn runs off-thread.
    pub fn start(provider: Box<dyn ChatProvider>, req: ChatRequest) -> Self {
        Self::start_with_tools(provider, req, None)
    }

    /// [`start`](Self::start) plus a canvas tool-request channel for
    /// tool-executing providers (the builtin agent loop). [`pump`]
    /// drains `tool_rx` each frame and executes the calls against the
    /// live editor state.
    pub fn start_with_tools(
        provider: Box<dyn ChatProvider>,
        req: ChatRequest,
        tool_rx: Option<Receiver<ChatToolRequest>>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("op-chat-turn".into())
            .spawn(move || {
                // `provider.send` itself returns a blocking iterator;
                // draining it here keeps the block off the UI thread.
                for delta in provider.send(req) {
                    if tx.send(delta).is_err() {
                        return; // chat panel went away — stop early
                    }
                }
            })
            .expect("spawn op-chat-turn thread");
        Self {
            rx,
            tool_rx,
            finished: false,
        }
    }

    /// Wrap externally-supplied channels — the CLI intent router
    /// (GAP #33) owns its own worker thread and feeds these directly.
    pub(crate) fn from_channels(
        rx: Receiver<ChatDelta>,
        tool_rx: Option<Receiver<ChatToolRequest>>,
    ) -> Self {
        Self {
            rx,
            tool_rx,
            finished: false,
        }
    }

    /// Drain every delta ready right now without blocking.
    pub fn poll(&mut self) -> ChatPoll {
        let mut text = String::new();
        let mut thinking = String::new();
        let mut tool_calls = Vec::new();
        let mut error = None;
        loop {
            match self.rx.try_recv() {
                Ok(ChatDelta::TextDelta(s)) => text.push_str(&s),
                Ok(ChatDelta::Thinking(s)) => thinking.push_str(&s),
                // Tool dispatch is the agent runtime's job; the panel
                // only surfaces the call in its collapsible tool view.
                Ok(ChatDelta::ToolUse { name, args }) => {
                    tool_calls.push(ChatToolCall { name, args });
                }
                Ok(ChatDelta::Error(msg)) => {
                    if error.is_none() {
                        error = Some(msg);
                    }
                }
                Ok(ChatDelta::Done { .. }) => self.finished = true,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.finished = true;
                    break;
                }
            }
        }
        ChatPoll {
            text,
            thinking,
            tool_calls,
            error,
            finished: self.finished,
        }
    }

    /// True once the turn has fully completed. Test-only accessor —
    /// the event-loop glue keys off `ChatPoll::finished` instead.
    #[cfg(test)]
    pub fn finished(&self) -> bool {
        self.finished
    }
}

/// Pump the in-flight turn's deltas into the trailing (assistant)
/// message, then execute any pending canvas tool calls against the
/// live editor state. Clears `current` once the turn finishes.
/// Returns true when the transcript changed so the caller can dirty
/// the redraw.
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
    // Execute pending canvas tool calls AFTER folding the deltas in —
    // the agent loop emits the `ToolUse` delta (which creates the
    // transcript card) before it forwards the request, so by the time
    // a request is visible its card already exists.
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

/// Drain every pending canvas tool request from the in-flight turn
/// and execute it against the live `EditorState` — the chat-loop
/// mirror of `design_session::pump_commands`. Each request is acked
/// with its result so the blocked worker resumes; the matching
/// transcript tool card is updated with the real result. Returns true
/// when state or transcript changed.
fn drain_tool_requests(state: &mut EditorState, session: &mut ChatSession) -> bool {
    let Some(tool_rx) = session.tool_rx.as_ref() else {
        return false;
    };
    let mut requests = Vec::new();
    while let Ok(req) = tool_rx.try_recv() {
        requests.push(req);
    }
    if requests.is_empty() {
        return false;
    }
    let mut changed = false;
    for req in requests {
        // Internal host op from the DESIGN_MODIFY route (GAP #33) —
        // applies the parsed modification nodes against the live
        // state. Never advertised to a model; no transcript card.
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
        // If the ack fails the worker already dropped its receiver
        // (turn aborted) — nothing to do.
        let _ = req.ack.send(result);
    }
    changed
}

/// Record an executed tool call's result on its transcript card: the
/// last matching `status:"running"` envelope gains `result` +
/// `status: done|error`, which the chat panel's tool card renders as
/// its Result line (same envelope shape the TS cards use).
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
