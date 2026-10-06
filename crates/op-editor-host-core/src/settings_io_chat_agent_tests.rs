//! Chat-agent persistence tests — sibling of `settings_io_tests.rs`
//! (that file sits at the 800-line cap).

use super::*;

#[test]
fn chat_agent_preference_round_trips_by_name_and_changes_fingerprint() {
    let mut src = EditorState::new();
    src.editor_ui.chat_selected_agent = 3; // OpenCode
    let expected_name = op_editor_core::AgentProvider::ALL[3].name();
    let before = fingerprint(&src);
    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    assert!(
        json.contains(&format!("\"chat_agent\":\"{expected_name}\"")),
        "the persisted value must be the agent's stable name, not its index: {json}"
    );

    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);
    assert_eq!(dst.editor_ui.chat_selected_agent, 3);
    assert_ne!(before, fingerprint(&EditorState::new()));
}

#[test]
fn chat_agent_preference_falls_back_to_zero_for_unknown_and_missing_names() {
    // A settings.json written by a newer build (provider renamed or
    // removed) must not dangle the index — it resolves to 0.
    let src = EditorState::new();
    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let json = json.replace(
        &format!(
            "\"chat_agent\":\"{}\"",
            op_editor_core::AgentProvider::ALL[0].name()
        ),
        "\"chat_agent\":\"Future Provider\"",
    );
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.chat_selected_agent = 5;
    apply_payload(&mut dst, payload);
    assert_eq!(dst.editor_ui.chat_selected_agent, 0);

    // A legacy file without the field keeps whatever the state held.
    let legacy = serde_json::json!({ "version": 1 });
    let payload: SettingsPayload = serde_json::from_value(legacy).unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.chat_selected_agent = 2;
    apply_payload(&mut dst, payload);
    assert_eq!(dst.editor_ui.chat_selected_agent, 2);
}
