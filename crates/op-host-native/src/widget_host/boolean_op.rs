//! Path boolean ops on the active selection (`apply_boolean_op`).
//!
//! Carved out of the `widget_host.rs` spine as pure code motion to keep
//! the spine under the repo's 800-line cap.

use super::*;

impl WidgetHostNative {
    /// Run a path boolean op on the active selection (Union /
    /// Subtract / Intersect / Exclude). Backed by skia's `Path::op`.
    /// Returns true when the op committed (≥ 2 Path nodes were
    /// selected + the result yielded a non-empty polyline).
    #[cfg(feature = "gl-host")]
    pub fn apply_boolean_op(&mut self, op: op_editor_core::BooleanOp) -> bool {
        // Codex stop-gate: boolean op shortcuts (Cmd+Alt+U/S/I/X)
        // mutate the document — commit any pending variable-row
        // edit first so the dirty draft lands before this op runs.
        self.commit_variable_row_focus_if_any();
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::NodeReplacement,
            ),
        ) {
            return true;
        }
        // The skia `Path::op` math runs against the layout-resolved
        // `LayoutScene` + the editor selection; the result polyline
        // is committed back through an `EditorState` mutator so the
        // host never edits the canonical tree directly.
        self.refresh_layout_scene();
        let selected: Vec<String> = self
            .editor_state
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let outcome = crate::boolean_ops::compute_boolean_op(&self.layout_scene, &selected, op);
        let Some(result) = outcome else {
            return false;
        };
        // Scene ids are the canonical `.op` ids — wrap straight into
        // `op_editor_core::NodeId`.
        let source_ids: Vec<op_editor_core::NodeId> = result
            .source_ids
            .iter()
            .map(op_editor_core::NodeId::new)
            .collect();
        let pre = self.editor_state.snapshot_for_history();
        let new_id = if let Some(allocator) = self.collab_id_allocator.as_mut() {
            self.editor_state
                .replace_paths_with_polyline_with_allocator(
                    &source_ids,
                    &result.contours,
                    allocator,
                )
        } else {
            Ok(self.editor_state.replace_paths_with_polyline(
                &source_ids,
                &result.contours,
                &mut self.next_node_id,
            ))
        };
        match new_id {
            Err(error) => {
                self.show_collab_id_error(error);
                true
            }
            Ok(Some(id)) => {
                self.editor_state.history_push_past(pre);
                self.editor_state.set_single_selection(id);
                self.mark_dirty();
                true
            }
            Ok(None) => false,
        }
    }
}
