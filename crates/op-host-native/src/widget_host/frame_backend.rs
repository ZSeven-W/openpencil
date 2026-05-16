//! Frame-scoped `RenderBackend` adapter over `NativeBackend` +
//! `&Canvas`. Lifetime-bound to the `SharedSkiaContext::with_frame`
//! closure body so widget code never sees the canvas borrow directly.
//!
//! Pulled out of `widget_host.rs` so the spine file stays under the
//! 800-line ceiling.

use crate::backend::NativeBackend;
use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};

pub struct NativeFrameBackend<'a> {
    inner: &'a mut NativeBackend,
    canvas: &'a skia_safe::Canvas,
}

impl<'a> NativeFrameBackend<'a> {
    pub fn new(inner: &'a mut NativeBackend, canvas: &'a skia_safe::Canvas) -> Self {
        Self { inner, canvas }
    }
}

impl<'a> RenderBackend for NativeFrameBackend<'a> {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.inner.fill_rect(self.canvas, rect, color);
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        self.inner.stroke_rect(self.canvas, rect, color, width);
    }

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        self.inner.draw_text(self.canvas, layout, origin);
    }

    fn clip_rect(&mut self, rect: Rect) {
        self.inner.clip_rect(self.canvas, rect);
    }

    fn save(&mut self) {
        let _ = self.inner.save(self.canvas);
    }

    fn restore(&mut self) {
        self.inner.restore(self.canvas);
    }

    fn translate(&mut self, offset: Point2D) {
        self.inner.translate(self.canvas, offset);
    }

    fn rotate(&mut self, radians: f32, pivot: Point2D) {
        self.inner.rotate(self.canvas, radians, pivot);
    }

    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        self.inner.stroke_line(self.canvas, from, to, color, width);
    }

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.inner.fill_round_rect(self.canvas, rect, radius, color);
    }

    fn fill_drop_shadow(&mut self, rect: Rect, radius: f32, blur: f32, color: Color) {
        self.inner
            .fill_drop_shadow(self.canvas, rect, radius, blur, color);
    }

    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32) {
        self.inner
            .stroke_round_rect(self.canvas, rect, radius, color, width);
    }

    fn stroke_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, color: Color, width: f32) {
        self.inner
            .stroke_svg_path(self.canvas, d, top_left, size, color, width);
    }

    fn fill_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, viewbox: f32, color: Color) {
        self.inner
            .fill_svg_path(self.canvas, d, top_left, size, viewbox, color);
    }

    fn fill_oval(&mut self, bounds: Rect, color: Color) {
        self.inner.fill_oval(self.canvas, bounds, color);
    }

    fn stroke_oval(&mut self, bounds: Rect, color: Color, width: f32) {
        self.inner.stroke_oval(self.canvas, bounds, color, width);
    }

    fn fill_polygon(&mut self, points: &[Point2D], color: Color) {
        self.inner.fill_polygon(self.canvas, points, color);
    }

    fn resize(&mut self, _width: u32, _height: u32) {}

    fn dpi_scale(&self) -> f32 {
        self.inner.dpi_scale()
    }

    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.inner.measure_text(text, font_size)
    }

    fn measure_text_weighted(&mut self, text: &str, font_size: f32, weight: u16) -> f32 {
        self.inner.measure_text_weighted(text, font_size, weight)
    }
}
