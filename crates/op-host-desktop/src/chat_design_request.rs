use op_editor_core::EditorState;
use op_orchestrator::{AppendContext, DesignRequest};

/// Resolve the selected chat model's id for the orchestrator. Only
/// built-in (API-key) agents expose a concrete model id; CLI/ACP agents
/// pick their own model internally and yield `None` (the CLI-side
/// selection rides `ChatProviderLlmClient::with_model` instead). The id
/// feeds model-aware orchestrator policy — tier-gated skill filtering,
/// the element-manifest routing gate, and the M3 thinking policy — and
/// matches the configuration the ab-v9 benchmarks ran with (op-smoke
/// has always passed `OPENPENCIL_ORCHESTRATOR_MODEL` through).
fn selected_builtin_model(state: &EditorState) -> Option<String> {
    let entry = state.chat.selected_model_entry()?;
    let id = entry.builtin_provider_id.as_deref()?;
    state
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .find(|agent| agent.id == id)
        .map(|agent| agent.model.trim().to_string())
        .filter(|model| !model.is_empty())
}

pub(crate) fn build_design_request(
    prompt: String,
    state: &EditorState,
    append_context: Option<AppendContext>,
) -> DesignRequest {
    DesignRequest {
        prompt,
        model: selected_builtin_model(state),
        provider: None,
        design_md: state.doc.design_md.clone(),
        // Detected by `chat_intent::detect_append_intent` when the
        // prompt asks to extend the existing page (GAP #33). TS wires
        // this from the agent tool executor (agent-tool-executor.ts:234);
        // the shell's design pipeline is that path's equivalent.
        append_context,
        concurrency: state.chat.agent_team_size,
        validation_enabled: true,
        visual_ref_enabled: false,
    }
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
                model: "MiniMax-M3".into(),
                base_url: "http://localhost:9".into(),
                enabled: true,
            });
        let mut entry = ModelEntry::new(AgentProvider::ClaudeCode, "MiniMax-M3", "MiniMax M3");
        entry.builtin_provider_id = Some("builtin-1".into());
        state.chat.available_models = vec![entry];
        state.chat.selected_model = 0;

        let req = build_design_request("draw a dashboard".into(), &state, None);

        // Drives tier-gated prompts, the manifest routing gate, and the
        // M3 thinking policy — must match the agent the session will call.
        assert_eq!(req.model.as_deref(), Some("MiniMax-M3"));
    }
}
