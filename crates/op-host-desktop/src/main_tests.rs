//! Redraw-scheduler tests for the desktop event loop. Split out of
//! `main.rs` to keep that file under the 800-line cap.

use super::*;

#[test]
fn cursor_only_redraw_without_visible_state_change_skips_present() {
    let mut app = DesktopApp::new();
    app.redraw_pending = true;
    app.pending_cursor_move = Some((1200.0, 20.0));

    assert!(!app.prepare_redraw());
    assert!(!app.redraw_pending);
    assert!(app.pending_cursor_move.is_none());
}

#[test]
fn consumed_press_dirties_existing_cursor_redraw_without_second_request() {
    let mut app = DesktopApp::new();
    app.redraw_pending = true;

    assert!(!app.request_redraw(true));
    assert!(app.prepare_redraw());
}

#[test]
fn cursor_redraw_still_paints_when_layer_hover_changes() {
    let mut app = DesktopApp::new();
    app.redraw_pending = true;
    app.pending_cursor_move = Some((
        20.0,
        op_editor_ui::widgets::TOP_BAR_HEIGHT + 8.0 + 28.0 + 16.0,
    ));

    assert!(app.prepare_redraw());
}
