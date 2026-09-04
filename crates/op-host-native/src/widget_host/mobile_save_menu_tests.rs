//! Native-host regressions for the mobile Save / Save As menu entries, the
//! save-name dialog's modal press ownership, and the removal of Run
//! (Preview) from the touch chrome.

use super::*;

#[test]
fn more_save_and_save_as_queue_the_shared_file_actions_and_close() {
    let (width, height) = (834.0, 1112.0);

    for (entry, expected) in [
        (MobileMoreEntry::SaveFile, FileAction::Save),
        (MobileMoreEntry::SaveAsFile, FileAction::SaveAs),
    ] {
        let mut host = touch_host(EditorSizeClass::Medium);
        assert!(press_more_entry(&mut host, entry, width, height));
        assert_eq!(
            host.editor_state().editor_ui.pending_file_action,
            Some(expected)
        );
        assert_eq!(host.editor_state().editor_ui.mobile_sheet, None);
    }
}

/// The mobile chrome must not offer Run: the More grid carries no Preview
/// tile any more, and no press on any of its tiles can enter preview mode.
#[test]
fn more_sheet_offers_no_run_preview_entry() {
    let (width, height) = (390.0, 844.0);
    let tile_count =
        MobileMoreEntry::visible(touch_host(EditorSizeClass::Compact).editor_state()).len();

    for index in 0..tile_count {
        // Fresh host per tile so one tile's surface cannot shadow the next
        // press.
        let mut host = touch_host(EditorSizeClass::Compact);
        host.editor_state_mut().editor_ui.mobile_sheet = Some(MobileSheetKind::More);
        let panel = host.mobile_sheet_rect(width, height, MobileSheetKind::More);
        let point = center(op_editor_ui::widgets::mobile_chrome::more_entry_rect(
            host.editor_state(),
            panel,
            index,
        ));
        host.apply_press(point.x, point.y, width, height);
        assert!(!host.preview_active(), "tile {index} entered preview mode");
    }
}

/// While the save-name dialog is open it is fully modal: presses outside
/// it dismiss, and the confirm button raises the one-shot confirmation.
#[test]
fn save_name_dialog_owns_presses_and_confirms() {
    let mut host = touch_host(EditorSizeClass::Compact);
    let (width, height) = (390.0, 844.0);
    host.editor_state_mut()
        .editor_ui
        .save_name_dialog
        .open_with("poster", false, 0);

    let dialog = op_editor_ui::widgets::SaveNameDialog::anchored(
        width,
        host_canvas_geometry::touch_app_bar_height(host.editor_state()),
    );
    // Confirm button raises the confirmation and keeps the dialog open for
    // the FFI drain.
    let rect = dialog.rect();
    let confirm = Point2D::new(
        rect.origin.x + rect.size.x - 20.0 - 48.0,
        rect.origin.y + rect.size.y - 16.0 - 18.0,
    );
    assert!(host.apply_press(confirm.x, confirm.y, width, height));
    assert!(host.editor_state().editor_ui.save_name_dialog.confirmed);
    assert!(host.editor_state().editor_ui.save_name_dialog.open);

    // A press outside the card cancels the prompt entirely.
    host.editor_state_mut().editor_ui.save_name_dialog.confirmed = false;
    assert!(host.apply_press(width / 2.0, height - 40.0, width, height));
    assert!(!host.editor_state().editor_ui.save_name_dialog.open);
}
