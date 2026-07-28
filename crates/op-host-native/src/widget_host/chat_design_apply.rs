//! Thin wrapper over the shared chat design-block apply
//! (`op_editor_core::host_ui_transitions`); parsing stays host-side
//! because `parse_design_json_nodes` lives behind the widget facade.

use super::WidgetHostNative;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_chat_design_block(
        &mut self,
        message_index: usize,
        code: &str,
    ) -> bool {
        if !self.collab_allows_document_mutation_from(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::BulkWrite,
            ),
            op_editor_core::CollabEditSource::Ai,
        ) {
            return true;
        }
        let Ok(nodes) = op_editor_ui::widgets::parse_design_json_nodes(code) else {
            return true;
        };
        if op_editor_core::host_ui_transitions::apply_chat_design_block(
            &mut self.editor_state,
            message_index,
            nodes,
        ) {
            self.mark_dirty();
        }
        true
    }
}
