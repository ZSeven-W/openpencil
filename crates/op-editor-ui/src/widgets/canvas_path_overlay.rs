//! Pen-tool authoring preview + path-editor overlays.
//!
//! Ports `apps/web/src/canvas/skia/skia-overlays.ts`:
//! - `drawPenPreview` (`:483-521`) — the constructed segments in
//!   selection blue, the dashed rubber band to the cursor (hidden
//!   while a handle drag is in flight), and the anchor/handle dots
//!   with the first anchor enlarged (close-path affordance).
//! - `drawPathEditor` (`:523-547`) — the half-alpha path trace plus
//!   anchor/handle dots for a single selected Path under the Select
//!   tool (TS paints it for any tool; interaction is Select-only).
//!
//! The Pen-tool edit mode keeps the pre-existing Rust "ghost handle"
//! painting (unset handles paint as grabbable ghost dots — a
//! deliberate superset of TS, which can only mint handles during
//! placement or via the anchor context menu).

use crate::layout_scene::{LayoutScene, SceneAnchor, SceneNode};
use crate::theme::Theme;
use crate::widgets::canvas_viewport::path_handle_positions;
use crate::widgets::canvas_viewport_paint::cubic_point;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::Viewport;

/// TS `SELECTION_BLUE` (`pen-core/constants.ts:11` — `#0d99ff`).
const SELECTION_BLUE: Color = Color {
    r: 13.0 / 255.0,
    g: 153.0 / 255.0,
    b: 255.0 / 255.0,
    a: 1.0,
};
/// TS `PEN_ANCHOR_FILL` — `#ffffff`.
const PEN_ANCHOR_FILL: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
/// TS `PEN_HANDLE_LINE_STROKE` — `#888888`.
const PEN_HANDLE_LINE: Color = Color {
    r: 0x88 as f32 / 255.0,
    g: 0x88 as f32 / 255.0,
    b: 0x88 as f32 / 255.0,
    a: 1.0,
};
/// TS `PEN_RUBBER_BAND_STROKE` — `rgba(13, 153, 255, 0.5)`.
const PEN_RUBBER_BAND: Color = Color {
    a: 0.5,
    ..SELECTION_BLUE
};
/// TS `PEN_ANCHOR_RADIUS` / `PEN_ANCHOR_FIRST_RADIUS` /
/// `PEN_HANDLE_DOT_RADIUS` / `PEN_RUBBER_BAND_DASH[0]` (screen px).
const ANCHOR_RADIUS: f32 = 4.0;
const ANCHOR_FIRST_RADIUS: f32 = 5.0;
const HANDLE_DOT_RADIUS: f32 = 3.0;
const RUBBER_BAND_DASH: f32 = 4.0;

/// Single entry point — gates the three overlay flavors:
/// 1. Pen authoring preview while a pen session is in flight.
/// 2. Ghost-handle edit overlay (Pen tool + single selected Path).
/// 3. TS `drawPathEditor` (any other tool + single selected Path).
#[allow(clippy::too_many_arguments)]
pub(super) fn paint_path_overlays(
    cx: &mut PaintCx<'_>,
    scene: &LayoutScene,
    theme: &Theme,
    tool: op_editor_core::Tool,
    pen_in_progress: Option<&str>,
    pen_cursor_doc: Option<Point2D>,
    pen_dragging_handle: bool,
    selected: &str,
    selected_count: usize,
    canvas_rect: Rect,
    viewport: &Viewport,
) {
    let Some(page) = scene.active_page() else {
        return;
    };
    if let Some(pen_id) = pen_in_progress {
        if let Some(node) = page.find(pen_id) {
            paint_pen_preview(
                cx,
                node,
                pen_cursor_doc,
                pen_dragging_handle,
                canvas_rect,
                viewport,
            );
        }
        return;
    }
    if selected_count != 1 {
        return;
    }
    let Some(node) = page.find(selected) else {
        return;
    };
    if !matches!(node.kind, crate::layout_scene::NodeKind::Path) || node.hidden {
        return;
    }
    with_node_rotation(cx, node, canvas_rect, viewport, |cx| {
        if matches!(tool, op_editor_core::Tool::Pen) {
            paint_ghost_edit_handles(cx, node, theme, canvas_rect, viewport);
        } else {
            paint_path_editor(cx, node, canvas_rect, viewport);
        }
    });
}

/// Wrap `f` in a save/rotate/restore matching the node's rotation so
/// the overlay sits on the rotated path (handle coords are stored in
/// the node's unrotated local frame).
fn with_node_rotation(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    canvas_rect: Rect,
    viewport: &Viewport,
    f: impl FnOnce(&mut PaintCx<'_>),
) {
    let rotated = node.rotation.abs() > f32::EPSILON;
    if rotated {
        let b = node.aggregate_bounds();
        let pivot = to_screen_point(
            Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0),
            canvas_rect,
            viewport,
        );
        cx.backend.save();
        cx.backend.rotate(node.rotation, pivot);
    }
    f(cx);
    if rotated {
        cx.backend.restore();
    }
}

fn to_screen_point(p: Point2D, canvas_rect: Rect, viewport: &Viewport) -> Point2D {
    Point2D::new(
        canvas_rect.origin.x + viewport.pan_x + p.x * viewport.zoom,
        canvas_rect.origin.y + viewport.pan_y + p.y * viewport.zoom,
    )
}

/// Fill + stroke a small circle — TS draws anchor/handle dots as two
/// `drawCircle` calls (fill then stroke).
fn dot(cx: &mut PaintCx<'_>, center: Point2D, r: f32, fill: Color, line: Color, line_w: f32) {
    let b = Rect {
        origin: Point2D::new(center.x - r, center.y - r),
        size: Point2D::new(r * 2.0, r * 2.0),
    };
    cx.backend.fill_oval(b, fill);
    cx.backend.stroke_oval(b, line, line_w);
}

/// Hand-segmented dashed line — the `RenderBackend` has no dash
/// effect, so the TS `PathEffect.MakeDash([4, 4])` is emulated with
/// 4 px screen-space on/off segments.
fn dashed_line(cx: &mut PaintCx<'_>, from: Point2D, to: Point2D, color: Color, width: f32) {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let len = dx.hypot(dy);
    if len <= f32::EPSILON {
        return;
    }
    let (ux, uy) = (dx / len, dy / len);
    let mut t = 0.0;
    while t < len {
        let end = (t + RUBBER_BAND_DASH).min(len);
        cx.backend.stroke_line(
            Point2D::new(from.x + ux * t, from.y + uy * t),
            Point2D::new(from.x + ux * end, from.y + uy * end),
            color,
            width,
        );
        t += RUBBER_BAND_DASH * 2.0;
    }
}

/// Flatten the anchor run into screen-space points — cubic segments
/// (either endpoint carries a handle) tessellate at 16 steps, exactly
/// like the canvas path painter, so the preview hugs the final paint.
fn flatten_to_screen(
    anchors: &[SceneAnchor],
    closed: bool,
    canvas_rect: Rect,
    viewport: &Viewport,
) -> Vec<Point2D> {
    let mut out: Vec<Point2D> = Vec::with_capacity(anchors.len() * 16 + 1);
    if anchors.is_empty() {
        return out;
    }
    let ts = |p: Point2D| to_screen_point(p, canvas_rect, viewport);
    out.push(ts(anchors[0].pos));
    let seg = |a: &SceneAnchor, b: &SceneAnchor, out: &mut Vec<Point2D>| {
        let (p0, p3) = (a.pos, b.pos);
        let p1 = a.handle_out.unwrap_or(p0);
        let p2 = b.handle_in.unwrap_or(p3);
        if p1 == p0 && p2 == p3 {
            out.push(ts(p3));
        } else {
            for i in 1..=16 {
                out.push(ts(cubic_point(p0, p1, p2, p3, i as f32 / 16.0)));
            }
        }
    };
    for pair in anchors.windows(2) {
        seg(&pair[0], &pair[1], &mut out);
    }
    if closed && anchors.len() > 1 {
        seg(&anchors[anchors.len() - 1], &anchors[0], &mut out);
    }
    out
}

fn stroke_polyline(cx: &mut PaintCx<'_>, pts: &[Point2D], color: Color, width: f32) {
    for pair in pts.windows(2) {
        cx.backend.stroke_line(pair[0], pair[1], color, width);
    }
}

/// TS `drawAnchorHandles` (`skia-overlays.ts:408-478`) — anchor dots
/// first (first anchor enlarged when `highlight_first`), then handle
/// lines + dots for SET handles painted on top.
fn draw_anchor_handles(
    cx: &mut PaintCx<'_>,
    anchors: &[SceneAnchor],
    canvas_rect: Rect,
    viewport: &Viewport,
    highlight_first: bool,
) {
    let ts = |p: Point2D| to_screen_point(p, canvas_rect, viewport);
    for (i, a) in anchors.iter().enumerate() {
        let r = if highlight_first && i == 0 {
            ANCHOR_FIRST_RADIUS
        } else {
            ANCHOR_RADIUS
        };
        dot(cx, ts(a.pos), r, PEN_ANCHOR_FILL, SELECTION_BLUE, 1.5);
    }
    for a in anchors {
        let center = ts(a.pos);
        for handle in [a.handle_in, a.handle_out].into_iter().flatten() {
            let hs = ts(handle);
            cx.backend.stroke_line(center, hs, PEN_HANDLE_LINE, 1.0);
            dot(
                cx,
                hs,
                HANDLE_DOT_RADIUS,
                SELECTION_BLUE,
                PEN_ANCHOR_FILL,
                1.0,
            );
        }
    }
}

/// TS `drawPenPreview` — committed segments in selection blue, the
/// dashed rubber band from the last anchor to the cursor (suppressed
/// while the press-drag is minting handles), then anchors + handles
/// with the first anchor enlarged.
fn paint_pen_preview(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    cursor_doc: Option<Point2D>,
    dragging_handle: bool,
    canvas_rect: Rect,
    viewport: &Viewport,
) {
    let anchors = &node.path_anchors;
    if anchors.is_empty() {
        return;
    }
    if anchors.len() > 1 {
        let pts = flatten_to_screen(anchors, false, canvas_rect, viewport);
        stroke_polyline(cx, &pts, SELECTION_BLUE, 1.5);
    }
    if let Some(cursor) = cursor_doc {
        if !dragging_handle {
            let last = anchors[anchors.len() - 1].pos;
            dashed_line(
                cx,
                to_screen_point(last, canvas_rect, viewport),
                to_screen_point(cursor, canvas_rect, viewport),
                PEN_RUBBER_BAND,
                1.0,
            );
        }
    }
    draw_anchor_handles(cx, anchors, canvas_rect, viewport, true);
}

/// TS `drawPathEditor` — the path trace at half alpha plus
/// anchor/handle dots (no first-anchor highlight). Paths without
/// resolved anchors fall back to plain dots over `points` (Rust
/// pragmatic extra; TS bails without parseable anchors).
fn paint_path_editor(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    canvas_rect: Rect,
    viewport: &Viewport,
) {
    let anchors = &node.path_anchors;
    if anchors.is_empty() {
        for p in &node.points {
            dot(
                cx,
                to_screen_point(*p, canvas_rect, viewport),
                ANCHOR_RADIUS,
                PEN_ANCHOR_FILL,
                SELECTION_BLUE,
                1.5,
            );
        }
        return;
    }
    let trace = Color {
        a: SELECTION_BLUE.a * 0.5,
        ..SELECTION_BLUE
    };
    let pts = flatten_to_screen(anchors, node.path_closed, canvas_rect, viewport);
    stroke_polyline(cx, &pts, trace, 1.5);
    draw_anchor_handles(cx, anchors, canvas_rect, viewport, false);
}

/// Pre-existing Rust Pen-tool edit overlay — anchor dots plus BOTH
/// bezier handles per anchor, painting a faint "ghost" dot when a
/// handle is unset (draggable to create it). Theme-tinted (not the TS
/// constants) to keep the established Pen-edit look.
fn paint_ghost_edit_handles(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    theme: &Theme,
    canvas_rect: Rect,
    viewport: &Viewport,
) {
    let zoom = viewport.zoom;
    let ts = |p: Point2D| to_screen_point(p, canvas_rect, viewport);
    let ghost = Color {
        a: theme.primary.a * 0.4,
        ..theme.primary
    };
    for anchor in &node.path_anchors {
        let center = ts(anchor.pos);
        let (hin, hout) = path_handle_positions(anchor, zoom);
        for (pos, is_set) in [
            (hin, anchor.handle_in.is_some()),
            (hout, anchor.handle_out.is_some()),
        ] {
            let hs = ts(pos);
            let tint = if is_set { theme.primary } else { ghost };
            cx.backend.stroke_line(center, hs, tint, 1.0);
            dot(cx, hs, 3.0, theme.background, tint, 1.5);
        }
        // Anchor dot painted last so it sits on top.
        dot(cx, center, 4.0, theme.background, theme.primary, 1.5);
    }
    // Paths with no anchor data still show plain dots.
    if node.path_anchors.is_empty() {
        for p in &node.points {
            dot(cx, ts(*p), 4.0, theme.background, theme.primary, 1.5);
        }
    }
}
