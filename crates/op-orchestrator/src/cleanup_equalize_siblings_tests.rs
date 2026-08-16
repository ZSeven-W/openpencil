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

// ── DS P2-d ①: same-position decorative-container fill votes ────────────────

/// An entry frame whose first child is a number chip (Rectangle) with a
/// primary solid fill, then a title text — the measured 0815 shape whose
/// chip colours drifted per item. `fills` is the chip's whole `fill` array.
fn chip_item(id: &str, name: &str, fills: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "frame",
        "id": id,
        "name": name,
        "layout": "vertical",
        "padding": 20,
        "gap": 12,
        "alignItems": "start",
        "children": [
            { "type": "rectangle", "id": format!("{id}-chip"), "name": "Chip",
              "width": 32, "height": 32, "cornerRadius": 8, "fill": fills },
            { "type": "text", "id": format!("{id}-title"), "content": "Title",
              "fontSize": 16, "fontWeight": 700 }
        ]
    })
}

fn solid(hex: &str) -> serde_json::Value {
    json!([{ "type": "solid", "color": hex }])
}

fn gradient() -> serde_json::Value {
    json!([{
        "type": "linear_gradient",
        "stops": [
            { "offset": 0, "color": "#000000" },
            { "offset": 1, "color": "#ffffff" }
        ]
    }])
}

fn chip_family(chip_fills: &[serde_json::Value]) -> serde_json::Value {
    json!({
        "type": "frame", "id": "root", "name": "Knowledge Card",
        "width": 1200, "height": 800, "layout": "vertical",
        "children": chip_fills.iter().enumerate().map(|(i, fill)| {
            chip_item(&format!("c{}", i + 1), &format!("Card {:02}", i + 1), fill.clone())
        }).collect::<Vec<_>>()
    })
}

fn insert_chip_family(sink: &mut VecDocSink, chip_fills: &[serde_json::Value]) {
    insert_tree(sink, &chip_family(chip_fills).to_string());
}

#[test]
fn a_drifted_chip_fill_is_aligned_to_the_majority_fill() {
    // The 0815 positive shape: four of five entries share one chip colour,
    // the fifth drifted — the outlier joins the family norm.
    let mut sink = VecDocSink::new();
    insert_chip_family(
        &mut sink,
        &[
            solid("#111111"),
            solid("#111111"),
            solid("#111111"),
            solid("#111111"),
            solid("#ff0000"),
        ],
    );

    let applied = run_pass(&mut sink, "root");
    assert_eq!(
        applied, 1,
        "only the drifted chip fill is repaired: {applied}"
    );
    assert_eq!(
        node_json(&sink, "c5-chip")["fill"][0]["color"],
        json!("#111111"),
        "the outlier chip joins the majority: {}",
        node_json(&sink, "c5-chip")
    );
    // Majority members stay untouched.
    assert_eq!(
        node_json(&sink, "c1-chip")["fill"][0]["color"],
        json!("#111111")
    );
}

#[test]
fn three_distinct_chip_fills_have_no_majority() {
    // Black / red / pink — one each on a three-member family: no value
    // clears 2/3, so nothing moves.
    let mut sink = VecDocSink::new();
    insert_chip_family(
        &mut sink,
        &[solid("#000000"), solid("#ff0000"), solid("#ff69b4")],
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "a 1/1/1 split has no provable norm"
    );
}

#[test]
fn a_two_two_one_chip_fill_split_has_no_majority() {
    // The measured 黑/黑/红/红/粉 distribution: 2/5 and 2/5 both fall short
    // of 2/3 — the pass must not guess between two tied camps.
    let mut sink = VecDocSink::new();
    insert_chip_family(
        &mut sink,
        &[
            solid("#000000"),
            solid("#000000"),
            solid("#ff0000"),
            solid("#ff0000"),
            solid("#ff69b4"),
        ],
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "a 2/2/1 split has no provable norm"
    );
}

#[test]
fn a_gradient_chip_blocks_the_whole_fill_position() {
    // One member decorates the chip slot with a gradient: the slot is not
    // proven to be a colour slot, so the ENTIRE position is skipped — even
    // the solid outlier that a per-member abstention would have fixed.
    let mut sink = VecDocSink::new();
    insert_chip_family(
        &mut sink,
        &[
            solid("#111111"),
            solid("#111111"),
            solid("#111111"),
            solid("#ff0000"),
            gradient(),
        ],
    );

    assert_eq!(run_pass(&mut sink, "root"), 0, "the slot stays unprovable");
    assert_eq!(
        node_json(&sink, "c5-chip")["fill"][0]["type"],
        json!("linear_gradient"),
        "the gradient decoration is never touched"
    );
    assert_eq!(
        node_json(&sink, "c4-chip")["fill"][0]["color"],
        json!("#ff0000"),
        "the solid outlier survives too — nothing at this position may move"
    );
}

#[test]
fn variable_reference_chip_fills_vote_by_reference_string() {
    // Same reference is same value: four `$primary` chips against one
    // `$accent` chip — the outlier joins the reference majority, and the
    // patch writes the reference, not a resolved hex.
    let mut sink = VecDocSink::new();
    insert_chip_family(
        &mut sink,
        &[
            solid("$primary"),
            solid("$primary"),
            solid("$primary"),
            solid("$primary"),
            solid("$accent"),
        ],
    );

    let applied = run_pass(&mut sink, "root");
    assert_eq!(applied, 1, "only the ref outlier is repaired: {applied}");
    assert_eq!(
        node_json(&sink, "c5-chip")["fill"][0]["color"],
        json!("$primary"),
        "the outlier joins by reference string: {}",
        node_json(&sink, "c5-chip")
    );
}

#[test]
fn mixed_reference_and_literal_chip_fills_are_not_touched() {
    // `$primary` (3) against `#ff0000` (2): a literal cannot be proven
    // synonymous with a reference, so even the 3/5 literal-free camp must
    // not absorb the others.
    let mut sink = VecDocSink::new();
    insert_chip_family(
        &mut sink,
        &[
            solid("$primary"),
            solid("$primary"),
            solid("$primary"),
            solid("#ff0000"),
            solid("#ff0000"),
        ],
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "the two value systems are never cross-compared"
    );
}

#[test]
fn text_node_fills_are_never_touched() {
    // The chip pass fires (4/5), while the title TEXT fills differ per item
    // — text colour is the contrast pass's territory, never the equalizer's.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "Knowledge Card",
            "width": 1200, "height": 800, "layout": "vertical",
            "children": [
                chip_item("c1", "Card 01", solid("#111111")),
                chip_item("c2", "Card 02", solid("#111111")),
                chip_item("c3", "Card 03", solid("#111111")),
                chip_item("c4", "Card 04", solid("#111111")),
                chip_item("c5", "Card 05", solid("#ff0000"))
            ]
        })
        .to_string(),
    );
    let title_fills = ["#ff0000", "#00ff00", "#0000ff", "#ffff00", "#ff00ff"];
    for (i, hex) in title_fills.iter().enumerate() {
        sink.state.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(format!("c{}-title", i + 1)),
            patch_json: json!({ "fill": [{ "type": "solid", "color": hex }] }).to_string(),
            page_id: None,
        });
    }
    sink.applied.clear();

    let applied = run_pass(&mut sink, "root");
    assert_eq!(
        applied, 1,
        "only the drifted chip fill is repaired — text fills are exempt: {applied}"
    );
    for (i, hex) in title_fills.iter().enumerate() {
        assert_eq!(
            node_json(&sink, &format!("c{}-title", i + 1))["fill"][0]["color"],
            json!(hex),
            "the text fill of item {} must survive the pass",
            i + 1
        );
    }
    assert_eq!(
        node_json(&sink, "c5-chip")["fill"][0]["color"],
        json!("#111111"),
        "the chip outlier still joins the majority"
    );
}

#[test]
fn a_missing_chip_fill_joins_the_majority_fill() {
    // Four chips share a colour and the fifth has NO fill at all — the
    // missing value is drift against the 2/3 norm and inherits it.
    let mut sink = VecDocSink::new();
    insert_chip_family(
        &mut sink,
        &[
            solid("#111111"),
            solid("#111111"),
            solid("#111111"),
            solid("#111111"),
            json!([]),
        ],
    );

    let applied = run_pass(&mut sink, "root");
    assert_eq!(applied, 1, "the missing fill joins the majority: {applied}");
    assert_eq!(
        node_json(&sink, "c5-chip")["fill"][0]["color"],
        json!("#111111"),
        "the fill-less chip inherits the family norm: {}",
        node_json(&sink, "c5-chip")
    );
}

#[test]
fn the_fill_pass_is_idempotent() {
    let mut sink = VecDocSink::new();
    insert_chip_family(
        &mut sink,
        &[
            solid("#111111"),
            solid("#111111"),
            solid("#111111"),
            solid("#111111"),
            solid("#ff0000"),
        ],
    );
    assert!(run_pass(&mut sink, "root") > 0, "first run repairs");
    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "the second run has nothing left to repair"
    );
}
