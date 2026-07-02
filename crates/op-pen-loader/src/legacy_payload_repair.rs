//! Paint-only compatibility repairs for legacy generated `.op` files.
//!
//! Layout repair adjusts computed rectangles before payload conversion. This
//! module handles authored helper strokes that should not paint as visible UI.

use jian_ops_schema::node::{FrameNode, PenNode};

use crate::payload::NodePayload;

pub(crate) fn repair_payload_for_legacy_node(node: &PenNode, payload: &mut NodePayload) {
    let PenNode::Frame(frame) = node else {
        return;
    };

    if is_legacy_status_bar_shell(frame, payload)
        || is_legacy_battery_wrapper(frame, payload)
        || is_legacy_help_expanded_row_side_stroke(frame, payload)
    {
        payload.stroke = None;
    } else if is_legacy_dark_help_card_missing_stroke_fill(payload) {
        if let Some(stroke) = payload.stroke.as_mut() {
            stroke.color = [
                0x2A as f32 / 255.0,
                0x2B as f32 / 255.0,
                0x30 as f32 / 255.0,
                1.0,
            ];
        }
    }
}

fn is_legacy_status_bar_shell(frame: &FrameNode, payload: &NodePayload) -> bool {
    if !matches!(frame.base.name.as_deref(), Some("Time" | "Levels")) {
        return false;
    }
    if frame.container.fill.is_some() || payload.stroke.is_none() {
        return false;
    }
    if frame
        .children
        .as_deref()
        .map(|children| children.is_empty())
        .unwrap_or(true)
    {
        return false;
    }

    // Height (~22) + the `Time`/`Levels` name + a no-fill stroke over
    // non-empty children pins this to an iPhone status-bar shell. The
    // width is NOT constrained: the shells are authored `fill_container`,
    // so a wider status bar (pencil-demo's 402-px bar) computes a much
    // larger width than the ~100 an earlier fixed-width sample had, and a
    // hard width match silently left those shells painting a black frame.
    nearly(payload.h, 22.0)
}

fn is_legacy_battery_wrapper(frame: &FrameNode, payload: &NodePayload) -> bool {
    if frame.base.name.as_deref() != Some("Frame")
        || frame.container.fill.is_some()
        || payload.stroke.is_none()
        || !nearly(payload.w, 27.328037)
        || !nearly(payload.h, 13.0)
    {
        return false;
    }
    let Some(children) = frame.children.as_deref() else {
        return false;
    };
    let has_child = |name: &str| {
        children
            .iter()
            .any(|child| child_name(child).is_some_and(|child_name| child_name == name))
    };
    has_child("Border") && has_child("Capacity")
}

fn is_legacy_help_expanded_row_side_stroke(frame: &FrameNode, payload: &NodePayload) -> bool {
    let Some(name) = frame.base.name.as_deref() else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    if !name.starts_with("help2") || frame.container.fill.is_none() {
        return false;
    }
    let Some(stroke) = payload.stroke.as_ref() else {
        return false;
    };
    let Some([top, right, bottom, left]) = stroke.sides else {
        return false;
    };
    top <= 0.0 && bottom <= 0.0 && right > 0.0 && left > 0.0
}

fn is_legacy_dark_help_card_missing_stroke_fill(payload: &NodePayload) -> bool {
    let name = payload.name.to_ascii_lowercase();
    if !name.starts_with("helpcard") {
        return false;
    }
    let Some(stroke) = payload.stroke.as_ref() else {
        return false;
    };
    if !is_black(stroke.color) {
        return false;
    }
    payload.fill.is_some_and(|fill| luminance(fill) < 0.2)
}

fn child_name(node: &PenNode) -> Option<&str> {
    match node {
        PenNode::Frame(n) => n.base.name.as_deref(),
        PenNode::Group(n) => n.base.name.as_deref(),
        PenNode::Rectangle(n) => n.base.name.as_deref(),
        PenNode::Text(n) => n.base.name.as_deref(),
        PenNode::TextInput(n) => n.base.name.as_deref(),
        PenNode::TextArea(n) => n.base.name.as_deref(),
        PenNode::Select(n) => n.base.name.as_deref(),
        PenNode::Switch(n) => n.base.name.as_deref(),
        PenNode::Checkbox(n) => n.base.name.as_deref(),
        PenNode::Slider(n) => n.base.name.as_deref(),
        PenNode::RadioGroup(n) => n.base.name.as_deref(),
        PenNode::NumberInput(n) => n.base.name.as_deref(),
        PenNode::Progress(n) => n.base.name.as_deref(),
        PenNode::Tabs(n) => n.base.name.as_deref(),
        PenNode::IconFont(n) => n.base.name.as_deref(),
        PenNode::Image(n) => n.base.name.as_deref(),
        PenNode::Ellipse(n) => n.base.name.as_deref(),
        PenNode::Line(n) => n.base.name.as_deref(),
        PenNode::Path(n) => n.base.name.as_deref(),
        PenNode::Polygon(n) => n.base.name.as_deref(),
        PenNode::Ref(n) => n.base.name.as_deref(),
    }
}

fn nearly(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.5
}

fn is_black(color: [f32; 4]) -> bool {
    color[0] <= 0.01 && color[1] <= 0.01 && color[2] <= 0.01 && color[3] > 0.0
}

fn luminance(color: [f32; 4]) -> f32 {
    0.2126 * color[0] + 0.7152 * color[1] + 0.0722 * color[2]
}
