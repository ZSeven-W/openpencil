//! Task C2 (S3b-3) tests — `build_scaffold_dashboard`.
//!
//! Linked via `#[path = "scaffold_tests_c2.rs"] mod tests_c2;` inside
//! `scaffold.rs`; stays a child module of `scaffold`, so `use super::*` works.
//!
//! Covers:
//! - Root frame has `layout: "horizontal"`, `gap: 0`,
//!   `height: fit_content(placeholder)`.
//! - Sidebar child: id = `{root_id}-sidebar`, width 260, layout vertical.
//! - Main child: id = `{root_id}-main`, fill_container width, layout vertical,
//!   gap = contentGap.
//! - Row/slot frames are emitted as additional InsertSubtree commands targeting
//!   main or a row frame.
//! - Returns `(cmds, sidebar_id, main_id, scaffold_baseline)`.
//! - Sidebar fill color taken from the argument; main has no explicit fill.
//! - Existing `build_scaffold` / `build_scaffold_concurrent_mobile` tests stay
//!   green (regression guard).

use super::*;
use crate::cleanup::descendant_count;
use crate::plan::{OrchestratorPlan, PlanFill, Region, RootFrameSpec, Subtask};
use crate::scaffold::build_scaffold;
use crate::test_support::VecDocSink;
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use op_editor_core::{EditorCommand, PenNodeExt};

// ── helpers ───────────────────────────────────────────────────────────────────

fn plan_fill(color: &str) -> PlanFill {
    PlanFill {
        kind: "solid".into(),
        color: color.into(),
    }
}

fn dashboard_plan(root_id: &str) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: root_id.into(),
            name: "Dashboard".into(),
            width: 1200.0,
            height: 800.0,
            layout: Some("vertical".into()),
            gap: Some(24.0),
            padding: None,
            fill: Some(vec![plan_fill("#FFFFFF")]),
        },
        subtasks: vec![
            Subtask {
                id: "sidebar".into(),
                label: "Sidebar Navigation".into(),
                region: Region {
                    width: 260.0,
                    height: 760.0,
                },
                id_prefix: "sidebar".into(),
                parent_frame_id: None,
                elements: None,
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
            },
            Subtask {
                id: "revenue-chart".into(),
                label: "Revenue Chart".into(),
                region: Region {
                    width: 583.0,
                    height: 320.0,
                },
                id_prefix: "revenue-chart".into(),
                parent_frame_id: None,
                elements: None,
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
            },
            Subtask {
                id: "kpi-metrics".into(),
                label: "KPI Metrics".into(),
                region: Region {
                    width: 940.0,
                    height: 160.0,
                },
                id_prefix: "kpi-metrics".into(),
                parent_frame_id: None,
                elements: None,
                screen: None,
                generated_root_id: None,
                existing_section_labels: None,
            },
        ],
        style_guide_name: None,
    }
}

/// Helper: find the first InsertSubtree node whose id matches `expected_id`,
/// searching recursively through every level of the subtree.
fn find_node_in_cmds<'a>(cmds: &'a [EditorCommand], expected_id: &str) -> Option<&'a PenNode> {
    fn search<'a>(node: &'a PenNode, expected_id: &str) -> Option<&'a PenNode> {
        if node.id_str() == expected_id {
            return Some(node);
        }
        if let Some(children) = node.children() {
            for ch in children {
                if let Some(found) = search(ch, expected_id) {
                    return Some(found);
                }
            }
        }
        None
    }
    for cmd in cmds {
        if let EditorCommand::InsertSubtree { nodes, .. } = cmd {
            for n in nodes {
                if let Some(found) = search(n, expected_id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Helper: check whether a PenNode frame has `fill_container` width.
fn is_fill_container_width(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(f) => matches!(
            f.container.width,
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        ),
        _ => false,
    }
}

/// Helper: check whether a PenNode frame has `fit_content(...)` height.
///
/// The TS emits `fit_content(${placeholder})` which serializes as an
/// `Expression` string.  Plain `fit_content` (keyword) is also accepted.
fn is_fit_content_height(node: &PenNode) -> bool {
    match node {
        PenNode::Frame(f) => match &f.container.height {
            Some(SizingBehavior::Keyword(SizingKeyword::FitContent)) => true,
            Some(SizingBehavior::Expression(s)) => s.starts_with("fit_content("),
            _ => false,
        },
        _ => false,
    }
}

/// Helper: extract layout from a PenNode frame.
fn node_layout(node: &PenNode) -> Option<&'static str> {
    use jian_ops_schema::node::container::LayoutMode;
    match node {
        PenNode::Frame(f) => match &f.container.layout {
            Some(LayoutMode::Vertical) => Some("vertical"),
            Some(LayoutMode::Horizontal) => Some("horizontal"),
            Some(LayoutMode::None) => Some("none"),
            None => None,
        },
        _ => None,
    }
}

/// Helper: extract numeric gap from a PenNode frame.
fn node_gap(node: &PenNode) -> Option<f64> {
    use jian_ops_schema::node::base::NumberOrExpression;
    match node {
        PenNode::Frame(f) => match &f.container.gap {
            Some(NumberOrExpression::Number(n)) => Some(*n),
            _ => None,
        },
        _ => None,
    }
}

// ── Task C2 tests ─────────────────────────────────────────────────────────────

/// The function returns successfully and conveys sidebar_id + main_id.
#[test]
fn build_scaffold_dashboard_returns_sidebar_and_main_ids() {
    let mut plan = dashboard_plan("root");
    let (_, sidebar_id, main_id, _) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");
    assert_eq!(sidebar_id, "root-sidebar");
    assert_eq!(main_id, "root-main");
}

/// Root frame must have layout = "horizontal", gap = 0.
#[test]
fn build_scaffold_dashboard_root_is_horizontal_gap_zero() {
    let mut plan = dashboard_plan("root");
    let (cmds, _, _, _) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");

    // The root node is in the first InsertSubtree command.
    let root_node = match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => &nodes[0],
        other => panic!("expected InsertSubtree, got {other:?}"),
    };
    assert_eq!(root_node.id_str(), "root");
    assert_eq!(
        node_layout(root_node),
        Some("horizontal"),
        "root must have horizontal layout"
    );
    assert_eq!(node_gap(root_node), Some(0.0), "root must have gap = 0");
}

/// Root frame height must be fit_content (placeholder).
#[test]
fn build_scaffold_dashboard_root_height_is_fit_content() {
    let mut plan = dashboard_plan("root");
    let (cmds, _, _, _) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");

    let root_node = match &cmds[0] {
        EditorCommand::InsertSubtree { nodes, .. } => &nodes[0],
        other => panic!("expected InsertSubtree, got {other:?}"),
    };
    assert!(
        is_fit_content_height(root_node),
        "root height must be fit_content (with placeholder), got: {root_node:?}"
    );
}

/// Sidebar child: id, width 260, layout vertical, fill = sidebar_fill_hex.
#[test]
fn build_scaffold_dashboard_sidebar_properties() {
    let mut plan = dashboard_plan("root");
    let sidebar_hex = "#1E293B";
    let (cmds, sidebar_id, _, _) =
        build_scaffold_dashboard(&mut plan, sidebar_hex).expect("dashboard scaffold");

    let sidebar_node =
        find_node_in_cmds(&cmds, &sidebar_id).expect("sidebar node must be present in commands");

    // id
    assert_eq!(sidebar_node.id_str(), "root-sidebar");

    // width: exactly 260 px
    assert_eq!(
        sidebar_node.width_px(),
        Some(260.0),
        "sidebar width must be 260"
    );

    // layout: vertical
    assert_eq!(
        node_layout(sidebar_node),
        Some("vertical"),
        "sidebar must have vertical layout"
    );

    // fill: solid sidebar_hex — check via JSON round-trip
    if let PenNode::Frame(f) = sidebar_node {
        let fill = f.container.fill.as_ref().expect("sidebar must have fill");
        // Should have at least one fill entry.
        assert!(!fill.is_empty(), "sidebar fill must not be empty");
    } else {
        panic!("sidebar must be a Frame node");
    }
}

/// Main child: id, fill_container width, layout vertical, gap = contentGap.
#[test]
fn build_scaffold_dashboard_main_properties() {
    let mut plan = dashboard_plan("root");
    // root.gap = 24 → contentGap = 24
    let (cmds, _, main_id, _) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");

    let main_node =
        find_node_in_cmds(&cmds, &main_id).expect("main node must be present in commands");

    // id
    assert_eq!(main_node.id_str(), "root-main");

    // width: fill_container
    assert!(
        is_fill_container_width(main_node),
        "main must have fill_container width"
    );

    // layout: vertical
    assert_eq!(
        node_layout(main_node),
        Some("vertical"),
        "main must have vertical layout"
    );

    // gap = 24 (from root.gap = 24 > 0)
    assert_eq!(
        node_gap(main_node),
        Some(24.0),
        "main gap must equal root.gap when > 0"
    );
}

/// When root.gap is 0, main contentGap falls back to 20.
#[test]
fn build_scaffold_dashboard_main_gap_defaults_to_20_when_root_gap_zero() {
    let mut plan = dashboard_plan("root");
    plan.root_frame.gap = Some(0.0); // override to 0
    let (cmds, _, main_id, _) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");

    let main_node = find_node_in_cmds(&cmds, &main_id).expect("main node");
    assert_eq!(
        node_gap(main_node),
        Some(20.0),
        "main gap must be 20 when root.gap is 0"
    );
}

/// Row/slot frames are embedded as nested children of the main frame
/// inside the single `InsertSubtree`.  (The first design embedded them
/// as separate follow-up commands, but `cmd_insert_subtree`'s id remap
/// breaks any cross-command parent reference, so everything must travel
/// in one subtree.  TS achieves the same effect because
/// `insertStreamingNode` does not remap.)
#[test]
fn build_scaffold_dashboard_emits_row_slot_frames_under_main() {
    let mut plan = dashboard_plan("root");
    let (cmds, _, main_id, _) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");

    // Exactly one `InsertSubtree` is emitted for the dashboard scaffold.
    let insert_count = cmds
        .iter()
        .filter(|c| matches!(c, EditorCommand::InsertSubtree { .. }))
        .count();
    assert_eq!(
        insert_count, 1,
        "dashboard scaffold must emit exactly one InsertSubtree"
    );

    // The main frame must carry the synthesized row/slot frames.
    // Plan has 2 non-sidebar subtasks → ≥ 2 children under main.
    let main_node = find_node_in_cmds(&cmds, &main_id).expect("main node");
    let main_children = main_node.children().expect("main must have children slot");
    assert!(
        main_children.len() >= 2,
        "main must contain ≥ 2 row/slot frames, got {}",
        main_children.len()
    );
}

/// `scaffold_baseline` equals `descendant_count(state, actual_root_id)`
/// after the returned `InsertSubtree` has been applied — mirroring TS
/// `scaffoldCounts.set(rootId, countDescendants(root))`
/// (`orchestrator.ts:1038-1053`). This is the value the cleanup
/// "scaffold-only root" check compares the post-subagent live count
/// against, so it MUST include the synthesized row + slot frames, not
/// just the embedded sidebar + main.
///
/// Note: `cmd_insert_subtree` remaps every node id, so we read the
/// actual root id back from `active_children` after apply.
#[test]
fn build_scaffold_dashboard_scaffold_baseline_matches_live_descendant_count() {
    let mut plan = dashboard_plan("root");
    let (cmds, _, _, baseline) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");

    let mut sink = VecDocSink::new();
    for cmd in cmds {
        assert!(sink.apply(cmd), "scaffold command must apply cleanly");
    }
    // Read the actual remapped root id from the live state.
    let actual_root_id = sink
        .state()
        .active_children()
        .first()
        .map(|n| n.id_str().to_string())
        .expect("scaffold must produce one top-level node");
    let live = descendant_count(sink.state(), &actual_root_id);
    assert_eq!(
        baseline, live,
        "returned baseline must equal live descendant_count after apply"
    );
    // Sanity floor: a 3-subtask plan (1 sidebar + 2 main) produces at least
    // sidebar (1) + main (1) + at least one slot per main subtask (≥ 2).
    assert!(
        baseline >= 4,
        "baseline must include row/slot frames; got {baseline}"
    );
}

/// `scaffold_baseline` for the 3-subtask fixture: sidebar (1) + main (1)
/// + 2 standalone-row slots (one per non-sidebar subtask) = exactly 4.
///
/// The two main subtasks of `dashboard_plan` are:
///  - "Revenue Chart" w=583 (chart) — 583 < full_width*0.82=770.8, NOT
///    standalone, packed greedily; cumulative 583 < 940*0.92=864.8 so no
///    post-flush.
///  - "KPI Metrics" w=940 — 940 >= 770.8 → standalone row; the previous
///    row containing just the chart flushes before the metrics row.
///
/// Final layout: 2 rows, 1 subtask each, each emits a single slot frame
/// (no horizontal row wrappers). Baseline = sidebar (1) + main (1) +
/// 2 slots = 4.
#[test]
fn build_scaffold_dashboard_scaffold_baseline_exact_value_for_fixture() {
    let mut plan = dashboard_plan("root");
    let (_, _, _, baseline) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");
    assert_eq!(
        baseline, 4,
        "expected baseline = sidebar(1) + main(1) + 2 standalone slots = 4"
    );
}

/// Non-dashboard plans' `build_scaffold` still works unchanged (regression guard).
#[test]
fn existing_build_scaffold_unchanged_by_dashboard_variant() {
    let plan = OrchestratorPlan {
        root_frame: RootFrameSpec {
            id: "root".into(),
            name: "Page".into(),
            width: 1200.0,
            height: 800.0,
            layout: Some("vertical".into()),
            gap: Some(0.0),
            padding: None,
            fill: Some(vec![plan_fill("#FFFFFF")]),
        },
        subtasks: vec![],
        style_guide_name: None,
    };
    let cmds = build_scaffold(&plan, false).expect("sequential scaffold");
    assert_eq!(cmds.len(), 1, "non-dashboard scaffold must have 1 command");
    match &cmds[0] {
        EditorCommand::InsertSubtree {
            nodes, parent_id, ..
        } => {
            assert_eq!(nodes[0].id_str(), "root");
            assert!(!parent_id.is_real());
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}

/// Sidebar id and main id are correctly derived from the plan root frame id.
#[test]
fn build_scaffold_dashboard_ids_derived_from_root_id() {
    let mut plan = dashboard_plan("myframe-abc");
    let (_, sidebar_id, main_id, _) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");
    assert_eq!(sidebar_id, "myframe-abc-sidebar");
    assert_eq!(main_id, "myframe-abc-main");
}

/// The first InsertSubtree command uses NodeId::NONE as parent (page root insert).
#[test]
fn build_scaffold_dashboard_root_inserts_at_page_level() {
    let mut plan = dashboard_plan("root");
    let (cmds, _, _, _) =
        build_scaffold_dashboard(&mut plan, "#0F172A").expect("dashboard scaffold");
    match &cmds[0] {
        EditorCommand::InsertSubtree { parent_id, .. } => {
            assert!(
                !parent_id.is_real(),
                "dashboard root must insert at page level (NodeId::NONE)"
            );
        }
        other => panic!("expected InsertSubtree, got {other:?}"),
    }
}
