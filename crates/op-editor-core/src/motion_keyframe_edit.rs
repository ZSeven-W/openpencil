//! Inspector edits of one keyframe's values and of an animation's
//! cubic-bezier curve.
//!
//! The keyframe-values mode lists the properties a keyframe holds and
//! adds only properties from the compositor whitelist
//! (`P1_MOTION_PROPERTIES`), so an edit never makes a node relayout. Each
//! function returns the edited copy of the animation and leaves every
//! other keyframe as it was.

use crate::motion_timeline_edit::sample_values;
use crate::pen_node_ext::PenNodeExt;
use jian_core::action::services::Easing as RuntimeEasing;
use jian_core::motion::as_cubic;
use jian_ops_schema::motion::{Easing, NodeAnimation, P1_MOTION_PROPERTIES};
use jian_ops_schema::node::PenNode;
use serde_json::Value;

/// The colour-valued whitelist properties (hex strings); the rest are
/// numbers.
const COLOR_PROPERTIES: [&str; 2] = ["fill", "stroke"];

/// Whitelisted properties keyframe `keyframe` does not hold yet, in
/// whitelist order.
pub fn addable_properties(animation: &NodeAnimation, keyframe: usize) -> Vec<&'static str> {
    let Some(frame) = animation.keyframes.get(keyframe) else {
        return Vec::new();
    };
    P1_MOTION_PROPERTIES
        .iter()
        .copied()
        .filter(|property| !frame.values.contains_key(*property))
        .collect()
}

/// Add `property` to keyframe `keyframe`. When other keyframes animate it
/// the new value is the one the curve already passes through there, so
/// adding it changes nothing on screen; otherwise it is the node's own
/// value (or the property's identity: opacity 1, scale 1, 0 elsewhere).
pub fn add_keyframe_value(
    animation: &NodeAnimation,
    keyframe: usize,
    property: &str,
    node: &PenNode,
) -> NodeAnimation {
    let mut edited = animation.clone();
    let Some(offset) = animation.keyframes.get(keyframe).map(|frame| frame.offset) else {
        return edited;
    };
    if !P1_MOTION_PROPERTIES.contains(&property) {
        return edited;
    }
    let value = sample_values(animation, offset)
        .remove(property)
        .unwrap_or_else(|| base_value(node, property));
    edited.keyframes[keyframe]
        .values
        .insert(property.to_owned(), value);
    edited
}

/// Set `property` of keyframe `keyframe` to `value`.
pub fn set_keyframe_value(
    animation: &NodeAnimation,
    keyframe: usize,
    property: &str,
    value: Value,
) -> NodeAnimation {
    let mut edited = animation.clone();
    if let Some(frame) = edited.keyframes.get_mut(keyframe) {
        frame.values.insert(property.to_owned(), value);
    }
    edited
}

/// Drop `property` from keyframe `keyframe`.
pub fn remove_keyframe_value(
    animation: &NodeAnimation,
    keyframe: usize,
    property: &str,
) -> NodeAnimation {
    let mut edited = animation.clone();
    if let Some(frame) = edited.keyframes.get_mut(keyframe) {
        frame.values.remove(property);
    }
    edited
}

/// Parse a typed value for `property`: a `#hex` colour for fill and
/// stroke, a number elsewhere. `None` drops the edit, like every other
/// unparsable panel field.
pub fn parse_keyframe_value(property: &str, draft: &str) -> Option<Value> {
    let draft = draft.trim();
    if COLOR_PROPERTIES.contains(&property) {
        let hex = draft.strip_prefix('#').unwrap_or(draft);
        let valid = matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit());
        return valid.then(|| Value::String(format!("#{hex}")));
    }
    let number = draft.parse::<f64>().ok().filter(|n| n.is_finite())?;
    serde_json::Number::from_f64(number).map(Value::Number)
}

/// A keyframe value as the Inspector shows it: numbers without a
/// trailing `.0`, strings unquoted.
pub fn format_keyframe_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => match number.as_f64() {
            Some(n) if n.fract() == 0.0 && n.abs() < 1e15 => format!("{}", n as i64),
            Some(n) => format!("{}", (n * 1000.0).round() / 1000.0),
            None => number.to_string(),
        },
        other => other.to_string(),
    }
}

/// The cubic-bezier control points the curve inputs show for `easing`.
/// Every named curve but ease-in-out is one cubic, shown exactly;
/// ease-in-out is two pieces, shown as its CSS keyword's single cubic.
pub fn bezier_controls(easing: &Easing) -> [f32; 4] {
    match as_cubic(RuntimeEasing::from(easing)).as_slice() {
        [piece] => piece.control,
        _ => [0.42, 0.0, 0.58, 1.0],
    }
}

/// `easing` as a cubic-bezier with control `slot` (x1, y1, x2, y2) set
/// to `value`. x stays in [0, 1] as CSS requires; y may overshoot.
pub fn with_bezier_control(easing: &Easing, slot: usize, value: f32) -> Easing {
    let mut control = bezier_controls(easing);
    if let Some(entry) = control.get_mut(slot) {
        *entry = if slot.is_multiple_of(2) {
            value.clamp(0.0, 1.0)
        } else {
            value.clamp(-5.0, 5.0)
        };
    }
    let [x1, y1, x2, y2] = control;
    Easing::CubicBezier(x1, y1, x2, y2)
}

/// The node's own value of a whitelisted property, the starting point a
/// newly keyed property animates from.
fn base_value(node: &PenNode, property: &str) -> Value {
    let base = node.base();
    match property {
        "opacity" => number(
            base.opacity
                .as_ref()
                .and_then(|opacity| opacity.as_number())
                .unwrap_or(1.0),
        ),
        "scaleX" | "scaleY" => number(1.0),
        "fill" => Value::String(
            crate::first_solid_fill_hex(node)
                .unwrap_or("#000000")
                .to_owned(),
        ),
        "stroke" => Value::String(
            crate::first_solid_stroke_hex(node)
                .unwrap_or("#000000")
                .to_owned(),
        ),
        _ => number(0.0),
    }
}

fn number(value: f64) -> Value {
    serde_json::Number::from_f64(value).map_or(Value::Null, Value::Number)
}

#[cfg(test)]
#[path = "motion_keyframe_edit_tests.rs"]
mod tests;
