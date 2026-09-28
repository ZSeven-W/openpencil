//! Bottom-nav pill chrome, spacing/tab slots and the structural bottom-nav
//! detection negatives.

use super::*;

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
    assert_eq!(
        serde_json::to_value(nav).expect("nav serializes")["width"],
        json!("fill_container")
    );
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
    assert_eq!(nav_json["width"], json!("fill_container"));
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
    assert_eq!(
        nav_json["width"],
        json!("fill_container"),
        "nav spread to full width"
    );
    assert_eq!(nav_json["justifyContent"], json!("space_between"));
    for id in ["home", "search", "orders", "profile"] {
        let tab = find_node(root, id).expect("tab survives");
        let tab_json = serde_json::to_value(tab).expect("tab serializes");
        assert_eq!(tab_json["width"], json!("fill_container"), "tab {id} fills");
    }
}

#[test]
fn cjk_named_nav_stays_inside_real_jian_width_and_stamps_semantics() {
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "root", "name": "Mobile", "width": 375, "height": 812,
        "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "header", "name": "Header Actions",
                "width": 180, "height": 56, "layout": "horizontal",
                "children": [
                    {"type": "frame", "id": "header-home", "role": "tab"},
                    {"type": "frame", "id": "header-search", "role": "tab"},
                    {"type": "frame", "id": "header-profile", "role": "tab"}
                ]
            },
            {
                "type": "frame", "id": "content", "name": "Content",
                "width": "fill_container", "height": 684
            },
            {
                "type": "frame", "id": "nav", "name": "底部导航栏",
                "width": 375, "height": 72, "layout": "horizontal",
                "padding": [8, 16, 8, 16],
                "children": [
                    {"type": "frame", "id": "home", "name": "Home Tab", "role": "tab"},
                    {"type": "frame", "id": "search", "name": "Search Tab", "role": "tab"},
                    {"type": "frame", "id": "profile", "name": "Profile Tab", "role": "tab"}
                ]
            }
        ]
    }))
    .expect("CJK nav fixture");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    let header = find_node(root, "header").expect("header");
    let nav = find_node(root, "nav").expect("nav");
    assert_eq!(
        serde_json::to_value(header).expect("header serializes")["width"],
        json!(180.0),
        "the top header row must not be normalized as bottom navigation"
    );
    assert_eq!(nav.base().role.as_deref(), Some("bottom-tab-bar"));
    assert_eq!(
        serde_json::to_value(nav).expect("nav serializes")["width"],
        json!("fill_container")
    );

    fn resolved_width(nodes: &[jian_scene::layout_scene::SceneNode], id: &str) -> Option<f64> {
        nodes.iter().find_map(|node| {
            (node.id == id)
                .then(|| f64::from(node.aggregate_bounds().size.x))
                .or_else(|| resolved_width(&node.children, id))
        })
    }
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
    let page = scene.active_page().expect("active page");
    let root_width = resolved_width(&page.children, "root").expect("resolved root");
    let nav_width = resolved_width(&page.children, "nav").expect("resolved nav");
    assert!(
        (nav_width - root_width).abs() <= 1.0,
        "nav padding must stay inside the root-width slot, got nav={nav_width} root={root_width}"
    );

    let issues = crate::geometry_validation::geometry_diagnostics(&sink.state);
    assert!(
        issues.iter().all(|issue| {
            !issue.contains("mobile-bottom-flush")
                && !issue.contains("mobile-root-bottom-nav-overflow")
        }),
        "normalized bottom navigation must not echo as missing or overflowing: {issues:?}"
    );
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
    // Full-width in FLEX FLOW: the nav must use fill sizing and carry no
    // authored x/y, otherwise padding widens it or absolute-positions it.
    assert_eq!(nav_json["width"], json!("fill_container"));
    assert!(
        nav_json.get("x").is_none_or(serde_json::Value::is_null),
        "nav must stay in flex flow (no authored x): {nav_json}"
    );
    assert_eq!(nav_json["justifyContent"], json!("space_between"));
}

#[test]
fn a_top_header_named_nav_bar_is_not_anchored_to_the_bottom() {
    // arena-m05 0929 (Gemini 3.8 Flash): `返回导航栏` (back button, title,
    // hot/latest tabs) planned first on a comments page ended up below the
    // input bar.
    let mut sink = VecDocSink::new();
    let tree: PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "root", "name": "评论列表", "width": 375, "height": 812,
        "layout": "vertical",
        "children": [
            {"type": "frame", "id": "header", "name": "返回导航栏", "width": "fill_container",
             "height": 72, "layout": "horizontal", "children": [
                {"type": "frame", "id": "back", "name": "返回按钮"},
                {"type": "text", "id": "title", "content": "全部评论 (8)"},
                {"type": "frame", "id": "tabs", "name": "排序标签"}
             ]},
            {"type": "frame", "id": "list", "name": "评论列表", "width": "fill_container", "height": 600},
            {"type": "frame", "id": "input", "name": "底部输入栏", "width": "fill_container", "height": 64}
        ]
    }))
    .expect("comments fixture");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });

    crate::cleanup::repair_mobile_structural_chrome_for_all_roots(&mut sink);
    crate::cleanup::anchor_bottom_nav_last_for_all_roots(&mut sink);

    let root = sink.state.active_children().first().expect("root");
    let order: Vec<&str> = root
        .children()
        .expect("children")
        .iter()
        .map(|child| child.id_str())
        .collect();
    assert_eq!(order, vec!["header", "list", "input"]);
    let header = find_node(root, "header").expect("header");
    assert_ne!(header.base().role.as_deref(), Some("bottom-tab-bar"));
}
