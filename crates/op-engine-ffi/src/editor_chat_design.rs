//! Mobile design-turn routing + preparation.
//!
//! Mirrors the desktop launch path (`op-host-desktop::
//! chat_session_launch_design`) on the pieces a phone runs: the
//! `op_orchestrator::classify_intent` gate, the shared design-agent system
//! prompt with prompt-matched depth skills + the canvas consistency brief,
//! the model-profile thinking policy, and the fresh-starter-frame clear.
//! The loop itself is the REAL desktop loop (`op_chat_agent::
//! chat_agent_loop`), launched from `editor_chat.rs`.

use op_editor_core::EditorState;

pub(crate) use op_chat_agent::backoff::DESIGN_LOOP_MAX_OUTPUT_TOKENS;

/// True when the message reads as a design request — the exact gate
/// `launch_if_pending` applies on desktop.
pub(crate) fn design_intent(message: &str) -> bool {
    matches!(
        op_orchestrator::classify_intent(message),
        op_orchestrator::Intent::Design
    )
}

/// Effective "disable hidden reasoning" decision for a design turn — the
/// desktop's `resolve_design_thinking` policy: a model whose profile is
/// `thinking_disabled` (glm-5.x / minimax / deepseek / …) burns its whole
/// output budget inside `<think>` and draws nothing, so the design turn
/// forces it off; everything else keeps the chat default (mobile chat
/// default is Adaptive, i.e. not disabled).
pub(crate) fn design_turn_disable_thinking(model: Option<&str>) -> bool {
    model
        .map(|m| op_orchestrator::resolve_model_profile(m).thinking_disabled)
        .unwrap_or(false)
}

/// Mobile-host capability note appended to the shared design-agent prompt.
/// The tool surface matches desktop minus the host arms a phone does not
/// have; and a design must NEVER be answered as HTML/CSS chat prose (the
/// exact failure the first text-only mobile chat build shipped).
const MOBILE_CAPABILITY_NOTE: &str = "\n\n---\n\n\
## Host capability note (this device)\n\n\
Unavailable on this host: `get_screenshot`, `export_nodes`, and \
`spawn_agents` (design every screen yourself, sequentially).\n\n\
NEVER answer a design request by writing HTML, CSS, or any other code as \
chat text — designs exist ONLY as canvas nodes created through \
`batch_design` tool calls. If you have not called `batch_design`, you have \
not designed anything.";

/// The design-turn system prompt — identical assembly to the desktop loop's
/// (`launch_design_loop_turn`): protocol + prompt-matched depth skills, the
/// canvas consistency brief when the canvas already holds work, then the
/// mobile capability note.
pub(crate) fn mobile_design_system_prompt(state: &EditorState, user_text: &str) -> String {
    let mut prompt = op_ai_skills::design_agent_system_prompt_with_skills_for(
        user_text,
        op_ai_skills::DesignVerifyProtocol::LayoutSnapshot,
    );
    if let Some(brief) = op_chat_agent::design_context::design_context_brief(state) {
        prompt.push_str("\n\n---\n\n");
        prompt.push_str(&brief);
        op_ai_skills::append_image_self_check_scope(&mut prompt);
    }
    prompt.push_str(MOBILE_CAPABILITY_NOTE);
    prompt
}

/// Screen-shaped top-level frame width band — mirrors the desktop routing
/// heuristic (`chat_session_launch_design::MOBILE_SCREEN_WIDTH`).
const MOBILE_SCREEN_WIDTH: std::ops::RangeInclusive<f64> = 320.0..=480.0;

/// True when the active page already holds a mobile-width top-level frame —
/// a continuation prompt with no ASCII mobile keyword still proves the
/// product is mobile through the canvas itself (desktop parity).
fn canvas_has_mobile_screen_root(state: &EditorState) -> bool {
    use op_editor_core::PenNodeExt;
    state.active_children().iter().any(|node| {
        matches!(node, jian_ops_schema::node::PenNode::Frame(_))
            && node
                .width_px()
                .is_some_and(|w| MOBILE_SCREEN_WIDTH.contains(&w))
    })
}

/// Root-seed hints for the indicator epoch (desktop
/// `begin_with_root_seed_hint` parity): `(mobile, continuation)`.
pub(crate) fn root_seed_hints(state: &EditorState, prompt: &str) -> (bool, bool) {
    let mobile = canvas_has_mobile_screen_root(state)
        || op_chat_agent::design_agent_tools::root_seed_prompt_is_mobile(prompt);
    let continuation = op_chat_agent::design_agent_tools::root_seed_prompt_is_continuation(prompt);
    (mobile, continuation)
}

/// Clear the pristine blank starter frame before a design lands — ported
/// from `op-host-desktop::chat_session_launch::clear_fresh_starter_frame_for_design`
/// so the generated design replaces the placeholder instead of stacking
/// beside it. Keeps a visual ghost at the starter's rect until the design's
/// root arrives (see [`reconcile_starter_ghost`]).
pub(crate) fn clear_fresh_starter_frame_for_design(state: &mut EditorState) -> bool {
    if !op_editor_core::blank_starter::active_page_is_blank_starter(state) {
        return false;
    }
    if let Some(only) = state.active_children().first() {
        use op_editor_core::PenNodeExt;
        let base = only.base();
        let (w, h) = (
            only.width_px().unwrap_or(1200.0),
            only.height_px().unwrap_or(800.0),
        );
        state.editor_ui.starter_ghost = Some([
            base.x.unwrap_or(0.0) as f32,
            base.y.unwrap_or(0.0) as f32,
            w as f32,
            h as f32,
        ]);
    }
    state.active_children_mut().clear();
    state.clear_selection();
    // Raw `active_children_mut()` bypasses the command/history path, so
    // bump the document revision explicitly (layer-panel row cache +
    // save-dirty tracking are keyed on it).
    state.mark_document_changed();
    true
}

/// Drop the starter ghost once the design's root landed or the turn ended
/// with nothing produced (desktop `reconcile_starter_ghost` parity).
pub(crate) fn reconcile_starter_ghost(state: &mut EditorState, turn_running: bool) -> bool {
    if state.editor_ui.starter_ghost.is_none() {
        return false;
    }
    if !state.active_children().is_empty() || !turn_running {
        state.editor_ui.starter_ghost = None;
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_verbs_and_nouns_classify_as_design() {
        for p in [
            "design a login page",
            "生成一个仪表盘",
            "设计一个暗色音乐流媒体 App 首页",
            "Luxury webapp for managing barbershop clients",
        ] {
            assert!(design_intent(p), "{p}");
        }
    }

    #[test]
    fn questions_classify_as_chat() {
        for p in ["what is a frame?", "这个怎么用", "thanks, that looks good"] {
            assert!(!design_intent(p), "{p}");
        }
    }

    #[test]
    fn mobile_prompt_carries_protocol_and_anti_html_ban() {
        let state = op_editor_core::EditorState::new();
        let prompt = mobile_design_system_prompt(&state, "设计一个音乐 App 首页");
        assert!(prompt.contains("batch_design"));
        // Script-first protocol (desktop parity) — the script arm is live on
        // mobile now that rquickjs builds via bindgen.
        assert!(prompt.contains("`batch_design`'s `script` mode"));
        assert!(prompt.contains("NEVER answer a design request by writing HTML"));
        assert!(prompt.contains("`spawn_agents`"));
        // Host-aware protocol: mobile's Step 8 verifies via snapshot_layout;
        // the screenshot check text must be absent entirely.
        assert!(prompt.contains("Verify with `snapshot_layout`"));
        assert!(!prompt.contains("The screenshot check is mandatory"));
        assert!(!prompt.contains("select:get_editor_state,get_guidelines,get_style_guide_tags,get_style_guide,get_variables,batch_get,snapshot_layout,batch_design,get_screenshot"));
    }

    #[test]
    fn reasoning_models_disable_thinking_for_design_turns() {
        assert!(design_turn_disable_thinking(Some("deepseek-chat")));
        assert!(design_turn_disable_thinking(Some("glm-5.2")));
        // Claude keeps the chat default.
        assert!(!design_turn_disable_thinking(Some("claude-opus-4")));
        assert!(!design_turn_disable_thinking(None));
    }

    #[test]
    fn starter_clear_and_ghost_reconcile_round_trip() {
        let mut state = op_editor_core::EditorState::starter();
        assert!(clear_fresh_starter_frame_for_design(&mut state));
        assert!(state.active_children().is_empty());
        assert!(state.editor_ui.starter_ghost.is_some());
        // Nothing landed yet + turn still running: ghost stays.
        assert!(!reconcile_starter_ghost(&mut state, true));
        // Turn over with nothing produced: ghost clears.
        assert!(reconcile_starter_ghost(&mut state, false));
        assert!(state.editor_ui.starter_ghost.is_none());
    }
}
