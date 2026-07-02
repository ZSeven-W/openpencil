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
use op_host_services::chat_builtin_http::ConfiguredBuiltinProvider;
use op_host_services::chat_canvas_tools::{chat_tool_channel, ChatToolRequest};
use op_host_services::chat_system_prompt::chat_history_from_transcript;
use op_host_services::design_agent_tools::design_tool_defs;

use super::clear_fresh_starter_frame_for_design;

/// Parse the design-agent-loop env-flag value without reading the env
/// directly — testable without env-var flakiness. Accepts "1", "true",
/// and "on" (after trimming); everything else (including `None`) → false.
pub fn parse_loop_flag(opt: Option<&str>) -> bool {
    matches!(opt.map(str::trim), Some("1") | Some("true") | Some("on"))
}

/// Pure predicate for the design-agent-loop gate.
///
/// Returns true when EITHER:
/// - `experimental` is true (Settings → System "Experimental features" toggle), OR
/// - `env` is a recognised truthy env-var value ("1" / "true" / "on").
///
/// Kept free of I/O so it is unit-testable without env-var flakiness.
pub fn loop_enabled(experimental: bool, env: Option<&str>) -> bool {
    experimental || parse_loop_flag(env)
}

/// Returns true when the design-agent loop should run for this turn.
///
/// Combines the in-app Settings → System "Experimental features" toggle
/// (`state.editor_ui.agent_settings.experimental_features_enabled`) with
/// the `OPENPENCIL_DESIGN_AGENT_LOOP` env var — either one being set is
/// sufficient. Both off → false, preserving the default orchestrator path.
pub(super) fn design_agent_loop_enabled(state: &op_editor_core::EditorState) -> bool {
    loop_enabled(
        state.editor_ui.agent_settings.experimental_features_enabled,
        std::env::var("OPENPENCIL_DESIGN_AGENT_LOOP")
            .ok()
            .as_deref(),
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
/// design toolset and a 16384-token per-turn budget.
pub(super) fn launch_design_loop_turn(
    host: &mut WidgetHostNative,
    user_text: String,
    current_chat: &mut Option<ChatSession>,
    current_design: &mut Option<DesignSession>,
) -> bool {
    if !design_agent_loop_enabled(host.editor_state()) {
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
    let chat = &mut host.editor_state_mut().chat;
    let effort = chat.effort_level;
    let attachments = std::mem::take(&mut chat.pending_attachments);
    let req = ChatRequest {
        system_prompt: op_ai_skills::design_agent_system_prompt().to_string(),
        user_message: user_text,
        history,
        // Per-TURN output cap. The real budget-burner was hidden reasoning, not
        // the DSL: a glm-5.2 loop run streamed ~94k thinking chars vs ~4k of
        // actual batch_design, blowing the cap so a turn truncated mid-JSON, its
        // tool call failed to parse, and the loop stopped early with an
        // unfinished design. The root fix is at the wire (the loop now sends
        // thinking:{type:disabled} for glm/minimax); 16384 (up from 8192) is
        // headroom so a genuinely large section-batch still completes per turn.
        max_output_tokens: 16384,
        thinking,
        effort,
        attachments,
        model: None,
    };
    *current_chat = Some(ChatSession::start_with_tools(provider, req, Some(tool_rx)));
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

    // ── loop_enabled OR-gate: 4 combinations ──────────────────────────────
    #[test]
    fn loop_enabled_both_off_is_false() {
        assert!(!loop_enabled(false, None), "both off must yield false");
    }

    #[test]
    fn loop_enabled_experimental_on_is_true() {
        assert!(
            loop_enabled(true, None),
            "experimental=true with no env var must yield true"
        );
    }

    #[test]
    fn loop_enabled_env_on_is_true() {
        assert!(
            loop_enabled(false, Some("1")),
            "experimental=false but env=1 must yield true"
        );
    }

    #[test]
    fn loop_enabled_both_on_is_true() {
        assert!(
            loop_enabled(true, Some("0")),
            "experimental=true overrides env=0"
        );
    }

    // ── parse_loop_flag (unchanged) ───────────────────────────────────────

    #[test]
    fn parse_loop_flag_returns_false_when_unset() {
        assert!(!parse_loop_flag(None), "unset env var must yield false");
    }

    #[test]
    fn parse_loop_flag_returns_true_for_accepted_values() {
        assert!(parse_loop_flag(Some("1")));
        assert!(parse_loop_flag(Some("true")));
        assert!(parse_loop_flag(Some("on")));
        // Whitespace around the value must be stripped.
        assert!(parse_loop_flag(Some("  1  ")));
        assert!(parse_loop_flag(Some(" true ")));
        assert!(parse_loop_flag(Some(" on ")));
    }

    #[test]
    fn parse_loop_flag_returns_false_for_rejected_values() {
        assert!(!parse_loop_flag(Some("0")));
        assert!(!parse_loop_flag(Some("false")));
        assert!(!parse_loop_flag(Some("off")));
        assert!(!parse_loop_flag(Some("")));
        assert!(!parse_loop_flag(Some("yes")));
        assert!(!parse_loop_flag(Some("True"))); // case-sensitive
        assert!(!parse_loop_flag(Some("ON"))); // case-sensitive
    }
}
