//! VariablesPanel draft commits — theme/variant header renames and the
//! per-row cell drafts (Name / Number / String / inline Color hex).
//!
//! The whole walk lives in the shared
//! `op_editor_core::host_variables_commit` (the web twin drives the same
//! functions); this file is only the `mark_dirty()` tail.

use super::WidgetHostNative;
use op_editor_core::host_variables_commit as vars_commit;

impl WidgetHostNative {
    /// Commit any pending VariablesPanel theme/variant header rename.
    pub(in crate::widget_host) fn commit_variables_panel_header_focus_if_any(&mut self) {
        let has_focus = self
            .editor_state
            .editor_ui
            .variables_theme_rename_axis
            .is_some()
            || self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .is_some();
        if has_focus
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::VariablesThemes,
                ),
            )
        {
            if vars_commit::discard_header_focus(&mut self.editor_state) {
                self.mark_dirty();
            }
            return;
        }
        if vars_commit::commit_header_focus(&mut self.editor_state) {
            self.mark_dirty();
        }
    }

    pub(in crate::widget_host) fn variable_axis_value_for_variant(
        &self,
        variant: usize,
    ) -> Option<(String, String)> {
        vars_commit::variable_axis_value_for_variant(&self.editor_state, variant)
    }

    /// Commit any pending VariablesPanel row edit (Number / String).
    pub(in crate::widget_host) fn commit_variable_row_focus_if_any(&mut self) {
        if self.editor_state.editor_ui.variable_row_focus.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::Unsupported(
                    op_editor_core::CollabUnsupportedFeature::VariablesThemes,
                ),
            )
        {
            if vars_commit::discard_row_focus(&mut self.editor_state) {
                self.mark_dirty();
            }
            return;
        }
        if vars_commit::commit_row_focus(&mut self.editor_state) {
            self.mark_dirty();
        }
    }
}
