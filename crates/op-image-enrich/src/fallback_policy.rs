//! Deterministic visual fallbacks for image slots that did not resolve.
//!
//! This module is transport-free. It only reads and rewrites the canonical
//! editor node tree, so every host that writes the failed-search marker gets
//! the same fallback tile without depending on a network runtime.

use std::collections::HashMap;

use jian_ops_schema::node::{container::LayoutMode, PenNode};
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
    /// A media slot underneath overlay text (the hero image in a `layout:
    /// none` stack with a title on top): a silent tonal block, no icon and no
    /// caption, so the tile never collides with the content laid over it.
    Covered,
}

impl ImageFallbackBranch {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Thumb => "thumb",
            Self::Media => "media",
            Self::Covered => "covered",
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
    collect_patches(root, resolved_widths, after_enrich, false, &mut patches);
    patches
}

fn collect_patches(
    node: &PenNode,
    resolved_widths: &HashMap<String, f64>,
    after_enrich: bool,
    covered_by_overlay: bool,
    patches: &mut Vec<ImageFallbackPatch>,
) {
    if let Some(patch) = fallback_patch(node, resolved_widths, after_enrich, covered_by_overlay) {
        patches.push(patch);
        return;
    }
    if let Some(children) = node.children() {
        let child_covered = is_overlay_stack_with_text(node, children);
        for child in children {
            collect_patches(child, resolved_widths, after_enrich, child_covered, patches);
        }
    }
}

/// A `layout: none` frame whose children include text: everything in it is
/// stacked, so a media slot there sits underneath (or behind) that text.
fn is_overlay_stack_with_text(node: &PenNode, children: &[PenNode]) -> bool {
    matches!(node, PenNode::Frame(frame) if frame.container.layout == Some(LayoutMode::None))
        && children.iter().any(has_text_descendant)
}

fn has_text_descendant(node: &PenNode) -> bool {
    // A caption created by this policy is not authored overlay content. In
    // particular, it must not make its own parent stack become covered on the
    // next pass. Edited captions and other authored children remain content.
    let generated = policy_media_children(node);
    matches!(node, PenNode::Text(_))
        || node.children().is_some_and(|children| {
            children.iter().any(|child| {
                !is_policy_media_child(node, child, &generated) && has_text_descendant(child)
            })
        })
}

fn fallback_patch(
    node: &PenNode,
    resolved_widths: &HashMap<String, f64>,
    after_enrich: bool,
    covered_by_overlay: bool,
) -> Option<ImageFallbackPatch> {
    if is_image_fallback(node) {
        if covered_by_overlay {
            if let Some(patch) = cover_existing_media_fallback(node) {
                return Some(patch);
            }
        }
        return normalize_legacy_media_caption(node);
    }
    if !matches!(
        node,
        PenNode::Image(_) | PenNode::Frame(_) | PenNode::Rectangle(_)
    ) || !is_failed_image_slot(node, after_enrich)
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
    } else if covered_by_overlay {
        ImageFallbackBranch::Covered
    } else {
        ImageFallbackBranch::Media
    };
    Some(fallback_patch_for_branch(node, branch))
}

/// Build the policy decoration for a known branch without reclassifying the
/// current geometry, which may have changed since a fallback was saved.
pub(crate) fn fallback_patch_for_branch(
    node: &PenNode,
    branch: ImageFallbackBranch,
) -> ImageFallbackPatch {
    let query = search_query(node);
    let icon_name = icon_name_for_query(&query);
    let name = fallback_name(node.base().name.as_deref());
    let patch = match branch {
        ImageFallbackBranch::Thumb => thumbnail_patch(node, &name, &icon_name),
        ImageFallbackBranch::Media => {
            media_patch(node, &name, &icon_name, &caption_for_query(&query))
        }
        ImageFallbackBranch::Covered => covered_patch(node, &name),
    };
    ImageFallbackPatch {
        node_id: node.id_str().to_string(),
        branch,
        patch_json: patch.to_string(),
    }
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
            &[
                "dumbbell",
                "dumbbells",
                "fitness",
                "exercise",
                "exercises",
                "workout",
                "workouts",
            ][..],
            "dumbbell",
        ),
        (
            &["food", "foods", "dish", "dishes", "meal", "meals"][..],
            "utensils",
        ),
        (&["coffee", "coffees"][..], "coffee"),
        (
            &["product", "products", "shirt", "shirts", "shoe", "shoes"][..],
            "shopping-bag",
        ),
        (&["map", "maps"][..], "map"),
        (
            &[
                "user",
                "users",
                "avatar",
                "avatars",
                "portrait",
                "portraits",
            ][..],
            "user",
        ),
        (
            &["house", "houses", "hotel", "hotels", "room", "rooms"][..],
            "building",
        ),
        (&["car", "cars", "ride", "rides"][..], "car"),
        (&["music"][..], "music"),
        (&["video", "videos"][..], "play"),
    ];
    table
        .iter()
        .find(|(keywords, _)| {
            // Whole words avoid unrelated substrings (skincare/car,
            // mushroom/room). Explicit plurals preserve common queries
            // without applying a stemmer to arbitrary subject names.
            query
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| keywords.contains(&word))
        })
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

fn media_patch(node: &PenNode, name: &str, icon_name: &str, caption: &str) -> Value {
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
                    "width": "fill_container",
                    "height": "fit_content",
                    "textGrowth": "fixed-width",
                    "fontSize": 12,
                    "textAlign": "center",
                    "fill": solid_fill("$--muted-foreground")
                }
            ],
            "explain": fallback_explain(node, ImageFallbackBranch::Media)
        }),
    )
}

/// Reconstruct the exact decorations this policy owns. A matching id alone
/// is insufficient: a user may have edited the caption or icon after failure.
fn policy_media_children(node: &PenNode) -> Vec<PenNode> {
    if !node
        .base()
        .explain
        .as_deref()
        .is_some_and(|explain| explain.starts_with("image fallback: media "))
        || fallback_metadata(node).is_none()
    {
        return Vec::new();
    }
    let query = fallback_search_query_for_host(node);
    let patch = media_patch(
        node,
        "",
        &icon_name_for_query(&query),
        &caption_for_query(&query),
    );
    serde_json::from_value(patch["children"].clone()).unwrap_or_default()
}

fn legacy_caption_width(node: &PenNode) -> Option<f64> {
    // Resolved widths were not stored with legacy tiles. Without an authored
    // numeric media width, ownership of a fixed-width caption is unprovable.
    node.width_px()
        .filter(|width| width.is_finite() && *width > 96.0)
        .map(|width| width.clamp(1.0, 160.0))
}

fn is_policy_media_child(node: &PenNode, child: &PenNode, generated: &[PenNode]) -> bool {
    if generated.is_empty() {
        return false;
    }
    if generated.contains(child) {
        return true;
    }
    // The earlier caption used a fixed 16px height. Recognize it in the same
    // pass as current captions, but require every other property to match.
    let Ok(mut value) = serde_json::to_value(child) else {
        return false;
    };
    if value["type"] != "text"
        || value["height"].as_f64() != Some(16.0)
        || legacy_caption_width(node).is_none()
        || value["width"].as_f64() != legacy_caption_width(node)
        || !value["textGrowth"].is_null()
    {
        return false;
    }
    value["width"] = json!("fill_container");
    value["height"] = json!("fit_content");
    value["textGrowth"] = json!("fixed-width");
    serde_json::from_value::<PenNode>(value).is_ok_and(|normalized| generated.contains(&normalized))
}

/// Repair a tile produced without its parent overlay context. Only pure,
/// unchanged policy decorations are safe to remove. Customized tiles retain
/// their existing content and retry behavior; this does not redefine the
/// restoration protocol for image-filled containers with authored children.
fn cover_existing_media_fallback(node: &PenNode) -> Option<ImageFallbackPatch> {
    let generated = policy_media_children(node);
    let children = node.children()?;
    if children.is_empty()
        || !children
            .iter()
            .all(|child| is_policy_media_child(node, child, &generated))
    {
        return None;
    }
    let metadata = node
        .base()
        .explain
        .as_deref()?
        .strip_prefix("image fallback: media ")?;
    Some(ImageFallbackPatch {
        node_id: node.id_str().to_owned(),
        branch: ImageFallbackBranch::Covered,
        patch_json: json!({
            "children": [],
            "explain": format!("image fallback: covered {metadata}")
        })
        .to_string(),
    })
}

/// Upgrade only the policy-owned legacy caption layout. Keep edited copy,
/// typography, intent metadata and node ids intact; unrelated text is not a
/// fallback caption merely because its name resembles one.
fn normalize_legacy_media_caption(node: &PenNode) -> Option<ImageFallbackPatch> {
    if !node
        .base()
        .explain
        .as_deref()?
        .starts_with("image fallback: media ")
    {
        return None;
    }
    let mut children = serde_json::to_value(node.children()?).ok()?;
    let caption_id = format!("{}-image-fallback-caption", node.id_str());
    let caption = children
        .as_array_mut()?
        .iter_mut()
        .find(|child| child["id"] == caption_id && child["type"] == "text")?;
    if caption["height"].as_f64() != Some(16.0)
        || legacy_caption_width(node).is_none()
        || caption["width"].as_f64() != legacy_caption_width(node)
        || !caption["textGrowth"].is_null()
    {
        return None;
    }
    caption["width"] = json!("fill_container");
    caption["height"] = json!("fit_content");
    caption["textGrowth"] = json!("fixed-width");
    Some(ImageFallbackPatch {
        node_id: node.id_str().to_owned(),
        branch: ImageFallbackBranch::Media,
        patch_json: json!({"children":children}).to_string(),
    })
}

fn covered_patch(node: &PenNode, name: &str) -> Value {
    fallback_frame_patch(
        node,
        json!({
            "type": "frame",
            "name": name,
            "fill": solid_fill("$--muted"),
            "stroke": null,
            "children": [],
            "explain": fallback_explain(node, ImageFallbackBranch::Covered)
        }),
    )
}

pub fn apply_image_fallback_policy_to_node(state: &mut EditorState, node_id: &NodeId) -> bool {
    let children = state.active_children();
    let Some(node) = walkers::find_node(children, node_id) else {
        return false;
    };
    let covered_by_overlay = walkers::find_parent_and_index(children, node_id)
        .and_then(|(parent, _)| parent)
        .and_then(|parent_id| walkers::find_node(children, &parent_id))
        .is_some_and(|parent| {
            parent
                .children()
                .is_some_and(|children| is_overlay_stack_with_text(parent, children))
        });
    // Targeted enrichment must use the same parent context as the full-tree
    // policy, and must never apply a descendant's patch to this target.
    let Some(patch) = fallback_patch(node, &HashMap::new(), false, covered_by_overlay) else {
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

#[cfg(test)]
#[path = "fallback_policy_context_tests.rs"]
mod context_tests;
