use crate::cleanup::run_cleanup_passes;
use crate::geometry_validation::geometry_diagnostics;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};

fn plan() -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Dashboard".into(),
            width: 1200.0,
            height: 800.0,
            layout: Some("horizontal".into()),
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    }
}

fn node(v: Value) -> PenNode {
    serde_json::from_value::<PenNode>(v).expect("valid PenNode fixture")
}

fn node_from_str(s: &str) -> PenNode {
    serde_json::from_str::<PenNode>(s).expect("valid PenNode fixture")
}

fn insert_root(root: PenNode) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink
}

fn run_cleanup(sink: &mut VecDocSink) {
    let root_id = sink.state.active_children()[0].id_str().to_string();
    run_cleanup_passes(sink, &plan(), &[&root_id]);
}

fn active_root_value(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialize root")
}

fn find_by_name<'a>(v: &'a Value, name: &str) -> &'a Value {
    if v.get("name").and_then(Value::as_str) == Some(name) {
        return v;
    }
    for child in v
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(hit) = find_by_name_opt(child, name) {
            return hit;
        }
    }
    panic!("missing node named {name}");
}

fn find_by_name_opt<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    if v.get("name").and_then(Value::as_str) == Some(name) {
        return Some(v);
    }
    v.get("children")
        .and_then(Value::as_array)?
        .iter()
        .find_map(|child| find_by_name_opt(child, name))
}

fn layout(v: &Value) -> Option<&str> {
    v.get("layout").and_then(Value::as_str)
}

fn keyword<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

fn number(v: &Value, key: &str) -> Option<f64> {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(s)) => s.parse().ok(),
        _ => None,
    }
}

fn landing_navbar_in_sidebar() -> PenNode {
    node_from_str(
        r#"{
          "type": "frame", "id": "root", "name": "Barbershop Dashboard",
          "width": 1200, "height": 800, "layout": "horizontal", "children": [
            {
              "type": "frame", "id": "sidebar", "name": "Sidebar",
              "width": 260, "height": "fill_container", "layout": "vertical",
              "clipContent": true, "children": [
                {
                  "type": "frame", "id": "topnav", "name": "Top Navigation Bar",
                  "width": "fill_container", "height": "fit_content",
                  "layout": "vertical", "children": [
                    {
                      "type": "frame", "id": "surface", "name": "Nav Surface",
                      "width": "fill_container", "height": "fit_content",
                      "layout": "vertical", "padding": [40, 60, 0, 60],
                      "children": [
                        {
                          "type": "frame", "id": "row", "name": "Nav Row",
                          "width": "fill_container", "height": "80",
                          "layout": "horizontal", "gap": 16,
                          "justifyContent": "space_between",
                          "alignItems": "center", "children": [
                            {
                              "type": "frame", "id": "brand", "name": "Brand",
                              "width": "fill_container", "height": "fit_content",
                              "layout": "vertical", "children": [
                                {
                                  "type": "text", "id": "brand-name",
                                  "name": "Brand Name", "width": "fill_container",
                                  "content": "MAISON LUXE", "fontSize": 24
                                }
                              ]
                            },
                            {
                              "type": "frame", "id": "links", "name": "Nav Links",
                              "role": "nav-link", "width": "fill_container",
                              "height": "fit_content", "layout": "horizontal",
                              "gap": 48, "children": [
                                {"type": "text", "id": "dash", "name": "Dashboard Link", "width": "fill_container", "content": "DASHBOARD", "fontSize": 13},
                                {"type": "text", "id": "clients", "name": "Clients Link", "width": "fill_container", "content": "CLIENTS", "fontSize": 13},
                                {"type": "text", "id": "appt", "name": "Appointments Link", "width": "fill_container", "content": "APPOINTMENTS", "fontSize": 13},
                                {"type": "text", "id": "inventory", "name": "Inventory Link", "width": "fill_container", "content": "INVENTORY", "fontSize": 13}
                              ]
                            },
                            {
                              "type": "frame", "id": "actions", "name": "Actions",
                              "width": "fill_container", "height": "fit_content",
                              "layout": "horizontal", "gap": 24, "children": [
                                {"type": "frame", "id": "bell", "name": "Bell Wrap", "width": "40", "height": "40", "children": []},
                                {"type": "frame", "id": "avatar", "name": "Admin Avatar", "width": "40", "height": "40", "children": []}
                              ]
                            }
                          ]
                        },
                        {
                          "type": "frame", "id": "hero-pad", "name": "Hero Padding",
                          "width": "fill_container", "height": "336",
                          "layout": "vertical", "children": [
                            {
                              "type": "text", "id": "headline", "name": "Headline",
                              "width": "fill_container", "content": "The Art of Grooming",
                              "fontSize": 64, "textGrowth": "fixed-width"
                            }
                          ]
                        }
                      ]
                    }
                  ]
                }
              ]
            },
            {
              "type": "frame", "id": "main", "name": "Main Content",
              "width": "fill_container", "height": "fill_container",
              "layout": "vertical", "children": []
            }
          ]
        }"#,
    )
}

#[test]
fn restacks_landing_navbar_archetype_inside_narrow_sidebar() {
    let mut sink = insert_root(landing_navbar_in_sidebar());
    let diagnostics = geometry_diagnostics(&sink.state);
    assert!(
        diagnostics
            .iter()
            .any(|line| line.contains("sidebar contains a horizontal navbar archetype")),
        "expected sidebar archetype diagnostic, got {diagnostics:?}"
    );

    run_cleanup(&mut sink);
    let root = active_root_value(&sink);
    let nav_row = find_by_name(&root, "Nav Row");
    assert_eq!(layout(nav_row), Some("vertical"));
    assert_eq!(number(nav_row, "gap"), Some(8.0));
    assert_eq!(keyword(nav_row, "justifyContent"), Some("start"));

    let nav_links = find_by_name(&root, "Nav Links");
    assert_eq!(layout(nav_links), Some("vertical"));
    assert_eq!(number(nav_links, "gap"), Some(8.0));
    for label in [
        "Dashboard Link",
        "Clients Link",
        "Appointments Link",
        "Inventory Link",
    ] {
        assert_eq!(
            keyword(find_by_name(nav_links, label), "width"),
            Some("fill_container"),
            "{label} should fill the vertical sidebar rail"
        );
    }

    assert_eq!(
        keyword(find_by_name(&root, "Actions"), "width"),
        Some("fit_content")
    );
    assert_eq!(
        number(find_by_name(&root, "Headline"), "fontSize"),
        Some(28.0)
    );
    assert_eq!(
        keyword(find_by_name(&root, "Hero Padding"), "height"),
        Some("fit_content")
    );
    assert_eq!(
        find_by_name(&root, "Nav Surface").get("padding"),
        Some(&json!([40.0, 20.0, 0.0, 20.0]))
    );
}

#[test]
fn genuine_vertical_sidebar_is_untouched() {
    let mut sink = insert_root(node(json!({
        "type": "frame", "id": "root", "name": "Dashboard",
        "width": 1200, "height": 800, "layout": "horizontal", "children": [
            {
                "type": "frame", "id": "sidebar", "name": "Sidebar",
                "width": 260, "height": "fill_container", "layout": "vertical",
                "children": [
                    {"type": "frame", "id": "brand", "name": "Brand Block", "width": "fill_container", "height": "fit_content", "layout": "vertical", "children": []},
                    {"type": "frame", "id": "nav", "name": "Sidebar Nav", "width": "fill_container", "height": "fit_content", "layout": "vertical", "gap": 12, "children": [
                        {"type": "text", "id": "home", "name": "Home", "width": "fill_container", "content": "Home", "fontSize": 14},
                        {"type": "text", "id": "clients", "name": "Clients", "width": "fill_container", "content": "Clients", "fontSize": 14}
                    ]},
                    {"type": "frame", "id": "profile", "name": "Footer Profile", "width": "fill_container", "height": "fit_content", "layout": "horizontal", "children": []}
                ]
            },
            {"type": "frame", "id": "main", "name": "Main Content", "width": "fill_container", "height": "fill_container", "layout": "vertical", "children": []}
        ]
    })));
    run_cleanup(&mut sink);
    let root = active_root_value(&sink);
    assert_eq!(layout(find_by_name(&root, "Sidebar Nav")), Some("vertical"));
    assert_eq!(
        number(find_by_name(&root, "Sidebar Nav"), "gap"),
        Some(12.0)
    );
}

#[test]
fn wide_top_navbar_is_untouched() {
    let mut sink = insert_root(node(json!({
        "type": "frame", "id": "root", "name": "Landing Page",
        "width": 1200, "height": 800, "layout": "vertical", "children": [
            {
                "type": "frame", "id": "topnav", "name": "Top Navigation Bar",
                "width": 1200, "height": 80, "layout": "horizontal",
                "gap": 48, "justifyContent": "space_between", "children": [
                    {"type": "text", "id": "brand", "name": "Brand", "content": "MAISON", "fontSize": 24},
                    {"type": "text", "id": "about", "name": "About Link", "content": "About", "fontSize": 14},
                    {"type": "text", "id": "work", "name": "Work Link", "content": "Work", "fontSize": 14},
                    {"type": "text", "id": "contact", "name": "Contact Link", "content": "Contact", "fontSize": 14}
                ]
            },
            {"type": "frame", "id": "hero", "name": "Hero", "width": "fill_container", "height": 600, "layout": "vertical", "children": []}
        ]
    })));
    run_cleanup(&mut sink);
    let root = active_root_value(&sink);
    let topnav = find_by_name(&root, "Top Navigation Bar");
    assert_eq!(layout(topnav), Some("horizontal"));
    assert_eq!(number(topnav, "gap"), Some(48.0));
    assert_eq!(keyword(topnav, "justifyContent"), Some("space_between"));
}

#[test]
fn narrow_card_with_two_chips_is_untouched() {
    let mut sink = insert_root(node(json!({
        "type": "frame", "id": "root", "name": "Dashboard",
        "width": 1200, "height": 800, "layout": "horizontal", "children": [
            {
                "type": "frame", "id": "card", "name": "Filter Card",
                "width": 260, "height": "fit_content", "layout": "vertical",
                "children": [
                    {
                        "type": "frame", "id": "chips", "name": "Chip Row",
                        "width": 120, "height": "fit_content", "layout": "horizontal",
                        "gap": 8, "children": [
                            {"type": "text", "id": "new", "name": "New Chip", "content": "New", "fontSize": 12},
                            {"type": "text", "id": "vip", "name": "VIP Chip", "content": "VIP", "fontSize": 12}
                        ]
                    }
                ]
            },
            {"type": "frame", "id": "main", "name": "Main Content", "width": "fill_container", "height": "fill_container", "layout": "vertical", "children": []}
        ]
    })));
    run_cleanup(&mut sink);
    let root = active_root_value(&sink);
    let chips = find_by_name(&root, "Chip Row");
    assert_eq!(layout(chips), Some("horizontal"));
    assert_eq!(number(chips, "gap"), Some(8.0));
}
