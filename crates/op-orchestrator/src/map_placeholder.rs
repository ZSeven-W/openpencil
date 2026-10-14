//! Deterministic map placeholders for image slots.
//!
//! Maps are authored as image intent by the model, but they are not stock
//! photography. This module turns map intent into a small vector-like scene
//! before image targets are collected. The builder is pure and returns a
//! shallow patch, which preserves the original node's id and geometry.

use std::collections::HashMap;

use crate::image_fallback_policy::ResolvedRect;
use crate::role_defaults::Theme;
use jian_ops_schema::node::PenNode;
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
    collect_patches(root, rects, theme, false, &mut patches);
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
        if let Some(patch) = map_patch(node, rects, theme) {
            patches.push(patch);
            return;
        }
    }
    let map_named = inside_map_named || node.base().name.as_deref().is_some_and(has_map_name_word);
    if let Some(children) = node.children() {
        for child in children {
            collect_patches(child, rects, theme, map_named, patches);
        }
    }
}

fn map_patch(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    theme: Theme,
) -> Option<MapPlaceholderPatch> {
    if !is_map_placeholder_candidate(node) {
        return None;
    }

    let (width, height) = dimensions(node, rects);
    if !matches!(node, PenNode::Image(_)) && width < MIN_SHAPE_MAP_WIDTH {
        return None;
    }
    let patch = build_map_placeholder(node, width, height, theme);
    Some(MapPlaceholderPatch {
        node_id: node.id_str().to_string(),
        patch_json: patch.to_string(),
    })
}

fn is_map_placeholder_candidate(node: &PenNode) -> bool {
    match node {
        PenNode::Image(image) => {
            node.base().name.as_deref().is_some_and(has_map_word)
                || image
                    .image_search_query
                    .as_deref()
                    .is_some_and(has_map_word)
                || image.image_prompt.as_deref().is_some_and(has_map_word)
        }
        PenNode::Frame(_) | PenNode::Rectangle(_) => {
            node.base().name.as_deref().is_some_and(has_map_name_word)
                && !contains_protected_descendant(node)
        }
        _ => false,
    }
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
    let mut hint = node.base().name.clone().unwrap_or_default();
    if let PenNode::Image(image) = node {
        hint.push_str(image.image_prompt.as_deref().unwrap_or_default());
        hint.push_str(image.image_search_query.as_deref().unwrap_or_default());
    }
    let kind = crate::map_templates::MapTemplate::from_hint(&hint);
    let mut blocks = crate::map_templates::children(node.id_str(), width, height, theme, kind);
    let preserved = node
        .children()
        .into_iter()
        .flatten()
        .filter(|child| has_numeric_position(child))
        .filter_map(|child| serde_json::to_value(child).ok());
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
        "layout": "none",
        "clipContent": true,
        "fill": solid_fill(if theme == Theme::Dark { "$--secondary" } else { "$--muted" }),
        "imageSearchQuery": null,
        "imagePrompt": null,
        "children": children,
        "explain": format!("map-template:{}; illustrative, not geographic data", kind.key())
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
