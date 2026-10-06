//! Minimal HTML/CSS export for document-level motion declarations.

use super::{node_children, node_hidden, root_nodes};
use crate::{fmt_num, html_escape};
use jian_ops_schema::motion::{Easing, MotionTrigger, NodeAnimation};
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

fn render_keyframes(out: &mut String, node_id: &str, index: usize, animation: &NodeAnimation) {
    let name = animation_name(node_id, index);
    out.push_str(&format!("@keyframes {name} {{\n"));
    for keyframe in &animation.keyframes {
        out.push_str(&format!(
            "  {}% {{\n",
            fmt_num(f64::from(keyframe.offset) * 100.0)
        ));
        for (property, value) in keyframe_declarations(&keyframe.values) {
            out.push_str(&format!("    {property}: {value};\n"));
        }
        out.push_str("  }\n");
    }
    out.push_str("}\n");
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
                format!(
                    "{} {}ms {} {}ms {} both",
                    animation_name(node.id_str(), index),
                    animation.duration_ms,
                    easing_to_css(&animation.easing),
                    animation.delay_ms,
                    animation.iterations
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
            "opacity" => declarations.push(("opacity".to_string(), css_value(value))),
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
