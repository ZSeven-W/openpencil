//! arena-w02 (Opus 5.5, 2026-10-01): the reply box sat below the clipped
//! reading pane's bottom edge and was invisible in the 1440×900 shot.

use super::*;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::PenNodeExt;
use serde_json::json;

fn with_ids(v: &mut Value, next: &mut usize) {
    if let Some(obj) = v.as_object_mut() {
        if !obj.contains_key("id") {
            obj.insert("id".into(), json!(format!("n{next}")));
        }
        *next += 1;
        if let Some(children) = obj.get_mut("children").and_then(|c| c.as_array_mut()) {
            for child in children {
                with_ids(child, next);
            }
        }
    }
}

fn block(id: &str, height: f64) -> Value {
    json!({"type": "frame", "id": id, "name": id, "width": "fill_container",
           "height": height, "layout": "none",
           "fill": [{"type": "solid", "color": "#EEEEEE"}]})
}

/// The w02 shape: three columns in a fixed 900px root; the reading pane
/// holds a header, a long body and the reply box last.
fn mail_client(composer_name: &str) -> Value {
    json!({
        "type": "frame", "id": "root", "name": "Email Client",
        "width": 1440, "height": 900, "layout": "horizontal",
        "children": [
            {"type": "frame", "id": "sidebar", "name": "Sidebar", "width": 260,
             "height": "fill_container", "layout": "vertical", "clipContent": true,
             "children": [block("folders", 400.0)]},
            {"type": "frame", "id": "main", "name": "Main Content",
             "width": "fill_container", "height": "fill_container", "layout": "vertical",
             "children": [block("list", 700.0)]},
            {"type": "frame", "id": "rail", "name": "Right Panel", "width": 480,
             "height": "fill_container", "layout": "vertical", "clipContent": true,
             "padding": [32, 24],
             "children": [{
                "type": "frame", "id": "reading", "name": "Reading Pane",
                "width": "fill_container", "height": "fit_content", "layout": "vertical",
                "children": [
                    block("message-header", 160.0),
                    {"type": "frame", "id": "content", "name": "message-content",
                     "width": "fill_container", "height": "fit_content",
                     "layout": "vertical", "gap": 16,
                     "children": [
                        block("message-body", 700.0),
                        {"type": "frame", "id": "composer", "name": composer_name,
                         "width": "fill_container", "height": 120, "layout": "vertical",
                         "children": [block("reply-textarea", 80.0)]}
                     ]}
                ]
             }]}
        ]
    })
}

fn run(mut root: Value) -> (VecDocSink, String) {
    with_ids(&mut root, &mut 0);
    let root: PenNode = serde_json::from_value(root).expect("valid root");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![root],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let root_id = sink.state().active_children()[0].id_str().to_string();
    geometry_validate_and_fix(&mut sink, &root_id);
    (sink, root_id)
}

fn bottom(rects: &HashMap<String, Rect>, id: &str) -> f64 {
    let r = rects.get(id).unwrap_or_else(|| panic!("rect for {id}"));
    r.y + r.h
}

#[test]
fn a_reply_box_cut_by_the_pane_edge_is_pinned_inside_it() {
    let (sink, _) = run(mail_client("reply-box"));
    let rects = resolved_rects(sink.state());
    let root = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    let id_of = |name: &str| {
        find(&root, name).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string()
    };

    let pane_bottom = bottom(&rects, &id_of("Right Panel"));
    let composer_bottom = bottom(&rects, &id_of("reply-box"));
    assert!(
        composer_bottom <= pane_bottom + CUT_EPS,
        "reply box ends at {composer_bottom} below the pane's {pane_bottom}"
    );
    let body = find(&root, "message-body").expect("body");
    assert_eq!(body["height"], "fill_container");
    assert_eq!(body["clipContent"], true);
}

#[test]
fn a_composer_that_already_fits_is_left_alone() {
    let mut doc = mail_client("reply-box");
    doc["children"][2]["children"][0]["children"][1]["children"][0]["height"] = json!(200);
    let (sink, _) = run(doc);
    let root = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    assert_eq!(find(&root, "message-body").unwrap()["height"], 200.0);
    assert_eq!(
        find(&root, "Reading Pane").unwrap()["height"],
        "fit_content"
    );
}

#[test]
fn a_reply_button_is_not_a_composer() {
    let (sink, _) = run(mail_client("reply-button"));
    let root = serde_json::to_value(sink.state().active_children()[0].clone()).unwrap();
    assert_eq!(
        find(&root, "Reading Pane").unwrap()["height"],
        "fit_content"
    );
}

fn find<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    if v.get("name").and_then(|x| x.as_str()) == Some(name) {
        return Some(v);
    }
    v.get("children")
        .and_then(|c| c.as_array())
        .into_iter()
        .flatten()
        .find_map(|c| find(c, name))
}
