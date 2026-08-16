//! Shared-painter bridge for raster export + `debug_screenshot`.
//!
//! Historically `export.rs` carried its own skia `paint_node` mirror
//! which diverged from the live canvas renderer (image nodes painted
//! as plain rects, `clip_content` ignored, text painted with a default
//! typeface and no styled runs). This module retires the mirror:
//! raster painting routes through the SAME scene painter the editor
//! canvas uses ([`op_editor_ui::widgets::canvas_viewport_paint`]),
//! driven over the export's offscreen `skia_safe::Canvas` via
//! [`op_host_native::NativeFrameBackend`]. One painter, one set of
//! pixels — what an LLM agent sees in a screenshot is what the user
//! sees on the canvas.
//!
//! The caller's canvas matrix already carries the export scale and the
//! margin translation (`render_raster_bytes`), so the shared painter
//! runs at `zoom = 1` with a zero viewport origin — exactly the
//! editor-canvas math at zoom 1 with the pan baked into the matrix.
//!
//! Vector writers stay on their own paths: `svg.rs` is a
//! hand-rolled SVG serializer (divergence documented there), while
//! `export_pdf.rs` paints through this bridge too — skia's PDF canvas
//! is a `skia_safe::Canvas`, so it inherits the same parity.

use op_editor_ui::layout_scene::SceneNode;
use op_editor_ui::widgets::{canvas_viewport_paint, PaintCx};
use op_editor_ui::{Point2D, Rect};
use op_host_native::{NativeBackend, NativeFrameBackend};
use skia_safe::Canvas;
use std::cell::RefCell;

thread_local! {
    /// Per-thread `NativeBackend` so repeated exports / screenshots
    /// don't re-pay the constructor's CJK font prewarm and keep the
    /// decoded-image + typeface caches warm across captures. The
    /// backend is frame-scoped by design (every method takes the
    /// canvas), so reusing it across surfaces is safe.
    static EXPORT_BACKEND: RefCell<NativeBackend> =
        RefCell::new(NativeBackend::with_dpi(1.0));
}

/// Cull rect covering all finite doc-space geometry. Export never
/// culls — the offscreen surface is already cropped to the painted
/// bounds, so every node the surface can show must paint.
fn no_cull() -> Rect {
    Rect {
        origin: Point2D::new(-1.0e15, -1.0e15),
        size: Point2D::new(2.0e15, 2.0e15),
    }
}

/// Paint one resolved scene node + subtree onto `canvas` through the
/// shared canvas painter (images, `clip_content`, styled text runs,
/// gradients, effects — full live-canvas semantics).
pub fn paint_node(canvas: &Canvas, node: &SceneNode) {
    paint_nodes(canvas, std::slice::from_ref(node));
}

/// Paint top-level page nodes in live-canvas z-order. Scene children
/// are ordered topmost-first (layer-panel order), so the painter walks
/// them in reverse — same as `canvas_viewport.rs`'s paint loop.
pub fn paint_nodes(canvas: &Canvas, nodes: &[SceneNode]) {
    EXPORT_BACKEND.with(|cell| {
        let mut backend = cell.borrow_mut();
        ensure_images_decoded(&mut backend, nodes);
        let mut frame = NativeFrameBackend::new(&mut backend, canvas);
        let mut cx = PaintCx {
            backend: &mut frame,
        };
        for node in nodes.iter().rev() {
            // `paint_node` is the off-canvas public entry: base scene only,
            // no editor overlays (reveal/hover/selection/pen).
            canvas_viewport_paint::paint_node(&mut cx, node, Point2D::ZERO, 1.0, no_cull());
        }
    });
}

/// Headless paints have no event loop to pump the async image-decode
/// seam, so a single pass would export the editor's placeholder art for
/// every not-yet-rasterized image. Run discovery passes on a throwaway
/// 1×1 surface (paint records pending decode ids + fills the byte
/// cache), decode them synchronously into the backend's raster cache,
/// and repeat until a pass records nothing new.
fn ensure_images_decoded(backend: &mut NativeBackend, nodes: &[SceneNode]) {
    use op_editor_ui::widgets::canvas_viewport_image::{
        cached_bytes_for, mark_decode_done, take_pending_decodes,
    };
    // Nested subtrees can reveal new images once parents decode is not a
    // thing (the scene is static), but the pending queue is bounded, so
    // one discovery pass may not capture every miss — iterate.
    for _ in 0..64 {
        let Some(mut surface) = skia_safe::surfaces::raster_n32_premul((1, 1)) else {
            return;
        };
        {
            let mut frame = NativeFrameBackend::new(backend, surface.canvas());
            let mut cx = PaintCx {
                backend: &mut frame,
            };
            for node in nodes.iter().rev() {
                canvas_viewport_paint::paint_node(&mut cx, node, Point2D::ZERO, 1.0, no_cull());
            }
        }
        let pending = take_pending_decodes(usize::MAX);
        if pending.is_empty() {
            return;
        }
        for entry in pending {
            if let Some(bytes) = cached_bytes_for(entry.id) {
                // Always full size here. The export scale lives in the
                // caller's canvas matrix, not in this backend's DPI, so
                // the size paint asks for understates what a 2x/4x
                // export needs — rastering full keeps exports sharp.
                if let Some(image) = op_host_native::decode_raster(&bytes) {
                    backend.install_raster_image(entry.id, image, u32::MAX);
                }
            }
            mark_decode_done(entry.id);
        }
    }
}
