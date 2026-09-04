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
use op_host_services::design_agent_tools::{
    design_tool_defs, root_seed_prompt_is_continuation, root_seed_prompt_is_mobile,
};

use op_editor_core::scene_template_catalog::TemplateScene;

use super::clear_fresh_starter_frame_for_design;
use super::providers::selected_builtin_agent_config;

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

/// E-lite: the loop-path system-prompt fact naming the user's
/// `chat.agent_team_size` (⚡Nx) setting, so the model actually knows how
/// many parallel agents it's allowed before deciding whether to
/// `spawn_agents` for a multi-screen request — the design-agent.md
/// spawn_agents guidance no longer uses a fixed screen-count threshold, it
/// asks whether "the user's parallel-agents setting allows it", which is
/// meaningless unless the model is TOLD the setting. `None` at the default
/// of 1 — a team of one has nothing to parallelize, so the fact would only
/// add noise for the common case.
fn team_size_fact(agent_team_size: u32) -> Option<String> {
    (agent_team_size > 1).then(|| format!("User's parallel-agents setting: {agent_team_size}"))
}

/// Screen-shaped top-level frame width band — mirrors
/// `op_orchestrator::wire_screen_navigation::MOBILE_SCREEN_WIDTH`. Kept as a
/// small local duplicate rather than an import: this is a host-side ROUTING
/// heuristic at a different architectural layer than the orchestrator's
/// generation-time screen-wiring pass, and reaching into that pass's
/// internals for one width check would be the wrong dependency direction.
const MOBILE_SCREEN_WIDTH: std::ops::RangeInclusive<f64> = 320.0..=480.0;

/// True when the active page already holds a screen-shaped mobile-width
/// top-level frame — i.e. the canvas is already an in-progress mobile app,
/// independent of what the CURRENT turn's prompt text says.
///
/// A pure-Chinese continuation prompt ("继续生成剩下3个页面") carries no
/// ASCII mobile keyword `root_seed_prompt_is_mobile` can match, so without
/// this canvas signal a mobile multi-screen continuation silently misroutes
/// to the classic orchestrator (see the multi-screen fan-out break plan in
/// openpencil-docs).
fn canvas_has_mobile_screen_root(state: &op_editor_core::EditorState) -> bool {
    use op_editor_core::PenNodeExt;
    state.active_children().iter().any(|node| {
        matches!(node, jian_ops_schema::node::PenNode::Frame(_))
            && node
                .width_px()
                .is_some_and(|w| MOBILE_SCREEN_WIDTH.contains(&w))
    })
}

/// Mobile-root seeding must use the same canvas-aware signal as loop routing.
/// A continuation prompt can omit every mobile keyword while the existing
/// canvas still proves that the next root belongs to the same mobile app.
fn root_seed_mobile_for_turn(state: &op_editor_core::EditorState, prompt: &str) -> bool {
    canvas_has_mobile_screen_root(state) || root_seed_prompt_is_mobile(prompt)
}

/// Pure predicate for the design-agent-loop gate.
///
/// Routing has three states: a truthy env value or the Settings experimental
/// toggle forces the loop for every prompt; when neither force-loop condition
/// applies, a falsy env value forces the single-shot orchestrator; otherwise
/// mobile and landing prompts, OR a canvas that already has a mobile-width
/// screen root (`existing_mobile_root`), auto-route to the loop, while
/// desktop dashboard, web-app, and other prompts on an otherwise-empty/
/// desktop canvas stay on the orchestrator.
///
/// `existing_mobile_root` is the fix for a misroute the prompt text alone
/// can't see: a continuation turn ("继续生成剩下3个页面") has no ASCII
/// mobile keyword, but the canvas it's continuing already proves the
/// product is mobile — that fact must win regardless of what THIS prompt
/// says.
///
/// The A/B evidence summarized in the openpencil-docs sonar plan validated the
/// loop as better for mobile/landing work and worse for desktop dashboard work.
///
/// Kept free of I/O so it is unit-testable without env-var flakiness.
pub fn loop_enabled(
    experimental: bool,
    env: Option<&str>,
    prompt: &str,
    existing_mobile_root: bool,
) -> bool {
    if experimental {
        return true;
    }
    if let Some(force_loop) = parse_loop_env(env) {
        return force_loop;
    }
    existing_mobile_root || root_seed_prompt_is_mobile(prompt) || prompt_is_landing(prompt)
}

/// Returns true when the design-agent loop should run for this turn.
///
/// Explicit settings force one path; otherwise mobile and landing prompts, or
/// an existing mobile-width screen root already on the canvas, use the loop.
/// CLI providers never reach this gate — they stay on the orchestrator path
/// in `launch_if_pending`.
pub(super) fn design_agent_loop_enabled(state: &op_editor_core::EditorState, prompt: &str) -> bool {
    loop_enabled(
        state.editor_ui.agent_settings.experimental_features_enabled,
        std::env::var("OPENPENCIL_DESIGN_AGENT_LOOP")
            .ok()
            .as_deref(),
        prompt,
        canvas_has_mobile_screen_root(state),
    )
}

/// What a design turn establishes the document to be, if anything.
///
/// Two facts have to line up: the prompt asks for a deck, AND the page held
/// nothing before the turn started. The second condition is the load-bearing
/// one — generating a deck onto a canvas that already holds someone's app
/// design adds slides to their document, it does not turn their document into
/// a deck, and relabelling it would change what Preview does to work they
/// never asked us to reinterpret. Nothing else is inferred: artboard size in
/// particular says nothing, since 1920×1080 documents that are not decks are
/// ordinary.
///
/// Pure so the policy is testable without a host or a model.
pub(crate) fn design_turn_scenario(prompt: &str, page_was_empty: bool) -> Option<TemplateScene> {
    (page_was_empty
        && op_orchestrator::detect_design_type(prompt).type_ == op_orchestrator::DesignType::Slides)
        .then_some(TemplateScene::Slides)
}

/// Apply [`design_turn_scenario`] before the turn touches the canvas.
///
/// MUST run before the starter frame is cleared for the generation — after
/// that the page is empty for every prompt and the fact gate would wave
/// through decks generated onto real work. A turn that establishes nothing
/// leaves any existing tag alone.
pub(super) fn stamp_design_turn_scenario(state: &mut op_editor_core::EditorState, prompt: &str) {
    let page_was_empty =
        state.active_children().is_empty() || super::active_page_is_blank_starter_frame(state);
    if let Some(scenario) = design_turn_scenario(prompt, page_was_empty) {
        state.editor_ui.scenario = Some(scenario);
    }
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
    let config = selected_builtin_agent_config(state, entry)?;
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&config)?;
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
        .and_then(op_editor_core::ModelEntry::builtin_model_id);
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
    // Finalize-lifecycle invariant (0718-1-k3-1 postmortem) — see
    // `chat_session::finalize_design_session_if_needed`'s doc comment.
    crate::chat_session::finalize_design_session_if_needed(host, current_chat, "teardown-backstop");
    *current_chat = None;
    *current_design = None;
    if super::super::allow_ai_bulk_write(host)
        && clear_fresh_starter_frame_for_design(host.editor_state_mut())
    {
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
    let root_seed_mobile = root_seed_mobile_for_turn(host.editor_state(), &user_text);
    // Only a request that promises sibling screens may make later roots
    // inherit the live screen's artboard; see `root_seed_prompt_is_continuation`.
    let root_seed_continuation = root_seed_prompt_is_continuation(&user_text);
    let agent_team_size = host.editor_state().chat.agent_team_size;
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
    // E-lite: tell the model the ⚡Nx setting it's actually operating under
    // (see `team_size_fact`) — independent of whether a canvas brief exists,
    // so a fresh-canvas multi-screen ask with team_size > 1 still knows it
    // may `spawn_agents`.
    if let Some(fact) = team_size_fact(agent_team_size) {
        system_prompt.push_str("\n\n---\n\n");
        system_prompt.push_str(&fact);
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
    op_editor_core::agent_indicators::begin_with_root_seed_hint(
        root_seed_mobile,
        root_seed_continuation,
    );
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
    fn clicking_the_footer_toggle_changes_what_a_design_turn_asks_for() {
        // The whole chain in one test, because every link of it was present
        // and the feature still did not exist: the chip that set
        // `thinking_mode` was painted by nothing, so no click could reach the
        // policy below. Black-box on purpose — the test finds the control the
        // way a user does (a press somewhere in the panel), not by importing
        // its rect.
        use op_editor_core::EditorState;
        use op_editor_ui::widgets::chat_click_flow::apply_chat_hit;
        use op_editor_ui::widgets::{AIChatHit, AIChatPlaceholder, AI_CHAT_HEIGHT, AI_CHAT_WIDTH};
        use op_editor_ui::{Point2D, Rect};

        let mut state = EditorState::new();
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        let toggle_point = {
            let panel = AIChatPlaceholder::from_editor(&state);
            let input = panel.input_rect(rect);
            let mut found = None;
            let mut y = input.origin.y;
            while y < input.origin.y + input.size.y && found.is_none() {
                let mut x = input.origin.x;
                while x < input.origin.x + input.size.x {
                    let point = Point2D::new(x, y);
                    if panel.hit_test(rect, point) == Some(AIChatHit::CycleThinking) {
                        found = Some(point);
                        break;
                    }
                    x += 1.0;
                }
                y += 1.0;
            }
            found.expect("the chat footer must expose a thinking-mode control")
        };

        // A model the profile does not force off follows the user's choice.
        const FREE: Option<&str> = Some("claude-opus-4");
        assert_eq!(
            resolve_design_thinking(FREE, state.chat.thinking_mode),
            ThinkingMode::Adaptive
        );
        for want in [
            ThinkingMode::Disabled,
            ThinkingMode::Enabled,
            ThinkingMode::Adaptive,
        ] {
            let hit = AIChatPlaceholder::from_editor(&state)
                .hit_test(rect, toggle_point)
                .expect("the toggle stays where it was found");
            apply_chat_hit(&mut state, hit, 0);
            assert_eq!(state.chat.thinking_mode, want);
            assert_eq!(
                resolve_design_thinking(FREE, state.chat.thinking_mode),
                want,
                "a design turn must ask for what the footer now shows"
            );
            // …and a `thinking_disabled` model stays forced off through every
            // one of those states. That override is deliberate, not a bug the
            // toggle is expected to defeat.
            assert_eq!(
                resolve_design_thinking(Some("glm-5.2"), state.chat.thinking_mode),
                ThinkingMode::Disabled
            );
        }
    }

    #[test]
    fn unknown_or_absent_model_keeps_chat_default() {
        // No selected builtin → keep the user's choice (don't silently disable).
        assert_eq!(
            resolve_design_thinking(None, ThinkingMode::Enabled),
            ThinkingMode::Enabled
        );
    }

    // ── document scenario stamped by a generation turn ────────────────────
    #[test]
    fn only_a_deck_prompt_on_an_empty_page_establishes_a_slides_document() {
        for deck_prompt in ["a pitch deck for our seed round", "做一个季度汇报 PPT"] {
            assert_eq!(
                design_turn_scenario(deck_prompt, true),
                Some(TemplateScene::Slides),
                "{deck_prompt}"
            );
            // The same ask against existing work adds slides to someone's
            // document; it does not redefine what that document is.
            assert_eq!(
                design_turn_scenario(deck_prompt, false),
                None,
                "{deck_prompt}"
            );
        }
        for other_prompt in [
            "design an admin analytics dashboard",
            "a mobile onboarding screen",
            "a landing page for a coffee shop",
        ] {
            assert_eq!(
                design_turn_scenario(other_prompt, true),
                None,
                "{other_prompt}"
            );
        }
    }

    #[test]
    fn stamping_reads_the_canvas_the_user_still_has() {
        // A pristine starter canvas is empty as far as authored content goes.
        let mut starter = op_editor_core::EditorState::starter();
        stamp_design_turn_scenario(&mut starter, "build me a 6-slide pitch deck");
        assert_eq!(starter.editor_ui.scenario, Some(TemplateScene::Slides));

        // A canvas with the user's own work keeps its identity, tag or none.
        let mut working = op_editor_core::EditorState::starter();
        let mut next_id = 20;
        working.create_node_for_tool(
            op_editor_core::Tool::Rect,
            &mut next_id,
            24.0,
            32.0,
            120.0,
            80.0,
        );
        stamp_design_turn_scenario(&mut working, "build me a 6-slide pitch deck");
        assert_eq!(working.editor_ui.scenario, None);

        // A turn that establishes nothing never clears an existing tag.
        let mut tagged = op_editor_core::EditorState::starter();
        tagged.editor_ui.scenario = Some(TemplateScene::Carousel);
        stamp_design_turn_scenario(&mut tagged, "design an admin analytics dashboard");
        assert_eq!(tagged.editor_ui.scenario, Some(TemplateScene::Carousel));
    }

    // ── prompt-aware design-loop routing ───────────────────────────
    #[test]
    fn loop_enabled_explicit_on_routes_any_prompt_to_loop() {
        let dashboard = "Design an admin analytics dashboard web app";
        assert!(loop_enabled(true, None, dashboard, false));
        assert!(loop_enabled(false, Some("1"), dashboard, false));
        assert!(loop_enabled(false, Some("true"), dashboard, false));
        assert!(loop_enabled(false, Some(" on "), dashboard, false));
    }

    #[test]
    fn loop_enabled_explicit_off_routes_any_prompt_to_orchestrator() {
        let mobile = "Design a mobile fitness app home";
        assert!(!loop_enabled(false, Some("0"), mobile, false));
        assert!(!loop_enabled(false, Some("false"), mobile, false));
        assert!(!loop_enabled(false, Some(" off "), mobile, false));
        // An explicit force-off wins even when the canvas already has a
        // mobile screen root — explicit settings always take precedence.
        assert!(!loop_enabled(false, Some("0"), mobile, true));
    }

    #[test]
    fn loop_enabled_auto_routes_mobile_prompt_to_loop() {
        assert!(loop_enabled(
            false,
            None,
            "Design a mobile fitness app home",
            false
        ));
    }

    #[test]
    fn loop_enabled_auto_routes_landing_prompt_to_loop() {
        assert!(loop_enabled(
            false,
            None,
            "Design a landing page for a climate SaaS",
            false
        ));
    }

    #[test]
    fn loop_enabled_auto_routes_dashboard_prompt_to_orchestrator() {
        assert!(!loop_enabled(
            false,
            None,
            "Design an admin analytics dashboard web app",
            false
        ));
    }

    #[test]
    fn loop_enabled_canvas_mobile_root_routes_ascii_blind_continuation_to_loop() {
        // "继续生成剩下3个页面" (continue generating the remaining 3 pages)
        // carries no ASCII mobile keyword `root_seed_prompt_is_mobile` can
        // match — the multi-screen fan-out break this fixes. When the canvas
        // already proves the product is mobile, that fact must route the
        // continuation to the loop regardless of the prompt's own text.
        assert!(loop_enabled(false, None, "继续生成剩下3个页面", true));
    }

    #[test]
    fn loop_enabled_canvas_mobile_root_false_keeps_prior_routing_for_ascii_blind_prompt() {
        // Regression lock: on an empty/desktop canvas the SAME
        // ASCII-keyword-blind prompt must keep today's behavior (routed to
        // the orchestrator) — the canvas signal only ADDS a route, it never
        // changes the text-only outcome when it's false.
        assert!(!loop_enabled(false, None, "继续生成剩下3个页面", false));
    }

    #[test]
    fn loop_enabled_auto_routes_ambiguous_homepage_to_orchestrator() {
        assert!(!loop_enabled(
            false,
            None,
            "Design the dashboard homepage for admin console",
            false
        ));
    }

    #[test]
    fn loop_enabled_auto_routes_web_app_homepage_to_orchestrator() {
        assert!(!loop_enabled(
            false,
            None,
            "Design the homepage for a project management web app",
            false
        ));
    }

    // ── canvas-state mobile-root signal ─────────────────────────────
    fn mobile_screen_frame(id: &str, width: f64) -> jian_ops_schema::node::PenNode {
        serde_json::from_value(serde_json::json!({
            "type": "frame",
            "id": id,
            "name": "Home",
            "width": width,
            "height": 844,
            "children": []
        }))
        .expect("frame fixture")
    }

    #[test]
    fn canvas_has_mobile_screen_root_true_for_390_wide_top_level_frame() {
        let mut state = op_editor_core::EditorState::new();
        state.active_children_mut().clear();
        state
            .active_children_mut()
            .push(mobile_screen_frame("home", 390.0));
        assert!(canvas_has_mobile_screen_root(&state));
    }

    #[test]
    fn canvas_has_mobile_screen_root_false_for_empty_canvas() {
        let mut state = op_editor_core::EditorState::new();
        state.active_children_mut().clear();
        assert!(!canvas_has_mobile_screen_root(&state));
    }

    #[test]
    fn canvas_has_mobile_screen_root_false_for_desktop_width_frame() {
        let mut state = op_editor_core::EditorState::new();
        state.active_children_mut().clear();
        state
            .active_children_mut()
            .push(mobile_screen_frame("home", 1440.0));
        assert!(!canvas_has_mobile_screen_root(&state));
    }

    #[test]
    fn mobile_continuation_keeps_mobile_root_seed_without_prompt_keywords() {
        let mut state = op_editor_core::EditorState::new();
        state.active_children_mut().clear();
        state
            .active_children_mut()
            .push(mobile_screen_frame("home", 390.0));

        assert!(root_seed_mobile_for_turn(&state, "继续生成剩余界面"));
    }

    #[test]
    fn empty_canvas_continuation_does_not_invent_mobile_root_seed() {
        let mut state = op_editor_core::EditorState::new();
        state.active_children_mut().clear();

        assert!(!root_seed_mobile_for_turn(&state, "继续生成剩余界面"));
    }

    // ── E-lite: team-size system-prompt fact ────────────────────────
    #[test]
    fn team_size_fact_is_none_at_the_default_of_one() {
        // A team of one has nothing to parallelize — injecting the fact
        // would only add noise to every single-agent turn.
        assert_eq!(team_size_fact(1), None);
    }

    #[test]
    fn team_size_fact_names_the_setting_when_above_one() {
        assert_eq!(
            team_size_fact(4).as_deref(),
            Some("User's parallel-agents setting: 4")
        );
        assert_eq!(
            team_size_fact(6).as_deref(),
            Some("User's parallel-agents setting: 6")
        );
    }
}
