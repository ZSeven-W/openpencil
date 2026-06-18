//! Blur-on-blank-press — DOM-parity input defocus (web host).
//!
//! Mirrors the native host's `widget_host/blur_inputs.rs`: any press
//! that lands on blank chrome (a panel gap, a popover dismiss, the
//! bare canvas) commits + defocuses every text input at once instead
//! of leaving a stale caret eating keystrokes.

use super::WidgetHost;

impl WidgetHost {
    /// True when any chrome text input holds keyboard focus.
    fn any_text_input_focused(&self) -> bool {
        let eui = &self.editor_state.editor_ui;
        let git = &eui.git_panel;
        self.editor_state.ui.property_focus.is_some()
            || eui.effect_param_focus.is_some()
            || eui.variable_row_focus.is_some()
            || eui.variables_theme_rename_axis.is_some()
            || eui.variables_variant_rename_value.is_some()
            || eui.preset_name_input_active()
            || self.variables_search_active()
            || eui.agent_settings.focus.is_some()
            || eui.chat_model_picker.open
            || self.editor_state.chat.focused
            || git.commit_focused
            || git.remote_focused
            || git.https_focused
            || git.branch_create_focused
            || git.author_name_focused
            || git.author_email_focused
            || git
                .clone_form
                .as_ref()
                .is_some_and(|form| form.focus.is_some())
    }

    pub(in crate::widget_host) fn commit_property_family_focus_if_any(&mut self) -> bool {
        let was_focused = self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some();
        if was_focused {
            self.commit_property_focus_if_any();
        }
        was_focused
    }

    /// Commit + defocus every chrome text input. Returns `true` when
    /// any input was focused (or the chat model-picker popover was
    /// open) so blank-press callers can report a visible change.
    pub(in crate::widget_host) fn blur_text_inputs_on_blank_press(&mut self) -> bool {
        let was_focused = self.any_text_input_focused();
        self.commit_property_family_focus_if_any();
        // VariablesPanel drafts COMMIT on blur (header renames + row
        // cells), mirroring the native blur helper; the search box
        // just defocuses, keeping its typed filter.
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
        self.editor_state.editor_ui.variables_search_focus = false;
        // Settings-modal inputs (MCP port, agent / image-gen fields).
        self.commit_settings_focus();
        // Preset save-as-name input discards on blur, matching native
        // and TS outside-mousedown behavior.
        self.editor_state.editor_ui.variables_preset_name_focus = false;
        // Git-panel inputs drop focus while preserving drafts.
        let _ = self.editor_state.editor_ui.git_panel.defocus_text_inputs();
        // Chat input + its model-picker popover (same field set as
        // the picker-close branch in `apply_escape`).
        self.editor_state.editor_ui.close_chat_model_picker();
        self.editor_state.chat.blur_input(self.now_ms);
        if was_focused {
            self.mark_dirty();
        }
        was_focused
    }
}
