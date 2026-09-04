//! Layout estimators used by cleanup passes.

use jian_ops_schema::node::{
    ContainerProps, LayoutMode, NumberOrExpression, Padding, PenNode, TextContent, TextNode,
};
use op_editor_core::PenNodeExt;

pub(crate) fn root_content_height(root: &PenNode) -> Option<i32> {
    let height = container_content_height(root)?;
    (height > 0.0).then_some(height.round() as i32)
}

fn intrinsic_height(node: &PenNode) -> Option<f64> {
    if let Some(height) = node.height_px().filter(|height| *height > 0.0) {
        return Some(height);
    }

    match node {
        PenNode::Frame(_) | PenNode::Group(_) | PenNode::Rectangle(_) => {
            container_content_height(node)
        }
        PenNode::Ref(n) => stacked_children_height(n.children.as_deref(), 0.0, 0.0),
        PenNode::Text(text) => Some(estimated_text_height(text)),
        PenNode::IconFont(_) => node.width_px().or(Some(24.0)),
        _ => None,
    }
}

fn container_content_height(node: &PenNode) -> Option<f64> {
    let children = node.children()?;
    if children.is_empty() {
        return None;
    }
    let padding_y = container_props(node).map(padding_y).unwrap_or(0.0);
    let gap = container_props(node)
        .and_then(|props| props.gap.as_ref())
        .and_then(number_value)
        .unwrap_or(0.0);
    let layout = container_props(node).and_then(|props| props.layout.as_ref());

    match layout {
        Some(LayoutMode::Horizontal) => children
            .iter()
            .filter_map(intrinsic_height)
            .reduce(f64::max)
            .map(|height| height + padding_y),
        // `layout: none` compiles to a single-cell grid (jian 2026-07-28), so
        // the children OVERLAY each other: the container is as tall as its
        // tallest layer, not the sum of them. Summing here made the
        // root-height repair inflate every overlay stack to N× its height.
        Some(LayoutMode::None) => children
            .iter()
            .filter_map(intrinsic_height)
            .reduce(f64::max)
            .map(|height| height + padding_y),
        _ => stacked_children_height(Some(children.as_slice()), gap, padding_y),
    }
}

fn stacked_children_height(children: Option<&[PenNode]>, gap: f64, padding_y: f64) -> Option<f64> {
    let children = children?;
    if children.is_empty() {
        return None;
    }

    let mut total = padding_y;
    let mut measured = 0usize;
    for child in children {
        let Some(height) = intrinsic_height(child) else {
            continue;
        };
        if measured > 0 {
            total += gap;
        }
        total += height;
        measured += 1;
    }
    (measured > 0).then_some(total)
}

fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&n.container),
        PenNode::Group(n) => Some(&n.container),
        PenNode::Rectangle(n) => Some(&n.container),
        _ => None,
    }
}

fn padding_y(props: &ContainerProps) -> f64 {
    match props.padding.as_ref() {
        Some(Padding::Uniform(v)) => v * 2.0,
        Some(Padding::XY([y, _])) => y * 2.0,
        Some(Padding::LtrB([top, _, bottom, _])) => top + bottom,
        Some(Padding::Expression(_)) | None => 0.0,
    }
}

fn number_value(value: &NumberOrExpression) -> Option<f64> {
    match value {
        NumberOrExpression::Number(v) => Some(*v),
        NumberOrExpression::Expression(_) => None,
    }
}

fn estimated_text_height(text: &TextNode) -> f64 {
    let font_size = text.font_size.unwrap_or(14.0).max(1.0);
    let line_height = match text.line_height {
        Some(value) if value > 4.0 => value,
        Some(value) if value > 0.0 => font_size * value,
        _ => font_size * 1.25,
    };
    line_height * text_line_count(&text.content) as f64
}

fn text_line_count(content: &TextContent) -> usize {
    match content {
        TextContent::Plain(text) => text.lines().count().max(1),
        TextContent::Styled(segments) => segments
            .iter()
            .flat_map(|segment| segment.text.lines())
            .count()
            .max(1),
    }
}
