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
use crate::node_id::NodeId;
use crate::selection::SelectionState;
use crate::state::EditorState;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Document-dependent comparison state. Camera and other view-only state stay
/// outside history; picking a direction must be undoable as a usable comparison.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkspaceDirectionsSnapshot {
    run_epoch: u64,
    count: u8,
    variants: Arc<Vec<crate::WorkspaceVariant>>,
}

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
    /// Primary fill-variable cache at snapshot time. The cache is
    /// document-derived but must round-trip with undo when an edit changes
    /// which authored fill occupies index 0.
    pub fill_refs: HashMap<NodeId, String>,
    /// Stroke-variable companion to [`Self::fill_refs`]. Keeping both caches
    /// together prevents a history restore from producing asymmetric token
    /// resolution state.
    pub stroke_refs: HashMap<NodeId, String>,
    /// Whether the current document must keep its authored coordinates instead
    /// of running flex layout. Layout mutations can clear this document-level
    /// latch, so undo / redo must restore it alongside the document snapshot.
    pub preserve_authored_geometry: bool,
    pub(crate) workspace_directions: Option<WorkspaceDirectionsSnapshot>,
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

impl EditorState {
    /// Group a synchronous document operation into one undo step. Individual
    /// validated writes retain their normal behavior; only intermediate history
    /// entries are collapsed and revision-only writes gain a recovery snapshot.
    /// The closure must not call undo/redo or reset the
    /// document. A no-op preserves both history stacks, including existing redo.
    pub fn with_history_group<T>(&mut self, operation: impl FnOnce(&mut Self) -> T) -> T {
        let before = self.snapshot_for_history();
        let history = self.history.clone();
        let result = operation(self);
        if self.revision != before.revision {
            // Restore the old stacks before pushing the group so reaching the
            // cap inside a large operation cannot evict unrelated past entries.
            self.history = history;
            self.history_push_past(before);
        }
        result
    }

    /// Snapshot the editor's undoable state without pushing it.
    ///
    /// The snapshot covers the document, selection, active page, component
    /// registry, and transient variable-reference caches. View-only UI state
    /// such as collapsed layers is intentionally excluded.
    pub fn snapshot_for_history(&self) -> EditorSnapshot {
        // After an undo, the redo back is the state we came from and therefore
        // the closest sharing anchor. Normal forward editing falls through to
        // the previous undo entry.
        let anchor = self
            .history
            .future
            .back()
            .or_else(|| self.history.past.back());
        self.snapshot_for_history_with_anchor(anchor)
    }

    /// [`snapshot_for_history`](Self::snapshot_for_history) with an explicit
    /// adjacent snapshot whose unchanged top-level subtrees may be shared.
    pub fn snapshot_for_history_with_anchor(
        &self,
        anchor: Option<&EditorSnapshot>,
    ) -> EditorSnapshot {
        EditorSnapshot {
            doc: SharedDoc::capture(&self.doc, anchor.map(|snapshot| &snapshot.doc)),
            selection: self.selection.clone(),
            active_page_index: self.ui.active_page_index,
            components: SharedComponents::capture(
                &self.components,
                anchor.map(|snapshot| &snapshot.components),
            ),
            app_state_owner: self.app_state_owner.clone(),
            fill_refs: self.ui.variables.fill_refs.clone(),
            stroke_refs: self.ui.variables.stroke_refs.clone(),
            preserve_authored_geometry: self.editor_ui.preserve_authored_geometry,
            workspace_directions: self.capture_workspace_directions(anchor),
            revision: self.revision,
        }
    }

    /// Push a snapshot onto the undo stack, clear redo, and enforce the cap.
    pub fn history_push_past(&mut self, snapshot: EditorSnapshot) {
        self.history_push_count = self.history_push_count.saturating_add(1);
        self.history.past.push_back(snapshot);
        if self.history.past.len() > HISTORY_CAP {
            self.history.past.pop_front();
        }
        self.history.future.clear();
        self.mark_document_changed();
    }

    /// Push the current state before a transactional edit.
    pub fn commit_history(&mut self) {
        let snapshot = self.snapshot_for_history();
        self.history_push_past(snapshot);
    }

    /// Materialize and restore all undoable editor state from a snapshot.
    pub(crate) fn restore(&mut self, snapshot: EditorSnapshot) {
        self.doc = snapshot.doc.materialize();
        self.selection = snapshot.selection;
        self.ui.active_page_index = snapshot.active_page_index;
        self.components = snapshot.components.materialize();
        self.app_state_owner = snapshot.app_state_owner;
        self.ui.variables.fill_refs = snapshot.fill_refs;
        self.ui.variables.stroke_refs = snapshot.stroke_refs;
        self.editor_ui.preserve_authored_geometry = snapshot.preserve_authored_geometry;
        self.restore_workspace_directions(snapshot.workspace_directions);
        self.revision = snapshot.revision;
        self.sync_dirty_flag();
    }

    fn capture_workspace_directions(
        &self,
        anchor: Option<&EditorSnapshot>,
    ) -> Option<WorkspaceDirectionsSnapshot> {
        let workspace = &self.editor_ui.workspace;
        if !workspace.active {
            return None;
        }
        let variants = anchor
            .and_then(|snapshot| snapshot.workspace_directions.as_ref())
            .filter(|snapshot| snapshot.variants.as_ref() == &workspace.variants)
            .map(|snapshot| Arc::clone(&snapshot.variants))
            .unwrap_or_else(|| Arc::new(workspace.variants.clone()));
        Some(WorkspaceDirectionsSnapshot {
            run_epoch: workspace.run_epoch,
            count: workspace.variant_count,
            variants,
        })
    }

    fn restore_workspace_directions(&mut self, snapshot: Option<WorkspaceDirectionsSnapshot>) {
        let Some(snapshot) = snapshot else { return };
        let workspace = &mut self.editor_ui.workspace;
        // History from another run must never bring its old chooser into a new
        // generation. Professional/reading mode transitions remain view state.
        if !workspace.active || workspace.run_epoch != snapshot.run_epoch {
            return;
        }
        if workspace.variant_count == snapshot.count
            && &workspace.variants == snapshot.variants.as_ref()
        {
            return;
        }
        workspace.variant_count = snapshot.count;
        workspace.variants = snapshot.variants.as_ref().clone();
        workspace.view = if workspace.variants.is_empty() {
            crate::WorkspaceView::default_for(workspace.family)
        } else {
            crate::WorkspaceView::AllBoards
        };
        workspace.selected = 0;
        workspace.fitted_board_count = 0;
        workspace.fitted_bounds = None;
    }

    /// Undo the last change. Returns false when the undo stack is empty.
    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.history.past.pop_back() else {
            return false;
        };
        let current = self.snapshot_for_history_with_anchor(Some(&previous));
        self.history.future.push_back(current);
        self.restore(previous);
        true
    }

    /// Redo the last undone change. Returns false when redo is empty.
    pub fn redo(&mut self) -> bool {
        let Some(next) = self.history.future.pop_back() else {
            return false;
        };
        let current = self.snapshot_for_history_with_anchor(Some(&next));
        self.history.past.push_back(current);
        self.restore(next);
        true
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
