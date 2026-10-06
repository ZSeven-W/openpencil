//! Shared node predicates + padding readers for the mobile content-rail
//! repair — split from `mobile_content_rail.rs` (800-line file cap).
//! Detection only: mobile-screen shape, chrome/full-bleed roles, surface
//! transparency, scroller shapes, and the padding encode/decode helpers.

use super::*;
use jian_ops_schema::node::{container::ContainerProps, Padding};
use std::collections::BTreeMap;

pub(super) fn looks_like_mobile_screen(root: &PenNode) -> bool {
    let Some(props) = container_props(root) else {
        return false;
    };
    let Some(SizingBehavior::Number(width)) = props.width else {
        return false;
    };
    if !(MIN_MOBILE_WIDTH..=MAX_MOBILE_WIDTH).contains(&width)
        || props.layout != Some(LayoutMode::Vertical)
    {
        return false;
    }
    let Some(children) = root.children() else {
        return false;
    };
    let numeric_min_height_is_mobile = props
        .limits
        .min_height
        .is_some_and(|height| height.is_finite() && height >= 568.0);
    let tall_or_screen_structured = numeric_min_height_is_mobile
        || match props.height {
            Some(SizingBehavior::Number(height)) => height >= 568.0,
            _ => children.len() >= 4 || children.iter().any(is_mobile_chrome),
        };
    tall_or_screen_structured && children.len() >= 2
}

pub(super) fn infer_content_rail(sections: &[PenNode]) -> f64 {
    let mut counts: BTreeMap<i64, usize> = BTreeMap::new();
    for section in sections {
        if is_mobile_chrome(section)
            || is_intentional_full_bleed_role(section)
            || !is_transparent_surface(section)
        {
            continue;
        }
        let Some((left, right)) = horizontal_padding(section) else {
            continue;
        };
        if (left - right).abs() > 0.5 || !(MIN_CONTENT_RAIL..=MAX_CONTENT_RAIL).contains(&left) {
            continue;
        }
        *counts.entry(left.round() as i64).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(rail, count)| (*count, *rail))
        .map(|(rail, _)| rail as f64)
        .unwrap_or(DEFAULT_MOBILE_RAIL)
}

pub(super) fn is_mobile_chrome(node: &PenNode) -> bool {
    let role = node
        .base()
        .role
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if matches!(
        role.as_str(),
        "status-bar"
            | "bottom-tab-bar"
            | "bottom-nav"
            | "bottom-navigation-bar"
            | "tab-bar"
            | "tabbar"
    ) {
        return true;
    }
    let name = node
        .base()
        .name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    name.contains("status bar")
        || name.contains("bottom navigation")
        || name.contains("bottom nav")
        || name.contains("bottom tab")
}

pub(super) fn is_intentional_full_bleed_role(node: &PenNode) -> bool {
    matches!(
        node.base()
            .role
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "hero" | "banner" | "cover" | "header" | "top-nav" | "navbar"
    )
}

pub(super) fn is_transparent_surface(node: &PenNode) -> bool {
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    let has_fill = value
        .get("fill")
        .and_then(|fill| fill.as_array())
        .is_some_and(|fill| !fill.is_empty());
    let has_stroke = value.get("stroke").is_some_and(|stroke| !stroke.is_null());
    let has_effects = value
        .get("effects")
        .and_then(|effects| effects.as_array())
        .is_some_and(|effects| !effects.is_empty());
    let has_radius = value
        .get("cornerRadius")
        .and_then(|radius| radius.as_f64())
        .is_some_and(|radius| radius > 0.0);
    !has_fill && !has_stroke && !has_effects && !has_radius
}

pub(super) fn is_edge_spanning_insettable_surface(node: &PenNode, root_width: f64) -> bool {
    let Some(props) = container_props(node) else {
        return false;
    };
    if has_authored_position(node) {
        return false;
    }
    let spans_width = matches!(
        props.width.as_ref(),
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ) || matches!(
        props.width.as_ref(),
        Some(SizingBehavior::Number(width)) if *width >= root_width - 1.0
    );
    if !spans_width {
        return false;
    }
    let Ok(value) = serde_json::to_value(node) else {
        return false;
    };
    let has_stroke = value.get("stroke").is_some_and(|stroke| !stroke.is_null());
    let has_effects = value
        .get("effects")
        .and_then(|effects| effects.as_array())
        .is_some_and(|effects| !effects.is_empty());
    let has_radius = value
        .get("cornerRadius")
        .and_then(|radius| radius.as_f64())
        .is_some_and(|radius| radius > 0.0);
    has_stroke || has_effects || has_radius
}

pub(super) fn has_authored_position(node: &PenNode) -> bool {
    serde_json::to_value(node).ok().is_some_and(|value| {
        value.get("x").is_some_and(|x| !x.is_null()) || value.get("y").is_some_and(|y| !y.is_null())
    })
}

pub(super) fn has_full_bleed_media_child(node: &PenNode, root_width: f64) -> bool {
    node.children().is_some_and(|children| {
        children.iter().any(|child| {
            let role_is_media = matches!(
                child
                    .base()
                    .role
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .to_ascii_lowercase()
                    .as_str(),
                "hero" | "banner" | "cover" | "media" | "image-placeholder"
            );
            let image_like = matches!(child, PenNode::Image(_))
                || serde_json::to_value(child)
                    .ok()
                    .and_then(|value| value.get("fill").cloned())
                    .and_then(|fill| fill.as_array().cloned())
                    .is_some_and(|fills| {
                        fills.iter().any(|fill| {
                            fill.get("type").and_then(|kind| kind.as_str()) == Some("image")
                        })
                    });
            (role_is_media || image_like) && node_spans_width(child, root_width)
        })
    })
}

fn node_spans_width(node: &PenNode, root_width: f64) -> bool {
    container_props(node)
        .and_then(|props| props.width.as_ref())
        .is_some_and(|width| {
            matches!(width, SizingBehavior::Keyword(SizingKeyword::FillContainer))
                || matches!(width, SizingBehavior::Number(width) if *width >= root_width - 1.0)
        })
        || matches!(node, PenNode::Image(image) if {
            matches!(
                image.width.as_ref(),
                Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
            ) || matches!(
                image.width.as_ref(),
                Some(SizingBehavior::Number(width)) if *width >= root_width - 1.0
            )
        })
}

pub(super) fn has_text_or_icon_descendant(node: &PenNode) -> bool {
    matches!(node, PenNode::Text(_) | PenNode::IconFont(_))
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_text_or_icon_descendant))
}

pub(super) fn has_surface_content_descendant(node: &PenNode) -> bool {
    matches!(
        node,
        PenNode::Text(_) | PenNode::IconFont(_) | PenNode::Image(_)
    ) || node
        .children()
        .is_some_and(|children| children.iter().any(has_surface_content_descendant))
}

pub(super) fn is_clipped_horizontal_scroller(node: &PenNode) -> bool {
    container_props(node).is_some_and(|props| {
        props.layout == Some(LayoutMode::Horizontal) && props.clip_content == Some(true)
    })
}

pub(super) fn contains_clipped_horizontal_scroller(node: &PenNode) -> bool {
    if is_clipped_horizontal_scroller(node) {
        return true;
    }

    // Only transparent structural wrappers can pass scroller ownership up to
    // an ancestor section. Surfaced cards often contain clipped horizontal
    // progress meters; treating those meters as page rails suppresses the
    // card group's own mobile content inset.
    is_transparent_surface(node)
        && node
            .children()
            .is_some_and(|children| children.iter().any(contains_clipped_horizontal_scroller))
}

pub(super) fn is_scroller_header(node: &PenNode) -> bool {
    if !node.is_container() || !is_transparent_surface(node) || !has_text_or_icon_descendant(node) {
        return false;
    }
    let name = node
        .base()
        .name
        .as_deref()
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.contains("header") || name.contains("title") {
        return true;
    }
    let child_count = node.children().map_or(0, |children| children.len());
    child_count <= 3
        && container_props(node).is_some_and(|props| {
            props.layout == Some(LayoutMode::Horizontal)
                && props.height.as_ref().is_none_or(|height| match height {
                    SizingBehavior::Number(height) => *height <= 80.0,
                    _ => true,
                })
        })
}

pub(super) fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(node) => Some(&node.container),
        PenNode::Group(node) => Some(&node.container),
        PenNode::Rectangle(node) => Some(&node.container),
        _ => None,
    }
}

pub(super) fn horizontal_padding(node: &PenNode) -> Option<(f64, f64)> {
    match container_props(node)?.padding.as_ref()? {
        Padding::Uniform(value) => Some((*value, *value)),
        Padding::XY([_, horizontal]) => Some((*horizontal, *horizontal)),
        Padding::LtrB([_, right, _, left]) => Some((*left, *right)),
        Padding::Expression(_) => None,
    }
}

pub(super) fn has_expression_padding(node: &PenNode) -> bool {
    matches!(
        container_props(node).and_then(|props| props.padding.as_ref()),
        Some(Padding::Expression(_))
    )
}

pub(super) fn vertical_padding(node: &PenNode) -> (f64, f64) {
    match container_props(node).and_then(|props| props.padding.as_ref()) {
        Some(Padding::Uniform(value)) => (*value, *value),
        Some(Padding::XY([vertical, _])) => (*vertical, *vertical),
        Some(Padding::LtrB([top, _, bottom, _])) => (*top, *bottom),
        Some(Padding::Expression(_)) | None => (0.0, 0.0),
    }
}

pub(super) fn nonzero_pair((left, right): (f64, f64)) -> bool {
    left > 0.0 || right > 0.0
}

pub(super) fn padding_with_horizontal_rail(node: &PenNode, rail: f64) -> Vec<f64> {
    let (top, bottom) = vertical_padding(node);
    vec![top, rail, bottom, rail]
}

pub(super) fn padding_with_leading_rail(node: &PenNode, rail: f64) -> Vec<f64> {
    let (top, bottom) = vertical_padding(node);
    let right = horizontal_padding(node).map_or(0.0, |(_, right)| right);
    vec![top, right, bottom, rail]
}
