//! Dashboard-aware plan normalisation predicates + section sizing.
//!
//! Faithful port of `apps/web/src/services/ai/orchestrator.ts:175-237`.
//! These feed the deterministic plan normaliser (`plan_normalize.rs`), which
//! shapes dashboard-like plans for the generic sequential path. The former
//! bespoke sidebar+main *scaffold* strategy (and the detection / surface-color /
//! row bin-packing helpers that existed solely to build it) was removed — the
//! generic sequential pipeline + `cleanup_desktop_dashboard` handle dashboards.

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
    let identity = subtask_identity_text(st);
    if has_strong_sidebar_keyword(&identity) {
        return true;
    }
    if has_topbar_keyword(&identity) {
        return false;
    }
    if has_sidebar_keyword(&identity) {
        return true;
    }

    let elements = st.elements.as_deref().unwrap_or_default().to_lowercase();
    (identity.contains("left") || identity.contains("side")) && has_sidebar_keyword(&elements)
}

/// A STRONG sidebar signal — "sidebar" / "side bar" / "side nav" / "rail" — is
/// unambiguously a left rail (unlike the broad `nav`/`menu` tokens, which also
/// match a landing-page "Navigation" section). Used to pre-build the two-column
/// app-shell scaffold without requiring a dashboard-content gate.
pub(crate) fn is_strong_sidebar_subtask(st: &Subtask) -> bool {
    let t = subtask_text(st);
    has_strong_sidebar_keyword(&t)
}

/// Returns `true` when the prompt + plan subtasks suggest a dashboard-like
/// design (desktop data-heavy screen).
///
/// Matches `/(dashboard|admin|analytics|fintech|workspace|data)/` over
/// `prompt` concatenated with every subtask's `id + label + elements`.
///
/// Explicit landing-page intent vetoes every dashboard signal. Otherwise an
/// explicit dashboard or admin-console request wins before the plan's
/// structural landing-page anatomy is considered.
///
/// Port of TS `isDashboardLikePrompt` (`orchestrator.ts:192-197`).
pub(crate) fn is_dashboard_like_prompt(prompt: &str, plan: &OrchestratorPlan) -> bool {
    let prompt = prompt.to_lowercase();
    if has_explicit_landing_page_intent(&prompt) {
        return false;
    }
    if has_explicit_dashboard_intent(&prompt) {
        return true;
    }
    if plan_has_landing_anatomy(plan) {
        return false;
    }
    let subtask_text: String = plan
        .subtasks
        .iter()
        .map(subtask_text)
        .collect::<Vec<_>>()
        .join("\n");
    let text = format!("{prompt}\n{subtask_text}");
    has_dashboard_keyword(&text)
}

pub(crate) fn plan_has_landing_anatomy(plan: &OrchestratorPlan) -> bool {
    let identities: Vec<String> = plan.subtasks.iter().map(subtask_identity_text).collect();
    let has_hero = identities
        .iter()
        .any(|text| text.contains("hero") || text.contains("首屏") || text.contains("英雄"));
    let has_supporting_story = identities.iter().any(|text| {
        text.contains("feature")
            || text.contains("capabilit")
            || text.contains("workflow")
            || text.contains("testimonial")
            || text.contains("customer proof")
            || text.contains("logo")
            || text.contains("特性")
            || text.contains("功能")
            || text.contains("亮点")
            || text.contains("卖点")
            || text.contains("图文")
            || text.contains("评价")
            || text.contains("案例")
    });
    let has_conversion_end = identities.iter().any(|text| {
        text.contains("pricing")
            || text.contains("faq")
            || text.contains("footer")
            || text.contains("final cta")
            || text.contains("call to action")
            || text.contains("页脚")
            || text.contains("定价")
            || text.contains("常见问题")
    });
    has_hero && has_supporting_story && has_conversion_end
}

/// Returns `true` when the subtask is a dashboard-grade CONTENT section — a
/// table, metric/KPI/stat block, or chart. Used (with a sidebar subtask) as the
/// structural gate for pre-building the two-column scaffold, so a landing page
/// with a stray "Navigation" subtask (no data sections) is NOT mistaken for a
/// dashboard. Mirrors `app_shell`'s `section_has_dashboard_signal`.
pub(crate) fn is_dashboard_content_subtask(st: &Subtask) -> bool {
    let t = subtask_text(st);
    t.contains("table")
        || t.contains("metric")
        || t.contains("stat")
        || t.contains("kpi")
        || t.contains("chart")
        || t.contains("graph")
        || t.contains("analytics")
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

fn subtask_identity_text(st: &Subtask) -> String {
    format!("{} {}", st.id, st.label).to_lowercase()
}

fn has_strong_sidebar_keyword(text: &str) -> bool {
    text.contains("sidebar")
        || text.contains("side bar")
        || text.contains("side nav")
        || text.contains("side-nav")
        || text.contains("left rail")
        || text.contains("left nav")
        || text.contains("nav rail")
        || text.contains("navigation rail")
        // Chinese plans name the rail directly; checked before the top-bar
        // words below, so "侧边导航栏" stays a sidebar.
        || text.contains("侧边栏")
        || text.contains("侧栏")
        || text.contains("侧边导航")
        || text.contains("左侧导航")
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
    text.contains("top bar")
        || text.contains("header")
        || text.contains("top navigation")
        || text.contains("navigation bar")
        || text.contains("nav bar")
        || text.contains("navbar")
        // "导航栏" is literally "navigation bar" — a landing page's top nav. A
        // GLM-5.3-Flash EV page planned `nav (导航栏)` first and, with no
        // top-bar word it could read, was scaffolded as a sidebar dashboard.
        || text.contains("导航栏")
        || text.contains("顶栏")
        || text.contains("顶部导航")
        || text.contains("顶部栏")
        || text.contains("页头")
}

fn has_explicit_landing_page_intent(text: &str) -> bool {
    text.contains("landing page")
        || text.contains("landing-page")
        || text.contains("落地页")
        || text.contains("着陆页")
}

fn has_explicit_dashboard_intent(text: &str) -> bool {
    text.contains("dashboard") || text.contains("admin console") || text.contains("admin-console")
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
// §4.2 Section sizing
// ---------------------------------------------------------------------------

/// Infers the expected height (px) of a dashboard section subtask.
///
/// Precedence (first match wins):
/// - sidebar → 760
/// - header / top-bar → 96
/// - metric / kpi → 160
/// - chart / revenue → 320
/// - transaction / activity / feed → 320
/// - table / analytics / customer → 340
/// - default → 160
///
/// Port of TS `inferDashboardSectionHeight` (`orchestrator.ts:213-224`).
pub(crate) fn infer_dashboard_section_height(st: &Subtask) -> f64 {
    let text = subtask_text(st);
    if is_sidebar_subtask(st) {
        return 760.0;
    }
    if has_header_keyword(&text) {
        return 96.0;
    }
    if has_metric_kpi_keyword(&text) {
        return 160.0;
    }
    if has_chart_revenue_keyword(&text) {
        return 320.0;
    }
    if has_transaction_activity_feed_keyword(&text) {
        return 320.0;
    }
    if has_table_analytics_customer_keyword(&text) {
        return 340.0;
    }
    160.0
}

/// Infers the expected width (px) of a dashboard section subtask.
///
/// Uses `root_width` to derive `main_width = max(320, root_width - 260)`.
/// Returns:
/// - 260  for sidebar subtasks
/// - `main_width * 0.62` (rounded) for chart/revenue subtasks
/// - `main_width * 0.38` (rounded) for transaction/activity/feed subtasks
/// - `main_width` otherwise
///
/// Port of TS `inferDashboardSectionWidth` (`orchestrator.ts:226-237`).
pub(crate) fn infer_dashboard_section_width(st: &Subtask, root_width: f64) -> f64 {
    const SIDEBAR_WIDTH: f64 = 260.0;
    let main_width = f64::max(320.0, root_width - SIDEBAR_WIDTH);
    let text = subtask_text(st);
    if is_sidebar_subtask(st) {
        return SIDEBAR_WIDTH;
    }
    if has_chart_revenue_keyword(&text) {
        return (main_width * 0.62).round();
    }
    if has_transaction_activity_feed_keyword(&text) {
        return (main_width * 0.38).round();
    }
    main_width
}

/// `/(top\s*header|top\s*bar|header)/`
fn has_header_keyword(text: &str) -> bool {
    text.contains("top header")
        || text.contains("top bar")
        || text.contains("topheader")
        || text.contains("topbar")
        || text.contains("header")
}

/// `/(metric|kpi)/`
fn has_metric_kpi_keyword(text: &str) -> bool {
    text.contains("metric") || text.contains("kpi")
}

/// `/(chart|revenue)/`
fn has_chart_revenue_keyword(text: &str) -> bool {
    text.contains("chart") || text.contains("revenue")
}

/// `/(transaction|activity|feed)/`
fn has_transaction_activity_feed_keyword(text: &str) -> bool {
    text.contains("transaction") || text.contains("activity") || text.contains("feed")
}

/// `/(table|analytics|customer)/`
fn has_table_analytics_customer_keyword(text: &str) -> bool {
    text.contains("table") || text.contains("analytics") || text.contains("customer")
}

#[cfg(test)]
#[path = "dashboard_columns_tests.rs"]
mod tests;
