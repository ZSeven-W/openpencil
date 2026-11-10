//! The coordinate invariant with the motion timeline open: the panel's
//! strip leaves the canvas region, so hit-tests, the wheel anchor and the
//! floating chrome all agree it is not canvas.

use super::*;

const W: f32 = 1440.0;
const H: f32 = 900.0;

fn editor() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.home.hide();
    state
}

fn with_timeline(height: f32) -> EditorState {
    let mut state = editor();
    state.editor_ui.motion_timeline.open = true;
    state.editor_ui.motion_timeline.height = height;
    state
}

#[test]
fn the_panel_takes_the_bottom_of_the_canvas_area() {
    let closed = editor();
    let open = with_timeline(220.0);
    let (x, y, w, h) = canvas_region(&closed, W, H);
    assert_eq!(motion_timeline_rect(&closed, W, H), None);
    assert_eq!(canvas_region(&open, W, H), (x, y, w, h - 220.0));
    let panel = motion_timeline_rect(&open, W, H).expect("panel");
    assert_eq!(panel, Rect::xywh(x, y + h - 220.0, w, 220.0));
    // Never more than 60% of the area.
    let tall = with_timeline(5_000.0);
    assert_eq!(motion_timeline_rect(&tall, W, H).unwrap().size.y, h * 0.6);
}

#[test]
fn pointers_over_the_panel_are_not_over_the_canvas() {
    let open = with_timeline(220.0);
    let panel = motion_timeline_rect(&open, W, H).expect("panel");
    let inside = Point2D::new(panel.origin.x + 40.0, panel.origin.y + 40.0);
    assert!(!over_canvas(&open, inside.x, inside.y, W, H));
    assert!(over_canvas(&editor(), inside.x, inside.y, W, H));
    assert_eq!(canvas_doc_point(&open, inside.x, inside.y, W, H), None);
    // Just above the panel is still canvas.
    assert!(over_canvas(&open, inside.x, panel.origin.y - 2.0, W, H));
}

#[test]
fn the_wheel_anchor_and_toolbar_keep_their_place_while_the_status_bar_lifts() {
    let closed = editor();
    let open = with_timeline(220.0);
    // The panel only trims the bottom: the origin the wheel zoom anchors
    // to and the toolbar hanging from it do not move.
    assert_eq!(canvas_origin(&open), canvas_origin(&closed));
    assert_eq!(
        canvas_doc_point_unclamped(&open, 600.0, 300.0),
        canvas_doc_point_unclamped(&closed, 600.0, 300.0)
    );
    assert_eq!(toolbar_rect(&open, 300.0), toolbar_rect(&closed, 300.0));
    let status_closed = status_bar_rect(&closed, W, H).expect("status bar");
    let status_open = status_bar_rect(&open, W, H).expect("status bar");
    assert_eq!(status_open.origin.y, status_closed.origin.y - 220.0);
}

#[test]
fn preview_and_touch_layouts_hide_the_panel() {
    let mut preview = with_timeline(220.0);
    preview.editor_ui.preview.mode = true;
    assert_eq!(motion_timeline_rect(&preview, W, H), None);
    assert!(!motion_timeline_shown(&preview));
}
