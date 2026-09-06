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
const STREET_MIN: f64 = 14.0;
const STREET_MAX: f64 = 18.0;
const ROAD_MIN: f64 = 10.0;
const ROAD_MAX: f64 = 14.0;

/// One pure map-placeholder rewrite.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapPlaceholderPatch {
    pub node_id: String,
    pub patch_json: String,
}

/// Build patches for every map-intent image, frame, or rectangle below
/// `root`. Marked placeholders are skipped so running cleanup twice is stable.
pub fn map_placeholder(
    root: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    theme: Theme,
) -> Vec<MapPlaceholderPatch> {
    let mut patches = Vec::new();
    collect_patches(root, rects, theme, &mut patches);
    patches
}

fn collect_patches(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    theme: Theme,
    patches: &mut Vec<MapPlaceholderPatch>,
) {
    if let Some(patch) = map_patch(node, rects, theme) {
        patches.push(patch);
        return;
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_patches(child, rects, theme, patches);
        }
    }
}

fn map_patch(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    theme: Theme,
) -> Option<MapPlaceholderPatch> {
    let name = node.base().name.as_deref().unwrap_or_default();
    if name.ends_with(MAP_PLACEHOLDER_NAME_SUFFIX) {
        return None;
    }
    if !contains_map_words(node) {
        return None;
    }

    let (width, height) = dimensions(node, rects);
    let patch = build_map_placeholder(node, width, height, theme);
    Some(MapPlaceholderPatch {
        node_id: node.id_str().to_string(),
        patch_json: patch.to_string(),
    })
}

fn contains_map_words(node: &PenNode) -> bool {
    let name_matches = node.base().name.as_deref().is_some_and(has_map_name_word);
    let image_name_matches = node.base().name.as_deref().is_some_and(has_map_word);
    match node {
        PenNode::Image(image) => {
            image_name_matches
                || image
                    .image_search_query
                    .as_deref()
                    .is_some_and(has_map_word)
                || image.image_prompt.as_deref().is_some_and(has_map_word)
        }
        PenNode::Frame(frame) => {
            name_matches
                || frame
                    .image_search_query
                    .as_deref()
                    .is_some_and(has_map_word)
        }
        PenNode::Rectangle(_) => name_matches,
        _ => false,
    }
}

fn has_map_word(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("地图")
        || lower.contains("路线图")
        || lower.contains("route map")
        || lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|word| word == "navigation" || word.starts_with("map"))
}

fn has_map_name_word(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("地图")
        || lower.contains("路线图")
        || lower.contains("route map")
        || lower
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|word| word.starts_with("map"))
}

fn dimensions(node: &PenNode, rects: &HashMap<String, ResolvedRect>) -> (f64, f64) {
    let resolved = rects.get(node.id_str());
    let width = resolved
        .map(|rect| rect.width)
        .filter(|value| value.is_finite() && *value > 0.0)
        .or_else(|| node.width_px())
        .unwrap_or(320.0);
    let height = resolved
        .map(|rect| rect.height)
        .filter(|value| value.is_finite() && *value > 0.0)
        .or_else(|| node.height_px())
        .unwrap_or(200.0);
    (width.max(1.0), height.max(1.0))
}

/// Build the vector-like map scene. The FNV-1a seed affects the street width,
/// block count, and block inset; the same node id therefore always yields the
/// same geometry without relying on a toolchain- or process-seeded hasher.
pub fn build_map_placeholder(node: &PenNode, width: f64, height: f64, theme: Theme) -> Value {
    let id = node.id_str();
    let seed = fnv1a(id.as_bytes());
    let street = (STREET_MIN + f64::from((seed % 5) as u32)).min(STREET_MAX);
    let road = (ROAD_MIN + f64::from(((seed >> 8) % 5) as u32)).min(ROAD_MAX);
    let block_count = 4 + (seed % 3) as usize;
    let rows = block_count.div_ceil(3);
    let cell_width = ((width - 2.0 * street) / 3.0).max(1.0);
    let cell_height = ((height - (rows.saturating_sub(1) as f64) * street) / rows as f64).max(1.0);
    let inset = f64::from(((seed >> 16) % 5) as u32).min(4.0);

    let mut blocks = Vec::with_capacity(block_count);
    for index in 0..block_count {
        let column = index % 3;
        let row = index / 3;
        let x = column as f64 * (cell_width + street) + inset;
        let y = row as f64 * (cell_height + street) + inset;
        let block_width = (cell_width - 2.0 * inset).max(1.0);
        let block_height = (cell_height - 2.0 * inset).max(1.0);
        blocks.push(json!({
            "type": "rectangle",
            "id": format!("{id}-map-block-{}", index + 1),
            "name": format!("map-block-{}", index + 1),
            "x": x,
            "y": y,
            "width": block_width,
            "height": block_height,
            "cornerRadius": 6,
            "fill": solid_fill("$--card")
        }));
    }

    let road_y = clamp(height * 0.45 - road / 2.0, 0.0, (height - road).max(0.0));
    let road_x = clamp(width * 0.60 - road / 2.0, 0.0, (width - road).max(0.0));
    blocks.push(json!({
        "type": "rectangle",
        "id": format!("{id}-map-road-h"),
        "name": "map-road-h",
        "x": 0,
        "y": road_y,
        "width": width,
        "height": road,
        "fill": solid_fill("$--background")
    }));
    blocks.push(json!({
        "type": "rectangle",
        "id": format!("{id}-map-road-v"),
        "name": "map-road-v",
        "x": road_x,
        "y": 0,
        "width": road,
        "height": height,
        "fill": solid_fill("$--background")
    }));

    let lower_left = &blocks[(rows - 1) * 3];
    let upper_right = &blocks[2.min(block_count - 1)];
    let start = block_center(lower_left);
    let end = block_center(upper_right);
    blocks.push(json!({
        "type": "path",
        "id": format!("{id}-map-route"),
        "name": "map-route",
        "x": 0,
        "y": 0,
        "width": width,
        "height": height,
        "d": format!("M {} {} L {} {} L {} {}", start.0, start.1, end.0, start.1, end.0, end.1),
        "stroke": {
            "thickness": 4,
            "cap": "round",
            "join": "round",
            "fill": solid_fill("$--primary")
        }
    }));

    let pin_x = clamp(end.0 - 14.0, 0.0, (width - 28.0).max(0.0));
    let pin_y = clamp(end.1 - 28.0, 0.0, (height - 28.0).max(0.0));
    blocks.push(json!({
        "type": "icon_font",
        "id": format!("{id}-map-pin"),
        "name": "map-pin",
        "x": pin_x,
        "y": pin_y,
        "width": 28,
        "height": 28,
        "iconFontFamily": "lucide",
        "iconFontName": "map-pin",
        "fill": solid_fill("$--primary")
    }));

    let origin_x = clamp(start.0 - 7.0, 0.0, (width - 14.0).max(0.0));
    let origin_y = clamp(start.1 - 7.0, 0.0, (height - 14.0).max(0.0));
    let halo_x = clamp(start.0 - 14.0, 0.0, (width - 28.0).max(0.0));
    let halo_y = clamp(start.1 - 14.0, 0.0, (height - 28.0).max(0.0));
    blocks.push(json!({
        "type": "ellipse",
        "id": format!("{id}-map-origin-halo"),
        "name": "map-origin-halo",
        "x": halo_x,
        "y": halo_y,
        "width": 28,
        "height": 28,
        "fill": solid_fill_with_opacity("$--primary", 0.2)
    }));
    blocks.push(json!({
        "type": "ellipse",
        "id": format!("{id}-map-origin"),
        "name": "map-origin",
        "x": origin_x,
        "y": origin_y,
        "width": 14,
        "height": 14,
        "fill": solid_fill("$--primary")
    }));

    let preserved = node
        .children()
        .into_iter()
        .flatten()
        .filter(|child| has_numeric_position(child))
        .filter_map(|child| serde_json::to_value(child).ok());
    blocks.extend(preserved);

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
    json!({
        "type": "frame",
        "name": map_name,
        "layout": "none",
        "clipContent": true,
        "fill": solid_fill(if theme == Theme::Dark { "$--secondary" } else { "$--muted" }),
        "imageSearchQuery": null,
        "imagePrompt": null,
        "children": blocks,
        "explain": "map placeholder"
    })
}

fn block_center(value: &Value) -> (f64, f64) {
    let x = value["x"].as_f64().unwrap_or(0.0);
    let y = value["y"].as_f64().unwrap_or(0.0);
    let width = value["width"].as_f64().unwrap_or(1.0);
    let height = value["height"].as_f64().unwrap_or(1.0);
    (x + width / 2.0, y + height / 2.0)
}

fn has_numeric_position(node: &PenNode) -> bool {
    node.base().x.is_some_and(|value| value.is_finite())
        && node.base().y.is_some_and(|value| value.is_finite())
}

fn solid_fill(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

fn solid_fill_with_opacity(color: &str, opacity: f64) -> Value {
    json!([{ "type": "solid", "color": color, "opacity": opacity }])
}

fn clamp(value: f64, low: f64, high: f64) -> f64 {
    value.max(low).min(high)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
#[path = "map_placeholder_tests.rs"]
mod tests;
