//! Step 1b §1.4 native widget glue — the only file in shell-native
//! allowed to call into `openpencil_shell_core::widgets`. Mirrors the
//! shell-web `widget_host.rs` pattern so the four inspector widgets
//! (Tree / PropertyRow / Dropdown / TextInput) defined in shell-core
//! are truly cross-platform: shell-web reuses them via `WebBackend`,
//! shell-native via `NativeBackend` + this glue.
//!
//! Two pieces:
//!
//! 1. [`NativeFrameBackend`] — a frame-scoped wrapper that holds a
//!    `(&mut NativeBackend, &skia_safe::Canvas)` pair and `impl
//!    RenderBackend` for it. Spec §5.2.1 explicitly defers the
//!    `RenderBackend` impl on the canvas-borrow-bearing `NativeBackend`
//!    to Step 1c+ widget tree work; this is that landing site.
//!
//! 2. [`WidgetHostNative`] — owns one of each of the Step 1b widgets +
//!    a `paint(&self, &mut NativeFrameBackend, available_width)`
//!    method that dispatches like shell-web's WidgetHost. Phase D will
//!    add `apply_*` event handlers (winit input → shell-core
//!    `apply_ime` / `apply_key`) once the desktop event pipeline
//!    needs them.
//!
//! ### Mobile (iOS / Android) — Step 1f path
//!
//! Per spec §11 invariants: shell-native is gated to desktop OS
//! today (`backend` / `canvas_view_stub` / `widget_host` modules
//! cfg-gated to `macos | linux | windows`). Mobile widget rendering
//! lands in Step 1f via `context::EaglProvider` (iOS) /
//! `context::AndroidEglProvider` (Android) — both are zero-sized
//! placeholder structs in lib.rs today whose `GlContextProvider`
//! impls `unimplemented!()`. Per the 2026-05-10 user directive
//! ("安卓和ios 不需要 ipc / 本地 cli — 只需要 custom provider"):
//! mobile rendering is purely a custom-provider plugin point on the
//! existing `GlContextProvider` trait, NOT a separate IPC / CLI
//! pipeline.
//!
//! Crucially the widget glue here is platform-agnostic in shape:
//! - `NativeFrameBackend` only holds `&mut NativeBackend` +
//!   `&skia_safe::Canvas`; both compile on any target where
//!   skia-safe + jian-skia + the GL provider compile. No
//!   desktop-specific type names or APIs leak in.
//! - `WidgetHostNative` only consumes shell-core widgets + the
//!   `RenderBackend` trait; no winit / glutin / EGL types appear.
//! - When Step 1f promotes `NativeBackend` to compile on
//!   mobile (drop the desktop-only cfg in lib.rs once
//!   `EaglProvider` / `AndroidEglProvider` ship real impls),
//!   `WidgetHostNative` follows automatically — no rewrite, no
//!   additional glue. The mobile shell just constructs a different
//!   `SharedSkiaContext` (or the mobile equivalent) backed by its
//!   provider, then runs the same `host.paint(&mut frame, width)`.

use crate::backend::NativeBackend;
use openpencil_shell_core::document::Document;
use openpencil_shell_core::widgets::{
    CanvasViewport, LayerPanel, LayoutCx, PaintCx, PropertyPanel, Toolbar, Widget, MIN_RAIL_WIDTH,
};
use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend, TextLayout};

/// Frame-scoped `RenderBackend` adapter over `NativeBackend` +
/// `&Canvas`. Lifetime-bound to the `SharedSkiaContext::with_frame`
/// closure body so widget code never sees the canvas borrow directly.
///
/// Why this exists rather than `impl RenderBackend for NativeBackend`:
/// `NativeBackend` deliberately does NOT carry a canvas borrow (spec
/// §5.2.1 — the canvas reference is short-lived inside `with_frame`,
/// while `NativeBackend` lives across frames so its `jian_skia::
/// SkiaBackend` image cache survives). The wrapper gets the trait
/// shape without entangling the lifetime onto the long-lived backend
/// type.
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
    fn begin_frame(&mut self) {
        // No-op on native: `SharedSkiaContext::begin_frame` already
        // ran when the caller entered `with_frame`.
    }

    fn end_frame(&mut self) {
        // No-op on native: `SharedSkiaContext::present` runs after
        // `with_frame` returns, outside the wrapper's lifetime.
    }

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
        // `NativeBackend::save` returns the pre-save count, used by
        // `restore_to`. The trait-level `save` is fire-and-forget; we
        // discard the count and rely on the caller pairing each
        // `save` with one `restore`.
        let _count = self.inner.save(self.canvas);
    }

    fn restore(&mut self) {
        self.inner.restore(self.canvas);
    }

    fn translate(&mut self, offset: Point2D) {
        self.inner.translate(self.canvas, offset);
    }

    fn resize(&mut self, _width: u32, _height: u32) {
        // No-op via the trait: surface resize is owned by
        // `SharedSkiaContext::resize` and reaches `NativeBackend`
        // separately. Mirrors `NativeBackend::resize`'s no-op.
    }

    fn dpi_scale(&self) -> f32 {
        self.inner.dpi_scale()
    }
}

/// Native counterpart of shell-web's `widget_host::WidgetHost`.
/// Owns the document model + toolbar state; per-frame builds
/// LayerPanel / PropertyPanel / CanvasViewport / Toolbar from the
/// document and paints them in the same Toolbar-top + LayerPanel-
/// left + CanvasViewport-center + PropertyPanel-right layout
/// shell-web uses (Step 3), so cross-platform visual diff testing
/// compares apples to apples.
///
/// Step 1b/2 holdovers (Dropdown + TextInput aux widgets) retired
/// in Step 3 — the canvas viewport is the centerpiece now.
pub struct WidgetHostNative {
    document: Document,
    toolbar: Toolbar,
}

impl WidgetHostNative {
    pub fn new() -> Self {
        Self {
            document: Document::sample(),
            toolbar: Toolbar::default_set(),
        }
    }

    /// Paint the editor-UI composition. Layout matches shell-web's
    /// `WidgetHost::paint` for cross-platform parity:
    ///   - Toolbar pinned top, full width
    ///   - LayerPanel left rail
    ///   - CanvasViewport center (real document render)
    ///   - PropertyPanel right rail
    ///
    /// `// glue:` marker on the signature line keeps the future
    /// `tools/check-widget-boundary.sh` happy if the boundary script
    /// is extended to gate shell-native too (it currently scans only
    /// shell-web; Step 3+ may parameterize).
    pub fn paint(&self, frame: &mut NativeFrameBackend<'_>, viewport_width: f32) { // glue:
        let layout = LayoutCx {
            available_width: viewport_width,
            dpi: frame.dpi_scale(),
        };

        // Toolbar at the top of the viewport.
        let toolbar_box = self.toolbar.layout(&layout);
        let toolbar_rect = Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(viewport_width, toolbar_box.rect.size.y),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            self.toolbar.paint(&mut cx, toolbar_rect);
        }

        // Build editor-UI views from the current document.
        let layer_panel = LayerPanel::from_document(&self.document);
        let property_panel = PropertyPanel::for_selected(&self.document);
        let canvas = CanvasViewport::from_document(&self.document);

        // Rail widths clamped to non-negative + skipped below
        // the minimum usable width (codex Step 2 R1 CONCERN-3).
        // Rails take ~1/4 width each so the canvas gets the
        // middle ~1/2.
        let rail_w_raw = ((viewport_width / 4.0) - 8.0).min(240.0);
        let rail_w = rail_w_raw.max(0.0);
        if rail_w < MIN_RAIL_WIDTH {
            return;
        }
        let rail_top_y = toolbar_rect.size.y + 8.0;
        let rail_layout_cx = LayoutCx {
            available_width: rail_w,
            dpi: frame.dpi_scale(),
        };

        // LayerPanel pinned to the left.
        let lp_layout = layer_panel.layout(&rail_layout_cx);
        let lp_rect = Rect {
            origin: Point2D::new(8.0, rail_top_y),
            size: Point2D::new(rail_w, lp_layout.rect.size.y),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            layer_panel.paint(&mut cx, lp_rect);
        }

        // PropertyPanel pinned to the right.
        let pp_layout = property_panel.layout(&rail_layout_cx);
        let pp_rect = Rect {
            origin: Point2D::new(viewport_width - rail_w - 8.0, rail_top_y),
            size: Point2D::new(rail_w, pp_layout.rect.size.y),
        };
        {
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            property_panel.paint(&mut cx, pp_rect);
        }

        // CanvasViewport in the middle band — between the two
        // rails, below the toolbar. Window height isn't passed
        // through this signature; assume 600 px (matches default
        // winit window in inspector_window) minus the top rail
        // start. Step 4+ may pass viewport_height too.
        let canvas_x = lp_rect.origin.x + lp_rect.size.x + 8.0;
        let canvas_w = (pp_rect.origin.x - canvas_x - 8.0).max(0.0);
        if canvas_w >= MIN_RAIL_WIDTH {
            let canvas_rect = Rect {
                origin: Point2D::new(canvas_x, rail_top_y),
                size: Point2D::new(canvas_w, 600.0 - rail_top_y),
            };
            let mut cx = PaintCx {
                backend: &mut *frame,
            };
            canvas.paint(&mut cx, canvas_rect);
        }
    }
}

impl Default for WidgetHostNative {
    fn default() -> Self {
        Self::new()
    }
}
