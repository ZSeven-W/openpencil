//! Per-subtask scoped diagnostics for the `geometry_echo` in-loop
//! self-correction step (`concurrent::run_subtask_retry_ladder`).
//!
//! [`super::geometry_diagnostics`] walks the WHOLE document (every
//! top-level page root) and also runs the two page-root-specific bottom-nav
//! checks — appropriate for a post-run/cleanup-time sweep, but wrong for an
//! in-loop echo that must judge only the subtask that JUST landed (a
//! sibling subtask's pre-existing violations aren't this subtask's fault,
//! and the bottom-nav checks only make sense against the actual page root,
//! not an arbitrary inserted subtree). This module scopes the SAME
//! recursive detector (`collect_diagnostics`) to exactly the node ids one
//! subtask inserted.

use op_editor_core::{EditorState, NodeId, PenNodeExt};

use super::text_collision::push_text_collision_diagnostics;
use super::{
    collect_diagnostics, push_interaction_backfill_diagnostics, resolved_rects, MAX_DIAGNOSTICS,
};

/// Geometry diagnostics for exactly the subtrees rooted at `root_ids` —
/// the `inserted_root_ids` one subtask's `InsertSubtree` produced. Runs the
/// REAL resolved layout (same `resolved_rects` pass `geometry_diagnostics`
/// and `geometry_validate_and_fix` use) against the CURRENT `state`, so it
/// only ever sees something real to report when `state` already reflects
/// the insert — callers on a buffered sink (whose `state()` is a frozen
/// pre-phase snapshot) will get an empty result here rather than a stale
/// or wrong one, since `find_node` simply won't resolve the fresh id.
pub(crate) fn geometry_diagnostics_for_roots(
    state: &EditorState,
    root_ids: &[String],
) -> Vec<String> {
    let rects = resolved_rects(state);
    let mut out = Vec::new();
    push_interaction_backfill_diagnostics(state, Some(root_ids), &mut out);
    for root_id in root_ids {
        if out.len() >= MAX_DIAGNOSTICS {
            break;
        }
        let Some(root) = op_editor_core::walkers::find_node(
            state.active_children(),
            &NodeId::new(root_id.clone()),
        ) else {
            continue;
        };
        let Ok(v) = serde_json::to_value(root) else {
            continue;
        };
        if state
            .active_children()
            .iter()
            .any(|node| node.id_str() == root_id)
        {
            super::geometry_board_containment::collect(&v, &rects, &mut out);
        }
        push_text_collision_diagnostics(&v, &rects, &mut out);
        collect_diagnostics(&v, &rects, &mut out);
    }
    out.truncate(MAX_DIAGNOSTICS);
    out
}

#[cfg(test)]
#[path = "geometry_echo_tests.rs"]
mod tests;
