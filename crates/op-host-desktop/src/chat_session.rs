//! Background chat-turn runner.
//!
//! `ChatProvider::send()` hands back a *blocking* iterator — draining
//! it on the UI thread would freeze the window for the whole LLM
//! turn. `ChatSession` drains the turn on a dedicated worker thread
//! and exposes a non-blocking [`ChatSession::poll`] the winit event
//! loop pumps each frame, appending deltas to the in-flight assistant
//! message.

use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, CliName};
use op_editor_core::{ChatMessage, ChatToolCall, EditorState};
use op_host_native::WidgetHostNative;
use op_orchestrator::{classify_intent, DesignRequest, Intent};

use crate::chat_builtin_http::ConfiguredBuiltinProvider;
use crate::chat_claude::ClaudeCodeProvider;
use crate::chat_copilot::CopilotProvider;
use crate::chat_provider_llm::ChatProviderLlmClient;
use crate::chat_subprocess::SubprocessProvider;
use crate::design_session::DesignSession;

/// One in-flight chat turn. The worker thread owns the provider and
/// drains `provider.send()` into the channel; [`poll`] consumes
/// whatever is ready without blocking.
pub struct ChatSession {
    rx: Receiver<ChatDelta>,
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
    message.tool_calls.extend(poll.tool_calls.iter().cloned());
    if poll.finished {
        message.streaming = false;
    }
}

impl ChatSession {
    /// Spawn a worker that drains `provider.send(req)` into a
    /// channel. Returns immediately — the LLM turn runs off-thread.
    pub fn start(provider: Box<dyn ChatProvider>, req: ChatRequest) -> Self {
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

/// Drain `chat.pending_send` (raised by `ChatState::begin_send`) and
/// route it through the orchestrator intent gate. `Intent::Design`
/// requests with a configured `agent::Provider` launch into
/// `current_design`; everything else (chat intent, or design intent
/// with no Provider available) falls through to the existing
/// `ChatProvider` path in `current_chat`. A send fired mid-turn
/// replaces the in-flight session — the old worker thread drains
/// harmlessly once its channel receiver drops.
///
/// Returns true when *any* turn was launched (caller redraws).
pub fn launch_if_pending(
    host: &mut WidgetHostNative,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    let Some(user_text) = host.editor_state_mut().chat.pending_send.take() else {
        return false;
    };
    host.mark_editor_state_dirty();
    // Intent gate (op_orchestrator::classify_intent) — design verbs
    // route to the orchestrator pipeline, which reuses the chat
    // panel's currently-selected agent as its LLM (via
    // `ChatProviderLlmClient`). Unwired agents (Codex / OpenCode) fall
    // through to the chat path so `provider_for_agent` can surface the
    // unwired-agent error in the assistant bubble.
    if matches!(classify_intent(&user_text), Intent::Design) {
        if let Some(provider) = provider_for_selected_model(host) {
            *current_chat = None;
            let llm = ChatProviderLlmClient::new(Arc::from(provider));
            if clear_fresh_starter_frame_for_design(host.editor_state_mut()) {
                host.mark_editor_state_dirty();
            }
            let initial_state = host.editor_state().clone();
            let request = DesignRequest {
                prompt: user_text,
                // The chosen chat agent decides its own model; the
                // orchestrator only passes through `req.model` when
                // it explicitly overrides per sub-call (it doesn't
                // today).
                model: None,
                provider: None,
                design_md: initial_state.doc.design_md.clone(),
                append_context: None,
                concurrency: 1,
                validation_enabled: false,
                visual_ref_enabled: false,
            };
            *current_design = Some(DesignSession::start(llm, request, initial_state));
            return true;
        }
        // Design intent but the selected agent has no ChatProvider
        // bridge yet — fall through to the chat path so the unwired
        // agent error message lands in the assistant bubble.
    }
    // Taking the chat path — drop any in-flight design turn so its
    // worker's next `apply` returns false (channel dropped) and its
    // `Progress` deltas stop streaming into this turn's fresh bubble
    // (codex stop-gate: stale design session survived chat fallback,
    // kept overwriting the new bubble content + applying ack'd
    // EditorCommands long after the user moved on).
    *current_design = None;
    let Some(provider) = provider_for_selected_model(host) else {
        // Selected agent has no `ChatProvider` bridge yet (Codex /
        // OpenCode HTTP-server transport). Surface that honestly in
        // the assistant bubble instead of silently running a
        // different agent (codex stop-gate: silent reroute to
        // Claude misled the user about which CLI answered).
        //
        // Drop any in-flight session FIRST — otherwise the next
        // `pump` keeps streaming the previous agent's deltas into
        // this fresh error bubble (codex stop-gate: stale session
        // overwrote the unwired-agent error text).
        *current_chat = None;
        let name = selected_provider_label(host);
        let chat = &mut host.editor_state_mut().chat;
        if let Some(msg) = chat.messages.last_mut() {
            msg.content = format!(
                "error: {name} chat is not wired yet — its HTTP-server \
                 transport is still pending. Pick Claude Code, GitHub \
                 Copilot, or Gemini CLI via the model chip."
            );
            // The turn is aborted — `begin_send` created this bubble
            // as `streaming`; clear it so the panel doesn't keep
            // animating a stream that will never arrive.
            msg.streaming = false;
        }
        // This turn consumed the staged attachments (they are already
        // copied into the user message); drop them so they don't leak
        // into the next send.
        chat.pending_attachments.clear();
        host.mark_editor_state_dirty();
        // No session started; report the transcript change so the
        // caller repaints the error.
        return true;
    };
    // Thread the per-turn knobs the chat panel carries into the
    // request, then clear the staged attachments — they belong to
    // this turn only.
    let chat = &mut host.editor_state_mut().chat;
    let thinking = chat.thinking_mode;
    let effort = chat.effort_level;
    let attachments = std::mem::take(&mut chat.pending_attachments);
    let req = ChatRequest {
        system_prompt: String::new(),
        user_message: user_text,
        max_output_tokens: 4096,
        thinking,
        effort,
        attachments,
    };
    *current_chat = Some(ChatSession::start(provider, req));
    true
}

/// Drain a New Chat request raised by the widget layer. The transcript
/// has already been cleared inside `ChatState::new_chat`; this drops
/// any in-flight workers so stale deltas cannot repopulate it.
pub fn drain_new_chat_request(
    host: &mut WidgetHostNative,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    if !std::mem::take(&mut host.editor_state_mut().chat.pending_new_chat) {
        return false;
    }
    *current_chat = None;
    *current_design = None;
    host.mark_editor_state_dirty();
    true
}

fn clear_fresh_starter_frame_for_design(state: &mut EditorState) -> bool {
    if state.doc != EditorState::starter().doc {
        return false;
    }
    state.active_children_mut().clear();
    state.clear_selection();
    true
}

/// Build the `ChatProvider` for an agent index (into
/// `AgentProvider::ALL`: 0 ClaudeCode, 1 CodexCli, 2 OpenCode,
/// 3 GithubCopilot, 4 GeminiCli). Claude Code uses its dedicated
/// SDK adapter; Codex / Copilot / Gemini use the subprocess transport.
/// Returns `None` for OpenCode — its `ChatProvider` bridge isn't
/// wired yet; the caller surfaces an explicit error rather than
/// rerouting to a different agent.
fn provider_for_agent(agent_idx: usize) -> Option<Box<dyn ChatProvider>> {
    match agent_idx {
        0 => Some(Box::new(ClaudeCodeProvider::new())),
        1 => SubprocessProvider::for_cli(CliName::Codex)
            .map(|p| Box::new(p) as Box<dyn ChatProvider>),
        3 => Some(Box::new(CopilotProvider::new())),
        4 => SubprocessProvider::for_cli(CliName::Gemini)
            .map(|p| Box::new(p) as Box<dyn ChatProvider>),
        // 2 OpenCode — no bridge yet.
        _ => None,
    }
}

fn provider_for_selected_model(host: &WidgetHostNative) -> Option<Box<dyn ChatProvider>> {
    if let Some(entry) = host.editor_state().chat.selected_model_entry() {
        if let Some(id) = entry.builtin_provider_id.as_deref() {
            return provider_for_builtin(host.editor_state(), id);
        }
    }
    provider_for_agent(host.editor_state().editor_ui.chat_selected_agent)
}

fn provider_for_builtin(state: &EditorState, id: &str) -> Option<Box<dyn ChatProvider>> {
    let config = state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .find(|agent| agent.id == id && agent.ready())?;
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(config)?;
    Some(Box::new(provider))
}

fn selected_provider_label(host: &WidgetHostNative) -> String {
    if let Some(entry) = host.editor_state().chat.selected_model_entry() {
        if let Some(id) = entry.builtin_provider_id.as_deref() {
            if let Some(agent) = host
                .editor_state()
                .editor_ui
                .agent_settings
                .builtin_agents
                .iter()
                .find(|agent| agent.id == id)
            {
                return agent.display_name.clone();
            }
        }
    }
    let agent_idx = host.editor_state().editor_ui.chat_selected_agent;
    op_editor_core::AgentProvider::ALL
        .get(agent_idx)
        .map(|a| a.name().to_string())
        .unwrap_or_else(|| "This agent".into())
}

/// Pump the in-flight turn's deltas into the trailing (assistant)
/// message. Clears `current` once the turn finishes. Returns true
/// when the transcript changed so the caller can dirty the redraw.
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
        if changed {
            host.mark_editor_state_dirty();
        }
    }
    if poll.finished {
        *current = None;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::chat_provider::{EchoProvider, StopReason};

    #[test]
    fn session_streams_echo_provider_deltas_to_completion() {
        let provider = Box::new(EchoProvider {
            script: vec![
                ChatDelta::TextDelta("Hel".into()),
                ChatDelta::TextDelta("lo".into()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        });
        let mut session = ChatSession::start(
            provider,
            ChatRequest {
                system_prompt: String::new(),
                user_message: "hi".into(),
                max_output_tokens: 256,
                ..Default::default()
            },
        );
        // Drain to completion — poll in a bounded loop so a stuck
        // worker fails the test instead of hanging it.
        let mut acc = String::new();
        for _ in 0..1000 {
            let p = session.poll();
            acc.push_str(&p.text);
            if p.finished {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert!(session.finished(), "session must reach Done");
        assert_eq!(acc, "Hello");
    }

    #[test]
    fn poll_splits_thinking_and_tool_calls_from_answer_text() {
        let provider = Box::new(EchoProvider {
            script: vec![
                ChatDelta::Thinking("let me think".into()),
                ChatDelta::ToolUse {
                    name: "insert_node".into(),
                    args: "{\"kind\":\"rect\"}".into(),
                },
                ChatDelta::TextDelta("here is the answer".into()),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ],
        });
        let mut session = ChatSession::start(
            provider,
            ChatRequest {
                user_message: "x".into(),
                max_output_tokens: 64,
                ..Default::default()
            },
        );
        let mut text = String::new();
        let mut thinking = String::new();
        let mut tools = Vec::new();
        for _ in 0..1000 {
            let p = session.poll();
            text.push_str(&p.text);
            thinking.push_str(&p.thinking);
            tools.extend(p.tool_calls);
            if p.finished {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(text, "here is the answer", "answer text only");
        assert_eq!(thinking, "let me think", "thinking routed separately");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "insert_node");
        assert_eq!(tools[0].args, "{\"kind\":\"rect\"}");
    }

    #[test]
    fn apply_poll_appends_text_thinking_tools_and_clears_streaming_on_finish() {
        let mut msg = ChatMessage::assistant_streaming();
        apply_poll_to_message(
            &mut msg,
            &ChatPoll {
                text: "hi".into(),
                thinking: "reasoning".into(),
                tool_calls: vec![ChatToolCall {
                    name: "t".into(),
                    args: "{}".into(),
                }],
                error: None,
                finished: false,
            },
        );
        assert_eq!(msg.content, "hi");
        assert_eq!(msg.thinking, "reasoning");
        assert_eq!(msg.tool_calls.len(), 1);
        assert!(msg.streaming, "still streaming until the turn finishes");

        apply_poll_to_message(
            &mut msg,
            &ChatPoll {
                text: "!".into(),
                thinking: String::new(),
                tool_calls: vec![],
                error: None,
                finished: true,
            },
        );
        assert_eq!(msg.content, "hi!", "text accumulates across polls");
        assert!(!msg.streaming, "finished clears the streaming flag");
    }

    #[test]
    fn apply_poll_error_replaces_content_and_ends_stream() {
        let mut msg = ChatMessage::assistant_streaming();
        msg.content = "partial answer".into();
        apply_poll_to_message(
            &mut msg,
            &ChatPoll {
                text: String::new(),
                thinking: String::new(),
                tool_calls: vec![],
                error: Some("rate limited".into()),
                finished: true,
            },
        );
        assert_eq!(msg.content, "error: rate limited");
        assert!(!msg.streaming);
    }

    #[test]
    fn selected_builtin_model_routes_to_builtin_provider() {
        let mut host = WidgetHostNative::new();
        let id = host
            .editor_state_mut()
            .editor_ui
            .agent_settings
            .add_builtin_agent_with_defaults("Built-in Claude", "sk-test", "claude-sonnet-4-5");
        host.editor_state_mut().rebuild_chat_models();
        let idx = host
            .editor_state()
            .chat
            .available_models
            .iter()
            .position(|m| m.builtin_provider_id.as_deref() == Some(id.as_str()))
            .expect("built-in model should be selectable");
        host.editor_state_mut().select_chat_model(idx);

        let provider = provider_for_selected_model(&host).expect("built-in provider should build");
        assert_eq!(provider.provider_label(), "Built-in Claude");
    }

    #[test]
    fn session_surfaces_provider_error() {
        let provider = Box::new(EchoProvider {
            script: vec![ChatDelta::Error("boom".into())],
        });
        let mut session = ChatSession::start(
            provider,
            ChatRequest {
                system_prompt: String::new(),
                user_message: "x".into(),
                max_output_tokens: 0,
                ..Default::default()
            },
        );
        let mut err = None;
        for _ in 0..1000 {
            let p = session.poll();
            if p.error.is_some() {
                err = p.error;
            }
            if p.finished {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(err.as_deref(), Some("boom"));
    }

    #[test]
    fn design_turn_clears_fresh_starter_frame() {
        let mut state = op_editor_core::EditorState::starter();

        assert!(clear_fresh_starter_frame_for_design(&mut state));
        assert!(state.active_children().is_empty());
        assert!(state.selection.is_empty());
    }

    #[test]
    fn design_turn_preserves_non_starter_documents() {
        let mut state = op_editor_core::EditorState::starter();
        state.active_children_mut().clear();

        assert!(!clear_fresh_starter_frame_for_design(&mut state));
        assert!(state.active_children().is_empty());
    }
}
