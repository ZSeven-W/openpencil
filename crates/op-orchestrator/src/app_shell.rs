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
/// Outer gutter for the content column `[vertical, horizontal]`. Mirrors
/// Pencil's app-shell content `padding:[32,40]`.
const CONTENT_PADDING: [i64; 2] = [32, 40];

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

/// Drop the sidebar's footer (user/profile card) to the bottom of the now
/// full-height column. Pencil splits the sidebar into a Top + Bottom group with
/// `justifyContent: space_between`; weak models emit a flat column with a
/// FIXED-height spacer (e.g. 120px) that no longer reaches the bottom once the
/// sidebar stretches to the content height — so the footer floats mid-column.
/// Stretch an existing spacer to `fill_container`; if there is none, inject a
/// flexible spacer just before a footer-like last child.
fn sink_sidebar_footer(sidebar: &mut Value) {
    let sidebar_id = sidebar
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("sidebar")
        .to_string();
    let Some(kids) = sidebar.get_mut("children").and_then(Value::as_array_mut) else {
        return;
    };
    if kids.len() < 2 {
        return;
    }
    let mut stretched = false;
    for c in kids.iter_mut() {
        if ident_text(c).contains("spacer") {
            if let Some(o) = c.as_object_mut() {
                o.insert("height".into(), json!("fill_container"));
                stretched = true;
            }
        }
    }
    if stretched {
        return;
    }
    let last_is_footer = kids
        .last()
        .map(ident_text)
        .map(|t| {
            ["user", "profile", "account", "avatar", "footer", "member"]
                .iter()
                .any(|k| t.contains(k))
        })
        .unwrap_or(false);
    if last_is_footer {
        let pos = kids.len() - 1;
        kids.insert(
            pos,
            json!({
                "type": "frame",
                "id": format!("{sidebar_id}-spacer"),
                "name": "Sidebar Spacer",
                "width": "fill_container",
                "height": "fill_container",
                "children": [],
            }),
        );
    }
}

// ── Standalone footer-sink for ALREADY-STRUCTURED sidebars ──
//
// `reshape_sidebar_to_app_shell` (+ its `sink_sidebar_footer`) only fires when
// the root is a flat-vertical band that must be turned INTO an app-shell. When a
// weak model already emits the correct `[sidebar | content]` shell but stacks the
// sidebar nav as a FLAT fit_content column — brand, nav groups, then a user/Pro
// footer as the last item, with NO `space_between` and NO spacer — the footer
// rides directly under the nav and the bottom of the rail is dead space. This
// whole-root pass sinks that footer: promote the column to `fill_container` and
// inject a flexible spacer before its footer-like last child. Runs in
// `run_cleanup_passes` (after the app-shell reshape, so it only sees sidebars the
// reshape left alone). Self-contained per node; never drops anything.

/// True when a node reads as a sidebar FOOTER: an explicit footer-ish name, or
/// (for the common UNNAMED footer card) its subtree text carries an account /
/// owner / upgrade signal. Name-only detection misses glm's unnamed
/// `{avatar, "James Miller", "Shop Owner"}` card — the content check catches it.
fn is_footer_like(v: &Value) -> bool {
    let t = ident_text(v);
    if ["user", "profile", "account", "avatar", "footer", "member"]
        .iter()
        .any(|k| t.contains(k))
    {
        return true;
    }
    let mut text = String::new();
    collect_subtree_text(v, &mut text);
    [
        "owner", "admin", "account", "sign out", "log out", "upgrade", "pro plan", "go pro",
        "settings",
    ]
    .iter()
    .any(|k| text.contains(k))
}

/// Lowercased concatenation of every `text`/`content` string in the subtree.
fn collect_subtree_text(v: &Value, out: &mut String) {
    if let Some(content) = v.get("content").and_then(Value::as_str) {
        out.push_str(&content.to_lowercase());
        out.push(' ');
    }
    if let Some(kids) = v.get("children").and_then(Value::as_array) {
        for c in kids {
            collect_subtree_text(c, out);
        }
    }
}

/// Whole-root driver: recurse, sinking the footer of any flat sidebar-nav column.
pub(crate) fn sink_structured_sidebar_footers(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !sink_mut(&mut v) {
        return false;
    }
    match serde_json::from_value::<PenNode>(v) {
        Ok(new_node) => {
            *root = new_node;
            true
        }
        Err(_) => false,
    }
}

/// Pure predicate: does THIS node qualify as a flat sidebar nav whose footer
/// should be sunk? (sidebar-named vertical column, hug height, ≥3 children, last
/// child footer-like, no existing distribution intent / spacer.)
fn try_sink_flat_sidebar_nav(v: &Value) -> bool {
    if !is_sidebar_named(&ident_text(v)) {
        return false;
    }
    if layout_str(v) != Some("vertical") {
        return false;
    }
    // Already distributing or already has a spacer → leave alone (idempotent +
    // respects an author who DID express the pattern).
    if matches!(
        v.get("justifyContent").and_then(Value::as_str),
        Some("space_between") | Some("space_around") | Some("space_evenly")
    ) {
        return false;
    }
    let Some(kids) = v.get("children").and_then(Value::as_array) else {
        return false;
    };
    if kids.len() < 3 {
        return false;
    }
    if kids.iter().any(|c| ident_text(c).contains("spacer")) {
        return false;
    }
    kids.last().map(is_footer_like).unwrap_or(false)
}

/// A sidebar wrapper that already HAS the two-group anatomy — EXACTLY
/// [top group, footer-like bottom group] — but forgot the distribution
/// contract (`height:fill_container` + `justifyContent:space_between`), so
/// the footer floats right under the nav instead of sinking (measured: a
/// "Sidebar Navigation" with a correct Top Group / Bottom Group pair, hug
/// height, no justifyContent). The 3-children spacer arm can't reach it;
/// this arm just supplies the missing two properties.
fn try_sink_two_group_sidebar(v: &Value) -> bool {
    if !is_sidebar_named(&ident_text(v)) {
        return false;
    }
    if layout_str(v) != Some("vertical") {
        return false;
    }
    if matches!(
        v.get("justifyContent").and_then(Value::as_str),
        Some("space_between") | Some("space_around") | Some("space_evenly")
    ) {
        return false;
    }
    let Some(kids) = v.get("children").and_then(Value::as_array) else {
        return false;
    };
    kids.len() == 2 && two_group_footerish(&kids[1]) && !two_group_footerish(&kids[0])
}

/// Footer-ness for the two-group arm: the bottom group is usually NAMED
/// "Bottom Group" (not user/profile), with the profile card one level down —
/// so accept a "bottom"-named group or any DESCENDANT whose name reads as a
/// user/profile/account card, on top of the plain [`is_footer_like`] check.
fn two_group_footerish(v: &Value) -> bool {
    if is_footer_like(v) || ident_text(v).contains("bottom") {
        return true;
    }
    fn descendant_profile_named(v: &Value) -> bool {
        let t = ident_text(v);
        if ["user", "profile", "account", "avatar", "member"]
            .iter()
            .any(|k| t.contains(k))
        {
            return true;
        }
        v.get("children")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .any(descendant_profile_named)
    }
    descendant_profile_named(v)
}

/// Mutating walk: apply the sink to qualifying nodes, recursing into children.
fn sink_mut(v: &mut Value) -> bool {
    let mut changed = false;
    if try_sink_two_group_sidebar(v) {
        if let Some(obj) = v.as_object_mut() {
            obj.insert("height".into(), json!("fill_container"));
            obj.insert("justifyContent".into(), json!("space_between"));
            changed = true;
        }
    }
    if try_sink_flat_sidebar_nav(v) {
        let id = v
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("sidebar")
            .to_string();
        if let Some(obj) = v.as_object_mut() {
            obj.insert("height".into(), json!("fill_container"));
        }
        if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
            let pos = kids.len() - 1;
            kids.insert(
                pos,
                json!({
                    "type": "frame",
                    "id": format!("{id}-footer-spacer"),
                    "name": "Sidebar Spacer",
                    "width": "fill_container",
                    "height": "fill_container",
                    "children": [],
                }),
            );
            changed = true;
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= sink_mut(c);
        }
    }
    changed
}

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
    sink_sidebar_footer(&mut sidebar);

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
        // Outer page gutter (Pencil's app-shell content carries padding:[32,40]
        // so sections don't run edge-to-edge into the viewport). Without it the
        // new fill_container column lets the stat cards touch the right edge.
        "padding": CONTENT_PADDING,
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

// ── Evict mis-parented content sections from the sidebar column ──
//
// A dashboard's LEFT RAIL holds navigation only. But the two-column routing
// (`run.rs` → `dashboard_columns::is_sidebar_subtask`) matches the bare
// `nav`/`menu` substrings, so a weak model that describes a "Client Directory"
// content section with a "filter menu" — or emits it as a second forest root of
// the sidebar subtask — strands a full data TABLE inside the 260px `clipContent`
// rail, where it overflows and paints over the nav. This whole-root pass
// relocates such sections into the sibling `Main Content` column.
//
// Detection is STRUCTURAL (a real multi-column data table) or a content-only
// NAME token ("table"/"directory"/"data grid"). It deliberately does NOT reuse
// `section_has_dashboard_signal`, whose broad "analytics"/"metric" tokens also
// match sidebar MENU-ITEM labels (this very bug's rail carries an "Analytics"
// nav item — flagging the whole rail as content would evict the navigation
// itself). Runs in `run_cleanup_passes`. Self-contained; only MOVES nodes.

/// A `table` / `data grid`-NAMED container with a real multi-row body (≥2
/// horizontal rows). Requiring BOTH the name AND the row structure is what keeps
/// a NAV LIST out: a weak model's nav items are multi-child horizontal rows too
/// (icon + label + badge + chevron = 4 children was observed), but the nav
/// container is named "Navigation" / "Menu" / "Nav Group" — never "table". A
/// bare row/column count evicted the entire navigation; the name gate fixes it.
fn is_named_data_table(v: &Value) -> bool {
    if !is_table_named(&ident_text(v)) {
        return false;
    }
    v.get("children")
        .and_then(Value::as_array)
        .map(|kids| {
            kids.iter()
                .filter(|row| {
                    layout_str(row) == Some("horizontal")
                        && row
                            .get("children")
                            .and_then(Value::as_array)
                            .map(|c| c.len())
                            .unwrap_or(0)
                            >= 2
                })
                .count()
                >= 2
        })
        .unwrap_or(false)
}

fn is_table_named(t: &str) -> bool {
    t.contains("table")
        || t.contains("data grid")
        || t.contains("datagrid")
        || t.contains("data-grid")
}

/// True when a sidebar child is really a MAIN-CONTENT data section: its OWN name
/// reads as a data section, or its subtree holds an explicit (named) data table.
/// Deliberately NOT a bare row/column-count heuristic — weak-model nav items are
/// multi-child horizontal rows too, and a pure structural check evicted the
/// whole navigation. Names are the reliable discriminator (real tables are named
/// "Table" / "Client Table" / "Data Grid"; navs are "Navigation" / "Nav X").
fn sidebar_child_is_misplaced_content(v: &Value) -> bool {
    let t = ident_text(v);
    if t.contains("directory") || t.contains("data table") || t.contains("data grid") {
        return true;
    }
    // Content-section names that never belong in a nav rail: a schedule /
    // appointments / activity block is main-column material (measured: a
    // design-loop run parked "Today's Schedule" + its appointment cards in
    // the 260px sidebar, then rebuilt the main column WITHOUT deleting the
    // misplaced copy — the old name set only knew tables). Nav / brand /
    // profile / upgrade blocks don't carry these names, so the gate stays
    // safe for legitimate rail content.
    if [
        "schedule",
        "appointment",
        "activity",
        "upcoming",
        "recent",
        "timeline",
    ]
    .iter()
    .any(|k| t.contains(k))
    {
        return true;
    }
    fn walk(v: &Value) -> bool {
        is_named_data_table(v)
            || v.get("children")
                .and_then(Value::as_array)
                .is_some_and(|kids| kids.iter().any(walk))
    }
    walk(v)
}

/// The narrow left rail of a `[sidebar | main content]` shell — a `sidebar`-named
/// column that is a fixed width ≤ 400 (or non-numeric, where the strong name
/// carries it).
fn is_narrow_sidebar_column(v: &Value) -> bool {
    if !is_sidebar_named(&ident_text(v)) {
        return false;
    }
    match num(v, "width") {
        Some(w) => w <= 400.0,
        None => true,
    }
}

const SPLIT_SHELL_SIDEBAR_WIDTH: f64 = 260.0;

/// A model that already split the root into `[sidebar | main]` but left the root
/// WITHOUT a horizontal layout stacks (or overlaps) the two columns instead of
/// placing them side by side. Measured on the agentic loop: MiniMax-M3 emits
/// `Dashboard > {Sidebar, Main}` with `layout=None`, and
/// [`reshape_sidebar_to_app_shell`] deliberately SKIPS 2-child roots (its
/// `detect` assumes a 2-child root — and a keyword/absent root width — is
/// already a correct app-shell). This catches the already-split-but-flat case
/// it leaves behind: flip the root to a horizontal row and give the columns
/// definite widths so the content column doesn't collapse behind the sidebar.
/// It ALSO corrects an ALREADY-row shell whose sidebar hugs its content
/// (`height=fit_content`): such a sidebar is only as tall as its nav, so a
/// `space_between` / `fill_container` footer inside it has no room to sink and
/// floats mid-page — the sidebar is promoted to `fill_container` height so it
/// fills the row. Gated on a STRONG sidebar name + a non-sidebar sibling so a
/// legitimate two-section vertical page is never turned sideways.
pub(crate) fn ensure_split_shell_is_row(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !ensure_row_mut(&mut v) {
        return false;
    }
    match serde_json::from_value::<PenNode>(v) {
        Ok(new_node) => {
            *root = new_node;
            true
        }
        Err(_) => false,
    }
}

fn ensure_row_mut(v: &mut Value) -> bool {
    let already_row = layout_str(v) == Some("horizontal");
    let Some(kids) = v.get("children").and_then(Value::as_array) else {
        return false;
    };
    // Exactly [sidebar-column, content]: a narrow sidebar-named vertical column
    // first, a non-sidebar sibling second.
    if kids.len() != 2 {
        return false;
    }
    if !is_narrow_sidebar_column(&kids[0]) || !is_column_layout(&kids[0]) {
        return false;
    }
    if is_sidebar_named(&ident_text(&kids[1])) {
        return false;
    }
    let Some(obj) = v.as_object_mut() else {
        return false;
    };
    let mut changed = false;
    if !already_row {
        obj.insert("layout".into(), json!("horizontal"));
        obj.entry("alignItems").or_insert(json!("stretch"));
        changed = true;
    }
    let Some(kids_mut) = obj.get_mut("children").and_then(Value::as_array_mut) else {
        return changed;
    };
    // Sidebar: pin a fixed narrow WIDTH when it lacks a numeric one, and a
    // fill_container HEIGHT so it fills the row instead of hugging its content —
    // a `fit_content` sidebar is only as tall as its nav, so a `space_between` /
    // `fill_container` footer inside it has no room to sink and floats mid-page
    // (measured: a 260×532 sidebar in a 260×1234 row left the profile card
    // stranded 700px above the bottom).
    {
        let needs_w = num(&kids_mut[0], "width").is_none();
        let needs_h = kids_mut[0].get("height").and_then(Value::as_str) != Some("fill_container");
        if let Some(sb) = kids_mut[0].as_object_mut() {
            if needs_w {
                sb.insert("width".into(), json!(SPLIT_SHELL_SIDEBAR_WIDTH));
                changed = true;
            }
            if needs_h {
                sb.insert("height".into(), json!("fill_container"));
                changed = true;
            }
        }
    }
    // Main: fill the rest of the row (only when we just created the split — an
    // already-correct shell keeps its authored main width).
    if !already_row {
        if let Some(main) = kids_mut[1].as_object_mut() {
            main.insert("width".into(), json!("fill_container"));
        }
    }
    changed
}

/// Whole-root driver: relocate any data section stranded in a sidebar column
/// into the sibling `Main Content` column. Returns `true` iff it moved a node.
pub(crate) fn evict_content_from_sidebar_column(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !evict_mut(&mut v) {
        return false;
    }
    match serde_json::from_value::<PenNode>(v) {
        Ok(new_node) => {
            *root = new_node;
            true
        }
        Err(_) => false,
    }
}

/// Recurse; at every horizontal `[… sidebar … main content …]` node move the
/// sidebar column's misplaced content sections into the content column.
fn evict_mut(v: &mut Value) -> bool {
    let mut changed = try_evict_in_shell(v);
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= evict_mut(c);
        }
    }
    changed
}

fn try_evict_in_shell(shell: &mut Value) -> bool {
    if layout_str(shell) != Some("horizontal") {
        return false;
    }
    let Some(kids) = shell.get("children").and_then(Value::as_array) else {
        return false;
    };
    let sidebar_idx = kids.iter().position(is_narrow_sidebar_column);
    let content_idx = kids
        .iter()
        .position(|c| ident_text(c).contains("main content"));
    let (Some(si), Some(ci)) = (sidebar_idx, content_idx) else {
        return false;
    };
    if si == ci {
        return false;
    }
    // Drain the misplaced content sections out of the sidebar column.
    let mut moved: Vec<Value> = Vec::new();
    {
        let Some(kids_mut) = shell.get_mut("children").and_then(Value::as_array_mut) else {
            return false;
        };
        if let Some(sb_kids) = kids_mut[si]
            .get_mut("children")
            .and_then(Value::as_array_mut)
        {
            let mut i = 0;
            while i < sb_kids.len() {
                if sidebar_child_is_misplaced_content(&sb_kids[i]) {
                    moved.push(sb_kids.remove(i));
                } else {
                    i += 1;
                }
            }
        }
    }
    if moved.is_empty() {
        return false;
    }
    // Append them (in order) to the content column.
    let Some(kids_mut) = shell.get_mut("children").and_then(Value::as_array_mut) else {
        return false;
    };
    if let Some(ct_kids) = kids_mut[ci]
        .get_mut("children")
        .and_then(Value::as_array_mut)
    {
        ct_kids.append(&mut moved);
        true
    } else {
        false
    }
}

#[cfg(test)]
#[path = "app_shell_tests.rs"]
mod tests;
