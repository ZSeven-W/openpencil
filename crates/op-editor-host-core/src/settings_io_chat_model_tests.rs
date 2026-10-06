//! Chat-model persistence: the picker row (built-in / ACP included) survives
//! a save + load cycle. Sibling of `settings_io_chat_agent_tests.rs`.

use super::*;
use op_editor_core::{BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey};

fn glm_agent() -> BuiltinAgentConfig {
    BuiltinAgentConfig {
        id: "builtin-6".into(),
        preset: BuiltinAgentPresetKey::GlmCoding,
        display_name: "GLM Coding Plan".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key: "k".into(),
        models: vec!["glm-5.3".into(), "glm-5.3-flash".into()],
        base_url: "https://open.bigmodel.cn/api/coding/paas/v4".into(),
        enabled: true,
    }
}

#[test]
fn a_builtin_model_choice_survives_save_and_load() {
    let mut src = EditorState::new();
    src.editor_ui
        .agent_settings
        .builtin_agents
        .push(glm_agent());
    src.rebuild_chat_models();
    let flash = src
        .chat
        .available_models
        .iter()
        .position(|entry| entry.value == "builtin:builtin-6:glm-5.3-flash")
        .expect("the flash row is in the catalog");
    let before = fingerprint(&src);
    src.select_chat_model(flash);
    assert_ne!(
        before,
        fingerprint(&src),
        "changing the row must dirty the fingerprint"
    );

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    assert!(
        json.contains("\"chat_model\":\"builtin:builtin-6:glm-5.3-flash\""),
        "{json}"
    );

    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);
    assert_eq!(
        dst.chat
            .selected_model_entry()
            .map(|entry| entry.value.as_str()),
        Some("builtin:builtin-6:glm-5.3-flash"),
        "the relaunch lands on the row the user chose, not the first one"
    );
}

#[test]
fn a_missing_or_stale_chat_model_keeps_the_default_row() {
    let legacy = serde_json::json!({ "version": 1, "chat_model": "builtin:gone:nope" });
    let payload: SettingsPayload = serde_json::from_value(legacy).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);
    assert_eq!(dst.chat.selected_model, 0);
}
