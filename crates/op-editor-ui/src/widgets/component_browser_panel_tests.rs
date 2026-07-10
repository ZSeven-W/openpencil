//! Component-browser panel unit tests — split out of
//! `component_browser_panel.rs` to keep that module under the
//! 800-line ceiling. A `#[path]` submodule so it retains `super::`
//! access to the parent's private items (`filtered_call_count`, etc.).

use super::*;
use crate::RenderBackend;

#[derive(Default)]
struct NoopBackend;

impl RenderBackend for NoopBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _rect: Rect, _color: Color) {}
    fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {}
    fn draw_text(&mut self, _layout: &TextLayout, _origin: Point2D) {}
    fn clip_rect(&mut self, _rect: Rect) {}
    fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {}
    fn fill_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color) {}
    fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {}
    fn stroke_svg_path(
        &mut self,
        _d: &str,
        _top_left: Point2D,
        _size: f32,
        _color: Color,
        _width: f32,
    ) {
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _offset: Point2D) {}
    fn resize(&mut self, _width: u32, _height: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn open_state() -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.component_browser_open = true;
    state
}

fn rect() -> Rect {
    Rect::xywh(
        0.0,
        0.0,
        COMPONENT_BROWSER_PANEL_W,
        COMPONENT_BROWSER_PANEL_H,
    )
}

/// Before this fix, `hit_test` called `self.filtered()` once
/// directly AND again indirectly through `card_rects`. It must now
/// filter the kit set exactly once per call.
#[test]
fn hit_test_filters_the_kit_set_exactly_once() {
    let state = open_state();
    let panel = ComponentBrowserPanel::for_editor(&state).expect("panel open");
    let before = filtered_call_count();

    // A point well inside the card-grid area (below the header, kit
    // strip, pills, and search row) so `hit_test` walks into the
    // branch that filters the kit set — a header click resolves to
    // `DragHeader` and never reaches `filtered()` at all.
    let _ = panel.hit_test(
        rect(),
        Point2D::new(
            rect().origin.x + COMPONENT_BROWSER_PANEL_W / 2.0,
            rect().origin.y + COMPONENT_BROWSER_PANEL_H - 40.0,
        ),
    );

    assert_eq!(
        filtered_call_count(),
        before + 1,
        "a single hit_test pass must filter the kit set exactly once, not twice"
    );
}

/// Before this fix, `paint` called `self.filtered()` once directly
/// (for the empty-state check) AND again indirectly through
/// `card_rects` for the grid layout. It must now filter the kit set
/// exactly once per call.
#[test]
fn paint_filters_the_kit_set_exactly_once() {
    let state = open_state();
    let panel = ComponentBrowserPanel::for_editor(&state).expect("panel open");
    let before = filtered_call_count();
    let mut backend = NoopBackend;
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect());

    assert_eq!(
        filtered_call_count(),
        before + 1,
        "a single paint pass must filter the kit set exactly once, not twice"
    );
}
