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

use crate::layout_scene::{regular_polygon_points, SceneGradient, SceneNode};
use crate::layout_scene::{Effect, NodeKind};
use crate::widgets::canvas_viewport::EditCaret;
use crate::widgets::canvas_viewport_image::paint_image_node;
use crate::widgets::canvas_viewport_overlay::paint_fill_then_stroke;
use crate::widgets::canvas_viewport_text::paint_text_node;
use crate::widgets::canvas_viewport_widget::paint_widget_visual;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use std::collections::HashMap;

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
        // Inset shadows are painted inside the silhouette by the
        // per-kind painter, not here — skip them in the outer pass.
        if s.inner {
            continue;
        }
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

/// One flattened segment `a → b` appended onto `out` — a cubic when
/// either endpoint carries a handle, else a straight line.
fn flatten_segment(
    a: &crate::layout_scene::SceneAnchor,
    b: &crate::layout_scene::SceneAnchor,
    out: &mut Vec<Point2D>,
) {
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

/// Flatten a Path scene node into a doc-space polyline — cubic
/// segments whose endpoints carry handles are tessellated; a
/// handle-free path falls back to the straight `points` polyline.
/// A closed path appends the last-anchor → first-anchor segment.
pub(crate) fn flatten_path(node: &SceneNode) -> Vec<Point2D> {
    let anchors = &node.path_anchors;
    let has_handle = anchors
        .iter()
        .any(|a| a.handle_in.is_some() || a.handle_out.is_some());
    if anchors.len() < 2 || !has_handle {
        let mut out = node.points.clone();
        // Closed handle-free path — link the polyline back to its
        // start so the closing edge is drawn.
        if node.path_closed && out.len() > 2 {
            out.push(out[0]);
        }
        return out;
    }
    let mut out = Vec::with_capacity(anchors.len() * 16 + 16);
    out.push(anchors[0].pos);
    for pair in anchors.windows(2) {
        flatten_segment(&pair[0], &pair[1], &mut out);
    }
    if node.path_closed {
        flatten_segment(&anchors[anchors.len() - 1], &anchors[0], &mut out);
    }
    out
}

/// Push a children-clip for a `clipContent` container (root frames
/// included — the scene builder bakes that rule). Mirrors the TS
/// renderer (`document-flattener.ts` clip stack + `node-renderer.ts`
/// `clipRRect`): children clip to the container's bounds with the
/// corner radius clamped to half the height; the container's OWN fill
/// / stroke paint un-clipped before this. Returns whether a
/// `save` was pushed (caller must `restore` after the children).
/// Off-clip children skip via the regular viewport cull anyway — the
/// clip only trims partially-overflowing descendants.
fn push_clip_content(cx: &mut PaintCx<'_>, node: &SceneNode, world_rect: Rect, zoom: f32) -> bool {
    if !node.clip_content
        || node.children.is_empty()
        || world_rect.size.x <= 0.0
        || world_rect.size.y <= 0.0
    {
        return false;
    }
    cx.backend.save();
    // TS flattener: `cr = Math.min(crRaw, nodeH / 2)`.
    let radius = node.corner_radius.min(node.bounds.size.y / 2.0).max(0.0) * zoom;
    if radius > 0.5 {
        cx.backend.clip_round_rect(world_rect, radius);
    } else {
        cx.backend.clip_rect(world_rect);
    }
    true
}

/// Reveal timing for nodes that are being streamed onto the canvas.
#[derive(Clone, Copy)]
pub(crate) struct RevealSchedule<'a> {
    pub(crate) starts: &'a HashMap<String, u64>,
    pub(crate) now_ms: u64,
}

struct PaintNodeOptions<'a> {
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
    reveals: Option<RevealSchedule<'a>>,
}

#[derive(Clone, Copy)]
struct RevealPhase {
    t: f32,
    ease: f32,
}

#[derive(Clone, Copy)]
enum RevealPaintState {
    Idle,
    Pending,
    Active(RevealPhase),
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
    let options = PaintNodeOptions {
        viewport_origin,
        zoom,
        edit_caret,
        cull,
        reveals: None,
    };
    paint_node_inner(cx, node, &options, false);
}

pub(crate) fn paint_node_with_reveals(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
    reveals: RevealSchedule<'_>,
) {
    let options = PaintNodeOptions {
        viewport_origin,
        zoom,
        edit_caret,
        cull,
        reveals: Some(reveals),
    };
    paint_node_inner(cx, node, &options, false);
}

fn paint_node_inner(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    options: &PaintNodeOptions<'_>,
    ancestor_revealing: bool,
) {
    let viewport_origin = options.viewport_origin;
    let zoom = options.zoom;
    let edit_caret = &options.edit_caret;
    let cull = options.cull;
    // Hidden nodes (and their subtree) skip canvas paint entirely.
    // Layer panel still shows them, dimmed, so the user can unhide.
    let reveal_state = options
        .reveals
        .map(|schedule| reveal_paint_state(schedule, &node.id))
        .unwrap_or(RevealPaintState::Idle);
    if node.hidden || matches!(reveal_state, RevealPaintState::Pending) {
        return;
    }
    let own_reveal_phase = match reveal_state {
        RevealPaintState::Active(phase) if !ancestor_revealing => Some(phase),
        _ => None,
    };
    let suppress_descendant_reveals = match reveal_state {
        RevealPaintState::Active(phase) => {
            phase.t < op_editor_core::agent_indicators::REVEAL_CHILD_SUPPRESS_FRACTION
        }
        _ => false,
    };
    let descendant_has_revealing_ancestor = ancestor_revealing || suppress_descendant_reveals;
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

    let reveal_wrapped = own_reveal_phase
        .map(|phase| push_reveal_transform(cx, world_rect, phase))
        .unwrap_or(false);

    // Wrap the paint in save/transform/restore when the node carries
    // a mirror or non-zero rotation. Both pivot around the node's
    // own bounds centre; containers use their aggregate centre.
    let flipped = node.flip_x || node.flip_y;
    let rotated = node.rotation.abs() > f32::EPSILON;
    let transformed = flipped || rotated;
    if transformed {
        let pivot_doc = node.aggregate_bounds();
        let pivot = Point2D::new(
            viewport_origin.x + (pivot_doc.origin.x + pivot_doc.size.x / 2.0) * zoom,
            viewport_origin.y + (pivot_doc.origin.y + pivot_doc.size.y / 2.0) * zoom,
        );
        cx.backend.save();
        if flipped {
            cx.backend.scale(
                Point2D::new(
                    if node.flip_x { -1.0 } else { 1.0 },
                    if node.flip_y { -1.0 } else { 1.0 },
                ),
                pivot,
            );
        }
        if rotated {
            cx.backend.rotate(node.rotation, pivot);
        }
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
            // Image-fill Frames paint the bitmap behind their
            // children; gradient + solid fall back to the shared
            // fill/stroke painter. Without this branch a Frame whose
            // primary fill is `PenFill::Image { url }` only shows the
            // grey placeholder + its children, never the image.
            if let Some(src) = node.image_src.as_deref() {
                paint_image_node(cx, node, world_rect, zoom, src);
            } else {
                paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
            }
            // `tabs` degrades to a `frame` whose children are the tab
            // panels; paint the minimal tab-bar visual over the frame
            // fill, then the children render normally below.
            paint_widget_visual(cx, node, world_rect, zoom);
            let clipped = push_clip_content(cx, node, world_rect, zoom);
            for child in node.children.iter().rev() {
                paint_node_inner(cx, child, options, descendant_has_revealing_ancestor);
            }
            if clipped {
                cx.backend.restore();
            }
        }
        NodeKind::Other(tag) if tag == "icon_font" => crate::widgets::icons::paint_icon_font_node(
            cx.backend,
            node.font_family.as_str(),
            node.text.as_deref().unwrap_or(""),
            world_rect,
            node.fill,
        ),
        NodeKind::Group | NodeKind::Other(_) => {
            // `clipContent` is container-level in the canonical schema
            // (Frame / Group / Rectangle all carry it) — honour it on
            // every recursing container branch, not just Frame.
            let clipped = push_clip_content(cx, node, world_rect, zoom);
            for child in node.children.iter().rev() {
                paint_node_inner(cx, child, options, descendant_has_revealing_ancestor);
            }
            if clipped {
                cx.backend.restore();
            }
        }
        NodeKind::Rect => {
            // Composite widgets that degrade to `rect` (switch /
            // checkbox / slider / progress / radio_group / number_input
            // / text_area) paint their recognizable static visual on the
            // design surface instead of the bare rect.
            if paint_widget_visual(cx, node, world_rect, zoom) {
                // painted
            } else if let Some(src) = node.image_src.as_deref() {
                // Image nodes land as `kind="rect"` (the loader rewrites
                // their variant so non-image paths keep working). When a
                // `src` is carried, paint the bitmap; the grey `fill`
                // remains as the placeholder visible while the decoder
                // is missing the bytes (corrupt URL / unsupported codec).
                paint_image_node(cx, node, world_rect, zoom, src);
            } else {
                paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
            }
        }
        NodeKind::Ellipse => {
            if let Some(src) = node.image_src.as_deref() {
                // Image-fill ellipse: paint the bitmap clipped to the
                // ellipse silhouette via skia's `clip_oval`-style
                // approximation (no native clip_oval on the trait, so
                // fall back to the rect-clip path the painter has).
                paint_image_node(cx, node, world_rect, zoom, src);
                if let Some(stroke) = node.stroke {
                    cx.backend
                        .stroke_oval(world_rect, stroke.color, stroke.width * zoom);
                }
            } else {
                paint_ellipse(cx, node, world_rect, zoom);
            }
        }
        NodeKind::Polygon => {
            let pts = regular_polygon_points(world_rect, node.polygon_sides);
            // Image fills paint the bitmap in the AABB underneath the
            // polygon outline; the polygon silhouette is then drawn
            // by the stroke. A perfect clip-to-polygon path lands when
            // `RenderBackend` grows a polygon-clip primitive.
            if let Some(src) = node.image_src.as_deref() {
                paint_image_node(cx, node, world_rect, zoom, src);
            } else if let Some(fill) = node.fill {
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
            if let Some(d) = node.svg_path.as_deref() {
                paint_svg_path_node(cx, node, world_rect, zoom, d);
                if transformed {
                    cx.backend.restore();
                }
                if reveal_wrapped {
                    cx.backend.restore();
                }
                return;
            }
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
            // A closed path with a fill paints its enclosed area.
            let filled = node.path_closed && node.fill.is_some();
            if filled {
                let world: Vec<Point2D> = polyline.iter().map(|p| to_world(*p)).collect();
                cx.backend.fill_polygon(&world, node.fill.unwrap());
            }
            // Stroke: an explicit stroke always paints; with no
            // stroke, only an UNfilled path strokes (so it stays
            // visible) — a filled path must not draw an implicit
            // outline.
            let stroke = match node.stroke {
                Some(s) => Some((s.color, s.width * zoom)),
                None if !filled => Some((
                    node.fill.unwrap_or(crate::Color::BLACK),
                    (1.5_f32).max(zoom),
                )),
                None => None,
            };
            if let Some((color, width)) = stroke {
                for pair in polyline.windows(2) {
                    cx.backend
                        .stroke_line(to_world(pair[0]), to_world(pair[1]), color, width);
                }
            }
        }
        NodeKind::Text => {
            // text_input / select degrade to a `text` node but carry a
            // widget descriptor — paint the box + value/placeholder +
            // chevron static visual (in world coords) instead of bare
            // text. Painted before the doc-space text transform so its
            // own text runs land at the right spot.
            if paint_widget_visual(cx, node, world_rect, zoom) {
                // painted
            } else {
                let zoom = zoom.max(0.0001);
                cx.backend.save();
                cx.backend.translate(viewport_origin);
                cx.backend.scale(Point2D::new(zoom, zoom), Point2D::ZERO);
                paint_text_node(cx, node, node.bounds, zoom, edit_caret);
                cx.backend.restore();
            }
        }
    }

    if transformed {
        cx.backend.restore();
    }
    if reveal_wrapped {
        cx.backend.restore();
    }
}

fn reveal_paint_state(schedule: RevealSchedule<'_>, node_id: &str) -> RevealPaintState {
    let Some(started_at) = schedule.starts.get(node_id) else {
        return RevealPaintState::Idle;
    };
    if schedule.now_ms < *started_at {
        return RevealPaintState::Pending;
    }
    let elapsed = schedule.now_ms.saturating_sub(*started_at);
    if elapsed > op_editor_core::agent_indicators::REVEAL_DURATION_MS {
        return RevealPaintState::Idle;
    }
    let t = (elapsed as f32 / op_editor_core::agent_indicators::REVEAL_DURATION_MS as f32)
        .clamp(0.0, 1.0);
    RevealPaintState::Active(RevealPhase {
        t,
        ease: ease_in_out_sine(t),
    })
}

fn push_reveal_transform(cx: &mut PaintCx<'_>, rect: Rect, phase: RevealPhase) -> bool {
    if rect.size.x <= 0.0 || rect.size.y <= 0.0 {
        return false;
    }
    let settle = 1.0 - phase.ease;
    let lift = 6.5 * settle * (1.0 - phase.t * 0.18);
    let scale = 0.982 + 0.018 * phase.ease;
    let pivot = Point2D::new(
        rect.origin.x + rect.size.x / 2.0,
        rect.origin.y + rect.size.y / 2.0,
    );
    cx.backend.save();
    cx.backend.translate(Point2D::new(0.0, lift));
    cx.backend.scale(Point2D::new(scale, scale), pivot);
    true
}

fn ease_in_out_sine(t: f32) -> f32 {
    -(std::f32::consts::PI * t).cos() / 2.0 + 0.5
}

pub(crate) fn paint_svg_path_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    world_rect: Rect,
    zoom: f32,
    d: &str,
) {
    // Gradient-filled paths paint through the dedicated gradient
    // method (real shader on native, solid first-stop fallback on
    // backends without one); solid / image fills fall back to the
    // node's resolved `fill` colour.
    match node.gradient.as_ref() {
        Some(SceneGradient::Linear {
            angle_deg,
            opacity,
            stops,
        }) => {
            let flat: Vec<(f32, crate::Color)> =
                stops.iter().map(|s| (s.offset, s.color)).collect();
            cx.backend
                .fill_svg_path_in_rect_linear_gradient(d, world_rect, &flat, *angle_deg, *opacity);
        }
        Some(SceneGradient::Radial {
            cx: gx,
            cy,
            radius,
            opacity,
            stops,
        }) => {
            let flat: Vec<(f32, crate::Color)> =
                stops.iter().map(|s| (s.offset, s.color)).collect();
            cx.backend.fill_svg_path_in_rect_radial_gradient(
                d, world_rect, &flat, *gx, *cy, *radius, *opacity,
            );
        }
        None => {
            if let Some(fill) = node.fill {
                cx.backend.fill_svg_path_in_rect(d, world_rect, fill);
            }
        }
    }
    // Inset shadows paint over the fill, clipped to the path
    // silhouette. Outer shadows on paths stay deferred (no shape-mask
    // drop-shadow path for arbitrary vectors yet).
    for effect in &node.effects {
        let Effect::DropShadow(s) = effect;
        if s.inner {
            cx.backend.fill_inner_shadow_svg_path(
                d,
                world_rect,
                s.offset_x * zoom,
                s.offset_y * zoom,
                s.blur * zoom,
                s.color,
            );
        }
    }
    if let Some(stroke) = node.stroke {
        cx.backend
            .stroke_svg_path_in_rect(d, world_rect, stroke.color, stroke.width * zoom);
    }
}

#[cfg(test)]
#[path = "canvas_viewport_paint_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "canvas_viewport_reveal_tests.rs"]
mod reveal_tests;
