use super::*;
use op_editor_core::ImageTestStatus;

#[path = "settings_io_checked_tests.rs"]
mod checked_tests;
#[path = "settings_io_save_tests.rs"]
mod save_tests;

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
fn persisted_locale_parsing_uses_shared_bcp47_rules() {
    assert_eq!(
        resolve_persisted_locale(Locale::Ru, Some("EN_us.UTF-8")),
        Locale::EnUs
    );
    assert_eq!(
        resolve_persisted_locale(Locale::Ru, Some("zh-Hant-HK")),
        Locale::ZhTw
    );
    assert_eq!(
        resolve_persisted_locale(Locale::Ru, Some("in-ID")),
        Locale::Id
    );
}

#[test]
fn settings_payload_uses_shared_stable_locale_codes() {
    for locale in Locale::ALL {
        let mut state = EditorState::new();
        state.editor_ui.locale = locale;
        assert_eq!(to_payload(&state).locale.as_deref(), Some(locale.code()));
    }
}

#[test]
fn host_locale_seed_respects_process_environment_precedence() {
    let cases = [
        (
            "lc-all",
            Some("fr_FR.UTF-8"),
            Some("de_DE"),
            Some("ja_JP"),
            Locale::Fr,
        ),
        (
            "lc-messages",
            None,
            Some("zh_Hant.UTF-8"),
            Some("ja_JP"),
            Locale::ZhTw,
        ),
        (
            "empty-lc-all",
            Some(""),
            Some("tr_TR"),
            Some("ja_JP"),
            Locale::Tr,
        ),
        (
            "c-stops-fallback",
            Some("C"),
            Some("zh_CN"),
            Some("ja_JP"),
            Locale::EnUs,
        ),
        (
            "posix-stops-fallback",
            None,
            Some("POSIX"),
            Some("ja_JP"),
            Locale::EnUs,
        ),
        (
            "unsupported-stops-fallback",
            Some("xx_ZZ"),
            Some("zh_CN"),
            Some("ja_JP"),
            Locale::EnUs,
        ),
    ];

    for (case, lc_all, lc_messages, lang, expected) in cases {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .arg("--exact")
            .arg("settings_io::settings_io_tests::system_locale_environment_probe")
            .arg("--ignored")
            .env_remove("LC_ALL")
            .env_remove("LC_MESSAGES")
            .env_remove("LANG")
            .env("OPENPENCIL_EXPECTED_SYSTEM_LOCALE", expected.code());
        if let Some(value) = lc_all {
            command.env("LC_ALL", value);
        }
        if let Some(value) = lc_messages {
            command.env("LC_MESSAGES", value);
        }
        if let Some(value) = lang {
            command.env("LANG", value);
        }

        let output = command.output().expect("locale probe process starts");
        assert!(
            output.status.success(),
            "case={case}\nstdout={}\nstderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
#[ignore = "executed in isolated subprocesses by host_locale_seed_respects_process_environment_precedence"]
fn system_locale_environment_probe() {
    let Ok(expected) = std::env::var("OPENPENCIL_EXPECTED_SYSTEM_LOCALE") else {
        return;
    };
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ru;

    seed_system_locale(&mut state);

    assert_eq!(state.editor_ui.locale, Locale::from_tag(&expected).unwrap());
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
    // Connect Claude (0) + Antigravity (4) + DeepSeek Harness (6,
    // the append-only tail slot), leave the rest off.
    let mut src = EditorState::new();
    src.editor_ui.agent_settings.connected = [true, false, false, false, true, false, true];
    // Serialize → JSON → deserialize, the real on-disk path.
    let json = serde_json::to_string(&to_payload(&src)).unwrap();
    let payload: SettingsPayload = serde_json::from_str(&json).unwrap();
    let mut dst = EditorState::new();
    apply_payload(&mut dst, payload);
    assert_eq!(
        dst.editor_ui.agent_settings.connected,
        [true, false, false, false, true, false, true]
    );
}

#[test]
fn six_cli_mcp_payload_migrates_to_new_cli_count() {
    let payload: SettingsPayload = serde_json::from_str(
        r#"{"version":1,"mcp_cli_enabled":[true,false,true,false,true,true]}"#,
    )
    .unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.agent_settings.mcp_cli_enabled = [true; 13];

    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.mcp_cli_enabled,
        [true, false, false, true, true, false, false, false, false, false, false, false, false]
    );
}

#[test]
fn seven_cli_mcp_payload_keeps_its_toggles_and_leaves_the_new_clis_off() {
    let payload: SettingsPayload = serde_json::from_str(
        r#"{"version":1,"mcp_cli_enabled":[true,false,true,false,true,false,true]}"#,
    )
    .unwrap();
    let mut dst = EditorState::new();
    dst.editor_ui.agent_settings.mcp_cli_enabled = [true; 13];

    apply_payload(&mut dst, payload);

    assert_eq!(
        dst.editor_ui.agent_settings.mcp_cli_enabled,
        [true, false, true, false, true, false, true, false, false, false, false, false, false]
    );
}

#[path = "settings_io_payload_tests.rs"]
mod payload_tests;
