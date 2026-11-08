//! Minimal HTML/CSS export for document-level motion declarations.

use super::{node_children, node_hidden, root_nodes};
use crate::{fmt_num, html_escape};
use jian_core::motion::compile;
use jian_ops_schema::motion::{
    Easing, Iterations, MotionTrigger, NodeAnimation, NodeAnimationFillMode,
};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use op_editor_core::PenNodeExt;
use serde_json::Value;
use std::collections::BTreeMap;

/// Render all motion CSS for the HTML target. An empty string means the
/// document has no visible node with a transition or animation declaration.
pub(super) fn render(doc: &PenDocument) -> String {
    let mut animated = Vec::new();
    for node in root_nodes(doc) {
        collect_animated_nodes(node, &mut animated);
    }
    if animated.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for node in &animated {
        if let Some(animations) = node.motion_declarations().1 {
            for (index, animation) in animations.iter().enumerate() {
                render_keyframes(&mut out, node.id_str(), index, animation);
            }
        }
    }
    if doc.motion == Some(jian_ops_schema::motion::MotionPreference::Reduced) {
        out.push_str("/* Authored motion preference: reduced */\n");
        render_reduced_block(&mut out, &animated, false);
    }
    render_reduced_block(&mut out, &animated, true);
    out
}

fn collect_animated_nodes<'a>(node: &'a PenNode, out: &mut Vec<&'a PenNode>) {
    if node_hidden(node) {
        return;
    }
    let (transition, animations) = node.motion_declarations();
    if transition.is_some() || animations.is_some_and(|items| !items.is_empty()) {
        out.push(node);
    }
    for child in node_children(node) {
        collect_animated_nodes(child, out);
    }
}

/// Keyframes that replay Preview exactly. Preview eases the whole iteration
/// and then interpolates the authored keyframes linearly, while CSS eases
/// each keyframe interval, so the curve compiler cuts the iteration into
/// intervals that each carry the exact slice of Preview's curve.
fn render_keyframes(out: &mut String, node_id: &str, index: usize, animation: &NodeAnimation) {
    let name = animation_name(node_id, index);
    let compiled = compile(animation);
    out.push_str(&format!("@keyframes {name} {{\n"));
    if compiled.degraded {
        out.push_str(
            "  /* Approximated with 60 linear steps: the exact curve needs more than 64 segments. */\n",
        );
    }
    for segment in &compiled.segments {
        push_keyframe(out, segment.t0, &segment.from, Some(segment.bezier));
    }
    if let Some(last) = compiled.segments.last() {
        push_keyframe(out, last.t1, &last.to, None);
    }
    out.push_str("}\n");
}

fn push_keyframe(
    out: &mut String,
    at: f32,
    values: &BTreeMap<String, Value>,
    timing: Option<[f32; 4]>,
) {
    out.push_str(&format!("  {}% {{\n", rounded(f64::from(at) * 100.0)));
    for (property, value) in keyframe_declarations(&rounded_values(values)) {
        out.push_str(&format!("    {property}: {value};\n"));
    }
    if let Some(bezier) = timing {
        out.push_str(&format!(
            "    animation-timing-function: {};\n",
            timing_function(bezier)
        ));
    }
    out.push_str("  }\n");
}

/// A compiled interval's timing function.
fn timing_function(bezier: [f32; 4]) -> String {
    if bezier == [0.0, 0.0, 1.0, 1.0] {
        return "linear".to_owned();
    }
    let [x1, y1, x2, y2] = bezier.map(|value| rounded(f64::from(value)));
    format!("cubic-bezier({x1},{y1},{x2},{y2})")
}

/// Four decimals: compiled offsets and sampled values carry f32 noise.
fn rounded(value: f64) -> String {
    let value = (value * 10_000.0).round() / 10_000.0;
    fmt_num(if value == 0.0 { 0.0 } else { value })
}

fn rounded_values(values: &BTreeMap<String, Value>) -> BTreeMap<String, Value> {
    values
        .iter()
        .map(|(property, value)| {
            let value = match value.as_f64() {
                Some(number) => {
                    serde_json::Number::from_f64((number * 10_000.0).round() / 10_000.0)
                        .map_or_else(|| value.clone(), Value::Number)
                }
                None => value.clone(),
            };
            (property.clone(), value)
        })
        .collect()
}

fn render_reduced_block(out: &mut String, nodes: &[&PenNode], media: bool) {
    if media {
        out.push_str("@media (prefers-reduced-motion: reduce) {\n");
    }
    for node in nodes {
        let indent = if media { "  " } else { "" };
        out.push_str(&format!(
            "{indent}{} {{\n{indent}  animation: none;\n{indent}  transition: none;\n{indent}}}\n",
            selector(node.id_str())
        ));
    }
    if media {
        out.push_str("}\n");
    }
}

/// Inline CSS declarations attached to the generated element itself.
pub(super) fn inline_style(node: &PenNode) -> String {
    let (transition, animations) = node.motion_declarations();
    let mut style = String::new();
    if let Some(transition) = transition {
        let properties = transition
            .properties
            .as_ref()
            .map(|properties| properties.join(" "))
            .unwrap_or_else(|| "all".to_string());
        style.push_str(&format!(
            ";transition:{properties} {}ms {}",
            transition.duration_ms,
            easing_to_css(&transition.easing)
        ));
    }
    if let Some(animations) = animations.filter(|items| !items.is_empty()) {
        let declarations = animations
            .iter()
            .enumerate()
            .map(|(index, animation)| {
                // The curve lives in the keyframes' timing functions.
                format!(
                    "{} {}ms linear {}ms {} {}",
                    animation_name(node.id_str(), index),
                    animation.duration_ms,
                    animation.delay_ms,
                    iteration_count(&animation.iterations),
                    fill_mode(animation.fill_mode)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        style.push_str(&format!(";animation:{declarations}"));
        if animations
            .iter()
            .any(|animation| animation.trigger == MotionTrigger::InView)
        {
            style.push_str(";animation-timeline:view();animation-range:entry 0% entry 40%");
        }
    }
    style
}

/// Stable selector attribute for an element with a motion declaration.
pub(super) fn data_attribute(node: &PenNode) -> String {
    let (transition, animations) = node.motion_declarations();
    if transition.is_none() && animations.is_none_or(Vec::is_empty) {
        return String::new();
    }
    format!(
        " data-op-motion-node=\"{}\"",
        html_escape(&css_identifier(node.id_str()))
    )
}

fn keyframe_declarations(values: &BTreeMap<String, Value>) -> Vec<(String, String)> {
    let mut declarations = Vec::new();
    let mut transforms = TransformParts::default();
    for (property, value) in values {
        match property.as_str() {
            "translateX" => transforms.x = Some(css_number(value, "px")),
            "translateY" => transforms.y = Some(css_number(value, "px")),
            "scaleX" => transforms.scale_x = Some(css_number(value, "")),
            "scaleY" => transforms.scale_y = Some(css_number(value, "")),
            "rotation" => transforms.rotation = Some(css_number(value, "deg")),
            "opacity" => declarations.push(("opacity".to_string(), css_number(value, ""))),
            "fill" => declarations.push(("background-color".to_string(), css_value(value))),
            "stroke" => declarations.push(("border-color".to_string(), css_value(value))),
            "cornerRadius" => {
                declarations.push(("border-radius".to_string(), css_number(value, "px")))
            }
            "x" => declarations.push(("left".to_string(), css_number(value, "px"))),
            "y" => declarations.push(("top".to_string(), css_number(value, "px"))),
            "width" => declarations.push(("width".to_string(), css_number(value, "px"))),
            "height" => declarations.push(("height".to_string(), css_number(value, "px"))),
            _ => {}
        }
    }
    if let Some(transform) = transforms.css() {
        declarations.push(("transform".to_string(), transform));
    }
    declarations
}

#[derive(Default)]
struct TransformParts {
    x: Option<String>,
    y: Option<String>,
    scale_x: Option<String>,
    scale_y: Option<String>,
    rotation: Option<String>,
}

impl TransformParts {
    fn css(&self) -> Option<String> {
        let mut parts = Vec::new();
        if self.x.is_some() || self.y.is_some() {
            parts.push(format!(
                "translate({}, {})",
                self.x.as_deref().unwrap_or("0px"),
                self.y.as_deref().unwrap_or("0px")
            ));
        }
        if self.scale_x.is_some() || self.scale_y.is_some() {
            parts.push(format!(
                "scale({}, {})",
                self.scale_x.as_deref().unwrap_or("1"),
                self.scale_y.as_deref().unwrap_or("1")
            ));
        }
        if let Some(rotation) = &self.rotation {
            parts.push(format!("rotate({rotation})"));
        }
        (!parts.is_empty()).then(|| parts.join(" "))
    }
}

fn css_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => String::new(),
    }
}

fn css_number(value: &Value, suffix: &str) -> String {
    match value.as_f64() {
        Some(number) => format!("{}{suffix}", fmt_num(number)),
        None => css_value(value),
    }
}

fn easing_to_css(easing: &Easing) -> String {
    match easing {
        Easing::Linear => "linear".to_string(),
        Easing::Ease => "ease".to_string(),
        Easing::EaseIn => "ease-in".to_string(),
        Easing::EaseOut => "ease-out".to_string(),
        Easing::EaseInOut => "ease-in-out".to_string(),
        Easing::Standard | Easing::Emphasized => "cubic-bezier(0.2,0,0,1)".to_string(),
        Easing::EmphasizedDecelerate => "cubic-bezier(0.05,0.7,0.1,1)".to_string(),
        Easing::EmphasizedAccelerate => "cubic-bezier(0.3,0,0.8,0.15)".to_string(),
        Easing::CubicBezier(x1, y1, x2, y2) => format!("cubic-bezier({},{},{},{})", x1, y1, x2, y2),
    }
}

/// `animation-iteration-count`: the authored count, or `infinite`. Preview
/// plays at least one pass (an authored `0` plays once), so the export does
/// too.
fn iteration_count(iterations: &Iterations) -> String {
    match iterations {
        Iterations::Count(count) => (*count).max(1).to_string(),
        Iterations::Infinite => "infinite".to_owned(),
    }
}

/// `animation-fill-mode` matching Preview: the first keyframe always holds
/// through the delay, and only `forwards` keeps the last one afterwards.
fn fill_mode(fill: NodeAnimationFillMode) -> &'static str {
    match fill {
        NodeAnimationFillMode::Forwards => "both",
        NodeAnimationFillMode::None => "backwards",
    }
}

fn animation_name(node_id: &str, index: usize) -> String {
    format!("op-{}-{index}", css_identifier(node_id))
}

fn selector(node_id: &str) -> String {
    format!("[data-op-motion-node=\"{}\"]", css_identifier(node_id))
}

fn css_identifier(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    if sanitized.is_empty() {
        "node".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iteration_counts_cover_infinite_motion() {
        assert_eq!(iteration_count(&Iterations::Count(3)), "3");
        assert_eq!(iteration_count(&Iterations::Count(0)), "1");
        assert_eq!(iteration_count(&Iterations::Infinite), "infinite");
    }

    #[test]
    fn fill_modes_hold_the_first_keyframe_through_the_delay() {
        assert_eq!(fill_mode(NodeAnimationFillMode::Forwards), "both");
        assert_eq!(fill_mode(NodeAnimationFillMode::None), "backwards");
    }
}
