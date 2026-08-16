//! Atomic installation seam for a preflighted scene-template adoption.

use super::*;

impl WidgetHostNative {
    /// Commit a scene template state that the caller built and validated from
    /// a clone of the live state. Collaboration is re-checked at the commit
    /// boundary; rejection leaves the document and every derived cache intact.
    pub fn install_scene_template_state(
        &mut self,
        state: op_editor_core::EditorState,
        replaces_starter: bool,
    ) -> Result<(), Box<op_editor_core::EditorState>> {
        let action = if replaces_starter {
            op_editor_core::CollabGateAction::ReplaceDocument
        } else {
            op_editor_core::CollabGateAction::Document(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::BulkWrite,
                ),
            )
        };
        if !self.collab_allows_user_action(action) {
            return Err(Box::new(state));
        }

        self.editor_state = state;
        self.layout_transition = None;
        if replaces_starter {
            // Replacing the untouched starter supersedes any async work that
            // was dispatched for that placeholder document.
            self.document_epoch = self.document_epoch.wrapping_add(1);
        }
        self.force_rotate_layer_panel_owner();
        self.scene_cache.invalidate();
        self.editor_state_dirty = true;
        self.drop_pan_cache();
        self.arm_missing_fonts_detection();
        Ok(())
    }
}
