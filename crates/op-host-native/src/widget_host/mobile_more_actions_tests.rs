//! Direct dispatch coverage for the compact More sheet's feature and utility
//! entries. Geometry may reorder these actions, but their destinations must
//! stay identical.

use super::*;

const WIDTH: f32 = 390.0;
const HEIGHT: f32 = 844.0;

#[test]
fn more_ai_opens_the_dedicated_touch_sheet() {
    let mut host = touch_host(EditorSizeClass::Compact);
    assert!(press_more_entry(
        &mut host,
        MobileMoreEntry::Ai,
        WIDTH,
        HEIGHT,
    ));
    assert_eq!(
        host.editor_state().editor_ui.mobile_sheet,
        Some(MobileSheetKind::Ai)
    );
}

#[test]
fn more_language_and_settings_keep_their_native_and_engine_destinations() {
    let mut language = touch_host(EditorSizeClass::Compact);
    assert!(press_more_entry(
        &mut language,
        MobileMoreEntry::Language,
        WIDTH,
        HEIGHT,
    ));
    assert!(language.editor_state().editor_ui.pending_language_picker);
    assert_eq!(language.editor_state().editor_ui.mobile_sheet, None);

    let mut settings = touch_host(EditorSizeClass::Compact);
    assert!(press_more_entry(
        &mut settings,
        MobileMoreEntry::Settings,
        WIDTH,
        HEIGHT,
    ));
    assert!(settings.editor_state().editor_ui.agent_settings_open);
    assert_eq!(settings.editor_state().editor_ui.mobile_sheet, None);
}

#[test]
fn more_variables_and_promoted_export_still_open_their_tools() {
    let mut variables = touch_host(EditorSizeClass::Compact);
    assert!(press_more_entry(
        &mut variables,
        MobileMoreEntry::Variables,
        WIDTH,
        HEIGHT,
    ));
    assert!(variables.editor_state().editor_ui.variables_panel_open);
    assert_eq!(variables.editor_state().editor_ui.mobile_sheet, None);

    let mut export = touch_host(EditorSizeClass::Compact);
    assert!(press_more_entry(
        &mut export,
        MobileMoreEntry::Export,
        WIDTH,
        HEIGHT,
    ));
    assert!(export.editor_state().editor_ui.export_dialog_open);
    assert_eq!(export.editor_state().editor_ui.mobile_sheet, None);
}
