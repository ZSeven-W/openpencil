use super::*;
use op_editor_core::{BuiltinAgentKind, BuiltinAgentPresetKey, EditorState, ThemeMode};

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

fn legacy_v1_single_model_cards() -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "locale": "en-US",
        "builtin_agents": [
            {
                "id": "builtin-first",
                "preset": "doubao",
                "display_name": "First-card metadata",
                "kind": "openai-compat",
                "api_key": "sk-legacy-secret",
                "model": "model-a",
                "base_url": "https://api.example.com/v1/",
                "enabled": true
            },
            {
                "id": "builtin-second",
                "preset": "doubao",
                "display_name": "Second-card metadata",
                "kind": "openai-compat",
                "api_key": "sk-legacy-secret",
                "model": "model-b",
                "base_url": "https://api.example.com/v1",
                "enabled": true
            }
        ]
    })
}

fn assert_legacy_settings_snapshot_is_read_only(value: serde_json::Value) {
    let raw = serde_json::to_string(&value).expect("encode legacy settings fixture");
    let mut state = EditorState::new();
    let mut writes = Vec::new();
    let load = load_into_with(&mut state, Some(&raw), None, |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(!load.loaded, "fixture={raw}");
    assert!(load.unsupported_version, "fixture={raw}");
    assert!(writes.is_empty(), "incompatible v1 settings must survive");
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
    assert!(load.initial_settings_fingerprint(&state).is_none());
    assert!(load.initial_fingerprint(&state).write_disabled_for_test());
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
fn legacy_v2_single_model_cards_merge_and_rewrite_canonical_credentials() {
    let legacy = serde_json::json!({
        "version": 2,
        "builtin_agents": [
            {
                "id": "builtin-1",
                "preset": "custom",
                "display_name": "Private · model-a",
                "kind": "openai-compat",
                "api_key": "sk-current-secret",
                "model": "model-a",
                "base_url": "https://api.example.com/v1/",
                "enabled": true
            },
            {
                "id": "builtin-2",
                "preset": "custom",
                "display_name": "Private · model-b",
                "kind": "openai-compat",
                "api_key": "sk-current-secret",
                "model": "model-b",
                "base_url": "https://api.example.com/v1",
                "enabled": true
            }
        ],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null
    });
    let raw = serde_json::to_string(&legacy).expect("encode legacy credentials");
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(&mut state, None, Some(&raw), |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(load.loaded);
    assert!(!load.unsupported_version);
    assert!(!load.write_pending);
    assert_eq!(state.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].models,
        ["model-a", "model-b"]
    );
    assert!(
        state.editor_ui.agent_settings.builtin_agents[0].enabled,
        "the shared enabled state survives the migration"
    );
    assert_eq!(writes.len(), 1, "migration must rewrite the credential key");
    assert_eq!(writes[0].0, super::credential_storage_key());
    let rewritten: serde_json::Value =
        serde_json::from_str(&writes[0].1).expect("canonical credential JSON");
    assert_eq!(rewritten["version"], 2);
    assert_eq!(rewritten["builtin_agents"].as_array().unwrap().len(), 1);
    assert_eq!(rewritten["builtin_agents"][0]["id"], "builtin-1");
    assert_eq!(rewritten["builtin_agents"][0]["model"], "model-a");
    assert_eq!(rewritten["builtin_agents"][0]["enabled"], true);
    assert_eq!(
        rewritten["builtin_agents"][0]["models"],
        serde_json::json!(["model-a", "model-b"])
    );
}

#[test]
fn legacy_v2_mixed_enabled_model_cards_remain_read_only() {
    let mut mixed = serde_json::json!({
        "version": 2,
        "builtin_agents": [
            {
                "id": "builtin-1",
                "preset": "custom",
                "display_name": "Private · model-a",
                "kind": "openai-compat",
                "api_key": "sk-current-secret",
                "model": "model-a",
                "base_url": "https://api.example.com/v1",
                "enabled": true
            },
            {
                "id": "builtin-2",
                "preset": "custom",
                "display_name": "Private · model-b",
                "kind": "openai-compat",
                "api_key": "sk-current-secret",
                "model": "model-b",
                "base_url": "https://api.example.com/v1",
                "enabled": true
            }
        ],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null
    });
    mixed["builtin_agents"][0]["enabled"] = serde_json::json!(false);

    assert_credential_snapshot_is_read_only(mixed);
}

#[test]
fn legacy_v1_single_model_cards_merge_before_validation_and_split_storage() {
    let legacy = legacy_v1_single_model_cards();
    let raw = serde_json::to_string(&legacy).expect("encode legacy settings");
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(&mut state, Some(&raw), None, |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(load.loaded);
    assert!(!load.unsupported_version);
    assert!(!load.write_pending);
    let [agent] = state.editor_ui.agent_settings.builtin_agents.as_slice() else {
        panic!("legacy cards must merge into one provider")
    };
    assert_eq!(agent.id, "builtin-first");
    assert_eq!(agent.preset, BuiltinAgentPresetKey::Doubao);
    assert_eq!(agent.display_name, "First-card metadata");
    assert_eq!(agent.api_key, "sk-legacy-secret");
    assert_eq!(agent.base_url, "https://api.example.com/v1/");
    assert_eq!(agent.models, ["model-a", "model-b"]);
    assert!(
        agent.enabled,
        "the shared enabled state survives the migration"
    );

    assert_eq!(writes.len(), 2, "credentials split before settings scrub");
    assert_eq!(writes[0].0, super::credential_storage_key());
    let credentials: serde_json::Value =
        serde_json::from_str(&writes[0].1).expect("canonical credential JSON");
    assert_eq!(credentials["version"], 2);
    assert_eq!(credentials["builtin_agents"].as_array().unwrap().len(), 1);
    assert_eq!(credentials["builtin_agents"][0]["id"], "builtin-first");
    assert_eq!(credentials["builtin_agents"][0]["preset"], "doubao");
    assert_eq!(
        credentials["builtin_agents"][0]["display_name"],
        "First-card metadata"
    );
    assert_eq!(credentials["builtin_agents"][0]["model"], "model-a");
    assert_eq!(
        credentials["builtin_agents"][0]["models"],
        serde_json::json!(["model-a", "model-b"])
    );
    assert_eq!(credentials["builtin_agents"][0]["enabled"], true);

    assert_eq!(writes[1].0, super::settings_storage_key());
    let settings: serde_json::Value =
        serde_json::from_str(&writes[1].1).expect("sanitized settings JSON");
    assert!(settings.get("builtin_agents").is_none());
    assert!(!writes[1].1.contains("sk-legacy-secret"));
    assert_eq!(settings["locale"], "en-US");
}

#[test]
fn ordinary_settings_validation_does_not_accept_legacy_duplicate_cards() {
    assert_eq!(
        super::validation::settings_payload(&legacy_v1_single_model_cards()).unwrap_err(),
        super::validation::SettingsValidationError::DuplicateAgents
    );
}

#[test]
fn legacy_v1_mixed_single_and_multi_model_cards_remain_read_only() {
    let mut mixed = legacy_v1_single_model_cards();
    mixed["builtin_agents"][0]["models"] = serde_json::json!(["model-a", "model-c"]);

    assert_legacy_settings_snapshot_is_read_only(mixed);
}

#[test]
fn legacy_v1_mixed_enabled_model_cards_remain_read_only() {
    let mut mixed = legacy_v1_single_model_cards();
    mixed["builtin_agents"][0]["enabled"] = serde_json::json!(false);

    assert_legacy_settings_snapshot_is_read_only(mixed);
}

#[test]
fn legacy_v1_same_model_duplicate_cards_remain_read_only() {
    let mut duplicate = legacy_v1_single_model_cards();
    duplicate["builtin_agents"][1]["model"] = serde_json::json!("model-a");

    assert_legacy_settings_snapshot_is_read_only(duplicate);
}

#[test]
fn legacy_v1_duplicate_card_ids_remain_read_only() {
    let mut duplicate = legacy_v1_single_model_cards();
    duplicate["builtin_agents"][1]["id"] = serde_json::json!("builtin-first");

    assert_legacy_settings_snapshot_is_read_only(duplicate);
}

#[test]
fn legacy_v1_different_preset_cards_are_not_merged() {
    let mut different = legacy_v1_single_model_cards();
    different["builtin_agents"][1]["preset"] = serde_json::json!("custom");
    let raw = serde_json::to_string(&different).expect("encode legacy settings");
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(&mut state, Some(&raw), None, |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(load.loaded);
    assert!(!load.unsupported_version);
    assert_eq!(state.editor_ui.agent_settings.builtin_agents.len(), 2);
    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].preset,
        BuiltinAgentPresetKey::Doubao
    );
    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[1].preset,
        BuiltinAgentPresetKey::Custom
    );
    assert_eq!(writes.len(), 2, "v1 credentials still split from settings");
    let credentials: serde_json::Value =
        serde_json::from_str(&writes[0].1).expect("credential JSON");
    assert_eq!(credentials["builtin_agents"].as_array().unwrap().len(), 2);
}

#[test]
fn legacy_v1_some_and_missing_preset_cards_remain_read_only() {
    let mut mixed = legacy_v1_single_model_cards();
    mixed["builtin_agents"][1]
        .as_object_mut()
        .expect("built-in agent")
        .remove("preset");

    assert_legacy_settings_snapshot_is_read_only(mixed);
}

#[test]
fn legacy_v1_unknown_card_fields_remain_read_only_before_migration() {
    let mut future = legacy_v1_single_model_cards();
    future["builtin_agents"][1]["future_auth"] = serde_json::json!({"token":"future-secret"});

    assert_legacy_settings_snapshot_is_read_only(future);
}

#[test]
fn mixed_legacy_and_multi_model_provider_cards_remain_read_only() {
    let mut mixed = valid_credentials();
    mixed["builtin_agents"][0]["models"] =
        serde_json::json!(["current-model", "current-model-fast"]);
    mixed["builtin_agents"]
        .as_array_mut()
        .expect("built-in agents")
        .push(serde_json::json!({
            "id": "builtin-2",
            "preset": "custom",
            "display_name": "Conflicting legacy card",
            "kind": "openai-compat",
            "api_key": "sk-current-secret",
            "model": "legacy-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }));

    assert_credential_snapshot_is_read_only(mixed);
}

#[test]
fn same_model_legacy_provider_cards_remain_read_only() {
    let mut duplicate = valid_credentials();
    duplicate["builtin_agents"]
        .as_array_mut()
        .expect("built-in agents")
        .push(serde_json::json!({
            "id": "builtin-2",
            "preset": "custom",
            "display_name": "True duplicate",
            "kind": "openai-compat",
            "api_key": "sk-current-secret",
            "model": "current-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }));

    assert_credential_snapshot_is_read_only(duplicate);
}

#[test]
fn different_preset_legacy_provider_cards_are_not_migrated() {
    let mut different = valid_credentials();
    different["builtin_agents"]
        .as_array_mut()
        .expect("built-in agents")
        .push(serde_json::json!({
            "id": "builtin-2",
            "preset": "doubao",
            "display_name": "Different discovery preset",
            "kind": "openai-compat",
            "api_key": "sk-current-secret",
            "model": "other-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }));
    let raw = serde_json::to_string(&different).expect("encode legacy credentials");
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(&mut state, None, Some(&raw), |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(load.loaded);
    assert!(!load.unsupported_version);
    assert_eq!(state.editor_ui.agent_settings.builtin_agents.len(), 2);
    assert!(
        writes.is_empty(),
        "different presets need no migration rewrite"
    );
}

#[test]
fn duplicate_builtin_provider_ids_remain_read_only_even_for_distinct_backends() {
    let mut duplicate = valid_credentials();
    duplicate["builtin_agents"]
        .as_array_mut()
        .expect("built-in agents")
        .push(serde_json::json!({
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Different backend",
            "kind": "openai-compat",
            "api_key": "sk-other-secret",
            "model": "other-model",
            "base_url": "https://other.example/v1",
            "enabled": true
        }));

    assert_credential_snapshot_is_read_only(duplicate);
}

#[test]
fn padded_builtin_provider_ids_remain_read_only() {
    let mut padded = valid_credentials();
    padded["builtin_agents"][0]["id"] = serde_json::json!(" builtin-1 ");

    assert_credential_snapshot_is_read_only(padded);
}

#[test]
fn explicit_preset_loads_without_model_based_reclassification() {
    let mut credentials = valid_credentials();
    credentials["builtin_agents"][0]["preset"] = serde_json::json!("doubao");
    credentials["builtin_agents"][0]["model"] = serde_json::json!("ark-code-latest");
    credentials["builtin_agents"][0]["base_url"] =
        serde_json::json!("https://ark.cn-beijing.volces.com/api/coding");
    let raw = serde_json::to_string(&credentials).expect("encode credentials");
    let mut state = EditorState::new();
    let mut writes = 0;

    let load = load_into_with(&mut state, None, Some(&raw), |_, _| {
        writes += 1;
        true
    });

    assert!(load.loaded);
    assert!(!load.unsupported_version);
    assert_eq!(writes, 0, "an already-canonical snapshot needs no rewrite");
    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].preset,
        BuiltinAgentPresetKey::Doubao
    );
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
    assert_eq!(writes[0].0, super::settings_storage_key());
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
    assert_eq!(writes[0].0, super::credential_storage_key());
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
