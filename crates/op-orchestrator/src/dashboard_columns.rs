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

// ---------------------------------------------------------------------------
// §4.4 Sidebar surface color
// ---------------------------------------------------------------------------

/// Picks a fill color for the pre-built dashboard sidebar frame.
///
/// Precedence chain (first non-`None` wins):
/// 1. Style-guide catalog content — table match `"Sidebar Surface | #XXXXXX"`
///    then inline match `"Sidebar Surface … #XXXXXX"`.
/// 2. `design_md.colorPalette` role lookup:
///    `sidebar` → `panel` → `surface|card`.
/// 3. Returns `None`; the caller falls back to
///    `root_frame.fill[0].color` or `#0F172A`.
///
/// Port of TS `extractSidebarSurfaceColor` (`orchestrator-sidebar-color.ts`).
pub(crate) fn extract_sidebar_surface_color(
    style_guide_content: Option<&str>,
    design_md: Option<&jian_ops_schema::DesignMdSpec>,
) -> Option<String> {
    // 1. Catalog style-guide content
    if let Some(content) = style_guide_content {
        // Table match: "Sidebar Surface | #RRGGBB"
        if let Some(hex) = find_hex_after_pattern(content, "Sidebar Surface", true) {
            return Some(hex);
        }
        // Inline match: "Sidebar Surface …anything… #RRGGBB"
        if let Some(hex) = find_hex_after_pattern(content, "Sidebar Surface", false) {
            return Some(hex);
        }
    }

    // 2. design.md palette role lookup
    if let Some(spec) = design_md {
        if let Some(palette) = &spec.color_palette {
            if let Some(entry) = palette
                .iter()
                .find(|c| c.role.to_lowercase().contains("sidebar"))
            {
                return Some(entry.hex.to_uppercase());
            }
            if let Some(entry) = palette
                .iter()
                .find(|c| c.role.to_lowercase().contains("panel"))
            {
                return Some(entry.hex.to_uppercase());
            }
            if let Some(entry) = palette.iter().find(|c| {
                let r = c.role.to_lowercase();
                r.contains("surface") || r.contains("card")
            }) {
                return Some(entry.hex.to_uppercase());
            }
        }
    }

    None
}

/// Search `content` for a hex color that appears after `pattern` (case-insensitive).
///
/// When `require_pipe` is `true` the pattern must be followed (with optional
/// whitespace) by `|` then the hex — this matches the Markdown table form
/// `"Sidebar Surface | #RRGGBB"`.
///
/// When `require_pipe` is `false` it matches any `#RRGGBB` that appears
/// anywhere after the pattern on the same logical match region.
fn find_hex_after_pattern(content: &str, pattern: &str, require_pipe: bool) -> Option<String> {
    // Locate the pattern (case-insensitive)
    let lower = content.to_lowercase();
    let pat_lower = pattern.to_lowercase();
    let idx = lower.find(&pat_lower)?;
    let after = &content[idx + pattern.len()..];

    if require_pipe {
        // After the pattern, expect optional whitespace then `|` then optional
        // whitespace then `#RRGGBB`.
        let after_trimmed = after.trim_start();
        let after_pipe = after_trimmed.strip_prefix('|')?.trim_start();
        extract_leading_hex6(after_pipe)
    } else {
        // Find the first `#RRGGBB` anywhere in `after`.
        find_first_hex6(after)
    }
}

/// Extracts a `#RRGGBB` hex literal at the very start of `s` (ignoring leading
/// whitespace), uppercased.
fn extract_leading_hex6(s: &str) -> Option<String> {
    let s = s.trim_start();
    let s = s.strip_prefix('#')?;
    if s.len() >= 6 && s[..6].chars().all(|c| c.is_ascii_hexdigit()) {
        Some(format!("#{}", s[..6].to_uppercase()))
    } else {
        None
    }
}

/// Finds the first `#RRGGBB` (exactly 6 hex digits after `#`) in `s`,
/// uppercased.
fn find_first_hex6(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    for i in 0..len {
        if bytes[i] == b'#' && i + 7 <= len {
            let hex_part = &s[i + 1..i + 7];
            if hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
                return Some(format!("#{}", hex_part.to_uppercase()));
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Additional keyword helpers for §4.2
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests — split into sibling file to stay under 800 lines
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "dashboard_columns_tests.rs"]
mod tests;
