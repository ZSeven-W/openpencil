//! Desktop dashboard cleanup for no-sidebar sequential generations.
//!
//! Some prompts ask for a dashboard with a right insights panel, not a
//! sidebar. Those correctly bypass the TS-compatible dashboard-column
//! scaffold, but weaker models can still emit a 1200px root whose main
//! content row is only a few hundred pixels wide. This pass preserves
//! the authored two-column intent while making the row use the desktop
//! width.

use crate::plan::OrchestratorPlan;
use crate::types::DocSink;
use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{ContainerProps, LayoutMode};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, LayoutPropValue, NodeId, PenNodeExt};

const DESKTOP_DASHBOARD_MIN_WIDTH: f64 = 900.0;
const SPARSE_ROW_MAX_RATIO: f64 = 0.72;
const RIGHT_PANEL_WIDTH: f64 = 320.0;
const CONTENT_ROW_GAP: f64 = 24.0;

#[derive(Debug, Default)]
struct DesktopDashboardRepairs {
    fill_widths: Vec<NodeId>,
    fixed_widths: Vec<(NodeId, f64)>,
    row_gaps: Vec<(NodeId, f64)>,
}

pub(crate) fn repair_sparse_desktop_dashboard_rows(
    sink: &mut dyn DocSink,
    plan: &OrchestratorPlan,
    root_id: &str,
) {
    let repairs = {
        let Some(root) = sink
            .state()
            .active_children()
            .iter()
            .find(|node| node.id_str() == root_id)
        else {
            return;
        };
        let Some(root_width) = root.width_px() else {
            return;
        };
        if root_width < DESKTOP_DASHBOARD_MIN_WIDTH || !looks_dashboard_like(root, plan) {
            return;
        }
        let mut repairs = DesktopDashboardRepairs::default();
        collect_sparse_row_repairs(root, root_width, &mut repairs);
        repairs
    };

    for node_id in repairs.fill_widths {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "width".to_string(),
            value: LayoutPropValue::Keyword("fill_container".to_string()),
        });
    }
    for (node_id, width) in repairs.fixed_widths {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "width".to_string(),
            value: LayoutPropValue::Number(width),
        });
    }
    for (node_id, gap) in repairs.row_gaps {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id,
            property: "gap".to_string(),
            value: LayoutPropValue::Number(gap),
        });
    }
}

fn collect_sparse_row_repairs(
    node: &PenNode,
    root_width: f64,
    repairs: &mut DesktopDashboardRepairs,
) {
    if is_sparse_right_panel_row(node, root_width) {
        let children: &[PenNode] = node.children().map_or(&[], Vec::as_slice);
        if let Some(first) = children.first() {
            if !width_keyword(first, SizingKeyword::FillContainer) {
                repairs
                    .fill_widths
                    .push(NodeId::new(first.id_str().to_string()));
            }
        }
        if let Some(last) = children.last() {
            if width_px(last).is_none_or(|w| (w - RIGHT_PANEL_WIDTH).abs() > 0.5) {
                repairs
                    .fixed_widths
                    .push((NodeId::new(last.id_str().to_string()), RIGHT_PANEL_WIDTH));
            }
        }
        if gap_value(node) < 16.0 {
            repairs
                .row_gaps
                .push((NodeId::new(node.id_str().to_string()), CONTENT_ROW_GAP));
        }
    }

    if let Some(children) = node.children() {
        for child in children {
            collect_sparse_row_repairs(child, root_width, repairs);
        }
    }
}

fn is_sparse_right_panel_row(node: &PenNode, root_width: f64) -> bool {
    if !is_horizontal_container(node) {
        return false;
    }
    let Some(children) = node.children() else {
        return false;
    };
    if children.len() != 2 || !children.iter().all(PenNode::is_container) {
        return false;
    }
    let Some(last) = children.last() else {
        return false;
    };
    // Require the trailing column to actually signal a right-rail / insights
    // panel. A bare "panel" match was too broad — ordinary two-column rows
    // whose second child merely contains "panel" in its name would be
    // rewritten to the full desktop width.
    if !contains_any(&identity_haystack(last), &["right", "insight"]) {
        return false;
    }
    let numeric_sum: f64 = children.iter().filter_map(width_px).sum();
    numeric_sum > 0.0 && numeric_sum < root_width * SPARSE_ROW_MAX_RATIO
}

fn looks_dashboard_like(root: &PenNode, plan: &OrchestratorPlan) -> bool {
    let mut text = format!(
        "{} {}",
        plan.root_frame.name,
        root.base().name.as_deref().unwrap_or("")
    )
    .to_lowercase();
    collect_identity_text(root, &mut text);
    contains_any(
        &text,
        &[
            "dashboard",
            "analytics",
            "kpi",
            "metric",
            "chart",
            "tasks",
            "table",
            "insight",
        ],
    )
}

fn collect_identity_text(node: &PenNode, out: &mut String) {
    out.push(' ');
    out.push_str(&identity_haystack(node));
    if let Some(children) = node.children() {
        for child in children {
            collect_identity_text(child, out);
        }
    }
}

fn is_horizontal_container(node: &PenNode) -> bool {
    container_props(node)
        .and_then(|props| props.layout.as_ref())
        .is_some_and(|layout| *layout == LayoutMode::Horizontal)
}

fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&n.container),
        PenNode::Group(n) => Some(&n.container),
        PenNode::Rectangle(n) => Some(&n.container),
        _ => None,
    }
}

fn width_px(node: &PenNode) -> Option<f64> {
    match width_behavior(node) {
        Some(SizingBehavior::Number(v)) => Some(*v),
        _ => None,
    }
}

fn width_keyword(node: &PenNode, keyword: SizingKeyword) -> bool {
    matches!(width_behavior(node), Some(SizingBehavior::Keyword(value)) if *value == keyword)
}

fn width_behavior(node: &PenNode) -> Option<&SizingBehavior> {
    match node {
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
    }
}

fn gap_value(node: &PenNode) -> f64 {
    container_props(node)
        .and_then(|props| props.gap.as_ref())
        .and_then(number_value)
        .unwrap_or(0.0)
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
