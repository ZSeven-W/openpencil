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
// §4.5 Main-subtask normalisation + row bin-packing
// ---------------------------------------------------------------------------

/// Return value of [`group_dashboard_main_rows`].
pub(crate) struct DashboardRowGroups {
    /// Each element is a row of subtask *indices* (into `plan.subtasks`).
    pub rows: Vec<Vec<usize>>,
    /// The computed full width of the main column.
    pub full_width: f64,
    /// The gap to use between rows.
    pub row_gap: f64,
}

/// Normalises the main-content-container subtask in-place:
///
/// - Strips "metrics row" / "metrics container" phrases from its `elements`.
/// - Relabels it to `"Top Bar"`.
/// - Clamps its `region.height` to `[88, 120]`.
///
/// No-op when fewer than 2 main subtasks or when no
/// `is_main_content_container_subtask` is found.
///
/// Port of TS `normalizeDashboardMainSubtasks` (`orchestrator.ts:322-343`).
pub(crate) fn normalize_dashboard_main_subtasks(plan: &mut OrchestratorPlan) {
    let main_count = plan
        .subtasks
        .iter()
        .filter(|st| !is_sidebar_subtask(st))
        .count();
    if main_count < 2 {
        return;
    }

    // Find the index of the main-content-container subtask.
    let container_idx = plan
        .subtasks
        .iter()
        .position(is_main_content_container_subtask);
    let Some(idx) = container_idx else { return };

    let container = &mut plan.subtasks[idx];

    // Strip "metrics? (cards? )?(row|container)" phrases from elements.
    let raw = container.elements.clone().unwrap_or_default();
    let cleaned = strip_metrics_phrases(&raw);

    // Extract "top bar …" substring if present, else use the cleaned text, else
    // fall back to a fixed default.
    let top_bar_elements = extract_top_bar_fragment(&cleaned);

    container.label = "Top Bar".into();
    container.elements = Some(top_bar_elements);
    let h = container.region.height;
    container.region.height = h.clamp(88.0, 120.0);
}

/// Strips `[,;]? metrics? (cards? )?(row|container)…` phrases (case-insensitive).
///
/// Port of the two `.replace()` calls in TS `normalizeDashboardMainSubtasks`.
fn strip_metrics_phrases(s: &str) -> String {
    // Simple iterative approach: scan for "metric" and remove the phrase up to
    // the next punctuation boundary.
    //
    // TS regex: `/[,;]?\s*metrics?\s*(cards?\s*)?(row|container)[^,.;\]]*/gi`
    // We implement this with a manual scan to avoid pulling in the `regex` crate.
    let lower = s.to_lowercase();
    let bytes = lower.as_bytes();
    let src_bytes = s.as_bytes();
    let len = bytes.len();
    let mut result = String::with_capacity(len);
    let mut i = 0usize;

    while i < len {
        // Try to match the phrase starting at position `i`.
        if let Some(skip) = metrics_phrase_len(&lower[i..]) {
            // Also eat a leading `,` or `;` if present just before `i`.
            // The TS regex has `[,;]?` at the START of the match.
            // Since we build `result` left-to-right, trim trailing `, ` from result.
            trim_trailing_sep(&mut result);
            i += skip;
        } else {
            // Emit byte from original (preserving case).
            result.push(src_bytes[i] as char);
            i += 1;
        }
    }

    // Collapse multiple spaces and trim trailing punctuation.
    let collapsed = collapse_spaces(&result);
    trim_trailing_sep_str(&collapsed)
}

/// Returns the byte-length of a metrics phrase starting at the beginning of `s`
/// (lowercased), or `None` if no match.
///
/// Pattern: `\s*metrics?\s*(cards?\s*)?(row|container)[^,.;\]]*`
fn metrics_phrase_len(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    // Leading whitespace
    while i < len && bytes[i] == b' ' {
        i += 1;
    }

    // "metric" or "metrics"
    let metric = b"metric";
    if i + metric.len() > len {
        return None;
    }
    if &bytes[i..i + metric.len()] != metric {
        return None;
    }
    i += metric.len();
    if i < len && bytes[i] == b's' {
        i += 1;
    }

    // Optional whitespace
    while i < len && bytes[i] == b' ' {
        i += 1;
    }

    // Optional "cards? "
    let card = b"card";
    if i + card.len() <= len && &bytes[i..i + card.len()] == card {
        i += card.len();
        if i < len && bytes[i] == b's' {
            i += 1;
        }
        while i < len && bytes[i] == b' ' {
            i += 1;
        }
    }

    // "row" or "container"
    let row = b"row";
    let container = b"container";
    if i + row.len() <= len && &bytes[i..i + row.len()] == row {
        i += row.len();
    } else if i + container.len() <= len && &bytes[i..i + container.len()] == container {
        i += container.len();
    } else {
        return None;
    }

    // Consume everything up to the next `,`, `.`, `;`, `]`.
    while i < len && !matches!(bytes[i], b',' | b'.' | b';' | b']') {
        i += 1;
    }

    Some(i)
}

/// Trims a trailing `, ` or `; ` (or just `,`/`;`) from `result`.
fn trim_trailing_sep(result: &mut String) {
    let trimmed = result.trim_end_matches(' ');
    if trimmed.ends_with(',') || trimmed.ends_with(';') {
        let new_len = trimmed.len() - 1;
        result.truncate(new_len);
        // Also eat any trailing space before the sep.
        let trimmed2 = result.trim_end_matches(' ');
        let final_len = trimmed2.len();
        result.truncate(final_len);
    }
}

/// Collapses two-or-more spaces into one.
fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for c in s.chars() {
        if c == ' ' {
            if !last_space {
                out.push(' ');
            }
            last_space = true;
        } else {
            last_space = false;
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// Trims a trailing `;` or `,` (with optional whitespace) from a string slice.
fn trim_trailing_sep_str(s: &str) -> String {
    s.trim_end_matches([',', ';', ' ']).to_string()
}

/// Extracts the "top bar …" fragment from `cleaned` (case-insensitive), or
/// returns `cleaned` itself, or a fixed default when both are empty.
///
/// Port of TS:
/// ```ts
/// const topBarElements =
///   cleanedElements.match(/top\s*bar[^.;\]]*/i)?.[0]?.trim() ??
///   cleanedElements ??
///   'Top bar with page title, date range selector, and export button';
/// ```
fn extract_top_bar_fragment(cleaned: &str) -> String {
    if cleaned.is_empty() {
        return "Top bar with page title, date range selector, and export button".into();
    }

    // Case-insensitive search for "top" then optional space then "bar".
    let lower = cleaned.to_lowercase();
    if let Some(idx) = lower.find("top") {
        let rest = &lower[idx + 3..];
        // skip optional whitespace then "bar"
        let rest2 = rest.trim_start_matches(' ');
        if rest2.starts_with("bar") {
            // End at the next `.`, `;`, or `]`
            let start = idx;
            let after_bar = idx + 3 + (rest.len() - rest2.len()) + 3;
            let end = cleaned[after_bar..]
                .find(['.', ';', ']'])
                .map(|rel| after_bar + rel)
                .unwrap_or(cleaned.len());
            let fragment = cleaned[start..end].trim();
            if !fragment.is_empty() {
                return fragment.to_string();
            }
        }
    }

    cleaned.to_string()
}

/// Groups non-sidebar subtasks into rows using a greedy left-to-right bin-packer.
///
/// Returns [`DashboardRowGroups`] with rows as *indices into `plan.subtasks`*
/// (not clones), plus the computed `full_width` and `row_gap`.
///
/// Rules (port of TS `groupDashboardMainRows` `orchestrator.ts:345-399`):
/// - `full_width = max(root.width - 260, max of all main subtask region widths)`.
/// - A subtask is **standalone** (its own row) if:
///   - `region.width >= full_width * 0.82`, OR
///   - it is an `is_main_content_container_subtask`.
/// - Otherwise, greedily append to the current row:
///   - Pre-flush the current row BEFORE adding if the proposed cumulative
///     width (`currentWidth + gap + width`) would exceed `full_width * 1.05`.
///   - Post-flush AFTER adding when the cumulative width `>= full_width * 0.92`.
/// - `row_gap = root.gap` when `> 0`, else `24`.
pub(crate) fn group_dashboard_main_rows(plan: &OrchestratorPlan) -> DashboardRowGroups {
    // Indices of non-sidebar subtasks, preserving their original plan order.
    let main_indices: Vec<usize> = plan
        .subtasks
        .iter()
        .enumerate()
        .filter(|(_, st)| !is_sidebar_subtask(st))
        .map(|(i, _)| i)
        .collect();

    let full_width = {
        let base = plan.root_frame.width - 260.0;
        let max_w = main_indices
            .iter()
            .map(|&i| {
                let w = plan.subtasks[i].region.width;
                if w > 0.0 {
                    w
                } else {
                    0.0
                }
            })
            .fold(0.0_f64, f64::max);
        f64::max(base, max_w)
    };

    let row_gap = match plan.root_frame.gap {
        Some(g) if g > 0.0 => g,
        _ => 24.0,
    };

    if main_indices.is_empty() {
        return DashboardRowGroups {
            rows: vec![],
            full_width,
            row_gap,
        };
    }

    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut current_row: Vec<usize> = Vec::new();
    let mut current_width: f64 = 0.0;

    let flush =
        |rows: &mut Vec<Vec<usize>>, current_row: &mut Vec<usize>, current_width: &mut f64| {
            if !current_row.is_empty() {
                rows.push(std::mem::take(current_row));
                *current_width = 0.0;
            }
        };

    for &idx in &main_indices {
        let st = &plan.subtasks[idx];
        let width = if st.region.width > 0.0 {
            st.region.width
        } else {
            full_width
        };
        let is_standalone = width >= full_width * 0.82 || is_main_content_container_subtask(st);

        if is_standalone {
            flush(&mut rows, &mut current_row, &mut current_width);
            rows.push(vec![idx]);
            continue;
        }

        // Proposed cumulative width if we add this subtask.
        let next_width = if current_row.is_empty() {
            width
        } else {
            current_width + row_gap + width
        };

        // Pre-flush: if adding would exceed 1.05 * full_width.
        if !current_row.is_empty() && next_width > full_width * 1.05 {
            flush(&mut rows, &mut current_row, &mut current_width);
        }

        current_row.push(idx);
        current_width = if current_row.len() == 1 {
            width
        } else {
            current_width + row_gap + width
        };

        // Post-flush: if cumulative >= 0.92 * full_width.
        if current_width >= full_width * 0.92 {
            flush(&mut rows, &mut current_row, &mut current_width);
        }
    }

    flush(&mut rows, &mut current_row, &mut current_width);

    DashboardRowGroups {
        rows,
        full_width,
        row_gap,
    }
}

// ---------------------------------------------------------------------------
// §4.6 Slot assignment — implementation in sibling file (800-line cap)
// ---------------------------------------------------------------------------

mod slots;
#[allow(unused_imports)] // consumed by run.rs in a later task
pub(crate) use slots::{assign_dashboard_main_parents, RowFrameEntry};

// ---------------------------------------------------------------------------
// Tests — split into sibling files to stay under 800 lines
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "dashboard_columns_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "dashboard_columns_tests_b.rs"]
mod tests_b;

#[cfg(test)]
#[path = "dashboard_columns_tests_b2.rs"]
mod tests_b2;
