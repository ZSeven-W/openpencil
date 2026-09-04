//! Thin wrappers over the shared settings-modal input transitions
//! (`op_editor_core::host_ui_transitions`).

use op_editor_core::host_ui_transitions as shared;

use super::WidgetHostNative;

impl WidgetHostNative {
    pub(in crate::widget_host) fn apply_settings_text(&mut self, c: char) -> bool {
        if shared::settings_text(&mut self.editor_state.editor_ui, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub(in crate::widget_host) fn apply_settings_text_payload(&mut self, text: &str) -> bool {
        if shared::settings_text_payload(&mut self.editor_state.editor_ui, text, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub(in crate::widget_host) fn apply_settings_backspace(&mut self) -> bool {
        if shared::settings_backspace(&mut self.editor_state.editor_ui, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub(in crate::widget_host) fn apply_settings_delete_forward(&mut self) -> bool {
        if shared::settings_delete_forward(&mut self.editor_state.editor_ui, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_settings_caret(&mut self, forward: bool) -> bool {
        if shared::settings_caret(&mut self.editor_state.editor_ui, forward, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub(in crate::widget_host) fn clear_settings_caret(&mut self) {
        self.editor_state.editor_ui.settings_input.set_text("");
    }
}
