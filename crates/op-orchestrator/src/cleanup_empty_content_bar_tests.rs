use super::*;
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::test_support::VecDocSink;
use serde_json::{json, Value};

fn sink_with(root: Value) -> VecDocSink {
    let mut sink = VecDocSink::new();
    sink.state.doc.variables = Some(
        [(
            "--accent".to_string(),
            serde_json::from_value(json!({
                "type": "color",
                "value": [{"value": "#F6D7A7", "theme": {"Mode": "Light"}}]
            }))
            .expect("accent variable"),
        )]
        .into_iter()
        .collect(),
    );
    sink.state.doc.themes = Some(
        [("Mode".to_string(), vec!["Light".to_string()])]
            .into_iter()
            .collect(),
    );
    sink.state.doc.children = vec![serde_json::from_value(root).expect("root")];
    sink
}

fn app07_shaped_root() -> Value {
    json!({
        "type": "frame", "id": "root", "name": "Ride Home", "width": 390,
        "height": 844, "layout": "vertical", "children": [{
            "type": "frame", "id": "content", "name": "Main Content", "width": "fill_container",
            "layout": "vertical", "children": [
                {"type": "frame", "id": "vehicle", "width": "fill_container", "height": 80,
                 "children": [{"type": "text", "id": "vehicle-label", "content": "Vehicle"}]},
                {"type": "frame", "id": "coupon-bar", "name": "coupon-bar", "width": "fill_container",
                 "height": 56, "fill": [{"type": "solid", "color": "$--accent"}], "children": []},
                {"type": "rectangle", "id": "divider", "name": "divider", "width": "fill_container",
                 "height": 6, "fill": [{"type": "solid", "color": "#F6D7A7"}], "children": []},
                {"type": "frame", "id": "progress", "name": "progress track", "width": "fill_container",
                 "height": 56, "fill": [{"type": "solid", "color": "#F6D7A7"}], "children": []},
                {"type": "frame", "id": "with-text", "name": "Accent Bar", "width": "fill_container",
                 "height": 56, "fill": [{"type": "solid", "color": "#F6D7A7"}],
                 "children": [{"type": "text", "id": "bar-label", "content": "Offer"}]},
                {"type": "frame", "id": "map-canvas", "name": "map-canvas", "layout": "vertical",
                 "width": "fill_container", "height": 180, "children": [
                    {"type": "rectangle", "id": "map-road", "name": "map-road", "width": "fill_container",
                     "height": 56, "fill": [{"type": "solid", "color": "#F6D7A7"}], "children": []},
                    {"type": "frame", "id": "map-a", "height": 20},
                    {"type": "frame", "id": "map-b", "height": 20}
                ]}
            ]
        }]
    })
}

fn root_json(sink: &VecDocSink) -> Value {
    serde_json::to_value(&sink.state.active_children()[0]).expect("serialize root")
}

#[test]
fn app07_coupon_bar_is_removed_but_structural_bars_are_kept() {
    let mut sink = sink_with(app07_shaped_root());
    remove_empty_content_bars(&mut sink, "root");
    let content = &root_json(&sink)["children"][0];
    let ids: Vec<&str> = content["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|child| child["id"].as_str().unwrap())
        .collect();
    assert!(!ids.contains(&"coupon-bar"));
    assert!(ids.contains(&"divider"));
    assert!(ids.contains(&"progress"));
    assert!(ids.contains(&"with-text"));
    assert!(ids.contains(&"map-canvas"));
    assert_eq!(content["children"][4]["id"], "map-canvas");
}

#[test]
fn empty_content_bar_removal_is_itemized_with_its_node_name() {
    let mut sink = sink_with(app07_shaped_root());
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(&mut sink);
    remove_empty_content_bars(&mut counting, "root");
    let mut summary = RepairSummary::default();
    counter.checkpoint(&mut summary, CheckCategory::Structure, "empty-content-bar");

    assert_eq!(summary.total_repairs(), 1);
    assert_eq!(summary.records()[0].pass, "empty-content-bar");
    assert_eq!(
        summary.records()[0].node_name.as_deref(),
        Some("coupon-bar")
    );
}

#[test]
fn root_bar_is_not_considered_by_the_predicate() {
    let mut root = app07_shaped_root();
    root["fill"] = json!([{"type": "solid", "color": "#F6D7A7"}]);
    root["height"] = json!(56);
    let mut sink = sink_with(root);
    remove_empty_content_bars(&mut sink, "root");
    assert_eq!(root_json(&sink)["id"], "root");
}
