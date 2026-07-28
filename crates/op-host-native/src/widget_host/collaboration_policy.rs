//! Native-host projection of the shared collaboration mutation policy.
//!
//! The policy itself stays platform-neutral in `op-editor-core`. This thin
//! adapter only turns a typed rejection into bounded UI state so every native
//! input surface fails closed and explains why it was rejected.

use super::WidgetHostNative;
use op_editor_core::{
    CollabDocumentMutation, CollabEditSource, CollabGateAction, CollabGateReason, CollabNoticeKind,
    CollabRejectUiCode, IdAllocError,
};

impl WidgetHostNative {
    /// Desktop-owned async/file/MCP drains use this public seam so a request
    /// re-checks the current role and phase immediately before it mutates.
    pub fn gate_collaboration_action(
        &mut self,
        action: CollabGateAction,
        source: CollabEditSource,
    ) -> bool {
        self.collab_allows_action_from(action, source)
    }

    pub(in crate::widget_host) fn collab_allows_user_action(
        &mut self,
        action: CollabGateAction,
    ) -> bool {
        self.collab_allows_action_from(action, CollabEditSource::User)
    }

    pub(in crate::widget_host) fn collab_allows_action_from(
        &mut self,
        action: CollabGateAction,
        source: CollabEditSource,
    ) -> bool {
        match self.editor_state.editor_ui.collab.gate(action, source) {
            Ok(()) => true,
            Err(reason) => {
                self.show_collab_gate_rejection(reason);
                false
            }
        }
    }

    pub(in crate::widget_host) fn collab_allows_document_mutation(
        &mut self,
        mutation: CollabDocumentMutation,
    ) -> bool {
        self.collab_allows_user_action(CollabGateAction::Document(mutation))
    }

    pub(in crate::widget_host) fn collab_allows_document_mutation_from(
        &mut self,
        mutation: CollabDocumentMutation,
        source: CollabEditSource,
    ) -> bool {
        self.collab_allows_action_from(CollabGateAction::Document(mutation), source)
    }

    pub(in crate::widget_host) fn collab_allows_color_picker_mutation(&mut self) -> bool {
        use op_editor_core::{
            ui_draft::ColorTarget, CollabNodeField as Field,
            CollabUnsupportedFeature as Unsupported,
        };
        let Some(picker) = self.editor_state.ui.color_picker.as_ref() else {
            return true;
        };
        let mutation = if picker.variable.is_some() {
            CollabDocumentMutation::Unsupported(Unsupported::VariablesThemes)
        } else {
            match picker.target {
                ColorTarget::Fill => CollabDocumentMutation::NodeProperty(Field::Fill),
                ColorTarget::Stroke => CollabDocumentMutation::NodeProperty(Field::Stroke),
                ColorTarget::GradientStop(_) => {
                    CollabDocumentMutation::Unsupported(Unsupported::UnsupportedNodeProperty)
                }
                ColorTarget::EffectColor(_) => {
                    CollabDocumentMutation::Unsupported(Unsupported::Effects)
                }
            }
        };
        self.collab_allows_document_mutation(mutation)
    }

    /// Blur live colour drafts without letting a role/phase change turn the
    /// final blur into an otherwise-ungated document write.
    pub(in crate::widget_host) fn collab_blur_color_picker_inputs(&mut self) {
        let has_focus = self.editor_state.color_picker_hex_focused()
            || self.editor_state.color_picker_rgb_focused();
        if !has_focus {
            return;
        }
        if self.collab_allows_color_picker_mutation() {
            self.editor_state.color_picker_blur_hex();
            self.editor_state.color_picker_blur_rgb();
            return;
        }
        // Each accepted keystroke already applied live. On a later downgrade,
        // discard only the stale input focus/draft commit, preserving the last
        // accepted document state.
        if let Some(picker) = self.editor_state.ui.color_picker.as_mut() {
            picker.hex_focused = false;
            picker.rgb_focus = None;
        }
    }

    pub(in crate::widget_host) fn collab_allows_variables_mutation(&mut self) -> bool {
        self.collab_allows_document_mutation(CollabDocumentMutation::Unsupported(
            op_editor_core::CollabUnsupportedFeature::VariablesThemes,
        ))
    }

    pub(in crate::widget_host) fn show_collab_id_error(&mut self, error: IdAllocError) {
        let code = match error {
            IdAllocError::CounterExhausted => CollabRejectUiCode::ResourceLimit,
        };
        self.editor_state
            .editor_ui
            .collab
            .set_notice(CollabNoticeKind::Reject(code), self.now_ms);
        self.mark_dirty();
    }

    fn show_collab_gate_rejection(&mut self, reason: CollabGateReason) {
        self.editor_state
            .editor_ui
            .collab
            .set_notice(reason.notice_kind(), self.now_ms);
        self.mark_dirty();
    }
}
