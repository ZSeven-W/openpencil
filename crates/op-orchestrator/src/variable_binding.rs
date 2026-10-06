//! Generated-node design-variable binding.
//!
//! Sub-agent prompts ask models to prefer `$color-*` refs, but raw LLM
//! output can still carry literal hex values. This pass persists refs in
//! the generated subtree before `InsertSubtree`, so the property panel and
//! saved `.op` file both see the authored `$variable` token.
//!
//! Binding is **slot-aware**: a colour is matched only against variables whose
//! family can legitimately fill the slot it was found in. Matching on colour
//! distance alone silently rewrites a design's semantics — measured on
//! `0808-gm-1.op`, where a 36px headline's near-white literal happened to equal
//! `$--border`'s active-theme value and every card and section title in the
//! document came back bound to the BORDER token. Nothing looked wrong (the two
//! resolve to the same hex today), but the design was one theme flip — or one
//! palette repair — away from headlines rendered in hairline grey. See
//! [`slot_accepts`].

use jian_ops_schema::node::text::TextContent;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenEffect, PenFill, PenStroke};
use jian_ops_schema::variable::VariableKind;
use op_editor_core::{EditorState, PenNodeExt};

type ColorKey = (u8, u8, u8, u8);
const NEAR_COLOR_MAX_DISTANCE: f64 = 18.0;

/// Where in a node a colour was found — i.e. what the colour is FOR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorSlot {
    /// Glyph colour: a text node's fill, a styled span, an icon glyph.
    Text,
    /// A painted surface: a container / control's own fill.
    Surface,
    /// A hairline or outline.
    Stroke,
    /// Neither — shadow colours and anything else with no slot semantics.
    Any,
}

/// What a design variable is FOR, read from its name. The naming convention is
/// the only semantic signal a variable carries; its value is just a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorFamily {
    Text,
    Border,
    Surface,
    /// Accent / destructive / success / chart-N … — a meaning-bearing colour
    /// that is legitimately used as text, fill OR stroke, so it is never
    /// blocked by slot.
    Semantic,
}

fn family_of(name: &str) -> ColorFamily {
    let name = name.to_ascii_lowercase();
    // Text wins over the surface words on purpose: `--color-error-foreground` is a
    // text token that happens to name a state, not a state background.
    if name.contains("text") || name.contains("foreground") {
        return ColorFamily::Text;
    }
    if name.contains("border")
        || name.contains("outline")
        || name.contains("divider")
        || name.contains("input")
        || name.contains("ring")
    {
        return ColorFamily::Border;
    }
    if [
        "surface",
        "bg",
        "background",
        "card",
        "panel",
        "chip",
        "scrim",
        // shadcn slot names (B1): the gray wells, the sidebar ground, and
        // the status colours all paint surfaces.
        "muted",
        "secondary",
        "popover",
        "sidebar",
        "accent",
        "success",
        "warning",
        "error",
        "info",
    ]
    .iter()
    .any(|word| name.contains(word))
    {
        return ColorFamily::Surface;
    }
    ColorFamily::Semantic
}

/// May a variable of `family` bind into `slot`?
///
/// Deliberately a DENYLIST of category errors rather than a same-family
/// allowlist: an accent, a destructive red or a chart colour is legitimately
/// a glyph colour, a fill and a stroke, and requiring same-family would strip
/// theming from every accent-coloured element in a design. Only the pairs that
/// are structurally impossible are refused, and a refusal leaves the literal
/// hex in place — an unthemed but CORRECT colour beats a themed wrong one.
fn slot_accepts(slot: ColorSlot, family: ColorFamily) -> bool {
    match (slot, family) {
        // A glyph is never a hairline or a page surface.
        (ColorSlot::Text, ColorFamily::Border | ColorFamily::Surface) => false,
        // A surface / border is never a glyph colour. (`role_post_pass`'s
        // surface-discipline pass repairs this downstream by rewriting the
        // fill to `$--muted`; refusing the bind here is the same
        // judgement made at the source, and keeps the authored colour.)
        (ColorSlot::Surface | ColorSlot::Stroke, ColorFamily::Text) => false,
        _ => true,
    }
}

struct ColorRefs {
    candidates: Vec<ColorCandidate>,
}

struct ColorCandidate {
    key: ColorKey,
    family: ColorFamily,
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
    let mut candidates = Vec::new();
    let Some(variables) = state.doc.variables.as_ref() else {
        return ColorRefs { candidates };
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
        candidates.push(ColorCandidate {
            key,
            family: family_of(name),
            reference: format!("${name}"),
        });
    }
    ColorRefs { candidates }
}

/// Which slot a node's own `fill` occupies. Text and icon glyphs paint
/// foreground; every other node's fill paints a surface.
fn fill_slot(node: &PenNode) -> ColorSlot {
    match node {
        PenNode::Text(_) | PenNode::IconFont(_) => ColorSlot::Text,
        _ => ColorSlot::Surface,
    }
}

fn bind_node(node: &mut PenNode, refs: &ColorRefs) {
    let slot = fill_slot(node);
    if let Some(fills) = node_fills_mut(node) {
        bind_fills(fills, refs, slot);
    }
    if let Some(stroke) = node_stroke_mut(node).and_then(Option::as_mut) {
        if let Some(fills) = stroke.fill.as_mut() {
            bind_fills(fills, refs, ColorSlot::Stroke);
        }
    }
    if let Some(effects) = node_effects_mut(node) {
        for effect in effects {
            if let PenEffect::Shadow(body) = effect {
                bind_color_string(&mut body.color, refs, ColorSlot::Any);
            }
        }
    }
    if let PenNode::Text(text) = node {
        if let TextContent::Styled(segments) = &mut text.content {
            for segment in segments {
                if let Some(fill) = segment.fill.as_mut() {
                    bind_color_string(fill, refs, ColorSlot::Text);
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

fn bind_fills(fills: &mut [PenFill], refs: &ColorRefs, slot: ColorSlot) {
    for fill in fills {
        match fill {
            PenFill::Solid(body) => bind_color_string(&mut body.color, refs, slot),
            PenFill::LinearGradient(body) => {
                for stop in &mut body.stops {
                    bind_color_string(&mut stop.color, refs, slot);
                }
            }
            PenFill::RadialGradient(body) => {
                for stop in &mut body.stops {
                    bind_color_string(&mut stop.color, refs, slot);
                }
            }
            PenFill::MeshGradient(body) => {
                for stop in &mut body.stops {
                    bind_color_string(&mut stop.color, refs, slot);
                }
            }
            PenFill::Shader(body) => {
                // `color`-typed uniforms can carry `$ref` colours — bind
                // them like gradient stops. `float` / `vec` uniforms and
                // the SkSL source itself are left untouched.
                if let Some(uniforms) = &mut body.uniforms {
                    for value in uniforms.values_mut() {
                        if let jian_ops_schema::style::ShaderUniformValue::Color(c) = value {
                            bind_color_string(c, refs, slot);
                        }
                    }
                }
            }
            PenFill::Image(_) => {}
        }
    }
}

fn bind_color_string(color: &mut String, refs: &ColorRefs, slot: ColorSlot) {
    if color.trim_start().starts_with('$') {
        return;
    }
    let Some(key) = color_key(color) else {
        return;
    };
    if let Some(reference) = nearest_ref(key, refs, slot) {
        *color = reference.clone();
    }
}

/// Closest slot-compatible variable within [`NEAR_COLOR_MAX_DISTANCE`].
///
/// An exact match scores distance 0 and therefore always wins, and ties keep
/// the first candidate in variable-name order — the same resolution the
/// previous exact-map-then-nearest lookup produced.
fn nearest_ref(key: ColorKey, refs: &ColorRefs, slot: ColorSlot) -> Option<&String> {
    refs.candidates
        .iter()
        .filter(|candidate| slot_accepts(slot, candidate.family))
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

#[cfg(test)]
#[path = "variable_binding_tests.rs"]
mod tests;
