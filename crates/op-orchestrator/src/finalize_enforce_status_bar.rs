//! Finalize-time OS status-bar contract enforcement — ensures every mobile
//! screen root carries exactly one canonical status bar as its first child.
//!
//! **Rationale:** Measured defect shows different models produce different
//! status-bar outcomes:
//! - GLM-5.3: canonical bar (role="status-bar", children Time + Levels)
//! - Kimi K3: model-built bar (no role, custom children)
//! - Gemini 3.7 / DeepSeek: no bar at all
//!
//! Root-seeding only runs at root-creation time via `root_seed.rs`, which can be
//! escaped by:
//! - A model that builds its own bar in the same batch as the root (replaces the injected one)
//! - A model that never builds one and whose root has `height: "fit_content"` (skips injection)
//!
//! This pass runs at the END of finalize to enforce the contract deterministically:
//! every mobile screen root must carry exactly one canonical status bar as first child.
//!
//! ## Screen-root predicate
//!
//! Reuses the same mobile-artboard shape check from `unify_shared_status_bar`:
//! - Numeric height: [`crate::unfilled_screens::list_screen_candidates`]
//! - Fit-content height: resolved layout ≥ 480px tall (generated mobile roots)
//!
//! ## Three cases
//!
//! For each mobile root:
//! 1. **No status-bar child** → insert canonical bar at index 0
//! 2. **Non-canonical status bar** → replace it in place
//! 3. **Already canonical** → untouched, zero commands emitted
//!
//! The canonical predicate: `role: "status-bar"` + direct child named `Levels`.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, EditorState, NodeId, PenNodeExt};
use std::collections::HashSet;

use crate::cleanup::is_status_bar;
use crate::types::DocSink;
use crate::unfilled_screens::list_screen_candidates;
use crate::unify_shared_status_bar::looks_like_fit_content_mobile_root;

/// Entry point. Runs on every mobile screen root to enforce the status-bar contract.
pub fn finalize_enforce_status_bar_contract(sink: &mut dyn DocSink) {
    let screens = list_finalize_status_bar_screen_candidates(sink.state());
    if screens.is_empty() {
        return;
    }

    for screen_id in screens {
        enforce_status_bar_on_root(sink, &screen_id);
    }
}

/// Collect all mobile screen-shaped roots using the same gate as
/// `unify_shared_status_bar` (numeric + fit_content with resolved height).
fn list_finalize_status_bar_screen_candidates(state: &EditorState) -> Vec<String> {
    let numeric_ids: HashSet<String> = list_screen_candidates(state)
        .into_iter()
        .map(|screen| screen.node_id)
        .collect();

    let fit_content_ids: HashSet<&str> = state
        .active_children()
        .iter()
        .filter(|node| looks_like_fit_content_mobile_root(node))
        .map(PenNodeExt::id_str)
        .collect();

    if fit_content_ids.is_empty() {
        return state
            .active_children()
            .iter()
            .filter(|node| numeric_ids.contains(node.id_str()))
            .map(|node| node.id_str().to_string())
            .collect();
    }

    // Resolve fit_content roots against real layout to filter only those tall enough.
    let scene = op_pen_loader::editor_state_to_layout_scene(state);
    let resolved_fit_content_ids: HashSet<&str> = scene
        .active_page()
        .map(|page| {
            page.children
                .iter()
                .filter(|node| fit_content_ids.contains(node.id.as_str()))
                .filter(|node| {
                    let height = f64::from(node.bounds.size.y);
                    height.is_finite() && height >= 480.0
                })
                .map(|node| node.id.as_str())
                .collect()
        })
        .unwrap_or_default();

    state
        .active_children()
        .iter()
        .filter(|node| {
            numeric_ids.contains(node.id_str()) || resolved_fit_content_ids.contains(node.id_str())
        })
        .map(|node| node.id_str().to_string())
        .collect()
}

/// Enforce the status-bar contract on a single root:
/// - No status bar → insert canonical bar at index 0
/// - Non-canonical → replace it
/// - Canonical → leave untouched
fn enforce_status_bar_on_root(sink: &mut dyn DocSink, root_id: &str) {
    let root = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    );

    let Some(root) = root else {
        return;
    };

    // Find the first direct child that looks like a status bar (either canonical or not).
    let existing_status_bar_index = root
        .children()
        .and_then(|children| children.iter().position(is_status_bar));

    match existing_status_bar_index {
        Some(index) => {
            // There's a status-bar-like node at this index. Check if it's canonical.
            if let Some(child) = root.children().and_then(|c| c.get(index)) {
                if !is_canonical_status_bar(child) {
                    // Replace the non-canonical bar with the canonical one.
                    replace_non_canonical_status_bar(sink, root_id, index);
                }
                // If already canonical, do nothing.
            }
        }
        None => {
            // No status bar at all. Insert a canonical one at index 0.
            insert_canonical_status_bar(sink, root_id);
        }
    }
}

/// Insert a canonical status bar at index 0 of the root.
fn insert_canonical_status_bar(sink: &mut dyn DocSink, root_id: &str) {
    let root = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    );

    let Some(root) = root else {
        return;
    };

    // Get root dimensions and fill for the status bar builder.
    let width = root.width_px().unwrap_or(390.0);
    let fill_hex = op_editor_core::first_solid_fill_hex(root)
        .unwrap_or("#ffffff")
        .to_string();

    // Build the canonical status bar.
    let Ok(mut bar) = crate::scaffold::mobile_status_bar_node(root_id, &fill_hex, width) else {
        return;
    };

    // Ensure the role is canonical.
    bar.base_mut().role = Some("status-bar".to_string());

    // Insert at index 0.
    sink.apply(EditorCommand::InsertSubtree {
        nodes: vec![bar],
        parent_id: NodeId::new(root_id.to_string()),
        page_id: None,
    });

    // Move the newly inserted bar to index 0 (InsertSubtree appends it as the last child).
    let Some(updated_root) = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        return;
    };

    if let Some(last_child) = updated_root.children().and_then(|c| c.last()) {
        let new_bar_id = NodeId::new(last_child.id_str().to_string());
        sink.apply(EditorCommand::MoveNode {
            node_id: new_bar_id,
            target_parent: NodeId::new(root_id.to_string()),
            page_id: None,
            index: Some(0),
        });
    }
}

/// Replace a non-canonical status bar at the given index with a canonical one.
fn replace_non_canonical_status_bar(sink: &mut dyn DocSink, root_id: &str, index: usize) {
    let root = op_editor_core::walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    );

    let Some(root) = root else {
        return;
    };

    let Some(children) = root.children() else {
        return;
    };

    if let Some(old_bar) = children.get(index) {
        let old_bar_id = old_bar.id_str().to_string();

        // Build the canonical replacement.
        let width = root.width_px().unwrap_or(390.0);
        let fill_hex = op_editor_core::first_solid_fill_hex(root)
            .unwrap_or("#ffffff")
            .to_string();

        let Ok(mut new_bar) = crate::scaffold::mobile_status_bar_node(root_id, &fill_hex, width)
        else {
            return;
        };

        // Ensure the role is canonical.
        new_bar.base_mut().role = Some("status-bar".to_string());

        // Delete the old non-canonical bar.
        sink.apply(EditorCommand::DeleteNode {
            node_id: NodeId::new(old_bar_id),
            page_id: None,
        });

        // Insert the new canonical bar at index 0 (it will be appended first, then moved).
        sink.apply(EditorCommand::InsertSubtree {
            nodes: vec![new_bar],
            parent_id: NodeId::new(root_id.to_string()),
            page_id: None,
        });

        // Move the newly inserted bar to index 0.
        let Some(updated_root) = op_editor_core::walkers::find_node(
            sink.state().active_children(),
            &NodeId::new(root_id.to_string()),
        ) else {
            return;
        };

        if let Some(last_child) = updated_root.children().and_then(|c| c.last()) {
            let new_bar_id = NodeId::new(last_child.id_str().to_string());
            sink.apply(EditorCommand::MoveNode {
                node_id: new_bar_id,
                target_parent: NodeId::new(root_id.to_string()),
                page_id: None,
                index: Some(0),
            });
        }
    }
}

/// Check if a node is a canonical status bar:
/// - Has `role: "status-bar"`
/// - Has a direct child named `Levels`
fn is_canonical_status_bar(node: &PenNode) -> bool {
    node.base().role.as_deref() == Some("status-bar")
        && node.children().is_some_and(|children| {
            children
                .iter()
                .any(|c| c.base().name.as_deref() == Some("Levels"))
        })
}

#[cfg(test)]
#[path = "finalize_enforce_status_bar_tests.rs"]
mod tests;
