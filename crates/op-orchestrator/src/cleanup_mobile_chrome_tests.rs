use super::*;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::container::CornerRadius;
use serde_json::json;

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "P".into(),
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

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

#[test]
fn cleanup_strips_mobile_search_categories_section_shell() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Food App Home",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFF8F0" }],
            "children": [
                {
                    "type": "frame",
                    "id": "search-categories-shell",
                    "name": "Search And Categories",
                    "role": "section",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "padding": [16, 24],
                    "gap": 12,
                    "cornerRadius": 28,
                    "fill": [{ "type": "solid", "color": "#D7EBFF" }],
                    "stroke": {
                        "thickness": 1,
                        "fill": [{ "type": "solid", "color": "#C7DDF5" }]
                    },
                    "effects": [{
                        "type": "shadow",
                        "offsetX": 0,
                        "offsetY": 10,
                        "blur": 22,
                        "spread": 0,
                        "color": "#0000001F"
                    }],
                    "children": [
                        {
                            "type": "frame",
                            "id": "search-bar",
                            "name": "Search Bar",
                            "role": "search-bar",
                            "width": "fill_container",
                            "height": 50,
                            "cornerRadius": 16,
                            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                            "stroke": {
                                "thickness": 1,
                                "fill": [{ "type": "solid", "color": "#E8D4C4" }]
                            },
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "category-row",
                            "name": "Category Chips",
                            "width": "fill_container",
                            "height": 78,
                            "children": []
                        }
                    ]
                },
                {
                    "type": "frame",
                    "id": "promo-card",
                    "name": "Limited Offer Promo Card",
                    "role": "card",
                    "width": "fill_container",
                    "height": 168,
                    "cornerRadius": 24,
                    "fill": [{ "type": "solid", "color": "#FF6B00" }],
                    "children": []
                }
            ]
        }"##,
    )
    .expect("mobile shell json");
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
    let shell = find_node(root, "search-categories-shell").expect("shell survives");
    let promo = find_node(root, "promo-card").expect("promo survives");
    match shell {
        PenNode::Frame(frame) => {
            assert!(
                frame.container.fill.is_none(),
                "structural shell fill is removed"
            );
            assert!(
                frame.container.stroke.is_none(),
                "structural shell stroke is removed"
            );
            assert!(
                frame.container.effects.is_none(),
                "structural shell shadow is removed"
            );
            assert_eq!(
                frame.container.corner_radius,
                Some(CornerRadius::Uniform(0.0)),
                "structural shell radius is neutralized"
            );
        }
        _ => panic!("shell should be a frame"),
    }
    assert_eq!(
        first_solid_fill_hex(promo),
        Some("#FF6B00"),
        "real promo/card surfaces should keep their visual treatment"
    );
}

#[test]
fn cleanup_strips_misrolled_mobile_search_bar_wrapper() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Food App Home",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFF8F0" }],
            "children": [
                {
                    "type": "frame",
                    "id": "outer-search",
                    "name": "Search Bar",
                    "role": "search-bar",
                    "width": "fill_container",
                    "height": 84,
                    "layout": "horizontal",
                    "padding": [10, 16],
                    "cornerRadius": 28,
                    "fill": [{ "type": "solid", "color": "#D7EBFF" }],
                    "stroke": {
                        "thickness": 1,
                        "fill": [{ "type": "solid", "color": "#D0E2F4" }]
                    },
                    "children": [
                        {
                            "type": "frame",
                            "id": "real-input",
                            "name": "Search Input",
                            "role": "form-input",
                            "width": "fill_container",
                            "height": 48,
                            "cornerRadius": 14,
                            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                            "stroke": {
                                "thickness": 1,
                                "fill": [{ "type": "solid", "color": "#E2E8F0" }]
                            },
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "filter-button",
                            "name": "Filter Button",
                            "role": "icon-button",
                            "width": 48,
                            "height": 48,
                            "cornerRadius": 14,
                            "fill": [{ "type": "solid", "color": "#FF6B00" }],
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    )
    .expect("mobile search json");
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
    let outer = find_node(root, "outer-search").expect("outer search survives");
    let input = find_node(root, "real-input").expect("input survives");
    let filter = find_node(root, "filter-button").expect("filter survives");
    match outer {
        PenNode::Frame(frame) => {
            assert!(
                frame.container.fill.is_none(),
                "outer search fill is removed"
            );
            assert!(
                frame.container.stroke.is_none(),
                "outer search stroke is removed"
            );
            assert_eq!(
                frame.container.corner_radius,
                Some(CornerRadius::Uniform(0.0)),
                "outer search radius is neutralized"
            );
        }
        _ => panic!("outer search should be a frame"),
    }
    assert_eq!(first_solid_fill_hex(input), Some("#FFFFFF"));
    assert_eq!(first_solid_fill_hex(filter), Some("#FF6B00"));
}

#[test]
fn cleanup_anchors_short_mobile_bottom_nav_to_viewport_bottom() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Food App Home",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "gap": 20,
            "fill": [{ "type": "solid", "color": "#FFF8F0" }],
            "children": [
                {
                    "type": "frame",
                    "id": "status",
                    "name": "Status Bar",
                    "role": "status-bar",
                    "width": "fill_container",
                    "height": 62
                },
                {
                    "type": "frame",
                    "id": "content",
                    "name": "Popular Near You",
                    "role": "section",
                    "width": "fill_container",
                    "height": "fit_content",
                    "layout": "vertical",
                    "children": [
                        {
                            "type": "frame",
                            "id": "restaurant-card",
                            "name": "Restaurant Card",
                            "width": "fill_container",
                            "height": 420
                        }
                    ]
                },
                {
                    "type": "frame",
                    "id": "bottom-nav",
                    "name": "Bottom Navigation",
                    "role": "bottom-tab-bar",
                    "width": "fill_container",
                    "height": 64,
                    "layout": "horizontal",
                    "children": [
                        {"type": "frame", "id": "home-tab", "name": "Home", "role": "tab", "width": "fill_container", "height": "fill_container"},
                        {"type": "frame", "id": "search-tab", "name": "Search", "role": "tab", "width": "fill_container", "height": "fill_container"},
                        {"type": "frame", "id": "orders-tab", "name": "Orders", "role": "tab", "width": "fill_container", "height": "fill_container"},
                        {"type": "frame", "id": "profile-tab", "name": "Profile", "role": "tab", "width": "fill_container", "height": "fill_container"}
                    ]
                }
            ]
        }"##,
    )
    .expect("mobile bottom nav json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    run_cleanup_passes(&mut sink, &plan(), &["root"]);

    assert!(
        sink.applied.iter().any(|cmd| matches!(
            cmd,
            EditorCommand::SetNodeLayoutProp { node_id, property, value }
                if node_id.as_str() == "content"
                    && property == "height"
                    && matches!(value, LayoutPropValue::Keyword(keyword) if keyword == "fill_container")
        )),
        "cleanup should make the content before a short mobile bottom nav fill remaining viewport height: {:?}",
        sink.applied
    );
}

#[test]
fn cleanup_strips_mobile_bottom_nav_pill_chrome() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Food App Home",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFF8F0" }],
            "children": [
                {
                    "type": "frame",
                    "id": "content",
                    "name": "Content",
                    "width": "fill_container",
                    "height": 760,
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "bottom-nav",
                    "name": "Bottom Navigation",
                    "role": "bottom-tab-bar",
                    "width": 320,
                    "height": 72,
                    "layout": "horizontal",
                    "padding": [8, 24],
                    "gap": 0,
                    "cornerRadius": 999,
                    "fill": [{ "type": "solid", "color": "#FFFFFF" }],
                    "stroke": {
                        "thickness": 1,
                        "fill": [{ "type": "solid", "color": "#E8D4C4" }]
                    },
                    "effects": [{
                        "type": "shadow",
                        "offsetX": 0,
                        "offsetY": -8,
                        "blur": 18,
                        "spread": 0,
                        "color": "#0000001F"
                    }],
                    "children": [
                        {
                            "type": "frame",
                            "id": "home-tab",
                            "name": "Home Tab",
                            "role": "button",
                            "width": "fill_container",
                            "height": "fill_container",
                            "layout": "vertical",
                            "cornerRadius": 999,
                            "fill": [{ "type": "solid", "color": "#FFF0B8" }],
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "search-tab",
                            "name": "Search Tab",
                            "role": "button",
                            "width": "fill_container",
                            "height": "fill_container",
                            "layout": "vertical",
                            "cornerRadius": 999,
                            "fill": [{ "type": "solid", "color": "#F8FAFC" }],
                            "stroke": {
                                "thickness": 1,
                                "fill": [{ "type": "solid", "color": "#E2E8F0" }]
                            },
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "orders-tab",
                            "name": "Orders Tab",
                            "role": "button",
                            "width": "fill_container",
                            "height": "fill_container",
                            "layout": "vertical",
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "profile-tab",
                            "name": "Profile Tab",
                            "role": "button",
                            "width": "fill_container",
                            "height": "fill_container",
                            "layout": "vertical",
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    )
    .expect("mobile nav json");
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
    let nav = find_node(root, "bottom-nav").expect("nav survives");
    let home = find_node(root, "home-tab").expect("home tab survives");
    let search = find_node(root, "search-tab").expect("search tab survives");
    assert_eq!(nav.width_px(), Some(390.0));
    match nav {
        PenNode::Frame(frame) => {
            assert!(frame.container.stroke.is_none(), "nav stroke is removed");
            assert!(frame.container.effects.is_none(), "nav shadow is removed");
            assert_eq!(
                frame.container.corner_radius,
                Some(CornerRadius::Uniform(0.0)),
                "nav rounded pill is neutralized"
            );
        }
        _ => panic!("bottom nav should be a frame"),
    }
    for tab in [home, search] {
        match tab {
            PenNode::Frame(frame) => {
                assert!(frame.container.fill.is_none(), "tab fill is removed");
                assert!(frame.container.stroke.is_none(), "tab stroke is removed");
                assert_eq!(
                    frame.container.corner_radius,
                    Some(CornerRadius::Uniform(0.0)),
                    "tab rounded tile is neutralized"
                );
            }
            _ => panic!("tab should be a frame"),
        }
    }
}

#[test]
fn cleanup_normalizes_mobile_bottom_nav_spacing_and_tab_slots() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame",
            "id": "root",
            "name": "Food App Home",
            "width": 390,
            "height": 844,
            "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFF8F0" }],
            "children": [
                {
                    "type": "frame",
                    "id": "content",
                    "name": "Content",
                    "width": "fill_container",
                    "height": 760,
                    "children": []
                },
                {
                    "type": "frame",
                    "id": "bottom-nav",
                    "name": "Bottom Navigation",
                    "role": "bottom-tab-bar",
                    "width": 280,
                    "height": 64,
                    "layout": "horizontal",
                    "padding": [0, 0],
                    "gap": 20,
                    "children": [
                        {
                            "type": "frame",
                            "id": "home-tab",
                            "name": "Home Tab",
                            "role": "tab",
                            "width": 44,
                            "height": 56,
                            "layout": "vertical",
                            "gap": 10,
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "search-tab",
                            "name": "Search Tab",
                            "role": "tab",
                            "width": 44,
                            "height": 56,
                            "layout": "vertical",
                            "gap": 10,
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "orders-tab",
                            "name": "Orders Tab",
                            "role": "tab",
                            "width": 44,
                            "height": 56,
                            "layout": "vertical",
                            "gap": 10,
                            "children": []
                        },
                        {
                            "type": "frame",
                            "id": "profile-tab",
                            "name": "Profile Tab",
                            "role": "tab",
                            "width": 44,
                            "height": 56,
                            "layout": "vertical",
                            "gap": 10,
                            "children": []
                        }
                    ]
                }
            ]
        }"##,
    )
    .expect("mobile nav json");
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
    let nav = find_node(root, "bottom-nav").expect("nav survives");
    let nav_json = serde_json::to_value(nav).expect("nav serializes");
    assert_eq!(nav_json["width"], json!(390.0));
    assert_eq!(nav_json["height"], json!(72.0));
    assert_eq!(nav_json["gap"], json!(0.0));
    assert_eq!(nav_json["padding"], json!([8.0, 16.0, 8.0, 16.0]));
    assert_eq!(nav_json["justifyContent"], json!("space_between"));
    assert_eq!(nav_json["alignItems"], json!("center"));

    for id in ["home-tab", "search-tab", "orders-tab", "profile-tab"] {
        let tab = find_node(root, id).expect("tab survives");
        let tab_json = serde_json::to_value(tab).expect("tab serializes");
        assert_eq!(tab_json["width"], json!("fill_container"));
        assert_eq!(tab_json["height"], json!("fill_container"));
        assert_eq!(tab_json["gap"], json!(4.0));
        assert_eq!(tab_json["padding"], json!([4.0, 0.0]));
        assert_eq!(tab_json["justifyContent"], json!("center"));
        assert_eq!(tab_json["alignItems"], json!("center"));
    }
}

#[test]
fn cleanup_normalizes_structurally_detected_bottom_nav_without_role_or_name() {
    // A nav row the model left UNTAGGED — generic name, no role="bottom-tab-bar",
    // just a horizontal row of Home/Search/Orders/Profile items — must still be
    // spread to full width (the "拥挤" crammed-nav fix).
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame", "id": "root", "name": "Food App Home",
            "width": 390, "height": 844, "layout": "vertical",
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                { "type": "frame", "id": "content", "name": "Content", "width": "fill_container", "height": 760, "children": [] },
                {
                    "type": "frame", "id": "footer", "name": "Footer",
                    "width": 240, "height": 64, "layout": "horizontal", "gap": 16,
                    "children": [
                        { "type": "frame", "id": "home", "name": "Home", "width": 48, "height": 56, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "home-i", "iconFontName": "home", "width": 24, "height": 24 },
                            { "type": "text", "id": "home-t", "content": "Home" }
                        ] },
                        { "type": "frame", "id": "search", "name": "Search", "width": 48, "height": 56, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "search-i", "iconFontName": "search", "width": 24, "height": 24 },
                            { "type": "text", "id": "search-t", "content": "Search" }
                        ] },
                        { "type": "frame", "id": "orders", "name": "Orders", "width": 48, "height": 56, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "orders-i", "iconFontName": "clipboard", "width": 24, "height": 24 },
                            { "type": "text", "id": "orders-t", "content": "Orders" }
                        ] },
                        { "type": "frame", "id": "profile", "name": "Profile", "width": 48, "height": 56, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "profile-i", "iconFontName": "user", "width": 24, "height": 24 },
                            { "type": "text", "id": "profile-t", "content": "Profile" }
                        ] }
                    ]
                }
            ]
        }"##,
    )
    .expect("nav json");
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
    let nav = find_node(root, "footer").expect("nav survives");
    let nav_json = serde_json::to_value(nav).expect("nav serializes");
    assert_eq!(nav_json["width"], json!(390.0), "nav spread to full width");
    assert_eq!(nav_json["justifyContent"], json!("space_between"));
    for id in ["home", "search", "orders", "profile"] {
        let tab = find_node(root, id).expect("tab survives");
        let tab_json = serde_json::to_value(tab).expect("tab serializes");
        assert_eq!(tab_json["width"], json!("fill_container"), "tab {id} fills");
    }
}

#[test]
fn cleanup_does_not_treat_header_action_row_as_bottom_nav() {
    // Guard against over-broad structural detection: a HEADER row at the TOP —
    // even one of LABELED icon+text buttons named like nav destinations — must
    // NOT be mistaken for a bottom nav and stretched to a full-width tab bar.
    // Only the LAST top-level section is eligible for the structural fallback.
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame", "id": "root", "name": "Home", "width": 390, "height": 844,
            "layout": "vertical", "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                {
                    "type": "frame", "id": "header-actions", "name": "Header Actions",
                    "width": 180, "height": 56, "layout": "horizontal", "gap": 12,
                    "children": [
                        { "type": "frame", "id": "search-btn", "name": "Search", "width": 48, "height": 52, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "s-i", "iconFontName": "search", "width": 24, "height": 24 },
                            { "type": "text", "id": "s-t", "content": "Search" }
                        ] },
                        { "type": "frame", "id": "cart-btn", "name": "Cart", "width": 48, "height": 52, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "c-i", "iconFontName": "shopping-cart", "width": 24, "height": 24 },
                            { "type": "text", "id": "c-t", "content": "Cart" }
                        ] },
                        { "type": "frame", "id": "profile-btn", "name": "Profile", "width": 48, "height": 52, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "p-i", "iconFontName": "user", "width": 24, "height": 24 },
                            { "type": "text", "id": "p-t", "content": "Profile" }
                        ] }
                    ]
                },
                { "type": "frame", "id": "content", "name": "Content", "width": "fill_container", "height": 700, "children": [] }
            ]
        }"##,
    )
    .expect("header json");
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
    let header = find_node(root, "header-actions").expect("header survives");
    let header_json = serde_json::to_value(header).expect("header serializes");
    // The header keeps its own width — it was NOT stretched to a full-width nav.
    assert_eq!(
        header_json["width"],
        json!(180.0),
        "a top header action row must not be normalized as a bottom nav: {header_json}"
    );
}

#[test]
fn cleanup_does_not_treat_named_top_navbar_as_bottom_nav() {
    // A TOP navbar (named "Navigation Bar", first section) with labeled nav
    // items must NOT be mistaken for a bottom nav. "navbar"/"nav bar"/"tab bar"
    // are ambiguous names that are no longer matched; only "bottom …" names or
    // the bottom-gated structural fallback qualify. (Codex: named top navs.)
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame", "id": "root", "name": "Home", "width": 390, "height": 844,
            "layout": "vertical", "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                {
                    "type": "frame", "id": "topnav", "name": "Navigation Bar",
                    "width": 220, "height": 56, "layout": "horizontal", "gap": 12,
                    "children": [
                        { "type": "frame", "id": "t-home", "name": "Home", "width": 56, "height": 52, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "t1i", "iconFontName": "home", "width": 24, "height": 24 },
                            { "type": "text", "id": "t1t", "content": "Home" } ] },
                        { "type": "frame", "id": "t-search", "name": "Search", "width": 56, "height": 52, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "t2i", "iconFontName": "search", "width": 24, "height": 24 },
                            { "type": "text", "id": "t2t", "content": "Search" } ] },
                        { "type": "frame", "id": "t-profile", "name": "Profile", "width": 56, "height": 52, "layout": "vertical", "children": [
                            { "type": "icon_font", "id": "t3i", "iconFontName": "user", "width": 24, "height": 24 },
                            { "type": "text", "id": "t3t", "content": "Profile" } ] }
                    ]
                },
                { "type": "frame", "id": "content", "name": "Content", "width": "fill_container", "height": 700, "children": [] }
            ]
        }"##,
    )
    .expect("topnav json");
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
    let topnav = find_node(root, "topnav").expect("topnav survives");
    let topnav_json = serde_json::to_value(topnav).expect("topnav serializes");
    assert_eq!(
        topnav_json["width"],
        json!(220.0),
        "a top navbar must not be normalized as a bottom nav: {topnav_json}"
    );
}

#[test]
fn cleanup_structural_nav_only_matches_bottom_row_inside_single_wrapper() {
    // The whole screen is ONE content wrapper (the only top-level child). The
    // structural fallback must apply ONLY to the wrapper's LAST child (the real
    // bottom nav) — a labeled nav-named row at the TOP of the wrapper (a header)
    // must NOT be stretched. (Codex: nested header rows over-matched.)
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_str(
        r##"{
            "type": "frame", "id": "root", "name": "Home", "width": 390, "height": 844,
            "layout": "vertical", "fill": [{ "type": "solid", "color": "#FFFFFF" }],
            "children": [
                {
                    "type": "frame", "id": "wrapper", "name": "Content Wrapper",
                    "width": "fill_container", "height": "fit_content", "layout": "vertical",
                    "children": [
                        {
                            "type": "frame", "id": "top-row", "name": "Quick Links",
                            "width": 190, "height": 56, "layout": "horizontal", "gap": 12,
                            "children": [
                                { "type": "frame", "id": "h-home", "name": "Home", "width": 56, "height": 52, "layout": "vertical", "children": [
                                    { "type": "icon_font", "id": "h1i", "iconFontName": "home", "width": 24, "height": 24 },
                                    { "type": "text", "id": "h1t", "content": "Home" } ] },
                                { "type": "frame", "id": "h-search", "name": "Search", "width": 56, "height": 52, "layout": "vertical", "children": [
                                    { "type": "icon_font", "id": "h2i", "iconFontName": "search", "width": 24, "height": 24 },
                                    { "type": "text", "id": "h2t", "content": "Search" } ] },
                                { "type": "frame", "id": "h-profile", "name": "Profile", "width": 56, "height": 52, "layout": "vertical", "children": [
                                    { "type": "icon_font", "id": "h3i", "iconFontName": "user", "width": 24, "height": 24 },
                                    { "type": "text", "id": "h3t", "content": "Profile" } ] }
                            ]
                        },
                        { "type": "frame", "id": "body", "name": "Body", "width": "fill_container", "height": 600, "children": [] },
                        {
                            "type": "frame", "id": "nav-row", "name": "Nav",
                            "width": 200, "height": 64, "layout": "horizontal", "gap": 12,
                            "children": [
                                { "type": "frame", "id": "n-home", "name": "Home", "width": 56, "height": 56, "layout": "vertical", "children": [
                                    { "type": "icon_font", "id": "n1i", "iconFontName": "home", "width": 24, "height": 24 },
                                    { "type": "text", "id": "n1t", "content": "Home" } ] },
                                { "type": "frame", "id": "n-orders", "name": "Orders", "width": 56, "height": 56, "layout": "vertical", "children": [
                                    { "type": "icon_font", "id": "n2i", "iconFontName": "clipboard", "width": 24, "height": 24 },
                                    { "type": "text", "id": "n2t", "content": "Orders" } ] },
                                { "type": "frame", "id": "n-profile", "name": "Profile", "width": 56, "height": 56, "layout": "vertical", "children": [
                                    { "type": "icon_font", "id": "n3i", "iconFontName": "user", "width": 24, "height": 24 },
                                    { "type": "text", "id": "n3t", "content": "Profile" } ] }
                            ]
                        }
                    ]
                }
            ]
        }"##,
    )
    .expect("wrapper json");
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
    // Top header row keeps its own width (NOT mistaken for a nav).
    let top = find_node(root, "top-row").expect("top-row survives");
    let top_json = serde_json::to_value(top).expect("top serializes");
    assert_eq!(
        top_json["width"],
        json!(190.0),
        "a nested header row at the top must not be normalized as a nav: {top_json}"
    );
    // The bottom nav row IS spread to full width.
    let nav = find_node(root, "nav-row").expect("nav-row survives");
    let nav_json = serde_json::to_value(nav).expect("nav serializes");
    // Full-width in FLEX FLOW: numeric root-width or fill_container both
    // qualify. The nav must NOT carry an authored x/y — that reads as
    // absolute placement and buries it at the root's top-left corner.
    assert!(
        nav_json["width"] == json!(390.0) || nav_json["width"] == json!("fill_container"),
        "the bottom nav row should still be spread full-width: {nav_json}"
    );
    assert!(
        nav_json.get("x").is_none_or(serde_json::Value::is_null),
        "nav must stay in flex flow (no authored x): {nav_json}"
    );
    assert_eq!(nav_json["justifyContent"], json!("space_between"));
}
