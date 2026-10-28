//! Model-catalog route tests for the web-canvas daemon.

use super::*;

#[test]
fn get_ai_models_returns_json_array() {
    let mut state = fresh_state();
    state
        .editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Built-in", "sk-test", "built-in-model");
    state.editor.chat.discovered_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "cli-model",
        "CLI model",
    )];
    state
        .editor
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
    state.editor.rebuild_chat_models();

    let r = handle_web_canvas_request("GET", "/api/ai/models", "", &mut state);
    assert!(r.status.starts_with("200"));
    let models =
        serde_json::from_str::<Vec<serde_json::Value>>(&r.body).expect("models body is valid JSON");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0]["displayName"], "built-in-model");
    assert_eq!(models[0]["providerDisplayName"], "Built-in");
    assert!(models[0]["builtinProviderId"].as_str().is_some());
}
