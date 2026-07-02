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

// ── sink_structured_sidebar_footers (already-correct [sidebar | content] shell) ──

#[test]
fn structured_sidebar_flat_nav_sinks_unnamed_owner_footer() {
    // The glm shape the app-shell reshape does NOT touch (root is already a
    // proper horizontal shell): a `Left Sidebar Navigation` column that stacks
    // brand + 2 nav groups + a Go-Pro card + an UNNAMED user footer flat, hug
    // height, no space_between / spacer. The footer must sink: column → fill,
    // flexible spacer injected before the footer.
    let mut root = node(json!({
        "type": "frame", "id": "root", "name": "Dashboard", "layout": "horizontal",
        "width": 1200, "height": "fit_content", "children": [
            { "type": "frame", "id": "sb", "name": "Sidebar", "layout": "vertical",
              "width": 260, "height": "fill_container", "children": [
                { "type": "frame", "id": "nav", "name": "Left Sidebar Navigation",
                  "layout": "vertical", "width": "fill_container", "height": "fit_content",
                  "children": [
                    { "type": "frame", "id": "brand", "name": "Brand", "children": [] },
                    { "type": "frame", "id": "g1", "name": "Overview Group", "children": [] },
                    { "type": "frame", "id": "g2", "name": "Manage Group", "children": [] },
                    { "type": "frame", "id": "pro", "children": [
                        { "type": "text", "id": "pro-t", "content": "Go Pro" }
                    ]},
                    { "type": "frame", "id": "ftr", "children": [
                        { "type": "text", "id": "ftr-n", "content": "James Miller" },
                        { "type": "text", "id": "ftr-r", "content": "Shop Owner" }
                    ]}
                  ]}
              ]},
            { "type": "frame", "id": "main", "name": "Main Content", "layout": "vertical",
              "width": "fill_container", "height": "fit_content", "children": [] }
        ]
    }));
    assert!(
        sink_structured_sidebar_footers(&mut root),
        "must sink the flat-nav footer"
    );
    let v = val(&root);
    let nav = &v["children"][0]["children"][0];
    assert_eq!(
        nav["height"],
        json!("fill_container"),
        "nav column promoted so the spacer has room"
    );
    let nk = nav["children"].as_array().unwrap();
    assert_eq!(nk.len(), 6, "a spacer was injected (5 → 6)");
    assert_eq!(
        nk[4]["name"],
        json!("Sidebar Spacer"),
        "spacer before footer"
    );
    assert_eq!(nk[4]["height"], json!("fill_container"));
    // The owner footer (last, unnamed — caught by content signal) stays last.
    assert_eq!(nk[5]["id"], json!("ftr"));
}

#[test]
fn structured_sidebar_pure_nav_without_footer_is_left_alone() {
    // A sidebar nav whose last child is just another nav link (no owner/Pro/user
    // footer signal) must NOT be touched — pinning a nav link to the bottom would
    // be wrong.
    let mut root = node(json!({
        "type": "frame", "id": "root", "layout": "horizontal", "width": 1200, "children": [
            { "type": "frame", "id": "sb", "name": "Sidebar Nav", "layout": "vertical",
              "width": 240, "height": "fill_container", "children": [
                { "type": "frame", "id": "l1", "children": [{ "type": "text", "id": "l1t", "content": "Home" }] },
                { "type": "frame", "id": "l2", "children": [{ "type": "text", "id": "l2t", "content": "Reports" }] },
                { "type": "frame", "id": "l3", "children": [{ "type": "text", "id": "l3t", "content": "Calendar" }] }
              ]}
        ]
    }));
    assert!(
        !sink_structured_sidebar_footers(&mut root),
        "no footer signal → no change"
    );
}

// ── evict_content_from_sidebar_column ──

/// A horizontal ROW frame with `cols` text cells — the table-row shape.
fn table_row(id: &str, cols: usize) -> Value {
    let cells: Vec<Value> = (0..cols)
        .map(|c| json!({ "type": "text", "id": format!("{id}-c{c}"), "content": "x" }))
        .collect();
    json!({ "type": "frame", "id": id, "name": "Row", "layout": "horizontal", "children": cells })
}

/// A `[Sidebar | Main Content]` shell. `sidebar_children` populate the 260px
/// rail; Main Content starts with a single stats section.
fn shell(sidebar_children: Vec<Value>) -> PenNode {
    node(json!({
        "type": "frame", "id": "root", "name": "Barbershop Client Management",
        "layout": "horizontal", "width": 1200, "alignItems": "stretch", "children": [
            { "type": "frame", "id": "sb", "name": "Sidebar", "layout": "vertical",
              "width": 260, "height": "fill_container", "children": sidebar_children },
            { "type": "frame", "id": "mc", "name": "Main Content", "layout": "vertical",
              "width": "fill_container", "height": "fit_content", "children": [
                section("Overview Stats", json!("fill_container"), json!("fit_content"))
            ]}
        ]
    }))
}

/// A sidebar nav column whose menu carries an "Analytics" item (2-child rows) —
/// the false-positive guard: its name/structure must NOT read as a data table.
fn nav_with_analytics() -> Value {
    json!({
        "type": "frame", "id": "nav", "name": "Sidebar Navigation", "layout": "vertical",
        "justifyContent": "space_between", "children": [
            { "type": "frame", "id": "navgrp", "name": "Nav Group", "layout": "vertical", "children": [
                { "type": "frame", "id": "ni1", "name": "Analytics", "layout": "horizontal", "children": [
                    { "type": "text", "id": "ni1i", "content": "•" },
                    { "type": "text", "id": "ni1t", "content": "Analytics" }
                ]},
                { "type": "frame", "id": "ni2", "name": "Settings", "layout": "horizontal", "children": [
                    { "type": "text", "id": "ni2i", "content": "•" },
                    { "type": "text", "id": "ni2t", "content": "Settings" }
                ]}
            ]}
        ]
    })
}

#[test]
fn client_directory_table_evicted_from_sidebar_to_main_content() {
    // The reported bug: a full data-table section landed in the 260px sidebar.
    let client_directory = json!({
        "type": "frame", "id": "cd", "name": "Client Directory", "layout": "vertical",
        "width": "fill_container", "children": [
            { "type": "frame", "id": "tbl", "name": "Table", "layout": "vertical", "children": [
                table_row("hdr", 6), table_row("r1", 6), table_row("r2", 6)
            ]}
        ]
    });
    let mut root = shell(vec![nav_with_analytics(), client_directory]);
    assert!(
        evict_content_from_sidebar_column(&mut root),
        "the data section must be evicted"
    );
    let v = val(&root);
    let kids = v["children"].as_array().unwrap();
    let sidebar = &kids[0];
    let content = &kids[1];
    // Sidebar keeps ONLY the nav (the "Analytics" menu item did not drag it out).
    let sb_kids = sidebar["children"].as_array().unwrap();
    assert_eq!(sb_kids.len(), 1, "only the nav remains in the rail");
    assert!(ident_text(&sb_kids[0]).contains("navigation"));
    // Client Directory now lives in Main Content.
    let mc_kids = content["children"].as_array().unwrap();
    assert_eq!(mc_kids.len(), 2, "stats + relocated directory");
    assert!(
        mc_kids
            .iter()
            .any(|c| ident_text(c).contains("client directory")),
        "directory moved into the content column"
    );
}

#[test]
fn neutrally_named_section_with_named_table_is_evicted() {
    // The section name is neutral ("Panel"); it is detected via a `table`-named
    // frame in its subtree that has a real multi-row body.
    let panel = json!({
        "type": "frame", "id": "pnl", "name": "Panel", "layout": "vertical",
        "width": "fill_container", "children": [
            { "type": "frame", "id": "grid", "name": "Client Table", "layout": "vertical", "children": [
                table_row("g0", 5), table_row("g1", 5)
            ]}
        ]
    });
    let mut root = shell(vec![nav_with_analytics(), panel]);
    assert!(
        evict_content_from_sidebar_column(&mut root),
        "a named data table is content even under a neutral section name"
    );
    let v = val(&root);
    assert_eq!(v["children"][0]["children"].as_array().unwrap().len(), 1);
    assert_eq!(v["children"][1]["children"].as_array().unwrap().len(), 2);
}

#[test]
fn nav_with_multi_child_items_is_not_evicted() {
    // Regression: a weak model gave each nav item FOUR children (icon, label,
    // badge, chevron). A bare "≥2 rows × ≥4 cols" table heuristic evicted the
    // ENTIRE navigation, emptying the rail. The name gate ("Navigation" is not
    // a table) must keep it in place.
    let nav = json!({
        "type": "frame", "id": "nav", "name": "Left Sidebar", "layout": "vertical", "children": [
            { "type": "frame", "id": "navi", "name": "Navigation", "layout": "vertical", "children": [
                { "type": "frame", "id": "d", "name": "Nav Dashboard", "layout": "horizontal", "children": [
                    { "type": "icon_font", "id": "d-i", "iconFontName": "home" },
                    { "type": "text", "id": "d-l", "content": "Dashboard" },
                    { "type": "text", "id": "d-b", "content": "3" },
                    { "type": "icon_font", "id": "d-c", "iconFontName": "chevron-right" }
                ]},
                { "type": "frame", "id": "c", "name": "Nav Clients", "layout": "horizontal", "children": [
                    { "type": "icon_font", "id": "c-i", "iconFontName": "users" },
                    { "type": "text", "id": "c-l", "content": "Clients" },
                    { "type": "text", "id": "c-b", "content": "12" },
                    { "type": "icon_font", "id": "c-c", "iconFontName": "chevron-right" }
                ]},
                { "type": "frame", "id": "a", "name": "Nav Analytics", "layout": "horizontal", "children": [
                    { "type": "icon_font", "id": "a-i", "iconFontName": "chart" },
                    { "type": "text", "id": "a-l", "content": "Analytics" },
                    { "type": "text", "id": "a-b", "content": "" },
                    { "type": "icon_font", "id": "a-c", "iconFontName": "chevron-right" }
                ]}
            ]}
        ]
    });
    let mut root = shell(vec![nav]);
    assert!(
        !evict_content_from_sidebar_column(&mut root),
        "multi-child nav items are NOT a data table — the nav stays in the rail"
    );
    let v = val(&root);
    assert_eq!(
        v["children"][0]["children"].as_array().unwrap().len(),
        1,
        "the nav is still the sidebar's child"
    );
}

#[test]
fn nav_only_sidebar_is_left_alone() {
    // A rail holding ONLY navigation (with the "Analytics" item) must not be
    // touched — nothing to evict.
    let mut root = shell(vec![nav_with_analytics()]);
    assert!(
        !evict_content_from_sidebar_column(&mut root),
        "pure-nav sidebar is a no-op"
    );
}

#[test]
fn eviction_without_main_content_column_is_noop() {
    // No `Main Content` sibling → there is nowhere to relocate to; leave as-is
    // rather than dropping the section.
    let mut root = node(json!({
        "type": "frame", "id": "root", "layout": "horizontal", "width": 1200, "children": [
            { "type": "frame", "id": "sb", "name": "Sidebar", "layout": "vertical", "width": 260,
              "children": [
                { "type": "frame", "id": "tbl", "name": "Client Table", "layout": "vertical",
                  "children": [ table_row("r0", 5), table_row("r1", 5) ] }
              ]}
        ]
    }));
    assert!(
        !evict_content_from_sidebar_column(&mut root),
        "no content column → no-op (never drop)"
    );
}

#[test]
fn split_shell_without_row_layout_is_flipped() {
    // MiniMax-M3 in the agentic loop: the root already carries [Sidebar, Main]
    // but layout=None, so the two columns stack/overlap. Flip to a horizontal
    // row with definite column widths.
    let mut root = node(json!({
        "type": "frame", "id": "root", "name": "Dashboard", "layout": "vertical",
        "children": [
            {"type":"frame","id":"sb","name":"Sidebar","layout":"vertical","height":"fill_container","children":[]},
            {"type":"frame","id":"main","name":"Main","layout":"vertical","children":[]}
        ]
    }));
    assert!(
        ensure_split_shell_is_row(&mut root),
        "already-split flat shell must be flipped to a row"
    );
    let v = val(&root);
    assert_eq!(v["layout"], json!("horizontal"), "root → horizontal row");
    assert_eq!(
        v["children"][0]["width"].as_f64(),
        Some(260.0),
        "sidebar pinned to a fixed rail width"
    );
    assert_eq!(
        v["children"][1]["width"],
        json!("fill_container"),
        "main fills the rest of the row"
    );
}

#[test]
fn legit_two_section_vertical_page_not_flipped() {
    // A real 2-section vertical page (no sidebar-named first column) must NEVER
    // be turned sideways.
    let mut root = node(json!({
        "type": "frame", "id": "root", "name": "Landing", "layout": "vertical",
        "children": [
            {"type":"frame","id":"hero","name":"Hero","layout":"vertical","children":[]},
            {"type":"frame","id":"feat","name":"Features","layout":"vertical","children":[]}
        ]
    }));
    assert!(
        !ensure_split_shell_is_row(&mut root),
        "non-sidebar 2-section page must stay vertical"
    );
    assert_eq!(val(&root)["layout"], json!("vertical"));
}

#[test]
fn already_horizontal_shell_gets_fill_height_sidebar() {
    // A shell that is ALREADY a row but whose sidebar hugs its content
    // (height=fit_content) leaves the footer floating — the sidebar is only as
    // tall as its nav, so its `space_between` child has no room to sink. Promote
    // the sidebar to fill_container height even though the root is already a row.
    let mut root = node(json!({
        "type": "frame", "id": "root", "name": "Page", "layout": "horizontal",
        "width": 1200, "height": 800,
        "children": [
            {"type":"frame","id":"sb","name":"Sidebar","layout":"vertical","width":260,"height":"fit_content",
             "children":[{"type":"frame","id":"nav","name":"Nav","layout":"vertical",
                          "justifyContent":"space_between","height":"fill_container","children":[]}]},
            {"type":"frame","id":"main","name":"Main","layout":"vertical",
             "width":"fill_container","height":"fill_container","children":[]}
        ]
    }));
    assert!(
        ensure_split_shell_is_row(&mut root),
        "an already-row shell with a fit_content sidebar must still be corrected"
    );
    let v = val(&root);
    assert_eq!(v["layout"], json!("horizontal"), "root stays a row");
    assert_eq!(
        v["children"][0]["height"],
        json!("fill_container"),
        "sidebar height → fill_container so its footer can sink"
    );
}
