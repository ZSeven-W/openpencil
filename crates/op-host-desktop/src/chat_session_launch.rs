//! Chat-turn launch + provider routing — `launch_if_pending` and the
//! per-provider transport builders, split out of `chat_session.rs` at
//! the 800-line cap. Declared as a `#[path]` child of `chat_session`
//! so the session type and external `chat_session::` paths stay put.

use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;

use op_ai::chat_history::{trim_chat_history, DEFAULT_MAX_CHARS, DEFAULT_MAX_MESSAGES};
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, CliName};
use op_editor_core::EditorState;
use op_host_native::WidgetHostNative;
use op_orchestrator::{classify_intent, Intent};

use crate::chat_acp::AcpProvider;
use op_editor_host_core::design::DesignSession;
use op_host_services::chat_builtin_http::ConfiguredBuiltinProvider;
use op_host_services::chat_canvas_tools::{chat_tool_channel, chat_tool_defs, ChatToolRequest};
use op_host_services::chat_claude::ClaudeCodeProvider;
use op_host_services::chat_copilot::CopilotProvider;
use op_host_services::chat_http_server::OpenCodeProvider;
use op_host_services::chat_provider_llm::ChatProviderLlmClient;
use op_host_services::chat_subprocess::SubprocessProvider;
use op_host_services::chat_system_prompt::{
    build_agent_system_prompt, build_chat_system_prompt, chat_history_from_transcript,
};

use super::ChatSession;

#[path = "chat_design_request.rs"]
mod chat_design_request;
use chat_design_request::build_design_request;

// Design-agent-loop helpers (flag gate + provider builder + turn launcher)
// split out at the 800-line cap; see module docs there.
#[path = "chat_session_launch_design.rs"]
pub(super) mod launch_design;
use launch_design::launch_design_loop_turn;

/// Drain `chat.pending_send` (raised by `ChatState::begin_send`) and
/// route it.
///
/// CLI (standard-mode) selections route through the TS three-way
/// classifier on a worker thread (`chat_intent::run_cli_turn`, GAP
/// #33): a `ChatSession` AND a `DesignSession` are parked up front,
/// and the worker drives whichever route the classification picks
/// (chat stream / DESIGN_MODIFY / new-design orchestrator), dropping
/// the other route's channels so its pump retires the unused session.
///
/// Builtin / ACP selections keep the established shell routing (TS
/// returns early for them too, into its agent loop): the keyword
/// intent gate (`op_orchestrator::classify_intent`) sends design
/// verbs to the orchestrator pipeline, builtin chat turns to the
/// tool-executing agent loop, and everything else to the plain
/// `ChatProvider` path. A send fired mid-turn replaces the in-flight
/// session — the old worker thread drains harmlessly once its channel
/// receiver drops.
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
    // TS parity (ai-chat-handlers.ts:560-679): builtin / ACP entries
    // take their own early-return paths; ONLY external CLI providers
    // run the standard-mode classify → modify/new/chat pipeline.
    let is_builtin_or_acp = host
        .editor_state()
        .chat
        .selected_model_entry()
        .map(|entry| entry.builtin_provider_id.is_some() || entry.acp_agent_id().is_some())
        .unwrap_or(false);
    if !is_builtin_or_acp {
        if launch_cli_standard_turn(host, &user_text, current_chat, current_design) {
            return true;
        }
        // CLI transport construction failed — fall through to the
        // honest-error path below.
    } else if should_launch_direct_modify(host.editor_state(), &user_text) {
        if launch_direct_modify_turn(host, &user_text, current_chat, current_design) {
            return true;
        }
    } else if matches!(classify_intent(&user_text), Intent::Design) {
        // Phase 2.3: When the design-agent-loop flag is ON and a built-in
        // provider is configured, run the agentic tool-loop with the 14-tool
        // design toolset instead of the orchestrator pipeline. Flag OFF falls
        // through to the orchestrator path below — byte-for-byte unchanged.
        if launch_design_loop_turn(host, user_text.clone(), current_chat, current_design) {
            return true;
        }
        // Orchestrator path — unchanged when flag is OFF or no built-in
        // design provider is available. The append context the TS tool path
        // detects (agent-tool-executor.ts:234) rides the request here.
        if let Some(provider) = provider_for_selected_model(host) {
            *current_chat = None;
            // Share one provider Arc between the design LLM and the
            // (flag-gated) Class-C vision validator so the real loop reuses
            // the user's selected auth/model.
            let provider_arc: Arc<dyn ChatProvider> = Arc::from(provider);
            let llm = ChatProviderLlmClient::new(provider_arc.clone())
                .with_model(selected_cli_model_id(host));
            if clear_fresh_starter_frame_for_design(host.editor_state_mut()) {
                host.mark_editor_state_dirty();
            }
            let append_context = op_host_services::chat_intent::detect_append_intent(
                host.editor_state(),
                &user_text,
            );
            let initial_state = host.editor_state().clone();
            let request = build_design_request(user_text, &initial_state, append_context);
            *current_design = Some(op_host_services::design_session::start(
                llm,
                request,
                initial_state,
                Some(provider_arc),
            ));
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
    // Per-turn context (GAP #31): prior transcript turns, trimmed by
    // the TS sliding-window policy. `begin_send` already pushed this
    // turn's user message + empty assistant bubble — the transcript
    // mapper excludes both.
    let history = trim_chat_history(
        &chat_history_from_transcript(&host.editor_state().chat.messages),
        DEFAULT_MAX_MESSAGES,
        DEFAULT_MAX_CHARS,
    );
    // Builtin (API-key) providers run the tool-executing agent loop
    // (GAP #32): canvas tool defs ride the request and the UI thread
    // executes each call via the session's tool channel.
    if let Some((provider, tool_rx)) = builtin_provider_with_tools(host) {
        let system_prompt = build_agent_system_prompt(host.editor_state());
        // This builtin agent loop carries the full canvas toolset (`batch_design`
        // included), so a design request runs *here* for an API-key model like
        // glm-5.2 (experimental flag off → no design-agent loop, builtin → no CLI
        // provider). Reasoning models burn their whole budget on hidden `<think>`
        // and draw nothing with thinking left on (glm-5.2 measured thinking≈30k /
        // text=0 → empty Frame). Force it off for `thinking_disabled` models, same
        // as the design-agent loop. Resolved before the `&mut` borrow below.
        let thinking = launch_design::design_turn_thinking_mode(host);
        let chat = &mut host.editor_state_mut().chat;
        let effort = chat.effort_level;
        let attachments = std::mem::take(&mut chat.pending_attachments);
        let req = ChatRequest {
            system_prompt,
            user_message: user_text,
            history,
            max_output_tokens: 4096,
            thinking,
            effort,
            attachments,
            // Built-in entries carry their model inside the provider's
            // own config — see `selected_cli_model_id`.
            model: None,
        };
        *current_chat = Some(ChatSession::start_with_tools(provider, req, Some(tool_rx)));
        return true;
    }
    let Some(provider) = chat_provider_for_selected_model(host) else {
        // Selected agent has no `ChatProvider` bridge (all five CLI
        // agents are wired today, so this is a stale-index / not-ready
        // builtin/ACP guard). Surface that honestly in the assistant
        // bubble instead of silently running a different agent (codex
        // stop-gate: silent reroute to Claude misled the user about
        // which CLI answered).
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
                "error: {name} chat is not available — no transport \
                 could be built for this selection. Pick another agent \
                 via the model chip."
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
    // this turn only. Every turn now carries the context-rich chat
    // system prompt (TS buildChatSystemPrompt port) — CLI transports
    // fold it (plus a history digest) into their prompt string.
    let system_prompt = build_chat_system_prompt(host.editor_state(), &user_text);
    let model = selected_cli_model_id(host);
    let chat = &mut host.editor_state_mut().chat;
    let thinking = chat.thinking_mode;
    let effort = chat.effort_level;
    let attachments = std::mem::take(&mut chat.pending_attachments);
    let req = ChatRequest {
        system_prompt,
        user_message: user_text,
        history,
        max_output_tokens: 4096,
        thinking,
        effort,
        attachments,
        model,
    };
    *current_chat = Some(ChatSession::start(provider, req));
    true
}

fn should_launch_direct_modify(state: &EditorState, user_text: &str) -> bool {
    // A whole-screen draw request ("继续画一下 search 页面") must reach the
    // design pipeline's new-frame route, never get hijacked into editing the
    // existing frame in place — even when it also trips the modify classifier.
    if op_host_services::chat_intent::requests_new_whole_screen(user_text) {
        return false;
    }
    let keyword_intent = op_host_services::chat_intent::classify_by_keywords(user_text);
    let selected_target_instruction = !state.selection.set.is_empty()
        && keyword_intent != op_host_services::chat_intent::DesignIntent::Chat;
    (keyword_intent == op_host_services::chat_intent::DesignIntent::Modify
        || selected_target_instruction)
        && op_host_services::chat_intent::build_modify_plan(state, user_text).is_some()
}

fn launch_direct_modify_turn(
    host: &mut WidgetHostNative,
    user_text: &str,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    let Some(provider) = provider_for_selected_model(host) else {
        return false;
    };
    let Some(plan) =
        op_host_services::chat_intent::build_modify_plan(host.editor_state(), user_text)
    else {
        return false;
    };
    let request = ChatRequest {
        system_prompt: plan.system_prompt,
        user_message: plan.user_message,
        max_output_tokens: 8192,
        model: selected_cli_model_id(host),
        // Structured-JSON turn: reasoning models (MiniMax-M3, GLM-5.x)
        // burn the whole output budget inside <think> and emit zero
        // nodes (measured: an M3 modify turn died in analysis prose).
        // Same policy as the orchestrator's design subtasks.
        thinking: op_ai::chat_provider::ThinkingMode::Disabled,
        ..Default::default()
    };
    let (chat_tx, chat_rx) = mpsc::channel::<ChatDelta>();
    let (executor, tool_rx) = chat_tool_channel();
    *current_design = None;
    *current_chat = Some(ChatSession::from_channels(chat_rx, Some(tool_rx)));
    thread::Builder::new()
        .name("op-chat-modify".into())
        .spawn(move || {
            op_host_services::chat_intent::run_modify_turn(
                provider.as_ref(),
                request,
                &chat_tx,
                &executor,
            );
        })
        .expect("spawn op-chat-modify thread");
    true
}

/// Launch a CLI standard-mode turn (GAP #33): pre-build every route's
/// inputs + channels on the UI thread, park a `ChatSession` and a
/// `DesignSession` (mirroring `from_channels` docs), and hand the
/// route decision to `chat_intent::run_cli_turn` on a worker thread —
/// classification needs an LLM round-trip and must not block the UI.
///
/// Returns false when any transport fails to build (caller falls to
/// the honest-error path).
fn launch_cli_standard_turn(
    host: &mut WidgetHostNative,
    user_text: &str,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    // All three transports up front: classification + design run
    // session-untracked (TS classify/generate calls never join the
    // chat conversation); the chat route resumes the chat session.
    let (Some(classify_provider), Some(chat_provider), Some(design_provider)) = (
        provider_for_selected_model(host),
        chat_provider_for_selected_model(host),
        provider_for_selected_model(host),
    ) else {
        return false;
    };
    let model = selected_cli_model_id(host);

    // Rust-only starter handling: classification resolves async on
    // the worker, which cannot mutate the doc — so the pristine
    // starter sample clears eagerly when the keyword pre-gate already
    // reads design intent. A keyword-design turn the LLM later
    // classifies as chat loses only the untouched starter sample.
    if matches!(classify_intent(user_text), Intent::Design)
        && clear_fresh_starter_frame_for_design(host.editor_state_mut())
    {
        host.mark_editor_state_dirty();
    }

    // Route inputs, post-clear (TS reads the live doc at this point).
    let state = host.editor_state();
    let page_children_empty = state.active_children().is_empty();
    // TS `hasSelection` folds into build_modify_plan's target pick.
    let history = trim_chat_history(
        &chat_history_from_transcript(&state.chat.messages),
        DEFAULT_MAX_MESSAGES,
        DEFAULT_MAX_CHARS,
    );
    let system_prompt = build_chat_system_prompt(state, user_text);
    let modify_plan = op_host_services::chat_intent::build_modify_plan(state, user_text);
    let append_context = op_host_services::chat_intent::detect_append_intent(state, user_text);
    let initial_state = state.clone();
    let design_request =
        build_design_request(user_text.to_string(), &initial_state, append_context);

    let chat = &mut host.editor_state_mut().chat;
    let thinking = chat.thinking_mode;
    let effort = chat.effort_level;
    let attachments = std::mem::take(&mut chat.pending_attachments);
    let chat_request = ChatRequest {
        system_prompt,
        user_message: user_text.to_string(),
        history,
        max_output_tokens: 4096,
        thinking,
        effort,
        attachments,
        model: model.clone(),
    };
    // TS generateDesignModification: fresh single-shot request — no
    // history, no attachments, provider-default thinking.
    let modify_request = modify_plan.map(|plan| ChatRequest {
        system_prompt: plan.system_prompt,
        user_message: plan.user_message,
        max_output_tokens: 8192,
        model: model.clone(),
        ..Default::default()
    });

    // Channels for all three routes; the worker drops the unused
    // ones, whose pumps then retire their sessions.
    let (chat_tx, chat_rx) = mpsc::channel::<ChatDelta>();
    let (executor, tool_rx) = chat_tool_channel();
    let (delta_tx, delta_rx) = mpsc::channel();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let indicator_epoch = op_editor_core::agent_indicators::begin();
    *current_chat = Some(ChatSession::from_channels(chat_rx, Some(tool_rx)));
    *current_design = Some(DesignSession::from_channels_with_epoch(
        delta_rx,
        cmd_rx,
        indicator_epoch,
    ));

    let plan = op_host_services::chat_intent::CliTurnPlan {
        user_text: user_text.to_string(),
        page_children_empty,
        classify_provider,
        chat_provider,
        design_provider,
        chat_request,
        modify_request,
        design_request,
        initial_state,
        indicator_epoch,
        model,
    };
    thread::Builder::new()
        .name("op-chat-intent".into())
        .spawn(move || {
            op_host_services::chat_intent::run_cli_turn(plan, chat_tx, executor, delta_tx, cmd_tx);
        })
        .expect("spawn op-chat-intent thread");
    true
}

/// Drain a New Chat request raised by the widget layer.
///
/// The widget handler ([`crate::widget_host`] `AIChatHit::NewChat`) already
/// pushed the fresh tab via `ChatSessions::new_tab` — the active tab keeps its
/// history intact while the new tab starts blank. This drain only does the
/// host-side worker cleanup the widget layer cannot reach: drop any in-flight
/// workers (so stale deltas can't repopulate the previous tab's transcript)
/// and forget any resumable provider session.
///
/// Returns the index of the tab a still-running turn was bound to, if any, so
/// the caller can clear its `chat_running_tab` field (the run we just aborted
/// must no longer target any tab).
pub fn drain_new_chat_request(
    host: &mut WidgetHostNative,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    if !std::mem::take(&mut host.editor_state_mut().chat.pending_new_chat) {
        return false;
    }
    // The fresh tab was already opened by the widget handler — do NOT push a
    // second one here (one "+" click == one new tab).
    *current_chat = None;
    *current_design = None;
    if let Some(epoch) = op_editor_core::agent_indicators::active_epoch() {
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    }
    // A fresh tab must start a fresh provider conversation — forget any
    // resumable Claude Code / Copilot session so stale context cannot
    // leak into the new chat.
    op_host_services::chat_claude::reset_claude_chat_session();
    op_host_services::chat_copilot::reset_copilot_chat_session();
    host.mark_editor_state_dirty();
    true
}

/// Drain a Stop request raised by the widget layer. The transcript
/// has already had its streaming flags cleared; this only drops the
/// in-flight workers so stale deltas cannot append after cancellation.
pub fn drain_stop_request(
    host: &mut WidgetHostNative,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    if !std::mem::take(&mut host.editor_state_mut().chat.pending_stop_chat) {
        return false;
    }
    *current_chat = None;
    *current_design = None;
    if let Some(epoch) = op_editor_core::agent_indicators::active_epoch() {
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    }
    host.mark_editor_state_dirty();
    true
}

pub(crate) fn clear_fresh_starter_frame_for_design(state: &mut EditorState) -> bool {
    if !active_page_is_blank_starter_frame(state) {
        return false;
    }
    state.active_children_mut().clear();
    state.clear_selection();
    true
}

fn active_page_is_blank_starter_frame(state: &EditorState) -> bool {
    let children = state.active_children();
    let [only] = children else {
        return false;
    };
    is_blank_starter_frame(only)
}

fn is_blank_starter_frame(node: &jian_ops_schema::node::PenNode) -> bool {
    let jian_ops_schema::node::PenNode::Frame(frame) = node else {
        return false;
    };
    if frame.base.id != "n10" || frame.base.name.as_deref() != Some("Frame") {
        return false;
    }
    if frame
        .children
        .as_ref()
        .map(|c| !c.is_empty())
        .unwrap_or(false)
    {
        return false;
    }
    if !near(frame.base.x.unwrap_or(0.0), 0.0) || !near(frame.base.y.unwrap_or(0.0), 0.0) {
        return false;
    }
    if !matches_number(&frame.container.width, 1200.0)
        || !matches_number(&frame.container.height, 800.0)
    {
        return false;
    }
    if frame.container.stroke.is_some() {
        return false;
    }
    match frame.container.fill.as_deref() {
        Some([jian_ops_schema::style::PenFill::Solid(fill)]) => {
            fill.color.eq_ignore_ascii_case("#ffffff")
        }
        _ => false,
    }
}

fn matches_number(value: &Option<jian_ops_schema::sizing::SizingBehavior>, expected: f64) -> bool {
    matches!(
        value,
        Some(jian_ops_schema::sizing::SizingBehavior::Number(n)) if near(*n, expected)
    )
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 0.01
}

/// Build the `ChatProvider` for an agent index (into
/// `AgentProvider::ALL`: 0 ClaudeCode, 1 CodexCli, 2 OpenCode,
/// 3 GithubCopilot, 4 GeminiCli). Claude Code uses its dedicated
/// SDK adapter; Codex / Gemini use the subprocess transport; Copilot
/// rides the official SDK; OpenCode chats over its local HTTP server
/// (`chat_http_server.rs`).
///
/// `chat_session` opts the Claude Code and Copilot adapters into
/// their process-wide chat resume slots (multi-turn context via
/// `--resume` / `session.resume`). The design / codegen paths pass
/// `false` so their sub-requests never join — or pollute — the
/// user's chat conversation. OpenCode opens a fresh server session
/// per turn (TS parity: `streamViaOpenCode` never resumes), so it
/// has no chat/non-chat split.
fn provider_for_agent(agent_idx: usize, chat_session: bool) -> Option<Box<dyn ChatProvider>> {
    match agent_idx {
        0 => Some(Box::new(if chat_session {
            ClaudeCodeProvider::for_chat()
        } else {
            ClaudeCodeProvider::new()
        })),
        1 => SubprocessProvider::for_cli(CliName::Codex)
            .map(|p| Box::new(p) as Box<dyn ChatProvider>),
        2 => Some(Box::new(OpenCodeProvider::new())),
        3 => Some(Box::new(if chat_session {
            CopilotProvider::for_chat()
        } else {
            CopilotProvider::new()
        })),
        4 => SubprocessProvider::for_cli(CliName::Gemini)
            .map(|p| Box::new(p) as Box<dyn ChatProvider>),
        _ => None,
    }
}

/// Provider for non-chat consumers (design orchestrator LLM, codegen)
/// — session-untracked so their requests stay out of the user's chat
/// conversation.
pub(crate) fn provider_for_selected_model(
    host: &WidgetHostNative,
) -> Option<Box<dyn ChatProvider>> {
    provider_for_selected_model_impl(host, false)
}

/// Provider for the chat panel's own turns — Claude Code / Copilot
/// resume their process-wide chat sessions across sends.
fn chat_provider_for_selected_model(host: &WidgetHostNative) -> Option<Box<dyn ChatProvider>> {
    provider_for_selected_model_impl(host, true)
}

fn provider_for_selected_model_impl(
    host: &WidgetHostNative,
    chat_session: bool,
) -> Option<Box<dyn ChatProvider>> {
    if let Some(entry) = host.editor_state().chat.selected_model_entry() {
        if let Some(id) = entry.builtin_provider_id.as_deref() {
            return provider_for_builtin(host.editor_state(), id);
        }
        if let Some(id) = entry.acp_agent_id() {
            return provider_for_acp(host.editor_state(), id);
        }
    }
    provider_for_agent(
        host.editor_state().editor_ui.chat_selected_agent,
        chat_session,
    )
}

/// When the selected chat model is a ready builtin (API-key) entry,
/// build its provider with the canvas tool set + a fresh UI tool
/// channel — the GAP #32 tool-executing path. `None` falls through to
/// the plain provider routing.
pub(crate) fn builtin_provider_with_tools(
    host: &WidgetHostNative,
) -> Option<(Box<dyn ChatProvider>, Receiver<ChatToolRequest>)> {
    let state = host.editor_state();
    let entry = state.chat.selected_model_entry()?;
    let id = entry.builtin_provider_id.as_deref()?;
    let config = state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .find(|agent| agent.id == id && agent.ready())?;
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(config)?;
    let (executor, tool_rx) = chat_tool_channel();
    let provider = provider.with_canvas_tools(chat_tool_defs(), Arc::new(executor));
    Some((Box::new(provider), tool_rx))
}

/// Model id to forward to the routed CLI transport, from the chat
/// panel's selected model entry (`chat.available_models[selected_model]`).
///
/// Only CLI-backed entries qualify:
/// - built-in entries (`builtin_provider_id`) carry their model inside
///   `ConfiguredBuiltinProvider`'s own config (`entry.value` is the
///   composite `builtin:<id>:<model>`, not a wire id);
/// - ACP entries (`acp:<id>`) address an agent, not a model;
/// - an entry whose provider differs from the agent actually routed by
///   [`provider_for_agent`] must not leak its id to a different CLI
///   (selection sync normally keeps them aligned, but a stale index
///   after a rebuild could diverge).
///
/// Blank ids collapse to `None` so transports never emit an empty
/// model flag.
pub(crate) fn selected_cli_model_id(host: &WidgetHostNative) -> Option<String> {
    let state = host.editor_state();
    let entry = state.chat.selected_model_entry()?;
    if entry.builtin_provider_id.is_some() || entry.acp_agent_id().is_some() {
        return None;
    }
    let routed = op_editor_core::AgentProvider::ALL.get(state.editor_ui.chat_selected_agent)?;
    if *routed != entry.provider {
        return None;
    }
    let value = entry.value.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.to_string())
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

fn provider_for_acp(state: &EditorState, id: &str) -> Option<Box<dyn ChatProvider>> {
    let settings = &state.editor_ui.agent_settings;
    let config = settings.acp_agents.iter().find(|agent| {
        agent.id == id && agent.ready() && settings.acp_agent_verified_connected(&agent.id)
    })?;
    // Canvas tool surface for the agent (TS parity, agent.ts:503-521):
    // the live MCP server's HTTP endpoint when it is running. `None`
    // makes the provider refuse the turn with the TS error message.
    let mcp = state.editor_ui.agent_settings.mcp_server;
    let live_mcp_port = mcp.running.then_some(mcp.port);
    Some(Box::new(AcpProvider::new(
        acp_config_for_provider(config),
        live_mcp_port,
    )))
}

fn acp_config_for_provider(agent: &op_editor_core::AcpAgentConfig) -> op_acp::AcpAgentConfig {
    op_acp::AcpAgentConfig {
        id: agent.id.clone(),
        display_name: agent.display_name.clone(),
        connection_type: match agent.connection_type {
            op_editor_core::AcpConnectionType::Local => op_acp::ConnectionType::Local,
            op_editor_core::AcpConnectionType::Remote => op_acp::ConnectionType::Remote,
        },
        command: match agent.connection_type {
            op_editor_core::AcpConnectionType::Local => Some(agent.command.clone()),
            op_editor_core::AcpConnectionType::Remote => None,
        },
        args: agent.args.clone(),
        env: agent.env.clone(),
        url: agent.url.clone(),
        enabled: agent.enabled,
    }
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
        if let Some(id) = entry.acp_agent_id() {
            if let Some(agent) = host
                .editor_state()
                .editor_ui
                .agent_settings
                .acp_agents
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

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::pen_node_ext::PenNodeExt;

    fn frame(
        id: &str,
        name: &str,
        children: Vec<jian_ops_schema::node::PenNode>,
    ) -> jian_ops_schema::node::PenNode {
        let mut node: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": id,
            "name": name,
            "width": 390,
            "height": 120,
            "children": []
        }))
        .expect("frame fixture");
        if let Some(kids) = node.children_mut() {
            *kids = children;
        }
        node
    }

    #[test]
    fn builtin_design_keyword_with_existing_target_prefers_modify_route() {
        let mut state = EditorState::new();
        state.active_children_mut().clear();
        state.active_children_mut().push(frame(
            "screen",
            "Food App Home",
            vec![frame("popular-card", "Bella Napoli Pizzeria", Vec::new())],
        ));
        state.set_single_selection(op_editor_core::NodeId::new("popular-card"));

        assert!(
            should_launch_direct_modify(&state, "修改成饺子"),
            "selected existing design + modify wording should update in place, not start a new orchestrator design"
        );
    }
}

#[cfg(test)]
#[path = "chat_session_launch_selection_tests.rs"]
mod selection_tests;
