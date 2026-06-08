use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use op_editor_core::LayoutPropValue;
use serde_json::json;

fn desktop_plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Dashboard".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

#[test]
fn cleanup_expands_sparse_desktop_dashboard_right_panel_rows() {
    let mut sink = VecDocSink::new();
    let tree = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Dashboard",
        "width": 1200,
        "height": 800,
        "layout": "vertical",
        "children": [{
            "type": "frame",
            "id": "main-area",
            "name": "Main Dashboard Area",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "children": [{
                "type": "frame",
                "id": "content-row",
                "name": "Content Row",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "horizontal",
                "gap": 0,
                "children": [
                    {
                        "type": "frame",
                        "id": "left-col",
                        "name": "Left Column",
                        "width": 408,
                        "height": "fit_content",
                        "layout": "vertical",
                        "children": []
                    },
                    {
                        "type": "frame",
                        "id": "right-col",
                        "name": "Right Insights Panel",
                        "width": 175,
                        "height": "fit_content",
                        "layout": "vertical",
                        "children": []
                    }
                ]
            }]
        }]
    }))
    .expect("dashboard fixture");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    let left_id = node_id_by_name(&sink, "Left Column");
    let right_id = node_id_by_name(&sink, "Right Insights Panel");
    let row_id = node_id_by_name(&sink, "Content Row");
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &desktop_plan(), &[&root_id]);

    assert!(
        sink.applied.iter().any(|cmd| matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Keyword(keyword),
            } if node_id.as_str() == left_id
                && property == "width"
                && keyword == "fill_container"
        )),
        "desktop dashboard left column should absorb remaining main-row width"
    );
    assert!(
        sink.applied.iter().any(|cmd| matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Number(width),
            } if node_id.as_str() == right_id
                && property == "width"
                && (*width - 320.0).abs() < f64::EPSILON
        )),
        "right insights column should land on a usable desktop panel width"
    );
    assert!(
        sink.applied.iter().any(|cmd| matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                node_id,
                property,
                value: LayoutPropValue::Number(gap),
            } if node_id.as_str() == row_id
                && property == "gap"
                && (*gap - 24.0).abs() < f64::EPSILON
        )),
        "desktop dashboard content rows should regain a professional column gap"
    );
}

#[test]
fn cleanup_preserves_adequate_desktop_dashboard_rows() {
    let mut sink = VecDocSink::new();
    let tree = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Dashboard",
        "width": 1200,
        "height": 800,
        "layout": "vertical",
        "children": [{
            "type": "frame",
            "id": "content-row",
            "name": "Content Row",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "horizontal",
            "gap": 24,
            "children": [
                {
                    "type": "frame",
                    "id": "left-col",
                    "name": "Left Column",
                    "width": 760,
                    "height": "fit_content",
                    "layout": "vertical",
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "right-col",
                    "name": "Right Insights Panel",
                    "width": 320,
                    "height": "fit_content",
                    "layout": "vertical",
                    "children": []
                }
            ]
        }]
    }))
    .expect("dashboard fixture");
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state.active_children()[0].id_str().to_string();
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &desktop_plan(), &[&root_id]);

    assert!(
        !sink.applied.iter().any(|cmd| matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp {
                property,
                ..
            } if property == "width" || property == "gap"
        )),
        "adequate desktop dashboard rows should not be rewritten"
    );
}

fn node_id_by_name(sink: &VecDocSink, name: &str) -> String {
    fn find(node: &jian_ops_schema::node::PenNode, name: &str) -> Option<String> {
        if node.base().name.as_deref() == Some(name) {
            return Some(node.id_str().to_string());
        }
        node.children()?.iter().find_map(|child| find(child, name))
    }

    sink.state
        .active_children()
        .iter()
        .find_map(|node| find(node, name))
        .unwrap_or_else(|| panic!("missing node named {name}"))
}
