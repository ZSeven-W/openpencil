use super::*;
use op_editor_core::{EditorState, Locale, ThemeMode};

#[test]
fn host_locale_query_accepts_only_exact_bcp47_values() {
    assert_eq!(host_locale_from_query("?locale=zh-CN"), Some(Locale::ZhCn));
    assert_eq!(
        host_locale_from_query("?embed=vscode&locale=en-US"),
        Some(Locale::EnUs)
    );
    for query in ["", "?locale=zh", "?locale=en", "?locale=EN", "?locale="] {
        assert_eq!(host_locale_from_query(query), None, "{query}");
    }
}

#[test]
fn transient_host_locale_does_not_change_persistence_fingerprint() {
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::ZhCn;
    let before = fingerprint(&state);
    state.editor_ui.set_host_locale_override(Some(Locale::EnUs));
    assert_eq!(state.editor_ui.effective_locale(), Locale::EnUs);
    assert_eq!(state.editor_ui.locale, Locale::ZhCn);
    assert_eq!(fingerprint(&state), before);
}

#[test]
fn settings_payload_restores_locale_but_never_theme() {
    // Theme moved out of the account-scoped payload semantics: it is a device
    // preference resolved from its own unpartitioned key, so reading it here
    // would let an incoming account flip the screen. The field is still
    // written for older builds — see `partition_tests` — and is still readable
    // as the one-time migration source through `payload_theme_of`.
    let payload = r#"{"version":1,"theme":"light","locale":"en-US"}"#;
    let mut state = EditorState::new();
    let before_theme = state.editor_ui.theme_mode;

    apply_json(&mut state, payload).expect("settings payload should parse");

    assert_eq!(state.editor_ui.locale, Locale::EnUs);
    assert_eq!(state.editor_ui.theme_mode, before_theme);
    assert_eq!(payload_theme_of(Some(payload)), Some(ThemeMode::Light));
}

#[test]
fn fingerprint_changes_when_theme_changes() {
    let mut state = EditorState::new();
    let before = fingerprint(&state);

    state.editor_ui.theme_mode = ThemeMode::Light;

    assert_ne!(before, fingerprint(&state));
}

#[test]
fn settings_payload_round_trips_recent_files_and_mcp_preferences() {
    let mut src = EditorState::new();
    src.editor_ui.theme_mode = ThemeMode::Light;
    src.editor_ui.locale = Locale::Ja;
    src.editor_ui.agent_settings.mcp_server.port = 4321;
    src.editor_ui.agent_settings.mcp_cli_enabled[1] = true;
    src.editor_ui.agent_settings.images_advanced_open = true;
    src.editor_ui.agent_settings.auto_update_enabled = false;
    src.editor_ui.agent_settings.experimental_features_enabled = true;
    src.editor_ui.recent_files = vec![
        RecentFile {
            path: "/tmp/a.op".into(),
            modified_at: 1,
        },
        RecentFile {
            path: "/tmp/b.op".into(),
            modified_at: 2,
        },
    ];
    let json = serde_json::to_string(&to_payload(&src)).expect("settings serialize");
    let mut dst = EditorState::new();

    apply_json(&mut dst, &json).expect("settings payload parses");

    // Theme is written into the blob (older builds read it) but not applied
    // from it — it is a device preference. `payload_theme_of` is the only
    // reader, and it is used once, for the migration.
    assert_eq!(payload_theme_of(Some(&json)), Some(ThemeMode::Light));
    assert_eq!(dst.editor_ui.locale, Locale::Ja);
    assert_eq!(dst.editor_ui.agent_settings.mcp_server.port, 4321);
    assert!(dst.editor_ui.agent_settings.mcp_cli_enabled[1]);
    assert!(dst.editor_ui.agent_settings.images_advanced_open);
    assert!(!dst.editor_ui.agent_settings.auto_update_enabled);
    assert!(dst.editor_ui.agent_settings.experimental_features_enabled);
    assert_eq!(dst.editor_ui.recent_files.len(), 2);
    assert_eq!(dst.editor_ui.recent_files[0].path, "/tmp/a.op");
}

#[test]
fn legacy_mcp_flags_drop_gemini_without_shifting_google_antigravity() {
    let legacy = r#"{
        "version":1,
        "mcp_cli_enabled":[true,false,true,false,true,false,true,true]
    }"#;
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let loaded = load_into_with(&mut state, Some(legacy), None, |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(
        !loaded.loaded,
        "ordinary settings do not contain a credential payload"
    );
    assert_eq!(
        state.editor_ui.agent_settings.mcp_cli_enabled,
        [true, false, false, true, false, true, true, false, false, false, false, false, false]
    );
    let rewritten = writes
        .iter()
        .find_map(|(key, json)| (*key == super::settings_storage_key()).then_some(json))
        .expect("legacy positional settings should be rewritten");
    let value: serde_json::Value = serde_json::from_str(rewritten).expect("rewritten settings");
    assert_eq!(
        value["mcp_cli_enabled"],
        serde_json::json!([
            true, false, false, true, false, true, true, false, false, false, false, false, false
        ])
    );
}

#[test]
fn separate_settings_and_credential_payloads_round_trip_their_own_fields() {
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.add_builtin_agent_config(
        "MiniMax",
        "sk-test",
        "MiniMax-M2.7",
        BuiltinAgentKind::Anthropic,
        "https://api.minimaxi.com/anthropic",
    );
    assert!(src.editor_ui.agent_settings.builtin_agents[0].add_model("MiniMax-M3"));
    let image_profile_id = src.editor_ui.agent_settings.add_image_gen_profile();
    let profile = &mut src.editor_ui.agent_settings.image_gen_profiles[0];
    profile.name = "Gemini Image".into();
    profile.provider = ImageGenProvider::Gemini;
    profile.api_key = "image-key".into();
    profile.model = "gemini-image".into();
    assert!(src
        .editor_ui
        .agent_settings
        .set_active_image_gen_profile(&image_profile_id));
    let settings_json = serde_json::to_string(&to_payload(&src)).expect("settings serialize");
    let credential_json = credentials_json(&src).expect("credentials serialize");
    assert!(!settings_json.contains("acp_agents"));
    assert!(!credential_json.contains("acp_agents"));
    let mut dst = EditorState::new();

    apply_json(&mut dst, &settings_json).expect("settings payload parses");
    apply_credential_json(&mut dst, &credential_json).expect("credential payload parses");

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].display_name,
        "MiniMax"
    );
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].preset,
        BuiltinAgentPresetKey::MiniMax
    );
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].models,
        ["MiniMax-M2.7", "MiniMax-M3"]
    );
    assert!(dst.editor_ui.agent_settings.acp_agents.is_empty());
    assert_eq!(dst.editor_ui.agent_settings.image_gen_profiles.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.image_gen_profiles[0].provider,
        ImageGenProvider::Gemini
    );
    assert_eq!(
        dst.editor_ui
            .agent_settings
            .active_image_gen_profile_id
            .as_deref(),
        Some(image_profile_id.as_str())
    );
}

#[test]
fn credential_fingerprint_ignores_theme_but_tracks_every_secret_category() {
    let mut state = EditorState::new();
    let baseline = credential_fingerprint(&state);

    state.editor_ui.theme_mode = ThemeMode::Light;
    assert_eq!(baseline, credential_fingerprint(&state));

    state.editor_ui.agent_settings.openverse_client_secret = "openverse-secret".into();
    assert_ne!(baseline, credential_fingerprint(&state));

    let mut state = EditorState::new();
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Private",
        "sk-test",
        "private-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    assert_ne!(baseline, credential_fingerprint(&state));

    let mut state = EditorState::new();
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.image_gen_profiles[0].api_key = "image-key".into();
    assert_ne!(baseline, credential_fingerprint(&state));
}

#[test]
fn general_settings_fingerprint_ignores_credentials_from_another_tab() {
    let mut state = EditorState::new();
    let baseline = fingerprint(&state);

    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Private",
        "sk-separate-store",
        "private-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    state.editor_ui.agent_settings.openverse_client_secret = "openverse".into();

    assert_eq!(baseline, fingerprint(&state));
}

#[test]
fn credential_payload_contains_credentials_but_not_document_or_recent_files() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Private",
        "sk-test",
        "private-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    state.editor_ui.agent_settings.add_image_gen_profile();
    state.editor_ui.agent_settings.image_gen_profiles[0].api_key = "image-key".into();
    state.editor_ui.agent_settings.openverse_client_id = "openverse-client".into();
    state.editor_ui.agent_settings.openverse_client_secret = "openverse-secret".into();
    state.editor_ui.recent_files.push(RecentFile {
        path: "/tmp/must-not-leak.op".into(),
        modified_at: 1,
    });

    let json = credentials_json(&state).expect("credential JSON");
    let payload: serde_json::Value = serde_json::from_str(&json).expect("credential payload");

    assert!(json.contains("sk-test"));
    assert!(json.contains("image-key"));
    assert!(json.contains("openverse-secret"));
    assert!(!json.contains("recent_files"));
    assert!(!json.contains("must-not-leak"));
    assert!(!json.contains("document"));
    assert_eq!(payload["version"], 2);
    assert!(payload.get("owner_id").is_none());
}

#[test]
fn web_credential_snapshots_omit_acp_configuration_entirely() {
    let mut state = EditorState::new();
    let mut env = std::collections::BTreeMap::new();
    env.insert("TOKEN".into(), "acp-env-secret".into());
    state.editor_ui.agent_settings.add_acp_agent_config(
        "Local ACP",
        op_editor_core::AcpConnectionType::Local,
        "/bin/sh",
        vec!["-c".into(), "acp-command-secret".into()],
        env,
        None,
        true,
    );

    let json = server_credentials_json(&state).expect("server credential JSON");
    let payload: serde_json::Value = serde_json::from_str(&json).expect("credential payload");

    assert!(payload.get("acp_agents").is_none());
    assert!(!json.contains("/bin/sh"));
    assert!(!json.contains("acp-command-secret"));
    assert!(!json.contains("acp-env-secret"));
}

#[test]
fn failed_local_storage_write_does_not_advance_the_settings_fingerprint() {
    let mut state = EditorState::new();
    let mut baseline = fingerprint(&state);
    state.editor_ui.theme_mode = ThemeMode::Light;

    assert!(!save_if_changed_with(&state, &mut baseline, |_| false));
    assert_ne!(baseline, fingerprint(&state));

    assert!(save_if_changed_with(&state, &mut baseline, |_| true));
    assert_eq!(baseline, fingerprint(&state));
}

#[test]
fn failed_credential_storage_write_does_not_advance_its_fingerprint() {
    let mut state = EditorState::new();
    let mut baseline = credential_fingerprint(&state);
    state.editor_ui.agent_settings.openverse_client_secret = "must-retry".into();

    assert!(save_credentials_if_changed_with(&state, &mut baseline, |_, _| false).is_none());
    assert_ne!(baseline, credential_fingerprint(&state));

    assert!(save_credentials_if_changed_with(&state, &mut baseline, |_, _| true).is_some());
    assert_eq!(baseline, credential_fingerprint(&state));
}

#[test]
fn general_settings_payload_never_contains_credentials() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Private",
        "sk-must-not-leak",
        "private-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    state.editor_ui.agent_settings.openverse_client_secret = "openverse-secret".into();
    state.editor_ui.agent_settings.add_acp_agent_config(
        "Private ACP",
        op_editor_core::AcpConnectionType::Remote,
        "command-secret",
        vec!["--token=args-secret".into()],
        std::collections::BTreeMap::from([("TOKEN".into(), "env-secret".into())]),
        Some("https://user:url-secret@example.test/acp".into()),
        true,
    );

    let json = serde_json::to_string(&to_payload(&state)).unwrap();

    assert!(!json.contains("sk-must-not-leak"));
    assert!(!json.contains("openverse-secret"));
    assert!(!json.contains("command-secret"));
    assert!(!json.contains("args-secret"));
    assert!(!json.contains("env-secret"));
    assert!(!json.contains("url-secret"));
    assert!(!json.contains("Private ACP"));
    assert!(!json.contains("acp_agents"));
    assert!(!json.contains("\"openverse_oauth\""));
    assert!(!json.contains("\"builtin_agents\""));
    assert!(!json.contains("\"image_gen_profiles\""));
    assert!(!json.contains("\"active_image_gen_profile_id\""));
}

#[test]
fn an_empty_credential_snapshot_round_trips_as_authoritative_empty_state() {
    let source = EditorState::new();
    let json = credentials_json(&source).unwrap();
    let mut target = EditorState::new();
    target.editor_ui.agent_settings.add_builtin_agent_config(
        "Stale",
        "sk-stale",
        "stale-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );

    apply_credential_json(&mut target, &json).unwrap();

    assert!(target.editor_ui.agent_settings.builtin_agents.is_empty());
}

#[test]
fn unsupported_settings_version_is_not_treated_as_a_loaded_snapshot() {
    let mut state = EditorState::new();

    assert!(apply_json(&mut state, r#"{"version":999}"#).is_err());
    assert!(apply_credential_json(&mut state, r#"{"version":999}"#).is_err());
}

#[test]
fn future_credential_snapshot_fails_closed_without_overwriting_the_raw_value() {
    let future = r#"{
        "version": 3,
        "builtin_agents": [{
            "id": "future-agent",
            "api_key": "future-secret-must-survive-downgrade"
        }]
    }"#;
    let mut state = EditorState::new();
    let mut writes = 0;

    let load = load_into_with(&mut state, None, Some(future), |_, _| {
        writes += 1;
        true
    });

    assert!(!load.loaded);
    assert!(!load.write_pending);
    assert_eq!(writes, 0, "an older client must preserve a future snapshot");
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
}

#[test]
fn future_credential_snapshot_remains_read_only_after_a_provider_edit() {
    let future = r#"{
        "version": 3,
        "builtin_agents": [{
            "id": "future-agent",
            "api_key": "future-secret-must-survive-downgrade"
        }]
    }"#;
    let mut state = EditorState::new();
    let load = load_into_with(&mut state, None, Some(future), |_, _| true);
    let mut baseline = load.initial_fingerprint(&state);

    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Edited",
        "sk-edited",
        "edited-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    let mut writes = 0;
    let saved = save_credentials_if_changed_with(&state, &mut baseline, |key, _| {
        if *key == super::credential_storage_key() {
            writes += 1;
        }
        true
    });

    assert!(saved.is_none());
    assert_eq!(
        writes, 0,
        "an older client must not overwrite a future credential snapshot"
    );
}

#[test]
fn future_general_settings_snapshot_is_never_sanitized_by_an_older_client() {
    let future = r#"{
        "version": 2,
        "theme": "light",
        "builtin_agents": [{
            "id": "future-agent",
            "api_key": "future-general-secret-must-survive-downgrade"
        }]
    }"#;
    let mut state = EditorState::new();
    let mut writes = 0;

    let load = load_into_with(&mut state, Some(future), None, |_, _| {
        writes += 1;
        true
    });

    assert!(!load.loaded);
    assert!(load.unsupported_version);
    assert_eq!(writes, 0, "an older client must preserve future settings");
    assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
}

#[test]
fn future_general_settings_remain_read_only_after_a_theme_change() {
    let future = r#"{
        "version": 2,
        "theme": "dark",
        "future_field": "must-survive-downgrade"
    }"#;
    let mut state = EditorState::new();
    let load = load_into_with(&mut state, Some(future), None, |_, _| true);
    let mut baseline = load.initial_settings_fingerprint(&state);

    state.editor_ui.theme_mode = ThemeMode::Light;
    let mut writes = 0;
    if let Some(baseline) = baseline.as_mut() {
        let _ = save_if_changed_with(&state, baseline, |_| {
            writes += 1;
            true
        });
    }

    assert_eq!(
        writes, 0,
        "a mounted older client must not overwrite v2 settings"
    );
}

#[test]
fn corrupt_separate_snapshot_never_revives_legacy_credentials() {
    let legacy = r#"{
        "version": 1,
        "theme": "light",
        "builtin_agents": [{
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Legacy",
            "kind": "openai-compat",
            "api_key": "sk-legacy",
            "model": "legacy-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }]
    }"#;
    let mut state = EditorState::new();

    let loaded = apply_stored_snapshots(&mut state, Some(legacy), Some("{corrupt"));

    assert_eq!(loaded.source, StoredCredentialSource::InvalidSeparate);
    assert!(loaded.sanitize_legacy_settings);
    // The blob's `"theme": "light"` is NOT applied — theme is device-level.
    // What this test is really about is the credentials, which must not come
    // back from the legacy blob.
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
}

#[test]
fn corrupt_separate_snapshot_queues_its_persisted_empty_replacement_for_server_sync() {
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(&mut state, None, Some("{corrupt"), |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(
        load.loaded,
        "the healed empty snapshot must be synced at mount"
    );
    assert!(!load.write_pending);
    let credential_write = writes
        .iter()
        .find(|(key, _)| *key == super::credential_storage_key())
        .expect("the corrupt credential snapshot is replaced");
    let value: serde_json::Value =
        serde_json::from_str(&credential_write.1).expect("replacement is valid JSON");
    assert_eq!(value["builtin_agents"], serde_json::json!([]));
    assert!(value.get("acp_agents").is_none());
    assert_eq!(value["image_gen_profiles"], serde_json::json!([]));
}

#[test]
fn valid_separate_snapshot_loads_without_overwriting_future_general_settings() {
    let credentials = r#"{
        "version": 2,
        "builtin_agents": [{
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Current",
            "kind": "openai-compat",
            "api_key": "sk-current",
            "model": "current-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null
    }"#;
    let mut state = EditorState::new();

    let loaded = apply_stored_snapshots(
        &mut state,
        Some(r#"{"version":999,"theme":"light"}"#),
        Some(credentials),
    );

    assert_eq!(loaded.source, StoredCredentialSource::Separate);
    assert!(!loaded.sanitize_legacy_settings);
    assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);
    assert_eq!(state.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].api_key,
        "sk-current"
    );
}

#[test]
fn valid_empty_separate_snapshot_is_authoritative() {
    let mut state = EditorState::new();
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Stale",
        "sk-stale",
        "stale-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    let empty = r#"{
        "version": 2,
        "builtin_agents": [],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null
    }"#;

    let loaded = apply_stored_snapshots(&mut state, None, Some(empty));

    assert_eq!(loaded.source, StoredCredentialSource::Separate);
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
}

#[test]
fn unknown_legacy_credential_records_are_preserved_read_only() {
    let legacy = r#"{
        "version": 1,
        "theme": "light",
        "builtin_agents": [{
            "id": "builtin-invalid",
            "display_name": "Invalid but secret-bearing",
            "kind": "unsupported-provider",
            "api_key": "sk-must-be-sanitized",
            "model": "unused",
            "base_url": "https://example.test/v1",
            "enabled": true
        }]
    }"#;
    let mut state = EditorState::new();

    let loaded = apply_stored_snapshots(&mut state, Some(legacy), None);

    assert_eq!(loaded.source, StoredCredentialSource::None);
    assert!(loaded.unsupported_settings_version);
    assert!(!loaded.sanitize_legacy_settings);
    assert!(state.editor_ui.agent_settings.builtin_agents.is_empty());
    assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);
}

#[test]
fn malformed_legacy_credential_records_are_preserved_read_only() {
    let legacy = r#"{
        "version": 1,
        "theme": "light",
        "builtin_agents": [{
            "api_key": "sk-malformed-must-be-sanitized",
            "enabled": "not-a-boolean"
        }]
    }"#;
    let mut state = EditorState::new();

    let loaded = apply_stored_snapshots(&mut state, Some(legacy), None);

    assert_eq!(loaded.source, StoredCredentialSource::None);
    assert!(loaded.unsupported_settings_version);
    assert!(!loaded.sanitize_legacy_settings);
    assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);
}

#[test]
fn unrelated_invalid_legacy_field_blocks_credential_migration_without_data_loss() {
    let legacy = r#"{
        "version": 1,
        "theme": "light",
        "mcp_cli_enabled": [true],
        "builtin_agents": [{
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Legacy",
            "kind": "openai-compat",
            "api_key": "sk-must-survive-invalid-general-field",
            "model": "legacy-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }]
    }"#;
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(&mut state, Some(legacy), None, |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(!load.loaded);
    assert!(load.unsupported_version);
    assert!(
        writes.is_empty(),
        "migration must preserve the raw snapshot when unrelated fields are invalid"
    );
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
        save_credentials_if_changed_with(&state, &mut credential_baseline, |key, json| {
            writes.push((key.to_string(), json.to_string()));
            true
        })
        .is_none()
    );
    assert!(writes.is_empty());
}

#[test]
fn failed_legacy_credential_write_stays_loaded_and_retries_before_sanitizing() {
    let legacy = r#"{
        "version": 1,
        "theme": "light",
        "builtin_agents": [{
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Legacy",
            "kind": "openai-compat",
            "api_key": "sk-retry-migration",
            "model": "legacy-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }]
    }"#;
    let mut state = EditorState::new();
    let mut first_writes = Vec::new();

    let load = load_into_with(&mut state, Some(legacy), None, |key, json| {
        first_writes.push((key.to_string(), json.to_string()));
        false
    });

    assert!(load.loaded);
    assert!(load.write_pending);
    assert_eq!(first_writes.len(), 1);
    assert_eq!(first_writes[0].0, super::credential_storage_key());
    assert!(first_writes[0].1.contains("sk-retry-migration"));

    let mut baseline = load.initial_fingerprint(&state);
    assert_ne!(baseline, credential_fingerprint(&state));
    let mut retry_writes = Vec::new();
    let saved = save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
        retry_writes.push((key.to_string(), json.to_string()));
        true
    });

    // The mount-time `loaded` signal already authorizes the initial daemon
    // sync. Retrying local persistence must not emit a second sync payload.
    assert!(saved.is_none());
    assert_eq!(retry_writes.len(), 2);
    assert_eq!(retry_writes[0].0, super::credential_storage_key());
    assert_eq!(retry_writes[1].0, super::settings_storage_key());
    assert!(!retry_writes[1].1.contains("sk-retry-migration"));
    assert_eq!(baseline, credential_fingerprint(&state));
}

#[test]
fn pending_legacy_migration_blocks_an_ordinary_settings_write() {
    let legacy = r#"{
        "version": 1,
        "theme": "light",
        "builtin_agents": [{
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Legacy",
            "kind": "openai-compat",
            "api_key": "sk-only-durable-copy",
            "model": "legacy-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }]
    }"#;
    let mut state = EditorState::new();
    let load = load_into_with(&mut state, Some(legacy), None, |_, _| false);
    let mut credential_baseline = load.initial_fingerprint(&state);
    let mut settings_baseline = fingerprint(&state);
    // Locale, not theme: theme is no longer restored from the blob, so setting
    // it here would leave the fingerprint unchanged and the test would pass
    // for the wrong reason. Locale is genuinely account-scoped — and it has to
    // be a value the default is not, or this is the same no-op trap.
    assert_ne!(state.editor_ui.locale, Locale::EnUs);
    state.editor_ui.locale = Locale::EnUs;

    assert!(
        save_credentials_if_changed_with(&state, &mut credential_baseline, |_, _| false).is_none()
    );
    let mut general_write_attempted = false;
    if !credential_migration_pending(&credential_baseline) {
        let _ = save_if_changed_with(&state, &mut settings_baseline, |_| {
            general_write_attempted = true;
            true
        });
    }

    assert!(!general_write_attempted);
    assert_ne!(settings_baseline, fingerprint(&state));
}

#[path = "web_settings_partition_tests.rs"]
mod partition_tests;
