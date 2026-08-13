//! Focused native-host coverage for slideshow touch navigation.

use super::super::WidgetHostNative;
use op_editor_core::scene_template_catalog::TemplateScene;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_core::EditorState;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 390.0;
const VIEWPORT_H: f32 = 844.0;

/// Three 16:9 boards side by side — the shape a generated deck has.
const THREE_BOARD_DECK: &str = r##"{
    "version": "1.0.0",
    "children": [
        { "type": "frame", "id": "slide-1", "x": 0, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#ffffff"}], "children": [] },
        { "type": "frame", "id": "slide-2", "x": 2100, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#eeeeee"}], "children": [] },
        { "type": "frame", "id": "slide-3", "x": 4200, "y": 0,
          "width": 1920, "height": 1080,
          "fill": [{"type":"solid","color":"#dddddd"}], "children": [] }
    ]
}"##;

fn presenting_host() -> WidgetHostNative {
    let document = jian_ops_schema::load_str(THREE_BOARD_DECK)
        .expect("parse slideshow fixture")
        .value;
    let mut state = EditorState::from_document(document);
    state.editor_ui.scenario = Some(TemplateScene::Slides);
    state.editor_ui.touch = true;
    state.editor_ui.size_class = EditorSizeClass::Compact;
    let mut host = WidgetHostNative::new();
    host.install_imported_state(state);
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host.enter_preview((VIEWPORT_W, VIEWPORT_H));
    assert!(host.preview_slideshow_active(), "fixture presents");
    host
}

fn board_point(host: &WidgetHostNative) -> Point2D {
    let canvas = host.preview_canvas_rect(VIEWPORT_W, VIEWPORT_H);
    Point2D::new(
        canvas.origin.x + canvas.size.x / 2.0,
        canvas.origin.y + canvas.size.y / 4.0,
    )
}

fn board_on_screen(host: &WidgetHostNative) -> Option<String> {
    host.editor_state
        .preview_slideshow()
        .and_then(|slideshow| slideshow.current_board())
        .map(str::to_string)
}

fn drag(host: &mut WidgetHostNative, start: Point2D, delta: Point2D) {
    host.apply_cursor_move(start.x, start.y);
    host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H);
    host.apply_cursor_move(start.x + delta.x, start.y + delta.y);
    host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H);
}

#[test]
fn horizontal_swipes_step_the_deck_in_their_visual_direction() {
    let mut host = presenting_host();
    let start = board_point(&host);

    drag(&mut host, start, Point2D::new(-80.0, 12.0));
    assert_eq!(
        board_on_screen(&host).as_deref(),
        Some("slide-2"),
        "swiping left reveals the next slide"
    );

    drag(&mut host, start, Point2D::new(80.0, -12.0));
    assert_eq!(
        board_on_screen(&host).as_deref(),
        Some("slide-1"),
        "swiping right returns to the previous slide"
    );
}

#[test]
fn short_vertical_and_diagonal_drags_do_not_change_slides() {
    let mut host = presenting_host();
    let start = board_point(&host);

    // Past tap slop but below swipe activation: neither a tap nor a swipe.
    drag(&mut host, start, Point2D::new(-32.0, 2.0));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));

    // Plenty of travel, but the vertical axis owns this gesture.
    drag(&mut host, start, Point2D::new(-24.0, 100.0));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));

    // A near-diagonal drag is intentionally ambiguous and also stays put.
    drag(&mut host, start, Point2D::new(-80.0, 72.0));
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));
}

#[test]
fn system_cancel_discards_an_armed_swipe_and_its_late_release() {
    let mut host = presenting_host();
    let start = board_point(&host);

    host.apply_cursor_move(start.x, start.y);
    host.apply_press(start.x, start.y, VIEWPORT_W, VIEWPORT_H);
    host.apply_cursor_move(start.x - 100.0, start.y + 4.0);
    assert!(host.slideshow_press_screen.is_some(), "swipe is armed");

    assert!(host.cancel_native_touch_gestures());
    assert!(
        host.slideshow_press_screen.is_none(),
        "cancel clears slideshow capture"
    );
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));

    assert!(
        !host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H),
        "a late platform Up has no cancelled gesture to commit"
    );
    assert_eq!(board_on_screen(&host).as_deref(), Some("slide-1"));
}
