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

use crate::selection::SelectionState;
use std::collections::VecDeque;

/// Largest number of undo entries kept. Past this the oldest entry is
/// dropped (`VecDeque::pop_front`) — matches shell-core's cap.
pub const HISTORY_CAP: usize = 100;

/// Snapshot of the editor state covered by undo / redo.
///
/// Holds a full `PenDocument` clone (node tree + pages + variables +
/// themes) so a variable-table edit can be undone the same way a node
/// edit can — the shell-core stop-gate that forced `var_table` into
/// the old snapshot is satisfied for free here.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorSnapshot {
    /// The canonical document at snapshot time.
    pub doc: jian_ops_schema::PenDocument,
    /// Selection at snapshot time.
    pub selection: SelectionState,
    /// Active page index at snapshot time.
    pub active_page_index: usize,
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
