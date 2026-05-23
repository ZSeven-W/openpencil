//! Tests for `dashboard_columns` — split from the main module to stay under
//! the 800-line ceiling.  Linked via `#[path]` in `dashboard_columns.rs`.

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

// ---- is_sidebar_subtask ----------------------------------------------------

#[test]
fn sidebar_subtask_true_for_sidebar_navigation() {
    let st = subtask("sidebar-nav", "Sidebar Navigation", None);
    assert!(is_sidebar_subtask(&st));
}

#[test]
fn sidebar_subtask_true_for_nav_in_elements() {
    let st = subtask("left-panel", "Left Panel", Some("nav links, menu items"));
    assert!(is_sidebar_subtask(&st));
}

#[test]
fn sidebar_subtask_false_for_top_bar() {
    // "Top Navigation Bar" does NOT contain "top bar" or "header",
    // so it IS a sidebar — use a proper "Top Bar" example for the false case.
    let st = subtask("top-bar", "Top Bar", None);
    assert!(!is_sidebar_subtask(&st));
}

#[test]
fn sidebar_subtask_false_for_header() {
    let st = subtask("header", "Header", None);
    assert!(!is_sidebar_subtask(&st));
}

#[test]
fn sidebar_subtask_false_for_no_keywords() {
    let st = subtask("hero", "Hero Section", None);
    assert!(!is_sidebar_subtask(&st));
}

// ---- is_main_content_container_subtask -------------------------------------

#[test]
fn main_content_container_true_for_main_content() {
    let st = subtask("main-content", "Main Content", None);
    assert!(is_main_content_container_subtask(&st));
}

#[test]
fn main_content_container_true_for_content_area() {
    let st = subtask("content-area", "Content Area", None);
    assert!(is_main_content_container_subtask(&st));
}

#[test]
fn main_content_container_false_for_metrics_chart() {
    // "main content" keyword present, but "chart" disqualifies
    let st = subtask(
        "main-chart",
        "Main Content",
        Some("metrics chart, revenue data"),
    );
    assert!(!is_main_content_container_subtask(&st));
}

#[test]
fn main_content_container_false_for_table() {
    let st = subtask("data-table", "Data Table", None);
    assert!(!is_main_content_container_subtask(&st));
}

#[test]
fn main_content_container_false_for_sidebar_keyword() {
    let st = subtask("main-sidebar", "Main Content Sidebar", None);
    assert!(!is_main_content_container_subtask(&st));
}

// ---- is_dashboard_like_prompt ----------------------------------------------

#[test]
fn dashboard_like_true_for_analytics_admin_dashboard() {
    let plan = plan_with(1200.0, vec![subtask("hero", "Hero Section", None)]);
    assert!(is_dashboard_like_prompt(
        "an analytics admin dashboard",
        &plan
    ));
}

#[test]
fn dashboard_like_true_from_subtask_text() {
    // prompt has no keyword but a subtask label does
    let plan = plan_with(
        1200.0,
        vec![subtask("analytics-panel", "Analytics Panel", None)],
    );
    assert!(is_dashboard_like_prompt("design a screen", &plan));
}

#[test]
fn dashboard_like_false_for_landing_page() {
    let plan = plan_with(
        1200.0,
        vec![subtask(
            "hero",
            "Hero Section",
            Some("headline, CTA button"),
        )],
    );
    assert!(!is_dashboard_like_prompt(
        "landing page for a startup",
        &plan
    ));
}

// ---- should_use_dashboard_columns ------------------------------------------

fn dashboard_plan() -> OrchestratorPlan {
    plan_with(
        1200.0,
        vec![
            subtask("sidebar", "Sidebar Navigation", Some("nav links")),
            subtask("metrics", "Metrics Row", Some("revenue chart, data table")),
            subtask("transactions", "Transactions", None),
        ],
    )
}

#[test]
fn should_use_dashboard_columns_true_full_conditions() {
    assert!(should_use_dashboard_columns(
        "admin analytics dashboard",
        &dashboard_plan()
    ));
}

#[test]
fn should_use_dashboard_columns_false_width_too_narrow() {
    let mut plan = dashboard_plan();
    plan.root_frame.width = 375.0; // mobile width
    assert!(!should_use_dashboard_columns(
        "admin analytics dashboard",
        &plan
    ));
}

#[test]
fn should_use_dashboard_columns_false_no_dashboard_keyword() {
    // No dashboard-like keyword anywhere — prompt + subtasks
    let plan = plan_with(
        1200.0,
        vec![
            subtask("sidebar", "Sidebar Navigation", Some("nav links")),
            subtask("revenue", "Revenue Chart", None),
        ],
    );
    assert!(!should_use_dashboard_columns(
        "landing page hero section",
        &plan
    ));
}

#[test]
fn should_use_dashboard_columns_false_no_sidebar() {
    // No sidebar subtask
    let plan = plan_with(
        1200.0,
        vec![
            subtask("header", "Header", None),
            subtask("metrics", "Metrics Row", Some("revenue chart, table")),
        ],
    );
    assert!(!should_use_dashboard_columns("admin dashboard", &plan));
}

#[test]
fn should_use_dashboard_columns_false_no_data_panel() {
    // Has sidebar but no metric/chart/table subtask
    let plan = plan_with(
        1200.0,
        vec![
            subtask("sidebar", "Sidebar Navigation", Some("nav links")),
            subtask("hero", "Hero Section", Some("headline, CTA")),
        ],
    );
    assert!(!should_use_dashboard_columns("admin dashboard", &plan));
}

#[test]
fn should_use_dashboard_columns_false_width_exactly_480() {
    // width must be STRICTLY > 480
    let mut plan = dashboard_plan();
    plan.root_frame.width = 480.0;
    assert!(!should_use_dashboard_columns(
        "admin analytics dashboard",
        &plan
    ));
}

// ---- infer_dashboard_section_height ----------------------------------------

#[test]
fn section_height_sidebar_is_760() {
    let st = subtask("sidebar-nav", "Sidebar Navigation", None);
    assert_eq!(infer_dashboard_section_height(&st), 760.0);
}

#[test]
fn section_height_header_is_96() {
    let st = subtask("top-bar", "Top Bar", None);
    assert_eq!(infer_dashboard_section_height(&st), 96.0);
}

#[test]
fn section_height_metric_is_160() {
    let st = subtask("metrics", "Metrics Row", None);
    assert_eq!(infer_dashboard_section_height(&st), 160.0);
}

#[test]
fn section_height_kpi_is_160() {
    let st = subtask("kpi-row", "KPI Overview", None);
    assert_eq!(infer_dashboard_section_height(&st), 160.0);
}

#[test]
fn section_height_chart_is_320() {
    let st = subtask("revenue-chart", "Revenue Chart", None);
    assert_eq!(infer_dashboard_section_height(&st), 320.0);
}

#[test]
fn section_height_transaction_is_320() {
    let st = subtask("transactions", "Transactions Feed", None);
    assert_eq!(infer_dashboard_section_height(&st), 320.0);
}

#[test]
fn section_height_activity_is_320() {
    let st = subtask("activity-log", "Activity Log", None);
    assert_eq!(infer_dashboard_section_height(&st), 320.0);
}

#[test]
fn section_height_feed_is_320() {
    let st = subtask("news-feed", "News Feed", None);
    assert_eq!(infer_dashboard_section_height(&st), 320.0);
}

#[test]
fn section_height_table_is_340() {
    let st = subtask("data-table", "Data Table", None);
    assert_eq!(infer_dashboard_section_height(&st), 340.0);
}

#[test]
fn section_height_analytics_is_340() {
    let st = subtask("analytics", "Analytics Panel", None);
    assert_eq!(infer_dashboard_section_height(&st), 340.0);
}

#[test]
fn section_height_customer_is_340() {
    let st = subtask("customers", "Customer List", None);
    assert_eq!(infer_dashboard_section_height(&st), 340.0);
}

#[test]
fn section_height_default_is_160() {
    let st = subtask("hero", "Hero Section", None);
    assert_eq!(infer_dashboard_section_height(&st), 160.0);
}

// ---- infer_dashboard_section_width -----------------------------------------

#[test]
fn section_width_sidebar_returns_260() {
    let st = subtask("sidebar-nav", "Sidebar Navigation", None);
    assert_eq!(infer_dashboard_section_width(&st, 1200.0), 260.0);
}

#[test]
fn section_width_chart_returns_62_percent_of_main() {
    // main_width = max(320, 1200 - 260) = 940
    // 940 * 0.62 = 582.8 → rounds to 583
    let st = subtask("revenue-chart", "Revenue Chart", None);
    let expected = (940.0_f64 * 0.62).round();
    assert_eq!(infer_dashboard_section_width(&st, 1200.0), expected);
}

#[test]
fn section_width_transaction_returns_38_percent_of_main() {
    // main_width = 940
    // 940 * 0.38 = 357.2 → rounds to 357
    let st = subtask("transactions", "Transactions", None);
    let expected = (940.0_f64 * 0.38).round();
    assert_eq!(infer_dashboard_section_width(&st, 1200.0), expected);
}

#[test]
fn section_width_other_returns_full_main_width() {
    // main_width = 940
    let st = subtask("hero", "Hero Section", None);
    assert_eq!(infer_dashboard_section_width(&st, 1200.0), 940.0);
}

#[test]
fn section_width_main_width_floor_is_320() {
    // root_width = 400; 400 - 260 = 140 < 320 → main_width = 320
    let st = subtask("hero", "Hero Section", None);
    assert_eq!(infer_dashboard_section_width(&st, 400.0), 320.0);
}

#[test]
fn section_width_activity_returns_38_percent_of_main() {
    let st = subtask("activity-log", "Activity Log", None);
    let expected = (940.0_f64 * 0.38).round();
    assert_eq!(infer_dashboard_section_width(&st, 1200.0), expected);
}

// ---- extract_sidebar_surface_color -----------------------------------------

#[test]
fn sidebar_color_catalog_table_match() {
    let content = "| Sidebar Surface | #1E2A3B | Dark slate |\n| Background | #FFFFFF |";
    let result = extract_sidebar_surface_color(Some(content), None);
    assert_eq!(result.as_deref(), Some("#1E2A3B"));
}

#[test]
fn sidebar_color_catalog_inline_match() {
    let content = "Sidebar Surface color is #2D3748 (used for the sidebar background)";
    let result = extract_sidebar_surface_color(Some(content), None);
    assert_eq!(result.as_deref(), Some("#2D3748"));
}

#[test]
fn sidebar_color_design_md_sidebar_role() {
    let spec = jian_ops_schema::DesignMdSpec {
        raw: String::new(),
        project_name: None,
        visual_theme: None,
        color_palette: Some(vec![
            jian_ops_schema::DesignMdColor {
                name: "Sidebar".into(),
                hex: "#0F172A".into(),
                role: "sidebar background".into(),
            },
            jian_ops_schema::DesignMdColor {
                name: "Primary".into(),
                hex: "#3366FF".into(),
                role: "buttons and links".into(),
            },
        ]),
        typography: None,
        component_styles: None,
        layout_principles: None,
        generation_notes: None,
    };
    let result = extract_sidebar_surface_color(None, Some(&spec));
    assert_eq!(result.as_deref(), Some("#0F172A"));
}

#[test]
fn sidebar_color_design_md_panel_role_fallback() {
    let spec = jian_ops_schema::DesignMdSpec {
        raw: String::new(),
        project_name: None,
        visual_theme: None,
        color_palette: Some(vec![jian_ops_schema::DesignMdColor {
            name: "Panel".into(),
            hex: "#1A2035".into(),
            role: "panel background".into(),
        }]),
        typography: None,
        component_styles: None,
        layout_principles: None,
        generation_notes: None,
    };
    let result = extract_sidebar_surface_color(None, Some(&spec));
    assert_eq!(result.as_deref(), Some("#1A2035"));
}

#[test]
fn sidebar_color_design_md_surface_role_fallback() {
    let spec = jian_ops_schema::DesignMdSpec {
        raw: String::new(),
        project_name: None,
        visual_theme: None,
        color_palette: Some(vec![jian_ops_schema::DesignMdColor {
            name: "Surface".into(),
            hex: "#161B22".into(),
            role: "surface and card".into(),
        }]),
        typography: None,
        component_styles: None,
        layout_principles: None,
        generation_notes: None,
    };
    let result = extract_sidebar_surface_color(None, Some(&spec));
    assert_eq!(result.as_deref(), Some("#161B22"));
}

#[test]
fn sidebar_color_returns_none_when_no_match() {
    // No catalog content, no design.md → None (caller applies fallback)
    let result = extract_sidebar_surface_color(None, None);
    assert!(result.is_none());
}

#[test]
fn sidebar_color_uppercase_hex_output() {
    // Even if input is lowercase, output must be uppercase
    let content = "Sidebar Surface | #1e2a3b";
    let result = extract_sidebar_surface_color(Some(content), None);
    assert_eq!(result.as_deref(), Some("#1E2A3B"));
}

// B-series tests moved to `dashboard_columns_tests_b.rs` to stay under the
// 800-line ceiling.  Linked via `#[path]` in `dashboard_columns.rs`.
