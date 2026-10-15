//! Deterministic map placeholders for image slots.
//!
//! Maps are authored as image intent by the model, but they are not stock
//! photography. This module turns map intent into a small vector-like scene
//! before image targets are collected. The builder is pure and returns a
//! shallow patch, which preserves the original node's id and geometry.

use std::collections::HashMap;

use crate::image_fallback_policy::ResolvedRect;
use crate::role_defaults::Theme;
use jian_ops_schema::node::{base::NumberOrExpression, container::LayoutMode, PenNode};
use op_editor_core::PenNodeExt;
use serde_json::{json, Value};

const MAP_PLACEHOLDER_NAME_SUFFIX: &str = " (map placeholder)";
const MAP_PLACEHOLDER_WIDTH: f64 = 327.0;
const MAP_PLACEHOLDER_HEIGHT: f64 = 300.0;

/// One pure map-placeholder rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPlaceholderPatch {
    pub node_id: String,
    pub patch_json: String,
}

/// Build patches for every eligible map-intent node below `root`. Marked
/// placeholders are skipped so running cleanup twice is stable.
pub fn map_placeholder(
    root: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    theme: Theme,
) -> Vec<MapPlaceholderPatch> {
    let mut patches = Vec::new();
    collect_patches(root, rects, theme, false, false, &mut patches);
    patches
}

/// Minimum width for a frame/rectangle to count as a map slot. A model that
/// draws its own map names the inner blocks `map-park` / `map-water`; those
/// decorations must not each become a nested mini map.
const MIN_SHAPE_MAP_WIDTH: f64 = 200.0;

fn collect_patches(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    theme: Theme,
    inside_map_named: bool,
    sibling_layer_control: bool,
    patches: &mut Vec<MapPlaceholderPatch>,
) {
    if node
        .base()
        .name
        .as_deref()
        .is_some_and(|name| name.ends_with(MAP_PLACEHOLDER_NAME_SUFFIX))
    {
        return;
    }
    // Inside a container the model already named as a map (a hand-drawn
    // map), nothing below is a slot: the container itself was rejected as a
    // candidate because it carries texts/controls, and its children are its
    // decorations.
    // An explicit image slot is a slot wherever it sits; only shapes are
    // demoted to decorations inside a map-named container.
    if !inside_map_named || matches!(node, PenNode::Image(_)) {
        if let Some(patch) = map_patch(node, rects, theme, sibling_layer_control) {
            patches.push(patch);
            return;
        }
    }
    let map_named = inside_map_named || node.base().name.as_deref().is_some_and(has_map_name_word);
    if let Some(children) = node.children() {
        // A map image can sit behind authored controls in its owning viewport.
        // Only coordinate direct siblings in that absolute stack, not buttons
        // elsewhere on the page or in another map's viewport.
        let shared_viewport = matches!(node, PenNode::Frame(frame)
            if frame.container.layout == Some(LayoutMode::None))
            && children.iter().any(|child| {
                (!map_named || matches!(child, PenNode::Image(_)))
                    && eligible_map_dimensions(child, rects).is_some()
            });
        let has_layers = shared_viewport && children.iter().any(is_authored_layer_control);
        if has_layers {
            for control in children
                .iter()
                .filter(|child| is_authored_layer_control(child))
            {
                let patch = compact_layer_control_patch();
                let current = serde_json::to_value(control).unwrap_or(Value::Null);
                if patch
                    .as_object()
                    .unwrap()
                    .iter()
                    .any(|(key, value)| current[key] != *value)
                {
                    patches.push(MapPlaceholderPatch {
                        node_id: control.id_str().to_string(),
                        patch_json: patch.to_string(),
                    });
                }
            }
        }
        for child in children {
            collect_patches(child, rects, theme, map_named, has_layers, patches);
        }
    }
}

fn map_patch(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    theme: Theme,
    sibling_layer_control: bool,
) -> Option<MapPlaceholderPatch> {
    let (width, height) = eligible_map_dimensions(node, rects)?;
    let patch =
        build_map_placeholder_with_controls(node, width, height, theme, sibling_layer_control);
    Some(MapPlaceholderPatch {
        node_id: node.id_str().to_string(),
        patch_json: patch.to_string(),
    })
}

fn eligible_map_dimensions(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
) -> Option<(f64, f64)> {
    if !is_map_placeholder_candidate(node)
        || node
            .base()
            .name
            .as_deref()
            .is_some_and(|name| name.ends_with(MAP_PLACEHOLDER_NAME_SUFFIX))
    {
        return None;
    }
    let size = dimensions(node, rects);
    (matches!(node, PenNode::Image(_)) || size.0 >= MIN_SHAPE_MAP_WIDTH).then_some(size)
}

fn is_map_placeholder_candidate(node: &PenNode) -> bool {
    match node {
        PenNode::Image(image) => is_map_image_intent(
            node.base().name.as_deref(),
            image.image_search_query.as_deref(),
            image.image_prompt.as_deref(),
        ),
        PenNode::Frame(_) | PenNode::Rectangle(_) => {
            node.base().name.as_deref().is_some_and(has_map_name_word)
                && !node
                    .base()
                    .name
                    .as_deref()
                    .is_some_and(is_map_control_group)
                && !contains_protected_descendant(node)
        }
        _ => false,
    }
}

fn is_map_control_group(name: &str) -> bool {
    // Icon-only toolbars have no protected text descendants. They are still
    // functional controls, never empty map slots to replace with terrain.
    let lower = name.to_lowercase();
    ["控件", "按钮", "工具栏"]
        .iter()
        .any(|part| lower.contains(part))
        || lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| {
                matches!(
                    word,
                    "controls" | "control" | "toolbar" | "tools" | "buttons" | "button"
                )
            })
}

/// Share map intent with photo-only cleanup before the image becomes a frame.
pub(crate) fn is_map_image_intent(
    name: Option<&str>,
    query: Option<&str>,
    prompt: Option<&str>,
) -> bool {
    [name, query, prompt]
        .into_iter()
        .flatten()
        .any(has_map_word)
}

fn is_authored_layer_control(node: &PenNode) -> bool {
    // An absolute viewport defaults missing x/y to zero. Requiring both
    // coordinates missed visible authored controls and added a second one.
    // Callers must establish the shared absolute viewport or explicit position.
    if !matches!(node, PenNode::Frame(_)) || !is_visible(node) {
        return false;
    }
    let Some([icon]) = node.children().map(Vec::as_slice) else {
        return false;
    };
    is_visible(icon)
        && matches!(icon, PenNode::IconFont(icon)
            if icon.icon_font_name.rsplit(':').next() == Some("layers"))
}

fn is_visible(node: &PenNode) -> bool {
    op_design_lint::node_util::is_node_visible(node)
        && !matches!(&node.base().opacity, Some(NumberOrExpression::Number(value)) if *value <= 0.0)
}

fn compact_layer_control_patch() -> Value {
    // Fixed dimensions prevent the original full-width layers-bar regression.
    // Keep authored position, styling, icon and interactions intact.
    json!({"width":40,"height":40,"layout":"horizontal","padding":0,
        "justifyContent":"center","alignItems":"center"})
}

fn contains_protected_descendant(node: &PenNode) -> bool {
    node.children().is_some_and(|children| {
        children
            .iter()
            .any(|child| is_protected_node(child) || contains_protected_descendant(child))
    })
}

fn is_protected_node(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Text(_)
            | PenNode::Image(_)
            | PenNode::TextInput(_)
            | PenNode::Select(_)
            | PenNode::Checkbox(_)
            | PenNode::Switch(_)
            | PenNode::Slider(_)
            | PenNode::RadioGroup(_)
            | PenNode::NumberInput(_)
            | PenNode::Tabs(_)
    )
}

fn has_map_word(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("地图")
        || lower.contains("路线图")
        || lower.contains("route map")
        || lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|word| word == "navigation" || is_map_token(word))
}

fn has_map_name_word(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("地图")
        || lower.contains("路线图")
        || lower.contains("route map")
        || lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(is_map_token)
}

/// A whole word that names a map. Matching on the `map` PREFIX turned every
/// "mapo tofu" dish photo — and a Sichuan store hero whose prompt mentioned
/// it — into a fake street map on GLM-5.3-Flash and Opus 5.5 delivery pages
/// (0923-0924); "maple" and "mapping" read the same way.
fn is_map_token(word: &str) -> bool {
    matches!(word, "map" | "maps" | "mapview" | "mapbox" | "minimap")
}

fn dimensions(node: &PenNode, rects: &HashMap<String, ResolvedRect>) -> (f64, f64) {
    let resolved = rects.get(node.id_str());
    let width = resolved
        .map(|rect| rect.width)
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or(MAP_PLACEHOLDER_WIDTH);
    let height = if height_needs_placeholder_size(node) {
        MAP_PLACEHOLDER_HEIGHT
    } else {
        resolved
            .map(|rect| rect.height)
            .filter(|value| value.is_finite() && *value > 0.0)
            .or_else(|| node.height_px())
            .unwrap_or(MAP_PLACEHOLDER_HEIGHT)
    };
    (width.max(1.0), height.max(1.0))
}

/// Build an editable map from a fixed scene selected by image intent.
pub fn build_map_placeholder(node: &PenNode, width: f64, height: f64, theme: Theme) -> Value {
    build_map_placeholder_with_controls(node, width, height, theme, false)
}

fn build_map_placeholder_with_controls(
    node: &PenNode,
    width: f64,
    height: f64,
    theme: Theme,
    sibling_layer_control: bool,
) -> Value {
    let mut hint = node.base().name.clone().unwrap_or_default();
    if let PenNode::Image(image) = node {
        hint.push_str(image.image_prompt.as_deref().unwrap_or_default());
        hint.push_str(image.image_search_query.as_deref().unwrap_or_default());
    }
    let kind = crate::map_templates::MapTemplate::from_hint(&hint);
    let mut blocks = crate::map_templates::children(node.id_str(), width, height, theme, kind);
    let absolute_viewport = matches!(node, PenNode::Frame(frame)
        if frame.container.layout == Some(LayoutMode::None));
    let preserved_control = |child: &PenNode| {
        is_authored_layer_control(child) && (absolute_viewport || has_numeric_position(child))
    };
    if sibling_layer_control || node.children().into_iter().flatten().any(preserved_control) {
        blocks.retain(|block| block["name"] != "Map layer control");
    }
    for block in &mut blocks {
        block["explain"] = json!(format!(
            "map-template-part:{}",
            block["name"].as_str().unwrap_or_default()
        ));
    }
    let preserved = node
        .children()
        .into_iter()
        .flatten()
        .filter(|child| has_numeric_position(child) || preserved_control(child))
        .filter_map(|child| {
            let mut value = serde_json::to_value(child).ok()?;
            if is_authored_layer_control(child) {
                value
                    .as_object_mut()?
                    .extend(compact_layer_control_patch().as_object()?.clone());
            }
            Some(value)
        });
    // Authored controls remain above the deterministic map composition.
    let mut children: Vec<Value> = preserved.collect();
    children.append(&mut blocks);

    let map_name = format!(
        "{}{}",
        node.base()
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("Map"),
        MAP_PLACEHOLDER_NAME_SUFFIX
    );
    let mut patch = json!({
        "type": "frame",
        "name": map_name,
        "role": "map",
        "layout": "none",
        "clipContent": true,
        "fill": solid_fill(if theme == Theme::Dark { "$--secondary" } else { "$--muted" }),
        "imageSearchQuery": null,
        "imagePrompt": null,
        "children": children,
        "explain": format!("map-template:{}; illustrative, not geographic data; viewport:{width}x{height}", kind.key())
    });
    if height_needs_placeholder_size(node) {
        patch["height"] = json!(MAP_PLACEHOLDER_HEIGHT);
    }
    patch
}

fn height_needs_placeholder_size(node: &PenNode) -> bool {
    let Ok(serialized) = serde_json::to_value(node) else {
        return false;
    };
    match serialized.get("height") {
        None => true,
        Some(Value::String(value)) => value == "fit_content",
        _ => false,
    }
}

fn has_numeric_position(node: &PenNode) -> bool {
    node.base().x.is_some_and(|value| value.is_finite())
        && node.base().y.is_some_and(|value| value.is_finite())
}

fn solid_fill(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

#[cfg(test)]
#[path = "map_placeholder_tests.rs"]
mod tests;
