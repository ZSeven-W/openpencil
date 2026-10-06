//! Runtime-locale persistence boundaries for browser settings.

use super::*;
use op_editor_core::{EditorState, Locale};

#[test]
fn pending_locale_is_transient_and_does_not_change_persistence_fingerprint() {
    let mut state = EditorState::new();
    let before = fingerprint(&state);

    state.editor_ui.pending_locale = Some(Locale::Ja);

    assert_eq!(fingerprint(&state), before);
}

#[test]
fn account_partition_locale_waits_without_repainting_in_english() {
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ja;

    assert!(stage_partition_locale(&mut state, Locale::ZhCn, false));
    assert_eq!(state.editor_ui.locale, Locale::ZhCn);
    assert_eq!(state.editor_ui.pending_locale, Some(Locale::Ja));
    assert_eq!(
        state.editor_ui.locale_persistence_override,
        Some(Locale::Ja)
    );
    assert_eq!(fingerprint(&state).locale, Locale::Ja);
    assert_eq!(to_payload(&state).locale.as_deref(), Some("ja"));

    let mut ready_state = EditorState::new();
    ready_state.editor_ui.locale = Locale::Ja;
    assert!(!stage_partition_locale(
        &mut ready_state,
        Locale::ZhCn,
        true
    ));
    assert_eq!(ready_state.editor_ui.locale, Locale::Ja);
    assert_eq!(ready_state.editor_ui.pending_locale, None);
    assert_eq!(ready_state.editor_ui.locale_persistence_override, None);
}

#[test]
fn resetting_an_account_partition_drops_its_stale_pending_locale() {
    let mut state = EditorState::new();
    state.editor_ui.pending_locale = Some(Locale::De);
    state.editor_ui.locale_persistence_override = Some(Locale::Ja);

    reset_account_scoped_settings(&mut state);

    assert_eq!(state.editor_ui.pending_locale, None);
    assert_eq!(state.editor_ui.locale_persistence_override, None);
}

#[test]
fn unrelated_save_keeps_an_account_partitions_pending_locale() {
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ja;
    assert!(stage_partition_locale(&mut state, Locale::ZhCn, false));
    let mut before = fingerprint(&state);
    state.editor_ui.agent_settings.mcp_server.port += 1;
    let mut saved = None;

    assert!(save_if_changed_with(&state, &mut before, |json| {
        saved = Some(json.to_string());
        true
    }));

    let saved: serde_json::Value =
        serde_json::from_str(saved.as_deref().expect("settings payload")).unwrap();
    assert_eq!(saved["locale"], "ja");
}

#[test]
fn different_pending_picker_choice_does_not_overwrite_the_stored_partition_locale() {
    let mut state = EditorState::new();
    state.editor_ui.locale = Locale::Ja;
    assert!(stage_partition_locale(&mut state, Locale::ZhCn, false));

    assert!(state
        .editor_ui
        .set_locale_when_catalog_ready(Locale::De, false));

    assert_eq!(state.editor_ui.locale, Locale::ZhCn);
    assert_eq!(state.editor_ui.pending_locale, Some(Locale::De));
    assert_eq!(
        state.editor_ui.locale_persistence_override,
        Some(Locale::Ja)
    );
    assert_eq!(fingerprint(&state).locale, Locale::Ja);
    assert_eq!(to_payload(&state).locale.as_deref(), Some("ja"));
}
