//! Tests for `sibling_style_drift::repair_sibling_style_drift`: a family of
//! four or more structurally identical sibling tiles whose style facts
//! drifted gets unified to the majority norm; a lone "selected" tile, a
//! too-small family, non-twin siblings and chrome subtrees are left alone.

use super::*;
use crate::cleanup::run_cleanup_passes_with_summary;
use crate::plan::{OrchestratorPlan, RootFrameSpec};
use crate::repair_summary::{CheckCategory, RepairSummary};
use crate::test_support::VecDocSink;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::{json, Value};

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

fn node_json(sink: &VecDocSink, id: &str) -> Value {
    let node = sink
        .state
        .active_children()
        .iter()
        .find_map(|root| find_node(root, id))
        .unwrap_or_else(|| panic!("node `{id}` exists"));
    serde_json::to_value(node).expect("serialize")
}

fn run_pass(sink: &mut VecDocSink, root_id: &str) -> usize {
    repair_sibling_style_drift(sink, root_id)
}

fn run_driver(sink: &mut VecDocSink, root_id: &str) -> RepairSummary {
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: root_id.to_string(),
            name: "首页".into(),
            width: 390.0,
            height: 844.0,
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

/// The measured 2026-09-05 tile shape: `frame[frame[icon_font],text]` — a
/// 56x56 icon chip ("X 图标底") with a solid fill over a label. `stroked`
/// toggles the 1px `$--border` tile stroke; `icon_color` is the icon's fill
/// token.
fn tile(id: &str, stroked: bool, icon_color: &str) -> Value {
    let mut tile = json!({
        "type": "frame",
        "id": id,
        "name": format!("分类 {id}"),
        "layout": "vertical",
        "alignItems": "center",
        "gap": 6,
        "cornerRadius": 12,
        "children": [
            {
                "type": "frame",
                "id": format!("{id}-chip"),
                "name": format!("{id} 图标底"),
                "width": 56,
                "height": 56,
                "cornerRadius": 16,
                "fill": [{ "type": "solid", "color": "$--secondary" }],
                "children": [
                    {
                        "type": "icon_font",
                        "id": format!("{id}-icon"),
                        "iconFontName": "fork",
                        "width": 24,
                        "height": 24,
                        "fill": [{ "type": "solid", "color": icon_color }]
                    }
                ]
            },
            {
                "type": "text",
                "id": format!("{id}-label"),
                "content": "美食",
                "fontSize": 12
            }
        ]
    });
    if stroked {
        tile["stroke"] = json!({
            "thickness": 1,
            "fill": [{ "type": "solid", "color": "$--border" }]
        });
    }
    tile
}

/// The evidence shape: a 3x3 category grid planned as three row frames under
/// one grid parent. The first column (t1/t4/t7) drifted: no stroke, and a
/// `$--primary` icon instead of `$--secondary-foreground`.
fn category_grid() -> Value {
    let row = |row_id: &str, tiles: Vec<Value>| {
        json!({
            "type": "frame", "id": row_id, "name": row_id,
            "layout": "horizontal", "gap": 12,
            "children": tiles
        })
    };
    json!({
        "type": "frame", "id": "root", "name": "首页",
        "width": 390, "height": 844, "layout": "vertical",
        "children": [
            {
                "type": "frame", "id": "grid", "name": "分类网格",
                "layout": "vertical", "gap": 12,
                "children": [
                    row("row-1", vec![
                        tile("t1", false, "$--primary"),
                        tile("t2", true, "$--secondary-foreground"),
                        tile("t3", true, "$--secondary-foreground"),
                    ]),
                    row("row-2", vec![
                        tile("t4", false, "$--primary"),
                        tile("t5", true, "$--secondary-foreground"),
                        tile("t6", true, "$--secondary-foreground"),
                    ]),
                    row("row-3", vec![
                        tile("t7", false, "$--primary"),
                        tile("t8", true, "$--secondary-foreground"),
                        tile("t9", true, "$--secondary-foreground"),
                    ]),
                ]
            }
        ]
    })
}

/// A flat single row/column of tiles directly under the root.
fn flat_grid(tiles: Vec<Value>) -> Value {
    json!({
        "type": "frame", "id": "root", "name": "R",
        "width": 390, "height": 844, "layout": "horizontal",
        "children": tiles
    })
}

#[test]
fn the_drifted_first_column_joins_the_grid_norm() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &category_grid().to_string());

    let applied = run_pass(&mut sink, "root");
    assert_eq!(
        applied, 9,
        "three drifted tiles x three commands (stroke colour + thickness + icon fill): {applied}"
    );

    for id in ["t1", "t4", "t7"] {
        let tile = node_json(&sink, id);
        assert_eq!(
            tile["stroke"]["thickness"],
            json!(1.0),
            "the drifted tile inherits the majority stroke: {tile}"
        );
        assert_eq!(
            tile["stroke"]["fill"][0]["color"],
            json!("$--border"),
            "the adopted stroke colour is the majority camp's: {tile}"
        );
        let icon = node_json(&sink, &format!("{id}-icon"));
        assert_eq!(
            icon["fill"][0]["color"],
            json!("$--secondary-foreground"),
            "the drifted icon joins the majority token: {icon}"
        );
        // Text content and icon names are never touched.
        assert_eq!(
            node_json(&sink, &format!("{id}-label"))["content"],
            json!("美食")
        );
        assert_eq!(icon["iconFontName"], json!("fork"));
    }
    // The majority tiles were already on the norm and stay untouched.
    assert_eq!(node_json(&sink, "t2")["stroke"]["thickness"], json!(1.0));
    assert_eq!(
        node_json(&sink, "t2-icon")["fill"][0]["color"],
        json!("$--secondary-foreground")
    );
}

#[test]
fn a_single_outlier_in_a_four_tile_row_is_left_alone() {
    // One highlighted tile among four is deliberate selection, not drift.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &flat_grid(vec![
            tile("t1", false, "$--primary"),
            tile("t2", true, "$--secondary-foreground"),
            tile("t3", true, "$--secondary-foreground"),
            tile("t4", true, "$--secondary-foreground"),
        ])
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "a lone outlier is a selected tile, not drift"
    );
    assert!(
        node_json(&sink, "t1").get("stroke").is_none(),
        "the highlighted tile keeps its authored look"
    );
    assert_eq!(
        node_json(&sink, "t1-icon")["fill"][0]["color"],
        json!("$--primary")
    );
}

#[test]
fn a_family_of_three_is_untouched() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &flat_grid(vec![
            tile("t1", false, "$--primary"),
            tile("t2", true, "$--secondary-foreground"),
            tile("t3", true, "$--secondary-foreground"),
        ])
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "three tiles can drift together — a family needs four"
    );
}

#[test]
fn different_signatures_do_not_form_a_family() {
    // Two tiles and two badge-shaped frames: neither kind reaches four
    // members, so the drift inside each pair is unprovable.
    let badge = |id: &str, stroked: bool| {
        let mut badge = json!({
            "type": "frame", "id": id, "name": id,
            "layout": "horizontal", "gap": 4,
            "children": [
                { "type": "text", "id": format!("{id}-label"), "content": "B", "fontSize": 12 }
            ]
        });
        if stroked {
            badge["stroke"] = json!({
                "thickness": 1,
                "fill": [{ "type": "solid", "color": "$--border" }]
            });
        }
        badge
    };
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &flat_grid(vec![
            tile("t1", false, "$--primary"),
            tile("t2", true, "$--secondary-foreground"),
            badge("b1", false),
            badge("b2", true),
        ])
        .to_string(),
    );

    assert_eq!(run_pass(&mut sink, "root"), 0);
    assert!(node_json(&sink, "t1").get("stroke").is_none());
    assert!(node_json(&sink, "b1").get("stroke").is_none());
}

#[test]
fn status_bar_subtree_is_skipped() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "首页",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "status-bar", "role": "status-bar",
                    "layout": "horizontal", "gap": 8,
                    "children": [
                        tile("s1", false, "$--primary"),
                        tile("s2", true, "$--secondary-foreground"),
                        tile("s3", true, "$--secondary-foreground"),
                        tile("s4", true, "$--secondary-foreground"),
                    ]
                }
            ]
        })
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "chrome is not content — the status bar subtree is never entered"
    );
    assert!(node_json(&sink, "s1").get("stroke").is_none());
}

#[test]
fn bottom_tab_bar_subtree_is_skipped() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &json!({
            "type": "frame", "id": "root", "name": "首页",
            "width": 390, "height": 844, "layout": "vertical",
            "children": [
                {
                    "type": "frame", "id": "tab-bar", "role": "bottom-tab-bar",
                    "layout": "horizontal", "gap": 8,
                    "children": [
                        tile("n1", false, "$--primary"),
                        tile("n2", true, "$--secondary-foreground"),
                        tile("n3", true, "$--secondary-foreground"),
                        tile("n4", true, "$--secondary-foreground"),
                    ]
                }
            ]
        })
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "the bottom tab bar styles its tabs on purpose"
    );
    assert!(node_json(&sink, "n1").get("stroke").is_none());
}

#[test]
fn a_lone_paint_outlier_in_a_five_tile_row_reads_as_selected() {
    // Five members: the majority (4) is provable, so this isolates the
    // one-outlier carve-out from the no-majority rule.
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &flat_grid(vec![
            tile("t1", false, "$--primary"),
            tile("t2", true, "$--secondary-foreground"),
            tile("t3", true, "$--secondary-foreground"),
            tile("t4", true, "$--secondary-foreground"),
            tile("t5", true, "$--secondary-foreground"),
        ])
        .to_string(),
    );

    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "one paint-only outlier is a deliberately highlighted tile"
    );
    assert!(node_json(&sink, "t1").get("stroke").is_none());
}

#[test]
fn two_paint_outliers_are_drift_not_selection() {
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &flat_grid(vec![
            tile("t1", false, "$--primary"),
            tile("t2", false, "$--primary"),
            tile("t3", true, "$--secondary-foreground"),
            tile("t4", true, "$--secondary-foreground"),
            tile("t5", true, "$--secondary-foreground"),
            tile("t6", true, "$--secondary-foreground"),
        ])
        .to_string(),
    );

    let applied = run_pass(&mut sink, "root");
    assert_eq!(
        applied, 6,
        "two drifted tiles x three commands — selection is exactly one tile: {applied}"
    );
    for id in ["t1", "t2"] {
        assert_eq!(node_json(&sink, id)["stroke"]["thickness"], json!(1.0));
        assert_eq!(
            node_json(&sink, &format!("{id}-icon"))["fill"][0]["color"],
            json!("$--secondary-foreground")
        );
    }
}

#[test]
fn a_selected_marked_tile_is_never_edited() {
    // Two outliers: the "Selected" one keeps its authored highlight, the
    // unmarked one is drift and joins the norm.
    let mut marked = tile("t1", false, "$--primary");
    marked["name"] = json!("分类 Selected");
    let mut sink = VecDocSink::new();
    insert_tree(
        &mut sink,
        &flat_grid(vec![
            marked,
            tile("t2", false, "$--primary"),
            tile("t3", true, "$--secondary-foreground"),
            tile("t4", true, "$--secondary-foreground"),
            tile("t5", true, "$--secondary-foreground"),
            tile("t6", true, "$--secondary-foreground"),
        ])
        .to_string(),
    );

    let applied = run_pass(&mut sink, "root");
    assert_eq!(
        applied, 3,
        "only the unmarked outlier is repaired: {applied}"
    );
    assert!(
        node_json(&sink, "t1").get("stroke").is_none(),
        "the selected tile keeps its highlight"
    );
    assert_eq!(node_json(&sink, "t2")["stroke"]["thickness"], json!(1.0));
}

#[test]
fn the_pass_is_idempotent() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &category_grid().to_string());
    assert!(run_pass(&mut sink, "root") > 0, "first run repairs");
    assert_eq!(
        run_pass(&mut sink, "root"),
        0,
        "the second run has nothing left to repair"
    );
}

#[test]
fn driver_attributes_the_repairs_to_the_sibling_style_drift_checkpoint() {
    let mut sink = VecDocSink::new();
    insert_tree(&mut sink, &category_grid().to_string());
    let summary = run_driver(&mut sink, "root");

    assert!(
        summary.records().iter().any(|record| {
            record.pass == "sibling-style-drift" && record.category == CheckCategory::Structure
        }),
        "the pass must be mounted and checkpointed in the driver: {:?}",
        summary.records()
    );
}
