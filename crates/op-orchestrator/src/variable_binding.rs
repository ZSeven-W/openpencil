//! Generated-node design-variable binding.
//!
//! Sub-agent prompts ask models to prefer `$color-*` refs, but raw LLM
//! output can still carry literal hex values. This pass persists refs in
//! the generated subtree before `InsertSubtree`, so the property panel and
//! saved `.op` file both see the authored `$variable` token.

use std::collections::HashMap;

use jian_ops_schema::node::text::TextContent;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenEffect, PenFill, PenStroke};
use jian_ops_schema::variable::VariableKind;
use op_editor_core::{EditorState, PenNodeExt};

type ColorKey = (u8, u8, u8, u8);
const NEAR_COLOR_MAX_DISTANCE: f64 = 18.0;

struct ColorRefs {
    exact: HashMap<ColorKey, String>,
    candidates: Vec<ColorCandidate>,
}

struct ColorCandidate {
    key: ColorKey,
    reference: String,
}

pub(crate) fn bind_generated_color_variables(nodes: &mut [PenNode], state: &EditorState) {
    let refs = color_refs(state);
    if refs.candidates.is_empty() {
        return;
    }
    for node in nodes {
        bind_node(node, &refs);
    }
}

fn color_refs(state: &EditorState) -> ColorRefs {
    let mut exact = HashMap::new();
    let mut candidates = Vec::new();
    let Some(variables) = state.doc.variables.as_ref() else {
        return ColorRefs { exact, candidates };
    };
    for (name, def) in variables {
        if !matches!(def.kind, VariableKind::Color) {
            continue;
        }
        let Some(hex) = state.resolve_color_variable_hex(name) else {
            continue;
        };
        let Some(key) = color_key(&hex) else {
            continue;
        };
        let reference = format!("${name}");
        exact.entry(key).or_insert_with(|| reference.clone());
        candidates.push(ColorCandidate { key, reference });
    }
    ColorRefs { exact, candidates }
}

fn bind_node(node: &mut PenNode, refs: &ColorRefs) {
    if let Some(fills) = node_fills_mut(node) {
        bind_fills(fills, refs);
    }
    if let Some(stroke) = node_stroke_mut(node).and_then(Option::as_mut) {
        if let Some(fills) = stroke.fill.as_mut() {
            bind_fills(fills, refs);
        }
    }
    if let Some(effects) = node_effects_mut(node) {
        for effect in effects {
            if let PenEffect::Shadow(body) = effect {
                bind_color_string(&mut body.color, refs);
            }
        }
    }
    if let PenNode::Text(text) = node {
        if let TextContent::Styled(segments) = &mut text.content {
            for segment in segments {
                if let Some(fill) = segment.fill.as_mut() {
                    bind_color_string(fill, refs);
                }
            }
        }
    }
    if let Some(children) = node.children_mut() {
        for child in children {
            bind_node(child, refs);
        }
    }
}

fn bind_fills(fills: &mut [PenFill], refs: &ColorRefs) {
    for fill in fills {
        match fill {
            PenFill::Solid(body) => bind_color_string(&mut body.color, refs),
            PenFill::LinearGradient(body) => {
                for stop in &mut body.stops {
                    bind_color_string(&mut stop.color, refs);
                }
            }
            PenFill::RadialGradient(body) => {
                for stop in &mut body.stops {
                    bind_color_string(&mut stop.color, refs);
                }
            }
            PenFill::Image(_) => {}
        }
    }
}

fn bind_color_string(color: &mut String, refs: &ColorRefs) {
    if color.trim_start().starts_with('$') {
        return;
    }
    let Some(key) = color_key(color) else {
        return;
    };
    if let Some(reference) = refs.exact.get(&key).or_else(|| nearest_ref(key, refs)) {
        *color = reference.clone();
    }
}

fn nearest_ref(key: ColorKey, refs: &ColorRefs) -> Option<&String> {
    refs.candidates
        .iter()
        .map(|candidate| (color_distance(key, candidate.key), &candidate.reference))
        .filter(|(distance, _)| *distance <= NEAR_COLOR_MAX_DISTANCE)
        .min_by(|(a, _), (b, _)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, reference)| reference)
}

fn color_distance(a: ColorKey, b: ColorKey) -> f64 {
    let dr = a.0 as f64 - b.0 as f64;
    let dg = a.1 as f64 - b.1 as f64;
    let db = a.2 as f64 - b.2 as f64;
    let da = a.3 as f64 - b.3 as f64;
    (dr * dr + dg * dg + db * db + da * da).sqrt()
}

fn color_key(hex: &str) -> Option<ColorKey> {
    let (r, g, b) = op_editor_core::parse_hex_rgb(hex)?;
    let a = op_editor_core::parse_hex_alpha(hex);
    Some((channel(r), channel(g), channel(b), channel(a)))
}

fn channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn node_fills_mut(node: &mut PenNode) -> Option<&mut Vec<PenFill>> {
    match node {
        PenNode::Frame(n) => n.container.fill.as_mut(),
        PenNode::Group(n) => n.container.fill.as_mut(),
        PenNode::Rectangle(n) => n.container.fill.as_mut(),
        PenNode::Ellipse(n) => n.fill.as_mut(),
        PenNode::Polygon(n) => n.fill.as_mut(),
        PenNode::Path(n) => n.fill.as_mut(),
        PenNode::Text(n) => n.fill.as_mut(),
        PenNode::TextInput(n) => n.fill.as_mut(),
        PenNode::TextArea(n) => n.fill.as_mut(),
        PenNode::Select(n) => n.fill.as_mut(),
        PenNode::Switch(n) => n.fill.as_mut(),
        PenNode::Checkbox(n) => n.fill.as_mut(),
        PenNode::Slider(n) => n.fill.as_mut(),
        PenNode::RadioGroup(n) => n.fill.as_mut(),
        PenNode::NumberInput(n) => n.fill.as_mut(),
        PenNode::Progress(n) => n.fill.as_mut(),
        PenNode::Tabs(n) => n.fill.as_mut(),
        PenNode::IconFont(n) => n.fill.as_mut(),
        PenNode::Line(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

fn node_stroke_mut(node: &mut PenNode) -> Option<&mut Option<PenStroke>> {
    match node {
        PenNode::Frame(n) => Some(&mut n.container.stroke),
        PenNode::Group(n) => Some(&mut n.container.stroke),
        PenNode::Rectangle(n) => Some(&mut n.container.stroke),
        PenNode::Ellipse(n) => Some(&mut n.stroke),
        PenNode::Polygon(n) => Some(&mut n.stroke),
        PenNode::Path(n) => Some(&mut n.stroke),
        PenNode::Line(n) => Some(&mut n.stroke),
        PenNode::TextInput(n) => Some(&mut n.stroke),
        PenNode::TextArea(n) => Some(&mut n.stroke),
        PenNode::Select(n) => Some(&mut n.stroke),
        PenNode::Switch(n) => Some(&mut n.stroke),
        PenNode::Checkbox(n) => Some(&mut n.stroke),
        PenNode::Slider(n) => Some(&mut n.stroke),
        PenNode::RadioGroup(n) => Some(&mut n.stroke),
        PenNode::NumberInput(n) => Some(&mut n.stroke),
        PenNode::Progress(n) => Some(&mut n.stroke),
        PenNode::Tabs(n) => Some(&mut n.stroke),
        PenNode::IconFont(n) => Some(&mut n.stroke),
        PenNode::Text(_) | PenNode::Image(_) | PenNode::Ref(_) => None,
    }
}

fn node_effects_mut(node: &mut PenNode) -> Option<&mut Vec<PenEffect>> {
    match node {
        PenNode::Frame(n) => n.container.effects.as_mut(),
        PenNode::Group(n) => n.container.effects.as_mut(),
        PenNode::Rectangle(n) => n.container.effects.as_mut(),
        PenNode::Ellipse(n) => n.effects.as_mut(),
        PenNode::Polygon(n) => n.effects.as_mut(),
        PenNode::Path(n) => n.effects.as_mut(),
        PenNode::Line(n) => n.effects.as_mut(),
        PenNode::Text(n) => n.effects.as_mut(),
        PenNode::TextInput(n) => n.effects.as_mut(),
        PenNode::TextArea(n) => n.effects.as_mut(),
        PenNode::Select(n) => n.effects.as_mut(),
        PenNode::Switch(n) => n.effects.as_mut(),
        PenNode::Checkbox(n) => n.effects.as_mut(),
        PenNode::Slider(n) => n.effects.as_mut(),
        PenNode::RadioGroup(n) => n.effects.as_mut(),
        PenNode::NumberInput(n) => n.effects.as_mut(),
        PenNode::Progress(n) => n.effects.as_mut(),
        PenNode::Tabs(n) => n.effects.as_mut(),
        PenNode::Image(n) => n.effects.as_mut(),
        PenNode::IconFont(_) | PenNode::Ref(_) => None,
    }
}
