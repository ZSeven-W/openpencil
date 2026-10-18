//! Narrow rail promotion contracts, independent of planner phrase matching.
use super::*;
use crate::test_support::VecDocSink;

fn fixture(gutter: i32, body_height: i32) -> Value {
    json!({"version":"1.0.0","children":[{
        "type":"frame","id":"root","width":375,"height":812,
        "layout":"vertical","children":[
            {"type":"frame","id":"status","name":"Status Bar","width":"fill_container","height":62},
            {"type":"frame","id":"shell","width":"fill_container","height":"fit_content",
             "layout":"vertical","padding":[0,gutter,0,gutter],"children":[
                {"type":"frame","id":"body","width":"fill_container","height":body_height},
                {"type":"frame","id":"bar","role":"bottom-action-bar","width":"fill_container",
                 "height":"fit_content","layout":"horizontal","padding":[8,12,28,12],"children":[
                    {"type":"frame","id":"buy","role":"button","width":120,"height":44,
                     "events":{"onTap":[{"toast":"Checkout"}]},"children":[
                        {"type":"text","id":"label","content":"Checkout","fontSize":16,"height":24}
                    ]}
                ]}
            ]}
        ]
    }]})
}

fn sink(value: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state = EditorState::from_document(serde_json::from_value(value).unwrap());
    sink
}

fn node<'a>(sink: &'a VecDocSink, id: &str) -> &'a PenNode {
    op_editor_core::walkers::find_node(sink.state.active_children(), &NodeId::new(id)).unwrap()
}

#[test]
fn short_and_overflowing_rails_preserve_gutters_original_bar_and_replay() {
    for gutter in [0, 24] {
        for body_height in [220, 1000] {
            let mut sink = sink(fixture(gutter, body_height));
            let original = node(&sink, "bar").clone();
            assert!(super::super::repair(&mut sink, "root"));
            let scene = op_pen_loader::editor_state_to_active_page_layout_scene(&sink.state);
            let page = scene.active_page().unwrap();
            let bar = page.find("bar").unwrap().bounds;
            assert!((bar.origin.x - gutter as f32).abs() < 1.0);
            assert!((bar.size.x - (375 - 2 * gutter) as f32).abs() < 1.0);
            assert!((bar.origin.y + bar.size.y - 812.0).abs() < 1.0);
            assert_eq!(node(&sink, "bar"), &original);
            let children = node(&sink, "root").children().unwrap();
            assert_eq!(children.len(), 3);
            assert_eq!(children[1].base().role.as_deref(), Some("scroll-area"));
            let before = sink.state.doc.clone();
            assert!(!super::super::repair(&mut sink, "root"));
            assert_eq!(sink.state.doc, before);
        }
    }
}

#[test]
fn rail_promotion_rejects_unsafe_or_ambiguous_shells_without_mutation() {
    for case in [
        "card",
        "position",
        "token-padding",
        "asymmetric",
        "top-padding",
        "overlay",
        "fixed-height",
        "no-action",
        "ordinary-action",
        "multiple-bars",
    ] {
        let mut value = fixture(24, 220);
        let shell = &mut value["children"][0]["children"][1];
        match case {
            "card" => shell["role"] = json!("card"),
            "position" => shell["x"] = json!(0),
            "token-padding" => shell["padding"] = json!("$page-padding"),
            "asymmetric" => shell["padding"] = json!([0, 24, 0, 12]),
            "top-padding" => shell["padding"] = json!([12, 24, 0, 24]),
            "overlay" => shell["role"] = json!("overlay"),
            "fixed-height" => shell["height"] = json!(700),
            "no-action" => shell["children"][1]["children"] = json!([]),
            "ordinary-action" => {
                shell["children"][1].as_object_mut().unwrap().remove("role");
                shell["children"][1]["name"] = json!("Read more");
            }
            "multiple-bars" => {
                let mut another = shell["children"][1].clone();
                another["id"] = json!("another-bar");
                another["children"][0]["id"] = json!("another-buy");
                another["children"][0]["children"][0]["id"] = json!("another-label");
                shell["children"].as_array_mut().unwrap().insert(1, another);
            }
            _ => unreachable!(),
        }
        let mut sink = sink(value);
        let before = sink.state.doc.clone();
        assert!(!promote_nested_chrome(&mut sink, "root"), "{case}");
        assert_eq!(sink.state.doc, before, "{case}");
    }
}

#[test]
fn saved_scroll_closing_inset_stays_in_viewport_and_is_not_duplicated_on_rail() {
    let mut value = fixture(24, 1000);
    let shell = value["children"][0]["children"][1].take();
    value["children"][0]["children"][1] = json!({
        "type":"frame","id":"viewport","role":"scroll-area","layout":"vertical",
        "width":"fill_container","height":"fill_container","clipContent":true,
        "padding":[0,0,28,0],"events":{"onScroll":[{"set":{"$state.scrolled":"true"}}]},
        "children":[shell]
    });
    let mut sink = sink(value);
    let original = node(&sink, "bar").clone();
    assert!(super::super::repair(&mut sink, "root"));
    let viewport = serde_json::to_value(node(&sink, "viewport")).unwrap();
    assert_eq!(viewport["padding"], json!([0.0, 0.0, 28.0, 0.0]));
    let root = node(&sink, "root");
    assert_eq!(root.children().unwrap().len(), 3);
    let rail = serde_json::to_value(root.children().unwrap().last().unwrap()).unwrap();
    assert_eq!(rail["padding"], json!([0.0, 24.0, 0.0, 24.0]));
    assert_eq!(node(&sink, "bar"), &original);
    let before = sink.state.doc.clone();
    assert!(!super::super::repair(&mut sink, "root"));
    assert_eq!(sink.state.doc, before);
}
