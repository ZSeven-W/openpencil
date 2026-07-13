use crate::cleanup::run_cleanup_passes;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::json;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
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

#[test]
fn bottom_nav_wrapper_with_divider_keeps_tabbar_full_width() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Travel App",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "fill": [{"type": "solid", "color": "#FFF8F0"}],
        "children": [
            {
                "type": "frame",
                "id": "content",
                "name": "Content",
                "width": "fill_container",
                "height": 700,
                "children": []
            },
            {
                "type": "frame",
                "id": "bottom-nav",
                "name": "Bottom Navigation Bar",
                "role": "bottom-tab-bar",
                "width": 375,
                "height": 72,
                "layout": "vertical",
                "children": [
                    {
                        "type": "rectangle",
                        "id": "divider",
                        "name": "Nav Divider",
                        "width": "fill_container",
                        "height": 1,
                        "children": []
                    },
                    {
                        "type": "frame",
                        "id": "tabbar",
                        "name": "Tab Bar",
                        "role": "bottom-tab-bar",
                        "width": "fill_container",
                        "height": "fit_content",
                        "layout": "horizontal",
                        "children": [
                            tab("explore", "Explore", "compass"),
                            tab("wishlists", "Wishlists", "heart"),
                            tab("trips", "Trips", "luggage"),
                            tab("messages", "Messages", "message-circle"),
                            tab("profile", "Profile", "user")
                        ]
                    }
                ]
            }
        ]
    }))
    .expect("nav wrapper json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == "root")
        .expect("root survives");
    let outer = find_node(root, "bottom-nav").expect("outer nav survives");
    let inner = find_node(root, "tabbar").expect("inner tabbar survives");
    let outer_json = serde_json::to_value(outer).expect("outer serializes");
    let inner_json = serde_json::to_value(inner).expect("inner serializes");
    assert_eq!(
        outer_json["layout"],
        json!("vertical"),
        "divider and tabbar must not be laid out side by side: {outer_json}"
    );
    assert_eq!(outer_json["width"], json!(375.0));
    assert!(
        inner_json["width"] == json!("fill_container") || inner_json["width"] == json!(375.0),
        "inner tabbar must retain a full-width sizing mode: {inner_json}"
    );
    assert_eq!(inner_json["layout"], json!("horizontal"));
}

fn tab(id: &str, label: &str, icon: &str) -> serde_json::Value {
    json!({
        "type": "frame",
        "id": format!("{id}-tab"),
        "name": format!("{label} Tab"),
        "width": "fill_container",
        "height": "fill_container",
        "layout": "vertical",
        "children": [
            {"type": "icon_font", "id": format!("{id}-icon"), "iconFontName": icon, "width": 20, "height": 20},
            {"type": "text", "id": format!("{id}-label"), "content": label, "width": "fit_content", "height": "fit_content"}
        ]
    })
}

// ── anchor_bottom_nav_last: late "catch-up" section after the nav ─────────

#[test]
fn late_section_after_bottom_nav_moves_nav_back_to_last() {
    // test0710-1-m3.op shape: mobile root (fit_content height!) whose model
    // appended the greeting+search header AFTER the bottom tab bar.
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [{
            "type": "frame", "id": "root", "name": "Explore",
            "width": 375, "height": "fit_content", "layout": "vertical",
            "children": [
                { "type": "frame", "id": "sb", "name": "Status Bar", "role": "status-bar",
                  "width": "fill_container", "height": 62 },
                { "type": "frame", "id": "pop", "name": "Popular Destinations",
                  "width": "fill_container", "height": "fit_content",
                  "children": [ { "type": "text", "id": "t1", "name": "T", "content": "x",
                                   "width": 100, "height": 20 } ] },
                { "type": "frame", "id": "nav", "name": "Bottom Navigation Bar",
                  "role": "bottom-tab-bar", "width": "fill_container", "height": 72 },
                { "type": "frame", "id": "hdr", "name": "Header & Search",
                  "width": "fill_container", "height": "fit_content",
                  "children": [ { "type": "text", "id": "t2", "name": "T2", "content": "y",
                                   "width": 100, "height": 20 } ] }
            ]
        }]
    }))
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::cleanup::anchor_bottom_nav_last_for_all_roots(&mut sink);

    let root = &state.active_children()[0];
    let order: Vec<&str> = root
        .children()
        .expect("children")
        .iter()
        .map(|c| c.id_str())
        .collect();
    assert_eq!(
        order,
        vec!["sb", "pop", "hdr", "nav"],
        "nav must return to the last slot; content order otherwise preserved"
    );
}

#[test]
fn nav_already_last_and_desktop_roots_are_untouched() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_value(serde_json::json!({
        "version": "1.0",
        "children": [
            { "type": "frame", "id": "m", "name": "Mobile", "width": 390, "height": 844,
              "layout": "vertical",
              "children": [
                { "type": "frame", "id": "c", "name": "Content",
                  "width": "fill_container", "height": "fit_content" },
                { "type": "frame", "id": "nav", "name": "Bottom Tab Bar",
                  "role": "bottom-tab-bar", "width": "fill_container", "height": 72 }
              ] },
            { "type": "frame", "id": "d", "name": "Dashboard", "width": 1440, "height": 900,
              "layout": "vertical",
              "children": [
                { "type": "frame", "id": "navd", "name": "Bottom Tab Bar",
                  "role": "bottom-tab-bar", "width": "fill_container", "height": 72 },
                { "type": "frame", "id": "cd", "name": "Content",
                  "width": "fill_container", "height": "fit_content" }
              ] }
        ]
    }))
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let before = serde_json::to_string(state.active_children()).expect("snapshot");
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::cleanup::anchor_bottom_nav_last_for_all_roots(&mut sink);
    assert_eq!(
        serde_json::to_string(state.active_children()).expect("snapshot"),
        before,
        "nav-last mobile root and >480px desktop root must both be no-ops"
    );
}

/// GLM-5.2 measured shape (test0711-1.op): root is 390×`fit_content` and the
/// whole screen — nav included — lives inside one "Content Wrapper". The old
/// `is_mobile_root` height gate (>= 500px resolved) skipped every nav repair
/// for exactly this shape, so the hand-built nav shipped crooked.
#[test]
fn fit_content_root_with_wrapper_nested_nav_gets_normalized() {
    let doc: jian_ops_schema::PenDocument = serde_json::from_str(
        r##"{ "version": "1.0", "children": [{ "type": "frame", "id": "root", "name": "Explore Screen", "width": 390, "height": "fit_content", "layout": "vertical", "children": [ { "type": "frame", "id": "sb", "name": "Status Bar", "role": "status-bar", "width": "fill_container", "height": 62 }, { "type": "frame", "id": "wrap", "name": "Content Wrapper", "width": "fill_container", "height": "fit_content", "layout": "vertical", "children": [ { "type": "frame", "id": "hdr", "name": "Header", "width": "fill_container", "height": "fit_content", "children": [ { "type": "text", "id": "t1", "name": "T", "content": "Hello", "width": 100, "height": 20 } ] }, { "type": "frame", "id": "nav", "name": "Bottom Navigation", "role": "bottom-tab-bar", "width": "fill_container", "height": 64, "layout": "horizontal", "gap": 12, "children": [ { "type": "frame", "id": "tab1", "name": "Explore Tab", "width": 80, "height": 40, "layout": "vertical", "children": [ { "type": "text", "id": "l1", "name": "L", "content": "Explore", "width": 60, "height": 14 } ] }, { "type": "frame", "id": "tab2", "name": "Trips Tab", "width": 60, "height": 48, "layout": "vertical", "children": [ { "type": "text", "id": "l2", "name": "L", "content": "Trips", "width": 40, "height": 14 } ] }, { "type": "frame", "id": "tab3", "name": "Profile Tab", "width": 70, "height": 44, "layout": "vertical", "children": [ { "type": "text", "id": "l3", "name": "L", "content": "Profile", "width": 50, "height": 14 } ] } ] } ] } ] }] }"##,
    )
    .expect("doc");
    let mut state = op_editor_core::EditorState::from_document(doc);
    let mut sink = crate::loop_finalize::StateDocSink { state: &mut state };
    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);

    fn find_by_id<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
        if node.id_str() == id {
            return Some(node);
        }
        node.children()?.iter().find_map(|c| find_by_id(c, id))
    }
    let root = &state.active_children()[0];
    let nav = find_by_id(root, "nav").expect("nav");
    assert_eq!(
        nav.height_px(),
        Some(72.0),
        "nav surface normalized to 72px"
    );
    let tab = find_by_id(root, "tab1").expect("tab1");
    assert!(
        tab.width_px().is_none(),
        "tabs switch to fill_container so they distribute evenly, got {:?}",
        tab.width_px()
    );
}
