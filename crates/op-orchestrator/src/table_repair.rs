//! Table repair — regroup flat table cells into a Table→Row→Cell hierarchy.
//!
//! Weak models (glm-5.2 "Client Roster") emit a table as a header row followed
//! by FLAT sibling cells stacked vertically: `Table Header`, `R1 Client Cell`,
//! `R1 Visit`, `R1 Barber`, `R1 Spend`, `R1 Status Badge`, `R2 Client Cell`, …
//! Each client's cells render stacked (left-aligned, not under their columns)
//! and the full-width cells become full-width bars. This pass groups those flat
//! cells back into `Table(vertical) → Row(horizontal) → Cell` whose body cells
//! share the header's per-column widths.
//!
//! Run alongside `app_shell` in `cleanup::run_cleanup_passes`. Detection is
//! deliberately NARROW and never guesses (the adversarial design review showed
//! a heuristic header + chunk-of-N grouping over-fires on toolbars / tab bars /
//! text feeds and mis-segments when the column count drifts): it requires BOTH
//! an explicit header (role/name) AND every trailing cell to carry an explicit
//! `R{n}` / `row N` row-index name, and aborts on any ragged / conflicting /
//! ambiguous run. It is a safe backstop for the exact glm shape, not a general
//! table inferencer.

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

/// Regroup a dashboard's flat table cells into Table→Row→Cell. Mutates the
/// page-root in place via the serialize → mutate `Value` → deserialize
/// round-trip the section passes use. Returns `true` iff it restructured at
/// least one table; no-op + `false` otherwise (the node is never dropped).
pub(crate) fn regroup_flat_table_rows(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !regroup_in_value(&mut v) {
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

/// Post-order: recurse into children first, then try to regroup THIS node's
/// children (so a freshly-built Table subtree isn't re-descended in one pass).
fn regroup_in_value(v: &mut Value) -> bool {
    let mut changed = false;
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= regroup_in_value(c);
        }
    }
    changed | try_regroup_children(v) | try_reparent_orphans(v)
}

// ── tolerant Value readers (mirror the app_shell idiom) ──

fn num(v: &Value, key: &str) -> Option<f64> {
    let f = v.get(key)?;
    f.as_f64()
        .or_else(|| f.as_str().and_then(|s| s.parse::<f64>().ok()))
}

fn layout_str(v: &Value) -> Option<&str> {
    v.get("layout").and_then(Value::as_str)
}

fn role_str(v: &Value) -> Option<&str> {
    v.get("role").and_then(Value::as_str)
}

fn ident_text(v: &Value) -> String {
    let name = v.get("name").and_then(Value::as_str).unwrap_or("");
    let role = v.get("role").and_then(Value::as_str).unwrap_or("");
    format!("{name} {role}").to_lowercase()
}

fn is_column_layout(v: &Value) -> bool {
    !matches!(layout_str(v), Some("horizontal"))
}

/// An explicit table-header row: a horizontal frame of ≥2 column labels, tagged
/// by role or name. The heuristic "any horizontal frame of short texts" header
/// is intentionally NOT accepted — it false-positives on toolbars / tab bars.
fn is_table_header(v: &Value) -> Option<usize> {
    let t = ident_text(v);
    let tagged = role_str(v) == Some("table-header")
        || t.contains("table header")
        || t.contains("header row")
        || t.contains("column header")
        || t.contains("thead");
    if !tagged {
        return None;
    }
    if layout_str(v) != Some("horizontal") {
        return None;
    }
    let n = v
        .get("children")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    (2..=12).contains(&n).then_some(n)
}

/// A cell's explicit row index from an `R{n}` / `row N` / `row-N` name. Returns
/// `None` when the name carries no row index — which aborts the whole regroup
/// (we never guess boundaries from position alone).
fn row_index(v: &Value) -> Option<u32> {
    let name = v.get("name").and_then(Value::as_str)?.trim().to_lowercase();
    let rest = name
        .strip_prefix('r')
        .or_else(|| name.strip_prefix("row-"))
        .or_else(|| name.strip_prefix("row "))
        .or_else(|| name.strip_prefix("row"))?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u32>().ok()
}

/// A node that can be a table cell: a text, or a frame/group whose role/name
/// reads as a cell-ish element. Crucially every candidate must ALSO carry a row
/// index (checked by the caller) — this predicate only bounds the run.
fn is_cell_like(v: &Value) -> bool {
    matches!(
        v.get("type").and_then(Value::as_str),
        Some("text" | "frame" | "group")
    )
}

/// A structural region that terminates the flat-cell run (and is preserved).
fn is_run_terminator(v: &Value) -> bool {
    let t = ident_text(v);
    role_str(v) == Some("section")
        || t.contains("section")
        || t.contains("pagination")
        || t.contains("footer")
        || t.contains("navbar")
        || t.contains("table-row")
}

// ── detection + grouping ──

fn try_regroup_children(parent: &mut Value) -> bool {
    if !is_column_layout(parent) {
        return false;
    }
    let Some(kids) = parent.get("children").and_then(Value::as_array) else {
        return false;
    };
    if kids.len() < 3 {
        return false;
    }
    // Locate the header + its column count.
    let Some((h_idx, n_cols)) = kids
        .iter()
        .enumerate()
        .find_map(|(i, c)| is_table_header(c).map(|n| (i, n)))
    else {
        return false;
    };
    // Already-grouped? A table-row sibling means this ran already.
    if kids[h_idx + 1..]
        .iter()
        .any(|c| role_str(c) == Some("table-row"))
    {
        return false;
    }
    // Collect the contiguous flat-cell run after the header, requiring EVERY
    // cell to carry an explicit row index (abort on the first that doesn't).
    let header = &kids[h_idx];
    let col_widths = header_column_widths(header, n_cols);
    let (header_pad, header_gap) = header_metrics(header);

    let mut run: Vec<(u32, &Value)> = Vec::new();
    for c in &kids[h_idx + 1..] {
        if is_run_terminator(c) {
            break;
        }
        if !is_cell_like(c) {
            break;
        }
        let Some(idx) = row_index(c) else {
            // A cell with no row index → ambiguous → do not guess; abort.
            return false;
        };
        run.push((idx, c));
    }
    let group = match group_rows(&run, n_cols) {
        Some(g) => g,
        None => return false,
    };
    let k = group.len();
    let run_len = run.len();

    // Build the Table subtree: [header, row_0, … row_{k-1}].
    let parent_id = parent.get("id").and_then(Value::as_str).unwrap_or("table");
    let mut table_children: Vec<Value> = Vec::with_capacity(k + 1);
    table_children.push(header.clone());
    for (ri, row_cells) in group.into_iter().enumerate() {
        table_children.push(build_row(
            parent_id,
            ri,
            row_cells,
            &col_widths,
            &header_pad,
            header_gap,
        ));
    }
    let table = json!({
        "type": "frame",
        "id": format!("{parent_id}-table"),
        "name": "Table",
        "role": "table",
        "width": "fill_container",
        "layout": "vertical",
        "children": table_children,
    });

    // Splice: keep children[..h_idx], drop header+run, insert the Table frame,
    // keep the rest (the terminator and anything after it).
    let Some(arr) = parent.get_mut("children").and_then(Value::as_array_mut) else {
        return false;
    };
    let tail_start = h_idx + 1 + run_len;
    let tail: Vec<Value> = arr.split_off(tail_start);
    arr.truncate(h_idx); // drop the header + the flat cells
    arr.push(table);
    arr.extend(tail);
    true
}

/// Group the row-indexed run into `k` rows of exactly `n_cols` each. Returns
/// `None` (abort) on any irregularity: <2 rows, a row whose size ≠ n_cols, or a
/// non-contiguous index sequence. Order is preserved as encountered.
fn group_rows<'a>(run: &[(u32, &'a Value)], n_cols: usize) -> Option<Vec<Vec<&'a Value>>> {
    if run.is_empty() {
        return None;
    }
    let mut rows: Vec<(u32, Vec<&'a Value>)> = Vec::new();
    for (idx, cell) in run {
        match rows.last_mut() {
            Some((cur, cells)) if cur == idx => cells.push(cell),
            _ => rows.push((*idx, vec![*cell])),
        }
    }
    if rows.len() < 2 {
        return None;
    }
    // Every row must be exactly the header width, and the indices must be a
    // strictly increasing contiguous sequence (1,2,3,… or 0,1,2,…).
    let first = rows[0].0;
    for (offset, (idx, cells)) in rows.iter().enumerate() {
        if cells.len() != n_cols {
            return None;
        }
        if *idx != first + offset as u32 {
            return None;
        }
    }
    Some(rows.into_iter().map(|(_, cells)| cells).collect())
}

/// Per-column width from the header children: numeric width, else the x-gap to
/// the next column, else `fill_container`.
fn header_column_widths(header: &Value, n_cols: usize) -> Vec<Value> {
    let kids = header.get("children").and_then(Value::as_array);
    let mut out = Vec::with_capacity(n_cols);
    for j in 0..n_cols {
        let w = kids.and_then(|k| k.get(j)).and_then(|c| num(c, "width"));
        out.push(match w {
            Some(v) => json!(v),
            None => json!("fill_container"),
        });
    }
    out
}

fn header_metrics(header: &Value) -> (Value, f64) {
    let pad = header
        .get("padding")
        .cloned()
        .unwrap_or_else(|| json!([12, 16]));
    let gap = num(header, "gap").unwrap_or(16.0);
    (pad, gap)
}

/// Build one Row frame (role table-row → horizontal/fill/center defaults),
/// mapping each cell positionally to its column width.
fn build_row(
    parent_id: &str,
    ri: usize,
    cells: Vec<&Value>,
    col_widths: &[Value],
    pad: &Value,
    gap: f64,
) -> Value {
    let row_cells: Vec<Value> = cells
        .into_iter()
        .enumerate()
        .map(|(j, c)| {
            let mut cell = c.clone();
            if let Some(obj) = cell.as_object_mut() {
                let w = col_widths
                    .get(j)
                    .cloned()
                    .unwrap_or(json!("fill_container"));
                obj.insert("width".into(), w);
            }
            cell
        })
        .collect();
    json!({
        "type": "frame",
        "id": format!("{parent_id}-row-{ri}"),
        "name": format!("Row {}", ri + 1),
        "role": "table-row",
        "layout": "horizontal",
        "width": "fill_container",
        "alignItems": "center",
        "padding": pad,
        "gap": gap,
        "children": row_cells,
    })
}

// ── orphan list-row cells ──
//
// A second flat-structure bug: a weak model emits a list/appointment ROW frame
// but leaves some of its cells (avatar initials, status badge) as FLAT SIBLINGS
// right after the row instead of inside it — so they render stacked full-width
// below the row. Pattern: `<Row frame> <orphan> <orphan> <Row frame> <orphan>
// <orphan> …`. Reparent each row's trailing orphans back into it.

/// A list/appointment row container: a non-empty frame named/roled as a row.
fn is_list_row(v: &Value) -> bool {
    if v.get("type").and_then(Value::as_str) != Some("frame") {
        return false;
    }
    if v.get("children")
        .and_then(Value::as_array)
        .is_none_or(|k| k.is_empty())
    {
        return false;
    }
    let t = ident_text(v);
    matches!(role_str(v), Some("table-row" | "list-row" | "list-item"))
        || t.contains("row")
        || t.contains("list item")
        || t.contains("appointment")
}

/// A stray leaf that belongs inside the preceding row (a text / badge / icon /
/// layout-less small frame). Never a structural region or another row.
fn is_orphan_cell(v: &Value) -> bool {
    if is_list_row(v) {
        return false;
    }
    let t = ident_text(v);
    if [
        "section", "table", "navbar", "footer", "header", "sidebar", "chart", "card",
    ]
    .iter()
    .any(|k| t.contains(k))
    {
        return false;
    }
    match v.get("type").and_then(Value::as_str) {
        Some("text" | "icon_font" | "image" | "ellipse" | "rectangle") => true,
        // A frame/group only counts as an orphan cell when it carries no
        // layout of its own (a real sub-section would set one).
        Some("frame" | "group") => matches!(layout_str(v), None | Some("none")),
        _ => false,
    }
}

/// Reparent flat orphan cells back into the list row they belong to. Fires only
/// on a REGULAR pattern: ≥2 row frames, each followed by the SAME non-zero
/// number of orphan leaves — so a one-off stray sibling or an irregular list
/// never triggers a guess.
fn try_reparent_orphans(parent: &mut Value) -> bool {
    if !is_column_layout(parent) {
        return false;
    }
    let Some(kids) = parent.get("children").and_then(Value::as_array) else {
        return false;
    };
    // Walk the children, recording (row index, trailing orphan count).
    let mut rows: Vec<(usize, usize)> = Vec::new();
    let mut i = 0;
    while i < kids.len() {
        if is_list_row(&kids[i]) {
            let mut j = i + 1;
            while j < kids.len() && is_orphan_cell(&kids[j]) {
                j += 1;
            }
            rows.push((i, j - (i + 1)));
            i = j;
        } else {
            i += 1;
        }
    }
    if rows.len() < 2 {
        return false;
    }
    let orphans_per_row = rows[0].1;
    if orphans_per_row == 0 || rows.iter().any(|(_, c)| *c != orphans_per_row) {
        return false;
    }

    // Rebuild children, folding each row's trailing orphans into the row.
    let Some(arr) = parent.get_mut("children").and_then(Value::as_array_mut) else {
        return false;
    };
    let old: Vec<Value> = std::mem::take(arr);
    let mut out: Vec<Value> = Vec::with_capacity(old.len());
    let mut i = 0;
    while i < old.len() {
        if is_list_row(&old[i]) {
            let mut row = old[i].clone();
            let mut orphans: Vec<Value> = Vec::with_capacity(orphans_per_row);
            let mut j = i + 1;
            while j < old.len() && orphans.len() < orphans_per_row && is_orphan_cell(&old[j]) {
                orphans.push(old[j].clone());
                j += 1;
            }
            if let Some(rc) = row.get_mut("children").and_then(Value::as_array_mut) {
                rc.extend(orphans);
            }
            out.push(row);
            i = j;
        } else {
            out.push(old[i].clone());
            i += 1;
        }
    }
    *arr = out;
    true
}

// ── Table column gap ──
//
// Weak models emit a table's rows with NO column gap (`gap: null` / 0), so the
// columns render touching — a "SPEND" + "STATUS" header pair reads as
// "SPENDSTATUS", data cells crowd. This pass gives every ≥3-column row of a
// `table` / `data grid`-NAMED container a sensible column gap. Gated to
// table-named containers (never a nav / toolbar / chip row) and to ≥3-column
// rows, so it can't space out a 2-item header/search row. Runs after
// `regroup_flat_table_rows` in `run_cleanup_passes` (so a freshly-regrouped
// table is spaced too).

/// Default column gap injected into gap-less table rows.
const TABLE_COLUMN_GAP: f64 = 24.0;

/// Give gap-less rows of a table-named container a column gap. Returns `true`
/// iff it changed a row. Same round-trip as the sibling passes.
pub(crate) fn ensure_table_column_gap(root: &mut PenNode) -> bool {
    let Ok(mut v) = serde_json::to_value(&*root) else {
        return false;
    };
    if !ensure_gap_in_value(&mut v) {
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

fn ensure_gap_in_value(v: &mut Value) -> bool {
    let mut changed = false;
    if is_table_container(v) {
        if let Some(rows) = v.get_mut("children").and_then(Value::as_array_mut) {
            for row in rows.iter_mut() {
                // Rows may sit one level deeper, under an UNNAMED structural
                // wrapper (a model groups [toolbar row, rows-wrapper] inside the
                // table frame — measured: a "Client List Table" whose gap-less
                // rows all lived in such a wrapper and rendered columns
                // touching). The table NAME gate stays on the outer node; an
                // unnamed vertical wrapper inherits it.
                changed |= give_row_gap_through_wrappers(row);
            }
        }
    }
    if let Some(kids) = v.get_mut("children").and_then(Value::as_array_mut) {
        for c in kids.iter_mut() {
            changed |= ensure_gap_in_value(c);
        }
    }
    changed
}

/// Apply [`give_row_gap`] to `node`, descending through any CHAIN of unnamed
/// structural wrappers first (a model buries its rows arbitrarily deep in
/// nameless verticals — measured: two levels below the table frame).
fn give_row_gap_through_wrappers(node: &mut Value) -> bool {
    if is_unnamed_vertical_wrapper(node) {
        let mut changed = false;
        if let Some(inner) = node.get_mut("children").and_then(Value::as_array_mut) {
            for r in inner.iter_mut() {
                changed |= give_row_gap_through_wrappers(r);
            }
        }
        return changed;
    }
    give_row_gap(node)
}

/// Insert the default column gap when `row` is a gap-less ≥3-column row.
fn give_row_gap(row: &mut Value) -> bool {
    if layout_str(row) == Some("horizontal") && row_needs_gap(row) {
        if let Some(obj) = row.as_object_mut() {
            obj.insert("gap".into(), json!(TABLE_COLUMN_GAP));
            return true;
        }
    }
    false
}

/// A nameless vertical frame — pure structure between a table-named container
/// and its rows. A NAMED child (pagination, footer, toolbar) never qualifies.
fn is_unnamed_vertical_wrapper(v: &Value) -> bool {
    is_column_layout(v)
        && v.get("name")
            .and_then(Value::as_str)
            .map(|n| n.trim().is_empty())
            .unwrap_or(true)
}

/// A vertical container named like a table with ≥2 horizontal row children —
/// the same name-gate the sidebar-eviction pass uses, so a nav / toolbar is
/// never mistaken for a table.
fn is_table_container(v: &Value) -> bool {
    if !is_column_layout(v) {
        return false;
    }
    let t = ident_text(v);
    if !(t.contains("table") || t.contains("data grid") || t.contains("datagrid")) {
        return false;
    }
    v.get("children")
        .and_then(Value::as_array)
        .map(|kids| {
            kids.iter()
                .map(|k| {
                    // Count rows both directly AND through one unnamed
                    // structural wrapper (same tolerance as the gap pass).
                    count_rows_through_wrappers(k)
                })
                .sum::<usize>()
                >= 2
        })
        .unwrap_or(false)
}

/// Count row-like nodes, descending through chains of unnamed wrappers.
fn count_rows_through_wrappers(node: &Value) -> usize {
    if is_unnamed_vertical_wrapper(node) {
        return node
            .get("children")
            .and_then(Value::as_array)
            .map(|inner| inner.iter().map(count_rows_through_wrappers).sum())
            .unwrap_or(0);
    }
    usize::from(is_row_like(node))
}

/// A horizontal frame with ≥2 children — the row shape the container gate
/// counts.
fn is_row_like(r: &Value) -> bool {
    layout_str(r) == Some("horizontal")
        && r.get("children")
            .and_then(Value::as_array)
            .map(|c| c.len())
            .unwrap_or(0)
            >= 2
}

/// A ≥3-column row whose current gap is missing or effectively zero.
fn row_needs_gap(row: &Value) -> bool {
    let cols = row
        .get("children")
        .and_then(Value::as_array)
        .map(|c| c.len())
        .unwrap_or(0);
    if cols < 3 {
        return false;
    }
    match row.get("gap") {
        None | Some(Value::Null) => true,
        Some(_) => num(row, "gap").map(|g| g < 1.0).unwrap_or(true),
    }
}

#[cfg(test)]
#[path = "table_repair_tests.rs"]
mod tests;
