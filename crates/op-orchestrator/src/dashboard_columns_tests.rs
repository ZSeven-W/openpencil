//! Tests for `dashboard_columns` — the normalizer-facing detection predicates
//! plus section sizing. The tests for the removed bespoke-scaffold helpers
//! `should_use_dashboard_columns`, `is_main_content_container_subtask`,
//! `extract_sidebar_surface_color`, and the row bin-packing were removed with
//! those functions.

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
        insert_after_sibling_id: None,
        elements: elements.map(String::from),
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
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
fn sidebar_subtask_false_for_navigation_bar() {
    let st = subtask("nav", "Navigation Bar", None);
    assert!(!is_sidebar_subtask(&st));
}

#[test]
fn sidebar_subtask_true_for_sidebar_navigation_bar() {
    let st = subtask("sidebar", "Sidebar Navigation Bar", None);
    assert!(is_sidebar_subtask(&st));
}

#[test]
fn sidebar_subtask_false_for_footer_navigation_links() {
    let st = subtask(
        "cta",
        "Final CTA & Footer",
        Some("conversion CTA, footer navigation links, legal links"),
    );
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

#[test]
fn explicit_landing_page_intent_vetoes_dashboard_intent() {
    let plan = plan_with(
        1440.0,
        vec![subtask(
            "analytics",
            "Analytics Dashboard",
            Some("KPI cards and revenue chart"),
        )],
    );
    assert!(!is_dashboard_like_prompt(
        "Design a landing-page for an analytics dashboard",
        &plan
    ));
}

#[test]
fn explicit_dashboard_intent_wins_over_landing_anatomy() {
    let plan = plan_with(
        1440.0,
        vec![
            subtask("hero", "Hero Summary", Some("analytics KPIs")),
            subtask("workflow", "Workflow", Some("operations activity")),
            subtask("footer", "Footer", Some("workspace links")),
        ],
    );
    assert!(plan_has_landing_anatomy(&plan));
    assert!(is_dashboard_like_prompt(
        "Design an analytics dashboard for a growth team",
        &plan
    ));
}

#[test]
fn explicit_admin_console_intent_wins_over_landing_anatomy() {
    let plan = plan_with(
        1440.0,
        vec![
            subtask("hero", "Hero Summary", Some("account status")),
            subtask("workflow", "Workflow", Some("support queue")),
            subtask("footer", "Footer", Some("admin links")),
        ],
    );
    assert!(plan_has_landing_anatomy(&plan));
    assert!(is_dashboard_like_prompt(
        "Design an admin-console for support operations",
        &plan
    ));
}

#[test]
fn dashboard_like_false_for_landing_page_with_data_sections() {
    let plan = plan_with(
        1440.0,
        vec![
            subtask("nav", "Navigation Bar", Some("main navigation links")),
            subtask("hero", "Hero Section", Some("headline and product visual")),
            subtask(
                "capabilities",
                "Capability Stories",
                Some("live node graph"),
            ),
            subtask("proof", "Customer Proof", Some("three key metrics cards")),
            subtask("faq", "FAQ", Some("data privacy guarantees")),
        ],
    );
    assert!(!is_dashboard_like_prompt(
        "Design a responsive website for an AI workbench",
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
