//! `BaselineAdjustingBackend` — a transparent [`RenderBackend`] wrapper that
//! shifts text draws vertically by `baseline_delta_y` and forwards every other
//! call straight through to the inner backend.
//!
//! Text-input widgets (`jian_widgets::TextInputView`) paint their glyphs from
//! the top of the text box, but the editor's inputs want the run anchored at a
//! specific baseline. Each input previously carried its own hand-written
//! ~250-line forwarding wrapper (`TextInputBaselineBackend` in
//! `property_panel_text_input.rs`, `SettingsInputBackend` in
//! `agent_settings_caret.rs`, `TextAreaBaselineBackend` in
//! `ai_chat_input_text.rs`) that differed only in which subset of the trait it
//! bothered to forward. This single generic wrapper forwards the full trait
//! surface — forwarding to `inner` is the only correct behaviour for a
//! transparent wrapper — so the three copies collapse into one.

use crate::{Color, ImageAdjustments, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout};

pub(crate) struct BaselineAdjustingBackend<'a> {
    pub(crate) inner: &'a mut dyn RenderBackend,
    pub(crate) baseline_delta_y: f32,
}

impl RenderBackend for BaselineAdjustingBackend<'_> {
    fn begin_frame(&mut self) {
        self.inner.begin_frame();
    }

    fn end_frame(&mut self) {
        self.inner.end_frame();
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.inner.fill_rect(rect, color);
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        self.inner.stroke_rect(rect, color, width);
    }

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        let draw_origin = Point2D::new(origin.x, origin.y + self.baseline_delta_y);
        self.inner.draw_text(layout, draw_origin);
    }

    fn clip_rect(&mut self, rect: Rect) {
        self.inner.clip_rect(rect);
    }

    fn clip_round_rect(&mut self, rect: Rect, radius: f32) {
        self.inner.clip_round_rect(rect, radius);
    }

    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        self.inner.stroke_line(from, to, color, width);
    }

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.inner.fill_round_rect(rect, radius, color);
    }

    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32) {
        self.inner.stroke_round_rect(rect, radius, color, width);
    }

    fn stroke_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, color: Color, width: f32) {
        self.inner.stroke_svg_path(d, top_left, size, color, width);
    }

    fn fill_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, viewbox: f32, color: Color) {
        self.inner.fill_svg_path(d, top_left, size, viewbox, color);
    }

    fn fill_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color) {
        self.inner.fill_svg_path_in_rect(d, rect, color);
    }

    fn stroke_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color, width: f32) {
        self.inner.stroke_svg_path_in_rect(d, rect, color, width);
    }

    fn fill_svg_path_in_rect_linear_gradient(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        self.inner
            .fill_svg_path_in_rect_linear_gradient(d, rect, stops, angle_deg, opacity);
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
        self.inner
            .fill_inner_shadow_svg_path(d, rect, offset_x, offset_y, blur, color);
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
            d,
            rect,
            stops,
            cx_frac,
            cy_frac,
            radius_frac,
            opacity,
        );
    }

    fn fill_drop_shadow(&mut self, rect: Rect, radius: f32, blur: f32, color: Color) {
        self.inner.fill_drop_shadow(rect, radius, blur, color);
    }

    fn fill_oval(&mut self, bounds: Rect, color: Color) {
        self.inner.fill_oval(bounds, color);
    }

    fn stroke_oval(&mut self, bounds: Rect, color: Color, width: f32) {
        self.inner.stroke_oval(bounds, color, width);
    }

    fn fill_dots(&mut self, centers: &[Point2D], radius: f32, color: Color) {
        self.inner.fill_dots(centers, radius, color);
    }

    fn fill_polygon(&mut self, points: &[Point2D], color: Color) {
        self.inner.fill_polygon(points, color);
    }

    fn stroke_polygon(&mut self, points: &[Point2D], color: Color, width: f32) {
        self.inner.stroke_polygon(points, color, width);
    }

    fn draw_image(&mut self, rect: Rect, image_id: u64, encoded: &[u8]) {
        self.inner.draw_image(rect, image_id, encoded);
    }

    fn draw_image_with_mode(
        &mut self,
        rect: Rect,
        image_id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
    ) {
        self.inner
            .draw_image_with_mode(rect, image_id, encoded, mode);
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
        self.inner
            .fill_round_rect_linear_gradient(rect, radius, stops, angle_deg, opacity);
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
            rect,
            radius,
            stops,
            cx_frac,
            cy_frac,
            radius_frac,
            opacity,
        );
    }

    fn fill_round_rect_mesh_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        rows: u32,
        cols: u32,
        colors: &[Color],
        opacity: f32,
    ) {
        self.inner
            .fill_round_rect_mesh_gradient(rect, radius, rows, cols, colors, opacity);
    }

    fn fill_round_rect_shader(
        &mut self,
        rect: Rect,
        radius: f32,
        sksl: &str,
        uniforms: &[(&str, &[f32])],
        opacity: f32,
        fallback: Color,
    ) {
        self.inner
            .fill_round_rect_shader(rect, radius, sksl, uniforms, opacity, fallback);
    }

    fn save(&mut self) {
        self.inner.save();
    }

    fn restore(&mut self) {
        self.inner.restore();
    }

    fn translate(&mut self, offset: Point2D) {
        self.inner.translate(offset);
    }

    fn scale(&mut self, scale: Point2D, pivot: Point2D) {
        self.inner.scale(scale, pivot);
    }

    fn rotate(&mut self, radians: f32, pivot: Point2D) {
        self.inner.rotate(radians, pivot);
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.inner.resize(width, height);
    }

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

    fn measure_text_family_styled(
        &mut self,
        text: &str,
        font_size: f32,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> f32 {
        self.inner
            .measure_text_family_styled(text, font_size, family, weight, italic)
    }
}
