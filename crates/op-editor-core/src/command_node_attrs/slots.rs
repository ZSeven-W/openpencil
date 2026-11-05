//! Per-variant slot accessors + writers shared by the attribute
//! commands: corner radius, effects list, and the stroke thickness
//! family (uniform / per-side / sided). Carved off
//! `command_node_attrs.rs` to keep every file under the 800-line cap.

use crate::command::StrokeSide;
use jian_ops_schema::node::{CornerRadius, PenNode};
use jian_ops_schema::style::{PenEffect, PenStroke, SidedThickness, StrokeThickness};

/// Write a literal corner radius onto whatever variant carries one.
/// Frame / Group / Rectangle store `CornerRadius` on `container`;
/// Ellipse / Polygon carry an `f64`. Other kinds accept the call as a
/// silent no-op (parity with shell-core, where the radius was simply
/// invisible on non-rounded kinds). True when a field was written.
pub(super) fn write_corner_radius(node: &mut PenNode, radius: f64) -> bool {
    match node {
        PenNode::Frame(n) => {
            n.container.corner_radius = Some(CornerRadius::Uniform(radius));
            true
        }
        PenNode::Group(n) => {
            n.container.corner_radius = Some(CornerRadius::Uniform(radius));
            true
        }
        PenNode::Rectangle(n) => {
            n.container.corner_radius = Some(CornerRadius::Uniform(radius));
            true
        }
        PenNode::Image(n) => {
            n.corner_radius = Some(CornerRadius::Uniform(radius));
            true
        }
        PenNode::Ellipse(n) => {
            n.corner_radius = Some(radius);
            true
        }
        PenNode::Polygon(n) => {
            n.corner_radius = Some(radius);
            true
        }
        // Other kinds have no corner-radius field; the write is a
        // silent no-op so the command still reports success.
        _ => true,
    }
}

fn corner_radius_slot(node: &mut PenNode) -> Option<&mut Option<CornerRadius>> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container.corner_radius),
        PenNode::Group(n) => Some(&mut n.container.corner_radius),
        PenNode::Rectangle(n) => Some(&mut n.container.corner_radius),
        PenNode::Image(n) => Some(&mut n.corner_radius),
        _ => None,
    }
}

pub(super) fn write_corner_radius_at(node: &mut PenNode, index: usize, radius: f64) -> bool {
    let Some(slot) = corner_radius_slot(node) else {
        return false;
    };
    let mut values = match slot.as_ref() {
        Some(CornerRadius::Uniform(value)) => [*value; 4],
        Some(CornerRadius::PerCorner(values)) => *values,
        // Editing one corner of a `$variable` radius detaches it.
        Some(CornerRadius::Expression(_)) | None => [0.0; 4],
    };
    let Some(value) = values.get_mut(index) else {
        return false;
    };
    *value = radius;
    if values
        .iter()
        .all(|value| (*value - values[0]).abs() < f64::EPSILON)
    {
        *slot = Some(CornerRadius::Uniform(values[0]));
    } else {
        *slot = Some(CornerRadius::PerCorner(values));
    }
    true
}

/// Mutably borrow whatever variant's `effects` field. Frame / Group /
/// Rectangle keep it on `container`; the leaf kinds carry it directly.
/// `None` for IconFont / Ref (no effects field in the schema).
pub(super) fn node_effects_slot(node: &mut PenNode) -> Option<&mut Option<Vec<PenEffect>>> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container.effects),
        PenNode::Group(n) => Some(&mut n.container.effects),
        PenNode::Rectangle(n) => Some(&mut n.container.effects),
        PenNode::Ellipse(n) => Some(&mut n.effects),
        PenNode::Polygon(n) => Some(&mut n.effects),
        PenNode::Path(n) => Some(&mut n.effects),
        PenNode::Line(n) => Some(&mut n.effects),
        PenNode::Text(n) => Some(&mut n.effects),
        PenNode::TextInput(n) => Some(&mut n.effects),
        PenNode::Image(n) => Some(&mut n.effects),
        PenNode::TextArea(n) => Some(&mut n.effects),
        PenNode::Select(n) => Some(&mut n.effects),
        PenNode::Switch(n) => Some(&mut n.effects),
        PenNode::Checkbox(n) => Some(&mut n.effects),
        PenNode::Slider(n) => Some(&mut n.effects),
        PenNode::RadioGroup(n) => Some(&mut n.effects),
        PenNode::NumberInput(n) => Some(&mut n.effects),
        PenNode::Progress(n) => Some(&mut n.effects),
        PenNode::Tabs(n) => Some(&mut n.effects),
        PenNode::IconFont(_) | PenNode::Ref(_) => None,
    }
}

/// Mutably borrow whatever variant's `stroke` field. Mirrors the
/// `fills::node_stroke_mut` arm set. `None` for Text / Image / Ref.
pub(super) fn node_stroke_slot(node: &mut PenNode) -> Option<&mut Option<PenStroke>> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container.stroke),
        PenNode::Group(n) => Some(&mut n.container.stroke),
        PenNode::Rectangle(n) => Some(&mut n.container.stroke),
        PenNode::Ellipse(n) => Some(&mut n.stroke),
        PenNode::Polygon(n) => Some(&mut n.stroke),
        PenNode::Path(n) => Some(&mut n.stroke),
        PenNode::Line(n) => Some(&mut n.stroke),
        PenNode::TextInput(n) => Some(&mut n.stroke),
        PenNode::IconFont(n) => Some(&mut n.stroke),
        PenNode::TextArea(n) => Some(&mut n.stroke),
        PenNode::Select(n) => Some(&mut n.stroke),
        PenNode::Switch(n) => Some(&mut n.stroke),
        PenNode::Checkbox(n) => Some(&mut n.stroke),
        PenNode::Slider(n) => Some(&mut n.stroke),
        PenNode::RadioGroup(n) => Some(&mut n.stroke),
        PenNode::NumberInput(n) => Some(&mut n.stroke),
        PenNode::Progress(n) => Some(&mut n.stroke),
        PenNode::Tabs(n) => Some(&mut n.stroke),
        PenNode::Text(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

pub(super) fn set_stroke_width_preserving_sides(stroke: &mut PenStroke, width: f32) {
    stroke.thickness = match &stroke.thickness {
        StrokeThickness::Uniform(_) => StrokeThickness::Uniform(width),
        StrokeThickness::PerSide(sides) => {
            StrokeThickness::PerSide(sides.map(|side| if side > 0.0 { width } else { 0.0 }))
        }
        StrokeThickness::Sided(sides) => StrokeThickness::Sided(SidedThickness {
            top: scaled_side_width(sides.top, width),
            right: scaled_side_width(sides.right, width),
            bottom: scaled_side_width(sides.bottom, width),
            left: scaled_side_width(sides.left, width),
        }),
    };
}

fn scaled_side_width(side: Option<f32>, width: f32) -> Option<f32> {
    side.map(|value| if value > 0.0 { width } else { 0.0 })
}

pub(super) fn stroke_side_index(side: StrokeSide) -> usize {
    match side {
        StrokeSide::Top => 0,
        StrokeSide::Right => 1,
        StrokeSide::Bottom => 2,
        StrokeSide::Left => 3,
    }
}

pub(super) fn stroke_side_widths(thickness: &StrokeThickness) -> [f32; 4] {
    match thickness {
        StrokeThickness::Uniform(width) => [*width; 4],
        StrokeThickness::PerSide(sides) => *sides,
        StrokeThickness::Sided(sides) => [
            sides.top.unwrap_or(0.0),
            sides.right.unwrap_or(0.0),
            sides.bottom.unwrap_or(0.0),
            sides.left.unwrap_or(0.0),
        ],
    }
}

fn stroke_side_value(width: f32) -> Option<f32> {
    (width > 0.0).then_some(width)
}

pub(super) fn sided_stroke_thickness(widths: [f32; 4]) -> StrokeThickness {
    StrokeThickness::Sided(SidedThickness {
        top: stroke_side_value(widths[0]),
        right: stroke_side_value(widths[1]),
        bottom: stroke_side_value(widths[2]),
        left: stroke_side_value(widths[3]),
    })
}

pub(super) fn set_stroke_side_width(stroke: &mut PenStroke, side: StrokeSide, width: f32) {
    let mut widths = stroke_side_widths(&stroke.thickness);
    widths[stroke_side_index(side)] = width;
    stroke.thickness = sided_stroke_thickness(widths);
}
