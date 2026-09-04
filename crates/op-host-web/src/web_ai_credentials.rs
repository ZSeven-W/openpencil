use op_editor_core::{BuiltinAgentConfig, EditorState};

/// Serialize exactly one browser-owned built-in credential for a request-scoped
/// daemon call. Keeping this in one place makes chat and model discovery share
/// the same wire shape without ever uploading the rest of the local catalog.
pub(crate) fn builtin_credential(agent: &BuiltinAgentConfig) -> serde_json::Value {
    serde_json::json!({
        "id": agent.id,
        "preset": agent.preset.as_str(),
        "display_name": agent.display_name,
        "kind": match agent.kind {
            op_editor_core::BuiltinAgentKind::Anthropic => "anthropic",
            op_editor_core::BuiltinAgentKind::OpenAiCompat => "openai-compat",
        },
        "api_key": agent.api_key,
        // Discovery accepts an empty model. Request paths overwrite this
        // field with the selected saved model below.
        "model": agent.first_model().unwrap_or_default(),
        "base_url": agent.base_url,
        "enabled": agent.enabled,
    })
}

/// Resolve the selected model to the daemon wire id and, for a browser-local
/// built-in provider, attach exactly that provider's request-scoped
/// credential. Non-built-in/daemon models carry no credential.
pub(crate) fn selected_target(
    state: &EditorState,
) -> (String, Option<serde_json::Value>, Option<String>) {
    let selected = state.chat.selected_model_entry();
    let selected_builtin = selected
        .and_then(|entry| entry.builtin_provider_id.as_deref())
        .and_then(|id| {
            state
                .editor_ui
                .agent_settings
                .builtin_agents
                .iter()
                .find(|agent| agent.id == id && agent.ready())
        });
    let model = selected
        .and_then(|entry| entry.builtin_model_id())
        .filter(|model| selected_builtin.is_none_or(|agent| agent.has_model(model)))
        .map(str::to_string)
        .or_else(|| selected_builtin.and_then(|agent| agent.first_model().map(str::to_string)))
        .or_else(|| selected.map(|entry| entry.value.clone()))
        .unwrap_or_else(|| "default".to_string());
    let credential = selected_builtin.map(|agent| {
        let mut credential = builtin_credential(agent);
        credential["model"] = serde_json::Value::String(model.clone());
        credential
    });
    let daemon_builtin_id = if credential.is_none() {
        selected
            .filter(|entry| entry.value.starts_with("builtin:"))
            .and_then(|entry| entry.builtin_provider_id.as_deref())
            .and_then(|id| id.strip_prefix("daemon-builtin:"))
            .map(str::to_string)
    } else {
        None
    };
    (model, credential, daemon_builtin_id)
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

        let (model, credential, builtin_provider_id) = selected_target(&state);
        let credential = credential.expect("selected credential");

        assert_eq!(model, "private-model");
        assert_eq!(builtin_provider_id, None);
        assert_eq!(credential["api_key"], "sk-selected");
        assert!(!credential.to_string().contains("sk-other"));
    }

    #[test]
    fn selected_saved_model_overrides_the_configured_fallback_everywhere() {
        let mut state = EditorState::new();
        let id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "Private",
            "sk-selected",
            "fallback-b",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.openai.com/v1",
        );
        state.editor_ui.agent_settings.builtin_agents[0].set_models(["fallback-b", "saved-a"]);
        state.chat.available_models = vec![op_editor_core::ModelEntry::builtin(
            op_editor_core::AgentProvider::CodexCli,
            id.clone(),
            format!("builtin:{id}:saved-a"),
            "Saved A",
        )];
        state.chat.selected_model = 0;

        let (model, credential, builtin_provider_id) = selected_target(&state);

        assert_eq!(model, "saved-a");
        assert_eq!(builtin_provider_id, None);
        assert_eq!(credential.expect("credential")["model"], "saved-a");
    }

    #[test]
    fn stale_runtime_row_falls_back_to_the_current_configured_model() {
        let mut state = EditorState::new();
        let id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "Private",
            "sk-new",
            "current-model",
            BuiltinAgentKind::OpenAiCompat,
            "https://api.openai.com/v1",
        );
        state.chat.available_models = vec![op_editor_core::ModelEntry::builtin(
            op_editor_core::AgentProvider::CodexCli,
            id.clone(),
            format!("builtin:{id}:old-private-model"),
            "Old private model",
        )];
        state.chat.selected_model = 0;

        let (model, credential, builtin_provider_id) = selected_target(&state);

        assert_eq!(model, "current-model");
        assert_eq!(builtin_provider_id, None);
        assert_eq!(credential.expect("credential")["model"], "current-model");
    }
}
