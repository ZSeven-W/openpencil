use op_editor_core::{EditorState, PenNodeExt};
use op_orchestrator::{AppendContext, ContinuationContext, DesignRequest};

/// Resolve the selected chat model's capability id for the orchestrator.
/// Built-in (API-key) agents expose their concrete model id. ACP entries
/// preserve their `acp:<id>` catalog identity so the model-profile resolver
/// can choose its conservative weak-agent default instead of treating a
/// missing id as Full tier. The ACP marker is not a transport model override:
/// `selected_cli_model_id` still yields `None` for ACP providers.
///
/// Fixed CLI agents keep choosing their own model internally and yield `None`
/// here (the CLI-side selection rides `ChatProviderLlmClient::with_model`
/// instead). The returned id feeds model-aware orchestrator policy —
/// tier-gated skill filtering, the element-manifest routing gate, and the M3
/// thinking policy — and matches the configuration the ab-v9 benchmarks ran
/// with (op-smoke has always passed `OPENPENCIL_ORCHESTRATOR_MODEL` through).
fn selected_orchestrator_model(state: &EditorState) -> Option<String> {
    let entry = state.chat.selected_model_entry()?;
    if entry.acp_agent_id().is_some() {
        return Some(entry.value.clone());
    }
    entry.builtin_model_id().map(str::to_string)
}

pub(crate) fn build_design_request(
    prompt: String,
    state: &EditorState,
    append_context: Option<AppendContext>,
) -> DesignRequest {
    let continuation_context =
        sibling_continuation_context(state, &prompt, append_context.as_ref());
    DesignRequest {
        prompt,
        model: selected_orchestrator_model(state),
        provider: None,
        design_md: state.doc.design_md.clone(),
        // Detected by `chat_intent::detect_append_intent` when the
        // prompt asks to extend the existing page (GAP #33). TS wires
        // this from the agent tool executor (agent-tool-executor.ts:234);
        // the shell's design pipeline is that path's equivalent.
        append_context,
        continuation_context,
        concurrency: state.chat.agent_team_size,
        validation_enabled: true,
        visual_ref_enabled: false,
        // Policy the user set in the Asset Center: it overrides the
        // style guide the prompt would otherwise infer.
        pinned_style_guide: state.editor_ui.pinned_style_guide.clone(),
    }
}

/// Capture the existing screen's artboard contract for named sibling-screen
/// continuations. A blank starter is deliberately ignored: new documents keep
/// design-type inference instead of inheriting an arbitrary starter size.
fn sibling_continuation_context(
    state: &EditorState,
    prompt: &str,
    append_context: Option<&AppendContext>,
) -> Option<ContinuationContext> {
    if append_context.is_some() {
        return None;
    }
    let screen_names = op_host_services::chat_intent::listed_whole_screen_names(prompt);
    if screen_names.is_empty() {
        return None;
    }
    let screen = state.active_children().iter().rev().find(|node| {
        matches!(node, jian_ops_schema::node::PenNode::Frame(_))
            && node.children().is_some_and(|children| !children.is_empty())
            && node.width_px().is_some()
            && node.height_px().is_some()
    })?;
    Some(ContinuationContext {
        screen_width: screen.width_px()?,
        screen_height: screen.height_px()?,
        background_color: op_editor_core::first_solid_fill_hex(screen).map(str::to_string),
        screen_names,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_editor_core::{
        AgentProvider, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, EditorState,
        ModelEntry,
    };

    #[test]
    fn built_in_design_requests_enable_validation() {
        let mut state = EditorState::new();
        state.chat.agent_team_size = 4;

        let req = build_design_request("draw a mobile settings screen".into(), &state, None);

        assert!(req.validation_enabled);
        assert!(!req.visual_ref_enabled);
        assert_eq!(req.concurrency, 4);
        // No selected model entry → no model id (CLI agents pick their own).
        assert_eq!(req.model, None);
        assert!(req.append_context.is_none());
    }

    #[test]
    fn a_pinned_style_guide_rides_the_request() {
        // The pin is the user overriding style inference; if it does not
        // reach the request, the Asset Center's selected card is decoration.
        let mut state = EditorState::new();
        assert_eq!(
            build_design_request("draw a dashboard".into(), &state, None).pinned_style_guide,
            None
        );

        state.editor_ui.pinned_style_guide = Some("nordic-frost-light".into());
        let req = build_design_request("draw a dashboard".into(), &state, None);

        assert_eq!(
            req.pinned_style_guide.as_deref(),
            Some("nordic-frost-light")
        );
    }

    #[test]
    fn append_context_rides_the_request() {
        let state = EditorState::new();
        let ctx = AppendContext {
            target_parent_id: "content-root".into(),
            target_width: 390.0,
            existing_section_labels: vec!["Hero".into()],
            is_mobile: true,
        };

        let req = build_design_request("continue the page".into(), &state, Some(ctx));

        let ctx = req.append_context.expect("append context attached");
        assert_eq!(ctx.target_parent_id, "content-root");
        assert_eq!(ctx.existing_section_labels, vec!["Hero".to_string()]);
        assert!(req.continuation_context.is_none());
    }

    #[test]
    fn named_mobile_continuation_inherits_screen_contract() {
        let mut state = EditorState::new();
        state.active_children_mut().clear();
        state.active_children_mut().push(
            serde_json::from_value(serde_json::json!({
                "type": "frame",
                "id": "home",
                "name": "Nocturne 今夜",
                "width": 390,
                "height": 844,
                "fill": [{ "type": "solid", "color": "#050508" }],
                "children": [{ "type": "text", "id": "title", "content": "今夜天空" }]
            }))
            .expect("existing screen"),
        );

        let req = build_design_request("继续生成 星图、观测计划、我的3个界面".into(), &state, None);

        let context = req.continuation_context.expect("continuation context");
        assert_eq!(
            (context.screen_width, context.screen_height),
            (390.0, 844.0)
        );
        assert_eq!(context.background_color.as_deref(), Some("#050508"));
        assert_eq!(context.screen_names, ["星图", "观测计划", "我的"]);
    }

    #[test]
    fn blank_document_does_not_invent_a_continuation_contract() {
        let state = EditorState::new();
        let req = build_design_request(
            "Continue generating the Explore/Profile screens".into(),
            &state,
            None,
        );
        assert!(req.continuation_context.is_none());
    }

    #[test]
    fn listed_follow_on_screens_keep_x2_request_out_of_append_mode() {
        let mut state = EditorState::new();
        state.active_children_mut().clear();
        for (id, name) in [("home", "Home"), ("trips", "Trips"), ("saved", "Saved")] {
            state.active_children_mut().push(
                serde_json::from_value(serde_json::json!({
                    "type": "frame",
                    "id": id,
                    "name": name,
                    "width": 375,
                    "height": 812,
                    "children": [{
                        "type": "frame",
                        "id": format!("{id}-content"),
                        "name": "Content",
                        "width": 375,
                        "height": 700,
                        "children": []
                    }]
                }))
                .expect("screen fixture"),
            );
        }
        state.chat.agent_team_size = 2;
        for prompt in [
            "继续完成 explore/profile界面",
            "Continue generating the explore/profile interface",
        ] {
            let append_context =
                op_host_services::chat_intent::detect_append_intent(&state, prompt);
            let req = build_design_request(prompt.into(), &state, append_context);

            assert!(
                req.append_context.is_none(),
                "the continuation must create sibling roots instead of targeting Home: {prompt}"
            );
            assert_eq!(req.concurrency, 2, "the x2 setting must survive routing");
            assert!(
                op_host_services::chat_intent::should_auto_generate_design_md(
                    &state,
                    prompt,
                    req.append_context.as_ref(),
                ),
                "follow-on screens must extract the existing canvas design system first: {prompt}"
            );
        }
    }

    #[test]
    fn selected_builtin_agent_model_reaches_the_orchestrator() {
        let mut state = EditorState::new();
        state
            .editor_ui
            .agent_settings
            .builtin_agents
            .push(BuiltinAgentConfig {
                id: "builtin-1".into(),
                preset: BuiltinAgentPresetKey::Custom,
                display_name: "MiniMax".into(),
                kind: BuiltinAgentKind::OpenAiCompat,
                api_key: "sk-test".into(),
                models: vec!["MiniMax-M2.7".into(), "MiniMax-M3".into()],
                base_url: "http://localhost:9".into(),
                enabled: true,
            });
        state.chat.available_models = vec![ModelEntry::builtin(
            AgentProvider::ClaudeCode,
            "builtin-1",
            "builtin:builtin-1:MiniMax-M3",
            "MiniMax M3",
        )];
        state.chat.selected_model = 0;

        let req = build_design_request("draw a dashboard".into(), &state, None);

        // Drives tier-gated prompts, the manifest routing gate, and the
        // M3 thinking policy — must match the agent the session will call.
        assert_eq!(req.model.as_deref(), Some("MiniMax-M3"));
    }

    #[test]
    fn selected_acp_agent_reaches_the_orchestrator_as_basic_tier() {
        let mut state = EditorState::new();
        state.chat.available_models = vec![ModelEntry::acp("custom/vendor", "Custom ACP")];
        state.chat.selected_model = 0;

        let req = build_design_request("draw a dashboard".into(), &state, None);

        assert_eq!(req.model.as_deref(), Some("acp:custom/vendor"));
        assert_eq!(
            op_orchestrator::resolve_model_profile(req.model.as_deref().unwrap()).tier,
            op_orchestrator::ModelTier::Basic
        );
    }
}
