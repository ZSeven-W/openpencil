use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Mobile Food App".into(),
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

fn insert_root(value: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    let root: PenNode = serde_json::from_value(value).expect("root json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
    sink
}

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

fn active_node_value(sink: &VecDocSink, id: &str) -> Value {
    let node = sink
        .state
        .active_children()
        .iter()
        .find_map(|node| find_node(node, id))
        .expect("node exists");
    serde_json::to_value(node).expect("serialize node")
}

#[test]
fn fill_container_content_section_on_fixed_vertical_root_collapses() {
    let mut sink = insert_root(json!({
        "type": "frame",
        "id": "root",
        "name": "Food App",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "promo",
                "name": "Featured Promo Banner",
                "width": "fill_container",
                "height": "fill_container",
                "layout": "vertical",
                "children": [
                    {"type": "text", "id": "promo-title", "content": "Half-price ramen"}
                ]
            },
            {
                "type": "frame",
                "id": "categories",
                "name": "Food Categories",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "horizontal",
                "children": [
                    {"type": "text", "id": "cat-title", "content": "Sushi"}
                ]
            },
            {
                "type": "frame",
                "id": "restaurants",
                "name": "Popular Restaurants",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "children": []
            }
        ]
    }));

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    assert_eq!(
        active_node_value(&sink, "promo")["height"],
        json!("fit_content")
    );
    assert_eq!(
        active_node_value(&sink, "categories")["height"],
        json!("fit_content")
    );
    assert_eq!(
        active_node_value(&sink, "restaurants")["height"],
        json!("fit_content")
    );
}

#[test]
fn empty_spacer_fill_container_not_collapsed() {
    let mut sink = insert_root(json!({
        "type": "frame",
        "id": "root",
        "name": "Food App",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "spacer",
                "name": "Flexible Spacer",
                "width": "fill_container",
                "height": "fill_container",
                "children": []
            }
        ]
    }));

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    assert_eq!(
        active_node_value(&sink, "spacer")["height"],
        json!("fill_container")
    );
}

#[test]
fn fill_container_on_horizontal_root_child_untouched() {
    let mut sink = insert_root(json!({
        "type": "frame",
        "id": "root",
        "name": "Web App Shell",
        "width": 1200,
        "height": 800,
        "layout": "horizontal",
        "children": [
            {
                "type": "frame",
                "id": "sidebar",
                "name": "Sidebar",
                "width": 260,
                "height": "fill_container",
                "layout": "vertical",
                "children": [
                    {"type": "text", "id": "nav", "content": "Home"}
                ]
            }
        ]
    }));

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    assert_eq!(
        active_node_value(&sink, "sidebar")["height"],
        json!("fill_container")
    );
}

#[test]
fn fit_content_root_untouched() {
    let mut sink = insert_root(json!({
        "type": "frame",
        "id": "root",
        "name": "Scrolling Food App",
        "width": 375,
        "height": "fit_content",
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "promo",
                "name": "Featured Promo Banner",
                "width": "fill_container",
                "height": "fill_container",
                "layout": "vertical",
                "children": [
                    {"type": "text", "id": "promo-title", "content": "Daily deal"}
                ]
            }
        ]
    }));

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    assert_eq!(
        active_node_value(&sink, "promo")["height"],
        json!("fill_container")
    );
}
