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
//!
//! Shapes, path projection, reveal timing, paint in/out types and the
//! public entry points live in the sibling `canvas_viewport_paint/`
//! directory so every file stays under the 800-line cap; they are
//! re-exported here so existing `canvas_viewport_paint::…` paths keep
//! resolving.

use crate::layout_scene::{regular_polygon_points, SceneNode};
use crate::layout_scene::{Effect, NodeKind};
use crate::widgets::canvas_viewport_fill_layers::{
    fill_layer_fallback_color, paint_clipped_fill_layers_with, paint_clipped_shape_rich_fill_layer,
    paint_fill_layers, paint_fill_layers_then_stroke,
};
use crate::widgets::canvas_viewport_image::{paint_image_node, paint_image_node_without_stroke};
use crate::widgets::canvas_viewport_overlay::{
    paint_fill_then_stroke, paint_node_fill, paint_node_stroke, scaled_non_uniform_corner_radii,
};
use crate::widgets::canvas_viewport_text::paint_text_node;
use crate::widgets::canvas_viewport_widget::paint_widget_visual;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use jian_scene::path_geometry::flatten_path_points;

#[path = "canvas_viewport_paint_mask.rs"]
mod mask;
use mask::paint_child_siblings;

#[path = "canvas_viewport_layer_bounds.rs"]
mod layer_bounds;
use layer_bounds::{
    node_composite_layer_bounds, sibling_mask_layer_bounds, subtree_intersects_cull,
};

#[path = "canvas_viewport_paint/entry.rs"]
mod entry;
#[path = "canvas_viewport_paint/hits.rs"]
mod hits;
#[path = "canvas_viewport_paint/path_points.rs"]
mod path_points;
#[path = "canvas_viewport_paint/reveal.rs"]
mod reveal;
#[path = "canvas_viewport_paint/shapes.rs"]
mod shapes;

pub use entry::{paint_node, paint_scene_page, paint_scene_page_with_options};
pub(crate) use entry::{
    paint_node_with_options, paint_node_with_options_hiding, paint_scene_nodes_with_options_hiding,
};
pub use hits::PaintNodeHits;
use hits::PaintNodeOptions;
use path_points::doc_to_world_point;
pub(crate) use path_points::world_path_points;
#[cfg(test)]
pub(crate) use path_points::{flatten_path, WorldPathPoints};
pub use reveal::RevealSchedule;
#[cfg(test)]
pub(crate) use reveal::REVEAL_POP_MS;
use reveal::{reveal_paint_state, RevealPaintState};
pub(crate) use reveal::{reveal_pop_scale, REVEAL_WIREFRAME_MS};
#[cfg(test)]
pub(crate) use shapes::arc_polygon;
pub(crate) use shapes::paint_svg_path_node;
use shapes::{paint_drop_shadows, paint_ellipse, push_clip_content};

/// Effects whose zoom-scaled footprint stays under this many device
/// pixels are invisible — but their save-layers still break the GPU
/// render pass (measured: a zoomed-out page with ~4.6k blur effects
/// spent ~85% of every panned frame in per-draw render-pass submits
/// on the macOS GL-on-Metal driver). Sub-pixel effects skip instead.
const MIN_VISIBLE_EFFECT_DEVICE_PX: f32 = 0.3;

use super::canvas_overlay_transform::OverlayTransform;

/// Resolve the active tab option by authored/live value. Missing or stale
/// values deterministically fall back to the first tab/panel.
///
/// Thin forwarder to [`SceneWidget::active_tab_index`] — the canonical rule
/// lives in jian-scene so the canvas hit-test shares it verbatim.
pub fn tabs_active_index(widget: &crate::layout_scene::SceneWidget) -> usize {
    widget.active_tab_index()
}

/// Tabs are the only first-class widget whose children are alternative
/// panels rather than ordinary descendants. `tabs[i]` maps to `children[i]`.
///
/// Single-sourced with the canvas hit-test
/// ([`LayoutScene::node_path_at_doc_point`](jian_scene::layout_scene::LayoutScene::node_path_at_doc_point)):
/// both walk [`SceneNode::visible_children`]. A second copy of this rule is
/// what let a click land on a panel the painter never drew.
fn widget_children_to_paint(node: &SceneNode) -> &[SceneNode] {
    node.visible_children()
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
                if let Some(radii) = scaled_non_uniform_corner_radii(node, zoom) {
                    cx.backend.stroke_round_rect_per_corner(
                        world_rect,
                        radii,
                        blue.with_alpha(0.8 * ramp),
                        1.0,
                    );
                } else {
                    cx.backend.stroke_round_rect(
                        world_rect,
                        radius,
                        blue.with_alpha(0.8 * ramp),
                        1.0,
                    );
                }
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
    // Viewport culling — skip a complete off-screen subtree, not only leaves.
    // Open containers include overflowing descendants; transforms, strokes,
    // shadows, and blur use the same conservative bounds as save layers.
    if !subtree_intersects_cull(
        node,
        viewport_origin,
        zoom,
        cull,
        options.hidden,
        transforms,
    ) {
        return PaintNodeHits::default();
    }
    let dpi_scale = cx.backend.dpi_scale();
    // Sub-pixel LOD: a leaf under ~3/4 of a device pixel contributes
    // nothing visible, but its fill/stroke/clip ops still reach the
    // GPU — a zoomed-out 38k-node page carries ~8k such leaves per
    // frame. Always skipped (not only mid-gesture): it makes the
    // gesture-end full-quality repaint and every zoomed-out resting
    // frame proportionally cheaper at no visible cost.
    if node.children.is_empty()
        && world_rect.size.x.abs().max(world_rect.size.y.abs()) * dpi_scale < 0.75
    {
        return PaintNodeHits::default();
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

    // Background blur filters content already painted behind this
    // node, clipped to the node silhouette. Keep the backdrop layer
    // open while the node paints so translucent fills and children
    // composite over the filtered copy.
    let background_blur_sigma = node
        .effects
        .iter()
        .find_map(|effect| match effect {
            Effect::BackgroundBlur { radius } if *radius > 0.0 => Some(*radius * 0.5 * zoom),
            _ => None,
        })
        // Sub-pixel backdrop blur is invisible; skip its save-layer.
        // Gestures skip all backdrop layers (interactive degrade).
        .filter(|sigma| {
            !options.fast_interaction && sigma * dpi_scale >= MIN_VISIBLE_EFFECT_DEVICE_PX
        });
    let background_blur_pushed = if let Some(sigma) =
        background_blur_sigma.filter(|_| world_rect.size.x > 0.0 && world_rect.size.y > 0.0)
    {
        cx.backend.save();
        let radius = if node.kind == NodeKind::Ellipse {
            world_rect.size.x.min(world_rect.size.y) / 2.0
        } else {
            node.corner_radius * zoom
        };
        if let Some(radii) = scaled_non_uniform_corner_radii(node, zoom) {
            cx.backend.clip_round_rect_per_corner(world_rect, radii);
        } else if radius > 0.5 {
            cx.backend.clip_round_rect(world_rect, radius);
        } else {
            cx.backend.clip_rect(world_rect);
        }
        cx.backend.push_backdrop_blur_layer(sigma);
        true
    } else {
        false
    };

    // Apply local group opacity and node-level blend together after the node's
    // own paint and descendants have assembled. Mask roots suppress authored
    // blend against the mask-source layer, but still need a Normal layer when
    // local group opacity must be applied once.
    let suppress_node_blend = options.suppress_node_composite_id == Some(node.id.as_str());
    let node_composite_mode = if suppress_node_blend {
        crate::ImageBlendMode::Normal
    } else {
        node.blend_mode
    };
    let node_composite_opacity = if node.composite_opacity.is_finite() {
        node.composite_opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let node_composite_pushed =
        node_composite_mode != crate::ImageBlendMode::Normal || node_composite_opacity < 1.0;
    if node_composite_pushed {
        let bounds = node_composite_layer_bounds(
            node,
            options.viewport_origin,
            options.zoom,
            options.cull,
            options.hidden,
            transforms,
        );
        cx.backend
            .push_composite_layer(bounds, node_composite_opacity, node_composite_mode);
    }

    // Gaussian layer blur (Figma "Layer blur"): capture the node's
    // whole rendered output — shadows, fill, stroke, children — into
    // an offscreen layer and blur it on the matching `restore`. The
    // CSS radius → Skia sigma conversion is `radius / 2`, scaled by
    // the viewport zoom. Popped at every return path below alongside
    // the transform save.
    let blur_sigma = node
        .effects
        .iter()
        .find_map(|e| match e {
            Effect::Blur(b) if b.radius > 0.0 => Some(b.radius * 0.5 * zoom),
            _ => None,
        })
        // Sub-pixel layer blur is invisible; skip its save-layer.
        // Gestures skip all blur layers (interactive degrade).
        .filter(|sigma| {
            !options.fast_interaction && sigma * dpi_scale >= MIN_VISIBLE_EFFECT_DEVICE_PX
        });
    if let Some(sigma) = blur_sigma {
        cx.backend.push_blur_layer(sigma);
    }

    // Drop shadows paint behind the node's own fill. Only kinds
    // whose silhouette a rounded rect / ellipse can represent
    // faithfully (Frame / Rect / Ellipse) cast one; Polygon / Line
    // / Path shadows are deferred until a shape-mask path exists.
    if !node.effects.is_empty()
        && !options.fast_interaction
        && world_rect.size.x > 0.0
        && world_rect.size.y > 0.0
        && matches!(
            node.kind,
            NodeKind::Frame | NodeKind::Rect | NodeKind::Ellipse
        )
    {
        paint_drop_shadows(cx, node, world_rect, zoom, dpi_scale);
    }

    match &node.kind {
        NodeKind::Frame => {
            let has_children = !node.children.is_empty();
            // Image-fill Frames paint the bitmap behind their
            // children; gradient + solid fall back to the shared
            // fill/stroke painter. Without this branch a Frame whose
            // primary fill is `PenFill::Image { url }` only shows the
            // grey placeholder + its children, never the image.
            if has_children {
                if paint_fill_layers(cx, node, world_rect, zoom) {
                    // Container stroke is deferred until after its children.
                } else if let Some(src) = node.image_src.as_deref() {
                    paint_image_node_without_stroke(
                        cx,
                        node,
                        world_rect,
                        zoom,
                        src,
                        !options.mask_source,
                    );
                } else {
                    paint_node_fill(cx, node, world_rect, zoom, node.fill);
                }
            } else if paint_fill_layers_then_stroke(cx, node, world_rect, zoom) {
                // painted
            } else if let Some(src) = node.image_src.as_deref() {
                paint_image_node(cx, node, world_rect, zoom, src, !options.mask_source);
            } else {
                paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
            }
            // `tabs` degrades to a `frame`; retain its tab bar while only
            // the authored/live active panel participates in paint + hits.
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
            paint_child_siblings(
                cx,
                widget_children_to_paint(node),
                options,
                transforms,
                is_hovered,
                &mut hits,
            );
            if clipped {
                cx.backend.restore();
            }
            if has_children {
                paint_node_stroke(cx, node, world_rect, zoom);
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
            paint_child_siblings(
                cx,
                &node.children,
                options,
                transforms,
                is_hovered,
                &mut hits,
            );
            if clipped {
                cx.backend.restore();
            }
        }
        NodeKind::Rect => {
            let has_children = !node.children.is_empty();
            // Composite widgets that degrade to `rect` (switch /
            // checkbox / slider / progress / radio_group / number_input
            // / text_area) paint their recognizable static visual on the
            // design surface instead of the bare rect.
            let widget_painted = paint_widget_visual(cx, node, world_rect, zoom);
            if !widget_painted {
                if has_children {
                    if paint_fill_layers(cx, node, world_rect, zoom) {
                        // Container stroke is deferred until after its children.
                    } else if let Some(src) = node.image_src.as_deref() {
                        paint_image_node_without_stroke(
                            cx,
                            node,
                            world_rect,
                            zoom,
                            src,
                            !options.mask_source,
                        );
                    } else {
                        paint_node_fill(cx, node, world_rect, zoom, node.fill);
                    }
                } else if paint_fill_layers_then_stroke(cx, node, world_rect, zoom) {
                    // painted
                } else if let Some(src) = node.image_src.as_deref() {
                    // Image nodes land as `kind="rect"` (the loader rewrites
                    // their variant so non-image paths keep working). When a
                    // `src` is carried, paint the bitmap; the grey `fill`
                    // remains as the placeholder visible while the decoder
                    // is missing the bytes (corrupt URL / unsupported codec).
                    paint_image_node(cx, node, world_rect, zoom, src, !options.mask_source);
                } else {
                    paint_fill_then_stroke(cx, node, world_rect, zoom, node.fill);
                }
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
            paint_child_siblings(
                cx,
                &node.children,
                options,
                transforms,
                is_hovered,
                &mut hits,
            );
            if clipped {
                cx.backend.restore();
            }
            if has_children && !widget_painted {
                paint_node_stroke(cx, node, world_rect, zoom);
            }
        }
        NodeKind::Ellipse => {
            if !node.fill_layers.is_empty() {
                paint_ellipse(cx, node, world_rect, zoom);
            } else if let Some(src) = node.image_src.as_deref() {
                // Image-fill ellipse: paint the bitmap clipped to the
                // ellipse silhouette via skia's `clip_oval`-style
                // approximation (no native clip_oval on the trait, so
                // fall back to the rect-clip path the painter has).
                paint_image_node(cx, node, world_rect, zoom, src, !options.mask_source);
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
            let layered = paint_clipped_fill_layers_with(
                cx,
                node,
                world_rect,
                |backend| backend.clip_polygon(&pts),
                |cx, layer| {
                    if !paint_clipped_shape_rich_fill_layer(cx, node, layer, world_rect, zoom) {
                        if let Some(fill) = fill_layer_fallback_color(layer) {
                            // Let the polygon clip provide the only AA edge.
                            cx.backend.fill_rect(world_rect, fill);
                        }
                    }
                },
            );
            if !layered {
                // Image fills paint the bitmap in the AABB underneath the
                // polygon outline; the polygon silhouette is then drawn
                // by the stroke. A perfect clip-to-polygon path lands when
                // `RenderBackend` grows a polygon-clip primitive.
                if let Some(src) = node.image_src.as_deref() {
                    paint_image_node(cx, node, world_rect, zoom, src, !options.mask_source);
                } else if let Some(fill) = node.fill {
                    cx.backend.fill_polygon(&pts, fill);
                }
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
                if node_composite_pushed {
                    cx.backend.restore();
                }
                if background_blur_pushed {
                    cx.backend.restore();
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
            // A closed path with a fill paints its enclosed area. Canonical
            // fill stacks share the same ordering/blend compositor as other
            // shapes while retaining the polyline's exact silhouette.
            let world = node
                .path_closed
                .then(|| world_path_points(points, viewport_origin, zoom));
            let layered = world.as_ref().is_some_and(|world| {
                paint_clipped_fill_layers_with(
                    cx,
                    node,
                    world_rect,
                    |backend| backend.clip_polygon(world.as_slice()),
                    |cx, layer| {
                        if !paint_clipped_shape_rich_fill_layer(cx, node, layer, world_rect, zoom) {
                            if let Some(fill) = fill_layer_fallback_color(layer) {
                                // Let the path clip provide the only AA edge.
                                cx.backend.fill_rect(world_rect, fill);
                            }
                        }
                    },
                )
            });
            let filled = node.path_closed && (layered || node.fill.is_some());
            if filled && !layered {
                cx.backend
                    .fill_polygon(world.as_ref().unwrap().as_slice(), node.fill.unwrap());
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
    if node_composite_pushed {
        cx.backend.restore();
    }
    if background_blur_pushed {
        cx.backend.restore();
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

#[cfg(test)]
#[path = "canvas_viewport_paint_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "canvas_viewport_reveal_tests.rs"]
mod reveal_tests;

#[cfg(test)]
#[path = "canvas_viewport_stroke_align_tests.rs"]
mod stroke_align_tests;

#[cfg(test)]
#[path = "canvas_viewport_layered_shape_tests.rs"]
mod layered_shape_tests;

#[cfg(test)]
#[path = "canvas_viewport_node_blend_tests.rs"]
mod node_blend_tests;

#[cfg(test)]
#[path = "canvas_viewport_tabs_tests.rs"]
mod tabs_tests;
