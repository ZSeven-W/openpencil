//! Account-partition isolation: what an empty partition must clear, and what
//! the rebuilt persistence baselines must preserve.
//!
//! Split out of `web_settings_tests.rs` at the 800-line cap; nested under it
//! so `use super::*` still reaches the shared helpers.

use super::*;

fn state_with_runtime_builtin_catalog() -> (EditorState, String) {
    let mut state = EditorState::new();
    let id = state.editor_ui.agent_settings.add_builtin_agent_config(
        "Provider",
        "sk-old",
        "fallback-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://api.openai.com/v1",
    );
    let settings = &mut state.editor_ui.agent_settings;
    settings.request_ready_builtin_model_catalog_refreshes(1);
    let request = settings
        .take_pending_builtin_model_catalog_refresh()
        .expect("ready provider queues a catalog request");
    let expected = settings
        .builtin_model_catalog_config_for_request(&request)
        .expect("request resolves its provider");
    assert!(
        settings.apply_builtin_model_catalog_refresh_outcome_if_current(
            &expected,
            &request,
            op_editor_core::BuiltinModelCatalogRefreshOutcome::Success {
                models: vec![op_editor_core::BuiltinModelOption::new(
                    "remote-model",
                    "Remote model",
                )],
            },
        )
    );
    assert!(!settings.builtin_model_catalog_options(&id).is_empty());
    (state, id)
}

#[test]
fn an_empty_partition_clears_the_previous_accounts_credentials() {
    // The account-switch leak: after switching, the in-memory state still held
    // account A's API keys, and an empty partition for B meant "keep whatever
    // was there" instead of "no keys".
    let mut state = EditorState::new();
    let mut writes = Vec::new();
    // A signs in and stores a key.
    let credential_json = r#"{"version":2,"builtin_agents":[{"id":"a1","preset":"custom",
        "display_name":"A's model","kind":"openai-compat","api_key":"sk-account-a",
        "model":"m","base_url":"https://api.example.com/v1","enabled":true}],
        "image_gen_profiles":[],"active_image_gen_profile_id":null,"openverse_oauth":null}"#;
    super::storage::load_into_with(&mut state, None, Some(credential_json), |k, v| {
        writes.push((k.to_string(), v.to_string()));
        true
    });
    assert!(
        !state.editor_ui.agent_settings.builtin_agents.is_empty(),
        "A's credential must load in the first place"
    );

    // B signs in: their partition is empty.
    super::storage::load_into_with(&mut state, None, None, |_, _| true);
    assert!(
        state.editor_ui.agent_settings.builtin_agents.is_empty(),
        "B must not inherit A's API keys from an empty partition"
    );
}

#[test]
fn account_reset_clears_runtime_builtin_catalogs() {
    let (mut state, id) = state_with_runtime_builtin_catalog();

    super::reset_account_scoped_settings(&mut state);

    assert!(state
        .editor_ui
        .agent_settings
        .builtin_model_catalog_options(&id)
        .is_empty());
}

#[test]
fn either_builtin_snapshot_replacement_clears_runtime_catalogs() {
    let settings_snapshot = r#"{"version":1,"builtin_agents":[{"id":"builtin-1",
        "preset":"custom","display_name":"New","kind":"openai-compat",
        "api_key":"sk-new","model":"new-model",
        "base_url":"https://api.example.com/v1","enabled":true}]}"#;
    let (mut state, old_id) = state_with_runtime_builtin_catalog();
    super::apply_json(&mut state, settings_snapshot).expect("legacy settings snapshot");
    assert!(state
        .editor_ui
        .agent_settings
        .builtin_model_catalog_options(&old_id)
        .is_empty());

    let credential_snapshot = r#"{"version":2,"builtin_agents":[],
        "image_gen_profiles":[],"active_image_gen_profile_id":null,
        "openverse_oauth":null}"#;
    let (mut state, old_id) = state_with_runtime_builtin_catalog();
    super::apply_credential_json(&mut state, credential_snapshot).expect("credential snapshot");
    assert!(state
        .editor_ui
        .agent_settings
        .builtin_model_catalog_options(&old_id)
        .is_empty());
}

#[test]
fn an_empty_partition_resets_account_scoped_settings_to_defaults() {
    // `apply_payload` only writes fields the blob carries, so an empty
    // partition used to leave A's locale, recent files and provider config in
    // place for B.
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ja;
    state.editor_ui.recent_files = vec![RecentFile {
        path: "/a/secret-project.op".into(),
        modified_at: 1,
    }];
    state.editor_ui.agent_settings.mcp_server.port = 4242;
    state.editor_ui.agent_settings.openverse_client_id = "a-client".into();

    super::reset_account_scoped_settings(&mut state);
    super::storage::load_into_with(&mut state, None, None, |_, _| true);

    let defaults = op_editor_core::EditorUiState::default();
    let default_agents = op_editor_core::AgentSettings::default();
    assert_eq!(state.editor_ui.locale, defaults.locale, "locale must reset");
    assert!(
        state.editor_ui.recent_files.is_empty(),
        "B must not see A's recent files"
    );
    assert_eq!(
        state.editor_ui.agent_settings.mcp_server.port,
        default_agents.mcp_server.port
    );
    assert!(state
        .editor_ui
        .agent_settings
        .openverse_client_id
        .is_empty());
}

#[test]
fn a_populated_partition_still_wins_over_the_defaults() {
    // The reset must not erase the partition being loaded — defaults first,
    // then the target snapshot on top.
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ja;
    let settings = r#"{"version":1,"locale":"fr","mcp_port":5150}"#;

    super::reset_account_scoped_settings(&mut state);
    super::storage::load_into_with(&mut state, Some(settings), None, |_, _| true);

    assert_eq!(state.editor_ui.locale, Locale::Fr);
    assert_eq!(state.editor_ui.agent_settings.mcp_server.port, 5150);
}

#[test]
fn rebuilt_baselines_keep_saving_and_keep_failing_closed() {
    // The regression this pins: setting `settings_fingerprint` to `None`
    // makes the save gate (`if let Some(..)`) skip forever, so nothing was
    // ever persisted again after an account switch.
    let mut state = EditorState::new();
    let healthy = super::storage::load_into_with(&mut state, None, None, |_, _| true);
    assert!(
        healthy.initial_settings_fingerprint(&state).is_some(),
        "a healthy partition must leave the save path enabled"
    );
    assert!(!healthy
        .initial_fingerprint(&state)
        .write_disabled_for_test());

    // …and an unsupported snapshot must still fail closed rather than being
    // "reset" into a writable baseline.
    let unsupported = r#"{"version":9999}"#;
    let mut state = EditorState::new();
    let blocked = super::storage::load_into_with(&mut state, Some(unsupported), None, |_, _| true);
    assert!(
        blocked.initial_settings_fingerprint(&state).is_none(),
        "an unsupported snapshot must keep settings writes disabled"
    );
    assert!(blocked
        .initial_fingerprint(&state)
        .write_disabled_for_test());
}

// ── Theme is device-level, not account-level ──────────────────────────────
//
// Light or dark is a property of the screen you are sitting at. These pin the
// four things the split has to get right: a switch keeps the theme, the
// account reset does not touch it, existing data migrates once, and a change
// still persists.

#[test]
fn switching_accounts_keeps_the_devices_theme() {
    // A is on dark; B's blob says light. The screen must stay dark — B signing
    // in on this laptop does not change what the room looks like.
    let mut state = EditorState::new();
    state.editor_ui.theme_mode = ThemeMode::Dark;

    let b_blob = r#"{"version":1,"theme":"light","locale":"en-US"}"#;
    super::storage::load_into_with(&mut state, Some(b_blob), None, |_, _| true);
    assert_eq!(
        state.editor_ui.theme_mode,
        ThemeMode::Dark,
        "the incoming account's blob must not be read for theme"
    );

    // And the device key, when one exists, is what decides.
    assert_eq!(
        super::theme::resolve(
            Some(ThemeMode::Dark),
            Some(ThemeMode::Light),
            ThemeMode::Light
        ),
        ThemeMode::Dark
    );
}

#[test]
fn the_account_reset_leaves_the_theme_alone() {
    // Everything else here goes back to default so an empty partition cannot
    // inherit the previous account's values. Theme is the one exception, and
    // it has to be, or every switch would flip the screen.
    let mut state = EditorState::new();
    state.editor_ui.theme_mode = ThemeMode::Dark;
    state.editor_ui.locale = Locale::ZhCn;

    super::reset_account_scoped_settings(&mut state);

    assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);
    assert_eq!(
        state.editor_ui.locale,
        op_editor_core::EditorUiState::default().locale,
        "locale IS account-scoped and must still reset"
    );
}

#[test]
fn an_existing_accounts_theme_migrates_to_the_device_once() {
    // First run after the split: no device key, so the theme already stored in
    // the account blob is adopted rather than reset to default.
    let blob = r#"{"version":1,"theme":"dark","locale":"en-US"}"#;
    assert_eq!(
        super::payload_theme_of(Some(blob)),
        Some(ThemeMode::Dark),
        "the blob's theme is still readable as the migration source"
    );
    assert_eq!(
        super::theme::resolve(None, super::payload_theme_of(Some(blob)), ThemeMode::Light),
        ThemeMode::Dark
    );

    // A blob from a future version carries nothing this build should trust.
    assert_eq!(
        super::payload_theme_of(Some(r#"{"version":99,"theme":"dark"}"#)),
        None
    );
    assert_eq!(super::payload_theme_of(Some("not json")), None);
    assert_eq!(super::payload_theme_of(None), None);
}

#[test]
fn the_account_blob_still_carries_a_theme_for_older_builds() {
    // Write-only compatibility: an older bundle reads theme ONLY from here, so
    // dropping the field would silently reset the theme on a downgrade or in a
    // second tab still running the old build.
    let mut state = EditorState::new();
    state.editor_ui.theme_mode = ThemeMode::Dark;
    let json = serde_json::to_string(&super::to_payload(&state)).expect("payload serialises");
    let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid json");
    assert_eq!(parsed["theme"], serde_json::json!("dark"));
}

#[test]
fn a_theme_change_still_persists_after_an_account_switch() {
    // The B2c shape: a reset that leaves the save path unable to fire again.
    // The device theme is saved outside the settings fingerprint entirely, so
    // an unwritable partition blob cannot disable it.
    super::theme::reset_last_written_for_test();
    let mut state = EditorState::new();
    state.editor_ui.theme_mode = ThemeMode::Dark;

    let mut written = Vec::new();
    let mut record = |_: &str, value: &str| {
        written.push(value.to_string());
        true
    };
    assert_eq!(
        super::theme::save_if_changed_with(&state, &mut record),
        Ok(true)
    );

    // Switch accounts: reset + load, neither of which may disturb the theme.
    super::reset_account_scoped_settings(&mut state);
    super::storage::load_into_with(
        &mut state,
        Some(r#"{"version":1,"theme":"light"}"#),
        None,
        |_, _| true,
    );
    assert_eq!(state.editor_ui.theme_mode, ThemeMode::Dark);

    // The user now picks light. It must still reach storage.
    state.editor_ui.theme_mode = ThemeMode::Light;
    assert_eq!(
        super::theme::save_if_changed_with(&state, &mut record),
        Ok(true)
    );
    assert_eq!(written, vec!["dark".to_string(), "light".to_string()]);
}
