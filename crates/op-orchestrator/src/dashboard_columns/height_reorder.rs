//! §4.7 Placeholder height + child reorder.
//!
//! Split from `dashboard_columns.rs` to keep both files under the 800-line
//! ceiling. Faithful port of `orchestrator.ts:486-547`.

use super::{group_dashboard_main_rows, is_sidebar_subtask};
use crate::plan::OrchestratorPlan;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};

// ---------------------------------------------------------------------------
// §4.7a: `get_dashboard_placeholder_height`
// ---------------------------------------------------------------------------

/// Estimates the dashboard main-column placeholder height from the first 2-3
/// rows' tallest subtasks + the sidebar height.
///
/// Port of TS `getDashboardPlaceholderHeight` (`orchestrator.ts:486-508`).
///
/// Formula:
/// 1. `sidebarHeight = Σ max(0, st.region.height)` for sidebar subtasks.
/// 2. Group main rows via `group_dashboard_main_rows`.
/// 3. `visibleRows = rows[..min(rows.len(), if rows.len() >= 3 { 3 } else { 2 })]`.
/// 4. `foldHeight = Σ tallest-height-in-row + (visibleRows.len - 1) * rowGap`.
/// 5. `mainFoldHint = if !visibleRows.is_empty() { foldHeight } else { 560 }`.
/// 6. `sidebarHint = if sidebarHeight > 0 { min(sidebarHeight, 600) } else { 0 }`.
/// 7. Return `clamp(max(mainFoldHint, sidebarHint), 560, 680)`.
pub(crate) fn get_dashboard_placeholder_height(plan: &OrchestratorPlan) -> f64 {
    let row_groups = group_dashboard_main_rows(plan);
    let rows = &row_groups.rows;
    let row_gap = row_groups.row_gap;

    // Sidebar height: sum of max(0, height) for sidebar subtasks.
    let sidebar_height: f64 = plan
        .subtasks
        .iter()
        .filter(|st| is_sidebar_subtask(st))
        .map(|st| f64::max(0.0, st.region.height))
        .sum();

    // TS: `rows.slice(0, rows.length >= 3 ? 3 : 2)`
    let take = if rows.len() >= 3 { 3 } else { 2 };
    let visible_rows: Vec<&Vec<usize>> = rows.iter().take(take).collect();

    // foldHeight: sum per row of the max height among its subtasks + row gaps.
    let fold_height: f64 = visible_rows
        .iter()
        .map(|row| {
            // TS: `Math.max(0, ...row.map(st => typeof st.region.height === 'number' ? st.region.height : 0))`
            row.iter()
                .map(|&i| {
                    let h = plan.subtasks[i].region.height;
                    if h.is_finite() {
                        f64::max(0.0, h)
                    } else {
                        0.0
                    }
                })
                .fold(0.0_f64, f64::max)
        })
        .sum::<f64>()
        + f64::max(0.0, visible_rows.len() as f64 - 1.0) * row_gap;

    // TS: `mainFoldHint = visibleRows.length > 0 ? foldHeight : 560`
    let main_fold_hint = if visible_rows.is_empty() {
        560.0
    } else {
        fold_height
    };

    // TS: `sidebarHint = sidebarHeight > 0 ? Math.min(sidebarHeight, 600) : 0`
    let sidebar_hint = if sidebar_height > 0.0 {
        f64::min(sidebar_height, 600.0)
    } else {
        0.0
    };

    // TS: `Math.max(560, Math.min(Math.max(mainFoldHint, sidebarHint), 680))`
    f64::max(main_fold_hint, sidebar_hint).clamp(560.0, 680.0)
}

// ---------------------------------------------------------------------------
// §4.7b: `reorder_dashboard_main_children`
// ---------------------------------------------------------------------------

/// After sub-agent generation, re-sorts the main column's children to plan order.
///
/// For each non-sidebar subtask (in plan order), resolves which child of `main_id`
/// contains it (slot or row frame), then emits a `MoveNode` command for that
/// ancestor. Duplicate ancestors are deduplicated.
///
/// Port of TS `reorderDashboardMainChildren` (`orchestrator.ts:510-547`).
///
/// # Algorithm
/// For each non-sidebar subtask, the ancestor under `main_id` is resolved as:
/// 1. If `subtask.parent_frame_id.is_some() && != main_id`:
///    a. If that frame is a direct child of `main_id` → use it (slot = direct).
///    b. Else if *its* parent is a direct child of `main_id` → use that parent (row).
/// 2. Else → fall back to `subtask.generated_root_id`.
///
/// Each resolved ancestor id is pushed (once) to `desiredOrder`, then
/// `MoveNode { node_id: ancestor, target_parent: main_id }` is emitted for
/// every id in `desiredOrder` order — which causes `EditorState::apply` to
/// detach + re-append them in the desired sequence.
pub(crate) fn reorder_dashboard_main_children(
    plan: &OrchestratorPlan,
    main_id: &str,
    state: &EditorState,
) -> Vec<EditorCommand> {
    // Find the main node in the live document.
    let main_children: Vec<String> = {
        let top = state.active_children();
        // The main frame may be a top-level child or nested (inside the root scaffold).
        match find_children_of(top, main_id) {
            Some(ch) => ch.iter().map(|n| n.id_str().to_string()).collect(),
            None => return vec![],
        }
    };

    // Build desiredOrder.
    let mut desired_order: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let push_once =
        |id: &str, desired: &mut Vec<String>, seen: &mut std::collections::HashSet<String>| {
            if !id.is_empty() && !seen.contains(id) {
                seen.insert(id.to_string());
                desired.push(id.to_string());
            }
        };

    for subtask in &plan.subtasks {
        if is_sidebar_subtask(subtask) {
            continue;
        }

        if let Some(pfid) = subtask.parent_frame_id.as_deref() {
            if pfid != main_id {
                // Is pfid a direct child of main?
                if main_children.contains(&pfid.to_string()) {
                    push_once(pfid, &mut desired_order, &mut seen);
                    continue;
                }

                // Is pfid's parent a direct child of main? (pfid is a slot inside a row)
                let parent_of_pfid = find_parent_id_of(state.active_children(), pfid);
                if let Some(row_id) = parent_of_pfid {
                    if main_children.contains(&row_id) {
                        push_once(&row_id, &mut desired_order, &mut seen);
                        continue;
                    }
                }
            }
        }

        // Fallback: use generatedRootId.
        if let Some(root_id) = subtask.generated_root_id.as_deref() {
            push_once(root_id, &mut desired_order, &mut seen);
        }
    }

    // Emit MoveNode for each id in desiredOrder — applied in sequence, these
    // detach + re-append each child, producing the desired order.
    let main_node_id = NodeId::new(main_id.to_string());
    desired_order
        .into_iter()
        .map(|id| EditorCommand::MoveNode {
            node_id: NodeId::new(id),
            target_parent: main_node_id.clone(),
            page_id: None,
            index: None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Returns the children of the node with `target_id` anywhere in the forest,
/// or `None` when not found / not a container.
fn find_children_of<'a>(children: &'a [PenNode], target_id: &str) -> Option<&'a Vec<PenNode>> {
    for child in children {
        if child.id_str() == target_id {
            return child.children();
        }
        if let Some(grand) = child.children() {
            if let Some(found) = find_children_of(grand, target_id) {
                return Some(found);
            }
        }
    }
    None
}

/// Returns the id of the direct parent of `target_id` in the forest, or `None`.
fn find_parent_id_of(children: &[PenNode], target_id: &str) -> Option<String> {
    for child in children {
        if let Some(grand) = child.children() {
            if grand.iter().any(|n| n.id_str() == target_id) {
                return Some(child.id_str().to_string());
            }
            if let Some(id) = find_parent_id_of(grand, target_id) {
                return Some(id);
            }
        }
    }
    None
}
