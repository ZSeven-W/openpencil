//! Dashboard scaffold variant — S3b-3 Task C2.
//!
//! Split from `scaffold.rs` to keep both files under the 800-line ceiling.
//! Parallel to [`crate::scaffold::build_scaffold`] (single-root sequential)
//! and [`crate::scaffold::build_scaffold_concurrent_mobile`] (N-root
//! concurrent). Called only when `should_use_dashboard_columns` is true
//! (sequential path).
//!
//! Port of `orchestrator.ts:283-320` (`createDashboardColumnFrames`) +
//! `orchestrator.ts:954-1019` (the scaffold branch for `useDashboardColumns`).

use crate::cleanup::count_descendants;
use crate::dashboard_columns::{assign_dashboard_main_parents, get_dashboard_placeholder_height};
use crate::plan::OrchestratorPlan;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

/// Return type of [`build_scaffold_dashboard`]:
/// `(commands, sidebar_id, main_id, scaffold_baseline)`.
///
/// - `Vec<EditorCommand>` — a single `InsertSubtree` carrying the
///   horizontal root with `[sidebar, main]` embedded; `main`'s children
///   carry the synthesized row + slot frames produced by
///   `assign_dashboard_main_parents`. One command keeps the id remap
///   pass coherent — references between row / slot / main never break.
/// - `String` — the sidebar frame id (`{root_id}-sidebar`), in its
///   pre-remap form; C3 wiring resolves it to the live id via the
///   document tree.
/// - `String` — the main column frame id (`{root_id}-main`), same
///   resolution rule.
/// - `usize` — scaffold descendant-count baseline, equal to what
///   [`crate::cleanup::descendant_count`] returns for the root frame
///   after the `InsertSubtree` applies. Includes the sidebar + main +
///   every synthesized row + every synthesized slot, mirroring TS
///   `scaffoldCounts.set(rootId, countDescendants(root))` in
///   `orchestrator.ts:1038-1053`. Callers compare a later live count
///   against this baseline to decide whether a root is scaffold-only.
#[allow(dead_code)] // consumed by run.rs wiring in S3b-3 Task C3
pub(crate) type DashboardScaffoldResult = (Vec<EditorCommand>, String, String, usize);

/// Builds the dashboard-column scaffold variant (S3b-3 Task C2).
///
/// # Layout
/// - Root frame: `layout:"horizontal"`, `gap:0`,
///   `height:"fit_content({placeholder})"`.
/// - Sidebar child: `id={root_id}-sidebar`, `width:260`, `height:"fit_content"`,
///   `layout:"vertical"`, `fill:[{type:"solid",color:sidebar_fill_hex}]`.
/// - Main child: `id={root_id}-main`, `width:"fill_container"`,
///   `height:"fit_content"`, `layout:"vertical"`, `gap:content_gap`
///   (`root.gap > 0 ? root.gap : 20`).
/// - Row/slot frames synthesized by `assign_dashboard_main_parents` are
///   **embedded as nested children** of the main frame in the same
///   `InsertSubtree` command — `cmd_insert_subtree` remaps every node
///   id atomically, so emitting follow-up commands that reference
///   `main_id` as `parent_id` would fail (the logical id no longer
///   exists post-remap). One command keeps every parent / child
///   reference inside the subtree coherent.
///
/// # Side effect on plan
/// Mutates `plan.subtasks[*].parent_frame_id` via `assign_dashboard_main_parents`.
/// The written ids are the pre-remap logical ids; C3 wiring resolves
/// them to live ids by walking the document.
///
/// # Errors
/// Returns `Err` only on internal JSON template bugs (non-user-input errors).
#[allow(dead_code)] // consumed by run.rs wiring in S3b-3 Task C3
pub(crate) fn build_scaffold_dashboard(
    plan: &mut OrchestratorPlan,
    sidebar_fill_hex: &str,
) -> Result<DashboardScaffoldResult, String> {
    let rf = &plan.root_frame;
    let root_id = rf.id.clone();
    let fill_hex = rf
        .first_solid_hex()
        .unwrap_or_else(|| "#FFFFFF".to_string());

    // Placeholder height for the root frame's fit_content expression.
    let placeholder = get_dashboard_placeholder_height(plan);

    // content gap for the main column.
    let content_gap = match plan.root_frame.gap {
        Some(g) if g > 0.0 => g,
        _ => 20.0,
    };

    let sidebar_id = format!("{root_id}-sidebar");
    let main_id = format!("{root_id}-main");

    // Synthesize row + slot frames first so we can embed them as nested
    // children of `main` in the same `InsertSubtree`.  Embedding (rather
    // than queueing follow-up `InsertSubtree` commands per row/slot)
    // matters because `cmd_insert_subtree` remaps every node id, so a
    // logical `main_id` cannot be referenced as a `parent_id` after the
    // first apply — only nested children keep their relationships
    // through the remap.  Side effect: `plan.subtasks[*].parent_frame_id`
    // is written with the LOGICAL (pre-remap) ids; C3 wiring resolves
    // them to live ids by walking the document after apply.
    let row_frame_entries = assign_dashboard_main_parents(plan, &main_id)?;

    // Bucket entries by parent_id so we can construct nested `children`
    // arrays.  Order is preserved (vec, not map).
    let main_children = build_main_children(&main_id, &row_frame_entries)?;

    // Build sidebar + main child nodes.
    let sidebar_node_json = serde_json::json!({
        "type": "frame",
        "id": sidebar_id,
        "name": "Sidebar",
        "width": 260,
        "height": "fit_content",
        "layout": "vertical",
        "fill": [{ "type": "solid", "color": sidebar_fill_hex }],
        "children": [],
    });
    let main_node_json = serde_json::json!({
        "type": "frame",
        "id": main_id,
        "name": "Main Content",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": content_gap,
        "children": main_children,
    });

    // Root frame: horizontal, gap 0, fit_content(placeholder).
    let root_json = serde_json::json!({
        "type": "frame",
        "id": root_id,
        "name": plan.root_frame.name.clone(),
        "x": 0,
        "y": 0,
        "width": plan.root_frame.width,
        "height": format!("fit_content({placeholder})"),
        "layout": "horizontal",
        "gap": 0,
        "fill": [{ "type": "solid", "color": fill_hex }],
        "children": [sidebar_node_json, main_node_json],
    });

    let root_node: PenNode =
        serde_json::from_value(root_json).map_err(|e| format!("dashboard root frame: {e}"))?;

    // Baseline = recursive descendants of the root subtree, equivalent
    // to what `descendant_count(state, root_id)` returns after the
    // InsertSubtree applies (sidebar + main + all rows + all slots).
    let scaffold_baseline = count_descendants(&root_node);

    let cmds: Vec<EditorCommand> = vec![EditorCommand::InsertSubtree {
        nodes: vec![root_node],
        parent_id: NodeId::NONE,
    }];

    Ok((cmds, sidebar_id, main_id, scaffold_baseline))
}

/// Convert the flat `(node, parent_id)` list from `assign_dashboard_main_parents`
/// into a nested `serde_json::Value` array suitable for `main.children`.
///
/// Entries parented to `main_id` become top-level children.  Entries
/// parented to a row frame become children of that row.  Order matches
/// the entry order (row frame emitted first, then its slot children).
fn build_main_children(
    main_id: &str,
    entries: &[(PenNode, String)],
) -> Result<Vec<serde_json::Value>, String> {
    use serde_json::Value;

    // Pass 1: serialise every entry to JSON and bucket by parent id.
    let mut main_level: Vec<Value> = Vec::new();
    // Parallel arrays: `row_id` and the row's accumulated `children` Value list.
    let mut row_ids: Vec<String> = Vec::new();
    let mut row_children: Vec<Vec<Value>> = Vec::new();
    let mut row_index_by_id: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for (node, parent_id) in entries {
        let node_json: Value =
            serde_json::to_value(node).map_err(|e| format!("serialise row/slot: {e}"))?;
        let node_id = node.id_str().to_string();

        if parent_id == main_id {
            // Direct child of main — could be a slot (standalone row) or a
            // row frame (multi-subtask row).  Record its index so
            // subsequent slot entries can be appended to its `children`.
            row_ids.push(node_id.clone());
            row_children.push(Vec::new());
            row_index_by_id.insert(node_id, row_ids.len() - 1);
            main_level.push(node_json);
        } else if let Some(&idx) = row_index_by_id.get(parent_id) {
            // Slot inside a previously-emitted row frame.
            row_children[idx].push(node_json);
        } else {
            return Err(format!(
                "unknown parent_id `{parent_id}` for entry `{node_id}`"
            ));
        }
    }

    // Pass 2: stitch each row's collected slot children onto its node's
    // `children` array.
    for (idx, row_id) in row_ids.iter().enumerate() {
        if row_children[idx].is_empty() {
            continue; // standalone slot — no nested slots to stitch
        }
        // Order: main_level[i] corresponds to row_ids[i].
        if let Some(node_obj) = main_level[idx].as_object_mut() {
            node_obj.insert(
                "children".into(),
                Value::Array(std::mem::take(&mut row_children[idx])),
            );
        } else {
            return Err(format!("row `{row_id}` did not serialise as object"));
        }
    }

    Ok(main_level)
}

#[cfg(test)]
#[path = "scaffold_tests_c2.rs"]
mod tests_c2;
