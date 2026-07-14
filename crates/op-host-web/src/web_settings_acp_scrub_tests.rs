use super::*;
use op_editor_core::EditorState;

#[test]
fn legacy_acp_in_separate_credentials_is_removed_without_losing_builtin_keys() {
    let credentials = r#"{
        "version": 2,
        "builtin_agents": [{
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Current",
            "kind": "openai-compat",
            "api_key": "sk-must-survive-acp-removal",
            "model": "current-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }],
        "acp_agents": [{
            "id": "acp-legacy",
            "command": "acp-command-secret",
            "env": {"TOKEN": "acp-env-secret"}
        }],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null
    }"#;
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(&mut state, None, Some(credentials), |key, json| {
        writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(load.loaded);
    assert!(!load.write_pending);
    assert_eq!(state.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        state.editor_ui.agent_settings.builtin_agents[0].api_key,
        "sk-must-survive-acp-removal"
    );
    assert!(state.editor_ui.agent_settings.acp_agents.is_empty());
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].0, CREDENTIAL_STORAGE_KEY);
    assert!(writes[0].1.contains("sk-must-survive-acp-removal"));
    assert!(!writes[0].1.contains("acp_agents"));
    assert!(!writes[0].1.contains("acp-command-secret"));
    assert!(!writes[0].1.contains("acp-env-secret"));
}

#[test]
fn separate_acp_scrub_does_not_touch_future_general_settings() {
    let future_general = r#"{
        "version": 2,
        "future_field": "must-survive"
    }"#;
    let credentials = r#"{
        "version": 2,
        "builtin_agents": [{
            "id": "builtin-1",
            "preset": "custom",
            "display_name": "Current",
            "kind": "openai-compat",
            "api_key": "sk-must-survive-acp-removal",
            "model": "current-model",
            "base_url": "https://api.openai.com/v1",
            "enabled": true
        }],
        "acp_agents": [{
            "id": "acp-legacy",
            "command": "acp-command-secret"
        }],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null
    }"#;
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(
        &mut state,
        Some(future_general),
        Some(credentials),
        |key, json| {
            writes.push((key.to_string(), json.to_string()));
            true
        },
    );

    assert!(load.loaded);
    assert!(load.unsupported_version);
    assert!(load.initial_settings_fingerprint(&state).is_none());
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].0, CREDENTIAL_STORAGE_KEY);
    assert!(writes[0].1.contains("sk-must-survive-acp-removal"));
    assert!(!writes[0].1.contains("acp_agents"));
    assert!(!writes[0].1.contains("acp-command-secret"));
}

#[test]
fn acp_is_scrubbed_from_general_and_separate_snapshots_in_one_ordered_migration() {
    let legacy_general = r#"{
        "version": 1,
        "connected": [true, true, true, true, true],
        "acp_agents": [{
            "id": "acp-general",
            "command": "general-acp-secret"
        }]
    }"#;
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
        "acp_agents": [{
            "id": "acp-separate",
            "command": "separate-acp-secret"
        }],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null
    }"#;
    let mut state = EditorState::new();
    let mut writes = Vec::new();

    let load = load_into_with(
        &mut state,
        Some(legacy_general),
        Some(credentials),
        |key, json| {
            writes.push((key.to_string(), json.to_string()));
            true
        },
    );

    assert!(load.loaded);
    assert!(!load.write_pending);
    assert_eq!(writes.len(), 2);
    assert_eq!(writes[0].0, CREDENTIAL_STORAGE_KEY);
    assert_eq!(writes[1].0, STORAGE_KEY);
    for (_, json) in &writes {
        assert!(!json.contains("acp_agents"));
        assert!(!json.contains("general-acp-secret"));
        assert!(!json.contains("separate-acp-secret"));
        assert!(!json.contains("connected"));
    }
    assert!(writes[0].1.contains("sk-current"));
}

#[test]
fn legacy_acp_configuration_is_removed_without_loading_or_migrating_it() {
    let legacy = r#"{
        "version": 1,
        "connected": [true, true, true, true, true],
        "acp_agents": [{
            "id": "acp-7",
            "display_name": "Legacy ACP",
            "connection_type": "remote",
            "command": "legacy-command-secret",
            "args": ["--token=legacy-args-secret"],
            "env": {"TOKEN": "legacy-env-secret"},
            "url": "https://user:legacy-url-secret@example.test/acp",
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
    assert!(!load.write_pending);
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].0, STORAGE_KEY);
    assert!(!writes[0].1.contains("acp_agents"));
    assert!(!writes[0].1.contains("connected"));
    assert!(!writes[0].1.contains("legacy-command-secret"));
    assert!(!writes[0].1.contains("legacy-args-secret"));
    assert!(!writes[0].1.contains("legacy-env-secret"));
    assert!(!writes[0].1.contains("legacy-url-secret"));
    assert!(state.editor_ui.agent_settings.acp_agents.is_empty());
    assert_eq!(state.editor_ui.agent_settings.connected, [false; 5]);
}

#[test]
fn failed_supported_acp_only_scrub_retries_without_authoritative_empty_credentials() {
    let settings = r#"{
        "version": 1,
        "connected": [true, true, true, true, true],
        "acp_agents": [{"command":"supported-acp-secret"}]
    }"#;
    let mut state = EditorState::new();
    let mut initial_writes = Vec::new();

    let load = load_into_with(&mut state, Some(settings), None, |key, json| {
        initial_writes.push((key.to_string(), json.to_string()));
        false
    });

    assert!(!load.loaded);
    assert!(!load.unsupported_version);
    assert!(load.write_pending);
    assert_eq!(initial_writes.len(), 1);
    assert_eq!(initial_writes[0].0, STORAGE_KEY);

    let mut baseline = load.initial_fingerprint(&state);
    let mut retry_writes = Vec::new();
    let saved = save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
        retry_writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(saved.is_none());
    assert!(!credential_migration_pending(&baseline));
    assert_eq!(retry_writes.len(), 1);
    assert_eq!(retry_writes[0].0, STORAGE_KEY);
    assert!(!retry_writes[0].1.contains("acp_agents"));
    assert!(!retry_writes[0].1.contains("connected"));
    assert!(!retry_writes[0].1.contains("supported-acp-secret"));

    state.editor_ui.agent_settings.add_builtin_agent_config(
        "New local credential",
        "sk-user-edit",
        "new-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    let mut user_writes = Vec::new();
    assert!(
        save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
            user_writes.push((key.to_string(), json.to_string()));
            true
        })
        .is_some()
    );
    assert_eq!(user_writes.len(), 1);
    assert_eq!(user_writes[0].0, CREDENTIAL_STORAGE_KEY);
    assert!(user_writes[0].1.contains("sk-user-edit"));
}

#[test]
fn failed_read_only_credential_scrub_retries_without_enabling_user_writes() {
    let credentials = r#"{
        "version": 2,
        "builtin_agents": [],
        "acp_agents": [{"command":"acp-secret"}],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null,
        "future_credentials": {"token":"future-secret-must-survive"}
    }"#;
    let mut state = EditorState::new();
    let mut initial_writes = Vec::new();

    let load = load_into_with(&mut state, None, Some(credentials), |key, json| {
        initial_writes.push((key.to_string(), json.to_string()));
        false
    });

    assert!(!load.loaded);
    assert!(load.unsupported_version);
    assert!(load.write_pending);
    assert_eq!(initial_writes.len(), 1);
    assert_eq!(initial_writes[0].0, CREDENTIAL_STORAGE_KEY);

    let mut baseline = load.initial_fingerprint(&state);
    let mut retry_writes = Vec::new();
    let saved = save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
        retry_writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(saved.is_none());
    assert!(!credential_migration_pending(&baseline));
    assert_eq!(retry_writes.len(), 1);
    assert_eq!(retry_writes[0].0, CREDENTIAL_STORAGE_KEY);
    assert!(!retry_writes[0].1.contains("acp_agents"));
    assert!(!retry_writes[0].1.contains("acp-secret"));
    assert!(retry_writes[0].1.contains("future-secret-must-survive"));

    state.editor_ui.agent_settings.add_builtin_agent_config(
        "Must stay read-only",
        "sk-must-not-write",
        "future-model",
        BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    let mut user_writes = 0;
    assert!(
        save_credentials_if_changed_with(&state, &mut baseline, |_, _| {
            user_writes += 1;
            true
        })
        .is_none()
    );
    assert_eq!(user_writes, 0);
}

#[test]
fn failed_read_only_general_scrub_retries_without_creating_credential_snapshot() {
    let settings = r#"{
        "version": 1,
        "acp_agents": [{"command":"general-acp-secret"}],
        "connected": [true, true, true, true, true],
        "future_settings": {"token":"future-general-secret-must-survive"}
    }"#;
    let mut state = EditorState::new();
    let mut initial_writes = Vec::new();

    let load = load_into_with(&mut state, Some(settings), None, |key, json| {
        initial_writes.push((key.to_string(), json.to_string()));
        false
    });

    assert!(!load.loaded);
    assert!(load.unsupported_version);
    assert!(load.write_pending);
    assert_eq!(initial_writes.len(), 1);
    assert_eq!(initial_writes[0].0, STORAGE_KEY);

    let mut baseline = load.initial_fingerprint(&state);
    let mut retry_writes = Vec::new();
    let saved = save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
        retry_writes.push((key.to_string(), json.to_string()));
        true
    });

    assert!(saved.is_none());
    assert!(!credential_migration_pending(&baseline));
    assert_eq!(retry_writes.len(), 1);
    assert_eq!(retry_writes[0].0, STORAGE_KEY);
    assert!(!retry_writes[0].1.contains("acp_agents"));
    assert!(!retry_writes[0].1.contains("connected"));
    assert!(!retry_writes[0].1.contains("general-acp-secret"));
    assert!(retry_writes[0]
        .1
        .contains("future-general-secret-must-survive"));
}

#[test]
fn partial_read_only_scrub_retry_clears_only_the_successful_write() {
    let settings = r#"{
        "version": 1,
        "acp_agents": [{"command":"general-acp-secret"}],
        "future_settings": {"token":"future-general-secret"}
    }"#;
    let credentials = r#"{
        "version": 2,
        "builtin_agents": [],
        "acp_agents": [{"command":"credential-acp-secret"}],
        "image_gen_profiles": [],
        "active_image_gen_profile_id": null,
        "openverse_oauth": null,
        "future_credentials": {"token":"future-credential-secret"}
    }"#;
    let mut state = EditorState::new();
    let mut initial_writes = Vec::new();
    let load = load_into_with(
        &mut state,
        Some(settings),
        Some(credentials),
        |key, json| {
            initial_writes.push((key.to_string(), json.to_string()));
            false
        },
    );

    assert!(load.write_pending);
    assert_eq!(initial_writes.len(), 1);
    assert_eq!(initial_writes[0].0, CREDENTIAL_STORAGE_KEY);

    let mut baseline = load.initial_fingerprint(&state);
    let mut first_retry = Vec::new();
    assert!(
        save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
            first_retry.push((key.to_string(), json.to_string()));
            key == CREDENTIAL_STORAGE_KEY
        })
        .is_none()
    );
    assert!(credential_migration_pending(&baseline));
    assert_eq!(first_retry.len(), 2);
    assert_eq!(first_retry[0].0, CREDENTIAL_STORAGE_KEY);
    assert_eq!(first_retry[1].0, STORAGE_KEY);
    assert!(first_retry[0].1.contains("future-credential-secret"));
    assert!(!first_retry[0].1.contains("credential-acp-secret"));

    let mut second_retry = Vec::new();
    assert!(
        save_credentials_if_changed_with(&state, &mut baseline, |key, json| {
            second_retry.push((key.to_string(), json.to_string()));
            true
        })
        .is_none()
    );
    assert!(!credential_migration_pending(&baseline));
    assert_eq!(second_retry.len(), 1);
    assert_eq!(second_retry[0].0, STORAGE_KEY);
    assert!(second_retry[0].1.contains("future-general-secret"));
    assert!(!second_retry[0].1.contains("general-acp-secret"));
}
