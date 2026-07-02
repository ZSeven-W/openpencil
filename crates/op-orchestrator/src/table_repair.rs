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
    changed | try_regroup_children(v)
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

#[cfg(test)]
#[path = "table_repair_tests.rs"]
mod tests;
