use crate::cleanup::run_cleanup_passes;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::json;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Explore".into(),
            width: 375.0,
            height: 812.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

fn node_json(sink: &VecDocSink, id: &str) -> serde_json::Value {
    let node = sink
        .state
        .active_children()
        .iter()
        .find_map(|root| find_node(root, id))
        .expect("node exists");
    serde_json::to_value(node).expect("node serializes")
}

#[test]
fn cleanup_preserves_flush_right_horizontal_scroller_geometry() {
    let tree: PenNode = serde_json::from_str(r##"{
        "type": "frame",
        "id": "root",
        "name": "Explore",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "children": [{
            "type": "frame",
            "id": "section",
            "name": "Popular Destinations Rail",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "header",
                    "name": "Section Header",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "horizontal",
                    "padding": [0, 24],
                    "children": [{
                        "type": "text",
                        "id": "title",
                        "content": "Popular Destinations",
                        "width": "fit_content",
                        "height": "fit_content"
                    }]
                },
                {
                    "type": "frame",
                    "id": "viewport",
                    "name": "Destinations Viewport",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "horizontal",
                    "clipContent": true,
                    "padding": [0, 0, 0, 24],
                    "children": [{
                        "type": "frame",
                        "id": "rail",
                        "name": "Destinations Rail",
                        "width": "fit_content",
                        "height": "fit_content",
                        "layout": "horizontal",
                        "gap": 12,
                        "children": [
                            {
                                "type": "frame",
                                "id": "kyoto",
                                "name": "Kyoto Card",
                                "width": 294,
                                "height": 300,
                                "layout": "vertical",
                                "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E5E7EB"}]},
                                "children": []
                            },
                            {
                                "type": "frame",
                                "id": "santorini",
                                "name": "Santorini Card",
                                "width": 208,
                                "height": 300,
                                "layout": "vertical",
                                "children": []
                            }
                        ]
                    }]
                }
            ]
        }]
    }"##)
    .expect("scroller fixture");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    assert!(
        node_json(&sink, "section").get("padding").is_none(),
        "the section must not gain a generic mobile inset"
    );
    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 0.0, 0.0, 24.0]),
        "cleanup must preserve the authored flush trailing edge"
    );
    assert_eq!(node_json(&sink, "rail")["gap"], json!(12.0));
    assert_eq!(node_json(&sink, "kyoto")["width"], json!(294.0));
    assert_eq!(node_json(&sink, "santorini")["width"], json!(208.0));
}
