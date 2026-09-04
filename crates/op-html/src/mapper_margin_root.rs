//! Root-element and body margins on the synthesized document frame.

use crate::css::cascade::ComputedStyle;
use crate::import_warning::ImportWarning;
use jian_ops_schema::node::container::ContainerProps;

use super::{padding_edges, padding_from, resolve_edges, CollapseSet, CollapsedMargins, Edges};
use crate::mapper::MapCtx;

/// `<body>` is emitted as the document root, so there is no outer node that
/// can carry either the root element's or body's margin. Root margins form the
/// outer inset; body margins (after child collapse) add inside them.
pub(crate) fn apply_root_margins(
    container: &mut ContainerProps,
    document_style: &ComputedStyle,
    body_style: &ComputedStyle,
    collapsed: CollapsedMargins,
    context: &mut MapCtx<'_>,
) {
    let mut document_edges = resolve_edges(document_style, context);
    if document_edges.has_negative() {
        context.warn_once(ImportWarning::NegativeMarginsIgnored);
        document_edges = document_edges.non_negative();
    }
    let mut body_edges = if matches!(body_style.get("position"), Some("absolute" | "fixed")) {
        Edges::default()
    } else {
        let mut edges = resolve_edges(body_style, context);
        let mut top = CollapseSet::from_value(edges.top).merge(collapsed.leading);
        let mut bottom = CollapseSet::from_value(edges.bottom).merge(collapsed.trailing);
        if collapsed.through {
            top = top.merge(bottom);
            bottom = CollapseSet::default();
        }
        edges.top = top.value();
        edges.bottom = bottom.value();
        edges
    };
    if body_edges.has_negative() {
        context.warn_once(ImportWarning::NegativeMarginsIgnored);
        body_edges = body_edges.non_negative();
    }
    let edges = Edges {
        top: document_edges.top + body_edges.top,
        right: document_edges.right + body_edges.right,
        bottom: document_edges.bottom + body_edges.bottom,
        left: document_edges.left + body_edges.left,
    };
    if !edges.any() {
        return;
    }
    let current = padding_edges(container.padding.as_ref());
    container.padding = padding_from(Edges {
        top: current.top + edges.top,
        right: current.right + edges.right,
        bottom: current.bottom + edges.bottom,
        left: current.left + edges.left,
    });
}
