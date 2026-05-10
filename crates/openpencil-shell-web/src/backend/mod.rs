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

use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend, TextLayout};
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
    /// alloc failure); subsequent draw_text calls become no-ops
    /// rather than retrying.
    typeface: Option<skia_safe::Typeface>,
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
            last_present_error: None,
        })
    }

    /// Drain and return the most recent `end_frame` present error, if any.
    /// Cleared on read so subsequent frames do not re-surface the same
    /// failure. Use in mount entry / Phase E manual smoke verification.
    pub fn take_present_error(&mut self) -> Option<JsValue> {
        self.last_present_error.take()
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

    fn draw_text(&mut self, layout: &TextLayout, origin: Point2D) {
        // Step 3 codex stop-hook fix: actually render text via
        // `Canvas::draw_str` + the embedded Roboto Typeface (the
        // C-hard `custom_empty` font manager has no system fonts).
        // Lazy-init the typeface on first call; if the build fails
        // we leave `self.typeface = None` and silently no-op
        // subsequent calls so a font issue never crashes paint.
        if self.typeface.is_none() {
            // The C-hard wasm32-unknown-unknown skia build uses
            // FontMgr::custom_empty() — there's no system font
            // path. Construct one explicitly + register the
            // embedded Roboto bytes via new_from_data; both
            // calls return `Option`, so `None` propagates up
            // and the typeface stays None on any failure.
            self.typeface = skia_safe::FontMgr::custom_empty()
                .and_then(|mgr| mgr.new_from_data(ROBOTO_TTF, None));
        }
        let Some(typeface) = self.typeface.as_ref() else {
            return;
        };
        for run in layout.runs() {
            let font = skia_safe::Font::new(typeface, run.font_size);
            let jc = run.color;
            let paint = skia_safe::Paint::new(
                skia_safe::Color4f::new(
                    f32::from(jc.r()) / 255.0,
                    f32::from(jc.g()) / 255.0,
                    f32::from(jc.b()) / 255.0,
                    f32::from(jc.a()) / 255.0,
                ),
                None,
            );
            let run_origin = run.origin;
            self.surface.canvas().draw_str(
                run.content.as_str(),
                (
                    origin.x + run_origin.x,
                    origin.y + run_origin.y,
                ),
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
}
