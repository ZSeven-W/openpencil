//! Reconcile a fixed phone's flexible viewport after intrinsic footer repair.
//!
//! An absolute child inside a fit-content wrapper can initially contribute no
//! height to Taffy. Child-bound repair recovers that wrapper, so the explicit
//! scroll viewport must give its extra allocation back before root reflow.

use super::*;

pub(super) fn reserve_intrinsic_footer(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
    is_root: bool,
) {
    if !is_root
        || !is_frame(node)
        || props.clip_content != Some(true)
        || props.layout != Some(LayoutMode::Vertical)
        || !start_justified(props)
        || !matches!(props.width, Some(SizingBehavior::Number(w)) if (280.0..=480.0).contains(&w))
        || !matches!(props.height, Some(SizingBehavior::Number(h)) if h.is_finite() && h >= 500.0)
        || matches!(props.padding, Some(Padding::Expression(_)))
        || matches!(props.gap, Some(NumberOrExpression::Expression(_)))
        || kids.len() < 2
        || kids.iter().any(|child| {
            !layout_child(child)
                || base(child).is_some_and(|base| {
                    base.x.is_some() || base.y.is_some() || base.constraints.is_some()
                })
        })
    {
        return;
    }

    let scroll = &kids[kids.len() - 2];
    let footer = &kids[kids.len() - 1];
    if !is_scroll_viewport(scroll)
        || kids.iter().filter(|child| has_fill_height(child)).count() != 1
        || !has_fill_height(scroll)
        || !container_props(footer).is_some_and(|footer| {
            footer.layout == Some(LayoutMode::Vertical)
                && matches!(
                    footer.height,
                    Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
                )
        })
    {
        return;
    }
    let (Some(parent), Some(viewport), Some(footer_rect)) =
        (rect(node, rects), rect(scroll, rects), rect(footer, rects))
    else {
        return;
    };
    if !(44.0..=128.0).contains(&footer_rect.h) {
        return;
    }
    let Some(other_heights) = kids
        .iter()
        .filter(|child| node_id(child) != node_id(scroll))
        .map(|child| rect(child, rects).map(|rect| rect.h))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let padding = padding_sides(props.padding.as_ref());
    let gap = gap_value(props);
    if gap < 0.0 || !gap.is_finite() {
        return;
    }
    let available = parent.h
        - padding.top
        - padding.bottom
        - gap * (kids.len() - 1) as f32
        - other_heights.iter().sum::<f32>();
    // Only relinquish excess allocation. Do not grow content or guess what to
    // sacrifice when the authored chrome itself leaves no usable viewport.
    if available.is_finite() && available >= 1.0 && available < viewport.h - 0.5 {
        set_height(scroll, rects, available);
    }
}

fn has_fill_height(node: &PenNode) -> bool {
    matches!(
        node_sizing(node).1,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    )
}

fn is_scroll_viewport(node: &PenNode) -> bool {
    container_props(node).is_some_and(|props| {
        props.clip_content == Some(true)
            && props.layout == Some(LayoutMode::Vertical)
            && start_justified(props)
            // This pass only returns excess height; it does not rerun Taffy.
            // Keep authored size limits and height-dependent distribution
            // with Taffy rather than invalidating their solved child rects.
            && props.limits == Default::default()
    }) && base(node).is_some_and(|base| {
        matches!(
            base.role.as_deref(),
            Some("scroll" | "scroll-area" | "scroll-viewport" | "viewport")
        )
    })
}

#[cfg(test)]
#[path = "layout_repair_scroll_tests.rs"]
mod tests;
