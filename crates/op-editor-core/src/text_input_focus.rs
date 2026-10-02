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
        if self.editor_ui.home.visible && self.editor_ui.agent_settings_open {
            return self
                .editor_ui
                .agent_settings
                .focus
                .as_ref()
                .map(|_| &self.editor_ui.settings_input);
        }
        if self.editor_ui.home.visible {
            if self.editor_ui.home_composer_owns_keyboard() {
                return Some(&self.editor_ui.home.input);
            }
            return (!self.editor_ui.login_modal_open
                && !self.editor_ui.account_menu_open
                && !self.editor_ui.save_name_dialog.open
                && self.editor_ui.chat_model_picker.open)
                .then_some(&self.editor_ui.chat_model_picker_input);
        }
        // Image popovers paint above every editor surface. Resolve their
        // visible field first so stale focus underneath cannot split keyboard,
        // clipboard, and IME ownership across different inputs.
        let generate_configured = self.editor_ui.agent_settings.image_generation_configured();
        if self.editor_ui.image_panel.search_open || self.editor_ui.image_panel.generate_open {
            return self.editor_ui.image_panel.active_input(generate_configured);
        }
        if self.editor_ui.prompt_center.open {
            return Some(match self.editor_ui.prompt_center.focus {
                crate::PromptCenterFocus::Search => &self.editor_ui.prompt_center.search,
                crate::PromptCenterFocus::SaveTitle => &self.editor_ui.prompt_center.save_title,
            });
        }
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
        // The collaboration Join field rides the visibility-aware focus
        // predicate so a stale bit never claims blink/clipboard ownership.
        if self.editor_ui.collab_join_input_active() {
            return Some(&self.editor_ui.collab.panel.join_input);
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
        if self.editor_ui.home.visible && self.editor_ui.agent_settings_open {
            return self
                .editor_ui
                .agent_settings
                .focus
                .is_some()
                .then_some(&mut self.editor_ui.settings_input);
        }
        if self.editor_ui.home.visible {
            if self.editor_ui.home_composer_owns_keyboard() {
                return Some(&mut self.editor_ui.home.input);
            }
            return (!self.editor_ui.login_modal_open
                && !self.editor_ui.account_menu_open
                && !self.editor_ui.save_name_dialog.open
                && self.editor_ui.chat_model_picker.open)
                .then_some(&mut self.editor_ui.chat_model_picker_input);
        }
        let generate_configured = self.editor_ui.agent_settings.image_generation_configured();
        if self.editor_ui.image_panel.search_open || self.editor_ui.image_panel.generate_open {
            return self
                .editor_ui
                .image_panel
                .active_input_mut(generate_configured);
        }
        if self.editor_ui.prompt_center.open {
            return Some(self.editor_ui.prompt_center.focused_input_mut());
        }
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
        if self.editor_ui.collab_join_input_active() {
            return Some(&mut self.editor_ui.collab.panel.join_input);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_settings::{BuiltinAgentField, SettingsFocus};

    #[test]
    fn home_modal_keyboard_resolves_both_mutable_and_shared_input_to_settings() {
        let mut state = EditorState::new();
        let ui = &mut state.editor_ui;
        ui.home.visible = true;
        ui.home.composer_focused = true;
        ui.home.set_draft("untouched brief");
        ui.agent_settings_open = true;
        ui.agent_settings.focus = Some(SettingsFocus::BuiltinAgentDraft(BuiltinAgentField::Model));
        ui.settings_input.set_text("glm-5.2");
        ui.chat_model_picker.open = true;

        assert!(!state.editor_ui.home_composer_owns_keyboard());
        assert_eq!(state.active_text_input().unwrap().text(), "glm-5.2");
        state
            .active_text_input_mut()
            .unwrap()
            .set_text("glm-5.3-flash");
        assert_eq!(state.editor_ui.settings_input.text(), "glm-5.3-flash");
        assert_eq!(state.editor_ui.home.draft, "untouched brief");

        state.editor_ui.agent_settings.focus = None;
        assert!(
            state.active_text_input().is_none(),
            "a modal with no focused field blocks Home"
        );
        assert!(state.active_text_input_mut().is_none());
        state.editor_ui.agent_settings_open = false;
        state.editor_ui.chat_model_picker.open = false;
        assert!(state.editor_ui.home_composer_owns_keyboard());
        assert_eq!(state.active_text_input().unwrap().text(), "untouched brief");
    }
}
