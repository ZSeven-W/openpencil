use op_editor_core::EditorState;

/// Resolve the selected model to the daemon wire id and, for a browser-local
/// built-in provider, attach exactly that provider's request-scoped
/// credential. Non-built-in/daemon models carry no credential.
pub(crate) fn selected_target(state: &EditorState) -> (String, Option<serde_json::Value>) {
    let selected = state.chat.selected_model_entry();
    let selected_builtin = selected
        .and_then(|entry| entry.builtin_provider_id.as_deref())
        .and_then(|id| {
            state
                .editor_ui
                .agent_settings
                .builtin_agents
                .iter()
                .find(|agent| agent.id == id)
        });
    let model = selected_builtin
        .map(|agent| agent.model.clone())
        .or_else(|| selected.map(|entry| entry.value.clone()))
        .unwrap_or_else(|| "default".to_string());
    let credential = selected_builtin.map(|agent| {
        serde_json::json!({
            "id": agent.id,
            "preset": agent.preset.as_str(),
            "display_name": agent.display_name,
            "kind": match agent.kind {
                op_editor_core::BuiltinAgentKind::Anthropic => "anthropic",
                op_editor_core::BuiltinAgentKind::OpenAiCompat => "openai-compat",
            },
            "api_key": agent.api_key,
            "model": agent.model,
            "base_url": agent.base_url,
            "enabled": agent.enabled,
        })
    });
    (model, credential)
}

#[cfg(test)]
mod tests {
    use super::selected_target;
    use op_editor_core::{BuiltinAgentKind, EditorState};

    #[test]
    fn selected_target_attaches_only_the_selected_browser_builtin() {
        let mut state = EditorState::new();
        let selected_id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "Private",
            "sk-selected",
            "private-model",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.openai.com/v1",
        );
        state.editor_ui.agent_settings.add_builtin_agent_config(
            "Other",
            "sk-other",
            "other-model",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.openai.com/v1",
        );
        state.rebuild_chat_models();
        state.chat.selected_model = state
            .chat
            .available_models
            .iter()
            .position(|entry| entry.builtin_provider_id.as_deref() == Some(selected_id.as_str()))
            .unwrap();

        let (model, credential) = selected_target(&state);
        let credential = credential.expect("selected credential");

        assert_eq!(model, "private-model");
        assert_eq!(credential["api_key"], "sk-selected");
        assert!(!credential.to_string().contains("sk-other"));
    }
}
