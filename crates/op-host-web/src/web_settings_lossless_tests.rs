use super::*;
use op_editor_core::{BuiltinAgentKind, EditorState, ThemeMode};

fn valid_credentials() -> serde_json::Value {
    serde_json::json!({
        "version": 2,
        "builtin_agents": [{
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Current",
            "kind": "openai-compat",
            "api_key": "sk-current-secret",
            "model": "current-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null
    })
}

fn assert_credential_snapshot_is_read_only(value: serde_json::Value) {
    let raw = serde_json::to_string(&value).expect("encode credential fixture");
    let mut state = EditorState::new();
    let mut writes = Vec::new();
    let load = load_into_with(&mut state, None, Some(&raw), |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(!load.loaded);
    assert!(load.unsupported_version);
    assert!(
        writes.is_empty(),
        "incompatible raw credentials must survive"
    );
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());

    let mut baseline = load.initial_fingerprint(&state);
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Edited",
        "sk-edited",
        "edited-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    assert!(
        save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
            writes.push((key.to_string(), json.to_string()));
            true
        })
        .is_none()
    );
    assert!(writes.is_empty());
}

#[test]
fn same_version_unknown_credential_fields_remain_read_only() {
    let mut top_level = valid_credentials();
    top_level["future_credentials"] = serde_json::json!({"api_key":"future-top-secret"});
    assert_credential_snapshot_is_read_only(top_level);

    let mut nested = valid_credentials();
    nested["builtin_agents"][0]["future_auth"] =
        serde_json::json!({"token":"future-nested-secret"});
    assert_credential_snapshot_is_read_only(nested);
}

#[test]
fn lossy_credential_values_remain_read_only() {
    for alias in ["openai", "openai_compat"] {
        let mut kind_alias = valid_credentials();
        kind_alias["builtin_agents"][0]["kind"] = serde_json::json!(alias);
        assert_credential_snapshot_is_read_only(kind_alias);
    }

    let mut missing_preset = valid_credentials();
    missing_preset["builtin_agents"][0]
        .as_object_mut()
        .expect("built-in fixture")
        .remove("preset");
    assert_credential_snapshot_is_read_only(missing_preset);

    let mut normalized_preset = valid_credentials();
    normalized_preset["builtin_agents"][0]["preset"] = serde_json::json!("doubao");
    normalized_preset["builtin_agents"][0]["model"] = serde_json::json!("ark-code-latest");
    normalized_preset["builtin_agents"][0]["base_url"] =
        serde_json::json!("https://ark.cn-beijing.volces.com/api/coding");
    assert_credential_snapshot_is_read_only(normalized_preset);

    let mut unknown_kind = valid_credentials();
    unknown_kind["builtin_agents"][0]["kind"] = serde_json::json!("future-provider");
    assert_credential_snapshot_is_read_only(unknown_kind);

    let mut unknown_image_provider = valid_credentials();
    unknown_image_provider["image_gen_profiles"] = serde_json::json!([{
        "id":"igp-1",
        "name":"Future image",
        "provider":"future-provider",
        "api_key":"future-image-secret",
        "model":"future-image",
        "base_url":null
    }]);
    assert_credential_snapshot_is_read_only(unknown_image_provider);

    let mut bad_active = valid_credentials();
    bad_active["active_image_gen_profile_id"] = serde_json::json!("igp-missing");
    assert_credential_snapshot_is_read_only(bad_active);

    let mut implicit_active = valid_credentials();
    implicit_active["image_gen_profiles"] = serde_json::json!([{
        "id":"igp-1",
        "name":"Image",
        "provider":"openai",
        "api_key":"image-secret",
        "model":"gpt-image-1",
        "base_url":null
    }]);
    assert_credential_snapshot_is_read_only(implicit_active);

    let mut empty_openverse = valid_credentials();
    empty_openverse["openverse_oauth"] = serde_json::json!({"client_id":"","client_secret":""});
    assert_credential_snapshot_is_read_only(empty_openverse);

    let mut whitespace_openverse = valid_credentials();
    whitespace_openverse["openverse_oauth"] =
        serde_json::json!({"client_id":" client ","client_secret":" secret "});
    assert_credential_snapshot_is_read_only(whitespace_openverse);

    let mut duplicate = valid_credentials();
    duplicate["builtin_agents"] = serde_json::json!([
        duplicate["builtin_agents"][0].clone(),
        {
            "id": "builtin-2",
            "preset": "custom",
            "display_name": "Current",
            "kind": "openai-compat",
            "api_key": "sk-current-secret",
            "model": "current-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }
    ]);
    assert_credential_snapshot_is_read_only(duplicate);
}

#[test]
fn same_version_unknown_general_settings_remain_read_only() {
    let raw = r#"{
        "version":1,
        "theme":"light",
        "future_setting":{"secret":"must-survive"}
    }"#;
    let mut state = EditorState::new();
    let mut writes = 0;
    let load = load_into_with(&mut state, Some(raw), None, |_, _| {
        writes += 1;
        true
    });

    assert!(!load.loaded);
    assert!(load.unsupported_version);
    assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);
    assert_eq!(writes, 0);
    assert!(load.initial_settings_fingerprint(&state).is_none());

    let mut credential_baseline = load.initial_fingerprint(&state);
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Edited",
        "sk-edited",
        "edited-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    assert!(
        save_credentials_if_changed_with(&state, &mut credential_baseline, |_, _| {
            writes += 1;
            true
        })
        .is_none()
    );
    assert_eq!(writes, 0);
}

#[test]
fn general_values_that_would_be_normalized_remain_read_only() {
    let too_many_recent = (0..=RECENT_FILE_CAP)
        .map(|index| serde_json::json!({"path":format!("/{index}.op"),"modified_at":index}))
        .collect::<Vec<_>>();
    let cases = vec![
        serde_json::json!({"version":1,"theme":"sepia"}),
        serde_json::json!({"version":1,"locale":"future-locale"}),
        serde_json::json!({"version":1,"locale":"en"}),
        serde_json::json!({"version":1,"locale":"zh"}),
        serde_json::json!({"version":1,"mcp_port":80}),
        serde_json::json!({"version":1,"recent_files":too_many_recent}),
        serde_json::json!({
            "version":1,
            "builtin_agents":[{
                "id":"builtin-1","display_name":"Future","kind":"future-provider",
                "api_key":"future-secret","model":"future-model",
                "base_url":"https://future.example/v1","enabled":true
            }]
        }),
    ];

    for value in cases {
        let raw = serde_json::to_string(&value).expect("encode settings fixture");
        let mut state = EditorState::new();
        let mut writes = 0;
        let load = load_into_with(&mut state, Some(&raw), None, |_, _| {
            writes += 1;
            true
        });
        assert!(load.unsupported_version, "fixture={raw}");
        assert!(load.initial_settings_fingerprint(&state).is_none());
        assert_eq!(writes, 0, "fixture={raw}");
    }
}

#[test]
fn acp_scrub_preserves_unknown_general_fields_and_leaves_snapshot_read_only() {
    let raw = r#"{
        "version":1,
        "theme":"light",
        "connected":[true,true,true,true,true],
        "acp_agents":[{"id":"acp-1","command":"acp-secret"}],
        "future_setting":{"secret":"future-secret"}
    }"#;
    let mut state = EditorState::new();
    let mut writes = Vec::new();
    let load = load_into_with(&mut state, Some(raw), None, |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(!load.loaded);
    assert!(load.unsupported_version);
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].0, STORAGE_KEY);
    assert!(writes[0].1.contains("future_setting"));
    assert!(writes[0].1.contains("future-secret"));
    assert!(!writes[0].1.contains("acp_agents"));
    assert!(!writes[0].1.contains("acp-secret"));
    assert!(!writes[0].1.contains("connected"));
    assert!(load.initial_settings_fingerprint(&state).is_none());
}

#[test]
fn acp_scrub_preserves_unknown_credential_fields_and_disables_future_writes() {
    let mut value = valid_credentials();
    value["acp_agents"] = serde_json::json!([{
        "id":"acp-1",
        "command":"acp-secret"
    }]);
    value["future_credentials"] = serde_json::json!({"api_key":"future-credential-secret"});
    let raw = serde_json::to_string(&value).expect("encode credential fixture");
    let mut state = EditorState::new();
    let mut writes = Vec::new();
    let load = load_into_with(&mut state, None, Some(&raw), |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(!load.loaded);
    assert!(load.unsupported_version);
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].0, CREDENTIAL_STORAGE_KEY);
    assert!(writes[0].1.contains("future_credentials"));
    assert!(writes[0].1.contains("future-credential-secret"));
    assert!(!writes[0].1.contains("acp_agents"));
    assert!(!writes[0].1.contains("acp-secret"));

    let mut baseline = load.initial_fingerprint(&state);
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Edited",
        "sk-edited",
        "edited-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    assert!(
        save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
            writes.push((key.to_string(), json.to_string()));
            true
        })
        .is_none()
    );
    assert_eq!(writes.len(), 1);
}
