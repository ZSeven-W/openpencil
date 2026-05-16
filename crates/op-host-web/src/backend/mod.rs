//! `WebBackend` — Step 1b shell-web RenderBackend implementation.
//!
//! Step 1b §2.2 (post C-hard.2 lock-in 2026-05-09): wasm32-unknown-unknown
//! via the vendor/skia-safe-op fork + crates/wasm-libc-shim. This backend
//! draws into a skia-safe raster surface (N32_PREMUL) and presents the
//! snapshot to the host `<canvas>` via `CanvasRenderingContext2d::
//! put_image_data`. Phase A target = single red rectangle on a 960×640
//! canvas; Phase B+ adds widget tree dispatch through shell-core's
//! `RenderBackend` trait.
//!
//! Skia GL backend (WebGL2 via Skia GL) is deferred to Phase A round 2 once
//! the raster path is proven; raster fallback contract is documented in spec
//! §5.3.

pub mod skia_wasm;

use op_editor_ui::{Color, Point2D, Rect, RenderBackend, TextLayout};
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData};

/// Embedded Roboto-Regular TTF (35 KB, Apache 2.0 — copy of the
/// rust-skia test font asset). Step 3 codex stop-hook fix: the
/// C-hard wasm32-unknown-unknown build uses the
/// `skia_enable_fontmgr_custom_empty=yes` GN flag (see
/// `vendor/skia-safe-op/skia-bindings/build_support/platform/
/// wasm_unknown.rs`), which means there are NO system fonts
/// available to skia. Without an embedded TTF + Typeface, every
/// `draw_str` call silently produces no glyphs (Step 3 web smoke
/// "could not render the claimed canvas text"). We bake the font
/// into the bundle once at build time.
const ROBOTO_TTF: &[u8] = include_bytes!("../../assets/Roboto-Regular.ttf");

/// Embedded Noto Sans CJK subset (~8.7 KB, SIL OFL 1.1 — copy of
/// the rust-skia test resource at
/// `vendor/skia-safe-op/.../resources/fonts/NotoSansCJK-VF-subset.otf.ttc`).
/// Step 4 codex stop-hook fix: shell-core chrome strings include
/// CJK (`页面 / 图层 / 未命名 / 用 AI 开始设计` etc.) which Roboto
/// has no glyphs for, so the Roboto-only text path was rendering
/// `.notdef` boxes. The subset bundles enough Han / Latin coverage
/// to keep all chrome labels legible at <10 KB cost (bundle is
/// still well under the 1 MiB §6 ceiling — 916 KB vs 1 MiB after
/// adding this).
const NOTO_CJK_SUBSET: &[u8] = include_bytes!("../../assets/NotoSansCJK-Subset.ttc");

pub struct WebBackend {
    surface: skia_safe::Surface,
    canvas_element: HtmlCanvasElement,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    dpi_scale: f32,
    /// Lazy-initialised typeface backed by the embedded Roboto
    /// TTF. Built on first text draw so a no-text canvas pays no
    /// cost. `None` after a failed build (typeface invalid /
    /// alloc failure); the separate `typeface_tried` flag below
    /// gates retry so subsequent draw_text calls don't re-parse
    /// the TTF on every frame after a one-time failure (codex
    /// Step 3 R1 NIT-1).
    typeface: Option<skia_safe::Typeface>,
    /// Sticky flag: true once we've attempted typeface init,
    /// regardless of outcome. Subsequent draw_text calls skip
    /// the FontMgr / from_data round-trip if `typeface` is
    /// still None.
    typeface_tried: bool,
    /// Lazy-initialised CJK fallback typeface backed by
    /// [`NOTO_CJK_SUBSET`]. Same lazy + sticky-flag pattern as
    /// `typeface` above; covers the Han glyphs the chrome strings
    /// need without forcing a Roboto-only `.notdef` path.
    cjk_typeface: Option<skia_safe::Typeface>,
    cjk_typeface_tried: bool,
    /// Most recent present() error captured via the infallible
    /// `end_frame` trait method. Callers that need to propagate errors
    /// (e.g. `WebShell::mount`) call `take_present_error()` after the
    /// paint cycle.
    last_present_error: Option<JsValue>,
}

impl WebBackend {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        let surface = skia_wasm::make_raster_surface(width, height)?;
        let pixels = vec![0u8; (width as usize) * (height as usize) * 4];
        Ok(Self {
            surface,
            canvas_element: canvas,
            pixels,
            width,
            height,
            dpi_scale: 1.0,
            typeface: None,
            typeface_tried: false,
            cjk_typeface: None,
            cjk_typeface_tried: false,
            last_present_error: None,
        })
    }

    /// Drain and return the most recent `end_frame` present error, if any.
    /// Cleared on read so subsequent frames do not re-surface the same
    /// failure. Use in mount entry / Phase E manual smoke verification.
    pub fn take_present_error(&mut self) -> Option<JsValue> {
        self.last_present_error.take()
    }

    /// Current physical width of the host `<canvas>` element, refreshed
    /// at construction + on every `resize` call. Hosts use this so a
    /// repaint after the canvas DOM attribute changes pulls the new
    /// width through the WidgetHost layout (codex Step 3 stop-hook
    /// "web repaint ignores actual canvas size").
    pub fn canvas_width(&self) -> u32 {
        self.width
    }

    /// Current physical height of the host `<canvas>` element. Same
    /// refresh contract as `canvas_width`.
    pub fn canvas_height(&self) -> u32 {
        self.height
    }

    /// Snapshot the raster surface and `put_image_data` it onto the host
    /// `<canvas>`'s 2D context. Spec §5.3 raster-fallback contract:
    /// N32_PREMUL surface, RGBA8888 + Unpremul read, full-frame copy each
    /// frame. Phase A measures latency; >16ms (60fps budget) → revisit GL
    /// backend in Phase A round 2.
    pub fn present(&mut self) -> Result<(), JsValue> {
        let image = self.surface.image_snapshot();
        let info = skia_safe::ImageInfo::new(
            (self.width as i32, self.height as i32),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Unpremul,
            None,
        );
        let ok = image.read_pixels(
            &info,
            self.pixels.as_mut_slice(),
            (self.width as usize) * 4,
            (0, 0),
            skia_safe::image::CachingHint::Allow,
        );
        if !ok {
            return Err(JsValue::from_str("WebBackend: read_pixels failed"));
        }
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(self.pixels.as_mut_slice()),
            self.width,
            self.height,
        )?;
        let context = self
            .canvas_element
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("WebBackend: 2d context unavailable"))?
            .dyn_into::<CanvasRenderingContext2d>()?;
        context.put_image_data(&image_data, 0.0, 0.0)
    }
}

/// Implementation of shell-core `RenderBackend` over the raster surface.
/// Phase A wires `fill_rect` + `begin_frame` / `end_frame` only; the
/// remaining widget-facing methods are conservative stubs that Phase B+
/// will fill in alongside the shell-core widget set.
impl RenderBackend for WebBackend {
    fn begin_frame(&mut self) {
        // Raster surface starts each frame with whatever pixels the previous
        // present left behind; widgets that need a clean canvas should
        // explicitly clear via `fill_rect` covering the viewport.
    }

    fn end_frame(&mut self) {
        // RenderBackend trait is infallible by design (native + web share
        // the same shape). end_frame's fallible work is `present()` →
        // ImageData round-trip, which CAN fail (read_pixels / 2d-context /
        // put_image_data). We surface the latest result in
        // `last_present_error` so the caller can propagate it; both the
        // success and failure cases are stored, so a stale prior failure
        // does NOT bleed into a subsequent frame's status.
        // Callers that need real propagation (e.g. WebShell::mount) check
        // `take_present_error()` after each end_frame.
        match self.present() {
            Ok(()) => {
                self.last_present_error = None;
            }
            Err(e) => {
                web_sys::console::error_1(&e);
                self.last_present_error = Some(e);
            }
        }
    }

    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        self.surface.canvas().draw_rect(
            skia_safe::Rect::from_xywh(rect.origin.x, rect.origin.y, rect.size.x, rect.size.y),
            &paint,
        );
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32) {
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        self.surface.canvas().draw_rect(
            skia_safe::Rect::from_xywh(rect.origin.x, rect.origin.y, rect.size.x, rect.size.y),
            &paint,
        );
    }

    fn stroke_line(&mut self, from: Point2D, to: Point2D, color: Color, width: f32) {
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        paint.set_stroke_cap(skia_safe::PaintCap::Round);
        self.surface
            .canvas()
            .draw_line((from.x, from.y), (to.x, to.y), &paint);
    }

    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        let paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        let sk_rect =
            skia_safe::Rect::from_xywh(rect.origin.x, rect.origin.y, rect.size.x, rect.size.y);
        self.surface
            .canvas()
            .draw_round_rect(sk_rect, radius, radius, &paint);
    }

    fn fill_drop_shadow(&mut self, rect: Rect, radius: f32, blur: f32, color: Color) {
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_anti_alias(true);
        // CSS blur-radius → gaussian sigma is `radius / 2`; a zero
        // blur degrades to a crisp filled round rect.
        let sigma = blur * 0.5;
        if sigma > 0.0 {
            if let Some(mask) =
                skia_safe::MaskFilter::blur(skia_safe::BlurStyle::Normal, sigma, false)
            {
                paint.set_mask_filter(mask);
            }
        }
        let sk_rect =
            skia_safe::Rect::from_xywh(rect.origin.x, rect.origin.y, rect.size.x, rect.size.y);
        self.surface
            .canvas()
            .draw_round_rect(sk_rect, radius, radius, &paint);
    }

    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32) {
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        let sk_rect =
            skia_safe::Rect::from_xywh(rect.origin.x, rect.origin.y, rect.size.x, rect.size.y);
        self.surface
            .canvas()
            .draw_round_rect(sk_rect, radius, radius, &paint);
    }

    fn fill_oval(&mut self, bounds: Rect, color: Color) {
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_anti_alias(true);
        let sk_rect = skia_safe::Rect::from_xywh(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.x,
            bounds.size.y,
        );
        self.surface.canvas().draw_oval(sk_rect, &paint);
    }

    fn stroke_oval(&mut self, bounds: Rect, color: Color, width: f32) {
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_anti_alias(true);
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        let sk_rect = skia_safe::Rect::from_xywh(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.x,
            bounds.size.y,
        );
        self.surface.canvas().draw_oval(sk_rect, &paint);
    }

    fn fill_polygon(&mut self, points: &[Point2D], color: Color) {
        if points.len() < 3 {
            return;
        }
        let mut builder = skia_safe::PathBuilder::new();
        builder.move_to((points[0].x, points[0].y));
        for p in &points[1..] {
            builder.line_to((p.x, p.y));
        }
        builder.close();
        let path = builder.detach();
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_anti_alias(true);
        self.surface.canvas().draw_path(&path, &paint);
    }

    fn stroke_svg_path(&mut self, d: &str, top_left: Point2D, size: f32, color: Color, width: f32) {
        let Some(path) = skia_safe::utils::parse_path::from_svg(d) else {
            return;
        };
        let s = size / 24.0;
        let mut matrix = skia_safe::Matrix::new_identity();
        matrix.set_scale_translate((s, s), (top_left.x, top_left.y));
        let path = path.with_transform(&matrix);
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_stroke(true);
        paint.set_stroke_width(width);
        paint.set_anti_alias(true);
        paint.set_stroke_cap(skia_safe::PaintCap::Round);
        paint.set_stroke_join(skia_safe::PaintJoin::Round);
        self.surface.canvas().draw_path(&path, &paint);
    }

    fn fill_svg_path(
        &mut self,
        d: &str,
        top_left: Point2D,
        size: f32,
        viewbox: f32,
        color: Color,
    ) {
        let Some(path) = skia_safe::utils::parse_path::from_svg(d) else {
            return;
        };
        let s = size / viewbox;
        let mut matrix = skia_safe::Matrix::new_identity();
        matrix.set_scale_translate((s, s), (top_left.x, top_left.y));
        let path = path.with_transform(&matrix);
        let mut paint = skia_safe::Paint::new(
            skia_safe::Color4f::new(color.r, color.g, color.b, color.a),
            None,
        );
        paint.set_anti_alias(true);
        self.surface.canvas().draw_path(&path, &paint);
    }

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        // Step 3 codex stop-hook fix: actually render text via
        // `Canvas::draw_str` + embedded Typefaces (the C-hard
        // `custom_empty` font manager has no system fonts on
        // wasm32). Lazy-init both typefaces on first call; if a
        // build fails we leave the slot at `None` and silently
        // skip glyphs from that script so a font issue never
        // crashes paint.
        if !self.typeface_tried {
            // The C-hard wasm32-unknown-unknown skia build uses
            // FontMgr::custom_empty() — there's no system font
            // path. Construct one explicitly + register the
            // embedded Roboto bytes via new_from_data; both
            // calls return `Option`, so `None` propagates up
            // and the typeface stays None on any failure.
            // `typeface_tried` flips to true regardless of
            // success so a one-time failure doesn't re-parse
            // the TTF on every frame (codex Step 3 R1 NIT-1).
            self.typeface = skia_safe::FontMgr::custom_empty()
                .and_then(|mgr| mgr.new_from_data(ROBOTO_TTF, None));
            self.typeface_tried = true;
        }
        if !self.cjk_typeface_tried {
            // Step 4 codex stop-hook fix: chrome includes CJK
            // labels (`页面`, `图层`, `未命名` etc.) so a
            // Roboto-only path produces `.notdef` boxes for every
            // non-ASCII run. Build a second typeface from the
            // embedded Noto Sans CJK subset.
            self.cjk_typeface = skia_safe::FontMgr::custom_empty()
                .and_then(|mgr| mgr.new_from_data(NOTO_CJK_SUBSET, None));
            self.cjk_typeface_tried = true;
        }
        for run in layout.runs() {
            let typeface = if run.content.is_ascii() {
                self.typeface.as_ref()
            } else {
                // Non-ASCII run — prefer the CJK subset; if it
                // failed to init, fall back to Roboto (still
                // wrong, but at least the run isn't dropped
                // silently).
                self.cjk_typeface.as_ref().or(self.typeface.as_ref())
            };
            let Some(typeface) = typeface else {
                continue;
            };
            let font = skia_safe::Font::new(typeface, run.font_size);
            let jc = run.color;
            let mut paint = skia_safe::Paint::new(
                skia_safe::Color4f::new(
                    f32::from(jc.r()) / 255.0,
                    f32::from(jc.g()) / 255.0,
                    f32::from(jc.b()) / 255.0,
                    f32::from(jc.a()) / 255.0,
                ),
                None,
            );
            paint.set_anti_alias(true);
            // The wasm32 build ships a single weight per typeface
            // (Roboto-Regular + Noto CJK subset). Synthesise bold
            // for `fontWeight: 700`+ via stroke-and-fill so login.op
            // headlines paint heavy in the browser too — mirrors
            // the native backend's behaviour (skia.rs::draw_text).
            if run.font_weight >= 600 {
                paint.set_style(skia_safe::PaintStyle::StrokeAndFill);
                paint.set_stroke_width(run.font_size * 0.06);
            }
            let run_origin = run.origin;
            self.surface.canvas().draw_str(
                run.content.as_str(),
                (origin.x + run_origin.x, origin.y + run_origin.y),
                &font,
                &paint,
            );
        }
    }

    fn clip_rect(&mut self, rect: Rect) {
        self.surface.canvas().clip_rect(
            skia_safe::Rect::from_xywh(rect.origin.x, rect.origin.y, rect.size.x, rect.size.y),
            None,
            true,
        );
    }

    fn save(&mut self) {
        self.surface.canvas().save();
    }

    fn restore(&mut self) {
        self.surface.canvas().restore();
    }

    fn translate(&mut self, offset: Point2D) {
        self.surface.canvas().translate((offset.x, offset.y));
    }

    fn rotate(&mut self, radians: f32, pivot: Point2D) {
        let degrees = radians.to_degrees();
        self.surface
            .canvas()
            .rotate(degrees, Some(skia_safe::Point::new(pivot.x, pivot.y)));
    }

    fn resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if width == self.width && height == self.height {
            return;
        }
        if let Ok(surface) = skia_wasm::make_raster_surface(width, height) {
            self.surface = surface;
            self.pixels = vec![0u8; (width as usize) * (height as usize) * 4];
            self.width = width;
            self.height = height;
            self.canvas_element.set_width(width);
            self.canvas_element.set_height(height);
        }
    }

    fn dpi_scale(&self) -> f32 {
        self.dpi_scale
    }

    fn measure_text(&mut self, text: &str, font_size: f32) -> f32 {
        // Init typefaces if not already (mirrors draw_text).
        if !self.typeface_tried {
            self.typeface = skia_safe::FontMgr::custom_empty()
                .and_then(|mgr| mgr.new_from_data(ROBOTO_TTF, None));
            self.typeface_tried = true;
        }
        if !self.cjk_typeface_tried {
            self.cjk_typeface = skia_safe::FontMgr::custom_empty()
                .and_then(|mgr| mgr.new_from_data(NOTO_CJK_SUBSET, None));
            self.cjk_typeface_tried = true;
        }
        let typeface = if text.is_ascii() {
            self.typeface.as_ref()
        } else {
            self.cjk_typeface.as_ref().or(self.typeface.as_ref())
        };
        let Some(typeface) = typeface else {
            return text.chars().count() as f32 * font_size * 0.55;
        };
        let font = skia_safe::Font::new(typeface, font_size);
        let (advance, _bounds) = font.measure_str(text, None);
        advance
    }

    /// Weight-aware measure for the wasm32 backend. The bundle ships
    /// a single-weight Roboto-Regular + Noto CJK subset and emulates
    /// bold via the stroke-and-fill paint trick in `draw_text` (see
    /// the `run.font_weight >= 600` branch above). That means weight
    /// does NOT change glyph advance widths here — wrap decisions
    /// stay aligned with paint as long as we route through the
    /// same `Font::measure_str` path. Without this override the
    /// trait's default forwards to the heuristic in `render_backend.rs`,
    /// diverging from `draw_text` (codex BLOCK).
    fn measure_text_weighted(&mut self, text: &str, font_size: f32, _weight: u16) -> f32 {
        self.measure_text(text, font_size)
    }
}
