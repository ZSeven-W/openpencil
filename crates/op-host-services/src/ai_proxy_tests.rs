use super::*;

#[test]
fn models_json_deduplicates_models_without_changing_first_seen_order() {
    let mut editor = EditorState::new();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("First", "sk-first", "shared-model");
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Duplicate", "sk-duplicate", "shared-model");
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Second", "sk-second", "builtin-model");
    editor.chat.discovered_models = vec![
        op_editor_core::ModelEntry::new(
            op_editor_core::AgentProvider::ClaudeCode,
            "shared-model",
            "Shared",
        ),
        op_editor_core::ModelEntry::new(
            op_editor_core::AgentProvider::ClaudeCode,
            "cli-model",
            "CLI",
        ),
        op_editor_core::ModelEntry::new(
            op_editor_core::AgentProvider::ClaudeCode,
            "cli-model",
            "CLI duplicate",
        ),
    ];
    editor
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            op_editor_core::AgentProvider::ClaudeCode,
            op_editor_core::ProviderConnectOutcome {
                connected: true,
                info: Some("Connected via Claude Code".into()),
                ..Default::default()
            },
        );

    assert_eq!(
        serde_json::from_str::<Vec<String>>(&models_json(&editor)).expect("valid model list"),
        vec!["shared-model", "builtin-model"]
    );
}

#[test]
fn verified_cli_models_are_never_exposed_or_routed_by_the_web_proxy() {
    let mut editor = EditorState::new();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Built-in", "sk-built-in", "built-in-model");
    editor.chat.discovered_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "cli-model",
        "CLI model",
    )];
    editor
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            op_editor_core::AgentProvider::ClaudeCode,
            op_editor_core::ProviderConnectOutcome {
                connected: true,
                info: Some("Connected via CLI".into()),
                ..Default::default()
            },
        );
    editor.rebuild_chat_models();

    assert_eq!(
        serde_json::from_str::<Vec<String>>(&models_json(&editor)).expect("valid model list"),
        vec!["built-in-model"]
    );
    assert!(
        proxy_provider_with_chat_session(&editor, "cli-model", true).is_none(),
        "a CLI model must not fall back to an unrelated built-in provider"
    );
}

#[test]
fn server_persistence_does_not_allow_an_unapproved_public_request_endpoint() {
    let body = serde_json::json!({
        "model": "private-model",
        "user": "generate",
        "credential": {
            "id": "builtin-web-1",
            "preset": "custom",
            "display_name": "Private",
            "kind": "openai-compat",
            "api_key": "sk-transient",
            "model": "private-model",
            "base_url": "https://attacker.example/v1",
            "enabled": true
        }
    })
    .to_string();
    let request = parse_ai_stream_body(&body).expect("request parses");

    let result = proxy_provider_for_request(
        &EditorState::new(),
        &request,
        crate::web_credential_policy::WebCredentialPersistence::Server,
    );
    let Err(error) = result else {
        panic!("persistence must not authorize an arbitrary public provider endpoint");
    };

    assert!(error.contains("endpoint"), "unexpected error: {error}");
}

#[test]
fn persisted_browser_owned_agent_cannot_use_an_unapproved_public_endpoint() {
    let mut editor = EditorState::new();
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Browser",
        "sk-browser",
        "private-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://attacker.example/v1",
    );
    editor.editor_ui.agent_settings.builtin_agents[0].id =
        "web-credential:builtin:browser-agent".into();

    assert!(
        proxy_provider(&editor, "private-model").is_none(),
        "persisted browser-owned agents still require a preset or explicit allowlist"
    );
}

#[test]
fn disallowed_exact_browser_model_is_hidden_and_does_not_fall_back() {
    let mut editor = EditorState::new();
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Browser",
        "sk-browser",
        "private-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://attacker.example/v1",
    );
    editor.editor_ui.agent_settings.builtin_agents[0].id =
        "web-credential:builtin:browser-agent".into();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Operator", "sk-operator", "operator-model");

    assert_eq!(
        serde_json::from_str::<Vec<String>>(&models_json(&editor)).expect("valid model list"),
        vec!["operator-model"]
    );
    assert!(
        proxy_provider(&editor, "private-model").is_none(),
        "a disallowed exact browser model must not route to an unrelated provider"
    );
}
