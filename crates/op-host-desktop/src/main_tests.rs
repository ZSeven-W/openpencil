//! Redraw-scheduler tests for the desktop event loop. Split out of
//! `main.rs` to keep that file under the 800-line cap.

use super::*;

#[test]
fn cursor_only_redraw_without_visible_state_change_skips_present() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;
    app.pending_cursor_move = Some((1200.0, 20.0));

    assert!(!app.prepare_redraw());
    assert!(!app.redraw_pending);
    assert!(app.pending_cursor_move.is_none());
}

#[test]
fn consumed_press_dirties_existing_cursor_redraw_without_second_request() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;

    assert!(!app.request_redraw(true));
    assert!(app.prepare_redraw());
}

#[test]
fn cursor_redraw_still_paints_when_layer_hover_changes() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;
    app.pending_cursor_move = Some((
        20.0,
        op_editor_ui::widgets::TOP_BAR_HEIGHT + 8.0 + 28.0 + 16.0,
    ));

    assert!(app.prepare_redraw());
}

#[test]
fn fresh_app_fits_blank_frame_like_ts_canvas_init() {
    let app = DesktopApp::new(None);
    let v = app.host.editor_state().viewport;

    // Golden fit values track `property_panel_width` (the right rail is
    // shown on the fresh app, so the canvas region = 1440 − panel). At
    // the TS-matching `w-64` (256 px) panel the blank frame fits at 0.68.
    assert!((v.zoom - 0.68).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 158.0).abs() < 1e-2, "pan_y {}", v.pan_y);
}

#[test]
fn fresh_app_refits_blank_frame_to_actual_window_size_once() {
    let mut app = DesktopApp::new(None);
    app.viewport_width = 1000.0;
    app.viewport_height = 700.0;

    assert!(app.fit_initial_blank_frame_to_actual_viewport());
    let v = app.host.editor_state().viewport;
    assert!((v.zoom - 0.31333333).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 204.66666).abs() < 1e-2, "pan_y {}", v.pan_y);

    app.viewport_width = 1200.0;
    app.viewport_height = 800.0;
    assert!(!app.fit_initial_blank_frame_to_actual_viewport());
    let unchanged = app.host.editor_state().viewport;
    assert_eq!(v, unchanged);
}
