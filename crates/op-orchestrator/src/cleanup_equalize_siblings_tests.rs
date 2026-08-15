//! Tests for `cleanup_equalize_siblings::equalize_sibling_items` (DS P1-a):
//! a family of >= 3 sibling frames whose styling drifted gets aligned to the
//! majority norm; everything that is NOT provably a drifted family is left
//! alone.

use super::*;
use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairSummary};
use crate::test_support::VecDocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::json;

fn insert_tree(sink: &mut VecDocSink, json: &str) {
    let tree: PenNode = serde_json::from_str(json).expect("test tree json");
    sink.state.apply(EditorCommand::InsertAuthoredSubtree {
        nodes: vec![tree],
        parent_id: NodeId::NONE,
        page_id: None,
    });
    sink.applied.clear();
}

fn find_node<'a>(node: &'a PenNode, id: &str) -> Option<&'a PenNode> {
    if node.id_str() == id {
        return Some(node);
    }
    node.children()?
        .iter()
        .find_map(|child| find_node(child, id))
}

fn node_json(sink: &VecDocSink, id: &str) -> serde_json::Value {
    let node = sink
        .state
        .active_children()
        .iter()
        .find_map(|root| find_node(root, id))
        .unwrap_or_else(|| panic!("node `{id}` exists"));
    serde_json::to_value(node).expect("serialize")
}

fn run_pass(sink: &mut VecDocSink, root_id: &str) -> usize {
    equalize_sibling_items(sink, root_id)
}

fn run_driver(sink: &mut VecDocSink, root_id: &str) -> RepairSummary {
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: root_id.to_string(),
            name: "Knowledge Card".into(),
            width: 1200.0,
            height: 800.0,
            layout: None,
            gap: None,
            padding: None,
            fill: None,
        },
        subtasks: vec![],
        style_guide_name: None,
    };
    let mut summary = RepairSummary::default();
    run_cleanup_passes_with_summary(sink, &plan, &[root_id], &mut summary);
    summary
}

fn item(
    id: &str,
    name: &str,
    padding: f64,
    align: &str,
    title_size: f64,
    title_weight: u32,
) -> serde_json::Value {
    json!({
        "type": "frame",
        "id": id,
        "name": name,
        "layout": "vertical",
        "padding": padding,
        "gap": 12,
        "alignItems": align,
        "children": [
            {
                "type": "text",
                "id": format!("{id}-title"),
                "content": "Title",
                "fontSize": title_size,
                "fontWeight": title_weight
            },
            {
                "type": "text",
                "id": format!("{id}-body"),
                "content": "Body copy",
                "fontSize": 14
            }
        ]
    })
}

/// The measured 0814 shape: five knowledge-card entries, item 01 drifted from
/// 02-05 on padding / alignItems / title size and weight.
fn drifted_five_cards() -> serde_json::Value {
    json!({
        "type": "frame",
        "id": "root",
        "name": "Knowledge Card",
        "width": 1200,
        "height": 800,
        "layout": "vertical",
        "children": [
            item("c1", "Card 01", 24.0, "center", 18.0, 600),
            item("c2", "Card 02", 20.0, "start", 16.0, 700),
            item("c3", "Card 03", 20.0, "start", 16.0, 700),
            item("c4", "Card 04", 20.0, "start", 16.0, 700),
            item("c5", "Card 05", 20.0, "start", 16.0, 700)
        ]
    })
}

fn insert_drifted(sink: &mut VecDocSink) {
    insert_tree(sink, &drifted_five_cards().to_string());
}

#[test]
fn a_drifted_member_is_aligned_to_the_majority_norm() {
    let mut sink = VecDocSink::new();
    insert_drifted(&mut sink);

    let applied = run_pass(&mut sink, "root");
    assert_eq!(
        applied, 2,
        "one container patch (padding+alignItems) and one title patch (size+weight): {applied}"
    );

    let card = node_json(&sink, "c1");
    assert_eq!(card["padding"], json!(20.0), "padding must align: {card}");
    assert_eq!(card["alignItems"], json!("start"), "alignment must align");
    let title = node_json(&sink, "c1-title");
    assert_eq!(title["fontSize"], json!(16.0));
    assert_eq!(title["fontWeight"], json!(700));
    // Majority members are untouched.
    assert_eq!(node_json(&sink, "c2")["padding"], json!(20.0));
}

#[test]
fn a_per_edge_padding_drift_is_aligned_edge_by_edge() {
    // The "缩进逐条漂移" half of the defect: per-edge drifts, not whole-side.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "R", "width": 1200, "height": 800,
            "layout": "vertical",
            "children": [
                item("c1", "Card 01", 20.0, "start", 16.0, 700),
                item("c2", "Card 02", 20.0, "start", 16.0, 700),
                { "type": "frame", "id": "c3", "name": "Card 03", "layout": "vertical",
                  "gap": 12, "alignItems": "start",
                  "padding": [20, 32, 20, 20],
                  "children": [
                      { "type": "text", "id": "c3-title", "content": "T", "fontSize": 16 },
                      { "type": "text", "id": "c3-body", "content": "B", "fontSize": 14 }
                  ] }
            ]
        })
        .to_string(),
    );

    let applied = run_pass(&mut sink, "root");
    assert_eq!(
        applied, 2,
        "the right-edge outlier plus the missing fontWeight joining the 700 majority: {applied}"
    );
    assert_eq!(
        node_json(&sink, "c3")["padding"],
        json!(20.0),
        "the drifted right edge must join the majority while the rest stay"
    );
    assert_eq!(
        node_json(&sink, "c3-title")["fontWeight"],
        json!(700),
        "the unset title weight must join the majority's 700"
    );
}

#[test]
fn two_items_are_not_a_family() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "R", "width": 1200, "height": 800,
            "layout": "vertical",
            "children": [
                item("c1", "Card 01", 24.0, "start", 16.0, 700),
                item("c2", "Card 02", 20.0, "start", 16.0, 700)
            ]
        })
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "two items agree by accident"
    );
}

#[test]
fn a_deliberate_hero_first_item_is_skipped_but_the_rest_align() {
    // The hero differs in STRUCTURE (an extra image subtree), so it is
    // excluded from voting and from editing — restructure is intent.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "R", "width": 1200, "height": 800,
            "layout": "vertical",
            "children": [
                { "type": "frame", "id": "hero", "name": "Item 01", "layout": "vertical",
                  "padding": 32, "gap": 12,
                  "children": [
                      { "type": "image", "id": "hero-img", "width": 300, "height": 200, "src": "h.png" },
                      { "type": "text", "id": "hero-title", "content": "Hero", "fontSize": 28 }
                  ] },
                item("c2", "Item 02", 20.0, "start", 16.0, 700),
                item("c3", "Item 03", 20.0, "start", 16.0, 700),
                { "type": "frame", "id": "c4", "name": "Item 04", "layout": "vertical",
                  "padding": 24, "gap": 12, "alignItems": "start",
                  "children": [
                      { "type": "text", "id": "c4-title", "content": "T", "fontSize": 16 },
                      { "type": "text", "id": "c4-body", "content": "B", "fontSize": 14 }
                  ] }
            ]
        })
        .to_string(),
    );

    let applied = run_pass(&mut sink, "root");
    assert_eq!(
        applied, 2,
        "padding plus the missing title weight joining the 700 majority: {applied}"
    );
    assert_eq!(
        node_json(&sink, "c4")["padding"],
        json!(20.0),
        "the consistent outlier joins the consistent majority"
    );
    assert_eq!(
        node_json(&sink, "c4-title")["fontWeight"],
        json!(700),
        "the unset title weight joins the consistent majority"
    );
    // The hero keeps its authored design, structure included.
    let hero = node_json(&sink, "hero");
    assert_eq!(
        hero["padding"],
        json!(32.0),
        "the hero's padding is its own"
    );
    assert_eq!(
        hero["children"].as_array().map(Vec::len),
        Some(2),
        "the hero's image subtree must survive"
    );
}

#[test]
fn equal_font_sizes_produce_no_repairs() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "R", "width": 1200, "height": 800,
            "layout": "vertical",
            "children": [
                item("c1", "Card 01", 20.0, "start", 16.0, 700),
                item("c2", "Card 02", 20.0, "start", 16.0, 700),
                item("c3", "Card 03", 20.0, "start", 16.0, 700)
            ]
        })
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "a consistent family needs no repairs"
    );
}

#[test]
fn three_members_with_three_paddings_have_no_majority() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "R", "width": 1200, "height": 800,
            "layout": "vertical",
            "children": [
                item("c1", "Card 01", 20.0, "start", 16.0, 700),
                item("c2", "Card 02", 24.0, "start", 16.0, 700),
                item("c3", "Card 03", 32.0, "start", 16.0, 700)
            ]
        })
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "no provable majority means no repair basis"
    );
}

#[test]
fn a_non_auto_layout_parent_is_not_a_family() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "R", "width": 1200, "height": 800,
            "children": [
                item("c1", "Card 01", 24.0, "start", 16.0, 700),
                item("c2", "Card 02", 20.0, "start", 16.0, 700),
                item("c3", "Card 03", 20.0, "start", 16.0, 700)
            ]
        })
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "alignment only makes sense in flow"
    );
}

#[test]
fn childless_same_name_offspring_of_different_names_do_not_group() {
    // Three childless frames have an EMPTY (and therefore identical)
    // kind-sequence; an empty sequence proves nothing, and the names do not
    // share a stem — so this must not read as a family.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "R", "width": 1200, "height": 800,
            "layout": "vertical",
            "children": [
                { "type": "frame", "id": "s1", "name": "Title", "padding": 20 },
                { "type": "frame", "id": "s2", "name": "Body", "padding": 20 },
                { "type": "frame", "id": "s3", "name": "Meta", "padding": 24 }
            ]
        })
        .to_string(),
    );

    assert_eq!(run_pass(&mut sink, "root"), 0);
}

#[test]
fn the_pass_is_idempotent() {
    let mut sink = VecDocSink::new();
    insert_drifted(&mut sink);
    assert!(run_pass(&mut sink, "root") > 0, "first run repairs");
    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "the second run has nothing left to repair"
    );
}

#[test]
fn driver_attributes_the_repairs_to_the_equalize_checkpoint() {
    let mut sink = VecDocSink::new();
    insert_drifted(&mut sink);
    let summary = run_driver(&mut sink, "root");

    assert!(
        summary.records().iter().any(|record| {
            record.pass == "equalize-sibling-items" && record.category == CheckCategory::Structure
        }),
        "the pass must be mounted and checkpointed in the driver: {:?}",
        summary.records()
    );
}
