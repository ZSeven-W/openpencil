//! App-shell restructure — a single deterministic structural post-pass that
//! turns a flat-vertical desktop dashboard whose FIRST child is a full-width
//! sidebar into a horizontal `[sidebar(fixed) | content-column(fill)]` shell.
//!
//! Weak models (glm-5.x etc.) routinely emit a "Sidebar Navigation" as the
//! first child of a vertical root at the *full* page width, so it renders as a
//! full-width top band instead of a narrow left column. The orchestrator's old
//! plan-level "dashboard bespoke scaffold" (that pre-built the two-column root)
//! was removed in 346bcaa8; this is its lightweight node-tree replacement — one
//! pass, no row/slot bin-packing, no placeholder-height estimation.
//!
//! Runs once over each assembled page-root in `cleanup::run_cleanup_passes` —
//! the whole-doc finalize point SHARED by the orchestrator (per-subtask role
//! passes already ran) and the agentic loop (`apply_loop_finalize` ran the
//! whole-doc role passes) — so the moved sections keep their resolved roles.
//!
//! Both broken shapes the orchestrator emits are handled: a VERTICAL root with
//! the sidebar stacked as a full-width band, and a HORIZONTAL root with the
//! sidebar AND every content section crammed into one row. Detection is
//! intentionally strict (see [`detect`]); the adversarial design review flagged
//! restaurant "Menu" pages, "Navy" hero sections, top-nav bars, mobile/tablet
//! roots, and multi-screen files as the false-positives to avoid, so the gate
//! leans on a STRONG sidebar-name token + a real dashboard-content gate (a short
//! numeric header height is rejected, but `fit_content` sidebars are allowed).

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

/// Fixed left-sidebar column width. Mirrors the surviving sizing constant in
/// `dashboard_columns.rs` for parity with the removed scaffold.
const SIDEBAR_WIDTH: f64 = 260.0;
/// Desktop floor — excludes phones/tablets. Mirrors
/// `cleanup_desktop_dashboard::DESKTOP_DASHBOARD_MIN_WIDTH`.
const DESKTOP_MIN_WIDTH: f64 = 900.0;
/// A real sidebar is full-height; a 64–96px full-width band is a header.
const MIN_SIDEBAR_HEIGHT: f64 = 200.0;

/// Restructure a flat-vertical desktop dashboard whose first child is a
/// full-width sidebar into a horizontal `[sidebar(fixed) | content(fill)]`
/// app-shell. Mutates the page-root wrapper in place via the same serialize →
/// mutate `Value` → deserialize round-trip the section passes use. Returns
/// `true` iff it restructured; no-op + `false` when the strict detection
/// criteria do not hold or the round-trip fails (the node is never dropped).
pub(crate) fn reshape_sidebar_to_app_shell(wrapper: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*wrapper) else {
        return false;
    };
    if !detect(&v) {
        return false;
    }
    if !restructure(&mut v) {
        return false;
    }
    match serde_json::from_value::<PenNode>(v) {
        Ok(new_node) => {
            *wrapper = new_node;
            true
        }
        // Bad round-trip: leave the wrapper exactly as it was.
        Err(_) => false,
    }
}

// ── Value readers (mirror the tolerant accessors the sibling passes use) ──

/// `width`/`height`/`gap` as f64, tolerant of numeric strings like `"605"`.
/// Returns `None` for keyword sizings (`fill_container` / `fit_content`).
fn num(v: &Value, key: &str) -> Option<f64> {
    let field = v.get(key)?;
    field
        .as_f64()
        .or_else(|| field.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn layout_str(v: &Value) -> Option<&str> {
    v.get("layout").and_then(Value::as_str)
}

/// Lowercased `name + id` identity text used for keyword matching.
fn ident_text(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("");
    let id = v.get("id").and_then(Value::as_str).unwrap_or("");
    format!("{name} {id}").to_lowercase()
}

/// A STRONG left-rail signal. Deliberately excludes the bare `"nav"` / `"menu"`
/// substrings (they match "Navy Hero", restaurant "Menu", "Navigation Guide"),
/// and excludes anything that reads as a top bar.
fn is_sidebar_named(t: &str) -> bool {
    let strong = t.contains("sidebar")
        || t.contains("side bar")
        || t.contains("side nav")
        || t.contains("side-nav")
        || t.contains("left rail")
        || t.contains("left nav")
        || t.contains("nav rail")
        || t.contains("navigation rail")
        || t.contains("nav-rail");
    strong && !is_topbar_named(t)
}

fn is_topbar_named(t: &str) -> bool {
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
fn section_has_dashboard_signal(v: &Value) -> bool {
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
fn is_screen_like(v: &Value) -> bool {
    let t = ident_text(v);
    if t.contains("screen") || t.contains("artboard") || t.contains(" page") || t.ends_with("page")
    {
        return true;
    }
    matches!(num(v, "width"), Some(w) if w >= DESKTOP_MIN_WIDTH)
        && matches!(num(v, "height"), Some(h) if h >= 700.0)
}

/// Vertical / none / absent layout (a column or absolute container — NOT a row).
fn is_column_layout(v: &Value) -> bool {
    !matches!(layout_str(v), Some("horizontal"))
}

// ── Detection ──

/// All criteria must hold (see module doc for the false-positives each guards).
fn detect(v: &Value) -> bool {
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

/// Recursively retarget any descendant whose width was sized to (about) the OLD
/// full root width down to `fill_container`, so full-width sections/dividers
/// fill their new narrower column instead of overflowing it (taffy never
/// shrinks the cross axis). Targeted: only ~full-width Number widths change.
fn fill_full_width_descendants(v: &mut Value, old_root_w: f64) {
    if let Some(w) = num(v, "width") {
        if w >= 0.9 * old_root_w {
            if let Some(obj) = v.as_object_mut() {
                obj.insert("width".into(), json!("fill_container"));
            }
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            fill_full_width_descendants(c, old_root_w);
        }
    }
}

/// Detection already passed: split the wrapper's children into `[sidebar |
/// content-column]` and flip the wrapper to a horizontal app-shell. Returns
/// `false` only if the tree shape changed under us (defensive).
fn restructure(v: &mut Value) -> bool {
    let root_w = match num(v, "width") {
        Some(w) => w,
        None => return false,
    };
    let content_gap = num(v, "gap").filter(|g| *g > 0.0).unwrap_or(24.0);
    let content_id = format!(
        "{}-content",
        v.get("id").and_then(Value::as_str).unwrap_or("root")
    );

    let Some(obj) = v.as_object_mut() else {
        return false;
    };
    let Some(kids) = obj.get_mut("children").and_then(Value::as_array_mut) else {
        return false;
    };
    if kids.len() < 3 {
        return false;
    }
    let mut sidebar = kids.remove(0);
    let mut content_sections: Vec<Value> = std::mem::take(kids);

    // Sidebar → fixed narrow vertical column that stretches to the row height.
    // `height: fill_container` is the LOAD-BEARING cross-axis stretch (jian maps
    // `alignItems: stretch` to FlexStart, so the wrapper's alignItems alone
    // would NOT stretch it). Clip + width-retarget keep its old full-width
    // descendants from bleeding over the content column.
    if let Some(s) = sidebar.as_object_mut() {
        s.insert("width".into(), json!(SIDEBAR_WIDTH));
        s.insert("height".into(), json!("fill_container"));
        if s.get("layout").and_then(Value::as_str) != Some("vertical") {
            s.insert("layout".into(), json!("vertical"));
        }
        s.insert("clipContent".into(), json!(true));
    }
    fill_full_width_descendants(&mut sidebar, root_w);

    // Content sections → fill the new column instead of the old full root width.
    for section in content_sections.iter_mut() {
        fill_full_width_descendants(section, root_w);
    }

    let content = json!({
        "type": "frame",
        "id": content_id,
        "name": "Main Content",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": content_gap,
        "children": content_sections,
    });

    obj.insert("children".into(), json!([sidebar, content]));
    obj.insert("layout".into(), json!("horizontal"));
    obj.insert("gap".into(), json!(0));
    obj.insert("alignItems".into(), json!("stretch"));
    // The old height was the SUM of the vertical stack (incl. the sidebar); the
    // horizontal shell only needs the taller column. Track content instead of
    // leaving ~600px of dead canvas below.
    obj.insert("height".into(), json!("fit_content"));
    true
}

#[cfg(test)]
#[path = "app_shell_tests.rs"]
mod tests;
