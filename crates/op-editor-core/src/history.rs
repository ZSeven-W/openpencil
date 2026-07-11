//! Undo / redo stacks for the editor.
//!
//! Models `openpencil-shell-core::document`'s `History` +
//! `DocumentSnapshot` pair, retargeted onto the canonical document
//! model: a snapshot is a deep copy of the editable subset —
//! `PenDocument` (which now carries the node tree, pages, variables
//! and themes) plus the selection and active-page index.
//!
//! In shell-core the snapshot had a separate `var_table` field because
//! variables lived outside the node model; here they are part of
//! `PenDocument`, so cloning the document captures them automatically
//! (spec §5.2 — single canonical document model, no second table).
//!
//! Mutators push onto `past` BEFORE a transactional edit; this task is
//! types-only (Task 4.5 ports the mutator `impl`s).

use crate::history_snapshot::{SharedComponents, SharedDoc};
use crate::selection::SelectionState;
use std::collections::VecDeque;

/// Largest number of undo entries kept. Past this the oldest entry is
/// dropped (`VecDeque::pop_front`) — matches shell-core's cap.
pub const HISTORY_CAP: usize = 100;

/// Snapshot of the editor state covered by undo / redo.
///
/// Holds a **structurally-shared** view of the canonical document
/// ([`SharedDoc`] — top-level nodes shared by `Arc` across adjacent
/// snapshots) and the component prototypes ([`SharedComponents`]) plus
/// the transient registries whose behavior depends on document edits.
/// A variable-table edit can therefore be undone the same way a node
/// edit can, and component promotion stays in sync with the persisted
/// `reusable` flag. See [`crate::history_snapshot`] for the sharing +
/// copy-on-write rules; snapshots materialize back to an owned
/// `PenDocument` on every restore.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorSnapshot {
    /// The canonical document at snapshot time, shared at top-level-node
    /// granularity.
    pub doc: SharedDoc,
    /// Selection at snapshot time.
    pub selection: SelectionState,
    /// Active page index at snapshot time.
    pub active_page_index: usize,
    /// Runtime component registry mirrored from reusable document nodes
    /// and explicit component commands, shared by `Arc` per prototype.
    pub components: SharedComponents,
    /// `MergeAppState` ownership map (`key → owning plan_idx`) at
    /// snapshot time. Must travel with the snapshot: doc.state is
    /// restored on undo / batch rollback, so ownership left behind
    /// would mark keys as generation-owned that the restored document
    /// no longer carries — later merges would be silently skipped or
    /// mis-resolved against a stale owner.
    pub app_state_owner: std::collections::BTreeMap<String, usize>,
    /// Document revision at snapshot time. Restoring it lets undo back
    /// to a saved snapshot clear the dirty marker naturally.
    pub revision: u64,
}

/// Editor undo / redo stacks. `VecDeque` so the over-cap eviction is an
/// O(1) `pop_front` rather than an O(n) `Vec::remove(0)`.
#[derive(Debug, Clone, Default)]
pub struct History {
    pub past: VecDeque<EditorSnapshot>,
    pub future: VecDeque<EditorSnapshot>,
}

impl History {
    /// An empty history — no undo, no redo.
    pub fn new() -> Self {
        Self::default()
    }

    /// True when there is at least one undo entry.
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    /// True when there is at least one redo entry.
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_history_has_no_undo_or_redo() {
        let h = History::new();
        assert!(!h.can_undo());
        assert!(!h.can_redo());
    }
}
