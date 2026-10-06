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

/// Serializes concurrent exports through the decode pump. The pending-decode
/// queue is process-global, and `take_pending_decodes` moves entries into a
/// global in-flight set: a concurrent consumer can TAKE an id this export's
/// discovery pass just recorded, install the raster into ITS backend, and
/// leave this export's thread-local backend without the bitmap while the
/// queue reads empty. Two parallel `export_node_raster` calls did exactly
/// that on the macos-aarch64 CI leg (2026-08-28): the export shipped the
/// placeholder glyph where the decoded bitmap belonged. Exports are rare and
/// short; serializing them is free.
static DECODE_PUMP: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Headless paints have no event loop to pump the async image-decode
/// seam, so a single pass would export the editor's placeholder art for
/// every not-yet-rasterized image. Run discovery passes on a throwaway
/// 1×1 surface (paint records pending decode ids + fills the byte
/// cache), decode them synchronously into the backend's raster cache,
/// and repeat until the scene stops producing work.
///
/// Exit discipline: an empty take alone does NOT mean the scene is fully
/// decoded. An id taken by an external pump (an editor frame loop in the
/// same process) sits in the global in-flight set, where it suppresses
/// re-recording, until that pump marks it done — only then does the next
/// discovery pass re-record the miss so this export can decode it into its
/// own backend. So a pass is conclusive only when NOTHING was in flight
/// before its paint ran (so the paint could record every miss) AND the take
/// after the paint found nothing AND nothing is in flight after the take.
/// The pre-paint sample matters: an external `mark_decode_done` landing
/// between the discovery paint and a post-only in-flight check made an
/// empty take look conclusive even though that paint had still been
/// suppressed from re-recording the released id — the export returned
/// without the bitmap and shipped placeholder art (windows-x86_64 CI,
/// 2026-08-29, the stolen-decode regression test).
fn ensure_images_decoded(backend: &mut NativeBackend, nodes: &[SceneNode]) {
    use op_editor_ui::widgets::canvas_viewport_image::{
        cached_bytes_for, has_in_flight_decodes, mark_decode_done, take_pending_decodes,
    };
    let _pump = DECODE_PUMP
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // The scene is static, but the pending queue is bounded (one discovery
    // pass may not capture every miss) and an external consumer can hold an
    // id in flight across passes — iterate, with a bounded budget.
    for _ in 0..64 {
        // Sampled BEFORE the paint — see "Exit discipline" above.
        let in_flight_before_paint = has_in_flight_decodes();
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
            if !in_flight_before_paint && !has_in_flight_decodes() {
                return;
            }
            // Someone else is — or was, while this pass painted — decoding
            // an id this scene may need; give them a moment to mark it done
            // so a pass that starts clean can re-record it.
            std::thread::sleep(std::time::Duration::from_millis(1));
            continue;
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
