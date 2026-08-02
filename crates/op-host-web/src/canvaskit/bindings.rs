//! `op_ck_bridge.js` / `op_ck_image_cache.js` wasm-bindgen FFI declarations.
//!
//! Split out of `canvaskit.rs`: this module holds only the `extern "C"`
//! binding blocks — the `OpCk` bridge object and its flat scalar-arg draw ops,
//! plus the image-cache factory hook. No logic lives here.

use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/op_ck_bridge.js")]
extern "C" {
    /// Async init: load CanvasKit, make a WebGL surface on `canvas_id`, build
    /// the bridge object. Text uses browser/system fonts via the JS bridge.
    #[wasm_bindgen(js_name = opCkInit, catch)]
    pub(super) fn op_ck_init(canvas_id: &str) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(js_name = setImageCacheFactory)]
    pub(super) fn set_image_cache_factory(factory: &js_sys::Function);

    /// The bridge object: flat scalar-arg ops over a CanvasKit canvas.
    pub type OpCk;
    #[wasm_bindgen(method, js_name = beginFrame)]
    pub(super) fn begin_frame(this: &OpCk);
    #[wasm_bindgen(method, js_name = endFrame)]
    pub(super) fn end_frame(this: &OpCk);
    #[wasm_bindgen(method)]
    pub(super) fn clear(this: &OpCk, r: f32, g: f32, b: f32, a: f32);
    #[wasm_bindgen(method, js_name = fillRect)]
    pub(super) fn fill_rect(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = strokeRect)]
    pub(super) fn stroke_rect(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRect)]
    pub(super) fn fill_round_rect(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRectPerCorner)]
    pub(super) fn fill_round_rect_per_corner(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRectLinearGradient)]
    pub(super) fn fill_round_rect_linear_gradient(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad: f32,
        stops: &[f32],
        angle_deg: f32,
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRectLinearGradientPerCorner)]
    pub(super) fn fill_round_rect_linear_gradient_per_corner(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
        stops: &[f32],
        angle_deg: f32,
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRectRadialGradient)]
    pub(super) fn fill_round_rect_radial_gradient(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad: f32,
        stops: &[f32],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRectRadialGradientPerCorner)]
    pub(super) fn fill_round_rect_radial_gradient_per_corner(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
        stops: &[f32],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRectMeshGradient)]
    pub(super) fn fill_round_rect_mesh_gradient(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad: f32,
        rows: u32,
        cols: u32,
        colors: &[f32],
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillRoundRectMeshGradientPerCorner)]
    pub(super) fn fill_round_rect_mesh_gradient_per_corner(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
        rows: u32,
        cols: u32,
        colors: &[f32],
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = strokeRoundRect)]
    pub(super) fn stroke_round_rect(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rad: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = strokeRoundRectPerCorner)]
    pub(super) fn stroke_round_rect_per_corner(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillOval)]
    pub(super) fn fill_oval(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = strokeOval)]
    pub(super) fn stroke_oval(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = strokeLine)]
    pub(super) fn stroke_line(
        this: &OpCk,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillPolygon)]
    pub(super) fn fill_polygon(this: &OpCk, pts: &[f32], r: f32, g: f32, b: f32, a: f32);
    #[wasm_bindgen(method, js_name = strokeSvgPath)]
    pub(super) fn stroke_svg_path(
        this: &OpCk,
        d: &str,
        tx: f32,
        ty: f32,
        scale: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillSvgPath)]
    pub(super) fn fill_svg_path(
        this: &OpCk,
        d: &str,
        tx: f32,
        ty: f32,
        scale: f32,
        even_odd: bool,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = fillSvgPathInRect)]
    pub(super) fn fill_svg_path_in_rect(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = strokeSvgPathInRect)]
    pub(super) fn stroke_svg_path_in_rect(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        sw: f32,
    );
    #[wasm_bindgen(method, js_name = fillSvgPathInRectLinearGradient)]
    pub(super) fn fill_svg_path_in_rect_linear_gradient(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
        stops: &[f32],
        angle_deg: f32,
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillSvgPathInRectRadialGradient)]
    pub(super) fn fill_svg_path_in_rect_radial_gradient(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
        stops: &[f32],
        cx_frac: f32,
        cy_frac: f32,
        radius_frac: f32,
        opacity: f32,
    );
    #[wasm_bindgen(method, js_name = fillInnerShadowSvgPath)]
    pub(super) fn fill_inner_shadow_svg_path(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
        offset_x: f32,
        offset_y: f32,
        blur: f32,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = drawText)]
    pub(super) fn draw_text(
        this: &OpCk,
        t: &str,
        family: &str,
        x: f32,
        y: f32,
        sz: f32,
        weight: i32,
        italic: bool,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    #[wasm_bindgen(method, js_name = drawImageWithOptions)]
    pub(super) fn draw_image_with_options(
        this: &OpCk,
        image_id_lo: u32,
        image_id_hi: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        mode: u8,
        transform: &[f32],
        adjustments: &[f32],
        opacity: f32,
        corner_radius: f32,
        blend_mode: u8,
        original_width: f32,
        original_height: f32,
        tile_scale: f32,
    );
    #[wasm_bindgen(method, js_name = imageDecoded)]
    pub(super) fn image_decoded(
        this: &OpCk,
        image_id_lo: u32,
        image_id_hi: u32,
        max_edge_px: u32,
    ) -> bool;
    #[wasm_bindgen(method, js_name = decodeImage)]
    pub(super) fn decode_image(
        this: &OpCk,
        image_id_lo: u32,
        image_id_hi: u32,
        encoded: &[u8],
        max_edge_px: u32,
    ) -> bool;
    #[wasm_bindgen(method, js_name = drawImageThumb)]
    pub(super) fn draw_image_thumb(
        this: &OpCk,
        image_id_lo: u32,
        image_id_hi: u32,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        jpeg: &[u8],
    );
    #[wasm_bindgen(method, js_name = measureText)]
    pub(super) fn measure_text(this: &OpCk, t: &str, sz: f32) -> f32;
    #[wasm_bindgen(method, js_name = measureTextStyled)]
    pub(super) fn measure_text_styled(
        this: &OpCk,
        t: &str,
        sz: f32,
        weight: i32,
        italic: bool,
    ) -> f32;
    #[wasm_bindgen(method, js_name = textAscent)]
    pub(super) fn text_ascent(this: &OpCk, family: &str, sz: f32, weight: i32) -> f32;
    #[wasm_bindgen(method, js_name = textFirstBaseline)]
    pub(super) fn text_first_baseline(
        this: &OpCk,
        text: &str,
        family: &str,
        sz: f32,
        weight: i32,
        italic: bool,
        line_height: f32,
    ) -> f32;
    /// Family-aware measure: when `family` resolves to a registered imported
    /// font the whole run is measured with that single typeface, so the caret /
    /// layout geometry agrees to sub-pixel with what `drawText` paints for the
    /// same (text, family, sz, weight, italic). Empty `family` = family-blind.
    #[wasm_bindgen(method, js_name = measureTextFamilyStyled)]
    pub(super) fn measure_text_family_styled(
        this: &OpCk,
        t: &str,
        family: &str,
        sz: f32,
        weight: i32,
        italic: bool,
    ) -> f32;
    /// Paint a complex-script run (Arabic and friends) through the browser's
    /// own text engine, which resolves bidi and contextual joining that
    /// CanvasKit's `drawText` cmap lookup cannot. The run is passed whole —
    /// segmenting it first would reorder within segments but lay the segments
    /// out in storage order.
    #[wasm_bindgen(method, js_name = drawShapedText)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn draw_shaped_text(
        this: &OpCk,
        t: &str,
        family: &str,
        x: f32,
        y: f32,
        sz: f32,
        weight: i32,
        italic: bool,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    );
    /// Measure a complex-script run on the same engine `draw_shaped_text`
    /// paints with, so wrap decisions and caret geometry match the glyphs.
    #[wasm_bindgen(method, js_name = measureShapedText)]
    pub(super) fn measure_shaped_text(
        this: &OpCk,
        t: &str,
        family: &str,
        sz: f32,
        weight: i32,
        italic: bool,
    ) -> f32;
    #[wasm_bindgen(method, js_name = registerSystemFont)]
    pub(super) fn register_system_font(this: &OpCk, family: &str, bytes: &[u8]) -> bool;
    /// Register a user-imported font face; the family becomes selectable by
    /// name in `drawText` / `measureTextFamilyStyled`. Replaces any prior face
    /// under the same (case-insensitive) family key. Returns `false` on parse
    /// failure.
    #[wasm_bindgen(method, js_name = registerImportedFont)]
    pub(super) fn register_imported_font(this: &OpCk, family: &str, bytes: &[u8]) -> bool;
    /// Display names of every registered imported family — mirrors the JS
    /// registry into the Rust snapshot after add / remove and at mount.
    #[wasm_bindgen(method, js_name = importedFamilyList)]
    pub(super) fn imported_family_list(this: &OpCk) -> Vec<String>;
    /// Drop a previously imported font face by family name (no-op if absent).
    #[wasm_bindgen(method, js_name = removeImportedFont)]
    pub(super) fn remove_imported_font(this: &OpCk, family: &str);
    #[wasm_bindgen(method, js_name = clipRect)]
    pub(super) fn clip_rect(this: &OpCk, x: f32, y: f32, w: f32, h: f32);
    #[wasm_bindgen(method, js_name = clipRoundRect)]
    pub(super) fn clip_round_rect(this: &OpCk, x: f32, y: f32, w: f32, h: f32, rad: f32);
    #[wasm_bindgen(method, js_name = clipRoundRectPerCorner)]
    pub(super) fn clip_round_rect_per_corner(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    );
    #[wasm_bindgen(method, js_name = clipOval)]
    pub(super) fn clip_oval(this: &OpCk, x: f32, y: f32, w: f32, h: f32);
    #[wasm_bindgen(method, js_name = clipPolygon)]
    pub(super) fn clip_polygon(this: &OpCk, points: &[f32]);
    #[wasm_bindgen(method, js_name = clipSvgPathInRect)]
    pub(super) fn clip_svg_path_in_rect(
        this: &OpCk,
        d: &str,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        even_odd: bool,
    );
    #[wasm_bindgen(method)]
    pub(super) fn save(this: &OpCk);
    #[wasm_bindgen(method, js_name = pushCompositeLayer)]
    pub(super) fn push_composite_layer(
        this: &OpCk,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        opacity: f32,
        blend_mode: u8,
    );
    #[wasm_bindgen(method, js_name = pushMaskSourceLayer)]
    pub(super) fn push_mask_source_layer(this: &OpCk, luminance: bool);
    #[wasm_bindgen(method, js_name = pushBlendLayer)]
    pub(super) fn push_blend_layer(this: &OpCk, blend_mode: u8);
    #[wasm_bindgen(method, js_name = pushBackdropBlurLayer)]
    pub(super) fn push_backdrop_blur_layer(this: &OpCk, sigma: f32);
    #[wasm_bindgen(method)]
    pub(super) fn restore(this: &OpCk);
    #[wasm_bindgen(method)]
    pub(super) fn translate(this: &OpCk, x: f32, y: f32);
    #[wasm_bindgen(method)]
    pub(super) fn scale(this: &OpCk, sx: f32, sy: f32);
    #[wasm_bindgen(method)]
    pub(super) fn rotate(this: &OpCk, deg: f32, px: f32, py: f32);
    #[wasm_bindgen(method)]
    pub(super) fn resize(this: &OpCk, w: u32, h: u32);
    /// Set the device-pixel-ratio used to supersample the offscreen text raster
    /// so glyph bitmaps stay crisp on HiDPI displays.
    #[wasm_bindgen(method, js_name = setDpr)]
    pub(super) fn set_dpr(this: &OpCk, dpr: f32);
}

#[wasm_bindgen(module = "/src/op_ck_image_cache.js")]
extern "C" {
    #[wasm_bindgen(js_name = webImageCacheFactory)]
    pub(super) fn web_image_cache_factory() -> js_sys::Function;
}
