//! Empty-header adoption heuristics — split from `mobile_content_rail.rs`
//! (800-line file cap). Finds the narrow generated failure shape where an
//! empty authored header shell is immediately followed by the brand title
//! and compact action that clearly belong to it.

use super::*;

#[derive(Debug)]
pub(super) struct HeaderAdoption {
    pub(super) header_id: String,
    pub(super) children: Vec<(String, usize)>,
}

/// Finds the narrow, generated failure shape where an empty authored header
/// shell is immediately followed by the content that clearly belongs to it.
/// Both a brand title and a compact semantic action are required, and only a
/// search rail may sit between them. This deliberately declines plausible but
/// ambiguous reconstruction instead of guessing at document ownership.
pub(super) fn collect_header_adoption(sections: &[PenNode]) -> Option<HeaderAdoption> {
    let candidates: Vec<(usize, &PenNode)> = sections
        .iter()
        .enumerate()
        .filter(|(_, node)| is_empty_header_shell(node))
        .collect();
    let [(header_index, header)] = candidates.as_slice() else {
        return None;
    };

    let mut brands = Vec::new();
    let mut actions = Vec::new();
    for (index, node) in sections
        .iter()
        .enumerate()
        .skip(*header_index + 1)
        .take(MAX_HEADER_NEIGHBOR_SIBLINGS)
    {
        if is_brand_title(node) {
            brands.push((node.id_str().to_string(), index));
        } else if is_compact_header_action(node) {
            actions.push((node.id_str().to_string(), index));
        } else if !is_search_rail(node) {
            break;
        }
    }

    let ([brand], [action]) = (brands.as_slice(), actions.as_slice()) else {
        return None;
    };
    if brand.1 >= action.1 {
        return None;
    }

    Some(HeaderAdoption {
        header_id: header.id_str().to_string(),
        children: vec![brand.clone(), action.clone()],
    })
}

pub(super) fn is_empty_header_shell(node: &PenNode) -> bool {
    if is_mobile_chrome(node)
        || !is_transparent_surface(node)
        || has_authored_position(node)
        || node.children().is_none_or(|children| !children.is_empty())
    {
        return false;
    }
    let Some(props) = container_props(node) else {
        return false;
    };
    if props.layout != Some(LayoutMode::Horizontal)
        || !matches!(
            props.width.as_ref(),
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        )
        || node.height_px().is_some_and(|height| height > 96.0)
    {
        return false;
    }

    let semantic = semantic_label(node);
    semantic_has_any(&semantic, &["header", "navbar", "nav", "navigation"])
}

fn is_brand_title(node: &PenNode) -> bool {
    let PenNode::Text(text) = node else {
        return false;
    };
    if has_authored_position(node) || !(18.0..=40.0).contains(&text.font_size.unwrap_or(16.0)) {
        return false;
    }
    semantic_has_any(&semantic_label(node), &["brand", "logo", "wordmark"])
}

pub(super) fn is_compact_header_action(node: &PenNode) -> bool {
    if has_authored_position(node) {
        return false;
    }
    let semantic = match node {
        PenNode::IconFont(icon) => format!(
            "{} {}",
            semantic_label(node),
            icon.icon_font_name.to_ascii_lowercase()
        ),
        _ => semantic_label(node),
    };
    if !semantic_has_any(
        &semantic,
        &[
            "action",
            "button",
            "cart",
            "bag",
            "menu",
            "profile",
            "account",
            "avatar",
            "notification",
            "bell",
            "favorite",
            "wishlist",
            "search",
        ],
    ) {
        return false;
    }

    match node {
        PenNode::IconFont(icon) => {
            let icon_semantic = icon.icon_font_name.to_ascii_lowercase();
            semantic_has_any(
                &format!("{semantic} {icon_semantic}"),
                &[
                    "cart",
                    "bag",
                    "menu",
                    "profile",
                    "account",
                    "avatar",
                    "notification",
                    "bell",
                    "favorite",
                    "wishlist",
                    "search",
                ],
            ) && node.width_px().is_some_and(|width| width <= 48.0)
                && node.height_px().is_some_and(|height| height <= 48.0)
        }
        _ if node.is_container() => {
            node.width_px()
                .is_some_and(|width| width > 0.0 && width <= 64.0)
                && node
                    .height_px()
                    .is_some_and(|height| height > 0.0 && height <= 64.0)
                && node
                    .children()
                    .is_some_and(|children| !children.is_empty() && children.len() <= 2)
                && has_icon_descendant(node)
                && !has_text_descendant(node)
        }
        _ => false,
    }
}

fn is_search_rail(node: &PenNode) -> bool {
    node.is_container()
        && semantic_has_any(&semantic_label(node), &["search"])
        && container_props(node).is_some_and(|props| {
            matches!(
                props.width.as_ref(),
                Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
            )
        })
}

pub(super) fn is_ordinary_root_leaf(node: &PenNode) -> bool {
    if !matches!(node, PenNode::Text(_) | PenNode::IconFont(_))
        || has_authored_position(node)
        || is_mobile_chrome(node)
        || is_intentional_full_bleed_role(node)
    {
        return false;
    }
    let semantic = semantic_label(node);
    !has_system_chrome_semantics(&semantic)
        && !semantic_has_any(
            &semantic,
            &[
                "header", "navbar", "brand", "logo", "wordmark", "hero", "banner", "cover",
            ],
        )
        && !is_compact_header_action(node)
}

fn has_system_chrome_semantics(semantic: &str) -> bool {
    matches!(
        semantic.trim(),
        "time"
            | "wifi"
            | "wi-fi"
            | "cellular"
            | "cellular connection"
            | "battery"
            | "battery capacity"
            | "system status"
            | "status icon"
    )
}

fn has_icon_descendant(node: &PenNode) -> bool {
    matches!(node, PenNode::IconFont(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_icon_descendant))
}

fn has_text_descendant(node: &PenNode) -> bool {
    matches!(node, PenNode::Text(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_text_descendant))
}

fn semantic_label(node: &PenNode) -> String {
    format!(
        "{} {}",
        node.base().role.as_deref().unwrap_or(""),
        node.base().name.as_deref().unwrap_or("")
    )
    .trim()
    .to_ascii_lowercase()
}

fn semantic_has_any(semantic: &str, candidates: &[&str]) -> bool {
    semantic
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| candidates.contains(&word))
}
