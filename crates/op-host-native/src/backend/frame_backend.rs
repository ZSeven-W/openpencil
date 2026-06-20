//! Frame-scoped `RenderBackend` adapter over `NativeBackend` +
//! `&Canvas`. Lifetime-bound to the `SharedSkiaContext::with_frame`
//! closure body so widget code never sees the canvas borrow directly.
//!
//! Pulled out of `widget_host.rs` so the spine file stays under the
//! 800-line ceiling.

use crate::backend::NativeBackend;
use op_editor_ui::{
    Color, ImageAdjustments, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout,
};

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

    fn clip_round_rect(&mut self, rect: Rect, radius: f32) {
        self.inner.clip_round_rect(self.canvas, rect, radius);
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

    fn scale(&mut self, scale: Point2D, pivot: Point2D) {
        self.inner.scale(self.canvas, scale, pivot);
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

    fn fill_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color) {
        self.inner
            .fill_svg_path_in_rect(self.canvas, d, rect, color);
    }

    fn stroke_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color, width: f32) {
        self.inner
            .stroke_svg_path_in_rect(self.canvas, d, rect, color, width);
    }

    fn fill_svg_path_in_rect_linear_gradient(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        self.inner.fill_svg_path_in_rect_linear_gradient(
            self.canvas,
            d,
            rect,
            stops,
            angle_deg,
            opacity,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_svg_path_in_rect_radial_gradient(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    ) {
        self.inner.fill_svg_path_in_rect_radial_gradient(
            self.canvas,
            d,
            rect,
            stops,
            cx_frac,
            cy_frac,
            radius_frac,
            opacity,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_inner_shadow_svg_path(
        &mut self,
        d: &str,
        rect: Rect,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        color: Color,
    ) {
        self.inner.fill_inner_shadow_svg_path(
            self.canvas,
            d,
            rect,
            offset_x,
            offset_y,
            blur,
            color,
        );
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

    fn fill_dots(&mut self, centers: &[Point2D], radius: f32, color: Color) {
        self.inner.fill_dots(self.canvas, centers, radius, color);
    }

    fn draw_image(&mut self, rect: Rect, image_id: u64, encoded: &[u8]) {
        self.inner.draw_image(self.canvas, rect, image_id, encoded);
    }

    fn draw_image_with_mode(
        &mut self,
        rect: Rect,
        image_id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
    ) {
        self.inner
            .draw_image_with_mode(self.canvas, rect, image_id, encoded, mode);
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_image_with_options(
        &mut self,
        rect: Rect,
        image_id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
        adjustments: ImageAdjustments,
        opacity: f32,
        corner_radius: f32,
    ) {
        self.inner.draw_image_with_options(
            self.canvas,
            rect,
            image_id,
            encoded,
            mode,
            adjustments,
            opacity,
            corner_radius,
        );
    }

    fn fill_round_rect_linear_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        self.inner.fill_round_rect_linear_gradient(
            self.canvas,
            rect,
            radius,
            stops,
            angle_deg,
            opacity,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn fill_round_rect_radial_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        stops: &[(f32, Color)],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    ) {
        self.inner.fill_round_rect_radial_gradient(
            self.canvas,
            rect,
            radius,
            stops,
            cx_frac,
            cy_frac,
            radius_frac,
            opacity,
        );
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

    fn measure_text_styled(
        &mut self,
        text: &str,
        font_size: f32,
        weight: u16,
        italic: bool,
    ) -> f32 {
        self.inner
            .measure_text_styled(text, font_size, weight, italic)
    }

    fn measure_text_family(&mut self, text: &str, font_size: f32, family: &str) -> f32 {
        self.inner.measure_text_family(text, font_size, family)
    }
}
