//! Apply helpers for per-node attribute MCP write commands
//! (rotation / text / corner radius / font size / font weight /
//! stroke hex / stroke width / fill hex / name). Carved off
//! `mcp_apply.rs` to stay under the 800-line cap.
//!
//! Each helper is a free function over `&mut Document` so the
//! main match in `mcp_apply.rs` can call it with a single line.

use super::mcp_apply::find_node_mut_in_doc;
use super::variables::parse_hex_color;
use super::{Document, NodeId, NodeKind, Stroke};

pub(super) fn apply_set_node_font_size(
    doc: &mut Document,
    node_id: u64,
    font_size: f32,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    if !font_size.is_finite() || font_size <= 0.0 {
        return false;
    }
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    if !matches!(node.kind, NodeKind::Text) {
        return false;
    }
    node.font_size = font_size;
    true
}

pub(super) fn apply_set_node_font_weight(
    doc: &mut Document,
    node_id: u64,
    font_weight: u16,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    if font_weight == 0 || font_weight > 1000 {
        return false;
    }
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    if !matches!(node.kind, NodeKind::Text) {
        return false;
    }
    node.font_weight = font_weight;
    true
}

pub(super) fn apply_set_node_stroke_hex(
    doc: &mut Document,
    node_id: u64,
    hex: &str,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    let Some(color) = parse_hex_color(hex) else {
        return false;
    };
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    match &mut node.stroke {
        Some(s) => s.color = color,
        none @ None => {
            *none = Some(Stroke { color, width: 1.0 });
        }
    }
    true
}

pub(super) fn apply_set_node_stroke_width(
    doc: &mut Document,
    node_id: u64,
    width: f32,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    if !width.is_finite() || width < 0.0 {
        return false;
    }
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    if width == 0.0 {
        node.stroke = None;
    } else {
        match &mut node.stroke {
            Some(s) => s.width = width,
            none @ None => {
                *none = Some(Stroke {
                    color: crate::Color::BLACK,
                    width,
                });
            }
        }
    }
    true
}

pub(super) fn apply_set_node_fill_hex(
    doc: &mut Document,
    node_id: u64,
    hex: &str,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    let Some(color) = parse_hex_color(hex) else {
        return false;
    };
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    node.fill = Some(color);
    true
}

pub(super) fn apply_set_node_name(
    doc: &mut Document,
    node_id: u64,
    name: &str,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    node.name = trimmed.to_string();
    true
}

pub(super) fn apply_set_node_corner_radius(
    doc: &mut Document,
    node_id: u64,
    radius: f32,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    if !radius.is_finite() || radius < 0.0 {
        return false;
    }
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    node.corner_radius = radius;
    true
}

pub(super) fn apply_set_node_text(
    doc: &mut Document,
    node_id: u64,
    text: &str,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    if !matches!(node.kind, NodeKind::Text) {
        return false;
    }
    node.text = Some(text.to_string());
    true
}

pub(super) fn apply_set_node_rotation(
    doc: &mut Document,
    node_id: u64,
    degrees: f32,
) -> bool {
    let Some(target) = NodeId::from_mcp_u64(node_id) else {
        return false;
    };
    if !degrees.is_finite() {
        return false;
    }
    let Some(node) = find_node_mut_in_doc(doc, &target) else {
        return false;
    };
    node.rotation = degrees.to_radians();
    true
}

pub(super) fn apply_align_selected(doc: &mut Document, action: &str) -> bool {
    let parsed = match action {
        "left" => super::align::AlignAction::Left,
        "center_h" => super::align::AlignAction::CenterH,
        "right" => super::align::AlignAction::Right,
        "top" => super::align::AlignAction::Top,
        "center_v" => super::align::AlignAction::CenterV,
        "bottom" => super::align::AlignAction::Bottom,
        "distribute_h" => super::align::AlignAction::DistributeH,
        "distribute_v" => super::align::AlignAction::DistributeV,
        _ => return false,
    };
    doc.align_selected(parsed)
}
