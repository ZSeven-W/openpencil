//! The viewer's paint pipeline — the exact painter the desktop editor
//! canvas uses, driven onto a shell-owned GPU surface (or a raster
//! surface for the CPU frame path).
//!
//! `canvas_viewport_paint::paint_scene_page` paints in logical viewport
//! space: node doc-space rects are already scaled by the viewport
//! transform inside the painter (`viewport_origin + bounds * zoom`), so
//! the frame simply scales the canvas by `dpr` and hands over the
//! viewport. A tap-selected node gets a lightweight stroke overlay so
//! the viewer shows what is hit.

use crate::input::viewport_cull;
use crate::lifecycle::Session;
use jian_skia::SkiaSurface;
use op_editor_ui::layout_scene::{SceneNode, ScenePage};
use op_editor_ui::widgets::canvas_viewport_paint;
use op_editor_ui::widgets::PaintCx;
use op_editor_ui::{Color, Point2D, Rect, RenderBackend};
use op_host_native::NativeFrameBackend;

/// Viewer backdrop — a neutral canvas surface; the page paints over it.
const BACKDROP: Color = Color::rgb_u8(245, 245, 247);
/// Tap-selection highlight stroke (indigo, matching the editor's
/// selection accent family).
const SELECTION_COLOR: Color = Color::rgb_u8(79, 70, 229);
const SELECTION_STROKE: f32 = 2.0;

/// Paint one full frame: converge the image-decode queue, then paint
/// the active page + selection overlay onto `surface`.
pub(crate) fn paint_frame(session: &mut Session, surface: &mut SkiaSurface) {
    // The shared image registry records pending decodes during paint;
    // decode them synchronously and repaint until nothing new appears
    // (documents carry base64 data URLs; a single pass usually
    // converges). Remote images stay placeholders in the viewer.
    for _ in 0..64 {
        let discovered = discover_pending_decodes(session);
        if discovered == 0 {
            break;
        }
        decode_pending(session);
    }
    paint_into_canvas(session, surface.canvas());
}

/// Paint a discovery pass on a throwaway raster and report how many
/// pending decodes it recorded (mirrors the export path's
/// `ensure_images_decoded`).
///
/// The pass runs at the session's REAL size (capped): the viewer could
/// get away with 1×1, but the editor chrome lays out against the viewport
/// dimensions — a 1×1 paint would leave the widget host with 1-px panel
/// geometry and corrupt the following real frame.
fn discover_pending_decodes(session: &mut Session) -> usize {
    // Discovery runs on a 1×1 raster in VIEWER mode (image-decode
    // discovery only). In EDITOR mode it must paint at the real size —
    // the chrome lays out against the viewport — but it is EXPENSIVE, so
    // it runs only while new decodes keep appearing.
    #[cfg(feature = "editor")]
    if session.editor.is_some() {
        let (w, h) = session.logical;
        let width = ((w * session.dpr).round() as i32).clamp(1, 4096);
        let height = ((h * session.dpr).round() as i32).clamp(1, 4096);
        let Some(mut surface) = skia_safe::surfaces::raster_n32_premul((width, height)) else {
            return 0;
        };
        paint_into_canvas(session, surface.canvas());
        return op_editor_ui::widgets::canvas_viewport_image::pending_decode_count();
    }
    let Some(mut surface) = skia_safe::surfaces::raster_n32_premul((1, 1)) else {
        return 0;
    };
    paint_into_canvas(session, surface.canvas());
    op_editor_ui::widgets::canvas_viewport_image::pending_decode_count()
}

/// Install every pending encoded image into the backend's raster cache.
fn decode_pending(session: &mut Session) {
    use op_editor_ui::widgets::canvas_viewport_image::{
        cached_bytes_for, mark_decode_done, take_pending_decodes,
    };
    for pending in take_pending_decodes(usize::MAX) {
        let Some(bytes) = cached_bytes_for(pending.id) else {
            mark_decode_done(pending.id);
            continue;
        };
        if let Some((image, covers_edge_px)) =
            op_host_native::decode_raster_capped(&bytes, pending.max_edge_px)
        {
            session
                .backend
                .install_raster_image(pending.id, image, covers_edge_px);
        }
        mark_decode_done(pending.id);
    }
}

/// Paint the active page (plus selection overlay) onto `canvas`.
///
/// The session is borrowed field-wise so `scene` (read-only) and
/// `backend` (mutable) can be live at the same time — the frame adapter
/// holds `&mut NativeBackend` while the painter reads the scene.
pub(crate) fn paint_into_canvas(session: &mut Session, canvas: &skia_safe::Canvas) {
    // Editor mode paints the COMPLETE desktop chrome (top bar, layers,
    // property panel, toolbar, canvas) through the widget host; the
    // viewer mode paints the bare scene below.
    #[cfg(feature = "editor")]
    {
        // The chrome lays out against the stable safe-area-local viewport.
        // Keyboard occlusion is separate host state consumed only by focused
        // input surfaces, so showing the IME never resizes the canvas or app
        // chrome. Computed before the split borrow so the host/backend mutable
        // borrows stay disjoint.
        let (usable_w, usable_h) = session.editor_viewport();
        let (insets_left, insets_top) = (session.insets.left, session.insets.top);
        // Split borrows: the editor host and the backend are disjoint
        // fields of the session.
        let Session {
            editor, backend, ..
        } = session;
        if let Some(host) = editor.as_mut() {
            let root_background =
                op_editor_ui::widgets::editor_state_ext::theme_for(&host.editor_state().editor_ui)
                    .background;
            // Clear the complete drawable before translating into the usable
            // viewport. Extending the root theme surface into the platform-
            // owned bands keeps the status/cutout/gesture area visually
            // continuous without moving controls into unsafe space.
            canvas.reset_matrix();
            canvas.clear(skia_safe::Color4f::new(
                root_background.r,
                root_background.g,
                root_background.b,
                root_background.a,
            ));
            // The desktop runner scales the canvas by the DPI factor
            // before painting the chrome; the player must do the same so
            // the logical-point layout maps onto the physical surface.
            let (w, h) = (usable_w, usable_h);
            canvas.scale((session.dpr, session.dpr));
            if insets_left > 0.0 || insets_top > 0.0 {
                canvas.translate((insets_left, insets_top));
            }
            let mut frame = NativeFrameBackend::new(backend, canvas);
            frame.fill_rect(
                Rect {
                    origin: Point2D::new(0.0, 0.0),
                    size: Point2D::new(w, h),
                },
                Color::BLACK,
            );
            host.paint(&mut frame, w, h);
            return;
        }
    }

    // Viewer mode keeps its neutral backdrop edge-to-edge, then translates
    // only the interactive document viewport below.
    canvas.clear(skia_safe::Color4f::new(
        BACKDROP.r, BACKDROP.g, BACKDROP.b, BACKDROP.a,
    ));
    canvas.reset_matrix();
    canvas.scale((session.dpr, session.dpr));

    let origin = session.viewport_origin;
    let zoom = session.zoom;
    let selected = session.selected.clone();
    let logical = session.safe_area_viewport();
    let (insets_left, insets_top) = (session.insets.left, session.insets.top);
    let cull = viewport_cull(origin, zoom, logical);
    // Computed before the field-wise borrow so the paint pass can hand the
    // painter the draft + caret while `scene`/`backend` are live.
    let edit_caret = crate::text::paint_edit_caret(session);

    let Session { scene, backend, .. } = session;
    if insets_left > 0.0 || insets_top > 0.0 {
        canvas.translate((insets_left, insets_top));
    }
    let mut frame = NativeFrameBackend::new(backend, canvas);
    // A panned/zoomed document must not paint back into the platform-owned
    // bands. The full-surface clear above owns those pixels.
    frame.save();
    frame.clip_rect(Rect {
        origin: Point2D::ZERO,
        size: Point2D::new(logical.0, logical.1),
    });
    // Backdrop first (painted before the page), then the page, then the
    // selection stroke on top.
    frame.fill_rect(
        Rect {
            origin: Point2D::new(0.0, 0.0),
            size: Point2D::new(logical.0, logical.1),
        },
        BACKDROP,
    );
    let Some(page) = scene.active_page() else {
        frame.restore();
        return;
    };
    {
        let mut cx = PaintCx {
            backend: &mut frame,
        };
        // While a text edit is active, the painter renders the draft +
        // composition + caret for the edited node.
        canvas_viewport_paint::paint_scene_page_with_options(
            &mut cx, page, origin, zoom, cull, edit_caret,
        );
    }
    if let Some(id) = selected.as_deref() {
        if let Some(node) = find_node(&page.children, id) {
            paint_selection_overlay(&mut frame, node, origin, zoom);
        }
    }
    frame.restore();
}

/// Stroke the selected node's world-space rect.
fn paint_selection_overlay(
    frame: &mut NativeFrameBackend<'_>,
    node: &SceneNode,
    origin: Point2D,
    zoom: f32,
) {
    let world = world_rect(&node.bounds, origin, zoom);
    // Rounded stroke follows the node's corner radius like the editor.
    let radius = node.corner_radius * zoom;
    if radius > 0.5 {
        frame.stroke_round_rect(world, radius, SELECTION_COLOR, SELECTION_STROKE);
    } else {
        frame.stroke_rect(world, SELECTION_COLOR, SELECTION_STROKE);
    }
}

fn world_rect(bounds: &Rect, origin: Point2D, zoom: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            origin.x + bounds.origin.x * zoom,
            origin.y + bounds.origin.y * zoom,
        ),
        size: Point2D::new(bounds.size.x * zoom, bounds.size.y * zoom),
    }
}

/// Recursively find a scene node by id (children are topmost-first).
pub(crate) fn find_node<'a>(children: &'a [SceneNode], id: &str) -> Option<&'a SceneNode> {
    for child in children {
        if child.id == id {
            return Some(child);
        }
        if let Some(hit) = find_node(&child.children, id) {
            return Some(hit);
        }
    }
    None
}

/// Union of a page's top-level node bounds (with the precomputed
/// aggregate cache when present) — the fit-to-view target.
pub(crate) fn page_doc_bounds(page: &ScenePage) -> Rect {
    let mut union: Option<Rect> = None;
    for child in &page.children {
        let bounds = if child.aggregate_bounds_cache != Rect::ZERO {
            child.aggregate_bounds_cache
        } else {
            child.bounds
        };
        union = Some(match union {
            None => bounds,
            Some(acc) => union_rects(&acc, &bounds),
        });
    }
    union.unwrap_or(Rect::ZERO)
}

fn union_rects(a: &Rect, b: &Rect) -> Rect {
    let min_x = a.origin.x.min(b.origin.x);
    let min_y = a.origin.y.min(b.origin.y);
    let max_x = (a.origin.x + a.size.x).max(b.origin.x + b.size.x);
    let max_y = (a.origin.y + a.size.y).max(b.origin.y + b.size.y);
    Rect {
        origin: Point2D::new(min_x, min_y),
        size: Point2D::new(max_x - min_x, max_y - min_y),
    }
}

#[cfg(all(test, feature = "editor"))]
mod editor_viewport_tests {
    use super::*;
    use crate::desc::{Callbacks, CreateOptions};

    const SAMPLE_DOC: &str =
        include_str!("../../op-editor-core/assets/scene_templates/daily-sign-card.op");

    #[test]
    fn keyboard_occlusion_does_not_resize_the_editor_frame() {
        let mut session = Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: 320.0,
            height: 480.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
        })
        .expect("editor session");

        let mut surface = SkiaSurface::new_raster(320, 480);
        // Warm caches so the comparison isolates keyboard state rather than
        // first-paint bookkeeping.
        paint_into_canvas(&mut session, surface.canvas());
        paint_into_canvas(&mut session, surface.canvas());
        let mut before = vec![0_u8; 320 * 480 * 4];
        assert!(surface.read_rgba8(&mut before));

        session.keyboard = 120.0;
        session.sync_editor_keyboard_occlusion();
        paint_into_canvas(&mut session, surface.canvas());
        let mut after = vec![0_u8; 320 * 480 * 4];
        assert!(surface.read_rgba8(&mut after));

        assert_eq!(
            after, before,
            "an unfocused keyboard update must not translate or resize editor chrome"
        );
    }

    #[test]
    fn safe_area_bands_follow_the_active_light_theme() {
        let mut session = Session::new(CreateOptions {
            document: SAMPLE_DOC.to_owned(),
            width: 320.0,
            height: 480.0,
            dpr: 1.0,
            callbacks: Callbacks::default(),
            asset_base: None,
            editor_mode: true,
        })
        .expect("editor session");
        session.insets = crate::viewport::OpInsets {
            top: 24.0,
            right: 10.0,
            bottom: 20.0,
            left: 8.0,
        };
        session
            .editor
            .as_mut()
            .expect("editor host")
            .editor_state_mut()
            .editor_ui
            .theme_mode = op_editor_core::editor_ui_state::ThemeMode::Light;

        let mut surface = SkiaSurface::new_raster(320, 480);
        paint_into_canvas(&mut session, surface.canvas());
        let mut pixels = vec![0_u8; 320 * 480 * 4];
        assert!(surface.read_rgba8(&mut pixels));
        let top_band = (12 * 320 + 160) * 4;
        assert_eq!(
            &pixels[top_band..top_band + 4],
            &[0xef, 0xef, 0xef, 0xff],
            "light chrome must extend its root surface through the safe band"
        );
    }
}

/// Compute a viewport that fits the active page with breathing room.
pub(crate) fn fit_viewport(session: &mut Session) {
    let Some(page) = session.scene.active_page() else {
        return;
    };
    let bounds = page_doc_bounds(page);
    let (w, h) = session.safe_area_viewport();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 || w <= 0.0 || h <= 0.0 {
        session.viewport_origin = Point2D::ZERO;
        session.zoom = 1.0;
        return;
    }
    let zoom = (w / bounds.size.x).min(h / bounds.size.y) * 0.92;
    let zoom = zoom.clamp(crate::input::MIN_ZOOM, crate::input::MAX_ZOOM);
    let origin = Point2D::new(
        (w - bounds.size.x * zoom) * 0.5 - bounds.origin.x * zoom,
        (h - bounds.size.y * zoom) * 0.5 - bounds.origin.y * zoom,
    );
    // Direct assignment: fitting is not a user interaction, so it must
    // not arm the "keep my view on resize" flag.
    session.viewport_origin = origin;
    session.zoom = zoom;
}
