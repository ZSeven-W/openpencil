//! Minimal CSS export for document-level motion declarations, shared by the
//! HTML, Vue and Svelte targets.

use super::{node_children, node_hidden, root_nodes};
use crate::{fmt_num, html_escape};
use jian_core::action::services::Easing as RuntimeEasing;
use jian_core::motion::{as_cubic, compile, CubicPiece};
use jian_ops_schema::motion::{
    Easing, Iterations, MotionTrigger, NodeAnimation, NodeAnimationFillMode,
};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::PenDocument;
use op_editor_core::PenNodeExt;
use serde_json::Value;
use std::collections::BTreeMap;

/// Where the motion CSS lands. Component frameworks rename keyframes and
/// scope selectors inside their own `<style>`, while the inline `animation:`
/// shorthand and the `data-op-motion-node` attributes keep the plain names,
/// so each scope opts its rules out of that renaming the way it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MotionCssScope {
    /// A document-level stylesheet, which renames nothing: the HTML page's
    /// `<style>` and the unscoped `<style>` block of a Vue component.
    Document,
    /// A Svelte component `<style>`, which hashes keyframe names and scopes
    /// selectors unless they opt out with `-global-` and `:global(...)`.
    SvelteComponent,
}

/// Render all motion CSS for a document-level stylesheet. An empty string
/// means the document has no visible node with a transition or animation
/// declaration.
pub(super) fn render(doc: &PenDocument) -> String {
    render_in(doc, MotionCssScope::Document)
}

/// Render all motion CSS for the given stylesheet scope, keeping keyframe
/// names and selectors equal to the ones the inline markup references.
pub(super) fn render_in(doc: &PenDocument, scope: MotionCssScope) -> String {
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
                render_keyframes(&mut out, node.id_str(), index, animation, scope);
            }
        }
    }
    if doc.motion == Some(jian_ops_schema::motion::MotionPreference::Reduced) {
        out.push_str("/* Authored motion preference: reduced */\n");
        render_reduced_block(&mut out, &animated, false, scope);
    }
    render_reduced_block(&mut out, &animated, true, scope);
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
fn render_keyframes(
    out: &mut String,
    node_id: &str,
    index: usize,
    animation: &NodeAnimation,
    scope: MotionCssScope,
) {
    let name = animation_name(node_id, index);
    let compiled = compile(animation);
    // Svelte drops the `-global-` prefix and leaves the name unhashed.
    let global = match scope {
        MotionCssScope::Document => "",
        MotionCssScope::SvelteComponent => "-global-",
    };
    out.push_str(&format!("@keyframes {global}{name} {{\n"));
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

/// Zero-length, undelayed, single-pass animations end the moment they start
/// and then fill by their own fill mode.
const REDUCED_DECLARATIONS: [&str; 4] = [
    "animation-duration: 0s !important;",
    "animation-delay: 0s !important;",
    "animation-iteration-count: 1 !important;",
    "transition: none !important;",
];

/// Reduced motion the way Preview does it: every animation lands at once,
/// so one with `fillMode: forwards` shows its last keyframe and one with
/// `none` (exported as `backwards`) shows the authored look; transitions
/// switch instantly. The declarations are `!important` because the
/// animation itself is an inline `style`, which outranks any stylesheet rule
/// that is not.
fn render_reduced_block(out: &mut String, nodes: &[&PenNode], media: bool, scope: MotionCssScope) {
    if media {
        out.push_str("@media (prefers-reduced-motion: reduce) {\n");
    }
    let indent = if media { "  " } else { "" };
    for node in nodes {
        out.push_str(&format!("{indent}{} {{\n", selector(node.id_str(), scope)));
        for declaration in REDUCED_DECLARATIONS {
            out.push_str(&format!("{indent}  {declaration}\n"));
        }
        out.push_str(&format!("{indent}}}\n"));
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
            transition_timing(&transition.easing)
        ));
    }
    if let Some(animations) = animations.filter(|items| !items.is_empty()) {
        let declarations = animations
            .iter()
            .enumerate()
            // Manual motion plays only through a play_motion interaction,
            // which the export does not carry yet; like Preview before such
            // an interaction, the node keeps its authored look.
            .filter(|(_, animation)| animation.trigger != MotionTrigger::Manual)
            .map(|(index, animation)| {
                // The curve lives in the keyframes' timing functions. In-view
                // motion waits on its first keyframe for the observer script.
                format!(
                    "{} {}ms linear {}ms {} {}{}",
                    animation_name(node.id_str(), index),
                    animation.duration_ms,
                    animation.delay_ms,
                    iteration_count(&animation.iterations),
                    fill_mode(animation.fill_mode),
                    if animation.trigger == MotionTrigger::InView {
                        " paused"
                    } else {
                        ""
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        if !declarations.is_empty() {
            style.push_str(&format!(";animation:{declarations}"));
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
    let mut attributes = format!(
        " data-op-motion-node=\"{}\"",
        html_escape(&css_identifier(node.id_str()))
    );
    let in_view: Vec<String> = animations
        .into_iter()
        .flatten()
        .enumerate()
        .filter(|(_, animation)| animation.trigger == MotionTrigger::InView)
        .map(|(index, animation)| {
            let name = animation_name(node.id_str(), index);
            if animation.once {
                format!("{name}:once")
            } else {
                name
            }
        })
        .collect();
    if !in_view.is_empty() {
        attributes.push_str(&format!(" data-op-in-view=\"{}\"", in_view.join(" ")));
    }
    attributes
}

/// Whether any visible node starts an animation when it scrolls into view.
pub(super) fn has_in_view_motion(doc: &PenDocument) -> bool {
    let mut animated = Vec::new();
    for node in root_nodes(doc) {
        collect_animated_nodes(node, &mut animated);
    }
    animated.iter().any(|node| {
        node.motion_declarations().1.is_some_and(|animations| {
            animations
                .iter()
                .any(|a| a.trigger == MotionTrigger::InView)
        })
    })
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

/// Preview's transition curve as a CSS easing function. Preview applies its
/// whole curve to the transition, clamped to [0, 1]: one cubic piece that
/// stays inside that range is an exact `cubic-bezier`, while `easeInOut`
/// (two pieces) and curves Preview clamps become a sampled `linear()`.
fn transition_timing(easing: &Easing) -> String {
    if matches!(easing, Easing::Linear) {
        return "linear".to_owned();
    }
    let pieces = as_cubic(RuntimeEasing::from(easing));
    let samples: Vec<f32> = (0..=TIMING_SAMPLES)
        .map(|step| curve_at(&pieces, step as f32 / TIMING_SAMPLES as f32))
        .collect();
    let inside = samples
        .iter()
        .all(|value| (-1e-6..=1.0 + 1e-6).contains(value));
    if let ([piece], true) = (pieces.as_slice(), inside) {
        let [x1, y1, x2, y2] = piece.control.map(|value| rounded(f64::from(value)));
        return format!("cubic-bezier({x1},{y1},{x2},{y2})");
    }
    let points: Vec<String> = samples
        .iter()
        .map(|value| rounded(f64::from(value.clamp(0.0, 1.0))))
        .collect();
    format!("linear({})", points.join(","))
}

/// Evenly spaced points for a sampled `linear()` curve: 41 points keep the
/// worst case (easeInOut, overshoot curves) well under half a percent.
const TIMING_SAMPLES: usize = 40;

fn curve_at(pieces: &[CubicPiece], progress: f32) -> f32 {
    pieces
        .iter()
        .find(|piece| progress <= piece.x1)
        .or(pieces.last())
        .map_or(progress, |piece| piece.eval(progress))
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

fn selector(node_id: &str, scope: MotionCssScope) -> String {
    let selector = format!("[data-op-motion-node=\"{}\"]", css_identifier(node_id));
    match scope {
        MotionCssScope::Document => selector,
        MotionCssScope::SvelteComponent => format!(":global({selector})"),
    }
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
    fn transitions_time_with_preview_curves() {
        assert_eq!(transition_timing(&Easing::Linear), "linear");
        assert_eq!(
            transition_timing(&Easing::Ease),
            "cubic-bezier(0.3333,0,0.6667,1)"
        );
        assert_eq!(
            transition_timing(&Easing::Standard),
            "cubic-bezier(0.2,0,0,1)"
        );
        let in_out = transition_timing(&Easing::EaseInOut);
        assert!(in_out.starts_with("linear(0,"), "{in_out}");
        assert_eq!(in_out.matches(',').count(), TIMING_SAMPLES, "{in_out}");
        // Preview clamps an overshooting curve at 1.
        let overshoot = transition_timing(&Easing::CubicBezier(0.34, 1.56, 0.64, 1.0));
        assert!(overshoot.starts_with("linear("), "{overshoot}");
        assert!(!overshoot.contains("1."), "{overshoot}");
    }

    #[test]
    fn fill_modes_hold_the_first_keyframe_through_the_delay() {
        assert_eq!(fill_mode(NodeAnimationFillMode::Forwards), "both");
        assert_eq!(fill_mode(NodeAnimationFillMode::None), "backwards");
    }
}
