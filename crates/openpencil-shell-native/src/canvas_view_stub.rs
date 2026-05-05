//! `CanvasViewportStub` — surface-sharing + GL-state-isolation probe
//! (spec v19 §4 / round 4 BLOCK-R4-2 fix).
//!
//! Mimics the future Jian renderer API by drawing into the OP-owned Skia
//! canvas while deliberately leaving GL state perturbed (stencil test +
//! a non-default blend func). If `SharedSkiaContext` is correctly
//! sharing the GL context with chrome paint paths, the chrome's later
//! `fill_rect` etc. must still produce the expected pixels. The GPU
//! smoke + raster composition tests assert on this.
//!
//! The stub takes `&skia_safe::Canvas` (immutable) — Skia 0.97's canvas
//! draw / clip / transform methods are all `&self`, so the stub can
//! coexist with `NativeBackend::*` calls inside the same
//! `with_frame` closure (no borrow conflict).

use glow::HasContext;

/// Marker type — namespace for the static `render_into` entry point.
pub struct CanvasViewportStub;

impl CanvasViewportStub {
    /// Paint a Jian-renderer-shaped placeholder into `area`, then
    /// deliberately mutate the GL state without restoring. Used by
    /// GL-state-isolation tests to confirm `SharedSkiaContext`
    /// correctly resets state before the next chrome paint.
    pub fn render_into(
        glow_ctx: &glow::Context,
        canvas: &skia_safe::Canvas,
        area: skia_safe::Rect,
    ) {
        // 1. Paint a blue rectangle — a substitute for the (yet-to-be-built)
        //    Jian canvas widget. This proves the stub successfully wrote
        //    pixels into the same Skia surface that the chrome paints into.
        let mut paint = skia_safe::Paint::default();
        paint.set_color(skia_safe::Color::BLUE);
        paint.set_anti_alias(true);
        canvas.draw_rect(area, &paint);

        // 2. Pollute GL state without restoring (per spec §4).
        //    Tests rely on this to assert that chrome paints are
        //    insulated from foreign GL state by Skia's flush boundary.
        unsafe {
            glow_ctx.enable(glow::STENCIL_TEST);
            glow_ctx.blend_func(glow::ONE, glow::ZERO);
        }
    }
}
