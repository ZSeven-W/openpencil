//! Semantic-upgrade migration used by
//! `EditorCommand::PromoteLegacyWidgets`.
//!
//! Promotes explicitly-marked legacy frames (`role:"input"` /
//! `semantics.role == Input`, etc.) into first-class widget nodes
//! (`text_input` / `text_area` / `select` / `switch` / `checkbox` /
//! `slider`). The actual node-by-node rewrite lives in
//! `jian_ops_schema::promote`; this module wraps it with the editor's
//! history discipline so the whole rewrite is a single undoable step.

use crate::state::EditorState;
use jian_ops_schema::promote::{promote_document, PromoteNote};

/// Result of one [`EditorState::promote_legacy_widgets`] run — the count
/// of promoted nodes plus one note per promotion (id + from-role + new
/// widget tag), so a host can report exactly what migrated.
#[derive(Debug, Clone, Default)]
pub struct PromoteResult {
    /// One note per promotion, in document order.
    pub notes: Vec<PromoteNote>,
}

impl PromoteResult {
    /// Number of frames promoted to widget nodes.
    pub fn count(&self) -> usize {
        self.notes.len()
    }

    /// True when at least one frame was promoted.
    pub fn changed(&self) -> bool {
        !self.notes.is_empty()
    }
}

impl EditorState {
    /// Run the "semantic upgrade" migration over the whole document:
    /// promote explicitly-marked legacy frames into first-class widget
    /// nodes, persisting the rewrite into the document.
    ///
    /// History discipline matches every other batch mutation
    /// (`RefineDesign` / `DeleteSelected` / …): a snapshot is taken
    /// BEFORE the rewrite, but it is only pushed onto the undo stack
    /// when at least one node is promoted — a zero-promotion run never
    /// pollutes the undo stack with a no-op.
    ///
    /// Returns the [`PromoteResult`] (count + per-node notes) so the
    /// host can report how many nodes migrated.
    pub fn promote_legacy_widgets(&mut self) -> PromoteResult {
        let snap = self.snapshot_for_history();
        // `promote_document` walks `doc.children` + every `page.children`
        // + nested container children, replacing marked frames in place.
        let notes = promote_document(&mut self.doc);
        if !notes.is_empty() {
            self.history_push_past(snap);
        }
        PromoteResult { notes }
    }
}
