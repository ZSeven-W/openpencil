//! Test-only `RenderBackend` stub that records `fill_round_rect` and
//! `draw_text` and `fill_rect` calls and ignores everything else. Shared by
//! widget test modules that assert on flat fills (colour bands), round-rect
//! fills (button feedback washes, panel row tints) or on the exact strings a
//! widget painted (wrapping / ellipsizing).
//!
//! `measure_text` is intentionally left at the trait default (the width
//! heuristic in `jian_widgets::painter`), so tests measure the same way a
//! font-less environment does.

use crate::{Color, Point2D, Rect, RenderBackend, TextLayout};

#[derive(Default)]
pub(crate) struct CaptureBackend {
    pub(crate) round_fills: Vec<(Rect, f32, Color)>,
    /// `(text, origin)` of every `draw_text`, in paint order.
    pub(crate) texts: Vec<(String, Point2D)>,
    pub(crate) rect_fills: Vec<(Rect, Color)>,
    /// `(path, top-left, size, color, stroke)` for canonical 24×24 icons.
    pub(crate) svg_strokes: Vec<(String, Point2D, f32, Color, f32)>,
}

impl RenderBackend for CaptureBackend {
    fn begin_frame(&mut self) {}

    fn end_frame(&mut self) {}

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.rect_fills.push((rect, color));
    }

    fn stroke_rect(&mut self, _rect: Rect, _color: Color, _width: f32) {}

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        for run in layout.runs() {
            self.texts.push((run.content.clone(), origin));
        }
    }

    fn clip_rect(&mut self, _rect: Rect) {}

    fn stroke_line(&mut self, _from: Point2D, _to: Point2D, _color: Color, _width: f32) {}

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.round_fills.push((rect, radius, color));
    }

    fn stroke_round_rect(&mut self, _rect: Rect, _radius: f32, _color: Color, _width: f32) {}

    fn stroke_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, color: Color, width: f32) {
        self.svg_strokes
            .push((d.to_owned(), top_left, size, color, width));
    }

    fn save(&mut self) {}

    fn restore(&mut self) {}

    fn translate(&mut self, _offset: Point2D) {}

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn dpi_scale(&self) -> f32 {
        1.0
    }
}
