//! Tests for the app-shell restructure pass. The positive case is the reported
//! glm barbershop dashboard; the negatives are the false-positives the
//! adversarial design review flagged (top-nav, short header, mobile, already
//! horizontal, already-narrow sidebar, too-few sections, restaurant "Menu",
//! "Navy" hero, `fit_content` nav, no dashboard content, multi-screen file).

use super::*;

fn node(v: Value) -> PenNode {
    serde_json::from_value::<PenNode>(v).expect("valid PenNode fixture")
}

fn val(n: &PenNode) -> Value {
    serde_json::to_value(n).expect("serialize PenNode")
}

/// A leaf section frame with a name + sizing (children optional).
fn section(name: &str, width: Value, height: Value) -> Value {
    json!({
        "type": "frame", "id": name.replace(' ', "-"), "name": name,
        "width": width, "height": height, "layout": "vertical", "children": []
    })
}

/// The reported bug shape: vertical 1200-wide root, full-width sidebar first.
fn bug_wrapper() -> PenNode {
    node(json!({
        "type": "frame", "id": "root", "name": "Barbershop Client Management",
        "width": 1200, "height": 1775, "layout": "vertical", "gap": 32,
        "children": [
            { "type": "frame", "id": "n2", "name": "Sidebar Navigation",
              "width": 1200, "height": 605, "layout": "vertical",
              "children": [ section("Logo", json!(1152), json!(27)) ] },
            section("Top Header", json!(1200), json!(94)),
            section("Key Metrics", json!(1200), json!(117)),
            section("Client Table Section", json!(1200), json!(488)),
            section("Upcoming Appointments", json!(1200), json!(391)),
        ]
    }))
}

#[test]
fn positive_full_width_sidebar_dashboard_restructured() {
    let mut w = bug_wrapper();
    assert!(
        reshape_sidebar_to_app_shell(&mut w),
        "bug shape must restructure"
    );
    let v = val(&w);
    assert_eq!(
        layout_str(&v),
        Some("horizontal"),
        "root flips to horizontal"
    );
    let kids = v["children"].as_array().unwrap();
    assert_eq!(kids.len(), 2, "[sidebar | content]");

    let sidebar = &kids[0];
    assert!(ident_text(sidebar).contains("sidebar"));
    assert_eq!(
        num(sidebar, "width"),
        Some(SIDEBAR_WIDTH),
        "sidebar pinned to 260"
    );
    assert_eq!(
        sidebar["height"],
        json!("fill_container"),
        "sidebar stretches"
    );
    assert_eq!(sidebar["clipContent"], json!(true));

    let content = &kids[1];
    assert_eq!(content["name"], json!("Main Content"));
    assert_eq!(content["width"], json!("fill_container"));
    assert_eq!(layout_str(content), Some("vertical"));
    let sections: Vec<&str> = content["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        sections,
        [
            "Top Header",
            "Key Metrics",
            "Client Table Section",
            "Upcoming Appointments"
        ],
        "sections moved into the content column IN ORDER"
    );
}

#[test]
fn positive_full_width_sections_retargeted_to_fill() {
    let mut w = bug_wrapper();
    reshape_sidebar_to_app_shell(&mut w);
    let v = val(&w);
    let content = &v["children"][1];
    // The 1200-wide sections must become fill_container so they don't overflow
    // the ~940 content column.
    for s in content["children"].as_array().unwrap() {
        assert_eq!(
            s["width"],
            json!("fill_container"),
            "section {} retargeted",
            s["name"]
        );
    }
    // The sidebar's 1152-wide logo child likewise fills the 260 column.
    let logo = &v["children"][0]["children"][0];
    assert_eq!(logo["width"], json!("fill_container"));
}

fn assert_untouched(mut w: PenNode, why: &str) {
    let before = val(&w);
    assert!(
        !reshape_sidebar_to_app_shell(&mut w),
        "must NOT restructure: {why}"
    );
    assert_eq!(val(&w), before, "node unchanged: {why}");
}

#[test]
fn negative_top_nav_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Dashboard", "width": 1200, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "tn", "name": "Top Navigation", "width": 1200,
                  "height": 72, "layout": "horizontal", "children": [] },
                section("Key Metrics", json!(1200), json!(117)),
                section("Client Table", json!(1200), json!(400)),
            ]
        })),
        "top navigation (topbar keyword + horizontal + short)",
    );
}

#[test]
fn negative_header_short_strip_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Dashboard", "width": 1200, "layout": "vertical",
            "children": [
                section("Header", json!(1200), json!(80)),
                section("Metrics Grid", json!(1200), json!(117)),
                section("Data Table", json!(1200), json!(400)),
            ]
        })),
        "short full-width header, not a sidebar",
    );
}

#[test]
fn negative_mobile_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "App", "width": 390, "layout": "vertical",
            "children": [
                section("Sidebar Navigation", json!(390), json!(605)),
                section("Key Metrics", json!(390), json!(117)),
                section("Table", json!(390), json!(400)),
            ]
        })),
        "mobile width < 900",
    );
}

#[test]
fn negative_already_horizontal_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Dashboard", "width": 1200, "layout": "horizontal",
            "children": [
                section("Sidebar Navigation", json!(260), json!(605)),
                section("Main Content", json!("fill_container"), json!("fit_content")),
            ]
        })),
        "already app-shelled (horizontal root)",
    );
}

#[test]
fn negative_already_narrow_sidebar_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Dashboard", "width": 1200, "layout": "vertical",
            "children": [
                section("Sidebar Navigation", json!(240), json!(605)),
                section("Key Metrics", json!(1200), json!(117)),
                section("Client Table", json!(1200), json!(400)),
            ]
        })),
        "sidebar already a narrow left column (240 < 0.5*1200)",
    );
}

#[test]
fn negative_fewer_than_two_content_sections() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Dashboard", "width": 1200, "layout": "vertical",
            "children": [
                section("Sidebar Navigation", json!(1200), json!(605)),
                section("Client Table", json!(1200), json!(400)),
            ]
        })),
        "only one content section (len < 3)",
    );
}

#[test]
fn negative_restaurant_menu_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Restaurant", "width": 1280, "layout": "vertical",
            "children": [
                section("Menu", json!("fill_container"), json!(900)),
                section("About", json!(1280), json!(300)),
                section("Hours", json!(1280), json!(200)),
                section("Footer", json!(1280), json!(120)),
            ]
        })),
        "'Menu' is not a strong sidebar token (no sidebar/rail)",
    );
}

#[test]
fn negative_navy_hero_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Landing", "width": 1440, "layout": "vertical",
            "children": [
                section("Navy Hero", json!("fill_container"), json!(640)),
                section("Features", json!(1440), json!(400)),
                section("Pricing", json!(1440), json!(500)),
            ]
        })),
        "'Navy Hero' contains 'nav' substring but is not a sidebar",
    );
}

#[test]
fn negative_weak_nav_name_untouched() {
    // A WEAK nav name ("Navigation", no "sidebar"/rail token) is the real
    // false-positive risk now that fit_content heights are allowed — it must be
    // excluded by the strong-token name gate (criterion 4), not the height.
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Dashboard", "width": 1200, "layout": "vertical",
            "children": [
                { "type": "frame", "id": "nav", "name": "Navigation",
                  "width": "fill_container", "height": "fit_content", "layout": "vertical",
                  "children": [] },
                section("Key Metrics", json!(1200), json!(117)),
                section("Client Table", json!(1200), json!(400)),
            ]
        })),
        "'Navigation' is not a strong sidebar token (criterion 4)",
    );
}

#[test]
fn positive_horizontal_root_fit_content_sidebar_restructured() {
    // The orchestrator also emits the bug as a HORIZONTAL root with the sidebar
    // AND every section crammed into one row, the sidebar sized fill_container /
    // fit_content (the op-smoke barbershop output). This must restructure too.
    let mut w = node(json!({
        "type": "frame", "id": "r", "name": "Barbershop Dashboard",
        "width": 1200, "layout": "horizontal", "gap": 24,
        "children": [
            { "type": "frame", "id": "sb", "name": "Left Sidebar Navigation",
              "width": "fill_container", "height": "fit_content", "layout": "vertical",
              "children": [] },
            section("Top Header Bar", json!("fill_container"), json!("fit_content")),
            section("Key Metrics Row", json!("fill_container"), json!("fit_content")),
            section("Recent Clients Table", json!("fill_container"), json!("fit_content")),
        ]
    }));
    assert!(
        reshape_sidebar_to_app_shell(&mut w),
        "horizontal-root fit_content sidebar must restructure"
    );
    let v = val(&w);
    assert_eq!(layout_str(&v), Some("horizontal"));
    let kids = v["children"].as_array().unwrap();
    assert_eq!(kids.len(), 2, "[sidebar | Main Content]");
    assert_eq!(
        num(&kids[0], "width"),
        Some(SIDEBAR_WIDTH),
        "sidebar narrowed to 260"
    );
    assert_eq!(kids[1]["name"], json!("Main Content"));
    let inner: Vec<&str> = kids[1]["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        inner,
        ["Top Header Bar", "Key Metrics Row", "Recent Clients Table"],
        "the row sections moved into the content column"
    );
}

#[test]
fn negative_no_dashboard_content_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Marketing Site", "width": 1280, "layout": "vertical",
            "children": [
                section("Sidebar Navigation", json!(1280), json!(605)),
                section("About Us", json!(1280), json!(300)),
                section("Our Team", json!(1280), json!(300)),
                section("Contact", json!(1280), json!(200)),
            ]
        })),
        "no table/metric/chart sections (structural dashboard gate)",
    );
}

#[test]
fn negative_multiscreen_wrapper_untouched() {
    assert_untouched(
        node(json!({
            "type": "frame", "id": "r", "name": "Flows", "width": 1440, "layout": "vertical",
            "children": [
                section("Navigation Rail Screen", json!(1440), json!(900)),
                section("Detail Screen", json!(1440), json!(900)),
                section("Settings Screen", json!(1440), json!(900)),
            ]
        })),
        "multiple standalone screens, not dashboard sections",
    );
}

/// A vertical 1200-root with a custom sidebar + the two dashboard sections that
/// pass the gate; lets the sidebar-footer tests vary only the sidebar.
fn wrapper_with_sidebar(sidebar_children: Value) -> PenNode {
    node(json!({
        "type": "frame", "id": "root", "name": "Dashboard",
        "width": 1200, "height": 1400, "layout": "vertical", "gap": 24,
        "children": [
            { "type": "frame", "id": "sb", "name": "Sidebar Navigation",
              "width": 1200, "height": 700, "layout": "vertical", "children": sidebar_children },
            section("Key Metrics", json!(1200), json!(117)),
            section("Client Table", json!(1200), json!(400)),
            section("Upcoming", json!(1200), json!(300)),
        ]
    }))
}

#[test]
fn content_column_gets_outer_padding() {
    let mut w = bug_wrapper();
    reshape_sidebar_to_app_shell(&mut w);
    let v = val(&w);
    // The Main Content column carries the Pencil app-shell gutter so the
    // sections don't run edge-to-edge into the viewport (serialized as f64).
    let pad: Vec<f64> = v["children"][1]["padding"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_f64().unwrap())
        .collect();
    assert_eq!(pad, [32.0, 40.0]);
}

#[test]
fn sidebar_fixed_spacer_stretched_to_sink_footer() {
    let mut w = wrapper_with_sidebar(json!([
        { "type": "frame", "id": "logo", "name": "Brand Logo", "height": 29, "children": [] },
        { "type": "frame", "id": "nav", "name": "Nav Section", "height": 263, "children": [] },
        { "type": "frame", "id": "sp", "name": "Spacer", "height": 120, "children": [] },
        { "type": "frame", "id": "usr", "name": "User Profile Card", "height": 72, "children": [] }
    ]));
    assert!(reshape_sidebar_to_app_shell(&mut w));
    let v = val(&w);
    let sb_kids = v["children"][0]["children"].as_array().unwrap();
    let spacer = sb_kids
        .iter()
        .find(|c| c["name"] == json!("Spacer"))
        .unwrap();
    assert_eq!(
        spacer["height"],
        json!("fill_container"),
        "fixed spacer stretched"
    );
}

#[test]
fn sidebar_without_spacer_gets_one_injected_before_footer() {
    let mut w = wrapper_with_sidebar(json!([
        { "type": "frame", "id": "logo", "name": "Brand Logo", "height": 29, "children": [] },
        { "type": "frame", "id": "nav", "name": "Nav Section", "height": 263, "children": [] },
        { "type": "frame", "id": "usr", "name": "User Profile Card", "height": 72, "children": [] }
    ]));
    assert!(reshape_sidebar_to_app_shell(&mut w));
    let v = val(&w);
    let sb_kids = v["children"][0]["children"].as_array().unwrap();
    // A flexible spacer was inserted right before the footer card.
    assert_eq!(sb_kids.len(), 4, "spacer injected");
    assert_eq!(sb_kids[2]["name"], json!("Sidebar Spacer"));
    assert_eq!(sb_kids[2]["height"], json!("fill_container"));
    assert_eq!(
        sb_kids[3]["name"],
        json!("User Profile Card"),
        "footer stays last"
    );
}
