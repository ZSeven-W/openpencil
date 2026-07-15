use super::*;

#[path = "settings_io_checked_tests.rs"]
mod checked_tests;

fn settings_save_test_root(case: &str) -> std::path::PathBuf {
    let sequence = SETTINGS_TEMP_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "openpencil-settings-save-{}-{sequence}-{case}",
        std::process::id()
    ))
}

#[test]
fn persisted_locale_overrides_system_locale_seed() {
    assert_eq!(
        resolve_persisted_locale(Locale::Ru, Some("en-US")),
        Locale::EnUs
    );
}

#[test]
fn missing_or_invalid_persisted_locale_preserves_system_locale_seed() {
    for persisted in [None, Some(""), Some("unknown")] {
        assert_eq!(
            resolve_persisted_locale(Locale::Ru, persisted),
            Locale::Ru,
            "persisted locale {persisted:?} must preserve the caller's seed"
        );
    }
}

#[test]
fn apply_payload_persisted_locale_overrides_system_locale_seed() {
    let payload: SettingsPayload =
        serde_json::from_str(r#"{"version":1,"locale":"en-US"}"#).unwrap();
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ru;

    apply_payload(&mut state, payload);

    assert_eq!(state.editor_ui.locale, Locale::EnUs);
}

#[test]
fn apply_payload_missing_or_invalid_locale_preserves_system_locale_seed() {
    for json in [
        r#"{"version":1}"#,
        r#"{"version":1,"locale":""}"#,
        r#"{"version":1,"locale":"unknown"}"#,
    ] {
        let payload: SettingsPayload = serde_json::from_str(json).unwrap();
        let mut state = EditorState::new();
        state.editor_ui.locale = Locale::Ru;

        apply_payload(&mut state, payload);

        assert_eq!(
            state.editor_ui.locale,
            Locale::Ru,
            "payload {json} must preserve the caller's seed"
        );
    }
}

#[test]
fn locale_change_updates_settings_fingerprint() {
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ru;
    let before = fingerprint(&state);

    state.editor_ui.locale = Locale::Ja;

    assert_ne!(before, fingerprint(&state));
}

#[test]
fn imported_agents_are_excluded_from_persistence() {
    // A user-entered agent must persist; an auto-imported (e.g. Zode)
    // agent must NOT, so its API key never lands in settings.json.
    let mut state = EditorState::new();
    let manual = state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Manual", "manual-key", "m1");
    let imported = state
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Imported", "zode-key", "m2");
    state
        .editor_ui
        .agent_settings
        .imported_agent_ids
        .insert(imported.clone());

    let payload = to_payload(&state);
    let persisted = payload.builtin_agents.unwrap();
    let ids: Vec<_> = persisted.iter().map(|a| a.id.clone()).collect();
    assert!(ids.contains(&manual), "user-entered agent should persist");
    assert!(
        !ids.contains(&imported),
        "imported agent (and its key) must not be persisted"
    );
    assert!(
        persisted.iter().all(|a| a.api_key != "zode-key"),
        "imported API key must never reach settings.json"
    );
}

#[test]
fn connected_state_round_trips_through_payload() {
    // Connect Claude (0) + Gemini (4), leave the rest off.
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.connected = [true, false, false, false, true, true, false];
    // Serialize → JSON → deserialize, the real on-disk path.
    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);
    assert_eq!(
        dst.editor_ui.agent_settings.connected,
        [true, false, false, false, true, true, false]
    );
}

#[test]
fn six_cli_mcp_payload_migrates_to_new_cli_count() {
    let payload: SettingsPayload = serde_json::from_str(
        r#"{"version":1,"mcp_cli_enabled":[true,false,true,false,true,true]}"#,
    )
    .unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.agent_settings.mcp_cli_enabled = [true; 8];

    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.mcp_cli_enabled,
        [true, false, true, false, true, true, false, false]
    );
}

#[test]
fn five_provider_connected_payload_migrates_to_new_provider_count() {
    let payload: SettingsPayload =
        serde_json::from_str(r#"{"version":1,"connected":[true,false,false,false,true]}"#).unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.agent_settings.connected = [false, false, false, false, false, true, true];
    apply_payload(&mut dst, payload);
    assert_eq!(
        dst.editor_ui.agent_settings.connected,
        [true, false, false, false, true, false, false]
    );
}

#[test]
fn legacy_settings_without_connected_field_default_to_disconnected() {
    // A settings.json written before the `connected` field
    // existed must still load — the missing field defaults to
    // all-disconnected rather than failing the parse.
    let legacy = r#"{"version":1,"theme":"dark","locale":"en-US"}"#;
    let payload: SettingsPayload = serde_json::from_str(legacy).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);
    assert_eq!(dst.editor_ui.agent_settings.connected, [false; 7]);
}

#[test]
fn builtin_agents_round_trip_through_payload() {
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.add_builtin_agent_config(
        "MiniMax",
        "sk-test",
        "MiniMax-M2.7",
        BuiltinAgentKind::Anthropic,
        "https://api.minimaxi.com/anthropic",
    );

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].display_name,
        "MiniMax"
    );
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].api_key,
        "sk-test"
    );
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].preset,
        BuiltinAgentPresetKey::MiniMax
    );
}

#[test]
fn duplicate_builtin_agents_are_deduped_on_load() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-1",
                "display_name": "MINIMAX",
                "kind": "openai-compat",
                "api_key": "sk-test",
                "model": "MiniMax-M2.7",
                "base_url": "https://api.minimaxi.com/v1",
                "enabled": true
            },
            {
                "id": "builtin-2",
                "display_name": "MINIMAX",
                "kind": "openai-compat",
                "api_key": "sk-test",
                "model": "MiniMax-M2.7",
                "base_url": "https://api.minimaxi.com/v1",
                "enabled": true
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].id,
        "builtin-1"
    );
    assert_eq!(dst.editor_ui.agent_settings.next_builtin_agent_id, 2);
}

#[test]
fn duplicate_auto_named_builtin_agents_are_deduped_on_load() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-5",
                "display_name": "Built-in Agent 5",
                "kind": "anthropic",
                "api_key": "sk-test",
                "model": "claude-sonnet-4-5",
                "base_url": "https://api.anthropic.com",
                "enabled": true
            },
            {
                "id": "builtin-6",
                "display_name": "Built-in Agent 6",
                "kind": "anthropic",
                "api_key": "sk-test",
                "model": "claude-sonnet-4-5",
                "base_url": "https://api.anthropic.com",
                "enabled": true
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].display_name,
        "Built-in Agent 5"
    );
    assert_eq!(dst.editor_ui.agent_settings.next_builtin_agent_id, 6);
}

#[test]
fn builtin_agent_payload_without_api_key_loads_as_empty_key() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-2",
                "preset": "bailian-coding",
                "display_name": "百炼CP",
                "kind": "openai-compat",
                "model": "qwen3-coder-plus",
                "base_url": "https://coding.dashscope.aliyuncs.com/v1",
                "enabled": false
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    let agent = &dst.editor_ui.agent_settings.builtin_agents[0];
    assert_eq!(agent.display_name, "百炼CP");
    assert!(agent.api_key.is_empty());
    assert!(!agent.enabled);
}

#[test]
fn acp_agents_round_trip_through_payload() {
    let mut src = EditorState::new();
    let mut env = std::collections::BTreeMap::new();
    env.insert("ACP_TOKEN".into(), "secret".into());
    src.editor_ui.agent_settings.add_acp_agent_config(
        "Design Agent",
        AcpConnectionType::Local,
        "/usr/local/bin/design-agent",
        vec!["--stdio".into()],
        env,
        None,
        true,
    );
    src.editor_ui.agent_settings.add_acp_agent_config(
        "Remote Agent",
        AcpConnectionType::Remote,
        "",
        Vec::new(),
        std::collections::BTreeMap::new(),
        Some("ws://localhost:8100".into()),
        false,
    );

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.acp_agents.len(), 2);
    let local = &dst.editor_ui.agent_settings.acp_agents[0];
    assert_eq!(local.display_name, "Design Agent");
    assert_eq!(local.connection_type, AcpConnectionType::Local);
    assert_eq!(local.command, "/usr/local/bin/design-agent");
    assert!(!local.connected);
    assert_eq!(local.args, vec!["--stdio"]);
    assert_eq!(
        local.env.get("ACP_TOKEN").map(String::as_str),
        Some("secret")
    );
    let remote = &dst.editor_ui.agent_settings.acp_agents[1];
    assert_eq!(remote.connection_type, AcpConnectionType::Remote);
    assert_eq!(remote.url.as_deref(), Some("ws://localhost:8100"));
    assert!(!remote.enabled);
    assert_eq!(dst.editor_ui.agent_settings.next_acp_agent_id, 3);
}

#[test]
fn image_generation_profiles_round_trip_through_payload() {
    let mut src = EditorState::new();
    let first = src.editor_ui.agent_settings.add_image_gen_profile();
    let second = src.editor_ui.agent_settings.add_image_gen_profile();
    let second_profile = &mut src.editor_ui.agent_settings.image_gen_profiles[1];
    second_profile.name = "Gemini Image".into();
    second_profile.provider = ImageGenProvider::Gemini;
    second_profile.api_key = "image-key".into();
    second_profile.model = "gemini-image".into();
    second_profile.base_url = Some("https://images.example/v1".into());
    assert!(src
        .editor_ui
        .agent_settings
        .set_active_image_gen_profile(&second));

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.image_gen_profiles.len(), 2);
    assert_eq!(
        dst.editor_ui
            .agent_settings
            .active_image_gen_profile_id
            .as_deref(),
        Some(second.as_str())
    );
    assert_eq!(dst.editor_ui.agent_settings.image_gen_profiles[0].id, first);
    assert_eq!(
        dst.editor_ui.agent_settings.image_gen_profiles[1].provider,
        ImageGenProvider::Gemini
    );
    assert_eq!(
        dst.editor_ui.agent_settings.image_gen_profiles[1].base_url,
        Some("https://images.example/v1".into())
    );
}

#[test]
fn openverse_oauth_round_trips_through_payload() {
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.openverse_client_id = "client-id".into();
    src.editor_ui.agent_settings.openverse_client_secret = "client-secret".into();
    src.editor_ui.agent_settings.openverse_credential_owner = Some("browser".into());

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.openverse_client_id,
        "client-id"
    );
    assert_eq!(
        dst.editor_ui.agent_settings.openverse_client_secret,
        "client-secret"
    );
    assert_eq!(
        dst.editor_ui
            .agent_settings
            .openverse_credential_owner
            .as_deref(),
        Some("browser")
    );
}

#[test]
fn checked_save_reports_an_unwritable_parent() {
    let root = settings_save_test_root("unwritable-parent");
    let blocking_parent = root.join("not-a-directory");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(&blocking_parent, b"block").unwrap();
    let path = blocking_parent.join("settings.json");

    let result = save_checked_to_path(&EditorState::new(), &path);

    assert!(result.is_err());
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn checked_save_does_not_reuse_an_existing_temporary_path() {
    let root = settings_save_test_root("unique-temp");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("settings.json");
    let tmp = root.join("settings.json.tmp");
    std::fs::write(&tmp, b"do not replace").unwrap();

    let result = save_checked_to_path(&EditorState::new(), &path);

    assert!(result.is_ok());
    assert_eq!(std::fs::read(&tmp).unwrap(), b"do not replace");
    assert!(path.is_file());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn checked_save_removes_the_temporary_file_after_a_replace_failure() {
    let root = settings_save_test_root("replace-failure");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("settings.json");
    std::fs::create_dir(&path).unwrap();

    let result = save_checked_to_path(&EditorState::new(), &path);

    assert!(result.is_err());
    let remaining = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(remaining, vec![path]);
    let _ = std::fs::remove_dir_all(&root);
}

#[cfg(unix)]
#[test]
fn checked_save_enforces_private_permissions_on_the_final_file() {
    use std::os::unix::fs::PermissionsExt;

    let root = settings_save_test_root("private-mode");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("settings.json");
    let old_tmp = root.join("settings.json.tmp");
    std::fs::write(&old_tmp, b"old temporary contents").unwrap();
    std::fs::set_permissions(&old_tmp, std::fs::Permissions::from_mode(0o666)).unwrap();

    save_checked_to_path(&EditorState::new(), &path).unwrap();

    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn auto_update_preference_round_trips_through_payload() {
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.auto_update_enabled = false;

    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);

    assert!(!dst.editor_ui.agent_settings.auto_update_enabled);
}

#[test]
fn legacy_ark_coding_payload_with_doubao_preset_migrates_to_ark_coding() {
    let settings = r#"{
        "version": 1,
        "builtin_agents": [
            {
                "id": "builtin-3",
                "preset": "doubao",
                "display_name": "方舟CP",
                "kind": "anthropic",
                "api_key": "sk-test",
                "model": "ark-code-latest",
                "base_url": "https://ark.cn-beijing.volces.com/api/coding",
                "enabled": true
            }
        ]
    }"#;
    let payload: SettingsPayload = serde_json::from_str(settings).unwrap();
    let mut dst = EditorState::new();

    apply_payload(&mut dst, payload);

    assert_eq!(dst.editor_ui.agent_settings.builtin_agents.len(), 1);
    assert_eq!(
        dst.editor_ui.agent_settings.builtin_agents[0].preset,
        BuiltinAgentPresetKey::ArkCoding
    );
}
