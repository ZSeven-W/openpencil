//! Deterministic visual fallbacks for image slots that did not resolve.
//!
//! The policy itself is pure: it reads a node tree and resolved rectangles and
//! returns shallow patches. Hosts choose how to apply those patches, so the
//! cleanup driver can account for them through `DocSink` while enrichment can
//! apply the exact same result directly to an `EditorState`.

use std::collections::HashMap;

use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;
use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, PenNodeExt};
use serde_json::{json, Value};

/// The sentinel written when every image-search avenue failed.
pub const SEARCH_FAILED_PLACEHOLDER_SRC: &str = "placeholder://image-search-failed";

const SVG_PLACEHOLDER_SRC_PREFIX: &str = "data:image/svg+xml;charset=utf-8,%3Csvg";
const IMAGE_FALLBACK_NAME_SUFFIX: &str = " (image fallback)";

/// The resolved absolute rectangle used by cleanup policies.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// The visual branch selected for an unresolved image slot.
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

/// One pure image-fallback rewrite. `patch_json` is a shallow merge accepted
/// by `EditorCommand::PatchNodeData` and keeps the target node id in place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageFallbackPatch {
    pub node_id: String,
    pub branch: ImageFallbackBranch,
    pub patch_json: String,
}

/// Resolve the active page's absolute node rectangles through the same jian
/// layout scene used by geometry cleanup and image-target collection.
pub fn resolved_rects(state: &EditorState) -> HashMap<String, ResolvedRect> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let mut rects = HashMap::new();
    if let Some(page) = scene.active_page() {
        collect_rects(&page.children, &mut rects);
    }
    rects
}

fn collect_rects(nodes: &[SceneNode], rects: &mut HashMap<String, ResolvedRect>) {
    for node in nodes {
        let bounds = node.aggregate_bounds();
        rects.insert(
            node.id.clone(),
            ResolvedRect {
                x: f64::from(bounds.origin.x),
                y: f64::from(bounds.origin.y),
                width: f64::from(bounds.size.x),
                height: f64::from(bounds.size.y),
            },
        );
        collect_rects(&node.children, rects);
    }
}

/// Return every image-slot patch that should be applied to `root`.
///
/// Failed sentinels are always eligible. Empty and SVG placeholders are only
/// eligible after enrichment, so cleanup can leave genuine pending slots for
/// the provider pass. Resolved `op-image:` sources never match.
pub fn image_fallback_policy(
    root: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    after_enrich: bool,
) -> Vec<ImageFallbackPatch> {
    let mut patches = Vec::new();
    collect_patches(root, rects, after_enrich, &mut patches);
    patches
}

fn collect_patches(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    after_enrich: bool,
    patches: &mut Vec<ImageFallbackPatch>,
) {
    if let Some(patch) = fallback_patch(node, rects, after_enrich) {
        patches.push(patch);
        return;
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_patches(child, rects, after_enrich, patches);
        }
    }
}

fn fallback_patch(
    node: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    after_enrich: bool,
) -> Option<ImageFallbackPatch> {
    if !matches!(
        node,
        PenNode::Image(_) | PenNode::Frame(_) | PenNode::Rectangle(_)
    ) {
        return None;
    }
    if node
        .base()
        .name
        .as_deref()
        .is_some_and(|name| name.ends_with(IMAGE_FALLBACK_NAME_SUFFIX))
    {
        return None;
    }
    if !is_failed_image_slot(node, after_enrich) {
        return None;
    }

    let resolved_width = rects
        .get(node.id_str())
        .map(|rect| rect.width)
        .filter(|width| width.is_finite() && *width > 0.0)
        .or_else(|| node.width_px())
        .unwrap_or(0.0);
    if !resolved_width.is_finite() || resolved_width <= 0.0 {
        return None;
    }
    let branch = if resolved_width <= 96.0 {
        ImageFallbackBranch::Thumb
    } else {
        ImageFallbackBranch::Media
    };
    let search_query = search_query(node);
    let caption = caption_for_query(&search_query);
    let icon_name = icon_name_for_query(&search_query);
    let name = fallback_name(node.base().name.as_deref());
    let patch = match branch {
        ImageFallbackBranch::Thumb => thumbnail_patch(node.id_str(), &name, &icon_name),
        ImageFallbackBranch::Media => {
            media_patch(node.id_str(), &name, &icon_name, &caption, resolved_width)
        }
    };
    Some(ImageFallbackPatch {
        node_id: node.id_str().to_string(),
        branch,
        patch_json: patch.to_string(),
    })
}

fn is_failed_image_slot(node: &PenNode, after_enrich: bool) -> bool {
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
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string(),
        PenNode::Rectangle(_) => String::new(),
        _ => String::new(),
    }
}

/// Map the slot query to an icon name from the shipped Lucide catalog.
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

fn solid_fill(color: &str) -> Value {
    json!([{ "type": "solid", "color": color }])
}

fn thumbnail_patch(node_id: &str, name: &str, icon_name: &str) -> Value {
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
            "id": format!("{node_id}-image-fallback-icon"),
            "name": "Image fallback icon",
            "iconFontFamily": "lucide",
            "iconFontName": icon_name,
            "width": 20,
            "height": 20,
            "fill": solid_fill("$--muted-foreground")
        }],
        "imageSearchQuery": null,
        "imagePrompt": null,
        "explain": "image fallback: thumb"
    })
}

fn media_patch(node_id: &str, name: &str, icon_name: &str, caption: &str, width: f64) -> Value {
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
                "id": format!("{node_id}-image-fallback-icon"),
                "name": "Image fallback icon",
                "iconFontFamily": "lucide",
                "iconFontName": icon_name,
                "width": 30,
                "height": 30,
                "fill": solid_fill("$--muted-foreground")
            },
            {
                "type": "text",
                "id": format!("{node_id}-image-fallback-caption"),
                "name": "Image fallback caption",
                "content": caption,
                "width": width.clamp(1.0, 160.0),
                "height": 16,
                "fontSize": 12,
                "textAlign": "center",
                "fill": solid_fill("$--muted-foreground")
            }
        ],
        "imageSearchQuery": null,
        "imagePrompt": null,
        "explain": "image fallback: media"
    })
}

/// Apply the same policy to every page after an enrichment run and restore the
/// caller's active page. This is the host-facing hook for late enrichment.
pub fn apply_image_fallback_policy_to_state(state: &mut EditorState, after_enrich: bool) -> usize {
    let original_page = state.ui.active_page_index;
    let mut applied = 0;
    for page_index in 0..state.page_count() {
        if !state.set_active_page(page_index) && state.ui.active_page_index != page_index {
            continue;
        }
        let rects = resolved_rects(state);
        let roots: Vec<PenNode> = state.active_children().to_vec();
        let patches: Vec<_> = roots
            .iter()
            .flat_map(|root| image_fallback_policy(root, &rects, after_enrich))
            .collect();
        for patch in patches {
            if state.apply(EditorCommand::PatchNodeData {
                node_id: op_editor_core::NodeId::new(patch.node_id),
                patch_json: patch.patch_json,
                page_id: None,
            }) {
                applied += 1;
            }
        }
    }
    let _ = state.set_active_page(original_page);
    applied
}

#[cfg(test)]
#[path = "image_fallback_policy_tests.rs"]
mod tests;
