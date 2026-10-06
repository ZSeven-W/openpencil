use super::*;
use op_i18n::Locale;

#[test]
fn locale_choice_stays_on_the_old_language_until_the_catalog_is_ready() {
    let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
    assert_eq!(ui.locale, Locale::ZhCn);

    assert!(select_locale(&mut ui, Locale::De, false));
    assert_eq!(ui.locale, Locale::ZhCn);
    assert_eq!(ui.pending_locale, Some(Locale::De));

    assert!(!select_locale(&mut ui, Locale::De, true));
    assert_eq!(ui.locale, Locale::De);
    assert_eq!(ui.pending_locale, None);
}

#[test]
fn choosing_a_ready_locale_cancels_an_older_pending_choice() {
    let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
    ui.pending_locale = Some(Locale::Ja);
    ui.locale_persistence_override = Some(Locale::Ja);

    assert!(!select_locale(&mut ui, Locale::EnUs, true));
    assert_eq!(ui.locale, Locale::EnUs);
    assert_eq!(ui.pending_locale, None);
    assert_eq!(ui.locale_persistence_override, None);
}

#[test]
fn changing_a_partition_pending_choice_preserves_the_stored_locale() {
    let mut ui = op_editor_core::editor_ui_state::EditorUiState::new();
    ui.pending_locale = Some(Locale::Ja);
    ui.locale_persistence_override = Some(Locale::Ja);

    assert!(select_locale(&mut ui, Locale::De, false));
    assert_eq!(ui.locale, Locale::ZhCn);
    assert_eq!(ui.pending_locale, Some(Locale::De));
    assert_eq!(ui.locale_persistence_override, Some(Locale::Ja));
}
