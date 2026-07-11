use crate::layout_scene::SceneNode;
use crate::{Point2D, Rect};
use std::fmt::Write as _;

#[derive(Clone)]
pub(super) struct AncestorClip {
    id: String,
    bounds: Rect,
    corner_radius: f32,
    xform: glam::Affine2,
}

pub(super) fn find_node_with_ancestor_context<'a>(
    nodes: &'a [SceneNode],
    node_id: &str,
) -> Option<(&'a SceneNode, glam::Affine2, Vec<AncestorClip>)> {
    find_node_inner(nodes, node_id, glam::Affine2::IDENTITY, &mut Vec::new())
}

fn find_node_inner<'a>(
    nodes: &'a [SceneNode],
    node_id: &str,
    ancestor_xform: glam::Affine2,
    clips: &mut Vec<AncestorClip>,
) -> Option<(&'a SceneNode, glam::Affine2, Vec<AncestorClip>)> {
    for node in nodes {
        if node.id == node_id {
            return Some((node, ancestor_xform, clips.clone()));
        }
        let child_xform = ancestor_xform * node_transform(node);
        let clips_children = node.clip_content
            && !node.children.is_empty()
            && node.bounds.size.x != 0.0
            && node.bounds.size.y != 0.0;
        if clips_children {
            clips.push(AncestorClip {
                id: format!("ancestor-clip-{}", svg_id(&node.id)),
                bounds: normalize_rect(node.bounds),
                corner_radius: node.corner_radius.max(0.0),
                xform: child_xform,
            });
        }
        if let Some(found) = find_node_inner(&node.children, node_id, child_xform, clips) {
            return Some(found);
        }
        if clips_children {
            clips.pop();
        }
    }
    None
}

pub(super) fn node_transform(n: &SceneNode) -> glam::Affine2 {
    let pivot_rect = n.aggregate_bounds();
    if pivot_rect.size.x == 0.0 && pivot_rect.size.y == 0.0 {
        return glam::Affine2::IDENTITY;
    }
    let pivot = glam::Vec2::new(
        pivot_rect.origin.x + pivot_rect.size.x * 0.5,
        pivot_rect.origin.y + pivot_rect.size.y * 0.5,
    );
    let around = |inner| {
        glam::Affine2::from_translation(pivot) * inner * glam::Affine2::from_translation(-pivot)
    };
    let flip = around(glam::Affine2::from_scale(glam::Vec2::new(
        if n.flip_x { -1.0 } else { 1.0 },
        if n.flip_y { -1.0 } else { 1.0 },
    )));
    let rotation = around(glam::Affine2::from_angle(n.rotation));
    flip * rotation
}

pub(super) fn emit_affine_group_start(out: &mut String, xform: glam::Affine2) {
    let _ = write!(out, r#"<g transform="{}">"#, matrix_value(xform));
}

pub(super) fn emit_ancestor_clip_defs(out: &mut String, clips: &[AncestorClip]) {
    for clip in clips {
        let r = clip.bounds;
        let _ = write!(
            out,
            r#"<clipPath id="{}" clipPathUnits="userSpaceOnUse"><rect x="{}" y="{}" width="{}" height="{}" rx="{}" transform="{}"/></clipPath>"#,
            clip.id,
            r.origin.x,
            r.origin.y,
            r.size.x,
            r.size.y,
            clip.corner_radius,
            matrix_value(clip.xform),
        );
    }
}

pub(super) fn emit_ancestor_clip_groups_start(out: &mut String, clips: &[AncestorClip]) {
    for clip in clips {
        let _ = write!(out, r#"<g clip-path="url(#{})">"#, clip.id);
    }
}

pub(super) fn close_ancestor_clip_groups(out: &mut String, clips: &[AncestorClip]) {
    for _ in clips {
        out.push_str("</g>");
    }
}

pub(super) fn apply_ancestor_clip_bounds(mut bounds: Rect, clips: &[AncestorClip]) -> Option<Rect> {
    for clip in clips {
        bounds = intersect(bounds, transformed_bounds(clip.bounds, clip.xform))?;
    }
    Some(bounds)
}

pub(super) fn apply_clip_bounds(
    bounds: Rect,
    clip_bounds: Rect,
    xform: glam::Affine2,
) -> Option<Rect> {
    intersect(
        bounds,
        transformed_bounds(normalize_rect(clip_bounds), xform),
    )
}

pub(super) fn svg_id(id: &str) -> String {
    let mut out = String::with_capacity(2 + id.len() * 2);
    out.push_str("n-");
    for byte in id.as_bytes() {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn matrix_value(xform: glam::Affine2) -> String {
    let m = xform.matrix2;
    let t = xform.translation;
    format!(
        "matrix({} {} {} {} {} {})",
        m.x_axis.x, m.x_axis.y, m.y_axis.x, m.y_axis.y, t.x, t.y
    )
}

fn transformed_bounds(rect: Rect, xform: glam::Affine2) -> Rect {
    let points = [
        glam::Vec2::new(rect.origin.x, rect.origin.y),
        glam::Vec2::new(rect.origin.x + rect.size.x, rect.origin.y),
        glam::Vec2::new(rect.origin.x + rect.size.x, rect.origin.y + rect.size.y),
        glam::Vec2::new(rect.origin.x, rect.origin.y + rect.size.y),
    ]
    .map(|point| xform.transform_point2(point));
    let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    Rect::xywh(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn intersect(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a.origin.x.max(b.origin.x);
    let y0 = a.origin.y.max(b.origin.y);
    let x1 = (a.origin.x + a.size.x).min(b.origin.x + b.size.x);
    let y1 = (a.origin.y + a.size.y).min(b.origin.y + b.size.y);
    (x1 > x0 && y1 > y0).then(|| Rect::xywh(x0, y0, x1 - x0, y1 - y0))
}

fn normalize_rect(r: Rect) -> Rect {
    let x0 = r.origin.x.min(r.origin.x + r.size.x);
    let y0 = r.origin.y.min(r.origin.y + r.size.y);
    Rect {
        origin: Point2D::new(x0, y0),
        size: Point2D::new(r.size.x.abs(), r.size.y.abs()),
    }
}
