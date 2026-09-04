use super::*;

fn model_catalog(editor: &EditorState) -> Vec<Value> {
    serde_json::from_str(&models_json(editor)).expect("valid model list")
}

fn catalog_model_names(editor: &EditorState) -> Vec<String> {
    model_catalog(editor)
        .iter()
        .map(|row| {
            row["displayName"]
                .as_str()
                .expect("model display name")
                .to_string()
        })
        .collect()
}

fn exact_provider(
    editor: &EditorState,
    builtin_provider_id: &str,
    model: &str,
) -> Option<Box<dyn ChatProvider>> {
    let request = parse_ai_stream_body(
        &serde_json::json!({
            "builtinProviderId": builtin_provider_id,
            "model": model,
            "user": "hello",
        })
        .to_string(),
    )
    .expect("exact request parses");
    proxy_provider_for_request(
        editor,
        &request,
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
    )
    .expect("exact request is valid")
}

#[test]
fn models_json_preserves_provider_identity_and_first_seen_order() {
    let mut editor = EditorState::new();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("First", "sk-first", "shared-model");
    editor.editor_ui.agent_settings.builtin_agents[0]
        .set_models(["shared-model", "second-saved-model"]);
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

    let rows = model_catalog(&editor);
    assert_eq!(
        rows.iter()
            .map(|row| row["displayName"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![
            "shared-model",
            "second-saved-model",
            "shared-model",
            "builtin-model"
        ]
    );
    assert_ne!(rows[0]["builtinProviderId"], rows[2]["builtinProviderId"]);
    assert_ne!(rows[0]["value"], rows[2]["value"]);
    assert_eq!(rows[0]["providerDisplayName"], "First");
    assert_eq!(rows[2]["providerDisplayName"], "Duplicate");
    assert!(proxy_provider(&editor, "second-saved-model").is_some());
}

#[test]
fn structured_model_routes_equal_names_to_the_exact_provider() {
    let mut editor = EditorState::new();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("First", "sk-first", "shared-model");
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Second", "sk-second", "shared-model");
    editor.editor_ui.agent_settings.builtin_agents[0].id = "account:primary".into();
    editor.editor_ui.agent_settings.builtin_agents[1].id = "account:secondary".into();

    let provider = exact_provider(
        &editor,
        "account:secondary",
        "builtin:account:secondary:shared-model",
    )
    .expect("the exact provider builds");

    assert_eq!(provider.provider_label(), "Second");
}

#[test]
fn legacy_structured_model_without_separate_identity_routes_only_when_unique() {
    let mut editor = EditorState::new();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Old Web Provider", "sk-old", "saved:model");
    editor.editor_ui.agent_settings.builtin_agents[0].id = "account:old".into();

    let provider = proxy_provider(&editor, "builtin:account:old:saved:model")
        .expect("one exact generated value remains rolling-upgrade compatible");

    assert_eq!(provider.provider_label(), "Old Web Provider");
}

#[test]
fn structured_model_never_falls_back_to_a_shorter_overlapping_provider_id() {
    let mut editor = EditorState::new();
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Short",
        "sk-short",
        "secondary:shared-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Exact but disabled",
        "sk-exact",
        "shared-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://openrouter.ai/api/v1",
    );
    editor.editor_ui.agent_settings.builtin_agents[0].id = "account".into();
    editor.editor_ui.agent_settings.builtin_agents[1].id = "account:secondary".into();
    editor.editor_ui.agent_settings.builtin_agents[1].enabled = false;

    assert!(exact_provider(
        &editor,
        "account:secondary",
        "builtin:account:secondary:shared-model"
    )
    .is_none());
    assert!(
        proxy_provider(&editor, "builtin:account:secondary:shared-model").is_none(),
        "structured values without exact identity are never guessed"
    );
}

#[test]
fn exact_identity_disambiguates_a_non_injective_colon_join() {
    let mut editor = EditorState::new();
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Short",
        "sk-short",
        "secondary:shared",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Long",
        "sk-long",
        "shared",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://openrouter.ai/api/v1",
    );
    editor.editor_ui.agent_settings.builtin_agents[0].id = "account".into();
    editor.editor_ui.agent_settings.builtin_agents[1].id = "account:secondary".into();

    let rows = model_catalog(&editor);
    assert_eq!(rows[0]["value"], rows[1]["value"]);
    assert_ne!(rows[0]["builtinProviderId"], rows[1]["builtinProviderId"]);
    assert_eq!(
        exact_provider(&editor, "account", rows[0]["value"].as_str().unwrap())
            .expect("short provider")
            .provider_label(),
        "Short"
    );
    assert_eq!(
        exact_provider(
            &editor,
            "account:secondary",
            rows[1]["value"].as_str().unwrap()
        )
        .expect("long provider")
        .provider_label(),
        "Long"
    );
}

#[test]
fn ambiguous_legacy_bare_model_is_rejected() {
    let mut editor = EditorState::new();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("First", "sk-first", "shared-model");
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Second", "sk-second", "shared-model");

    assert!(proxy_provider(&editor, "shared-model").is_none());
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

    assert_eq!(catalog_model_names(&editor), vec!["built-in-model"]);
    assert!(
        proxy_provider_with_chat_session(&editor, "cli-model", true).is_none(),
        "a CLI model must not fall back to an unrelated built-in provider"
    );
}

#[test]
fn transient_request_accepts_a_public_https_endpoint_without_allowlist() {
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
            "base_url": "https://custom-gateway.example/v1",
            "enabled": true
        }
    })
    .to_string();
    let request = parse_ai_stream_body(&body).expect("request parses");

    let provider = proxy_provider_for_request(
        &EditorState::new(),
        &request,
        crate::web_credential_policy::WebCredentialPersistence::Server,
    )
    .expect("public HTTPS endpoint is accepted");
    assert!(provider.is_some(), "provider must build for the credential");
}

#[test]
fn server_persistence_does_not_allow_a_reserved_request_endpoint() {
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
            "base_url": "http://169.254.169.254/v1",
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
        panic!("persistence must not authorize a reserved provider endpoint");
    };

    let error = error.to_string();
    assert!(error.contains("endpoint"), "unexpected error: {error}");
}

#[test]
fn persisted_browser_owned_agent_can_use_a_public_https_endpoint() {
    let mut editor = EditorState::new();
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Browser",
        "sk-browser",
        "private-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://custom-gateway.example/v1",
    );
    editor.editor_ui.agent_settings.builtin_agents[0].id =
        "web-credential:builtin:browser-agent".into();

    assert!(
        proxy_provider(&editor, "private-model").is_some(),
        "public HTTPS endpoints no longer require a preset or explicit allowlist"
    );
    assert!(
        model_catalog(&editor).is_empty(),
        "browser-owned credentials are selected from the browser-local catalog, not echoed back"
    );
}

#[test]
fn persisted_browser_owned_agent_cannot_use_a_reserved_endpoint() {
    let mut editor = EditorState::new();
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Browser",
        "sk-browser",
        "private-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "http://10.0.0.7/v1",
    );
    editor.editor_ui.agent_settings.builtin_agents[0].id =
        "web-credential:builtin:browser-agent".into();

    assert!(
        proxy_provider(&editor, "private-model").is_none(),
        "reserved endpoints still require an explicit allowlist"
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
        "http://10.0.0.7/v1",
    );
    editor.editor_ui.agent_settings.builtin_agents[0].id =
        "web-credential:builtin:browser-agent".into();
    editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Operator", "sk-operator", "operator-model");

    assert_eq!(catalog_model_names(&editor), vec!["operator-model"]);
    assert!(
        proxy_provider(&editor, "private-model").is_none(),
        "a disallowed exact browser model must not route to an unrelated provider"
    );
}
