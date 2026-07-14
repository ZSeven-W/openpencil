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
use crate::widgets::canvas_viewport_overlay::{align_stroke_rect, paint_fill_then_stroke};
use crate::widgets::canvas_viewport_text::paint_text_node;
use crate::widgets::canvas_viewport_widget::paint_widget_visual;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use jian_scene::path_geometry::flatten_path_points;
use std::collections::{HashMap, HashSet};

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
        let Effect::DropShadow(s) = effect else {
            continue;
        };
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
            let w = stroke.width * zoom;
            let (rect, _) = align_stroke_rect(world_rect, 0.0, w, stroke.align);
            cx.backend.stroke_oval(rect, stroke.color, w);
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

const STACK_WORLD_PATH_POINTS: usize = 64;

// The large stack variant is intentional: hot path-overlay painting
// avoids heap allocation for the common small-polyline case.
#[allow(clippy::large_enum_variant)]
pub(crate) enum WorldPathPoints {
    Stack {
        points: [Point2D; STACK_WORLD_PATH_POINTS],
        len: usize,
    },
    Owned(Vec<Point2D>),
}

impl WorldPathPoints {
    pub(crate) fn as_slice(&self) -> &[Point2D] {
        match self {
            Self::Stack { points, len } => &points[..*len],
            Self::Owned(points) => points.as_slice(),
        }
    }
}

fn doc_to_world_point(p: Point2D, viewport_origin: Point2D, zoom: f32) -> Point2D {
    Point2D::new(
        viewport_origin.x + p.x * zoom,
        viewport_origin.y + p.y * zoom,
    )
}

pub(crate) fn world_path_points(
    points: &[Point2D],
    viewport_origin: Point2D,
    zoom: f32,
) -> WorldPathPoints {
    if points.len() <= STACK_WORLD_PATH_POINTS {
        let mut stack = [Point2D::ZERO; STACK_WORLD_PATH_POINTS];
        for (idx, point) in points.iter().copied().enumerate() {
            stack[idx] = doc_to_world_point(point, viewport_origin, zoom);
        }
        return WorldPathPoints::Stack {
            points: stack,
            len: points.len(),
        };
    }
    WorldPathPoints::Owned(
        points
            .iter()
            .copied()
            .map(|p| doc_to_world_point(p, viewport_origin, zoom))
            .collect(),
    )
}

/// Flatten a Path scene node into a doc-space polyline — cubic
/// segments whose endpoints carry handles are tessellated; a
/// handle-free path falls back to the straight `points` polyline.
/// A closed path appends the last-anchor → first-anchor segment.
#[cfg(test)]
pub(crate) fn flatten_path(node: &SceneNode) -> Vec<Point2D> {
    use jian_scene::path_geometry::PathPoints;
    match flatten_path_points(node) {
        PathPoints::Borrowed(points) => points.to_vec(),
        PathPoints::Owned(points) => points,
    }
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
pub struct RevealSchedule<'a> {
    pub(crate) starts: &'a HashMap<String, u64>,
    pub(crate) now_ms: u64,
}

/// Scale-in pop window right after a node's reveal starts — the Stitch
/// placement "pop" that replaced the old dashed border fade.
pub(crate) const REVEAL_POP_MS: u64 = 180;

/// Wireframe-ghost window BEFORE the pop: a revealing node first paints as
/// a blue outlined box at its own rect (Pencil parity — its streamed
/// elements materialize as periwinkle wireframes sized like the coming
/// content, then resolve), and only then pops in for real.
pub(crate) const REVEAL_WIREFRAME_MS: u64 = 260;

/// Placement pop: 0.85 → ~1.02 overshoot → 1.0 across [`REVEAL_POP_MS`]
/// (ease-out-back). `None` once the pop has settled.
pub(crate) fn reveal_pop_scale(elapsed_ms: u64) -> Option<f32> {
    if elapsed_ms >= REVEAL_POP_MS {
        return None;
    }
    let t = elapsed_ms as f32 / REVEAL_POP_MS as f32;
    let s = 1.70158f32;
    let u = t - 1.0;
    let back = u * u * ((s + 1.0) * u + s) + 1.0;
    Some(0.85 + 0.15 * back)
}

struct PaintNodeOptions<'a, 'generation> {
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
    reveals: Option<RevealSchedule<'a>>,
    hovered: Option<&'a str>,
    selected: Option<&'a str>,
    pen: Option<&'a str>,
    hidden: Option<&'a str>,
    now_ms: u64,
    generating_descendant_ids: Option<&'generation HashSet<String>>,
    generation_accent: Option<Color>,
    /// Queued empty shells: the skeleton shows, but as a quiet wireframe —
    /// only the on-deck shell gets the active radar (see
    /// `canvas_generation_scan::GenerationPaintSets::queued`).
    queued_shell_ids: Option<&'generation HashSet<String>>,
}

use super::canvas_overlay_transform::OverlayTransform;

#[derive(Default)]
pub struct PaintNodeHits<'a> {
    pub(crate) hover_rect: Option<Rect>,
    /// Root→node transform chain active where the hovered node paints;
    /// empty when `hover_rect` is `None` or the chain is identity.
    pub(crate) hover_transforms: Vec<OverlayTransform>,
    /// Direct visible children of the hovered focus node. Each child
    /// keeps the transform chain active at its own paint site so the
    /// dashed hierarchy hint follows rotated/flipped descendants.
    pub(crate) hover_child_rects: Vec<(Rect, Vec<OverlayTransform>)>,
    pub(crate) selected_node: Option<&'a SceneNode>,
    pub(crate) selected_transforms: Vec<OverlayTransform>,
    pub(crate) pen_node: Option<&'a SceneNode>,
}

impl<'a> PaintNodeHits<'a> {
    fn for_node(
        node: &'a SceneNode,
        options: &PaintNodeOptions<'_, '_>,
        transforms: &[OverlayTransform],
        parent_hovered: bool,
    ) -> Self {
        let is_hovered = options.hovered == Some(node.id.as_str());
        let outline_rect = (is_hovered || parent_hovered)
            .then(|| node_outline_rect(node, options))
            .flatten();
        let hover_rect = is_hovered.then_some(outline_rect).flatten();
        let selected_node = (options.selected == Some(node.id.as_str())).then_some(node);
        Self {
            hover_transforms: if hover_rect.is_some() {
                transforms.to_vec()
            } else {
                Vec::new()
            },
            hover_rect,
            hover_child_rects: if parent_hovered {
                outline_rect
                    .map(|rect| vec![(rect, transforms.to_vec())])
                    .unwrap_or_default()
            } else {
                Vec::new()
            },
            selected_transforms: if selected_node.is_some() {
                transforms.to_vec()
            } else {
                Vec::new()
            },
            selected_node,
            pen_node: (options.pen == Some(node.id.as_str())).then_some(node),
        }
    }

    pub(crate) fn merge_missing(&mut self, child: Self) {
        if self.hover_rect.is_none() {
            self.hover_rect = child.hover_rect;
            self.hover_transforms = child.hover_transforms;
        }
        self.hover_child_rects.extend(child.hover_child_rects);
        if self.selected_node.is_none() {
            self.selected_node = child.selected_node;
            self.selected_transforms = child.selected_transforms;
        }
        if self.pen_node.is_none() {
            self.pen_node = child.pen_node;
        }
    }
}

#[derive(Clone, Copy)]
enum RevealPaintState {
    Idle,
    Pending,
    Active { elapsed_ms: u64 },
}

/// Off-canvas public entry for painters outside this crate (raster/PDF
/// export, `debug_screenshot` in `op-host-desktop`). Paints the base
/// scene only — no editor overlays (reveal animation, hover outline,
/// selection highlight, pen preview) and no caret — forwarding to
/// [`paint_node_with_options`]. Keeps the cross-crate surface to types
/// that are already `pub` (`PaintCx` / `SceneNode` / `Point2D` / `Rect`)
/// so the overlay-only helper types stay crate-private.
pub fn paint_node(
    cx: &mut PaintCx<'_>,
    node: &SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    cull: Rect,
) {
    let _ = paint_node_with_options(
        cx,
        node,
        viewport_origin,
        zoom,
        None,
        cull,
        None,
        None,
        None,
        None,
    );
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_node_with_options<'a>(
    cx: &mut PaintCx<'_>,
    node: &'a SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
    reveals: Option<RevealSchedule<'a>>,
    hovered: Option<&'a str>,
    selected: Option<&'a str>,
    pen: Option<&'a str>,
) -> PaintNodeHits<'a> {
    paint_node_with_options_hiding(
        cx,
        node,
        viewport_origin,
        zoom,
        edit_caret,
        cull,
        reveals,
        hovered,
        selected,
        pen,
        None,
        0,
        None,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_node_with_options_hiding<'a>(
    cx: &mut PaintCx<'_>,
    node: &'a SceneNode,
    viewport_origin: Point2D,
    zoom: f32,
    edit_caret: Option<EditCaret>,
    cull: Rect,
    reveals: Option<RevealSchedule<'a>>,
    hovered: Option<&'a str>,
    selected: Option<&'a str>,
    pen: Option<&'a str>,
    hidden: Option<&'a str>,
    now_ms: u64,
    generating_descendant_ids: Option<&HashSet<String>>,
    generation_accent: Option<Color>,
    queued_shell_ids: Option<&HashSet<String>>,
) -> PaintNodeHits<'a> {
    let options = PaintNodeOptions {
        viewport_origin,
        zoom,
        edit_caret,
        cull,
        reveals,
        hovered,
        selected,
        pen,
        hidden,
        now_ms,
        generating_descendant_ids,
        generation_accent,
        queued_shell_ids,
    };
    paint_node_inner(cx, node, &options, &mut Vec::new(), false)
}

/// Paint a resolved scene page's node tree with the editor viewport
/// transform applied, WITHOUT any editor chrome (no selection outline /
/// handles / hover / grid / reveal animation / text-edit caret).
///
/// The Canvas Preview (Play) path uses this to render the live document
/// through the SAME mature painter the design canvas uses — so preview
/// is pixel-identical to the design surface (root offsets, images,
/// gradients, shadows, real text metrics) instead of jian's separate
/// MVP scene walker. The host (`op-host-native::preview`) overlays live
/// widget runtime state into the scene before calling this, and paints
/// its own focus caret on top.
///
/// `viewport_origin` is `canvas_rect.origin + (pan_x, pan_y)`; `cull`
/// is the canvas rect grown by the standard margin. Children paint
/// back-to-front (`.rev()`), matching [`super::canvas_viewport::CanvasViewport`]'s
/// own walk so z-order is identical.
pub fn paint_scene_page(
    cx: &mut PaintCx<'_>,
    page: &crate::layout_scene::ScenePage,
    viewport_origin: Point2D,
    zoom: f32,
    cull: Rect,
) {
    for child in page.children.iter().rev() {
        paint_node_with_options(
            cx,
            child,
            viewport_origin,
            zoom,
            None,
            cull,
            None,
            None,
            None,
            None,
        );
    }
}

fn paint_node_inner<'a>(
    cx: &mut PaintCx<'_>,
    node: &'a SceneNode,
    options: &PaintNodeOptions<'_, '_>,
    transforms: &mut Vec<OverlayTransform>,
    parent_hovered: bool,
) -> PaintNodeHits<'a> {
    if options.hidden == Some(node.id.as_str()) {
        return PaintNodeHits::default();
    }
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
        return PaintNodeHits::default();
    }
    let world_rect = Rect {
        origin: Point2D::new(
            viewport_origin.x + node.bounds.origin.x * zoom,
            viewport_origin.y + node.bounds.origin.y * zoom,
        ),
        size: Point2D::new(node.bounds.size.x * zoom, node.bounds.size.y * zoom),
    };
    // Wireframe ghost: the first beat of a reveal paints the node as a blue
    // outline box (content and children withheld) — the Pencil-style
    // materialization: ghost box → real element.
    if let RevealPaintState::Active { elapsed_ms } = reveal_state {
        if elapsed_ms < REVEAL_WIREFRAME_MS && world_rect.size.x > 0.0 && world_rect.size.y > 0.0 {
            let blue = super::canvas_generation_scan::SKELETON_BLUE;
            let t = elapsed_ms as f32 / REVEAL_WIREFRAME_MS as f32;
            // Brighten in fast, hold; the pop right after replaces it.
            let ramp = (t * 3.0).clamp(0.3, 1.0);
            let radius = node.corner_radius * zoom;
            cx.backend
                .fill_rect(world_rect, blue.with_alpha(0.08 * ramp));
            if radius > 0.5 {
                cx.backend
                    .stroke_round_rect(world_rect, radius, blue.with_alpha(0.8 * ramp), 1.0);
            } else {
                cx.backend
                    .stroke_rect(world_rect, blue.with_alpha(0.8 * ramp), 1.0);
            }
            return PaintNodeHits::default();
        }
    }
    let pop_scale = match reveal_state {
        RevealPaintState::Active { elapsed_ms } => {
            reveal_pop_scale(elapsed_ms.saturating_sub(REVEAL_WIREFRAME_MS))
        }
        _ => None,
    };
    // Viewport culling — bounded leaves skip paint entirely when
    // off-screen. Containers (bounds = ZERO) always recurse.
    if world_rect.size.x > 0.0 && world_rect.size.y > 0.0 && node.children.is_empty() {
        let off = world_rect.origin.x + world_rect.size.x < cull.origin.x
            || world_rect.origin.x > cull.origin.x + cull.size.x
            || world_rect.origin.y + world_rect.size.y < cull.origin.y
            || world_rect.origin.y > cull.origin.y + cull.size.y;
        if off {
            return PaintNodeHits::default();
        }
    }
    // Wrap the paint in save/transform/restore when the node carries
    // a mirror, a non-zero rotation, or an in-flight reveal pop. All
    // pivot around the node's own bounds centre; containers use their
    // aggregate centre.
    let flipped = node.flip_x || node.flip_y;
    let rotated = node.rotation.abs() > f32::EPSILON;
    let transformed = flipped || rotated || pop_scale.is_some();
    // Flip/rotation (not the transient reveal pop) also joins the
    // overlay transform chain so the hover outline replays it.
    let overlay_transformed = flipped || rotated;
    if transformed {
        let pivot_doc = node.aggregate_bounds();
        let pivot = Point2D::new(
            viewport_origin.x + (pivot_doc.origin.x + pivot_doc.size.x / 2.0) * zoom,
            viewport_origin.y + (pivot_doc.origin.y + pivot_doc.size.y / 2.0) * zoom,
        );
        cx.backend.save();
        if let Some(pop) = pop_scale {
            cx.backend.scale(Point2D::new(pop, pop), pivot);
        }
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
        if overlay_transformed {
            transforms.push(OverlayTransform {
                rotation: node.rotation,
                flip_x: node.flip_x,
                flip_y: node.flip_y,
                pivot,
            });
        }
    }
    let is_hovered = options.hovered == Some(node.id.as_str());
    let mut hits = PaintNodeHits::for_node(node, options, transforms, parent_hovered);

    // Gaussian layer blur (Figma "Layer blur"): capture the node's
    // whole rendered output — shadows, fill, stroke, children — into
    // an offscreen layer and blur it on the matching `restore`. The
    // CSS radius → Skia sigma conversion is `radius / 2`, scaled by
    // the viewport zoom. Popped at every return path below alongside
    // the transform save.
    let blur_sigma = node.effects.iter().find_map(|e| match e {
        Effect::Blur(b) if b.radius > 0.0 => Some(b.radius * 0.5 * zoom),
        _ => None,
    });
    if let Some(sigma) = blur_sigma {
        cx.backend.push_blur_layer(sigma);
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
            if let Some(accent) = options.generation_accent {
                let visually_empty = super::canvas_generation_scan::is_placeholder_section(node)
                    || options.reveals.is_some_and(|schedule| {
                        super::canvas_generation_scan::is_pending_filled_section(
                            node,
                            schedule.starts,
                            schedule.now_ms,
                        )
                    });
                if visually_empty {
                    let on_deck = options
                        .generating_descendant_ids
                        .is_some_and(|ids| ids.contains(&node.id));
                    let queued = options
                        .queued_shell_ids
                        .is_some_and(|ids| ids.contains(&node.id));
                    if on_deck {
                        super::canvas_generation_scan::paint_generation_scan(
                            cx,
                            node,
                            world_rect,
                            zoom,
                            options.now_ms,
                            accent,
                        );
                    } else if queued {
                        super::canvas_generation_scan::paint_queued_skeleton(
                            cx, node, world_rect, zoom, accent,
                        );
                    }
                }
            }
            let clipped = push_clip_content(cx, node, world_rect, zoom);
            for child in node.children.iter().rev() {
                let child_hover = paint_node_inner(cx, child, options, transforms, is_hovered);
                hits.merge_missing(child_hover);
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
                let child_hover = paint_node_inner(cx, child, options, transforms, is_hovered);
                hits.merge_missing(child_hover);
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
            // A `rectangle` is a CONTAINER in the canonical schema (it
            // carries `clipContent` like Frame / Group), so models nest
            // content inside one — e.g. an image-area rectangle wrapping
            // the destination photo. Recurse into its children after its
            // own fill, honouring clip; without this every child of a
            // rectangle (nested images, labels, badges) vanished behind
            // the rectangle's fill (measured: a travel page's 7 photos
            // all rendered as blank cards).
            let clipped = push_clip_content(cx, node, world_rect, zoom);
            for child in node.children.iter().rev() {
                let child_hover = paint_node_inner(cx, child, options, transforms, is_hovered);
                hits.merge_missing(child_hover);
            }
            if clipped {
                cx.backend.restore();
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
                if blur_sigma.is_some() {
                    cx.backend.restore();
                }
                if transformed {
                    cx.backend.restore();
                }
                if overlay_transformed {
                    transforms.pop();
                }
                return hits;
            }
            // Bezier-aware: when the path carries anchors with control
            // handles, flatten each cubic segment; otherwise fall back
            // to the straight `points` polyline.
            let polyline = flatten_path_points(node);
            let points = polyline.as_slice();
            // A closed path with a fill paints its enclosed area.
            let filled = node.path_closed && node.fill.is_some();
            if filled {
                let world = world_path_points(points, viewport_origin, zoom);
                cx.backend
                    .fill_polygon(world.as_slice(), node.fill.unwrap());
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
                for pair in points.windows(2) {
                    cx.backend.stroke_line(
                        doc_to_world_point(pair[0], viewport_origin, zoom),
                        doc_to_world_point(pair[1], viewport_origin, zoom),
                        color,
                        width,
                    );
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

    if blur_sigma.is_some() {
        cx.backend.restore();
    }
    if transformed {
        cx.backend.restore();
    }
    if overlay_transformed {
        transforms.pop();
    }
    hits
}

fn node_outline_rect(node: &SceneNode, options: &PaintNodeOptions<'_, '_>) -> Option<Rect> {
    let bounds = node.aggregate_bounds();
    if bounds.size.x <= 0.0 || bounds.size.y <= 0.0 {
        return None;
    }
    Some(Rect {
        origin: Point2D::new(
            options.viewport_origin.x + bounds.origin.x * options.zoom,
            options.viewport_origin.y + bounds.origin.y * options.zoom,
        ),
        size: Point2D::new(bounds.size.x * options.zoom, bounds.size.y * options.zoom),
    })
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
    RevealPaintState::Active {
        elapsed_ms: elapsed,
    }
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
        // Mesh gradients are a round-rect-only feature in v1 — there's
        // no per-vertex SVG-path fill path. Degrade to the node's
        // resolved first-vertex solid (baked by the loader) so a
        // mesh-filled path still paints.
        Some(SceneGradient::Mesh { .. }) | None => {
            if let Some(fill) = node.fill {
                cx.backend.fill_svg_path_in_rect(d, world_rect, fill);
            }
        }
    }
    // Inset shadows paint over the fill, clipped to the path
    // silhouette. Outer shadows on paths stay deferred (no shape-mask
    // drop-shadow path for arbitrary vectors yet).
    for effect in &node.effects {
        let Effect::DropShadow(s) = effect else {
            continue;
        };
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

#[cfg(test)]
#[path = "canvas_viewport_stroke_align_tests.rs"]
mod stroke_align_tests;
