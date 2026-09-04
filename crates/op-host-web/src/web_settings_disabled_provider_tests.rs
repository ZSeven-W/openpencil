use super::*;
use op_editor_core::{BuiltinAgentKind, EditorState};

#[test]
fn adding_over_a_disabled_provider_reenables_one_roundtrippable_card() {
    let mut source = EditorState::new();
    let settings = &mut source.editor_ui.agent_settings;
    let provider_id = settings.add_builtin_agent_configs(
        "Private",
        "sk-test",
        ["model-a"],
        BuiltinAgentKind::OpenAiCompat,
        "https://example.com/v1",
    );
    settings.builtin_agents[0].enabled = false;

    let restored_id = settings.add_builtin_agent_configs(
        "Alias",
        "sk-test",
        ["model-b"],
        BuiltinAgentKind::OpenAiCompat,
        "https://example.com/v1",
    );

    assert_eq!(restored_id, provider_id);
    assert_eq!(settings.builtin_agents.len(), 1);
    assert!(settings.builtin_agents[0].enabled);

    let json = credentials_json(&source).expect("credentials serialize");
    let mut restored = EditorState::new();
    apply_credential_json(&mut restored, &json).expect("strict credential payload round-trips");
    let agents = &restored.editor_ui.agent_settings.builtin_agents;
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].models, ["model-a", "model-b"]);
    assert!(agents[0].enabled);
}
