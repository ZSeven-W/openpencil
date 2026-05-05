//! `NativeBackend` — frame-scoped widget facade backed by `jian_skia`.
//!
//! Spec v19 §5.2.1 (round 3 BLOCK-R3-3 fix): `NativeBackend` does **not**
//! own a canvas borrow and does **not** carry a `'a` lifetime. Every
//! drawing method takes `canvas: &skia_safe::Canvas` (immutable —
//! skia-safe 0.97 `Canvas::*` are all `&self`) and forwards the work to
//! `jian_skia::SkiaBackend::draw_on_canvas`. Callers obtain the borrow
//! inside `SharedSkiaContext::with_frame(|canvas, glow| { … })` and pass
//! it through.
//!
//! The backend deliberately does **not** `impl RenderBackend` for OP's
//! widget-facing trait (spec §5.2.1 mirror surface, no direct impl in
//! Step 1a; saved for Step 1c+ widget tree).

use openpencil_shell_core::{Color, Point2D, Rect, TextLayout};

/// OP-flavoured (`f32` 0..=1 RGBA) → Jian (`u8` packed RGBA) color.
///
/// Spec §5.2 / round 5 CONCERN-R5-3: jian-core `Color` is `Color(pub u32)`
/// without named constants; OP `Color::{RED, GREEN, BLUE, BLACK, WHITE,
/// TRANSPARENT}` lives in shell-core. This helper closes the gap.
pub fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn channel(v: f32) -> u8 {
        // `clamp` first — `as u8` saturates negative to 0 but unconstrained
        // > 1.0 would wrap (e.g. 256.0 → 0). Use `.clamp` then `*255`.
        let scaled = (v.clamp(0.0, 1.0) * 255.0).round();
        scaled as u8
    }
    jian_core::scene::Color::rgba(channel(c.r), channel(c.g), channel(c.b), channel(c.a))
}

/// OP `Rect` (`origin / size: Vec2`) → Jian `euclid::Rect<f32>`.
pub fn to_jian_rect(r: Rect) -> jian_core::geometry::Rect {
    jian_core::geometry::Rect::new(
        jian_core::geometry::Point::new(r.origin.x, r.origin.y),
        jian_core::geometry::Size::new(r.size.x, r.size.y),
    )
}

/// `skia_safe::Rect` from an OP `Rect` — used by `clip_rect`.
fn to_sk_rect(r: Rect) -> skia_safe::Rect {
    skia_safe::Rect::from_xywh(r.origin.x, r.origin.y, r.size.x, r.size.y)
}

/// Frame-scoped Jian-DrawOp adapter (spec v19 §5.2.1).
///
/// The struct stays alive across frames so its underlying
/// `jian_skia::SkiaBackend` keeps its image cache; the canvas borrow is
/// passed in per-method so `&NativeBackend` and `&Canvas` can coexist
/// inside the same `with_frame` closure without aliasing.
pub struct NativeBackend {
    skia: jian_skia::SkiaBackend,
    dpi: f32,
}

impl NativeBackend {
    /// Spec §5.2.1 / plan v7 Task 2 Step 11: take an externally
    /// constructed `SkiaBackend` so callers can re-use one across
    /// `NativeBackend` / Jian host adapters that rely on the same
    /// image cache. The convenience `with_dpi` covers the common
    /// "fresh backend" path.
    pub fn new(skia: jian_skia::SkiaBackend, dpi: f32) -> Self {
        Self { skia, dpi }
    }

    /// Convenience constructor for tests and the basic-window demo.
    pub fn with_dpi(dpi: f32) -> Self {
        Self::new(jian_skia::SkiaBackend::new(), dpi)
    }

    /// `SharedSkiaContext` doesn't change the DPI per-frame, so callers
    /// (e.g. `WindowEvent::ScaleFactorChanged` handler) push it down
    /// here.
    pub fn set_dpi(&mut self, dpi: f32) {
        self.dpi = dpi;
    }

    /// Logical→physical scale factor (`window.scale_factor()`).
    pub fn dpi_scale(&self) -> f32 {
        self.dpi
    }

    // ── Frame markers ───────────────────────────────────────────────────

    /// Mirrors `RenderBackend::begin_frame`; jian-skia v0.0.1's
    /// `begin_frame` is a buffer-replay marker that needs a `SkiaSurface`
    /// — OP holds the GPU surface itself, so this is a no-op. The
    /// `canvas` argument is kept for API symmetry with the future
    /// `WithCanvas<'a>` impl.
    pub fn begin_frame(&mut self, _canvas: &skia_safe::Canvas) {}

    /// Mirrors `RenderBackend::end_frame`. No-op for the same reason as
    /// `begin_frame`; `SharedSkiaContext::present` flushes + swaps.
    pub fn end_frame(&mut self, _canvas: &skia_safe::Canvas) {}

    // ── Drawing primitives ──────────────────────────────────────────────

    /// Direct `DrawOp` dispatch — used by callers that already build
    /// `jian_core::render::DrawOp` (e.g. `jian_host_desktop::scene::collect_draws_with_state`).
    /// Spec §5.2.1 round 5 CONCERN-R5-2: this is the only public path to
    /// `self.skia.draw_on_canvas` so `skia` can stay private.
    #[tracing::instrument(skip_all)]
    pub fn draw_op(&mut self, canvas: &skia_safe::Canvas, op: &jian_core::render::DrawOp) {
        self.skia.draw_on_canvas(canvas, op);
    }

    /// Filled rectangle. Translates `(rect, color)` → `DrawOp::Rect` with
    /// a solid `Paint`.
    #[tracing::instrument(skip(self, canvas))]
    pub fn fill_rect(&mut self, canvas: &skia_safe::Canvas, rect: Rect, color: Color) {
        let op = jian_core::render::DrawOp::Rect {
            rect: to_jian_rect(rect),
            paint: jian_core::render::Paint::solid(to_jian_color(color)),
        };
        self.draw_op(canvas, &op);
    }

    /// Stroked rectangle with a fixed width.
    #[tracing::instrument(skip(self, canvas))]
    pub fn stroke_rect(
        &mut self,
        canvas: &skia_safe::Canvas,
        rect: Rect,
        color: Color,
        width: f32,
    ) {
        let paint = jian_core::render::Paint {
            fill: None,
            stroke: Some(jian_core::render::StrokeOp {
                color: to_jian_color(color),
                width,
            }),
            opacity: color.a.clamp(0.0, 1.0),
        };
        let op = jian_core::render::DrawOp::Rect {
            rect: to_jian_rect(rect),
            paint,
        };
        self.draw_op(canvas, &op);
    }

    /// Submit every shaped run in the layout as a `DrawOp::Text`,
    /// translated by `origin`. The translation is non-mutating
    /// (`TextLayout::translated` returns a fresh layout), so callers can
    /// reuse the same layout from multiple paint passes.
    #[tracing::instrument(skip(self, canvas, layout))]
    pub fn draw_text(&mut self, canvas: &skia_safe::Canvas, layout: &TextLayout, origin: Point2D) {
        let translated = layout.translated(origin);
        for run in translated.runs() {
            let op = jian_core::render::DrawOp::Text(run.clone());
            self.draw_op(canvas, &op);
        }
    }

    /// Push a clip region. Spec §5.2.1 / plan Step 14f.
    pub fn clip_rect(&self, canvas: &skia_safe::Canvas, rect: Rect) {
        canvas.clip_rect(to_sk_rect(rect), None, None);
    }

    /// Save the current canvas state. Returns the save count so
    /// `restore_to` can pop back to it. (`Canvas::save` returns the
    /// pre-save count; we pass it through.)
    pub fn save(&self, canvas: &skia_safe::Canvas) -> usize {
        canvas.save()
    }

    /// Pop the most recent save.
    pub fn restore(&self, canvas: &skia_safe::Canvas) {
        canvas.restore();
    }

    /// Restore the canvas state stack down to a specific count returned
    /// by [`save`]. Mirrors `Canvas::restore_to_count`.
    pub fn restore_to(&self, canvas: &skia_safe::Canvas, count: usize) {
        canvas.restore_to_count(count);
    }

    /// Translate the current canvas matrix.
    pub fn translate(&self, canvas: &skia_safe::Canvas, offset: Point2D) {
        canvas.translate((offset.x, offset.y));
    }

    /// No-op; surface resize is owned by `SharedSkiaContext::resize`.
    pub fn resize(&mut self, _width: u32, _height: u32) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_roundtrip_clamps_and_packs() {
        let red = to_jian_color(Color::RED);
        assert_eq!(red.r(), 255);
        assert_eq!(red.g(), 0);
        assert_eq!(red.b(), 0);
        assert_eq!(red.a(), 255);

        let transparent = to_jian_color(Color::TRANSPARENT);
        assert_eq!(transparent.a(), 0);

        // Out-of-range channels are clamped, not wrapped.
        let weird = to_jian_color(Color {
            r: -0.5,
            g: 2.0,
            b: 0.5,
            a: 1.0,
        });
        assert_eq!(weird.r(), 0);
        assert_eq!(weird.g(), 255);
        assert_eq!(weird.b(), 128);
    }

    #[test]
    fn rect_translation_keeps_size() {
        let r = Rect {
            origin: Point2D::new(10.0, 20.0),
            size: Point2D::new(30.0, 40.0),
        };
        let jr = to_jian_rect(r);
        assert!((jr.min_x() - 10.0).abs() < 1e-6);
        assert!((jr.min_y() - 20.0).abs() < 1e-6);
        assert!((jr.size.width - 30.0).abs() < 1e-6);
        assert!((jr.size.height - 40.0).abs() < 1e-6);
    }
}
