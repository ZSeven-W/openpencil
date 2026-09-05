//! Tests for the resolved mobile touch-target floor repair.

use super::*;
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::{json, Value};

fn rect(w: f64, h: f64) -> Rect {
    Rect {
        x: 0.0,
        y: 0.0,
        w,
        h,
    }
}

fn painted_button(id: &str, height: Value, child: Value) -> Value {
    json!({
        "type": "frame",
        "id": id,
        "width": "fill_container",
        "height": height,
        "layout": "horizontal",
        "fill": [{"type": "solid", "color": "$--primary"}],
        "children": [child]
    })
}

fn label(id: &str) -> Value {
    json!({"type": "text", "id": id, "fontSize": 17, "content": "登录"})
}

fn collect(node: Value, rects: HashMap<String, Rect>) -> Vec<EditorCommand> {
    let mut cmds = Vec::new();
    collect_touch_target_floor_fixes(&node, &rects, &HashSet::new(), &mut cmds);
    cmds
}

fn sink_with(root: Value) -> VecDocSink {
    let node: PenNode = serde_json::from_value(root).expect("fixture parses");
    let mut sink = VecDocSink::new();
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink
}

fn command_height(commands: &[EditorCommand], id: &str) -> Option<i32> {
    commands.iter().find_map(|command| match command {
        EditorCommand::UpdateNode {
            node_id,
            height: Some(height),
            ..
        } if node_id.as_str() == id => Some(*height),
        _ => None,
    })
}

#[test]
fn login_button_gets_the_48px_floor_and_layout_checkpoint() {
    let root = json!({
        "type": "frame", "id": "root", "width": 390, "height": 844,
        "layout": "vertical", "children": [
            painted_button("login", json!("fit_content"), label("login-label"))
        ]
    });
    let mut sink = sink_with(root);
    let root_id = sink.state.active_children()[0].id_str().to_string();
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(&mut sink);
    assert_eq!(repair_touch_target_floor(&mut counting, &root_id), 2);
    drop(counting);

    let mut summary = RepairSummary::default();
    counter.checkpoint(&mut summary, CheckCategory::Layout, "touch-target-floor");
    let root = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize root");
    assert_eq!(root["children"][0]["height"].as_f64(), Some(48.0));
    assert_eq!(root["children"][0]["alignItems"], json!("center"));
    assert_eq!(summary.repairs_for(CheckCategory::Layout), 2);
    assert!(summary
        .records()
        .iter()
        .all(|record| record.pass == "touch-target-floor"));
}

#[test]
fn a_36px_chip_88px_wide_is_untouched() {
    let chip = painted_button("chip", json!("fit_content"), label("chip-label"));
    let rects = HashMap::from([("chip".to_string(), rect(88.0, 36.0))]);
    assert!(collect(chip, rects).is_empty());
}

#[test]
fn a_52px_button_is_untouched() {
    let button = painted_button("button", json!("fit_content"), label("button-label"));
    let rects = HashMap::from([("button".to_string(), rect(240.0, 52.0))]);
    assert!(collect(button, rects).is_empty());
}

#[test]
fn a_card_with_a_nested_frame_child_is_untouched() {
    let card = painted_button(
        "card",
        json!("fit_content"),
        json!({"type": "frame", "id": "nested", "width": 80, "height": 24}),
    );
    let rects = HashMap::from([("card".to_string(), rect(240.0, 30.0))]);
    assert!(collect(card, rects).is_empty());
}

#[test]
fn a_filled_row_inside_layout_none_is_untouched() {
    let row = painted_button("row", json!("fit_content"), label("row-label"));
    let parent = json!({
        "type": "frame", "id": "overlay", "width": 300, "height": 100,
        "layout": "none", "children": [row]
    });
    let rects = HashMap::from([
        ("overlay".to_string(), rect(300.0, 100.0)),
        ("row".to_string(), rect(240.0, 30.0)),
    ]);
    assert!(collect(parent, rects).is_empty());
}

#[test]
fn a_page_form_root_does_not_run_the_pass() {
    let root = json!({
        "type": "frame", "id": "page", "width": 1200, "height": 900,
        "layout": "vertical", "children": [
            painted_button("button", json!("fit_content"), label("page-label"))
        ]
    });
    let mut sink = sink_with(root);
    let before = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize root");
    let root_id = sink.state.active_children()[0].id_str().to_string();
    assert_eq!(repair_touch_target_floor(&mut sink, &root_id), 0);
    let after = serde_json::to_value(&sink.state.active_children()[0]).expect("serialize root");
    assert_eq!(after, before);
    assert_eq!(command_height(&sink.applied, "button"), None);
}

#[test]
fn transparent_paint_and_status_bar_are_untouched() {
    let mut button = painted_button("button", json!("fit_content"), label("button-label"));
    button["fill"][0]["color"] = json!("#00000000");
    let status_button =
        painted_button("status-button", json!("fit_content"), label("status-label"));
    let status = json!({
        "type": "frame", "id": "status-bar", "role": "status-bar",
        "layout": "none", "children": [status_button]
    });
    let root = json!({
        "type": "frame", "id": "root", "width": 390, "height": 844,
        "layout": "vertical", "children": [status, button]
    });
    let mut sink = sink_with(root);
    let root_id = sink.state.active_children()[0].id_str().to_string();
    assert_eq!(repair_touch_target_floor(&mut sink, &root_id), 0);
}
