//! Query extraction for detected image slots: explicit bindings,
//! placeholder labels, node names, and ancestor semantic names. Carved out
//! of the `targets.rs` spine to keep it under the 800-line cap.

use jian_ops_schema::node::{PenNode, TextContent};
use op_editor_core::PenNodeExt as _;

use super::is_image_placeholder_frame;

/// Preserve provenance at extraction time; a slug-shaped explicit query is
/// still authored intent, whereas any node-name fallback can lack a subject.
pub(super) struct ExtractedQuery {
    pub text: String,
    pub from_node_name: bool,
}

impl ExtractedQuery {
    pub fn subject(text: String) -> Self {
        Self {
            text,
            from_node_name: false,
        }
    }

    fn node_name(text: String) -> Self {
        Self {
            text,
            from_node_name: true,
        }
    }
}

pub(super) fn extract_query_for_node(node: &PenNode, parent_names: &[String]) -> ExtractedQuery {
    if let PenNode::Image(image) = node {
        if let Some(query) = image
            .image_search_query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            return ExtractedQuery::subject(query.to_string());
        }
    }

    if let PenNode::Frame(frame) = node {
        if let Some(query) = frame
            .image_search_query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            return ExtractedQuery::subject(query.to_string());
        }
    }

    if is_image_placeholder_frame(node) {
        if let Some(label) = placeholder_label_text(node) {
            return ExtractedQuery::subject(label);
        }
    }

    if let Some(name) = node
        .base()
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if !is_generic_placeholder_name(name) {
            return ExtractedQuery::node_name(name.to_string());
        }
    }

    if let Some(parent_name) = parent_semantic_name(parent_names) {
        return ExtractedQuery::subject(parent_name);
    }

    ExtractedQuery::node_name(
        node.base()
            .name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or("placeholder")
            .to_string(),
    )
}

fn placeholder_label_text(node: &PenNode) -> Option<String> {
    let children = node.children()?;
    for child in children {
        let PenNode::Text(text) = child else {
            continue;
        };
        if text.base.role.as_deref() != Some("image-placeholder-label") {
            continue;
        }
        let label = match &text.content {
            TextContent::Plain(content) => content.trim().to_string(),
            TextContent::Styled(segments) => segments
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<String>()
                .trim()
                .to_string(),
        };
        if !label.is_empty() {
            return Some(label);
        }
    }
    None
}

pub(super) fn parent_semantic_name(parent_names: &[String]) -> Option<String> {
    parent_names.iter().take(3).find_map(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty()
            || is_generic_placeholder_name(trimmed)
            || is_layout_context_name(trimmed)
        {
            return None;
        }
        Some(trimmed.to_string())
    })
}

fn is_generic_placeholder_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "image"
            | "photo"
            | "cover"
            | "hero"
            | "thumbnail"
            | "thumb"
            | "picture"
            | "banner"
            | "poster"
            | "image placeholder"
            | "placeholder icon"
            | "placeholder"
            // A slot named "img" / "ph" carries no subject of its own — the
            // picture it wants is named by the card AROUND it ("Santorini").
            | "img"
            | "ph"
            | "pic"
            | "media"
            | "graphic"
            | "card image"
            | "card photo"
            | "product image"
            | "item image"
    )
}

fn is_layout_context_name(name: &str) -> bool {
    name.split(|c: char| !c.is_ascii_alphanumeric())
        .map(str::to_ascii_lowercase)
        .any(|word| {
            matches!(
                word.as_str(),
                "card"
                    | "wrapper"
                    | "container"
                    | "section"
                    | "frame"
                    | "root"
                    | "page"
                    | "stack"
                    | "row"
                    | "column"
                    | "content"
            )
        })
}

/// Page context can keep descriptive container names ("Selected Work Section")
/// even though those names are too broad to make good stock-search keywords.
/// Share the placeholder vocabulary above; reject only default layout names.
pub(super) fn page_context_name(name: &str) -> Option<String> {
    let normalized = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() || is_generic_placeholder_name(&normalized) {
        return None;
    }
    let words: Vec<_> = normalized
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let is_default = words.iter().all(|word| {
        let base = word
            .trim_end_matches(|c: char| c.is_ascii_digit())
            .to_ascii_lowercase();
        base.is_empty()
            || is_layout_context_name(&base)
            || matches!(
                base.as_str(),
                "group" | "layer" | "rectangle" | "untitled" | "document" | "doc"
            )
    });
    (!is_default).then_some(normalized)
}
