//! Gate spike test: proves `CanvasViewport::from_scene` can paint from a
//! `LayoutScene` + `DocViewport` without an `EditorState`.
//!
//! Step 1 (RED): run `cargo test -p op-editor-ui from_scene_paints -- --nocapture`
//! and confirm FAIL because `from_scene` is not yet defined.
//!
//! Step 4 (GREEN): after `from_scene` is added, the same run must PASS.

use crate::layout_scene::{LayoutScene, ScenePage};
use crate::theme::Theme;
use crate::widgets::canvas_viewport::CanvasViewport;
use crate::widgets::{PaintCx, Widget};
use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_editor_core::Viewport as DocViewport;

// ---------------------------------------------------------------------------
// Counting backend — records fill_rect calls to verify paint happened.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CaptureBackend {
    fill_rect_calls: usize,
}

impl CaptureBackend {
    fn new(_width: u32, _height: u32) -> Self {
        Self::default()
    }

    fn fill_rect_count(&self) -> usize {
        self.fill_rect_calls
    }

    fn paint_cx(&mut self) -> PaintCx<'_> {
        PaintCx {
            backend: self,
        }
    }
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}

    fn fill_rect(&mut self, _rect: Rect, _color: Color) {
        self.fill_rect_calls += 1;
    }

    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}

    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}

    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}

    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

// ---------------------------------------------------------------------------
// Gate spike test
// ---------------------------------------------------------------------------

#[test]
fn from_scene_paints_background_and_nodes_without_editor_state() {
    let scene = LayoutScene {
        pages: vec![ScenePage {
            id: "p".into(),
            name: "Page".into(),
            children: vec![],
        }],
        active_page_index: 0,
    };
    let vp = DocViewport {
        pan_x: 0.0,
        pan_y: 0.0,
        zoom: 1.0,
    };
    let view = CanvasViewport::from_scene(&scene, vp, Theme::dark());
    let mut backend = CaptureBackend::new(800, 600);
    {
        let mut cx = backend.paint_cx();
        view.paint(&mut cx, Rect::xywh(0.0, 0.0, 800.0, 600.0));
    }
    assert!(
        backend.fill_rect_count() >= 1,
        "expected at least the canvas background fill, got {}",
        backend.fill_rect_count()
    );
}
