//! Blur-on-blank-press — DOM-parity input defocus.
//!
//! In the TS editor every chrome input is a DOM `<input>`, so a
//! mousedown anywhere outside it blurs (and commits) it for free. The
//! Rust shell tracks each input's focus by hand, so every press path
//! that lands on blank chrome — a panel gap, a popover dismiss, the
//! bare canvas — routes through `blur_text_inputs_on_blank_press` to
//! commit + defocus them all at once instead of leaving a stale caret
//! eating keystrokes.

use super::WidgetHostNative;

impl WidgetHostNative {
    /// True when any chrome text input holds keyboard focus.
    fn any_text_input_focused(&self) -> bool {
        let ui = &self.editor_state.ui;
        let eui = &self.editor_state.editor_ui;
        let git = &eui.git_panel;
        ui.property_focus.is_some()
            || eui.variable_row_focus.is_some()
            || eui.effect_param_focus.is_some()
            || eui.variables_theme_rename_axis.is_some()
            || eui.variables_variant_rename_value.is_some()
            // #20: preset dropdown's save-as-name input.
            || eui.preset_name_input_active()
            || self.variables_search_active()
            || eui.agent_settings.focus.is_some()
            || eui.chat_model_picker_open
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

    /// Commit + defocus every chrome text input. Returns `true` when
    /// any input was focused (or the chat model-picker popover was
    /// open) so blank-press callers can report a visible change.
    pub(in crate::widget_host) fn blur_text_inputs_on_blank_press(&mut self) -> bool {
        let was_focused = self.any_text_input_focused();
        // Property-panel family — chains the variables-header,
        // variable-row, and effect-param commits ahead of the
        // property-focus commit itself.
        self.commit_property_focus_if_any();
        // Settings-modal inputs (MCP port, agent / image-gen fields).
        self.commit_settings_focus_if_any();
        // Git-panel inputs — focus flags drop, drafts persist.
        let _ = self.editor_state.editor_ui.git_panel.defocus_text_inputs();
        // Chat input + its model-picker popover (mirrors the
        // outside-click block in `click.rs`).
        let eui = &mut self.editor_state.editor_ui;
        // #20: the preset-name input discards on blur (TS closes the
        // popover's input on outside mousedown without saving).
        eui.variables_preset_name_focus = false;
        // Variables-panel search box defocuses; its typed filter
        // persists (TS keeps the input value on blur).
        eui.variables_search_focus = false;
        eui.chat_model_picker_open = false;
        eui.chat_model_picker_scroll = 0.0;
        eui.chat_model_picker_input.set_text("");
        eui.chat_model_picker_hover = None;
        self.editor_state.chat.blur_input(self.now_ms);
        if was_focused {
            self.mark_dirty();
        }
        was_focused
    }
}
