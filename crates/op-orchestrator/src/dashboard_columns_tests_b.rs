//! B1 tests for `dashboard_columns` — `normalize_dashboard_main_subtasks` +
//! `group_dashboard_main_rows`.  B2 tests live in `dashboard_columns_tests_b2.rs`.
//! Linked via `#[path]` in `dashboard_columns.rs`.

use super::*;
use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

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
    }
}

fn plan_with(width: f64, subtasks: Vec<Subtask>) -> OrchestratorPlan {
    OrchestratorPlan {
        root_frame: root(width),
        subtasks,
        style_guide_name: None,
    }
}

// ---- normalize_dashboard_main_subtasks -------------------------------------

/// Build a plan with a mix of sidebar + main subtasks for normalisation tests.
///
/// The container subtask uses label "Main Content" with elements that do NOT
/// contain data-panel keywords (metric/chart/table/etc.), so that
/// `is_main_content_container_subtask` returns `true` for it.
///
/// NOTE: `is_main_content_container_subtask` checks the COMBINED
/// `id + label + elements` text and returns false when any data-panel keyword
/// (e.g. "metric") appears.  Elements containing "metrics row" would be
/// disqualified.  The `normalize_strips_metrics_row_from_elements` test
/// therefore verifies the top-bar fragment extraction path instead.
fn norm_plan() -> OrchestratorPlan {
    let mut sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    sidebar.region = Region {
        width: 260.0,
        height: 760.0,
    };

    // Elements: no data-panel keywords → container qualifies.
    let mut container = subtask(
        "main-content",
        "Main Content",
        Some("Top bar with page title, date range selector, export button"),
    );
    container.region = Region {
        width: 940.0,
        height: 200.0,
    };

    let metrics = subtask("metrics", "Metrics Row", Some("kpi cards, revenue data"));
    let mut chart = subtask("revenue-chart", "Revenue Chart", None);
    chart.region = Region {
        width: 940.0,
        height: 320.0,
    };

    plan_with(1200.0, vec![sidebar, container, metrics, chart])
}

#[test]
fn normalize_relabels_main_content_to_top_bar() {
    let mut plan = norm_plan();
    normalize_dashboard_main_subtasks(&mut plan);
    let top_bar = plan.subtasks.iter().find(|st| st.id == "main-content");
    assert!(top_bar.is_some());
    assert_eq!(top_bar.unwrap().label, "Top Bar");
}

#[test]
fn normalize_strips_metrics_row_from_elements() {
    // Verifies the top-bar fragment extraction path.
    // norm_plan container elements = "Top bar with page title, date range selector, export button"
    // → after normalise, elements should still contain "top bar" (fragment extracted).
    let mut plan = norm_plan();
    normalize_dashboard_main_subtasks(&mut plan);
    let top_bar = plan
        .subtasks
        .iter()
        .find(|st| st.id == "main-content")
        .unwrap();
    let elems = top_bar.elements.as_deref().unwrap_or("");
    assert!(
        elems.to_lowercase().contains("top bar"),
        "expected 'top bar' in elements, got: {elems}"
    );
}

#[test]
fn normalize_clamps_height_to_88_floor() {
    let mut plan = norm_plan();
    // Set container height below floor
    let container = plan
        .subtasks
        .iter_mut()
        .find(|st| st.id == "main-content")
        .unwrap();
    container.region.height = 50.0;
    normalize_dashboard_main_subtasks(&mut plan);
    let top_bar = plan
        .subtasks
        .iter()
        .find(|st| st.id == "main-content")
        .unwrap();
    assert_eq!(top_bar.region.height, 88.0);
}

#[test]
fn normalize_clamps_height_to_120_ceiling() {
    let mut plan = norm_plan();
    let container = plan
        .subtasks
        .iter_mut()
        .find(|st| st.id == "main-content")
        .unwrap();
    container.region.height = 300.0;
    normalize_dashboard_main_subtasks(&mut plan);
    let top_bar = plan
        .subtasks
        .iter()
        .find(|st| st.id == "main-content")
        .unwrap();
    assert_eq!(top_bar.region.height, 120.0);
}

#[test]
fn normalize_preserves_in_range_height() {
    let mut plan = norm_plan();
    let container = plan
        .subtasks
        .iter_mut()
        .find(|st| st.id == "main-content")
        .unwrap();
    container.region.height = 100.0;
    normalize_dashboard_main_subtasks(&mut plan);
    let top_bar = plan
        .subtasks
        .iter()
        .find(|st| st.id == "main-content")
        .unwrap();
    assert_eq!(top_bar.region.height, 100.0);
}

#[test]
fn normalize_noop_when_fewer_than_two_main_subtasks() {
    // Only one non-sidebar subtask → no normalisation
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let container = subtask(
        "main-content",
        "Main Content",
        Some("Top bar with title, metrics row"),
    );
    let mut plan = plan_with(1200.0, vec![sidebar, container]);
    normalize_dashboard_main_subtasks(&mut plan);
    let container_after = plan
        .subtasks
        .iter()
        .find(|st| st.id == "main-content")
        .unwrap();
    // label should NOT be changed (only 1 main subtask)
    assert_eq!(container_after.label, "Main Content");
}

#[test]
fn normalize_noop_when_no_container_subtask() {
    // No is_main_content_container_subtask in the plan → noop
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let metrics = subtask("metrics", "Metrics Row", Some("kpi cards"));
    let chart = subtask("revenue-chart", "Revenue Chart", None);
    let mut plan = plan_with(1200.0, vec![sidebar, metrics, chart]);
    normalize_dashboard_main_subtasks(&mut plan);
    // Nothing was renamed
    assert!(plan.subtasks.iter().all(|st| st.label != "Top Bar"));
}

// ---- group_dashboard_main_rows ---------------------------------------------

/// Build a plan whose main-subtask widths exercise the bin-packer.
fn group_plan() -> OrchestratorPlan {
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));

    let mut topbar = subtask("top-bar", "Top Bar", None);
    topbar.region = Region {
        width: 940.0,
        height: 96.0,
    }; // full-width (>= 0.82*940)

    let mut chart = subtask("revenue-chart", "Revenue Chart", None);
    chart.region = Region {
        width: 583.0,
        height: 320.0,
    }; // ~62% → narrow

    let mut transactions = subtask("transactions", "Transactions", None);
    transactions.region = Region {
        width: 357.0,
        height: 320.0,
    }; // ~38% → narrow

    let mut table = subtask("customers", "Customer Table", None);
    table.region = Region {
        width: 940.0,
        height: 340.0,
    }; // full-width

    plan_with(1200.0, vec![sidebar, topbar, chart, transactions, table])
}

#[test]
fn group_full_width_subtask_gets_own_row() {
    let plan = group_plan();
    let result = group_dashboard_main_rows(&plan);
    // "Top Bar" (width 940 >= 940*0.82=770) is standalone → own row
    let topbar_row = result
        .rows
        .iter()
        .find(|row| row.iter().any(|&i| plan.subtasks[i].id == "top-bar"));
    assert!(topbar_row.is_some());
    assert_eq!(topbar_row.unwrap().len(), 1);
}

#[test]
fn group_narrow_subtasks_pack_into_shared_row() {
    let plan = group_plan();
    let result = group_dashboard_main_rows(&plan);
    // "Revenue Chart" (583) + "Transactions" (357) → cumulative 583+24+357=964 >= 940*0.92=864.8
    // They should be packed into the same row
    let chart_row = result
        .rows
        .iter()
        .find(|row| row.iter().any(|&i| plan.subtasks[i].id == "revenue-chart"));
    let tx_row = result
        .rows
        .iter()
        .find(|row| row.iter().any(|&i| plan.subtasks[i].id == "transactions"));
    assert!(chart_row.is_some());
    assert!(tx_row.is_some());
    assert_eq!(
        chart_row.unwrap().as_ptr(),
        tx_row.unwrap().as_ptr(),
        "chart and transactions should be in the same row"
    );
}

#[test]
fn group_full_width_table_gets_own_row() {
    let plan = group_plan();
    let result = group_dashboard_main_rows(&plan);
    let table_row = result
        .rows
        .iter()
        .find(|row| row.iter().any(|&i| plan.subtasks[i].id == "customers"));
    assert!(table_row.is_some());
    assert_eq!(table_row.unwrap().len(), 1);
}

#[test]
fn group_full_width_is_max_of_root_minus_260_and_subtask_widths() {
    let plan = group_plan();
    let result = group_dashboard_main_rows(&plan);
    // root.width = 1200; 1200 - 260 = 940; max subtask width is also 940
    assert_eq!(result.full_width, 940.0);
}

#[test]
fn group_row_gap_uses_root_gap_when_positive() {
    let mut plan = group_plan();
    plan.root_frame.gap = Some(16.0);
    let result = group_dashboard_main_rows(&plan);
    assert_eq!(result.row_gap, 16.0);
}

#[test]
fn group_row_gap_defaults_to_24_when_gap_zero() {
    let mut plan = group_plan();
    plan.root_frame.gap = Some(0.0);
    let result = group_dashboard_main_rows(&plan);
    assert_eq!(result.row_gap, 24.0);
}

#[test]
fn group_row_gap_defaults_to_24_when_gap_none() {
    let mut plan = group_plan();
    plan.root_frame.gap = None;
    let result = group_dashboard_main_rows(&plan);
    assert_eq!(result.row_gap, 24.0);
}

#[test]
fn group_main_content_container_is_always_standalone() {
    // Even if the main-content-container subtask has a narrow width,
    // it should be placed in its own row.
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut container = subtask("main-content", "Main Content", None);
    // narrow: only 300px (< full_width * 0.82)
    container.region = Region {
        width: 300.0,
        height: 96.0,
    };
    let mut chart = subtask("revenue-chart", "Revenue Chart", None);
    chart.region = Region {
        width: 583.0,
        height: 320.0,
    };
    let plan = plan_with(1200.0, vec![sidebar, container, chart]);
    let result = group_dashboard_main_rows(&plan);
    // The main-content-container row should have exactly 1 subtask
    let container_row = result
        .rows
        .iter()
        .find(|row| row.iter().any(|&i| plan.subtasks[i].id == "main-content"));
    assert!(container_row.is_some());
    assert_eq!(container_row.unwrap().len(), 1);
}

#[test]
fn group_empty_when_no_main_subtasks() {
    // Only sidebar subtasks → no rows
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let plan = plan_with(1200.0, vec![sidebar]);
    let result = group_dashboard_main_rows(&plan);
    assert!(result.rows.is_empty());
}

#[test]
fn group_rows_contain_subtask_indices_into_plan() {
    let plan = group_plan();
    let result = group_dashboard_main_rows(&plan);
    // Every index in rows must be a valid index into plan.subtasks, and must NOT be a sidebar
    for row in &result.rows {
        for &idx in row {
            let st = &plan.subtasks[idx];
            assert!(
                !is_sidebar_subtask(st),
                "row must not contain sidebar subtask: {}",
                st.id
            );
        }
    }
}

#[test]
fn group_pre_flush_when_would_exceed_1_05() {
    // full_width = max(1200-260=940, 400) = 940
    // a(400) + b(400): cumulative = 400+24+400 = 824 < 940*0.92=864.8 → no post-flush
    // before c: nextWidth = 824+24+400 = 1248 > 940*1.05=987 → pre-flush → row[a,b]
    // c alone → row[c] at end flush
    let sidebar = subtask("sidebar-nav", "Sidebar Navigation", Some("nav links"));
    let mut a = subtask("a", "Section A", None);
    a.region = Region {
        width: 400.0,
        height: 200.0,
    };
    let mut b = subtask("b", "Section B", None);
    b.region = Region {
        width: 400.0,
        height: 200.0,
    };
    let mut c = subtask("c", "Section C", None);
    c.region = Region {
        width: 400.0,
        height: 200.0,
    };
    let plan = plan_with(1200.0, vec![sidebar, a, b, c]);
    let result = group_dashboard_main_rows(&plan);
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.rows[0].len(), 2); // a + b
    assert_eq!(result.rows[1].len(), 1); // c
}

// B2 tests moved to `dashboard_columns_tests_b2.rs` (800-line cap).

// The following comment is intentionally left to mark the end of B1 content.
// B2 tests (`assign_dashboard_main_parents`) are in `dashboard_columns_tests_b2.rs`.

/// Placeholder to prevent empty tail — B1 tests above, B2 in sibling file.
fn _b1_end() {}
