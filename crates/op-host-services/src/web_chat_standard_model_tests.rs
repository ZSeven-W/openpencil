use super::selected_model_id;

#[test]
fn structured_builtin_keeps_its_concrete_model_for_standard_route_policy() {
    let request = crate::ai_proxy::parse_ai_stream_body(
        r#"{"builtinProviderId":"account:secondary","model":"builtin:account:secondary:shared:model","user":"hello"}"#,
    )
    .expect("request parses");

    assert_eq!(
        selected_model_id(&request, &op_editor_core::EditorState::new()).as_deref(),
        Some("shared:model")
    );
}

#[test]
fn old_web_structured_builtin_recovers_the_unique_saved_model_profile() {
    let request = crate::ai_proxy::parse_ai_stream_body(
        r#"{"provider":"codex-cli","model":"builtin:account:secondary:shared:model","user":"hello"}"#,
    )
    .expect("request parses");
    let mut snapshot = op_editor_core::EditorState::new();
    snapshot.editor_ui.agent_settings.add_builtin_agent_config(
        "Account",
        "sk-test",
        "shared:model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://api.example.com/v1",
    );
    snapshot.editor_ui.agent_settings.builtin_agents[0].id = "account:secondary".into();

    assert_eq!(
        selected_model_id(&request, &snapshot).as_deref(),
        Some("shared:model")
    );
}
