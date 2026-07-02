//! B2 tests for `dashboard_columns::slots::assign_dashboard_main_parents`.
//! Split from `dashboard_columns_tests_b.rs` to keep both files under 800 lines.
//! Linked via `#[path]` in `dashboard_columns.rs`.

use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};
use jian_ops_schema::node::PenNode;

// ---- helpers ---------------------------------------------------------------

fn root(width: f64) -> RootFrameSpec {
    RootFrameSpec {
        id: "root".into(),
        name: "Design".into(),
        width,
        height: 800.0,
        layout: Some("vertical".into()),
        gap: Some(0.0),
        padding: Some(0.0),
        fill: None,
    }
}

fn subtask(id: &str, label: &str, elements: Option<&str>) -> Subtask {
    Subtask {
        id: id.into(),
        label: label.into(),
        region: Region {
            width: 1200.0,
            height: 300.0,
        },
        id_prefix: id.into(),
        parent_frame_id: None,
        elements: elements.map(String::from),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
    }
}

fn plan_with(width: f64, subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: root(width),
        subtasks,
        style_guide_name: None,
    }
}

// ---- assign_dashboard_main_parents — B2 tests ------------------------------

/// Helper: pull the PenNode base id_str from a PenNode (must be a frame).
fn node_id(node: &PenNode) -> &str {
    match node {
        PenNode::Frame(f) => &f.base.id,
        other => panic!("expected frame node, got: {other:?}"),
    }
}

/// Helper: pull the layout field from a PenNode frame as a static str.
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

/// Helper: check whether a frame node has `fill_container` width.
fn is_fill_container_width(node: &PenNode) -> bool {
    use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
    match node {
        PenNode::Frame(f) => matches!(
            f.container.width,
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        ),
        _ => false,
    }
}

/// Helper: pull numeric width from a frame node (None if keyword/missing).
fn node_width_px(node: &PenNode) -> Option<f64> {
    use jian_ops_schema::sizing::SizingBehavior;
    match node {
        PenNode::Frame(f) => match &f.container.width {
            Some(SizingBehavior::Number(n)) => Some(*n),
            _ => None,
        },
        _ => None,
    }
}

/// Helper: pull the gap from a frame node (numeric only).
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

// ---- 1-subtask row: a single fill_container Slot frame ---------------------

#[test]
fn assign_single_subtask_row_produces_fill_container_slot() {
    // Plan: 1 sidebar + 1 main subtask → 1-row, 1-subtask.
    // Expected: a fill_container Slot frame for the subtask,
    //           no Dashboard Row wrapper.
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut topbar = subtask("top-bar", "Top Bar", None);
    topbar.region = Region {
        width: 940.0,
        height: 96.0,
    };
    let mut plan = plan_with(1200.0, vec![sidebar, topbar]);

    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    // Single-subtask row → exactly 1 synthesized frame (the slot).
    assert_eq!(frames.len(), 1, "expected 1 slot frame for 1-subtask row");
    let (node, parent_id) = &frames[0];
    assert_eq!(
        parent_id, main_id,
        "slot frame parent must be the main column id"
    );
    // The slot must be fill_container width.
    assert!(
        is_fill_container_width(node),
        "single-subtask slot must have fill_container width"
    );
    // Layout: vertical.
    assert_eq!(node_layout(node), Some("vertical"));
    // Slot id format: `{main_id}-row-{n}-{subtask_id}-slot`
    let slot_id = node_id(node);
    assert!(
        slot_id.starts_with(main_id),
        "slot id must start with main_id: {slot_id}"
    );
    assert!(
        slot_id.ends_with("-slot"),
        "slot id must end with '-slot': {slot_id}"
    );
    assert!(
        slot_id.contains("top-bar"),
        "slot id must contain subtask id 'top-bar': {slot_id}"
    );
}

#[test]
fn assign_single_subtask_row_sets_parent_frame_id_on_subtask() {
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut topbar = subtask("top-bar", "Top Bar", None);
    topbar.region = Region {
        width: 940.0,
        height: 96.0,
    };
    let mut plan = plan_with(1200.0, vec![sidebar, topbar]);
    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    // The subtask's parent_frame_id must equal the slot id.
    let slot_id = node_id(&frames[0].0);
    let topbar_st = plan.subtasks.iter().find(|st| st.id == "top-bar").unwrap();
    assert_eq!(
        topbar_st.parent_frame_id.as_deref(),
        Some(slot_id),
        "subtask parent_frame_id must equal slot id"
    );
}

// ---- N-subtask row: horizontal Dashboard Row + N Slot frames ---------------

#[test]
fn assign_three_subtask_row_produces_row_plus_slots() {
    // 3 narrow subtasks pack into a single row with root_width=1400.
    // full_width = max(1400-260=1140, 340) = 1140
    // All 3 narrow: 300, 300, 340 (< 1140*0.82=935).
    // 300+24+300=624 < 1140*0.92=1048.8 → no post-flush after 2nd
    // 624+24+340=988 < 1048.8 → no post-flush after 3rd → end flush → 1 row [a,b,c].
    // Expected: 1 Dashboard Row + 3 Slots = 4 frames.
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut s1 = subtask("section-a", "Section A", None);
    s1.region = Region {
        width: 300.0,
        height: 200.0,
    };
    let mut s2 = subtask("section-b", "Section B", None);
    s2.region = Region {
        width: 300.0,
        height: 200.0,
    };
    let mut s3 = subtask("section-c", "Section C", None);
    s3.region = Region {
        width: 340.0,
        height: 200.0,
    };
    let mut plan = plan_with(1400.0, vec![sidebar, s1, s2, s3]);

    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    // Expect 4 frames: 1 row + 3 slots.
    assert_eq!(
        frames.len(),
        4,
        "expected 1 Dashboard Row + 3 Slot frames = 4, got {}",
        frames.len()
    );
}

#[test]
fn assign_multi_subtask_row_row_frame_is_horizontal() {
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut s1 = subtask("section-a", "Section A", None);
    s1.region = Region {
        width: 300.0,
        height: 200.0,
    };
    let mut s2 = subtask("section-b", "Section B", None);
    s2.region = Region {
        width: 300.0,
        height: 200.0,
    };
    let mut plan = plan_with(1400.0, vec![sidebar, s1, s2]);

    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    // The row frame (parent_id = main_id, not ending in -slot) must be horizontal.
    let row_frame = frames
        .iter()
        .find(|(node, pid)| pid == main_id && !node_id(node).ends_with("-slot"))
        .expect("Dashboard Row frame not found");
    assert_eq!(
        node_layout(&row_frame.0),
        Some("horizontal"),
        "Dashboard Row must have horizontal layout"
    );
}

#[test]
fn assign_multi_subtask_row_last_slot_is_fill_container() {
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut s1 = subtask("section-a", "Section A", None);
    s1.region = Region {
        width: 300.0,
        height: 200.0,
    };
    let mut s2 = subtask("section-b", "Section B", None);
    s2.region = Region {
        width: 300.0,
        height: 200.0,
    };
    let mut plan = plan_with(1400.0, vec![sidebar, s1, s2]);

    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    let slot_frames: Vec<_> = frames
        .iter()
        .filter(|(node, _)| node_id(node).ends_with("-slot"))
        .collect();
    assert_eq!(slot_frames.len(), 2, "expected 2 slot frames");

    // The LAST slot must be fill_container.
    let last_slot = slot_frames.last().unwrap();
    assert!(
        is_fill_container_width(&last_slot.0),
        "last slot must have fill_container width"
    );
}

#[test]
fn assign_multi_subtask_slot_widths_proportional_min_220() {
    // 2-subtask row: section-a (width 400), section-b (width 400).
    // root_width = 1200 → full_width = max(940, 400) = 940
    // row_gap = 24; slot_gap = 24 * (2-1) = 24
    // available_width = max(320, 940 - 24) = 916
    // width_sum = 400 + 400 = 800
    // proportional for section-a: max(220, round(916 * 400 / 800)) = max(220, 458) = 458
    // idx=0: remaining_after=1; slotWidth = min(916-0-220, 458) = min(696, 458) = 458
    // section-b (last): fill_container
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut s1 = subtask("section-a", "Section A", None);
    s1.region = Region {
        width: 400.0,
        height: 200.0,
    };
    let mut s2 = subtask("section-b", "Section B", None);
    s2.region = Region {
        width: 400.0,
        height: 200.0,
    };
    let mut plan = plan_with(1200.0, vec![sidebar, s1, s2]);

    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    let slot_frames: Vec<_> = frames
        .iter()
        .filter(|(node, _)| node_id(node).ends_with("-slot"))
        .collect();
    assert_eq!(slot_frames.len(), 2);

    // First slot must have numeric width.
    let first_slot_width =
        node_width_px(&slot_frames[0].0).expect("first slot must have numeric width");
    assert!(
        first_slot_width >= 220.0,
        "slot width must be >= 220 (min floor), got {first_slot_width}"
    );
    // Specific proportional check: available=916, widthSum=800, proportional=458
    // clamped = min(916-0-220, 458) = min(696, 458) = 458
    assert_eq!(
        first_slot_width, 458.0,
        "first slot width should be 458 per proportional formula"
    );
    // Last slot is fill_container.
    assert!(
        is_fill_container_width(&slot_frames[1].0),
        "second (last) slot must be fill_container"
    );
}

#[test]
fn assign_slot_ids_match_expected_pattern() {
    // Row 1: 1-subtask (standalone top bar): id = "{main}-row-1-top-bar-slot"
    // Row 2: 2-subtask row: row id = "{main}-row-2",
    //        slot ids = "{main}-row-2-revenue-chart-slot" / "{main}-row-2-transactions-slot"
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut topbar = subtask("top-bar", "Top Bar", None);
    topbar.region = Region {
        width: 940.0,
        height: 96.0,
    };
    let mut chart = subtask("revenue-chart", "Revenue Chart", None);
    chart.region = Region {
        width: 583.0,
        height: 320.0,
    };
    let mut transactions = subtask("transactions", "Transactions", None);
    transactions.region = Region {
        width: 357.0,
        height: 320.0,
    };
    let mut plan = plan_with(1200.0, vec![sidebar, topbar, chart, transactions]);

    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    // Row 1 (1-subtask): slot id = "root-main-row-1-top-bar-slot"
    let slot1 = frames
        .iter()
        .find(|(node, _)| node_id(node) == "root-main-row-1-top-bar-slot");
    assert!(slot1.is_some(), "expected 'root-main-row-1-top-bar-slot'");

    // Row 2 (2-subtask): row id = "root-main-row-2"
    let row2 = frames
        .iter()
        .find(|(node, _)| node_id(node) == "root-main-row-2");
    assert!(row2.is_some(), "expected 'root-main-row-2'");

    // Row 2 slot ids:
    let slot_chart = frames
        .iter()
        .find(|(node, _)| node_id(node) == "root-main-row-2-revenue-chart-slot");
    let slot_tx = frames
        .iter()
        .find(|(node, _)| node_id(node) == "root-main-row-2-transactions-slot");
    assert!(
        slot_chart.is_some(),
        "expected 'root-main-row-2-revenue-chart-slot'"
    );
    assert!(
        slot_tx.is_some(),
        "expected 'root-main-row-2-transactions-slot'"
    );
}

#[test]
fn assign_row_frame_has_row_gap() {
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut s1 = subtask("section-a", "Section A", None);
    s1.region = Region {
        width: 300.0,
        height: 200.0,
    };
    let mut s2 = subtask("section-b", "Section B", None);
    s2.region = Region {
        width: 300.0,
        height: 200.0,
    };
    let mut plan = plan_with(1200.0, vec![sidebar, s1, s2]);
    plan.root_frame.gap = Some(16.0); // row_gap = 16

    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    let row_frame = frames
        .iter()
        .find(|(node, pid)| pid == main_id && !node_id(node).ends_with("-slot"))
        .expect("Dashboard Row frame not found");
    assert_eq!(
        node_gap(&row_frame.0),
        Some(16.0),
        "row frame gap must equal row_gap"
    );
}

#[test]
fn assign_empty_rows_returns_empty() {
    // Only sidebar → no rows → no synthesized frames.
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut plan = plan_with(1200.0, vec![sidebar]);
    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");
    assert!(frames.is_empty(), "empty rows → no synthesized frames");
}

#[test]
fn assign_available_width_floor_is_320() {
    // full_width = max(340-260=80, 200) = 200; slot_gap = 24*(2-1)=24
    // available_width = max(320, 200-24) = max(320, 176) = 320.
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut s1 = subtask("narrow-a", "Narrow A", None);
    s1.region = Region {
        width: 200.0,
        height: 100.0,
    };
    let mut s2 = subtask("narrow-b", "Narrow B", None);
    s2.region = Region {
        width: 200.0,
        height: 100.0,
    };
    let mut plan = plan_with(340.0, vec![sidebar, s1, s2]);

    let main_id = "root-main";
    let frames =
        assign_dashboard_main_parents(&mut plan, main_id).expect("assign_dashboard_main_parents");

    // Should produce row + 2 slots without panic.
    assert!(
        !frames.is_empty(),
        "should produce frames even with narrow available_width"
    );
    let slot_frames: Vec<_> = frames
        .iter()
        .filter(|(node, _)| node_id(node).ends_with("-slot"))
        .collect();
    assert_eq!(slot_frames.len(), 2);
    // First slot must be >= 220 (min floor).
    if let Some(w) = node_width_px(&slot_frames[0].0) {
        assert!(w >= 220.0, "slot width floor 220 must be respected: {w}");
    }
    // Last slot is fill_container.
    assert!(is_fill_container_width(&slot_frames[1].0));
}
