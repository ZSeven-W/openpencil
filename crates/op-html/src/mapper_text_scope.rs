use jian_ops_schema::node::container::ContainerProps;
use jian_ops_schema::node::PenNode;

use crate::css::cascade::{
    compute_pseudo_style_for_viewport, compute_style_for_viewport, ComputedStyle,
};
use crate::css::selectors::PseudoElement;
use crate::dom::{DomElement, DomNode};

use super::{visual, MapCtx};

/// Public compatibility entry point. Gradient-text state is private to the
/// recursive mapper and no longer changes the shape of [`MapCtx`].
pub fn map_element(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    parent_style: Option<&ComputedStyle>,
) -> Option<PenNode> {
    super::map_element_scoped(context, path, parent_style, parent_style, None)
}

/// Public compatibility entry point for callers that only need box mapping.
pub fn container_props_from(style: &ComputedStyle, context: &mut MapCtx<'_>) -> ContainerProps {
    super::container_props_from_impl(style, context, false)
}

/// Snapshot import transfers text-clipped fills using captured descendant
/// glyph data, so its box pass may suppress the warning before that transfer.
pub(crate) fn snapshot_container_props(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
) -> ContainerProps {
    super::container_props_from_impl(style, context, true)
}

pub(crate) fn container_props_with_text_scope(
    style: &ComputedStyle,
    context: &mut MapCtx<'_>,
    can_transfer: bool,
) -> (ContainerProps, Option<String>) {
    let mut container = super::container_props_from_impl(style, context, can_transfer);
    let fill = can_transfer
        .then(|| visual::take_text_clip_fill(style, &mut container.fill))
        .flatten();
    (container, fill)
}

pub(crate) fn map_container_children(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
    parent_style: Option<&ComputedStyle>,
    dom_children: &[DomNode],
    text_fill_override: Option<&str>,
) -> MappedChildren {
    let mut children =
        crate::text::map_children(context, path, style, dom_children, text_fill_override);
    let collapsed = super::margin::finish_children(context, style, parent_style, &mut children);
    if matches!(
        style.get("flex-direction"),
        Some("row-reverse" | "column-reverse")
    ) {
        children.reverse();
    }
    children = super::stack::layer_absolute_children(children);
    MappedChildren {
        nodes: super::grid::wrap_grid_rows(context, style, children),
        collapsed,
    }
}

pub(crate) struct MappedChildren {
    pub nodes: Vec<PenNode>,
    pub collapsed: super::margin::CollapsedMargins,
}

pub(crate) fn subtree_allows_text_clip_transfer(
    context: &MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
    children: &[DomNode],
) -> bool {
    if !visual::background_clips_text(style) {
        return false;
    }
    !list_marker_has_partial_alpha(path, style)
        && !children_have_partial_alpha(context, path, style, children)
        && !pseudos_have_partial_alpha(context, path, style)
}

pub(crate) fn style_allows_text_clip_transfer(style: &ComputedStyle) -> bool {
    visual::background_clips_text(style) && !visual::text_paint_has_partial_alpha(style)
}

fn children_have_partial_alpha(
    context: &MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
    children: &[DomNode],
) -> bool {
    children.iter().any(|child| match child {
        DomNode::Text(text) => !text.is_empty() && visual::text_paint_has_partial_alpha(style),
        DomNode::Element(element) => {
            let mut child_path = path.to_vec();
            child_path.push(element);
            let child_style = compute_style_for_viewport(
                &child_path,
                context.rules,
                Some(style),
                context.opts.base_font_size,
                context.opts.viewport_width,
                context.opts.viewport_height(),
            );
            child_style.get("display") != Some("none")
                && (pseudos_have_partial_alpha(context, &child_path, &child_style)
                    || children_have_partial_alpha(
                        context,
                        &child_path,
                        &child_style,
                        &element.children,
                    ))
        }
    })
}

fn pseudos_have_partial_alpha(
    context: &MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
) -> bool {
    [PseudoElement::Before, PseudoElement::After]
        .into_iter()
        .any(|pseudo| {
            let pseudo_style = compute_pseudo_style_for_viewport(
                path,
                context.rules,
                Some(style),
                context.opts.base_font_size,
                pseudo,
                context.opts.viewport_width,
                context.opts.viewport_height(),
            );
            pseudo_is_visible(&pseudo_style)
                && pseudo_style.get("content").is_some_and(|content| {
                    !matches!(
                        content.trim().to_ascii_lowercase().as_str(),
                        "none" | "normal"
                    )
                })
                && visual::text_paint_has_partial_alpha(&pseudo_style)
        })
}

fn list_marker_has_partial_alpha(path: &[&DomElement], style: &ComputedStyle) -> bool {
    path.last().is_some_and(|element| element.tag == "li")
        && !crate::list_markers::marker_is_suppressed(style)
        && visual::text_paint_has_partial_alpha(style)
}

fn pseudo_is_visible(style: &ComputedStyle) -> bool {
    style.get("display") != Some("none")
        && !matches!(style.get("visibility"), Some("hidden" | "collapse"))
        && style
            .get("opacity")
            .and_then(|value| value.parse::<f64>().ok())
            .is_none_or(|opacity| opacity > 0.0)
}
