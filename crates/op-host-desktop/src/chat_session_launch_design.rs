//! Design-agent-loop helpers split out of `chat_session_launch.rs` at the
//! 800-line cap. Provides the env-flag gate, the design-toolset provider
//! builder, and the `launch_design_loop_turn` entry point called from the
//! `Intent::Design` arm inside `launch_if_pending`.
//!
//! See `chat_session_launch.rs` for routing context.

use std::sync::mpsc::Receiver;
use std::sync::Arc;

use op_ai::chat_history::{trim_chat_history, DEFAULT_MAX_CHARS, DEFAULT_MAX_MESSAGES};
use op_ai::chat_provider::{ChatProvider, ChatRequest, ThinkingMode};
use op_editor_host_core::chat::ChatSession;
use op_editor_host_core::design::DesignSession;
use op_host_native::WidgetHostNative;
use op_host_services::chat_builtin_http::{
    ConfiguredBuiltinProvider, DESIGN_LOOP_MAX_OUTPUT_TOKENS,
};
use op_host_services::chat_canvas_tools::{chat_tool_channel, ChatToolRequest};
use op_host_services::chat_system_prompt::chat_history_from_transcript;
use op_host_services::design_agent_tools::{design_tool_defs, root_seed_prompt_is_mobile};

use super::clear_fresh_starter_frame_for_design;

/// Parses recognized force-loop / force-orchestrator environment values.
fn parse_loop_env(opt: Option<&str>) -> Option<bool> {
    match opt.map(str::trim) {
        Some("1" | "true" | "on") => Some(true),
        Some("0" | "false" | "off") => Some(false),
        _ => None,
    }
}

/// Returns true when the prompt describes a landing or marketing page.
fn prompt_is_landing(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    let words = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let is_desktop_product = words.iter().any(|word| {
        matches!(
            *word,
            "dashboard" | "admin" | "console" | "desktop" | "webapp"
        )
    }) || words.windows(2).any(|pair| pair == ["web", "app"])
        || lower.contains("后台");
    if is_desktop_product {
        return false;
    }

    words.iter().any(|word| {
        matches!(
            *word,
            "landing" | "website" | "homepage" | "marketing" | "hero"
        )
    }) || ["官网", "落地页", "营销"]
        .iter()
        .any(|keyword| lower.contains(keyword))
}

/// Pure predicate for the design-agent-loop gate.
///
/// Routing has three states: a truthy env value or the Settings experimental
/// toggle forces the loop for every prompt; when neither force-loop condition
/// applies, a falsy env value forces the single-shot orchestrator; otherwise
/// mobile and landing prompts auto-route to the loop while desktop dashboard,
/// web-app, and other prompts stay on the orchestrator.
///
/// The A/B evidence summarized in the openpencil-docs sonar plan validated the
/// loop as better for mobile/landing work and worse for desktop dashboard work.
///
/// Kept free of I/O so it is unit-testable without env-var flakiness.
pub fn loop_enabled(experimental: bool, env: Option<&str>, prompt: &str) -> bool {
    if experimental {
        return true;
    }
    if let Some(force_loop) = parse_loop_env(env) {
        return force_loop;
    }
    root_seed_prompt_is_mobile(prompt) || prompt_is_landing(prompt)
}

/// Returns true when the design-agent loop should run for this turn.
///
/// Explicit settings force one path; otherwise mobile and landing prompts use
/// the loop. CLI providers never reach this gate — they stay on the orchestrator
/// path in `launch_if_pending`.
pub(super) fn design_agent_loop_enabled(state: &op_editor_core::EditorState, prompt: &str) -> bool {
    loop_enabled(
        state.editor_ui.agent_settings.experimental_features_enabled,
        std::env::var("OPENPENCIL_DESIGN_AGENT_LOOP")
            .ok()
            .as_deref(),
        prompt,
    )
}

/// When the selected chat model is a ready builtin (API-key) entry,
/// build its provider with the design toolset + a fresh UI tool channel.
/// Returns `None` when no builtin is configured or ready — caller falls
/// through to the orchestrator path.
pub(crate) fn builtin_provider_with_design_tools(
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
    let provider = provider
        .with_canvas_tools(design_tool_defs(), Arc::new(executor))
        .with_loop_finalize();
    Some((Box::new(provider), tool_rx))
}

/// Effective thinking mode for a design turn (design-agent loop and
/// orchestrator sub-agent spawns alike).
///
/// The design pipeline is structured generation, not free chat: reasoning
/// models whose profile marks `thinking_disabled` (glm-5.x / minimax / …)
/// burn their whole token budget on hidden `<think>` and emit an *empty*
/// design when thinking is left on — glm-5.2 measured at thinking≈30k /
/// text=0 → nothing drawn. Force those to `Disabled`; the wire layer
/// (`chat_builtin_http`) then sends `thinking:{type:"disabled"}`. Claude and
/// other non-`thinking_disabled` models keep the chat's default — they use
/// thinking productively without starving content.
pub(crate) fn design_turn_thinking_mode(host: &WidgetHostNative) -> ThinkingMode {
    let state = host.editor_state();
    let model = state
        .chat
        .selected_model_entry()
        .and_then(|e| e.builtin_provider_id.as_deref())
        .and_then(|id| {
            state
                .editor_ui
                .agent_settings
                .builtin_agents
                .iter()
                .find(|a| a.id == id)
                .map(|a| a.model.as_str())
        });
    resolve_design_thinking(model, state.chat.thinking_mode)
}

/// Pure decision behind [`design_turn_thinking_mode`]: a model whose profile
/// is `thinking_disabled` is forced to `Disabled` for the design turn;
/// everything else (unknown model included → keep the user's choice) keeps the
/// chat default. Split out so the policy is unit-testable without a host.
fn resolve_design_thinking(model: Option<&str>, chat_default: ThinkingMode) -> ThinkingMode {
    let thinking_disabled = model
        .map(|m| op_orchestrator::resolve_model_profile(m).thinking_disabled)
        .unwrap_or(false);
    if thinking_disabled {
        ThinkingMode::Disabled
    } else {
        chat_default
    }
}

/// Launch the design-agent tool-loop turn when the flag is ON and a
/// built-in design provider is available. Returns true when the turn was
/// launched; false when the flag is OFF or no builtin is ready (caller
/// falls through to the orchestrator path).
///
/// Mirrors `launch_if_pending`'s builtin chat branch but uses the
/// design toolset and a section-batch-sized per-turn budget.
pub(super) fn launch_design_loop_turn(
    host: &mut WidgetHostNative,
    user_text: String,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    if !design_agent_loop_enabled(host.editor_state(), &user_text) {
        return false;
    }
    let Some((provider, tool_rx)) = builtin_provider_with_design_tools(host) else {
        return false;
    };
    *current_chat = None;
    *current_design = None;
    if clear_fresh_starter_frame_for_design(host.editor_state_mut()) {
        host.mark_editor_state_dirty();
    }
    let history = trim_chat_history(
        &chat_history_from_transcript(&host.editor_state().chat.messages),
        DEFAULT_MAX_MESSAGES,
        DEFAULT_MAX_CHARS,
    );
    // Force thinking off for reasoning models that would otherwise emit an
    // empty design (see `design_turn_thinking_mode`). Resolved before the
    // `&mut` borrow below.
    let thinking = design_turn_thinking_mode(host);
    let root_seed_mobile = root_seed_prompt_is_mobile(&user_text);
    let chat = &mut host.editor_state_mut().chat;
    let effort = chat.effort_level;
    let attachments = std::mem::take(&mut chat.pending_attachments);
    // Protocol base + prompt-matched domain depth (dashboard density floors,
    // mobile three-section architecture, …) — the same content supply the
    // orchestrator injects per subtask. Without it the loop model designs
    // from the 181-line protocol prompt alone and ships sparse screens (the
    // measured p14-type richness gap). On a non-empty canvas the consistency
    // brief rides along so screen 2+ reads as the same product (shared chrome
    // node ids, palette, typefaces).
    let mut system_prompt = op_ai_skills::design_agent_system_prompt_with_skills(&user_text);
    if let Some(brief) = op_host_services::design_context::design_context_brief(host.editor_state())
    {
        system_prompt.push_str("\n\n---\n\n");
        system_prompt.push_str(&brief);
        op_ai_skills::append_image_self_check_scope(&mut system_prompt);
    }
    let req = ChatRequest {
        system_prompt,
        user_message: user_text,
        history,
        // Design-loop turns should emit one <=25-op section batch, then wait for
        // tool feedback. 6144 matches that 4-6k-token batch size and avoids
        // rewarding monolithic whole-screen tool calls.
        max_output_tokens: DESIGN_LOOP_MAX_OUTPUT_TOKENS,
        thinking,
        effort,
        attachments,
        model: None,
    };
    // The host starts the indicator epoch before the worker can apply its first
    // design batch; the indicator pump adopts this epoch for badges/teardown.
    op_editor_core::agent_indicators::begin_with_root_seed_hint(root_seed_mobile);
    *current_chat =
        Some(ChatSession::start_with_tools(provider, req, Some(tool_rx)).into_design_loop());
    // Signal one agent running so the chat header shows "1/1 designing…"
    // and the canvas indicator pump registers frame glows.
    host.editor_state_mut().chat.agents_running = (1, 1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── design-turn thinking policy ───────────────────────────────────────
    #[test]
    fn reasoning_model_forces_thinking_off_for_design() {
        // glm-5.2 is `thinking_disabled` in the profile: with thinking left on
        // it burns its budget on `<think>` and draws nothing. The design turn
        // must override the chat default to `Disabled` regardless of choice.
        assert_eq!(
            resolve_design_thinking(Some("glm-5.2"), ThinkingMode::Adaptive),
            ThinkingMode::Disabled
        );
        assert_eq!(
            resolve_design_thinking(Some("MiniMax-M3"), ThinkingMode::Enabled),
            ThinkingMode::Disabled
        );
    }

    #[test]
    fn claude_keeps_chat_default_for_design() {
        // Claude is NOT `thinking_disabled` — it uses thinking productively
        // without starving content, so the design turn keeps the user's choice.
        assert_eq!(
            resolve_design_thinking(Some("claude-opus-4"), ThinkingMode::Adaptive),
            ThinkingMode::Adaptive
        );
        assert_eq!(
            resolve_design_thinking(Some("claude-sonnet-4-6"), ThinkingMode::Enabled),
            ThinkingMode::Enabled
        );
    }

    #[test]
    fn unknown_or_absent_model_keeps_chat_default() {
        // No selected builtin → keep the user's choice (don't silently disable).
        assert_eq!(
            resolve_design_thinking(None, ThinkingMode::Enabled),
            ThinkingMode::Enabled
        );
    }

    // ── prompt-aware design-loop routing ───────────────────────────
    #[test]
    fn loop_enabled_explicit_on_routes_any_prompt_to_loop() {
        let dashboard = "Design an admin analytics dashboard web app";
        assert!(loop_enabled(true, None, dashboard));
        assert!(loop_enabled(false, Some("1"), dashboard));
        assert!(loop_enabled(false, Some("true"), dashboard));
        assert!(loop_enabled(false, Some(" on "), dashboard));
    }

    #[test]
    fn loop_enabled_explicit_off_routes_any_prompt_to_orchestrator() {
        let mobile = "Design a mobile fitness app home";
        assert!(!loop_enabled(false, Some("0"), mobile));
        assert!(!loop_enabled(false, Some("false"), mobile));
        assert!(!loop_enabled(false, Some(" off "), mobile));
    }

    #[test]
    fn loop_enabled_auto_routes_mobile_prompt_to_loop() {
        assert!(loop_enabled(
            false,
            None,
            "Design a mobile fitness app home"
        ));
    }

    #[test]
    fn loop_enabled_auto_routes_landing_prompt_to_loop() {
        assert!(loop_enabled(
            false,
            None,
            "Design a landing page for a climate SaaS"
        ));
    }

    #[test]
    fn loop_enabled_auto_routes_dashboard_prompt_to_orchestrator() {
        assert!(!loop_enabled(
            false,
            None,
            "Design an admin analytics dashboard web app"
        ));
    }

    #[test]
    fn loop_enabled_auto_routes_ambiguous_homepage_to_orchestrator() {
        assert!(!loop_enabled(
            false,
            None,
            "Design the dashboard homepage for admin console"
        ));
    }

    #[test]
    fn loop_enabled_auto_routes_web_app_homepage_to_orchestrator() {
        assert!(!loop_enabled(
            false,
            None,
            "Design the homepage for a project management web app"
        ));
    }
}
