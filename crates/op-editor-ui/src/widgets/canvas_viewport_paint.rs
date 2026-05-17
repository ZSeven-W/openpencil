//! Per-kind node painter for [`super::canvas_viewport::CanvasViewport`].
//!
//! Walks a [`LayoutScene`](crate::layout_scene::LayoutScene)
//! [`SceneNode`] tree and reproduces the canvas pixel-for-pixel:
//! per-kind paint (Frame / Group / Rect / Ellipse / Polygon / Line /
//! Path / Text / `icon_font`), per-node rotation, corner radius,
//! pre-resolved fills / strokes, drop-shadow effects, viewport culling
//! and CJK-aware text wrap.
//!
//! Split out of `canvas_viewport.rs` to keep that file under the
//! 800-line ceiling. The scene's geometry is already layout-resolved
//! and its fills are already `$ref`-resolved, so this painter applies
//! only the viewport transform — no second layout pass, no variable
//! lookup.

use crate::layout_scene::SceneNode;
use crate::layout_scene::{Effect, NodeKind};
use crate::widgets::canvas_viewport::EditCaret;
use crate::widgets::canvas_viewport_overlay::{paint_fill_then_stroke, wrap_text};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};

/// Paint every `Effect::DropShadow` on `node` as a blurred shape
/// behind its fill. The shadow corner radius matches the node
/// kind — `corner_radius` for Frame / Rect, min-half for an
/// ellipse silhouette. Offset + blur scale by `zoom` so the
/// shadow tracks the node across viewport zoom.
fn paint_drop_shadows(cx: &mut PaintCx<'_>, node: &SceneNode, world_rect: Rect, zoom: f32) {
    let radius = if node.kind == NodeKind::Ellipse {
        world_rect.size.x.min(world_rect.size.y) / 2.0
    } else {
        node.corner_radius * zoom
    };
    for effect in &node.effects {
        let Effect::DropShadow(s) = effect;
        let shadow_rect = Rect {
            origin: Point2D::new(
                world_rect.origin.x + s.offset_x * zoom,
                world_rect.origin.y + s.offset_y * zoom,
            ),
            size: world_rect.size,
        };
        cx.backend
            .fill_drop_shadow(shadow_rect, radius, s.blur * zoom, s.color);
    }
}

/// Tessellate an ellipse arc / pie / donut-sector into a closed
/// polygon outline. `start_deg` / `sweep_deg` use the screen
/// convention (0° = +X, positive = clockwise); `inner` is the
/// donut-hole radius as a 0.0..=1.0 fraction.
pub(crate) fn arc_polygon(rect: Rect, start_deg: f32, sweep_deg: f32, inner: f32) -> Vec<Point2D> {
    let cx_pt = rect.origin.x + rect.size.x / 2.0;
    let cy_pt = rect.origin.y + rect.size.y / 2.0;
    let rx = rect.size.x / 2.0;
    let ry = rect.size.y / 2.0;
    // ~1 segment per 4° of sweep, clamped to a sane range.
    let segs = ((sweep_deg.abs() / 4.0).ceil() as usize).clamp(2, 512);
    let point = |frac: f32, scale: f32| -> Point2D {
        let ang = (start_deg + sweep_deg * frac).to_radians();
        Point2D::new(
            cx_pt + rx * scale * ang.cos(),
            cy_pt + ry * scale * ang.sin(),
        )
    };
    let mut poly = Vec::with_capacity(segs * 2 + 2);
    if inner > 0.001 {
        // Annular sector: outer arc start→end, inner arc end→start.
        for i in 0..=segs {
            poly.push(point(i as f32 / segs as f32, 1.0));
        }
        for i in (0..=segs).rev() {
            poly.push(point(i as f32 / segs as f32, inner));
        }
    } else {
        // Pie wedge: centre + outer arc.
        poly.push(Point2D::new(cx_pt, cy_pt));
        for i in 0..=segs {
            poly.push(point(i as f32 / segs as f32, 1.0));
        }
    }
    poly
}

/// Paint an Ellipse node — a full oval when no arc geometry is
/// authored, otherwise a tessellated pie / arc / donut sector.
fn paint_ellipse(cx: &mut PaintCx<'_>, node: &SceneNode, world_rect: Rect, zoom: f32) {
    let inner = node.arc_inner_radius.unwrap_or(0.0).clamp(0.0, 1.0);
    let has_arc = node.arc_start_angle.is_some() || node.arc_sweep_angle.is_some() || inner > 0.001;
    let sweep = node.arc_sweep_angle.unwrap_or(360.0);
    // A full-circle sweep with no donut hole is just a plain oval.
    if !has_arc || (sweep.abs() >= 359.9 && inner <= 0.001) {
        if let Some(fill) = node.fill {
            cx.backend.fill_oval(world_rect, fill);
        }
        if let Some(stroke) = node.stroke {
            cx.backend
                .stroke_oval(world_rect, stroke.color, stroke.width * zoom);
        }
        return;
    }
    let start = node.arc_start_angle.unwrap_or(0.0);
    let poly = arc_polygon(world_rect, start, sweep, inner);
    if let Some(fill) = node.fill {
        cx.backend.fill_polygon(&poly, fill);
    }
    if let Some(stroke) = node.stroke {
        let w = stroke.width * zoom;
        if sweep.abs() >= 359.9 && inner > 0.001 {
            // Full ring — stroke the two concentric ovals so the
            // polygon's radial seam isn't drawn.
            cx.backend.stroke_oval(world_rect, stroke.color, w);
            let iw = world_rect.size.x * inner;
            let ih = world_rect.size.y * inner;
            let inner_rect = Rect {
                origin: Point2D::new(
                    world_rect.origin.x + (world_rect.size.x - iw) / 2.0,
                    world_rect.origin.y + (world_rect.size.y - ih) / 2.0,
                ),
                size: Point2D::new(iw, ih),
            };
            cx.backend.stroke_oval(inner_rect, stroke.color, w);
        } else {
            cx.backend.stroke_polygon(&poly, stroke.color, w);
        }
    }
}

/// One point on the cubic Bezier `p0→p3` (control points `p1`,`p2`).
pub(crate) fn cubic_point(p0: Point2D, p1: Point2D, p2: Point2D, p3: Point2D, t: f32) -> Point2D {
    let u = 1.0 - t;
    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point2D::new(
        w0 * p0.x + w1 * p1.x + w2 * p2.x + w3 * p3.x,
        w0 * p0.y + w1 * p1.y + w2 * p2.y + w3 * p3.y,
    )
}

/// Flatten a Path scene node into a doc-space polyline — cubic
/// segments whose endpoints carry handles are tessellated; a
/// handle-free path falls back to the straight `points` polyline.
pub(crate) fn flatten_path(node: &SceneNode) -> Vec<Point2D> {
    let anchors = &node.path_anchors;
    let has_handle = anchors
        .iter()
        .any(|a| a.handle_in.is_some() || a.handle_out.is_some());
    if anchors.len() < 2 || !has_handle {
        return node.points.clone();
    }
    let mut out = Vec::with_capacity(anchors.len() * 16);
    out.push(anchors[0].pos);
    for pair in anchors.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let (p0, p3) = (a.pos, b.pos);
        let p1 = a.handle_out.unwrap_or(p0);
        let p2 = b.handle_in.unwrap_or(p3);
        if p1 == p0 && p2 == p3 {
            out.push(p3); // straight segment
        } else {
            for i in 1..=16 {
                out.push(cubic_point(p0, p1, p2, p3, i as f32 / 16.0));
            }
        }
    }
    out
}

/// Recursively paint one resolved [`SceneNode`] and its subtree.
///
/// `viewport_origin` is the canvas-rect origin shifted by the
/// viewport pan; `zoom` is the viewport zoom. The scene already
/// carries layout-resolved absolute doc-space bounds, so paint is a
/// straight `doc → world` transform.
pub fn paint_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
) {
    // Hidden nodes (and their subtree) skip canvas paint entirely.
    // Layer panel still shows them, dimmed, so the user can unhide.
    if node.hidden {
        return;
    }
    let world_rect = Rect {
        origin: Point2D::new(
            viewport_origin.x + node.bounds.origin.x * zoom,
            viewport_origin.y + node.bounds.origin.y * zoom,
        ),
        size: Point2D::new(node.bounds.size.x * zoom, node.bounds.size.y * zoom),
    };
    // Viewport culling — bounded leaves skip paint entirely when
    // off-screen. Containers (bounds = ZERO) always recurse.
    if world_rect.size.x > 0.0 && world_rect.size.y > 0.0 && node.children.is_empty() {
        let off = world_rect.origin.x + world_rect.size.x < cull.origin.x
            || world_rect.origin.x > cull.origin.x + cull.size.x
            || world_rect.origin.y + world_rect.size.y < cull.origin.y
            || world_rect.origin.y > cull.origin.y + cull.size.y;
        if off {
            return;
        }
    }

    // Wrap the paint in save/rotate/restore if the node carries a
    // non-zero rotation. Rotation pivots around the node's own
    // bounds centre — for containers, this is the aggregate centre.
    let rotated = node.rotation.abs() > f32::EPSILON;
    if rotated {
        let pivot_doc = node.aggregate_bounds();
        let pivot = Point2D::new(
            viewport_origin.x + (pivot_doc.origin.x + pivot_doc.size.x / 2.0) * zoom,
            viewport_origin.y + (pivot_doc.origin.y + pivot_doc.size.y / 2.0) * zoom,
        );
        cx.backend.save();
        cx.backend.rotate(node.rotation, pivot);
    }

    // Drop shadows paint behind the node's own fill. Only kinds
    // whose silhouette a rounded rect / ellipse can represent
    // faithfully (Frame / Rect / Ellipse) cast one; Polygon / Line
    // / Path shadows are deferred until a shape-mask path exists.
    if !node.effects.is_empty()
        && world_rect.size.x > 0.0
        && world_rect.size.y > 0.0
        && matches!(
            node.kind,
            NodeKind::Frame | NodeKind::Rect | NodeKind::Ellipse
        )
    {
        paint_drop_shadows(cx, node, world_rect, zoom);
    }

    match &node.kind {
        NodeKind::Frame => {
            paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, edit_caret.clone(), cull);
            }
        }
        NodeKind::Other(tag) if tag == "icon_font" => crate::widgets::icons::paint_icon_font_node(
            cx.backend,
            node.text.as_deref().unwrap_or(""),
            world_rect,
            node.fill,
        ),
        NodeKind::Group | NodeKind::Other(_) => {
            for child in &node.children {
                paint_node(cx, child, viewport_origin, zoom, edit_caret.clone(), cull);
            }
        }
        NodeKind::Rect => {
            paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
        }
        NodeKind::Ellipse => {
            paint_ellipse(cx, node, world_rect, zoom);
        }
        NodeKind::Polygon => {
            // Default triangle: top-centre, bottom-left, bottom-right.
            let cx_pt = world_rect.origin.x + world_rect.size.x / 2.0;
            let top_y = world_rect.origin.y;
            let bottom_y = world_rect.origin.y + world_rect.size.y;
            let left_x = world_rect.origin.x;
            let right_x = world_rect.origin.x + world_rect.size.x;
            let pts = [
                Point2D::new(cx_pt, top_y),
                Point2D::new(left_x, bottom_y),
                Point2D::new(right_x, bottom_y),
            ];
            if let Some(fill) = node.fill {
                cx.backend.fill_polygon(&pts, fill);
            }
            if let Some(stroke) = node.stroke {
                cx.backend
                    .stroke_polygon(&pts, stroke.color, stroke.width * zoom);
            }
        }
        NodeKind::Line => {
            // Top-left → bottom-right diagonal across the bounds,
            // stroked at the stroke width (or 1.5 if no stroke).
            let from = Point2D::new(world_rect.origin.x, world_rect.origin.y);
            let to = Point2D::new(
                world_rect.origin.x + world_rect.size.x,
                world_rect.origin.y + world_rect.size.y,
            );
            let (color, width) = match node.stroke {
                Some(s) => (s.color, s.width * zoom),
                None => (
                    node.fill.unwrap_or(crate::Color::BLACK),
                    (1.5_f32).max(zoom),
                ),
            };
            cx.backend.stroke_line(from, to, color, width);
        }
        NodeKind::Path => {
            let (color, width) = match node.stroke {
                Some(s) => (s.color, s.width * zoom),
                None => (
                    node.fill.unwrap_or(crate::Color::BLACK),
                    (1.5_f32).max(zoom),
                ),
            };
            let to_world = |p: Point2D| -> Point2D {
                Point2D::new(
                    viewport_origin.x + p.x * zoom,
                    viewport_origin.y + p.y * zoom,
                )
            };
            // Bezier-aware: when the path carries anchors with control
            // handles, flatten each cubic segment; otherwise fall back
            // to the straight `points` polyline.
            let polyline = flatten_path(node);
            for pair in polyline.windows(2) {
                cx.backend
                    .stroke_line(to_world(pair[0]), to_world(pair[1]), color, width);
            }
        }
        NodeKind::Text => {
            paint_text_node(cx, node, world_rect, zoom, &edit_caret);
        }
    }

    if rotated {
        cx.backend.restore();
    }
}

/// Paint a Text `SceneNode` — wrapped or single-line text plus the
/// edit caret when the node is the one being edited.
fn paint_text_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    edit_caret: &Option<EditCaret>,
) {
    let text = node.text.as_deref().unwrap_or("");
    // Ink colour follows the resolved fill (defaults to near black).
    let ink = node.fill.unwrap_or(crate::Color {
        r: 0.08,
        g: 0.08,
        b: 0.08,
        a: 1.0,
    });
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    // Honour authored font size from the canonical schema; default to
    // 13 px so editor-created text stays uniform. Baseline ≈ 1.08 × size.
    let base_size = if node.font_size > 0.0 {
        node.font_size
    } else {
        13.0
    };
    let font_size = base_size * zoom;
    let baseline_y = world_rect.origin.y + (base_size + 1.0) * zoom;
    if !text.is_empty() {
        let weight = if node.font_weight > 0 {
            node.font_weight
        } else {
            400
        };
        let jc = jian_core::scene::Color::rgba(ch(ink.r), ch(ink.g), ch(ink.b), ch(ink.a));
        let line_h = base_size * 1.35 * zoom;
        let mut ly = baseline_y;
        let lines: Vec<String> = if node.text_wrap {
            wrap_text(cx.backend, text, font_size, world_rect.size.x, weight)
        } else {
            text.split('\n').map(str::to_string).collect()
        };
        for line in lines {
            cx.backend.draw_text(
                &TextLayout::single_run(&line, "system-ui", font_size, jc, Point2D::new(0.0, 0.0))
                    .with_font_weight(weight),
                Point2D::new(world_rect.origin.x, ly),
            );
            ly += line_h;
        }
    }
    // Caret while editing — sits at the end of the text.
    if let Some(c) = edit_caret {
        if c.editing == node.id && jian_core::anim::blink_visible(c.now_ms, c.anchor_ms, 500) {
            let text_w = cx.backend.measure_text(text, font_size);
            let caret = Rect {
                origin: Point2D::new(
                    world_rect.origin.x + text_w,
                    world_rect.origin.y + 2.0 * zoom,
                ),
                size: Point2D::new(1.0_f32.max(zoom), font_size * 1.15),
            };
            cx.backend.fill_rect(caret, ink);
        }
    }
}

#[cfg(test)]
mod arc_tests {
    use super::arc_polygon;
    use crate::Rect;

    #[test]
    fn pie_polygon_starts_at_centre() {
        // 100×100 rect at origin → centre (50, 50).
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 90.0, 0.0);
        assert_eq!(poly[0].x, 50.0);
        assert_eq!(poly[0].y, 50.0);
        // First arc point at 0° = +X edge → (100, 50).
        assert!((poly[1].x - 100.0).abs() < 0.01);
        assert!((poly[1].y - 50.0).abs() < 0.01);
    }

    #[test]
    fn donut_polygon_has_outer_and_inner_rings() {
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 360.0, 0.5);
        // segs for 360° = 90; outer (segs+1) + inner (segs+1) points.
        assert_eq!(poly.len(), 2 * (90 + 1));
        // An inner-ring point sits at half the radius from centre.
        let last = poly[poly.len() - 1];
        let dist = ((last.x - 50.0).powi(2) + (last.y - 50.0).powi(2)).sqrt();
        assert!((dist - 25.0).abs() < 0.5, "inner radius ~25, got {dist}");
    }

    #[test]
    fn quarter_sweep_end_point_at_90_degrees() {
        // start 0°, sweep 90° → last outer point at +Y edge (50, 100).
        let poly = arc_polygon(Rect::xywh(0.0, 0.0, 100.0, 100.0), 0.0, 90.0, 0.0);
        let last = poly[poly.len() - 1];
        assert!((last.x - 50.0).abs() < 0.01);
        assert!((last.y - 100.0).abs() < 0.01);
    }
}

#[cfg(test)]
mod path_tests {
    use super::flatten_path;
    use crate::layout_scene::{NodeKind, SceneAnchor, SceneNode, ScenePointType};
    use crate::{Point2D, Rect};

    fn anchor(x: f32, y: f32, hout: Option<Point2D>) -> SceneAnchor {
        SceneAnchor {
            pos: Point2D::new(x, y),
            handle_in: None,
            handle_out: hout,
            point_type: ScenePointType::Corner,
        }
    }

    #[test]
    fn handle_free_path_falls_back_to_points() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(10.0, 0.0)];
        n.path_anchors = vec![anchor(0.0, 0.0, None), anchor(10.0, 0.0, None)];
        // No handles → straight polyline == points.
        assert_eq!(flatten_path(&n), n.points);
    }

    #[test]
    fn curved_segment_tessellates_into_many_points() {
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.points = vec![Point2D::new(0.0, 0.0), Point2D::new(100.0, 0.0)];
        n.path_anchors = vec![
            anchor(0.0, 0.0, Some(Point2D::new(0.0, 50.0))),
            anchor(100.0, 0.0, None),
        ];
        let poly = flatten_path(&n);
        // 1 start point + 16 tessellation steps for the cubic.
        assert_eq!(poly.len(), 17);
        assert_eq!(poly[0], Point2D::new(0.0, 0.0));
        assert_eq!(poly[poly.len() - 1], Point2D::new(100.0, 0.0));
        // Mid-curve bows toward the +Y handle.
        assert!(poly[8].y > 1.0, "curve bows toward the handle");
    }

    #[test]
    fn bounds_kept_so_helper_is_pure() {
        // flatten_path must not mutate the node.
        let mut n = SceneNode::leaf("p", NodeKind::Path);
        n.bounds = Rect::xywh(1.0, 2.0, 3.0, 4.0);
        let _ = flatten_path(&n);
        assert_eq!(n.bounds, Rect::xywh(1.0, 2.0, 3.0, 4.0));
    }
}
