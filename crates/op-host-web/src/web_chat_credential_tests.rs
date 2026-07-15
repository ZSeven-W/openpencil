use super::*;
use op_editor_core::chat::{AgentProvider, ModelEntry};

fn state_with_queued_send(text: &str) -> EditorState {
    let mut state = EditorState::new();
    state.chat.set_input_text(text);
    assert!(state.chat.begin_send());
    state
}

#[test]
fn prepare_turn_attaches_only_the_selected_builtin_credential() {
    let mut state = state_with_queued_send("hello");
    let selected_id = state.editor_ui.agent_settings.add_builtin_agent_config(
        "Private",
        "sk-selected",
        "private-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Other",
        "sk-must-not-leak",
        "other-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://other.test/v1",
    );
    state.rebuild_chat_models();
    state.chat.selected_model = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.as_deref() == Some(selected_id.as_str()))
        .expect("selected built-in model");

    let body: serde_json::Value = serde_json::from_str(
        &prepare_turn(&mut state)
            .expect("send was pending")
            .body_json,
    )
    .expect("body is JSON");

    assert_eq!(body["model"], "private-model");
    assert_eq!(body["credential"]["api_key"], "sk-selected");
    assert_eq!(body["credential"]["id"], selected_id);
    assert!(!body.to_string().contains("sk-must-not-leak"));
}

#[test]
fn prepare_turn_uses_null_credential_for_a_daemon_builtin_model() {
    let mut state = state_with_queued_send("hello");
    state.chat.available_models = vec![ModelEntry::builtin_with_display_name(
        AgentProvider::ClaudeCode,
        "daemon-builtin:claude-sonnet-4-5",
        "Server API Key",
        "claude-sonnet-4-5",
        "Claude Sonnet 4.5",
    )];

    let body: serde_json::Value = serde_json::from_str(
        &prepare_turn(&mut state)
            .expect("send was pending")
            .body_json,
    )
    .expect("body is JSON");

    assert!(body
        .get("credential")
        .is_some_and(serde_json::Value::is_null));
}

#[test]
fn daemon_model_refresh_preserves_browser_local_builtin_models() {
    let mut state = EditorState::new();
    let local_id = state.editor_ui.agent_settings.add_builtin_agent_config(
        "Private",
        "sk-local",
        "private-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    state.rebuild_chat_models();

    apply_models(&mut state, &["server-model".into()]);

    assert!(state
        .chat
        .available_models
        .iter()
        .any(|entry| entry.value == "server-model"));
    assert!(state
        .chat
        .available_models
        .iter()
        .any(|entry| { entry.builtin_provider_id.as_deref() == Some(local_id.as_str()) }));
}

#[test]
fn reconcile_models_restores_daemon_catalog_after_core_rebuild() {
    let mut state = EditorState::new();
    let local_id = state.editor_ui.agent_settings.add_builtin_agent_config(
        "Private",
        "sk-local",
        "private-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    apply_models(&mut state, &["server-model".into()]);
    state.chat.selected_model = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.as_deref() == Some(local_id.as_str()))
        .expect("browser-local model is selectable");

    state.rebuild_chat_models();
    assert!(
        state
            .chat
            .available_models
            .iter()
            .all(|entry| entry.value != "server-model"),
        "the regression precondition reproduces the core filter"
    );

    reconcile_models(&mut state);

    assert!(state
        .chat
        .available_models
        .iter()
        .any(|entry| entry.value == "server-model"));
    assert_eq!(
        state
            .chat
            .selected_model_entry()
            .and_then(|entry| entry.builtin_provider_id.as_deref()),
        Some(local_id.as_str())
    );
}

#[test]
fn reconcile_models_hides_acp_until_the_web_proxy_supports_acp_chat() {
    let mut state = EditorState::new();
    let id = state.editor_ui.agent_settings.add_acp_agent_config(
        "Local ACP",
        op_editor_core::AcpConnectionType::Local,
        "op-agent",
        Vec::new(),
        std::collections::BTreeMap::new(),
        None,
        true,
    );
    state
        .editor_ui
        .agent_settings
        .apply_acp_agent_connect_outcome(
            &id,
            op_editor_core::AcpAgentConnectOutcome {
                connected: true,
                info: Some("Local ACP".into()),
                error: None,
            },
        );
    state.rebuild_chat_models();
    assert!(state
        .chat
        .available_models
        .iter()
        .any(|entry| entry.value == format!("acp:{id}")));

    reconcile_models(&mut state);

    assert!(state
        .chat
        .available_models
        .iter()
        .all(|entry| entry.value != format!("acp:{id}")));
}

#[test]
fn reconcile_models_hides_cli_models_from_the_builtin_only_web_surface() {
    let mut state = EditorState::new();
    state.chat.discovered_models = vec![ModelEntry::new(
        AgentProvider::CodexCli,
        "gpt-cli",
        "CLI Model",
    )];

    reconcile_models(&mut state);

    assert!(state.chat.available_models.is_empty());
}

#[test]
fn canvaskit_reconciles_web_models_before_painting() {
    let source = include_str!("canvaskit.rs");
    let repaint = source
        .split("impl CkInner {")
        .nth(1)
        .and_then(|body| body.split("fn sync_a11y").next())
        .expect("CkInner repaint source");
    let reconcile = repaint
        .find("crate::web_chat::reconcile_models(self.host.editor_state_mut())")
        .expect("repaint reconciles the daemon and browser model catalogs");
    let paint = repaint
        .find("self.host.paint_dyn")
        .expect("repaint paints the host");

    assert!(
        reconcile < paint,
        "the picker must be reconciled before paint"
    );
}
