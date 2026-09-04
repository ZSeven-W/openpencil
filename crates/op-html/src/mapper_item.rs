//! Per-item post-passes shared by ordinary frames and replaced leaves.

use jian_ops_schema::node::base::PenNodeBase;
use jian_ops_schema::node::container::LayoutMode;
use jian_ops_schema::node::PenNode;

use crate::css::cascade::ComputedStyle;

use super::{grid_place, layout_heuristics, margin, node_access, offset, wrap};
use super::{BaseStyleOutcome, MapCtx};

fn parent_is_grid(parent_style: Option<&ComputedStyle>) -> bool {
    parent_style.is_some_and(|parent| matches!(parent.get("display"), Some("grid" | "inline-grid")))
}

/// Facts only the child's own computed style knows, stashed on the child so
/// the parent's post-pass can read them back. Every private carrier moves to
/// the outermost synthetic wrapper and is stripped by its owning post-pass.
pub(super) fn record_parent_hints(
    base: &mut PenNodeBase,
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
    context: &mut MapCtx<'_>,
) {
    if parent_is_grid(parent_style) {
        grid_place::record_placement(base, style, context);
    }
    if parent_style.is_some_and(wrap::is_wrapping_flex_row) {
        wrap::record_inline_margins(base, style, context);
    }
}

/// Apply the in-flow post-passes an ordinary element gets to a replaced leaf.
pub(super) fn finish_leaf(
    context: &mut MapCtx<'_>,
    mut node: PenNode,
    tag: &str,
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
    outcome: BaseStyleOutcome,
) -> PenNode {
    record_parent_hints(
        node_access::node_base_mut(&mut node),
        style,
        parent_style,
        context,
    );
    let node = offset::wrap_offset(context, node, outcome.flow_offset, outcome.reserved_box);
    let handled_by_parent = context.auto_margin_handled_by_parent;
    let node = align_by_auto_margins(context, node, style, parent_style, handled_by_parent);
    margin::wrap_margins(
        context,
        node,
        tag,
        style,
        parent_style,
        margin::CollapsedMargins::default(),
    )
}

/// CSS `margin-left/right:auto` becomes a full-width alignment row when the
/// parent's layout does not already provide the same placement.
pub(super) fn align_by_auto_margins(
    context: &mut MapCtx<'_>,
    node: PenNode,
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
    handled_by_parent: bool,
) -> PenNode {
    if handled_by_parent
        || matches!(style.get("position"), Some("absolute" | "fixed"))
        || layout_heuristics::is_inline_level(style)
        || !offset::parent_accepts_alignment_row(parent_style)
    {
        return node;
    }
    match offset::auto_margin_align(style, context) {
        Some(align) if !parent_align_covers(parent_style, align) => {
            offset::wrap_auto_margin(context, node, align)
        }
        _ => node,
    }
}

fn parent_align_covers(
    parent_style: Option<&ComputedStyle>,
    align: offset::AutoMarginAlign,
) -> bool {
    let Some(parent) = parent_style else {
        return false;
    };
    if layout_heuristics::layout_for(parent) == LayoutMode::Horizontal {
        return false;
    }
    let Some(items) = parent.get("align-items").map(str::trim) else {
        return false;
    };
    match align {
        offset::AutoMarginAlign::Center => items.eq_ignore_ascii_case("center"),
        offset::AutoMarginAlign::Start => {
            matches!(items.to_ascii_lowercase().as_str(), "flex-start" | "start")
        }
        offset::AutoMarginAlign::End => {
            matches!(items.to_ascii_lowercase().as_str(), "flex-end" | "end")
        }
    }
}
