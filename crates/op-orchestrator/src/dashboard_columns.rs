//! Dashboard column-layout detection predicates —— S3b-3 Plan A1.
//!
//! Faithful port of `apps/web/src/services/ai/orchestrator.ts:175-211`.
//! Callers land in Plan C (wiring); all items are `pub(crate)` with
//! `#![allow(dead_code)]` scaffolding acceptable until then.

#![allow(dead_code)]

use crate::plan::{OrchestratorPlan, Subtask};

// ---------------------------------------------------------------------------
// §4.1 Detection predicates
// ---------------------------------------------------------------------------

/// Returns `true` when the subtask represents a sidebar / navigation panel.
///
/// Matches `/(sidebar|side bar|navigation|nav|menu)/` over
/// `id + label + elements` (joined, lowercased), AND NOT `/(top bar|header)/`.
///
/// Port of TS `isSidebarSubtask` (`orchestrator.ts:175-180`).
pub(crate) fn is_sidebar_subtask(st: &Subtask) -> bool {
    let text = subtask_text(st);
    has_sidebar_keyword(&text) && !has_topbar_keyword(&text)
}

/// Returns `true` when the subtask is the main-content container placeholder
/// that the orchestrator normalises away.
///
/// Matches `/(main content|content area|main area|content column)/` AND NOT
/// `/(metric|chart|table|transaction|customer|analytics|revenue|growth|sidebar)/`.
///
/// Port of TS `isMainContentContainerSubtask` (`orchestrator.ts:182-190`).
pub(crate) fn is_main_content_container_subtask(st: &Subtask) -> bool {
    let text = subtask_text(st);
    has_main_content_keyword(&text) && !has_data_panel_keyword(&text)
}

/// Returns `true` when the prompt + plan subtasks suggest a dashboard-like
/// design (desktop data-heavy screen).
///
/// Matches `/(dashboard|admin|analytics|fintech|workspace|data)/` over
/// `prompt` concatenated with every subtask's `id + label + elements`.
///
/// Port of TS `isDashboardLikePrompt` (`orchestrator.ts:192-197`).
pub(crate) fn is_dashboard_like_prompt(prompt: &str, plan: &OrchestratorPlan) -> bool {
    let subtask_text: String = plan
        .subtasks
        .iter()
        .map(subtask_text)
        .collect::<Vec<_>>()
        .join("\n");
    let text = format!("{}\n{}", prompt, subtask_text).to_lowercase();
    has_dashboard_keyword(&text)
}

/// Returns `true` when the orchestrator should build a sidebar + main-column
/// layout instead of a single vertical stack.
///
/// Conditions (all must hold):
/// 1. `plan.root_frame.width > 480`
/// 2. `is_dashboard_like_prompt`
/// 3. Any subtask `is_sidebar_subtask`
/// 4. Any subtask's `label + elements` matches `/(metric|chart|table|
///    transaction|customer|revenue|growth|analytics|list)/`
///
/// Port of TS `shouldUseDashboardColumns` (`orchestrator.ts:199-211`).
pub(crate) fn should_use_dashboard_columns(prompt: &str, plan: &OrchestratorPlan) -> bool {
    if plan.root_frame.width <= 480.0 {
        return false;
    }

    let dashboard_like = is_dashboard_like_prompt(prompt, plan);
    let has_sidebar = plan.subtasks.iter().any(is_sidebar_subtask);
    let has_main_panels = plan.subtasks.iter().any(|st| {
        let text = format!("{} {}", st.label, st.elements.as_deref().unwrap_or("")).to_lowercase();
        has_data_panel_label_keyword(&text)
    });

    dashboard_like && has_sidebar && has_main_panels
}

// ---------------------------------------------------------------------------
// Keyword helpers (private)
// ---------------------------------------------------------------------------

/// `id + label + elements` joined and lowercased — the canonical "subtask text"
/// used by every predicate.
fn subtask_text(st: &Subtask) -> String {
    format!(
        "{} {} {}",
        st.id,
        st.label,
        st.elements.as_deref().unwrap_or("")
    )
    .to_lowercase()
}

/// `/(sidebar|side bar|navigation|nav|menu)/`
fn has_sidebar_keyword(text: &str) -> bool {
    text.contains("sidebar")
        || text.contains("side bar")
        || text.contains("navigation")
        || text.contains("nav")
        || text.contains("menu")
}

/// `/(top bar|header)/`
fn has_topbar_keyword(text: &str) -> bool {
    text.contains("top bar") || text.contains("header")
}

/// `/(main content|content area|main area|content column)/`
fn has_main_content_keyword(text: &str) -> bool {
    text.contains("main content")
        || text.contains("content area")
        || text.contains("main area")
        || text.contains("content column")
}

/// `/(metric|chart|table|transaction|customer|analytics|revenue|growth|sidebar)/`
/// — used as the NOT clause inside `is_main_content_container_subtask`.
fn has_data_panel_keyword(text: &str) -> bool {
    text.contains("metric")
        || text.contains("chart")
        || text.contains("table")
        || text.contains("transaction")
        || text.contains("customer")
        || text.contains("analytics")
        || text.contains("revenue")
        || text.contains("growth")
        || text.contains("sidebar")
}

/// `/(metric|chart|table|transaction|customer|revenue|growth|analytics|list)/`
/// — used in `should_use_dashboard_columns` main-panel detection (label+elements only).
fn has_data_panel_label_keyword(text: &str) -> bool {
    text.contains("metric")
        || text.contains("chart")
        || text.contains("table")
        || text.contains("transaction")
        || text.contains("customer")
        || text.contains("revenue")
        || text.contains("growth")
        || text.contains("analytics")
        || text.contains("list")
}

/// `/(dashboard|admin|analytics|fintech|workspace|data)/`
fn has_dashboard_keyword(text: &str) -> bool {
    text.contains("dashboard")
        || text.contains("admin")
        || text.contains("analytics")
        || text.contains("fintech")
        || text.contains("workspace")
        || text.contains("data")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{OrchestratorPlan, Region, RootFrameSpec, Subtask};

    // ---- helpers -----------------------------------------------------------

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
        }
    }

    fn plan_with(width: f64, subtasks: Vec<Subtask>) -> OrchestratorPlan {
        OrchestratorPlan {
            root_frame: root(width),
            subtasks,
            style_guide_name: None,
        }
    }

    // ---- is_sidebar_subtask ------------------------------------------------

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

    // ---- is_main_content_container_subtask ---------------------------------

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

    // ---- is_dashboard_like_prompt ------------------------------------------

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

    // ---- should_use_dashboard_columns --------------------------------------

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
}
