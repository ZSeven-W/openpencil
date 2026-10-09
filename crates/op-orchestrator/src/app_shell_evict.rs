//! Split-shell row enforcement and eviction of mis-parented content
//! sections from the sidebar column.

use super::*;

/// A `table` / `data grid`-NAMED container with a real multi-row body (≥2
/// horizontal rows). Requiring BOTH the name AND the row structure is what keeps
/// a NAV LIST out: a weak model's nav items are multi-child horizontal rows too
/// (icon + label + badge + chevron = 4 children was observed), but the nav
/// container is named "Navigation" / "Menu" / "Nav Group" — never "table". A
/// bare row/column count evicted the entire navigation; the name gate fixes it.
pub(super) fn is_named_data_table(v: &Value) -> bool {
    if !is_table_named(&ident_text(v))
        && !matches!(
            v.get("role").and_then(Value::as_str),
            Some("table" | "data-table" | "data-grid")
        )
    {
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

pub(super) fn is_table_named(t: &str) -> bool {
    t.contains("table")
        || t.contains("data grid")
        || t.contains("datagrid")
        || t.contains("data-grid")
        || t.contains("表格")
        || t.contains("数据表")
        || t.contains("数据网格")
}

/// True when a sidebar child is really a MAIN-CONTENT data section: its OWN name
/// reads as a data section, or its subtree holds an explicit (named) data table.
/// Deliberately NOT a bare row/column-count heuristic — weak-model nav items are
/// multi-child horizontal rows too, and a pure structural check evicted the
/// whole navigation. Names are the reliable discriminator (real tables are named
/// "Table" / "Client Table" / "Data Grid"; navs are "Navigation" / "Nav X").
pub(super) fn sidebar_child_is_misplaced_content(v: &Value) -> bool {
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
pub(super) fn is_narrow_sidebar_column(v: &Value) -> bool {
    if !is_sidebar_named(&ident_text(v)) && v.get("role").and_then(Value::as_str) != Some("sidebar")
    {
        return false;
    }
    match num(v, "width") {
        Some(w) => w <= 400.0,
        None => true,
    }
}

pub(super) const SPLIT_SHELL_SIDEBAR_WIDTH: f64 = 260.0;

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

pub(super) fn ensure_row_mut(v: &mut Value) -> bool {
    if num(v, "width").is_some_and(|w| w < DESKTOP_MIN_WIDTH) {
        return false;
    }
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
pub(super) fn evict_mut(v: &mut Value) -> bool {
    let mut changed = try_evict_in_shell(v);
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= evict_mut(c);
        }
    }
    changed
}

pub(super) fn try_evict_in_shell(shell: &mut Value) -> bool {
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
