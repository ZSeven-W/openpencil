//! Task B3 tests for `get_dashboard_placeholder_height` +
//! `reorder_dashboard_main_children`.
//! Linked via `#[path]` in `dashboard_columns.rs`.

use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use crate::test_support::VecDocSink;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};
use serde_json::json;

// ── helpers ──────────────────────────────────────────────────────────────────

fn root_spec(width: f64) -> RootFrameSpec {
    RootFrameSpec {
        id: "root".into(),
        name: "Design".into(),
        width,
        height: 800.0,
        layout: Some("vertical".into()),
        gap: None,
        padding: None,
        fill: None,
    }
}

fn st(id: &str, label: &str, width: f64, height: f64) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region { width, height },
        id_prefix: id.into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    }
}

fn sidebar_st(height: f64) -> Subtask {
    Subtask {
        id: "sidebar-nav".into(),
        label: "Sidebar Navigation".into(),
        region: Region {
            width: 260.0,
            height,
        },
        id_prefix: "sidebar-nav".into(),
        parent_frame_id: None,
        elements: Some("nav links".into()),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    }
}

fn plan_with(width: f64, subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: root_spec(width),
        subtasks,
        style_guide_name: None,
    }
}

// ── `get_dashboard_placeholder_height` tests ─────────────────────────────────

/// Below-floor: no non-sidebar rows → mainFoldHint=560, sidebarHint=0 → 560.
#[test]
fn placeholder_height_empty_rows_returns_floor() {
    // Only a sidebar subtask with height 0 → no non-sidebar rows produced.
    let plan = plan_with(1200.0, vec![sidebar_st(0.0)]);
    let h = get_dashboard_placeholder_height(&plan);
    assert_eq!(h, 560.0, "empty rows → floor 560");
}

/// In-range: 1 row with topbar h=600, sidebar h=700 (capped to 600 → sidebarHint=600).
/// visibleRows = first 2 (rows.len()=1 < 3) = [topbar row].
/// foldHeight = 600 (tallest in 1 row) + 0 gaps = 600.
/// mainFoldHint = 600; sidebarHint = min(700,600)=600.
/// max(600,600)=600 → clamp(560..680) = 600.
#[test]
fn placeholder_height_in_range() {
    let sidebar = sidebar_st(700.0);
    let topbar = st("topbar", "Top Bar", 940.0, 600.0);
    let plan = plan_with(1200.0, vec![sidebar, topbar]);
    let h = get_dashboard_placeholder_height(&plan);
    assert!(
        (560.0..=680.0).contains(&h),
        "in-range plan should give h in [560,680]: got {h}"
    );
    assert_eq!(h, 600.0, "specific formula check: expected 600");
}

/// Above-ceiling: 2 standalone rows each h=500.
/// foldHeight = 500 + 24 + 500 = 1024 → clamped to 680.
#[test]
fn placeholder_height_above_ceiling_clamped() {
    let sidebar = sidebar_st(0.0);
    // full_width = max(1200-260=940, 940) = 940; standalone if w >= 940*0.82=770.8
    let s1 = st("s1", "S1", 940.0, 500.0);
    let s2 = st("s2", "S2", 940.0, 500.0);
    let plan = plan_with(1200.0, vec![sidebar, s1, s2]);
    let h = get_dashboard_placeholder_height(&plan);
    assert_eq!(h, 680.0, "above-ceiling: expected 680 (clamped)");
}

/// When rows.len() >= 3, TS uses first 3.
/// 3 standalone rows h=300,200,100; rowGap=24.
/// foldHeight = 300+24+200+24+100 = 648 → clamp = 648.
#[test]
fn placeholder_height_three_rows_uses_first_three() {
    let sidebar = sidebar_st(0.0);
    let s1 = st("s1", "S1", 940.0, 300.0);
    let s2 = st("s2", "S2", 940.0, 200.0);
    let s3 = st("s3", "S3", 940.0, 100.0);
    let plan = plan_with(1200.0, vec![sidebar, s1, s2, s3]);
    let h = get_dashboard_placeholder_height(&plan);
    assert_eq!(h, 648.0, "3 rows: expected 648");
}

/// When rows.len() < 3 (here 2 rows), TS uses first 2.
/// heights: 350, 250; rowGap=24; foldHeight = 350+24+250 = 624.
#[test]
fn placeholder_height_two_rows_uses_first_two() {
    let sidebar = sidebar_st(0.0);
    let s1 = st("s1", "S1", 940.0, 350.0);
    let s2 = st("s2", "S2", 940.0, 250.0);
    let plan = plan_with(1200.0, vec![sidebar, s1, s2]);
    let h = get_dashboard_placeholder_height(&plan);
    assert_eq!(h, 624.0, "2 rows: expected 624");
}

// ── `reorder_dashboard_main_children` tests ──────────────────────────────────

/// Builds a PenNode frame from JSON for test use.
fn frame_val(id: &str, name: &str, children: serde_json::Value) -> serde_json::Value {
    json!({
        "type": "frame", "id": id, "name": name,
        "width": 100, "height": 100,
        "children": children,
    })
}

/// Finds the actual id of the first node named `name` in `state`'s active children
/// (recursively).
fn find_by_name(state: &op_editor_core::EditorState, name: &str) -> Option<String> {
    fn search(nodes: &[jian_ops_schema::node::PenNode], name: &str) -> Option<String> {
        for node in nodes {
            if node.base().name.as_deref() == Some(name) {
                return Some(node.id_str().to_string());
            }
            if let Some(ch) = node.children() {
                if let Some(id) = search(ch, name) {
                    return Some(id);
                }
            }
        }
        None
    }
    search(state.active_children(), name)
}

/// Basic reorder: 3 direct-child slots in order [C, B, A]; plan order [A, B, C].
/// Each subtask's parent_frame_id = its slot id (direct child of main).
/// Expected: 3 MoveNode commands in order slot-A, slot-B, slot-C.
#[test]
fn reorder_produces_move_node_commands_in_plan_order() {
    let mut sink = VecDocSink::new();

    // Insert main frame with children in [C, B, A] order.
    let main_node: jian_ops_schema::node::PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "root-main", "name": "Main",
        "width": 940, "height": 100,
        "children": [
            frame_val("slot-c", "SlotC", json!([])),
            frame_val("slot-b", "SlotB", json!([])),
            frame_val("slot-a", "SlotA", json!([])),
        ],
    }))
    .unwrap();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![main_node],
        parent_id: NodeId::NONE,
        page_id: None,
    });

    let main_id = find_by_name(&sink.state, "Main").expect("Main not found");
    let slot_a_id = find_by_name(&sink.state, "SlotA").expect("SlotA not found");
    let slot_b_id = find_by_name(&sink.state, "SlotB").expect("SlotB not found");
    let slot_c_id = find_by_name(&sink.state, "SlotC").expect("SlotC not found");

    // Plan: sidebar + subtask-a (pfid=slot-a) + subtask-b (pfid=slot-b) + subtask-c (pfid=slot-c).
    let sidebar = sidebar_st(0.0);
    let mut st_a = st("subtask-a", "Subtask A", 940.0, 200.0);
    st_a.parent_frame_id = Some(slot_a_id.clone());
    let mut st_b = st("subtask-b", "Subtask B", 940.0, 200.0);
    st_b.parent_frame_id = Some(slot_b_id.clone());
    let mut st_c = st("subtask-c", "Subtask C", 940.0, 200.0);
    st_c.parent_frame_id = Some(slot_c_id.clone());
    let plan = plan_with(1200.0, vec![sidebar, st_a, st_b, st_c]);

    let cmds = reorder_dashboard_main_children(&plan, &main_id, &sink.state);

    // Extract moved node ids.
    let moved_ids: Vec<String> = cmds
        .iter()
        .filter_map(|cmd| match cmd {
            EditorCommand::MoveNode { node_id, .. } => Some(node_id.as_str().to_string()),
            _ => None,
        })
        .collect();

    // All MoveNode targets should be main_id.
    for cmd in &cmds {
        if let EditorCommand::MoveNode { target_parent, .. } = cmd {
            assert_eq!(
                target_parent.as_str(),
                main_id,
                "MoveNode target must be main_id"
            );
        }
    }

    assert_eq!(moved_ids.len(), 3, "expected 3 MoveNode commands");
    assert_eq!(moved_ids[0], slot_a_id, "first move: slot-a");
    assert_eq!(moved_ids[1], slot_b_id, "second move: slot-b");
    assert_eq!(moved_ids[2], slot_c_id, "third move: slot-c");
}

/// Slot-in-row: subtask's parentFrameId is a slot under a row under main.
/// Expected: the row id (not the slot id) is used in the MoveNode command.
#[test]
fn reorder_resolves_slot_to_row_ancestor() {
    let mut sink = VecDocSink::new();

    // main has: [row-frame (with slot-a, slot-b), standalone-slot]
    let main_node: jian_ops_schema::node::PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "root-main", "name": "Main",
        "width": 940, "height": 100,
        "children": [
            {
                "type": "frame", "id": "row-2", "name": "Row2",
                "width": "fill_container", "height": "fit_content",
                "layout": "horizontal",
                "children": [
                    frame_val("slot-a", "SlotA", json!([])),
                    frame_val("slot-b", "SlotB", json!([])),
                ],
            },
            frame_val("standalone-slot", "StandaloneSlot", json!([])),
        ],
    }))
    .unwrap();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![main_node],
        parent_id: NodeId::NONE,
        page_id: None,
    });

    let main_id = find_by_name(&sink.state, "Main").expect("Main not found");
    let row2_id = find_by_name(&sink.state, "Row2").expect("Row2 not found");
    let standalone_id =
        find_by_name(&sink.state, "StandaloneSlot").expect("StandaloneSlot not found");
    let slot_a_id = find_by_name(&sink.state, "SlotA").expect("SlotA not found");

    // Plan: sidebar + standalone (pfid=standalone-slot, direct child of main)
    //               + subtask-a (pfid=slot-a, child of row-2 which is child of main)
    let sidebar = sidebar_st(0.0);
    let mut standalone_subtask = st("standalone", "Standalone", 940.0, 96.0);
    standalone_subtask.parent_frame_id = Some(standalone_id.clone());
    let mut subtask_a = st("subtask-a", "Subtask A", 460.0, 300.0);
    subtask_a.parent_frame_id = Some(slot_a_id.clone());
    let plan = plan_with(1200.0, vec![sidebar, standalone_subtask, subtask_a]);

    let cmds = reorder_dashboard_main_children(&plan, &main_id, &sink.state);
    let moved_ids: Vec<String> = cmds
        .iter()
        .filter_map(|cmd| match cmd {
            EditorCommand::MoveNode { node_id, .. } => Some(node_id.as_str().to_string()),
            _ => None,
        })
        .collect();

    assert_eq!(moved_ids.len(), 2, "expected 2 MoveNode commands");
    assert_eq!(
        moved_ids[0], standalone_id,
        "first: standalone slot (direct child of main)"
    );
    assert_eq!(moved_ids[1], row2_id, "second: row-2 (parent of slot-a)");
}

/// Sidebar subtasks are skipped.
#[test]
fn reorder_skips_sidebar_subtasks() {
    let sink = VecDocSink::new();
    let plan = plan_with(1200.0, vec![sidebar_st(300.0)]);
    let cmds = reorder_dashboard_main_children(&plan, "root-main", &sink.state);
    assert!(cmds.is_empty(), "all-sidebar plan → no commands");
}

/// Falls back to `generated_root_id` when `parent_frame_id` is None.
#[test]
fn reorder_uses_generated_root_id_as_fallback() {
    let mut sink = VecDocSink::new();

    let main_node: jian_ops_schema::node::PenNode = serde_json::from_value(json!({
        "type": "frame", "id": "root-main", "name": "Main",
        "width": 940, "height": 100,
        "children": [
            frame_val("gen-b", "GenB", json!([])),
            frame_val("gen-a", "GenA", json!([])),
        ],
    }))
    .unwrap();
    sink.state.apply(EditorCommand::InsertSubtree {
        nodes: vec![main_node],
        parent_id: NodeId::NONE,
        page_id: None,
    });

    let main_id = find_by_name(&sink.state, "Main").expect("Main not found");
    let gen_a_id = find_by_name(&sink.state, "GenA").expect("GenA not found");
    let gen_b_id = find_by_name(&sink.state, "GenB").expect("GenB not found");

    // subtask-a: no parent_frame_id → falls back to generated_root_id = gen_a.
    // subtask-b: no parent_frame_id → falls back to generated_root_id = gen_b.
    let sidebar = sidebar_st(0.0);
    let mut st_a = st("subtask-a", "Subtask A", 940.0, 200.0);
    st_a.parent_frame_id = None;
    st_a.generated_root_id = Some(gen_a_id.clone());
    let mut st_b = st("subtask-b", "Subtask B", 940.0, 200.0);
    st_b.parent_frame_id = None;
    st_b.generated_root_id = Some(gen_b_id.clone());
    let plan = plan_with(1200.0, vec![sidebar, st_a, st_b]);

    let cmds = reorder_dashboard_main_children(&plan, &main_id, &sink.state);
    let moved_ids: Vec<String> = cmds
        .iter()
        .filter_map(|cmd| match cmd {
            EditorCommand::MoveNode { node_id, .. } => Some(node_id.as_str().to_string()),
            _ => None,
        })
        .collect();

    assert_eq!(moved_ids.len(), 2, "expected 2 MoveNode commands");
    assert_eq!(moved_ids[0], gen_a_id, "first: gen_a");
    assert_eq!(moved_ids[1], gen_b_id, "second: gen_b");
}
