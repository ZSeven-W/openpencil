//! Measure-only [`RenderBackend`] facade over [`NativeBackend`].
//!
//! The canvas text-edit geometry helpers (`text_edit_layout`,
//! `caret_position`, `offset_at_point`) need real FontMgr-backed text
//! measurement but no canvas; every paint primitive is a no-op, exactly
//! like the desktop host's `MeasureOnly` in
//! `op-host-native/src/widget_host/text_edit_press.rs`.

use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};
use op_host_native::NativeBackend;

pub(crate) struct MeasureOnly<'a> {
    pub inner: &'a mut NativeBackend,
}

impl RenderBackend for MeasureOnly<'_> {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: Color) {}
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.inner.measure_text(text, font_size)
    }
    fn measure_text_weighted(&mut self, text: &str, font_size: f32, weight: u16) -> f32 {
        self.inner.measure_text_weighted(text, font_size, weight)
    }
    fn measure_text_family(&mut self, text: &str, font_size: f32, family: &str) -> f32 {
        self.inner.measure_text_family(text, font_size, family)
    }
    fn text_ascent(&mut self, font_size: f32, weight: u16) -> f32 {
        self.inner.text_ascent(font_size, weight)
    }
    fn text_ascent_family(&mut self, font_size: f32, family: &str, weight: u16) -> f32 {
        self.inner.text_ascent_family(font_size, family, weight)
    }
}
