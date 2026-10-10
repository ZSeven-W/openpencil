//! Rectangle emission helpers for the SVG serializer.

use super::{fill_attrs, fill_stroke_attrs, normalize_rect, stroke_attrs, xml_escape};
use crate::layout_scene::{NodeKind, SceneNode};
use std::fmt::Write as _;

pub(super) fn emit_rect(out: &mut String, node: &SceneNode) {
    if node.fill.is_none()
        && node.gradient.is_none()
        && node.stroke.is_none()
        && !matches!(node.kind, NodeKind::Rect)
    {
        return;
    }
    emit_rect_shape(out, node, Some(&node.id), &fill_stroke_attrs(node));
}

pub(super) fn emit_rect_fill(out: &mut String, node: &SceneNode) {
    emit_rect_shape(out, node, Some(&node.id), &fill_attrs(node));
}

pub(super) fn emit_widget_frame_background(out: &mut String, node: &SceneNode, defer_stroke: bool) {
    let attrs = if defer_stroke {
        fill_attrs(node)
    } else {
        fill_stroke_attrs(node)
    };
    // The widget group owns the scene id; this background is a paint detail.
    emit_rect_shape(out, node, None, &attrs);
}

pub(super) fn emit_rect_stroke_overlay(out: &mut String, node: &SceneNode) {
    let Some(stroke) = node.stroke else {
        return;
    };
    let attrs = format!(
        r#" fill="none"{}"#,
        stroke_attrs(stroke.color, stroke.width)
    );
    // The overlay is a paint detail, not a second scene node. Leaving it
    // unlabelled avoids collisions with authored ids such as `foo-stroke`.
    emit_rect_shape(out, node, None, &attrs);
}

fn emit_rect_shape(out: &mut String, node: &SceneNode, id: Option<&str>, attrs: &str) {
    let rect = normalize_rect(node.bounds);
    if rect.size.x == 0.0 && rect.size.y == 0.0 {
        return;
    }
    let rx = if node.corner_radius > 0.0 {
        format!(r#" rx="{}""#, node.corner_radius)
    } else {
        String::new()
    };
    let id = id
        .map(|id| format!(r#" id="{}""#, xml_escape(id)))
        .unwrap_or_default();
    let _ = write!(
        out,
        r#"<rect{id} x="{}" y="{}" width="{}" height="{}"{rx}{attrs}/>"#,
        rect.origin.x, rect.origin.y, rect.size.x, rect.size.y,
    );
}
