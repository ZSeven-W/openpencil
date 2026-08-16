//! `CanvasKitBackend` — the `RenderBackend` implementation over the CanvasKit
//! JS bridge, plus the async `init_backend` constructor.
//!
//! Split out of `canvaskit.rs`. A trait impl must be one block, so the flat
//! `OpCk` call bodies for the shape / gradient / SVG-path ops live in `ops`
//! and the trait methods here forward to them.

use op_editor_ui::{
    Color, ImageAdjustments, ImageBlendMode, ImageDrawMode, Point2D, Rect, RenderBackend,
    TextBaselineRequest, TextLayout,
};
use wasm_bindgen::prelude::*;

use super::bindings::{op_ck_init, set_image_cache_factory, web_image_cache_factory, OpCk};
use super::convert::{
    image_blend_mode_code, image_draw_mode_code, normalized_tile_scale, svg_path_even_odd,
    valid_original_size,
};
use super::ops;

/// `RenderBackend` over CanvasKit. Paints in logical (CSS) pixels; `begin_frame`
/// applies the device-pixel-ratio so output matches the native backend.
pub struct CanvasKitBackend {
    pub(super) ck: OpCk,
    pub(super) dpr: f32,
    /// Logical (CSS) viewport — what widget layout uses.
    logical_w: u32,
    logical_h: u32,
}

impl CanvasKitBackend {
    pub fn new(ck: OpCk, dpr: f32, logical_w: u32, logical_h: u32) -> Self {
        let dpr = dpr.max(1.0);
        ck.set_dpr(dpr);
        Self {
            ck,
            dpr,
            logical_w,
            logical_h,
        }
    }
    pub fn logical_size(&self) -> (f32, f32) {
        (self.logical_w as f32, self.logical_h as f32)
    }
    pub fn resize_for_display(&mut self, logical_w: u32, logical_h: u32, dpr: f32) {
        self.logical_w = logical_w.max(1);
        self.logical_h = logical_h.max(1);
        self.dpr = dpr.max(1.0);
        self.ck.set_dpr(self.dpr);
        let pw = ((self.logical_w as f32) * self.dpr).round() as u32;
        let ph = ((self.logical_h as f32) * self.dpr).round() as u32;
        self.ck.resize(pw.max(1), ph.max(1));
    }
    /// Register a user-imported font face (mirrors `register_system_font` but
    /// for the family-selectable imported registry). Returns `true` when the
    /// face parsed and is now selectable by `family`.
    pub fn register_imported_font(&mut self, family: &str, bytes: &[u8]) -> bool {
        self.ck.register_imported_font(family, bytes)
    }
    /// Register a font whose family is unknown (a fresh browser import). Returns
    /// the extracted family display name, or `None` on parse failure / no
    /// family name (the CanvasKit side returns an empty string).
    pub fn register_imported_font_from_bytes(&mut self, bytes: &[u8]) -> Option<String> {
        // The vendored CanvasKit build can't report a typeface's family name,
        // so parse it in Rust, then register through the family-known FFI.
        let family = crate::font_meta::parse_family(bytes)?;
        self.ck
            .register_imported_font(&family, bytes)
            .then_some(family)
    }
    /// Display names of every registered imported family.
    pub fn imported_family_list(&self) -> Vec<String> {
        self.ck.imported_family_list()
    }
    /// Drop a previously imported font face by family name.
    pub fn remove_imported_font(&mut self, family: &str) {
        self.ck.remove_imported_font(family);
    }

    /// Decode a bounded batch of image bytes requested by the last paint.
    ///
    /// The full CanvasKit host calls this from its repaint loop, while the
    /// standalone `op-web-sdk` owns a smaller independent pump and calls it
    /// directly. Keeping the queue drain on the backend makes both hosts use
    /// the same cache and failure bookkeeping.
    pub fn drain_pending_decodes(&mut self, max: usize) -> usize {
        use crate::image_decode_queue::{finish_web_decode, take_web_decode_batch};
        let batch = take_web_decode_batch(max);
        for job in &batch {
            let decoded = self.ck.decode_image(
                job.id as u32,
                (job.id >> 32) as u32,
                job.bytes.as_ref(),
                job.max_edge_px,
            );
            finish_web_decode(job.id, decoded);
        }
        batch.len()
    }
}

impl RenderBackend for CanvasKitBackend {
    fn begin_frame(&mut self) {
        self.ck.begin_frame();
        if (self.dpr - 1.0).abs() > f32::EPSILON {
            self.ck.scale(self.dpr, self.dpr);
        }
    }
    fn end_frame(&mut self) {
        self.ck.end_frame();
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        ops::fill_rect(&self.ck, rect, color);
    }
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        ops::stroke_rect(&self.ck, rect, color, width);
    }
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        ops::fill_round_rect(&self.ck, rect, radius, color);
    }
    fn fill_round_rect_per_corner(&mut self, rect: Rect, radii: [f32; 4], color: Color) {
        ops::fill_round_rect_per_corner(&self.ck, rect, radii, color);
    }
    fn fill_round_rect_linear_gradient(
        &mut self,
        rect: Rect,
        radius: f32,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        ops::fill_round_rect_linear_gradient(&self.ck, rect, radius, stops, angle_deg, opacity);
    }
    fn fill_round_rect_linear_gradient_per_corner(
        &mut self,
        rect: Rect,
        radii: [f32; 4],
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        ops::fill_round_rect_linear_gradient_per_corner(
            &self.ck, rect, radii, stops, angle_deg, opacity,
        );
    }
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
        ops::fill_round_rect_radial_gradient(
            &self.ck,
            rect,
            radius,
            stops,
            cx_frac,
            cy_frac,
            radius_frac,
            opacity,
        );
    }
    fn fill_round_rect_radial_gradient_per_corner(
        &mut self,
        rect: Rect,
        radii: [f32; 4],
        stops: &[(f32, Color)],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    ) {
        ops::fill_round_rect_radial_gradient_per_corner(
            &self.ck,
            rect,
            radii,
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
        ops::fill_round_rect_mesh_gradient(&self.ck, rect, radius, rows, cols, colors, opacity);
    }
    fn fill_round_rect_mesh_gradient_per_corner(
        &mut self,
        rect: Rect,
        radii: [f32; 4],
        rows: u32,
        cols: u32,
        colors: &[Color],
        opacity: f32,
    ) {
        ops::fill_round_rect_mesh_gradient_per_corner(
            &self.ck, rect, radii, rows, cols, colors, opacity,
        );
    }
    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32) {
        ops::stroke_round_rect(&self.ck, rect, radius, color, width);
    }
    fn stroke_round_rect_per_corner(
        &mut self,
        rect: Rect,
        radii: [f32; 4],
        color: Color,
        width: f32,
    ) {
        ops::stroke_round_rect_per_corner(&self.ck, rect, radii, color, width);
    }
    fn fill_oval(&mut self, bounds: Rect, color: Color) {
        ops::fill_oval(&self.ck, bounds, color);
    }
    fn stroke_oval(&mut self, bounds: Rect, color: Color, width: f32) {
        ops::stroke_oval(&self.ck, bounds, color, width);
    }
    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        self.ck.stroke_line(
            from.x, from.y, to.x, to.y, color.r, color.g, color.b, color.a, width,
        );
    }
    fn fill_polygon(&mut self, points: &[Point2D], color: Color) {
        ops::fill_polygon(&self.ck, points, color);
    }
    fn stroke_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, color: Color, width: f32) {
        ops::stroke_svg_path(&self.ck, d, top_left, size, color, width);
    }
    fn fill_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, viewbox: f32, color: Color) {
        let even_odd = svg_path_even_odd(d);
        self.fill_svg_path_with_fill_rule(d, top_left, size, viewbox, color, even_odd);
    }
    fn fill_svg_path_with_fill_rule(
        &mut self,
        d: &str,
        top_left: Point2D,
        size: f32,
        viewbox: f32,
        color: Color,
        even_odd: bool,
    ) {
        ops::fill_svg_path_with_fill_rule(&self.ck, d, top_left, size, viewbox, color, even_odd);
    }
    fn fill_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color) {
        let even_odd = svg_path_even_odd(d);
        self.fill_svg_path_in_rect_with_fill_rule(d, rect, color, even_odd);
    }
    fn fill_svg_path_in_rect_with_fill_rule(
        &mut self,
        d: &str,
        rect: Rect,
        color: Color,
        even_odd: bool,
    ) {
        ops::fill_svg_path_in_rect_with_fill_rule(&self.ck, d, rect, color, even_odd);
    }
    fn stroke_svg_path_in_rect(&mut self, d: &str, rect: Rect, color: Color, width: f32) {
        ops::stroke_svg_path_in_rect(&self.ck, d, rect, color, width);
    }
    fn fill_svg_path_in_rect_linear_gradient(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
    ) {
        self.fill_svg_path_in_rect_linear_gradient_with_fill_rule(
            d,
            rect,
            stops,
            angle_deg,
            opacity,
            svg_path_even_odd(d),
        );
    }
    fn fill_svg_path_in_rect_linear_gradient_with_fill_rule(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        angle_deg: f32,
        opacity: f32,
        even_odd: bool,
    ) {
        ops::fill_svg_path_in_rect_linear_gradient_with_fill_rule(
            &self.ck, d, rect, stops, angle_deg, opacity, even_odd,
        );
    }
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
        self.fill_svg_path_in_rect_radial_gradient_with_fill_rule(
            d,
            rect,
            stops,
            cx_frac,
            cy_frac,
            radius_frac,
            opacity,
            svg_path_even_odd(d),
        );
    }
    fn fill_svg_path_in_rect_radial_gradient_with_fill_rule(
        &mut self,
        d: &str,
        rect: Rect,
        stops: &[(f32, Color)],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
        even_odd: bool,
    ) {
        ops::fill_svg_path_in_rect_radial_gradient_with_fill_rule(
            &self.ck,
            d,
            rect,
            stops,
            cx_frac,
            cy_frac,
            radius_frac,
            opacity,
            even_odd,
        );
    }
    fn fill_inner_shadow_svg_path(
        &mut self,
        d: &str,
        rect: Rect,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        color: Color,
    ) {
        self.fill_inner_shadow_svg_path_with_fill_rule(
            d,
            rect,
            offset_x,
            offset_y,
            blur,
            color,
            svg_path_even_odd(d),
        );
    }
    fn fill_inner_shadow_svg_path_with_fill_rule(
        &mut self,
        d: &str,
        rect: Rect,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        color: Color,
        even_odd: bool,
    ) {
        ops::fill_inner_shadow_svg_path_with_fill_rule(
            &self.ck, d, rect, offset_x, offset_y, blur, color, even_odd,
        );
    }

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        let italic = layout.italic();
        for run in layout.runs() {
            let x = origin.x + run.origin.x;
            let y = origin.y + run.origin.y;
            // `TextRun.color` is `jian_core::scene::Color` (0-255 u8 channels),
            // unlike the f32-channel `Color` the fill ops take.
            let c = run.color;
            let (cr, cg, cb, ca) = (
                f32::from(c.r()) / 255.0,
                f32::from(c.g()) / 255.0,
                f32::from(c.b()) / 255.0,
                f32::from(c.a()) / 255.0,
            );
            // Same predicate the native host uses, so the two never disagree
            // about which runs need bidi + contextual joining.
            if op_editor_core::text_script::needs_complex_shaping(run.content.as_str()) {
                self.ck.draw_shaped_text(
                    run.content.as_str(),
                    run.font_family.as_str(),
                    x,
                    y,
                    run.font_size,
                    run.font_weight as i32,
                    italic,
                    cr,
                    cg,
                    cb,
                    ca,
                );
                continue;
            }
            self.ck.draw_text(
                run.content.as_str(),
                run.font_family.as_str(),
                x,
                y,
                run.font_size,
                run.font_weight as i32,
                italic,
                cr,
                cg,
                cb,
                ca,
            );
        }
    }
    fn image_decoded(&mut self, image_id: u64, encoded: &[u8], max_edge_px: u32) -> bool {
        let _ = encoded;
        self.ck
            .image_decoded(image_id as u32, (image_id >> 32) as u32, max_edge_px)
    }
    fn image_resident(&mut self, image_id: u64) -> bool {
        // A zero quality requirement asks only whether any raster is cached.
        self.ck
            .image_decoded(image_id as u32, (image_id >> 32) as u32, 0)
    }
    fn draw_image_thumb(&mut self, rect: Rect, image_id: u64, jpeg: &[u8]) {
        self.ck.draw_image_thumb(
            image_id as u32,
            (image_id >> 32) as u32,
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            jpeg,
        );
    }
    fn draw_image(&mut self, rect: Rect, image_id: u64, encoded: &[u8]) {
        self.draw_image_with_options_and_transform(
            rect,
            image_id,
            encoded,
            ImageDrawMode::Fit,
            ImageAdjustments::default(),
            1.0,
            0.0,
            None,
        );
    }
    fn draw_image_with_mode(
        &mut self,
        rect: Rect,
        image_id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
    ) {
        self.draw_image_with_options_and_transform(
            rect,
            image_id,
            encoded,
            mode,
            ImageAdjustments::default(),
            1.0,
            0.0,
            None,
        );
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
        self.draw_image_with_options_and_transform(
            rect,
            image_id,
            encoded,
            mode,
            adjustments,
            opacity,
            corner_radius,
            None,
        );
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_image_with_options_and_transform(
        &mut self,
        rect: Rect,
        image_id: u64,
        encoded: &[u8],
        mode: ImageDrawMode,
        adjustments: ImageAdjustments,
        opacity: f32,
        corner_radius: f32,
        transform: Option<[f32; 6]>,
    ) {
        self.draw_image_with_options_transform_and_blend(
            rect,
            image_id,
            encoded,
            mode,
            adjustments,
            opacity,
            corner_radius,
            transform,
            ImageBlendMode::Normal,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_image_with_options_transform_and_blend(
        &mut self,
        rect: Rect,
        image_id: u64,
        _encoded: &[u8],
        mode: ImageDrawMode,
        adjustments: ImageAdjustments,
        opacity: f32,
        corner_radius: f32,
        transform: Option<[f32; 6]>,
        blend_mode: ImageBlendMode,
    ) {
        self.draw_image_with_options_transform_blend_and_tile_scale(
            rect,
            image_id,
            _encoded,
            mode,
            adjustments,
            opacity,
            corner_radius,
            transform,
            blend_mode,
            None,
            1.0,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_image_with_options_transform_blend_and_tile_scale(
        &mut self,
        rect: Rect,
        image_id: u64,
        _encoded: &[u8],
        mode: ImageDrawMode,
        adjustments: ImageAdjustments,
        opacity: f32,
        corner_radius: f32,
        transform: Option<[f32; 6]>,
        blend_mode: ImageBlendMode,
        original_size: Option<[f32; 2]>,
        tile_scale: f32,
    ) {
        let transform = transform.as_ref().map_or(&[][..], |value| &value[..]);
        let adjustment_values = [
            adjustments.exposure,
            adjustments.contrast,
            adjustments.saturation,
            adjustments.temperature,
            adjustments.tint,
            adjustments.highlights,
            adjustments.shadows,
        ];
        let [original_width, original_height] =
            valid_original_size(original_size).unwrap_or([0.0, 0.0]);
        self.ck.draw_image_with_options(
            image_id as u32,
            (image_id >> 32) as u32,
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            image_draw_mode_code(mode),
            transform,
            &adjustment_values,
            opacity,
            corner_radius,
            image_blend_mode_code(blend_mode),
            original_width,
            original_height,
            normalized_tile_scale(tile_scale),
        );
    }
    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        self.ck.measure_text_styled(text, font_size, 400, false)
    }
    fn measure_text_weighted(&mut self, text: &str, font_size: f32, weight: u16) -> f32 {
        self.ck
            .measure_text_styled(text, font_size, i32::from(weight), false)
    }
    fn text_ascent(&mut self, font_size: f32, weight: u16) -> f32 {
        self.ck.text_ascent("", font_size, i32::from(weight))
    }
    fn text_ascent_family(&mut self, font_size: f32, family: &str, weight: u16) -> f32 {
        self.ck.text_ascent(family, font_size, i32::from(weight))
    }
    fn text_first_baseline(&mut self, request: &TextBaselineRequest<'_>) -> f32 {
        self.ck.text_first_baseline(
            request.text,
            request.font_family,
            request.font_size,
            i32::from(request.font_weight),
            request.italic,
            request.line_height,
        )
    }
    fn measure_text_styled(
        &mut self,
        text: &str,
        font_size: f32,
        weight: u16,
        italic: bool,
    ) -> f32 {
        self.ck
            .measure_text_styled(text, font_size, weight as i32, italic)
    }
    fn measure_text_family(&mut self, text: &str, font_size: f32, family: &str) -> f32 {
        self.ck
            .measure_text_family_styled(text, family, font_size, 400, false)
    }
    /// Family-aware measure so an editable field's caret / selection geometry
    /// lines up with the glyphs `draw_text` paints for a named imported family.
    /// Forwards to the JS `measureTextFamilyStyled`, which shares the exact
    /// typeface + font sizing the family-aware `drawText` path uses.
    fn measure_text_family_styled(
        &mut self,
        text: &str,
        font_size: f32,
        family: &str,
        weight: u16,
        italic: bool,
    ) -> f32 {
        if op_editor_core::text_script::needs_complex_shaping(text) {
            // Paint routes this run to the browser shaper, so measurement has
            // to as well — otherwise wrapping and carets are computed against
            // advances the painter never uses.
            return self
                .ck
                .measure_shaped_text(text, family, font_size, i32::from(weight), italic);
        }
        self.ck
            .measure_text_family_styled(text, family, font_size, i32::from(weight), italic)
    }

    fn clip_rect(&mut self, rect: Rect) {
        self.ck
            .clip_rect(rect.origin.x, rect.origin.y, rect.size.x, rect.size.y);
    }
    fn clip_round_rect(&mut self, rect: Rect, radius: f32) {
        self.ck.clip_round_rect(
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            radius,
        );
    }
    fn clip_round_rect_per_corner(&mut self, rect: Rect, radii: [f32; 4]) {
        self.ck.clip_round_rect_per_corner(
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            radii[0],
            radii[1],
            radii[2],
            radii[3],
        );
    }
    fn clip_oval(&mut self, bounds: Rect) {
        self.ck.clip_oval(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.x,
            bounds.size.y,
        );
    }
    fn clip_polygon(&mut self, points: &[Point2D]) {
        let mut flat = Vec::with_capacity(points.len() * 2);
        for point in points {
            flat.extend_from_slice(&[point.x, point.y]);
        }
        self.ck.clip_polygon(&flat);
    }
    fn clip_svg_path_in_rect(&mut self, d: &str, rect: Rect, even_odd: bool) {
        self.ck.clip_svg_path_in_rect(
            d,
            rect.origin.x,
            rect.origin.y,
            rect.size.x,
            rect.size.y,
            even_odd,
        );
    }
    fn save(&mut self) {
        self.ck.save();
    }
    fn push_composite_layer(&mut self, bounds: Rect, opacity: f32, mode: ImageBlendMode) {
        self.ck.push_composite_layer(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.x,
            bounds.size.y,
            opacity,
            image_blend_mode_code(mode),
        );
    }
    fn supports_pixel_masks(&self) -> bool {
        true
    }
    fn push_mask_source_layer(&mut self, luminance: bool) {
        self.ck.push_mask_source_layer(luminance);
    }
    fn push_blend_layer(&mut self, mode: ImageBlendMode) {
        self.ck.push_blend_layer(image_blend_mode_code(mode));
    }
    fn push_backdrop_blur_layer(&mut self, sigma: f32) {
        self.ck.push_backdrop_blur_layer(sigma);
    }
    fn restore(&mut self) {
        self.ck.restore();
    }
    fn translate(&mut self, offset: Point2D) {
        self.ck.translate(offset.x, offset.y);
    }
    fn scale(&mut self, scale: Point2D, pivot: Point2D) {
        // Mirror the native backend: scale about a pivot.
        self.ck.translate(pivot.x, pivot.y);
        self.ck.scale(scale.x, scale.y);
        self.ck.translate(-pivot.x, -pivot.y);
    }
    fn rotate(&mut self, radians: f32, pivot: Point2D) {
        self.ck.rotate(radians.to_degrees(), pivot.x, pivot.y);
    }
    fn resize(&mut self, width: u32, height: u32) {
        self.logical_w = width.max(1);
        self.logical_h = height.max(1);
        let pw = ((width as f32) * self.dpr).round() as u32;
        let ph = ((height as f32) * self.dpr).round() as u32;
        self.ck.resize(pw.max(1), ph.max(1));
    }
    fn dpi_scale(&self) -> f32 {
        self.dpr
    }
}

/// Initialise CanvasKit on `canvas_id` and return a ready `CanvasKitBackend`.
pub async fn init_backend(
    canvas_id: &str,
    dpr: f32,
    logical_w: u32,
    logical_h: u32,
) -> Result<CanvasKitBackend, JsValue> {
    let image_cache_factory = web_image_cache_factory();
    set_image_cache_factory(&image_cache_factory);
    let promise = op_ck_init(canvas_id)?;
    let ck_val = wasm_bindgen_futures::JsFuture::from(promise).await?;
    let ck: OpCk = ck_val.unchecked_into();
    Ok(CanvasKitBackend::new(ck, dpr, logical_w, logical_h))
}
