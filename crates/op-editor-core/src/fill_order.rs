//! Fill-layer reordering for canonical [`PenNode`] fill lists.

use crate::fills::{node_fills, node_fills_mut};
use crate::walkers::find_node_mut;
use crate::EditorState;
use jian_ops_schema::node::PenNode;

/// Move one fill from `from` to the final index `to`.
///
/// Invalid or identical indices return `false` without materializing an
/// absent fill list or otherwise changing the node.
pub fn move_fill(node: &mut PenNode, from: usize, to: usize) -> bool {
    let Some(len) = node_fills(node).map(Vec::len) else {
        return false;
    };
    if from == to || from >= len || to >= len {
        return false;
    }
    let fills = node_fills_mut(node).expect("validated fill-bearing node");
    let moved = fills.remove(from);
    fills.insert(to, moved);
    true
}

impl EditorState {
    /// Reorder a selected node's fills as one undoable document edit.
    ///
    /// The variable side table binds a node's primary fill rather than a
    /// particular fill layer. Crossing index 0 therefore clears that cache;
    /// the authored `$token`, when present, remains on the fill that moved.
    pub fn move_selected_fill(&mut self, from: usize, to: usize) -> bool {
        let selected = self.selection.anchor.clone();
        if !selected.is_real() || !self.is_editable(&selected) {
            return false;
        }
        let snapshot = self.snapshot_for_history();
        let Some(node) = find_node_mut(self.active_children_mut(), &selected) else {
            return false;
        };
        if !move_fill(node, from, to) {
            return false;
        }
        if from == 0 || to == 0 {
            self.ui.variables.fill_refs.remove(&selected);
        }
        self.history_push_past(snapshot);
        true
    }
}
