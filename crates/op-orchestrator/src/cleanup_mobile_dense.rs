//! Mobile dense-row cleanup for generated screens.
//!
//! This final cleanup pass catches a common weak-model failure mode:
//! bottom tabs or seven-day progress rows whose per-item padding makes
//! the horizontal row wider than a 390px mobile root.

use crate::types::DocSink;
use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{ContainerProps, LayoutMode, Padding};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, LayoutPropValue, NodeId, PenNodeExt};

#[derive(Debug, Clone, Copy)]
enum DenseRowKind {
    Nav,
    WeeklyProgress,
}

#[derive(Debug, Default)]
struct DenseRowRepairs {
    row_gaps: Vec<(NodeId, f64)>,
    row_paddings: Vec<(NodeId, Vec<f64>)>,
    child_paddings: Vec<(NodeId, Vec<f64>)>,
    fill_widths: Vec<NodeId>,
}

pub(crate) fn repair_dense_mobile_rows(sink: &mut dyn DocSink, root_id: &str) {
    let repairs = {
        let Some(root) = sink
            .state()
            .active_children()
            .iter()
            .find(|node| node.id_str() == root_id)
        else {
            return;
        };
        if !is_mobile_root(root) {
            return;
        }
        let mut repairs = DenseRowRepairs::default();
        collect_dense_row_repairs(root, &mut repairs);
        repairs
    };

    for (node_id, gap) in repairs.row_gaps {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "gap".to_string(),
            value: LayoutPropValue::Number(gap),
        });
    }
    for (node_id, padding) in repairs.row_paddings {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "padding".to_string(),
            value: LayoutPropValue::NumberArray(padding),
        });
    }
    for (node_id, padding) in repairs.child_paddings {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "padding".to_string(),
            value: LayoutPropValue::NumberArray(padding),
        });
    }
    for node_id in repairs.fill_widths {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "width".to_string(),
            value: LayoutPropValue::Keyword("fill_container".to_string()),
        });
    }
}

fn collect_dense_row_repairs(node: &PenNode, repairs: &mut DenseRowRepairs) {
    if let Some(kind) = dense_row_kind(node) {
        let row_id = NodeId::new(node.id_str().to_string());
        match kind {
            DenseRowKind::Nav => {
                if gap_value(node) > 4.0 {
                    repairs.row_gaps.push((row_id.clone(), 4.0));
                }
                if padding_exceeds(node, [8.0, 16.0, 8.0, 16.0]) {
                    repairs
                        .row_paddings
                        .push((row_id, vec![8.0, 16.0, 8.0, 16.0]));
                }
                collect_child_compaction(node, [8.0, 8.0, 8.0, 8.0], true, repairs);
            }
            DenseRowKind::WeeklyProgress => {
                if gap_value(node) > 4.0 {
                    repairs.row_gaps.push((row_id, 4.0));
                }
                collect_child_compaction(node, [4.0, 0.0, 4.0, 0.0], true, repairs);
            }
        }
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_dense_row_repairs(child, repairs);
        }
    }
}

fn collect_child_compaction(
    row: &PenNode,
    target_padding: [f64; 4],
    fill_fit_widths: bool,
    repairs: &mut DenseRowRepairs,
) {
    let Some(children) = row.children() else {
        return;
    };
    for child in children {
        if !child.is_container() {
            continue;
        }
        let child_id = NodeId::new(child.id_str().to_string());
        if padding_exceeds(child, target_padding) {
            repairs
                .child_paddings
                .push((child_id.clone(), target_padding.to_vec()));
        }
        if fill_fit_widths && width_keyword(child, SizingKeyword::FitContent) {
            repairs.fill_widths.push(child_id);
        }
    }
}

fn dense_row_kind(node: &PenNode) -> Option<DenseRowKind> {
    if !is_horizontal_container(node) {
        return None;
    }
    let children = node.children()?;
    if children.len() < 4 {
        return None;
    }

    let hay = identity_haystack(node);
    if contains_any(
        &hay,
        &[
            "bottom nav",
            "bottom-nav",
            "bottom navigation",
            "bottom-navigation",
            "bottom tab",
            "bottom-tab",
            "tab bar",
            "tab-bar",
            "tabbar",
        ],
    ) {
        return Some(DenseRowKind::Nav);
    }

    if children.len() >= 6
        && contains_any(
            &hay,
            &[
                "seven day",
                "seven-day",
                "week row",
                "weekly",
                "progress indicators",
                "day progress",
            ],
        )
    {
        return Some(DenseRowKind::WeeklyProgress);
    }

    None
}

fn is_mobile_root(root: &PenNode) -> bool {
    let width = root.width_px().unwrap_or(f64::INFINITY);
    let height = root.height_px().unwrap_or(0.0);
    width <= 480.0 && height >= 500.0
}

fn is_horizontal_container(node: &PenNode) -> bool {
    container_props(node)
        .and_then(|props| props.layout.as_ref())
        .is_some_and(|layout| *layout == LayoutMode::Horizontal)
}

fn gap_value(node: &PenNode) -> f64 {
    container_props(node)
        .and_then(|props| props.gap.as_ref())
        .and_then(number_value)
        .unwrap_or(0.0)
}

fn padding_exceeds(node: &PenNode, target: [f64; 4]) -> bool {
    let current = container_props(node)
        .and_then(|props| props.padding.as_ref())
        .map(padding_sides)
        .unwrap_or([0.0, 0.0, 0.0, 0.0]);
    current
        .iter()
        .zip(target.iter())
        .any(|(current, target)| *current > *target + 0.5)
}

fn padding_sides(padding: &Padding) -> [f64; 4] {
    match padding {
        Padding::Uniform(v) => [*v, *v, *v, *v],
        Padding::XY([y, x]) => [*y, *x, *y, *x],
        Padding::LtrB(values) => *values,
        Padding::Expression(_) => [0.0, 0.0, 0.0, 0.0],
    }
}

fn width_keyword(node: &PenNode, keyword: SizingKeyword) -> bool {
    let width = match node {
        PenNode::Frame(n) => n.container.width.as_ref(),
        PenNode::Group(n) => n.container.width.as_ref(),
        PenNode::Rectangle(n) => n.container.width.as_ref(),
        PenNode::Ellipse(n) => n.width.as_ref(),
        PenNode::Polygon(n) => n.width.as_ref(),
        PenNode::Path(n) => n.width.as_ref(),
        PenNode::Text(n) => n.width.as_ref(),
        PenNode::TextInput(n) => n.width.as_ref(),
        PenNode::TextArea(n) => n.width.as_ref(),
        PenNode::Select(n) => n.width.as_ref(),
        PenNode::Switch(n) => n.width.as_ref(),
        PenNode::Checkbox(n) => n.width.as_ref(),
        PenNode::Slider(n) => n.width.as_ref(),
        PenNode::RadioGroup(n) => n.width.as_ref(),
        PenNode::NumberInput(n) => n.width.as_ref(),
        PenNode::Progress(n) => n.width.as_ref(),
        PenNode::Tabs(n) => n.width.as_ref(),
        PenNode::Image(n) => n.width.as_ref(),
        PenNode::IconFont(n) => n.width.as_ref(),
        PenNode::Line(_) | PenNode::Ref(_) => None,
    };
    matches!(width, Some(SizingBehavior::Keyword(value)) if *value == keyword)
}

fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&n.container),
        PenNode::Group(n) => Some(&n.container),
        PenNode::Rectangle(n) => Some(&n.container),
        _ => None,
    }
}

fn number_value(value: &NumberOrExpression) -> Option<f64> {
    match value {
        NumberOrExpression::Number(v) => Some(*v),
        NumberOrExpression::Expression(_) => None,
    }
}

fn identity_haystack(node: &PenNode) -> String {
    [
        node.id_str(),
        node.base().name.as_deref().unwrap_or(""),
        node.base().role.as_deref().unwrap_or(""),
    ]
    .join(" ")
    .to_lowercase()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}
