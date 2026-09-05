//! Resolved-rect coverage for the single-line text-fit cleanup repair.

use super::*;
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId};
use serde_json::{json, Value};

fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect { x, y, w, h }
}

fn update_font_size(cmds: &[EditorCommand], id: &str) -> Option<f32> {
    cmds.iter().find_map(|cmd| match cmd {
        EditorCommand::SetNodeFontSize { node_id, font_size } if node_id.as_str() == id => {
            Some(*font_size)
        }
        _ => None,
    })
}

fn collect(card: Value, rects: HashMap<String, Rect>) -> Vec<EditorCommand> {
    let mut cmds = Vec::new();
    collect_text_fit_fixes(&card, &rects, &mut cmds);
    cmds
}

fn amount_card(text: Value) -> (Value, HashMap<String, Rect>) {
    (
        json!({
            "type": "frame",
            "id": "card",
            "width": 327,
            "height": 100,
            "clipContent": true,
            "children": [text]
        }),
        HashMap::from([
            ("card".to_string(), rect(0.0, 0.0, 327.0, 100.0)),
            ("text".to_string(), rect(0.0, 0.0, 370.0, 52.0)),
        ]),
    )
}

#[test]
fn measured_52px_amount_shrinks_to_the_proportional_floor() {
    let (card, rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "$52,480.16",
        "fontSize": 52,
        "fontWeight": 600,
        "textGrowth": "auto"
    }));

    let cmds = collect(card, rects);
    assert_eq!(update_font_size(&cmds, "text"), Some(45.0));
}

#[test]
fn a_17px_label_that_fits_is_untouched() {
    let (card, mut rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "Paid",
        "fontSize": 17,
        "textGrowth": "auto"
    }));
    rects.insert("text".to_string(), rect(0.0, 0.0, 100.0, 20.0));

    assert!(collect(card, rects).is_empty());
}

#[test]
fn wrapped_paragraph_is_untouched() {
    let (card, rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "A paragraph that intentionally wraps.",
        "fontSize": 40,
        "textGrowth": "fixed-width"
    }));

    assert!(collect(card, rects).is_empty());
}

#[test]
fn text_inside_a_horizontal_scroller_is_untouched() {
    let card = json!({
        "type": "frame",
        "id": "viewport",
        "width": 327,
        "height": 100,
        "layout": "horizontal",
        "clipContent": true,
        "children": [{
            "type": "text",
            "id": "text",
            "content": "Scrollable amount",
            "fontSize": 52,
            "textGrowth": "auto"
        }]
    });
    let rects = HashMap::from([
        ("viewport".to_string(), rect(0.0, 0.0, 327.0, 100.0)),
        ("text".to_string(), rect(0.0, 0.0, 370.0, 52.0)),
    ]);

    assert!(collect(card, rects).is_empty());
}

#[test]
fn text_that_cannot_fit_at_24px_is_untouched() {
    let (card, rects) = amount_card(json!({
        "type": "text",
        "id": "text",
        "content": "An exceptionally long amount",
        "fontSize": 52,
        "textGrowth": "auto"
    }));
    let mut rects = rects;
    rects.insert("text".to_string(), rect(0.0, 0.0, 800.0, 52.0));

    assert!(collect(card, rects).is_empty());
}

#[test]
fn cleanup_records_the_repair_under_text_fit() {
    let tree = json!({
        "type": "frame",
        "id": "root",
        "width": 327,
        "height": 100,
        "clipContent": true,
        "children": [{
            "type": "text",
            "id": "text",
            "content": "$52,480.16",
            "width": 370,
            "height": 52,
            "fontSize": 52,
            "textGrowth": "auto"
        }]
    });
    let node: PenNode = serde_json::from_value(tree).expect("fixture parses");
    let mut sink = VecDocSink::new();
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![node],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    let mut counter = RepairCounter::new();
    let mut counting = counter.wrap(&mut sink);
    assert_eq!(repair_text_fit(&mut counting, "root"), 1);
    let mut summary = RepairSummary::default();
    counter.checkpoint(&mut summary, CheckCategory::Overflow, "text-fit");

    let text = serde_json::to_value(&counting.state().active_children()[0]).expect("serialize");
    let font_size = text["children"][0]["fontSize"].as_f64();
    assert_eq!(font_size, Some(45.0));
    assert_eq!(summary.repairs_for(CheckCategory::Overflow), 1);
    assert_eq!(summary.records()[0].pass, "text-fit");
}

#[test]
fn status_bar_subtree_is_untouched() {
    let card = json!({
        "type": "frame",
        "id": "status-bar",
        "role": "status-bar",
        "width": 327,
        "height": 100,
        "children": [{
            "type": "text",
            "id": "text",
            "content": "09:41",
            "fontSize": 52,
            "textGrowth": "auto"
        }]
    });
    let rects = HashMap::from([
        ("status-bar".to_string(), rect(0.0, 0.0, 327.0, 100.0)),
        ("text".to_string(), rect(0.0, 0.0, 370.0, 52.0)),
    ]);

    assert!(collect(card, rects).is_empty());
}
