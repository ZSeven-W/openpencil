//! Editor selection state.
//!
//! Models `openpencil-shell-core::document::Document`'s selection
//! fields (`selected` + `selected_set`) as a dedicated struct so the
//! `EditorState` surface stays flat.
//!
//! Selection is **page-scoped** by design: the editor UI only shows
//! one page at a time, so a selection outside the active page is
//! effectively no selection. The active page lives on `UiDraftState`.
//!
//! Anchor invariant: `anchor` is the last entry of `set` (the most
//! recently added id), or `NodeId::NONE` when `set` is empty.

use crate::node_id::NodeId;

/// The editor's current selection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SelectionState {
    /// Anchor selection — last entry of `set`, or `NodeId::NONE`.
    /// Property-panel single-node edits + handle drags key off this.
    pub anchor: NodeId,
    /// Full selection set. Invariant: `anchor` equals the last entry.
    pub set: Vec<NodeId>,
}

impl SelectionState {
    /// An empty selection — no anchor, no set.
    pub fn empty() -> Self {
        Self::default()
    }

    /// True when nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    /// Number of selected nodes.
    pub fn len(&self) -> usize {
        self.set.len()
    }

    /// True when `id` is in the selection set.
    pub fn contains(&self, id: &NodeId) -> bool {
        self.set.contains(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_selection_has_none_anchor() {
        let s = SelectionState::empty();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
        assert!(!s.anchor.is_real());
    }

    #[test]
    fn contains_checks_the_set() {
        let s = SelectionState {
            anchor: NodeId::new("n2"),
            set: vec![NodeId::new("n1"), NodeId::new("n2")],
        };
        assert!(s.contains(&NodeId::new("n1")));
        assert!(!s.contains(&NodeId::new("n9")));
        assert_eq!(s.len(), 2);
    }
}
