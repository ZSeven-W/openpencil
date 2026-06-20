//! Focused text-input resolver shared by hosts.
//!
//! The priority order mirrors the native host's historical caret
//! wake-up branches. It is intentionally explicit: when multiple
//! stale focus flags coexist, the first active site wins.

use crate::editor_ui_state::CloneField;
use crate::state::EditorState;
use jian_core::text_input::TextInputState;

impl EditorState {
    pub fn active_text_input(&self) -> Option<&TextInputState> {
        if self.ui.text_editing.is_some() {
            return Some(&self.ui.text_edit_input);
        }
        // Colour-picker hex bar — renders through the unified `TextInputView`,
        // so its caret blink must drive the same redraw wake-up as every other
        // focused field.
        if let Some(picker) = &self.ui.color_picker {
            if picker.hex_focused {
                return Some(&picker.hex_input);
            }
            if picker.rgb_focus.is_some() {
                return Some(&picker.rgb_input);
            }
        }
        if let Some(rename) = &self.ui.layer_rename {
            return Some(&rename.input);
        }
        // Property-panel fields and effect-parameter fields share
        // `property_input`, so both focuses resolve to it (mirrors
        // `focused_input_selected_text` on the web host).
        if self.ui.property_focus.is_some() || self.editor_ui.effect_param_focus.is_some() {
            return Some(&self.ui.property_input);
        }
        if self.editor_ui.variables_theme_rename_axis.is_some()
            || self.editor_ui.variables_variant_rename_value.is_some()
        {
            return Some(&self.editor_ui.variables_header_input);
        }
        if self.editor_ui.variable_row_focus.is_some() {
            return Some(&self.editor_ui.variable_row_input);
        }
        if self.editor_ui.agent_settings_open && self.editor_ui.agent_settings.focus.is_some() {
            return Some(&self.editor_ui.settings_input);
        }
        if self.editor_ui.chat_model_picker.open {
            return Some(&self.editor_ui.chat_model_picker_input);
        }
        if self.chat.focused {
            return Some(&self.chat.input);
        }

        let git = &self.editor_ui.git_panel;
        if git.commit_focused {
            return Some(&git.commit_input);
        }
        if git.remote_focused {
            return Some(&git.remote_input);
        }
        if git.https_focused {
            return Some(&git.https_input);
        }
        if git.branch_create_focused {
            return Some(&git.branch_create_input);
        }
        if git.author_name_focused {
            return Some(&git.author_name_input);
        }
        if git.author_email_focused {
            return Some(&git.author_email_input);
        }
        if let Some(form) = &git.clone_form {
            return match form.focus {
                Some(CloneField::Url) => Some(&form.url_input),
                Some(CloneField::Dest) => Some(&form.dest_input),
                None => None,
            };
        }
        None
    }

    pub fn active_text_input_mut(&mut self) -> Option<&mut TextInputState> {
        if self.ui.text_editing.is_some() {
            return Some(&mut self.ui.text_edit_input);
        }
        if let Some(rename) = &mut self.ui.layer_rename {
            return Some(&mut rename.input);
        }
        if self.ui.property_focus.is_some() || self.editor_ui.effect_param_focus.is_some() {
            return Some(&mut self.ui.property_input);
        }

        let variables_header_active = self.editor_ui.variables_theme_rename_axis.is_some()
            || self.editor_ui.variables_variant_rename_value.is_some();
        if variables_header_active {
            return Some(&mut self.editor_ui.variables_header_input);
        }
        if self.editor_ui.variable_row_focus.is_some() {
            return Some(&mut self.editor_ui.variable_row_input);
        }
        if self.editor_ui.agent_settings_open && self.editor_ui.agent_settings.focus.is_some() {
            return Some(&mut self.editor_ui.settings_input);
        }
        if self.editor_ui.chat_model_picker.open {
            return Some(&mut self.editor_ui.chat_model_picker_input);
        }
        if self.chat.focused {
            return Some(&mut self.chat.input);
        }

        let git = &mut self.editor_ui.git_panel;
        if git.commit_focused {
            return Some(&mut git.commit_input);
        }
        if git.remote_focused {
            return Some(&mut git.remote_input);
        }
        if git.https_focused {
            return Some(&mut git.https_input);
        }
        if git.branch_create_focused {
            return Some(&mut git.branch_create_input);
        }
        if git.author_name_focused {
            return Some(&mut git.author_name_input);
        }
        if git.author_email_focused {
            return Some(&mut git.author_email_input);
        }
        if let Some(form) = &mut git.clone_form {
            return match form.focus {
                Some(CloneField::Url) => Some(&mut form.url_input),
                Some(CloneField::Dest) => Some(&mut form.dest_input),
                None => None,
            };
        }
        None
    }
}
