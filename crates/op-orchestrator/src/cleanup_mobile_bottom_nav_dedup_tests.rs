use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Mobile".into(),
            width: 390.0,
            height: 844.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn insert_tree(sink: &mut VecDocSink, json: &str) {
    let tree: PenNode = serde_json::from_str(json).expect("test tree json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
}

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

fn find_active_node<'a>(sink: &'a VecDocSink, id: &str) -> Option<&'a PenNode> {
    sink.state
        .active_children()
        .iter()
        .find_map(|node| find_node(node, id))
}

fn active_root(sink: &VecDocSink) -> &PenNode {
    find_active_node(sink, "root").expect("root survives")
}

fn direct_child_ids(root: &PenNode) -> Vec<&str> {
    root.children()
        .into_iter()
        .flatten()
        .map(PenNode::id_str)
        .collect()
}

#[test]
fn bottom_nav_detected_by_cjk_name() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Chinese Mobile Screen",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                {
                    "type": "frame",
                    "id": "content",
                    "name": "Content",
                    "role": "section",
                    "width": "fill_container",
                    "height": 300,
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "cjk-nav-section",
                    "name": "底部导航栏",
                    "role": "section",
                    "x": 24,
                    "width": 342,
                    "height": 88,
                    "layout": "horizontal",
                    "children": []
                }
            ]
        }"##,
    );

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let nav = find_active_node(&sink, "cjk-nav-section").expect("CJK bottom nav survives");
    // Authored absolute inset is CLEARED so the nav rejoins flex flow —
    // a written x (even 0) is absolute placement and gets buried at (0,0).
    assert_eq!(nav.base().x, None);
    assert_eq!(nav.width_px(), Some(390.0));
    assert_eq!(nav.height_px(), Some(72.0));
}

#[test]
fn duplicate_cjk_and_english_bottom_nav_deduped() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Chinese Mobile Screen",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                {
                    "type": "frame",
                    "id": "content",
                    "name": "Content",
                    "role": "section",
                    "width": "fill_container",
                    "height": 620,
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "cjk-nav-section",
                    "name": "底部导航栏",
                    "role": "section",
                    "width": "fill_container",
                    "height": 72,
                    "layout": "horizontal",
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "english-bottom-nav",
                    "name": "Bottom Navigation",
                    "role": "section",
                    "width": "fill_container",
                    "height": 72,
                    "layout": "horizontal",
                    "children": []
                }
            ]
        }"##,
    );

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let ids = direct_child_ids(active_root(&sink));
    assert!(
        !ids.contains(&"cjk-nav-section"),
        "earlier duplicate is removed"
    );
    assert!(
        ids.contains(&"english-bottom-nav"),
        "bottom-most duplicate is kept"
    );
    assert_eq!(
        ids.iter()
            .filter(|id| id.contains("bottom-nav"))
            .copied()
            .collect::<Vec<_>>(),
        vec!["english-bottom-nav"]
    );
}

#[test]
fn single_bottom_nav_not_removed() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Single Nav Mobile Screen",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                {
                    "type": "frame",
                    "id": "content",
                    "name": "Content",
                    "role": "section",
                    "width": "fill_container",
                    "height": 620,
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "cjk-nav-section",
                    "name": "底部导航栏",
                    "role": "section",
                    "width": "fill_container",
                    "height": 72,
                    "layout": "horizontal",
                    "children": []
                }
            ]
        }"##,
    );

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let ids = direct_child_ids(active_root(&sink));
    assert!(ids.contains(&"cjk-nav-section"));
}

#[test]
fn top_navbar_not_treated_as_bottom_nav() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Top And Bottom Nav Mobile Screen",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                {
                    "type": "frame",
                    "id": "top-navbar",
                    "name": "Top Navigation Bar",
                    "role": "top-app-bar",
                    "width": "fill_container",
                    "height": 64,
                    "layout": "horizontal",
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "content",
                    "name": "Content",
                    "role": "section",
                    "width": "fill_container",
                    "height": 556,
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "bottom-nav",
                    "name": "Bottom Navigation",
                    "role": "section",
                    "width": "fill_container",
                    "height": 72,
                    "layout": "horizontal",
                    "children": []
                }
            ]
        }"##,
    );

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let ids = direct_child_ids(active_root(&sink));
    assert!(ids.contains(&"top-navbar"), "top navbar is untouched");
    assert!(ids.contains(&"bottom-nav"), "sole bottom nav is untouched");
}
