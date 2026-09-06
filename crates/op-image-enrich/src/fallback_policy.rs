//! Deterministic visual fallbacks for image slots that did not resolve.
//!
//! This module is transport-free. It only reads and rewrites the canonical
//! editor node tree, so every host that writes the failed-search marker gets
//! the same fallback tile without depending on a network runtime.

use std::collections::HashMap;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use op_editor_core::{walkers, EditorCommand, EditorState, NodeId, PenNodeExt};
use serde_json::{json, Value};

use crate::targets::{image_request_mode, ImageRequestMode};

pub const SEARCH_FAILED_PLACEHOLDER_SRC: &str = "placeholder://image-search-failed";
pub const IMAGE_FALLBACK_NAME_SUFFIX: &str = " (image fallback)";

const IMAGE_FALLBACK_EXPLAIN_PREFIX: &str = "image fallback:";
const SVG_PLACEHOLDER_SRC_PREFIX: &str = "data:image/svg+xml;charset=utf-8,%3Csvg";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageFallbackBranch {
    Thumb,
    Media,
}

impl ImageFallbackBranch {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Thumb => "thumb",
            Self::Media => "media",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageFallbackPatch {
    pub node_id: String,
    pub branch: ImageFallbackBranch,
    pub patch_json: String,
}

pub fn image_fallback_policy(root: &PenNode, after_enrich: bool) -> Vec<ImageFallbackPatch> {
    image_fallback_policy_with_widths(root, &HashMap::new(), after_enrich)
}

/// Use resolved layout widths when available; otherwise use the authored
/// numeric width. A slot without a numeric width is treated as media.
pub fn image_fallback_policy_with_widths(
    root: &PenNode,
    resolved_widths: &HashMap<String, f64>,
    after_enrich: bool,
) -> Vec<ImageFallbackPatch> {
    let mut patches = Vec::new();
    collect_patches(root, resolved_widths, after_enrich, &mut patches);
    patches
}

fn collect_patches(
    node: &PenNode,
    resolved_widths: &HashMap<String, f64>,
    after_enrich: bool,
    patches: &mut Vec<ImageFallbackPatch>,
) {
    if let Some(patch) = fallback_patch(node, resolved_widths, after_enrich) {
        patches.push(patch);
        return;
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_patches(child, resolved_widths, after_enrich, patches);
        }
    }
}

fn fallback_patch(
    node: &PenNode,
    resolved_widths: &HashMap<String, f64>,
    after_enrich: bool,
) -> Option<ImageFallbackPatch> {
    if !matches!(
        node,
        PenNode::Image(_) | PenNode::Frame(_) | PenNode::Rectangle(_)
    ) || is_image_fallback(node)
        || !is_failed_image_slot(node, after_enrich)
    {
        return None;
    }

    let width = resolved_widths
        .get(node.id_str())
        .copied()
        .filter(|width| width.is_finite() && *width > 0.0)
        .or_else(|| {
            node.width_px()
                .filter(|width| width.is_finite() && *width >= 0.0)
        })
        .unwrap_or(160.0);
    let branch = if width <= 96.0 {
        ImageFallbackBranch::Thumb
    } else {
        ImageFallbackBranch::Media
    };
    let query = search_query(node);
    let icon_name = icon_name_for_query(&query);
    let name = fallback_name(node.base().name.as_deref());
    let patch = match branch {
        ImageFallbackBranch::Thumb => thumbnail_patch(node, &name, &icon_name),
        ImageFallbackBranch::Media => {
            media_patch(node, &name, &icon_name, &caption_for_query(&query), width)
        }
    };
    Some(ImageFallbackPatch {
        node_id: node.id_str().to_string(),
        branch,
        patch_json: patch.to_string(),
    })
}

fn is_failed_image_slot(node: &PenNode, after_enrich: bool) -> bool {
    if is_image_fallback(node) {
        return true;
    }
    let source = match node {
        PenNode::Image(image) => Some(image.src.as_ref()),
        PenNode::Frame(frame) => image_fill_source(frame.container.fill.as_deref()),
        PenNode::Rectangle(rectangle) => image_fill_source(rectangle.container.fill.as_deref()),
        _ => None,
    };
    let Some(source) = source else {
        return false;
    };
    source == SEARCH_FAILED_PLACEHOLDER_SRC
        || (after_enrich
            && (source.trim().is_empty() || source.starts_with(SVG_PLACEHOLDER_SRC_PREFIX)))
}

fn image_fill_source(fills: Option<&[PenFill]>) -> Option<&str> {
    let Some([PenFill::Image(image), ..]) = fills else {
        return None;
    };
    Some(image.url.as_ref())
}

pub fn is_image_fallback(node: &PenNode) -> bool {
    node.base()
        .name
        .as_deref()
        .is_some_and(|name| name.ends_with(IMAGE_FALLBACK_NAME_SUFFIX))
        || node
            .base()
            .explain
            .as_deref()
            .is_some_and(|explain| explain.starts_with(IMAGE_FALLBACK_EXPLAIN_PREFIX))
}

pub fn is_failed_image_slot_for_host(node: &PenNode) -> bool {
    is_failed_image_slot(node, false)
}

fn search_query(node: &PenNode) -> String {
    match node {
        PenNode::Image(image) => image
            .image_search_query
            .as_deref()
            .or(image.image_prompt.as_deref())
            .unwrap_or_default()
            .trim()
            .to_string(),
        PenNode::Frame(frame) => frame
            .image_search_query
            .clone()
            .or_else(|| fallback_image_prompt(node))
            .unwrap_or_default()
            .trim()
            .to_string(),
        PenNode::Rectangle(_) => fallback_image_prompt(node).unwrap_or_default(),
        _ => String::new(),
    }
}

fn original_search_query(node: &PenNode) -> Option<String> {
    match node {
        PenNode::Image(image) => image.image_search_query.clone(),
        PenNode::Frame(frame) => frame.image_search_query.clone(),
        _ => None,
    }
}

fn original_image_prompt(node: &PenNode) -> Option<String> {
    match node {
        PenNode::Image(image) => image.image_prompt.clone(),
        _ => fallback_image_prompt(node),
    }
}

pub fn icon_name_for_query(query: &str) -> String {
    let query = query.to_ascii_lowercase();
    let table = [
        (
            &["dumbbell", "fitness", "exercise", "workout"][..],
            "dumbbell",
        ),
        (&["food", "dish", "meal"][..], "utensils"),
        (&["coffee"][..], "coffee"),
        (&["product", "shirt", "shoe"][..], "shopping-bag"),
        (&["map"][..], "map"),
        (&["user", "avatar", "portrait"][..], "user"),
        (&["house", "hotel", "room"][..], "building"),
        (&["car", "ride"][..], "car"),
        (&["music"][..], "music"),
        (&["video"][..], "play"),
    ];
    table
        .iter()
        .find(|(keywords, _)| keywords.iter().any(|keyword| query.contains(keyword)))
        .map(|(_, icon)| (*icon).to_string())
        .unwrap_or_else(|| "image".to_string())
}

fn caption_for_query(query: &str) -> String {
    let query = query.trim();
    if query.is_empty() {
        return "Image".to_string();
    }
    let lower = query.to_lowercase();
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return "Image".to_string();
    };
    let sentence = first.to_uppercase().collect::<String>() + chars.as_str();
    sentence.chars().take(24).collect()
}

fn fallback_name(name: Option<&str>) -> String {
    let base = name.map(str::trim).filter(|name| !name.is_empty());
    format!("{}{}", base.unwrap_or("Image"), IMAGE_FALLBACK_NAME_SUFFIX)
}

fn request_mode_name(mode: ImageRequestMode) -> &'static str {
    match mode {
        ImageRequestMode::Auto => "auto",
        ImageRequestMode::Search => "search",
        ImageRequestMode::Generate => "generate",
    }
}

fn fallback_explain(node: &PenNode, branch: ImageFallbackBranch) -> String {
    let metadata = json!({
        "imageSearchQuery": original_search_query(node),
        "imagePrompt": original_image_prompt(node),
        "imageRequestMode": request_mode_name(image_request_mode(node)),
        "originalExplain": node.base().explain,
    });
    format!(
        "{} {} {}",
        IMAGE_FALLBACK_EXPLAIN_PREFIX,
        branch.as_str(),
        metadata
    )
}

fn fallback_metadata(node: &PenNode) -> Option<Value> {
    let explain = node.base().explain.as_deref()?;
    let rest = explain.strip_prefix(IMAGE_FALLBACK_EXPLAIN_PREFIX)?.trim();
    let (_, payload) = rest.split_once(char::is_whitespace)?;
    serde_json::from_str(payload.trim()).ok()
}

fn fallback_value(node: &PenNode, key: &str) -> Option<Value> {
    fallback_metadata(node)?.get(key).cloned()
}

fn fallback_image_prompt(node: &PenNode) -> Option<String> {
    fallback_value(node, "imagePrompt")?
        .as_str()
        .map(str::to_string)
}

pub fn fallback_request_mode(node: &PenNode) -> Option<ImageRequestMode> {
    let mode = fallback_value(node, "imageRequestMode")?
        .as_str()?
        .to_ascii_lowercase();
    match mode.as_str() {
        "auto" => Some(ImageRequestMode::Auto),
        "search" => Some(ImageRequestMode::Search),
        "generate" => Some(ImageRequestMode::Generate),
        _ => None,
    }
}

pub fn fallback_search_query_for_host(node: &PenNode) -> String {
    fallback_value(node, "imageSearchQuery")
        .and_then(|value| value.as_str().map(str::to_string))
        .or_else(|| match node {
            PenNode::Frame(frame) => frame.image_search_query.clone(),
            _ => None,
        })
        .or_else(|| fallback_image_prompt(node))
        .unwrap_or_default()
}

fn solid_fill(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

fn fallback_frame_patch(node: &PenNode, mut patch: Value) -> Value {
    let object = patch.as_object_mut().expect("fallback patch object");
    object.insert(
        "imageSearchQuery".to_string(),
        original_search_query(node)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    object.insert(
        "imagePrompt".to_string(),
        original_image_prompt(node)
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    patch
}

fn thumbnail_patch(node: &PenNode, name: &str, icon_name: &str) -> Value {
    fallback_frame_patch(
        node,
        json!({
            "type": "frame",
            "name": name,
            "fill": solid_fill("$--muted"),
            "stroke": null,
            "layout": "horizontal",
            "justifyContent": "center",
            "alignItems": "center",
            "children": [{
                "type": "icon_font",
                "id": format!("{}-image-fallback-icon", node.id_str()),
                "name": "Image fallback icon",
                "iconFontFamily": "lucide",
                "iconFontName": icon_name,
                "width": 20,
                "height": 20,
                "fill": solid_fill("$--muted-foreground")
            }],
            "explain": fallback_explain(node, ImageFallbackBranch::Thumb)
        }),
    )
}

fn media_patch(node: &PenNode, name: &str, icon_name: &str, caption: &str, width: f64) -> Value {
    fallback_frame_patch(
        node,
        json!({
            "type": "frame",
            "name": name,
            "fill": solid_fill("$--muted"),
            "stroke": null,
            "layout": "vertical",
            "justifyContent": "center",
            "alignItems": "center",
            "children": [
                {
                    "type": "icon_font",
                    "id": format!("{}-image-fallback-icon", node.id_str()),
                    "name": "Image fallback icon",
                    "iconFontFamily": "lucide",
                    "iconFontName": icon_name,
                    "width": 30,
                    "height": 30,
                    "fill": solid_fill("$--muted-foreground")
                },
                {
                    "type": "text",
                    "id": format!("{}-image-fallback-caption", node.id_str()),
                    "name": "Image fallback caption",
                    "content": caption,
                    "width": width.clamp(1.0, 160.0),
                    "height": 16,
                    "fontSize": 12,
                    "textAlign": "center",
                    "fill": solid_fill("$--muted-foreground")
                }
            ],
            "explain": fallback_explain(node, ImageFallbackBranch::Media)
        }),
    )
}

pub fn apply_image_fallback_policy_to_node(state: &mut EditorState, node_id: &NodeId) -> bool {
    let Some(node) = walkers::find_node(state.active_children(), node_id).cloned() else {
        return false;
    };
    let Some(patch) = image_fallback_policy(&node, false).into_iter().next() else {
        return false;
    };
    state.apply(EditorCommand::PatchNodeData {
        node_id: node_id.clone(),
        patch_json: patch.patch_json,
        page_id: None,
    })
}

/// Convert a fallback tile back into an image slot with the same id.
pub fn restore_image_fallback_node(node: &PenNode) -> Option<PenNode> {
    if !is_image_fallback(node) || !matches!(node, PenNode::Frame(_)) {
        return None;
    }
    let metadata = fallback_metadata(node);
    let mut value = serde_json::to_value(node).ok()?;
    let object = value.as_object_mut()?;
    object.insert("type".to_string(), Value::String("image".to_string()));
    object.insert("src".to_string(), Value::String(String::new()));
    for key in [
        "children",
        "fill",
        "stroke",
        "layout",
        "gap",
        "padding",
        "justifyContent",
        "alignItems",
        "clipContent",
        "stickyChildren",
        "reusable",
        "slot",
        "screen",
        "breakpoint",
    ] {
        object.remove(key);
    }
    if let Some(metadata) = metadata.as_ref() {
        for key in ["imageSearchQuery", "imagePrompt"] {
            match metadata.get(key) {
                Some(value) if !value.is_null() => {
                    object.insert(key.to_string(), value.clone());
                }
                _ => {
                    object.remove(key);
                }
            }
        }
        object.insert(
            "explain".to_string(),
            metadata
                .get("originalExplain")
                .cloned()
                .unwrap_or(Value::Null),
        );
    } else {
        object.insert("explain".to_string(), Value::Null);
    }
    let name = node
        .base()
        .name
        .as_deref()
        .and_then(|name| name.strip_suffix(IMAGE_FALLBACK_NAME_SUFFIX))
        .map(str::trim)
        .filter(|name| !name.is_empty());
    object.insert(
        "name".to_string(),
        name.map(|name| Value::String(name.to_string()))
            .unwrap_or(Value::Null),
    );
    serde_json::from_value(value).ok()
}

#[cfg(test)]
#[path = "fallback_policy_tests.rs"]
mod tests;
