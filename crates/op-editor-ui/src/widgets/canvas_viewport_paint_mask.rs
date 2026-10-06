//! Figma sibling masks for the shared canvas painter.
//!
//! Opaque path masks keep a cheap geometry-clip fast path. Every real ALPHA
//! or LUMINANCE mask uses two save-layers: front siblings assemble into an
//! isolated content layer, then the deferred mask subtree assembles in a
//! second layer whose restore paint applies DstIn (plus luma conversion for
//! LUMINANCE). Applying DstIn directly to the main canvas would erase the
//! already-painted backdrop and is therefore never used here.

use super::{
    flatten_path_points, paint_node_inner, sibling_mask_layer_bounds, world_path_points, NodeKind,
    OverlayTransform, PaintCx, PaintNodeHits, PaintNodeOptions, Point2D, Rect, SceneNode,
};
use crate::layout_scene::{MaskType, SceneFillLayer};
use crate::ImageBlendMode;

enum ActiveMask<'a> {
    Clip,
    Pixel { node: &'a SceneNode, kind: MaskType },
}

fn mask_type(node: &SceneNode) -> Option<MaskType> {
    node.mask_type
        .or_else(|| node.is_mask.then_some(MaskType::Alpha))
}

/// Paint a container's topmost-first child list back-to-front while honoring
/// sibling-mask runs. A new mask finishes the preceding run before it starts a
/// fresh one, so two masks never accidentally nest their DstIn operations.
pub(super) fn paint_child_siblings<'a>(
    cx: &mut PaintCx<'_>,
    children: &'a [SceneNode],
    options: &PaintNodeOptions<'_, '_>,
    transforms: &mut Vec<OverlayTransform>,
    parent_hovered: bool,
    hits: &mut PaintNodeHits<'a>,
) {
    let mut active = None;
    for (index, child) in children.iter().enumerate().rev() {
        let Some(kind) = mask_type(child) else {
            let child_hits = paint_node_inner(cx, child, options, transforms, parent_hovered);
            hits.merge_missing(child_hits);
            continue;
        };

        finish_active_mask(cx, active.take(), options, transforms);
        let hidden = child.hidden || options.hidden == Some(child.id.as_str());
        if !hidden {
            active = begin_mask(cx, child, kind, &children[..index], options, transforms);
            hits.merge_missing(PaintNodeHits::for_node(
                child,
                options,
                transforms,
                parent_hovered,
            ));
        }
    }
    finish_active_mask(cx, active, options, transforms);
}

fn begin_mask<'a>(
    cx: &mut PaintCx<'_>,
    mask: &'a SceneNode,
    kind: MaskType,
    front_siblings: &'a [SceneNode],
    options: &PaintNodeOptions<'_, '_>,
    transforms: &[OverlayTransform],
) -> Option<ActiveMask<'a>> {
    let clip_eligible = match kind {
        MaskType::Alpha => alpha_clip_is_exact(mask),
        MaskType::Vector => vector_clip_is_exact(mask),
        MaskType::Luminance => false,
    };
    if clip_eligible && push_path_mask_clip(cx, mask, options) {
        return Some(ActiveMask::Clip);
    }

    // General VECTOR masks need a dedicated opaque fill/stroke coverage
    // renderer. Falling back to the node's alpha would be a different mask
    // operation, so unsupported vector shapes stay explicitly unmasked.
    if kind == MaskType::Vector || !cx.backend.supports_pixel_masks() {
        return None;
    }

    // Only the siblings between this mask and the next mask toward the front
    // belong to this run. Include both their effect-aware subtrees and the
    // complete mask source, then cap the allocation at the visible cull.
    let content = front_siblings
        .iter()
        .rev()
        .take_while(|node| mask_type(node).is_none());
    let bounds = sibling_mask_layer_bounds(
        mask,
        content,
        options.viewport_origin,
        options.zoom,
        options.cull,
        options.hidden,
        transforms,
    );
    cx.backend
        .push_composite_layer(bounds, 1.0, ImageBlendMode::Normal);
    Some(ActiveMask::Pixel { node: mask, kind })
}

fn finish_active_mask(
    cx: &mut PaintCx<'_>,
    active: Option<ActiveMask<'_>>,
    options: &PaintNodeOptions<'_, '_>,
    transforms: &mut Vec<OverlayTransform>,
) {
    match active {
        None => {}
        Some(ActiveMask::Clip) => cx.backend.restore(),
        Some(ActiveMask::Pixel { node, kind }) => {
            cx.backend
                .push_mask_source_layer(kind == MaskType::Luminance);
            let mask_options = PaintNodeOptions {
                viewport_origin: options.viewport_origin,
                zoom: options.zoom,
                edit_caret: None,
                cull: options.cull,
                reveals: None,
                hovered: None,
                selected: None,
                pen: None,
                hidden: options.hidden,
                now_ms: options.now_ms,
                generating_descendant_ids: None,
                generation_accent: None,
                queued_shell_ids: None,
                mask_source: true,
                show_video_badge: options.show_video_badge,
                suppress_node_composite_id: Some(node.id.as_str()),
                // Mask coverage must stay exact even mid-gesture — a
                // skipped sub-pixel mask leaf would change what the
                // mask reveals, not just its fidelity.
                fast_interaction: false,
            };
            let _ = paint_node_inner(cx, node, &mask_options, transforms, false);
            // First restore applies the assembled source to isolated content
            // with DstIn; second restore composites the result over backdrop.
            cx.backend.restore();
            cx.backend.restore();
        }
    }
}

fn alpha_clip_is_exact(mask: &SceneNode) -> bool {
    if mask.kind != NodeKind::Path
        || mask.opacity < 0.999
        || mask.composite_opacity < 0.999
        || mask.stroke.is_some()
        || !mask.effects.is_empty()
        || !mask.children.is_empty()
    {
        return false;
    }
    if mask.fill_layers.is_empty() {
        return mask.fill.is_some_and(|color| color.a >= 0.999);
    }
    mask.fill_layers
        .iter()
        .any(|layer| matches!(layer, SceneFillLayer::Solid { color, .. } if color.a >= 0.999))
}

fn vector_clip_is_exact(mask: &SceneNode) -> bool {
    mask.kind == NodeKind::Path
        && mask.stroke.is_none()
        && mask.children.is_empty()
        && (!mask.fill_layers.is_empty()
            || mask.fill.is_some()
            || mask.gradient.is_some()
            || mask.image_src.is_some())
}

/// Install a path mask in device space while restoring the canvas
/// transform to the parent before masked siblings paint. Skia/CanvasKit clips
/// are captured in device coordinates, so applying the mask's own flip /
/// rotation, clipping, then applying the inverse keeps the clip transformed
/// without transforming the sibling content.
fn push_path_mask_clip(
    cx: &mut PaintCx<'_>,
    mask: &SceneNode,
    options: &PaintNodeOptions<'_, '_>,
) -> bool {
    if mask.hidden
        || mask.kind != NodeKind::Path
        || mask.bounds.size.x <= 0.0
        || mask.bounds.size.y <= 0.0
    {
        return false;
    }

    let viewport_origin = options.viewport_origin;
    let zoom = options.zoom;
    let world_rect = Rect {
        origin: Point2D::new(
            viewport_origin.x + mask.bounds.origin.x * zoom,
            viewport_origin.y + mask.bounds.origin.y * zoom,
        ),
        size: Point2D::new(mask.bounds.size.x * zoom, mask.bounds.size.y * zoom),
    };
    let svg_path = mask
        .svg_path
        .as_deref()
        .filter(|path| !path.trim().is_empty());
    let flattened = svg_path.is_none().then(|| flatten_path_points(mask));
    let polygon = flattened
        .as_ref()
        .filter(|_| mask.path_closed)
        .map(|points| world_path_points(points.as_slice(), viewport_origin, zoom))
        .filter(|points| points.as_slice().len() >= 3);
    if svg_path.is_none() && polygon.is_none() {
        return false;
    }

    cx.backend.save();
    let pivot_doc = mask.aggregate_bounds();
    let pivot = Point2D::new(
        viewport_origin.x + (pivot_doc.origin.x + pivot_doc.size.x / 2.0) * zoom,
        viewport_origin.y + (pivot_doc.origin.y + pivot_doc.size.y / 2.0) * zoom,
    );
    if mask.flip_x || mask.flip_y {
        cx.backend.scale(
            Point2D::new(
                if mask.flip_x { -1.0 } else { 1.0 },
                if mask.flip_y { -1.0 } else { 1.0 },
            ),
            pivot,
        );
    }
    if mask.rotation.abs() > f32::EPSILON {
        cx.backend.rotate(mask.rotation, pivot);
    }

    if let Some(path) = svg_path {
        cx.backend
            .clip_svg_path_in_rect(path, world_rect, mask.even_odd_fill);
    } else if let Some(points) = polygon.as_ref() {
        cx.backend.clip_polygon(points.as_slice());
    }

    // Return to the parent's transform while retaining the device-space clip.
    if mask.rotation.abs() > f32::EPSILON {
        cx.backend.rotate(-mask.rotation, pivot);
    }
    if mask.flip_x || mask.flip_y {
        cx.backend.scale(
            Point2D::new(
                if mask.flip_x { -1.0 } else { 1.0 },
                if mask.flip_y { -1.0 } else { 1.0 },
            ),
            pivot,
        );
    }
    true
}
