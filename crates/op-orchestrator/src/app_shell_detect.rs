//! Tolerant value readers and the app-shell detection predicate.

use super::*;

/// `width`/`height`/`gap` as f64, tolerant of numeric strings like `"605"`.
/// Returns `None` for keyword sizings (`fill_container` / `fit_content`).
pub(super) fn num(v: &Value, key: &str) -> Option<f64> {
    let field = v.get(key)?;
    field
        .as_f64()
        .or_else(|| field.as_str().and_then(|s| s.parse::<f64>().ok()))
}

pub(super) fn layout_str(v: &Value) -> Option<&str> {
    v.get("layout").and_then(Value::as_str)
}

/// Lowercased `name + id` identity text used for keyword matching.
pub(super) fn ident_text(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("");
    format!("{name} {id}").to_lowercase()
}

/// A STRONG left-rail signal. Deliberately excludes the bare `"nav"` / `"menu"`
/// substrings (they match "Navy Hero", restaurant "Menu", "Navigation Guide"),
/// and excludes anything that reads as a top bar.
pub(super) fn is_sidebar_named(t: &str) -> bool {
    let strong = t.contains("sidebar")
        || t.contains("side bar")
        || t.contains("side nav")
        || t.contains("side-nav")
        || t.contains("left rail")
        || t.contains("left nav")
        || t.contains("nav rail")
        || t.contains("navigation rail")
        || t.contains("nav-rail")
        || t.contains("侧边栏")
        || t.contains("侧栏")
        || t.contains("侧边导航")
        || t.contains("左侧导航");
    strong && !is_topbar_named(t)
}

pub(super) fn is_topbar_named(t: &str) -> bool {
    t.contains("top bar")
        || t.contains("topbar")
        || t.contains("top nav")
        || t.contains("top-nav")
        || t.contains("header")
        || t.contains("app bar")
        || t.contains("appbar")
        || t.contains("breadcrumb")
}

/// True when a section subtree carries a dashboard-grade signal (a table, a
/// metric/KPI/stat block, or a chart). Used as the STRUCTURAL intent gate — far
/// more robust than a root-name keyword (the bug root "Barbershop Client
/// Management" carries no dashboard word, but its sections are "Key Metrics" +
/// "Client Table"). Recurses the whole section.
pub(super) fn section_has_dashboard_signal(v: &Value) -> bool {
    let t = ident_text(v);
    if t.contains("table")
        || t.contains("metric")
        || t.contains("stat")
        || t.contains("kpi")
        || t.contains("chart")
        || t.contains("graph")
        || t.contains("analytics")
        || t.contains("data grid")
        || t.contains("datagrid")
    {
        return true;
    }
    v.get("children")
        .and_then(Value::as_array)
        .is_some_and(|kids| kids.iter().any(section_has_dashboard_signal))
}

/// A child that is itself a whole screen/page (multi-screen file guard): named
/// screen/page, or both wide and tall enough to be a standalone artboard.
pub(super) fn is_screen_like(v: &Value) -> bool {
    let t = ident_text(v);
    if t.contains("screen") || t.contains("artboard") || t.contains(" page") || t.ends_with("page")
    {
        return true;
    }
    matches!(num(v, "width"), Some(w) if w >= DESKTOP_MIN_WIDTH)
        && matches!(num(v, "height"), Some(h) if h >= 700.0)
}

/// Vertical / none / absent layout (a column or absolute container — NOT a row).
pub(super) fn is_column_layout(v: &Value) -> bool {
    !matches!(layout_str(v), Some("horizontal"))
}

// ── Detection ──

/// All criteria must hold (see module doc for the false-positives each guards).
pub(super) fn detect(v: &Value) -> bool {
    // 1. Desktop width (numeric ≥ 900 — excludes mobile/tablet; a
    //    keyword-width root is treated as out-of-scope, not reshaped).
    let Some(root_w) = num(v, "width") else {
        return false;
    };
    if root_w < DESKTOP_MIN_WIDTH {
        return false;
    }
    // 2. Sidebar + at least two content sections. A *vertical* root has the
    //    sidebar stacked as a full-width band; a *horizontal* root has the
    //    sidebar AND every content section crammed into one row (the orchestrator
    //    assembles both shapes). Both are handled — only the already-correct
    //    `[sidebar | content]` 2-child app-shell is left alone, which the ≥3
    //    floor + the `Main Content` idempotency guard below cover.
    let Some(kids) = v.get("children").and_then(Value::as_array) else {
        return false;
    };
    if kids.len() < 3 {
        return false;
    }
    // Idempotency: never re-wrap a root that already carries our content column.
    if kids.iter().any(|c| ident_text(c).contains("main content")) {
        return false;
    }
    // 9. Multi-screen file: never app-shell a wrapper of standalone screens.
    if kids.iter().filter(|c| is_screen_like(c)).count() >= 2 {
        return false;
    }
    let first = &kids[0];
    // 4. First child reads as a sidebar (strong tokens, not a top bar).
    if !is_sidebar_named(&ident_text(first)) {
        return false;
    }
    // 5. First child is a column, not a horizontal nav row.
    if !is_column_layout(first) {
        return false;
    }
    // 6. First child is NOT a short top strip. A header/top-bar band is a small
    //    EXPLICIT numeric height (64–96px); skip those. A `fit_content` /
    //    `fill_container` (non-numeric) height is allowed — real sidebars are
    //    frequently authored that way — because the STRONG sidebar-name token
    //    (criterion 4) + the dashboard-content gate (criterion 8) already carry
    //    the specificity; height is only used to reject short numeric headers.
    if matches!(num(first, "height"), Some(h) if h < MIN_SIDEBAR_HEIGHT) {
        return false;
    }
    // 7. First child spans ~full width (the actual bug signature). A child
    //    already narrower than half the root is an existing left column → skip.
    let first_is_full_width = match first.get("width") {
        Some(Value::String(s)) if s == "fill_container" => true,
        None => true,
        _ => matches!(num(first, "width"), Some(w) if w >= 0.9 * root_w),
    };
    if !first_is_full_width {
        return false;
    }
    if matches!(num(first, "width"), Some(w) if w < 0.5 * root_w) {
        return false;
    }
    // 8. STRUCTURAL dashboard gate — ≥2 content sections carry a table / metric
    //    / chart signal. Kills restaurant-menu / landing / portfolio / settings
    //    false-positives without depending on the (too-narrow) root name.
    let dashboard_sections = kids[1..]
        .iter()
        .filter(|s| section_has_dashboard_signal(s))
        .count();
    dashboard_sections >= 2
}

// ── Restructure ──
