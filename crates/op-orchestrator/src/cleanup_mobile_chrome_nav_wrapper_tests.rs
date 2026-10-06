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
    assert!(
        root.children().expect("root children").iter().any(|child| {
            child.id_str() == "bottom-nav" && find_node(child, "tabbar").is_some()
        }),
        "a divider-only wrapper must retain its nested tabbar"
    );
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

#[test]
fn mixed_business_wrapper_promotes_real_tabbar_to_mobile_root() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Weather App",
        "width": 375,
        "height": 1285,
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "content",
                "name": "Forecast Content",
                "width": "fill_container",
                "height": 800,
                "children": []
            },
            {
                "type": "frame",
                "id": "mixed-shell",
                "name": "Bottom Navigation Bar",
                "role": "bottom-tab-bar",
                "width": "fill_container",
                "height": 458,
                "layout": "vertical",
                "cornerRadius": 24,
                "fill": [{"type": "solid", "color": "#111111"}],
                "stroke": {
                    "thickness": 1,
                    "fill": [{"type": "solid", "color": "#334155"}]
                },
                "effects": [{
                    "type": "shadow",
                    "offsetX": 0,
                    "offsetY": 8,
                    "blur": 20,
                    "spread": 0,
                    "color": "#00000033"
                }],
                "children": [
                    {
                        "type": "frame",
                        "id": "alert",
                        "name": "Weather Alert Banner",
                        "width": "fill_container",
                        "height": 136,
                        "children": [
                            {
                                "type": "text",
                                "id": "alert-copy",
                                "content": "Flash Flood & High Wind Warning",
                                "width": "fill_container",
                                "height": 24
                            }
                        ]
                    },
                    {
                        "type": "frame",
                        "id": "metrics",
                        "name": "Metrics Grid",
                        "width": "fill_container",
                        "height": 226,
                        "children": [
                            {
                                "type": "text",
                                "id": "metrics-copy",
                                "content": "Humidity 88%",
                                "width": "fit_content",
                                "height": 24
                            }
                        ]
                    },
                    {
                        "type": "frame",
                        "id": "tabbar",
                        "name": "Bottom Tab Bar",
                        "role": "bottom-tab-bar",
                        "width": "fill_container",
                        "height": 72,
                        "layout": "horizontal",
                        "children": [
                            tab("now", "Now", "cloud-sun"),
                            tab("radar", "Radar", "radar"),
                            tab("locations", "Locations", "map-pin"),
                            tab("settings", "Settings", "settings")
                        ]
                    }
                ]
            }
        ]
    }))
    .expect("mixed nav wrapper json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == "root")
        .expect("root survives");
    let root_children = root.children().expect("root children");
    assert_eq!(
        root_children
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["content", "mixed-shell", "tabbar"],
        "the real tabbar must become the mobile root's last child"
    );
    let shell = &root_children[1];
    assert_eq!(
        shell.base().role.as_deref(),
        None,
        "the mixed content shell must stop claiming bottom-tab-bar semantics"
    );
    assert_eq!(
        shell.base().name.as_deref(),
        Some("App Content"),
        "the promoted shell must no longer be excluded from content cleanup by a nav name"
    );
    let shell_json = serde_json::to_value(shell).expect("shell serializes");
    assert!(
        shell_json
            .get("fill")
            .is_none_or(serde_json::Value::is_null)
            && shell_json
                .get("stroke")
                .is_none_or(serde_json::Value::is_null)
            && shell_json
                .get("effects")
                .is_none_or(serde_json::Value::is_null)
            && shell_json["cornerRadius"] == json!(0.0),
        "the demoted content shell must be transparent: {shell_json}"
    );
    assert_eq!(
        shell
            .children()
            .expect("shell children")
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["alert", "metrics"],
        "business sections remain grouped while only the true nav row is promoted"
    );
    let nav = &root_children[2];
    assert_eq!(nav.height_px(), Some(72.0));
    assert_eq!(
        serde_json::to_value(nav).expect("nav serializes")["width"],
        json!("fill_container"),
        "normal nav normalization runs after promotion"
    );
}

#[test]
fn ambiguous_mixed_wrapper_with_two_nav_rows_is_not_split() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Ambiguous App",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "children": [
            {
                "type": "frame",
                "id": "mixed-shell",
                "name": "Bottom Navigation Bar",
                "role": "bottom-tab-bar",
                "width": "fill_container",
                "height": 300,
                "layout": "vertical",
                "children": [
                    {
                        "type": "frame",
                        "id": "business",
                        "name": "Summary",
                        "width": "fill_container",
                        "height": 120,
                        "children": [
                            {
                                "type": "text",
                                "id": "summary-copy",
                                "content": "Summary",
                                "width": "fit_content",
                                "height": 24
                            }
                        ]
                    },
                    {
                        "type": "frame",
                        "id": "tabbar-a",
                        "name": "Bottom Tab Bar A",
                        "role": "bottom-tab-bar",
                        "width": "fill_container",
                        "height": 72,
                        "layout": "horizontal",
                        "children": [
                            tab("a-home", "Home", "home"),
                            tab("a-search", "Search", "search"),
                            tab("a-profile", "Profile", "user")
                        ]
                    },
                    {
                        "type": "frame",
                        "id": "tabbar-b",
                        "name": "Bottom Tab Bar B",
                        "role": "bottom-tab-bar",
                        "width": "fill_container",
                        "height": 72,
                        "layout": "horizontal",
                        "children": [
                            tab("b-home", "Home", "home"),
                            tab("b-search", "Search", "search"),
                            tab("b-profile", "Profile", "user")
                        ]
                    }
                ]
            }
        ]
    }))
    .expect("ambiguous nav wrapper json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);

    let root = sink
        .state
        .active_children()
        .iter()
        .find(|node| node.id_str() == "root")
        .expect("root survives");
    let root_children = root.children().expect("root children");
    assert_eq!(
        root_children
            .iter()
            .map(PenNodeExt::id_str)
            .collect::<Vec<_>>(),
        vec!["mixed-shell"],
        "two plausible nav rows are ambiguous and must not be reparented"
    );
    let shell = &root_children[0];
    assert_eq!(shell.base().role.as_deref(), Some("bottom-tab-bar"));
    assert!(find_node(shell, "tabbar-a").is_some());
    assert!(find_node(shell, "tabbar-b").is_some());
}

#[test]
fn non_trailing_structural_top_tabs_are_not_promoted_as_bottom_nav() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile App",
        "width": 375,
        "height": 812,
        "layout": "vertical",
        "children": [{
            "type": "frame",
            "id": "mixed-shell",
            "name": "Bottom Navigation Bar",
            "role": "bottom-tab-bar",
            "width": "fill_container",
            "height": "fit_content",
            "layout": "vertical",
            "children": [
                {
                    "type": "frame",
                    "id": "top-tabs",
                    "name": "Header Tabs",
                    "width": "fill_container",
                    "height": 56,
                    "layout": "horizontal",
                    "children": [
                        tab("home", "Home", "home"),
                        tab("search", "Search", "search"),
                        tab("profile", "Profile", "user")
                    ]
                },
                {
                    "type": "frame",
                    "id": "business-content",
                    "name": "Business Content",
                    "width": "fill_container",
                    "height": 300,
                    "children": [{
                        "type": "text",
                        "id": "copy",
                        "content": "Dashboard content",
                        "width": "fit_content",
                        "height": "fit_content"
                    }]
                }
            ]
        }]
    }))
    .expect("top tabs fixture");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    let root_children = root.children().expect("root children");
    assert_eq!(root_children.len(), 1);
    assert_eq!(root_children[0].id_str(), "mixed-shell");
    assert_eq!(root_children[0].base().role.as_deref(), None);
    assert_eq!(
        root_children[0].base().name.as_deref(),
        Some("App Content"),
        "the mislabeled outer shell must be demoted instead of collapsed into nav chrome"
    );
    assert!(
        find_node(&root_children[0], "top-tabs").is_some(),
        "a non-trailing structural header row must stay inside its authored shell"
    );
    let shell_json = serde_json::to_value(&root_children[0]).expect("shell serializes");
    assert_eq!(shell_json["width"], json!("fill_container"));
    assert_eq!(shell_json["height"], json!("fit_content"));
    assert_eq!(shell_json["layout"], json!("vertical"));
    assert!(shell_json["padding"].is_null());
    let top_tabs = find_node(&root_children[0], "top-tabs").expect("top tabs survive");
    let top_tabs_json = serde_json::to_value(top_tabs).expect("top tabs serialize");
    assert_eq!(top_tabs_json["width"], json!("fill_container"));
    assert_eq!(top_tabs_json["height"], json!(56.0));
    assert_eq!(top_tabs_json["layout"], json!("horizontal"));
    assert!(top_tabs_json["padding"].is_null());
}

#[test]
fn explicit_horizontal_text_only_bottom_nav_is_not_demoted() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"content","height":700},
            {
                "type":"frame","id":"nav","name":"Tab Bar","role":"bottom-tab-bar",
                "width":"fill_container","height":72,"layout":"horizontal",
                "children":[
                    {"type":"frame","id":"home-tab","children":[
                        {"type":"text","id":"home-label","content":"Home"}
                    ]},
                    {"type":"frame","id":"saved-tab","children":[
                        {"type":"text","id":"saved-label","content":"Saved"}
                    ]}
                ]
            }
        ]
    }))
    .expect("explicit text-only nav fixture");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    let nav = find_node(root, "nav").expect("nav survives");
    assert_eq!(nav.base().role.as_deref(), Some("bottom-tab-bar"));
    assert_eq!(nav.base().name.as_deref(), Some("Tab Bar"));
    let nav_json = serde_json::to_value(nav).expect("nav serializes");
    assert_eq!(nav_json["width"], json!("fill_container"));
    assert_eq!(nav_json["height"], json!(72.0));
    assert_eq!(nav_json["layout"], json!("horizontal"));
}

#[test]
fn explicit_bottom_nav_with_missing_layout_is_repaired_not_demoted() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
        "children":[
            {"type":"frame","id":"content","height":700},
            {
                "type":"frame","id":"nav","name":"Tab Bar","role":"bottom-tab-bar",
                "width":"fill_container","height":"fit_content",
                "children":[
                    {"type":"frame","id":"home-tab","role":"tab","children":[
                        {"type":"text","id":"home-label","content":"Home"}
                    ]},
                    {"type":"frame","id":"saved-tab","role":"tab","children":[
                        {"type":"text","id":"saved-label","content":"Saved"}
                    ]},
                    {"type":"frame","id":"profile-tab","role":"tab","children":[
                        {"type":"text","id":"profile-label","content":"Profile"}
                    ]}
                ]
            }
        ]
    }))
    .expect("explicit missing-layout nav fixture");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    let nav = find_node(root, "nav").expect("nav survives");
    assert_eq!(nav.base().role.as_deref(), Some("bottom-tab-bar"));
    let nav_json = serde_json::to_value(nav).expect("nav serializes");
    assert_eq!(nav_json["width"], json!("fill_container"));
    assert_eq!(nav_json["height"], json!(72.0));
    assert_eq!(nav_json["layout"], json!("horizontal"));
}

/// GLM app-03 rep-1 shape: the named bottom-nav wrapper is VERTICAL and holds
/// the real tab row (an unnamed structural row of icon+label tabs) plus a
/// home-indicator clearance frame. The surface patch must land on the tab
/// row — not on the wrapper, which would lay the clearance out beside the
/// tabs on one horizontal row.
#[test]
fn structural_tab_row_in_named_wrapper_gets_surface_patch_wrapper_stays_vertical() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "root", "name": "App", "width": 375, "height": 812,
        "layout": "vertical",
        "children": [
            {"type": "frame", "id": "content", "name": "Content",
             "width": "fill_container", "height": 718, "children": []},
            {
                "type": "frame", "id": "bottom-nav", "name": "Bottom Navigation Bar",
                "width": "fill_container", "height": 94, "layout": "vertical",
                "children": [
                    {"type": "frame", "id": "tab-row", "name": "Tab Items Row",
                     "width": "fill_container", "height": 62, "layout": "horizontal",
                     "children": [
                        tab("home", "HOME", "home"),
                        tab("cards", "CARDS", "wallet"),
                        tab("insights", "INSIGHTS", "chart-pie"),
                        tab("profile", "PROFILE", "user")
                     ]},
                    {"type": "frame", "id": "clearance", "name": "Home Indicator Clearance",
                     "width": "fill_container", "height": 22, "layout": "horizontal",
                     "children": [
                        {"type": "rectangle", "id": "indicator", "name": "Home Indicator",
                         "width": 120, "height": 5, "children": []}
                     ]}
                ]
            }
        ]
    }))
    .expect("nav wrapper with clearance json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    let wrapper = find_node(root, "bottom-nav").expect("wrapper survives");
    let wrapper_json = serde_json::to_value(wrapper).expect("wrapper serializes");
    assert_eq!(
        wrapper_json["layout"],
        json!("vertical"),
        "wrapper must stay a vertical stack above the clearance: {wrapper_json}"
    );
    assert_eq!(wrapper_json["gap"], json!(0.0));
    assert_eq!(wrapper_json["width"], json!("fill_container"));
    assert_eq!(wrapper_json["height"], json!("fit_content"));

    let row = find_node(root, "tab-row").expect("tab row survives");
    assert_eq!(
        row.base().role.as_deref(),
        Some("bottom-tab-bar"),
        "the structural tab row carries the nav surface patch"
    );
    let row_json = serde_json::to_value(row).expect("row serializes");
    assert_eq!(row_json["layout"], json!("horizontal"));
    assert_eq!(row_json["height"], json!(72.0));
    assert_eq!(row_json["width"], json!("fill_container"));
    assert_eq!(row_json["justifyContent"], json!("space_between"));

    for id in ["home-tab", "cards-tab", "insights-tab", "profile-tab"] {
        let tab_json = serde_json::to_value(find_node(root, id).expect("tab survives"))
            .expect("tab serializes");
        assert_eq!(tab_json["width"], json!("fill_container"), "tab {id}");
        assert_eq!(tab_json["height"], json!("fill_container"), "tab {id}");
        assert_eq!(tab_json["gap"], json!(4.0), "tab {id}");
    }

    let clearance = find_node(root, "clearance").expect("clearance survives");
    let clearance_json = serde_json::to_value(clearance).expect("clearance serializes");
    assert_eq!(
        clearance_json["height"],
        json!(22.0),
        "the clearance frame must be untouched: {clearance_json}"
    );
    assert_eq!(clearance_json["layout"], json!("horizontal"));
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
