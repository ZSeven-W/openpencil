//! Expand explicit vertical scrolling surfaces in an ephemeral static export.
//!
//! Keep the canonical device viewport intact. Re-layout a copy with measured
//! content heights so the footer follows the complete body instead of covering
//! its last visible row. Ordinary clipped cards and horizontal carousels remain
//! clipped. Preview and the professional editing canvas keep authored geometry;
//! normal-mode reading and full-content exports use this projection.

use jian_ops_schema::{
    node::{
        container::{ContainerProps, JustifyContent, LayoutMode, Padding},
        PenNode,
    },
    sizing::SizingBehavior,
};
use op_editor_core::PenNodeExt;

use crate::payload::NodePayload;

pub(crate) fn roots(roots: &[PenNode]) -> Option<Vec<PenNode>> {
    if !roots.iter().any(has_vertical_scroll) {
        return None;
    }
    let page = crate::adapter::pen_roots_to_page_payload("export", "Export", roots, 0, false);
    let mut projected = roots.to_vec();
    for (node, measured) in projected.iter_mut().zip(&page.children) {
        expand(node, measured);
    }
    (projected != roots).then_some(projected)
}

fn props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&n.container),
        PenNode::Group(n) => Some(&n.container),
        PenNode::Rectangle(n) => Some(&n.container),
        _ => None,
    }
}

fn vertical_scroll(node: &PenNode) -> bool {
    matches!(
        node.base().role.as_deref(),
        Some("scroll" | "scroll-area" | "scroll-viewport")
    ) && props(node).is_some_and(|p| {
        p.layout == Some(LayoutMode::Vertical)
            && p.clip_content == Some(true)
            && matches!(p.justify_content, None | Some(JustifyContent::Start))
    })
}

fn has_vertical_scroll(node: &PenNode) -> bool {
    vertical_scroll(node)
        || node
            .children()
            .is_some_and(|children| children.iter().any(has_vertical_scroll))
}

fn vertical_padding(props: &ContainerProps) -> Option<(f32, f32)> {
    let (top, bottom) = match props.padding.as_ref() {
        None => (0.0, 0.0),
        Some(Padding::Uniform(n)) => (*n, *n),
        Some(Padding::XY([vertical, _])) => (*vertical, *vertical),
        Some(Padding::LtrB([top, _, bottom, _])) => (*top, *bottom),
        Some(Padding::Expression(_)) => return None,
    };
    (top.is_finite() && top >= 0.0 && bottom.is_finite() && bottom >= 0.0)
        .then_some((top as f32, bottom as f32))
}

fn visible_bottom(node: &NodePayload) -> f32 {
    let bottom = node.y + node.h;
    if node.clip_content {
        bottom
    } else {
        node.children
            .iter()
            .map(visible_bottom)
            .fold(bottom, f32::max)
    }
}

/// Return growth for the parent's normal flow. Absolute/rotated surfaces and
/// size-limited ancestors keep their existing semantics rather than guessing a
/// new relationship between independent layers.
fn expand(node: &mut PenNode, measured: &NodePayload) -> f32 {
    if node.base().visible == Some(false)
        || node.base().rotation.is_some_and(|angle| angle != 0.0)
        || node.children().is_none()
    {
        return 0.0;
    }
    let Some(container) = props(node) else {
        return 0.0;
    };
    let vertical = container.layout == Some(LayoutMode::Vertical);
    if (!vertical && container.layout != Some(LayoutMode::Horizontal))
        || container.limits != Default::default()
        || measured.h <= 0.0
        || !measured.h.is_finite()
    {
        return 0.0;
    }
    let Some((top_inset, bottom_inset)) = vertical_padding(container) else {
        return 0.0;
    };
    let scroll = vertical_scroll(node);
    let Some(children) = node.children_mut() else {
        return 0.0;
    };
    let mut flow_growth = 0.0_f32;
    let mut content_bottom = measured.y;
    let mut row_height = measured.h;
    for child in children {
        if child.base().visible == Some(false)
            || child.base().x.is_some()
            || child.base().y.is_some()
            || child.base().constraints.is_some()
        {
            continue;
        }
        let Some(rect) = measured.children.iter().find(|m| m.id == child.base().id) else {
            continue;
        };
        let growth = expand(child, rect);
        let shift = if vertical { flow_growth } else { 0.0 };
        content_bottom = content_bottom.max(visible_bottom(rect) + growth + shift);
        row_height = row_height.max(rect.h + growth + top_inset + bottom_inset);
        if vertical {
            flow_growth += growth;
        } else {
            flow_growth = flow_growth.max(growth);
        }
    }
    // Non-scroll parents only grow to carry an expanded descendant. A plain
    // overflowing artboard/card is not permission to discard its clipping.
    let height = if scroll {
        (content_bottom - measured.y + bottom_inset).max(measured.h)
    } else if flow_growth <= 0.5 {
        measured.h
    } else if vertical {
        (content_bottom - measured.y + bottom_inset).max(measured.h)
    } else {
        row_height
    };
    let growth = height - measured.h;
    if !growth.is_finite() || growth <= 0.5 {
        return 0.0;
    }
    let container = match node {
        PenNode::Frame(n) => &mut n.container,
        PenNode::Group(n) => &mut n.container,
        PenNode::Rectangle(n) => &mut n.container,
        _ => return 0.0,
    };
    container.height = Some(SizingBehavior::Number(height as f64));
    growth
}

#[cfg(test)]
#[path = "full_content_snapshot_tests.rs"]
mod tests;
