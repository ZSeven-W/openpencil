//! Shared focused text-input helpers for the web widget host.

use super::WidgetHost;

impl WidgetHost {
    /// True iff a text-input surface owns the keyboard. Gates the
    /// editor shortcuts so typing into a focused input never
    /// duplicates / nudges / reorders nodes.
    pub(crate) fn input_active(&self) -> bool {
        let ui = &self.editor_state.ui;
        self.editor_state.editor_ui.collab_join_input_active()
            || ui.layer_rename.is_some()
            || ui.text_editing.is_some()
            || ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
            || self.editor_state.editor_ui.variable_row_focus.is_some()
            || self
                .editor_state
                .editor_ui
                .variables_theme_rename_axis
                .is_some()
            || self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .is_some()
            // #20: the preset dropdown's save-as-name input owns the keyboard
            // (native parity — without this a bare letter would switch tools
            // instead of typing into the preset name).
            || self.editor_state.editor_ui.preset_name_input_active()
            || self.variables_search_active()
            || self.editor_state.editor_ui.font_picker.open
            || self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
            || (self.editor_state.editor_ui.agent_settings_open
                && self.editor_state.editor_ui.agent_settings.focus.is_some())
            || self.editor_state.editor_ui.icon_picker.open
            || self.editor_state.editor_ui.prompt_center.open
            // Desktop opens with search active; touch waits for an explicit
            // field tap so the hidden DOM input cannot raise the software
            // keyboard over the gallery on entry.
            || self.editor_state.editor_ui.scene_template_center.input_active()
            || self.editor_state.editor_ui.chat_model_picker.open
            || self.editor_state.editor_ui.component_browser_open
            || self.editor_state.chat.focused
            || self.git_commit_focus_active()
            || self.git_remote_focus_active()
            || self.git_https_focus_active()
            || self.git_branch_create_focus_active()
            || self.git_author_focus_active()
            || self.git_clone_input_active()
    }

    /// Visible non-chat input ownership for clipboard routing. This mirrors
    /// native: stale chat focus must not win over a modal, picker, or image
    /// popover layered above it.
    pub(crate) fn non_chat_input_owns_keyboard(&self) -> bool {
        let editor_ui = &self.editor_state.editor_ui;
        if editor_ui.collab_join_input_active()
            || editor_ui.font_picker.open
            || editor_ui.image_panel.search_open
            || editor_ui.image_panel.generate_open
            || self.variables_search_active()
            || editor_ui.preset_name_input_active()
            || editor_ui.icon_picker.open
            || editor_ui.prompt_center.open
            || editor_ui.scene_template_center.input_active()
            || editor_ui.component_browser_open
            || self.git_commit_focus_active()
            || self.git_remote_focus_active()
            || self.git_https_focus_active()
            || self.git_branch_create_focus_active()
            || self.git_author_focus_active()
            || self.git_clone_input_active()
        {
            return true;
        }
        if self.editor_state.chat.focused {
            self.editor_state
                .active_text_input()
                .is_some_and(|active| !std::ptr::eq(active, &self.editor_state.chat.input))
        } else {
            self.input_active()
        }
    }

    pub(in crate::widget_host) fn git_commit_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open
            && panel.commit_focused
            && !panel.loading
            && !panel.branch_picker_open
            && !panel.author_prompt
    }

    pub(in crate::widget_host) fn git_remote_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.remote_focused && !panel.loading && !panel.branch_picker_open
    }

    pub(in crate::widget_host) fn git_https_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.https_focused && !panel.loading && !panel.branch_picker_open
    }

    pub(in crate::widget_host) fn git_branch_create_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.branch_create_focused && !panel.loading
    }

    pub(in crate::widget_host) fn git_author_focus_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open
            && panel.author_prompt
            && (panel.author_name_focused || panel.author_email_focused)
            && !panel.loading
    }

    pub(in crate::widget_host) fn git_ready_popover_open(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open
            && panel.in_repo
            && !panel.loading
            && !panel.merging
            && panel.diff.is_none()
            && panel.merge_resolve.is_none()
            && (panel.branch_picker_open || panel.overflow_open)
    }

    pub(in crate::widget_host) fn git_clone_input_active(&self) -> bool {
        let panel = &self.editor_state.editor_ui.git_panel;
        panel.open && panel.clone_form.is_some()
    }

    /// Highlighted text of whichever non-chat input currently owns the
    /// keyboard, in the same priority order as `apply_input_select_all`.
    /// `None` when no `TextInputState`-backed input is focused or the focused
    /// input has no selection. Drives Ctrl/Cmd+C for property / settings /
    /// rename / variable inputs. Chat input copy is handled separately in
    /// `apply_copy` (its own selection model). The canvas text-node editor and
    /// the icon / component-browser search fields are not covered here — their
    /// selection is not a queryable `TextInputState` slice.
    pub(in crate::widget_host) fn focused_input_selected_text(&self) -> Option<String> {
        fn slice(state: &jian_core::text_input::TextInputState) -> Option<String> {
            let (start, end) = state.highlight_range()?;
            Some(state.text().get(start..end)?.to_string())
        }
        let ui = &self.editor_state.ui;
        let eui = &self.editor_state.editor_ui;
        // The painted image popover is topmost. Resolve it before any stale
        // focus bit left on an obscured settings/Git/property field so copy
        // and cut cannot split from typing and IME routing.
        let generate_configured = eui.agent_settings.image_generation_configured();
        if eui.image_panel.search_open || eui.image_panel.generate_open {
            return eui
                .image_panel
                .active_input(generate_configured)
                .and_then(slice);
        }
        if eui.prompt_center.open {
            let input = match eui.prompt_center.focus {
                op_editor_core::PromptCenterFocus::Search => &eui.prompt_center.search,
                op_editor_core::PromptCenterFocus::SaveTitle => &eui.prompt_center.save_title,
            };
            return slice(input);
        }
        if eui.scene_template_center.input_active() {
            // Copying from the gallery's own fields, not from whatever the
            // canvas had selected behind it.
            return op_editor_core::scene_template_keyboard::selected_text(&self.editor_state)
                .map(str::to_string);
        }
        if eui.agent_settings.focus.is_some() {
            return slice(&eui.settings_input);
        }
        if self.git_clone_input_active() {
            if let Some(form) = eui.git_panel.clone_form.as_ref() {
                return match form.focus {
                    Some(op_editor_core::CloneField::Url) => slice(&form.url_input),
                    Some(op_editor_core::CloneField::Dest) => slice(&form.dest_input),
                    None => None,
                };
            }
        }
        if self.git_commit_focus_active() {
            return slice(&eui.git_panel.commit_input);
        }
        if self.git_remote_focus_active() {
            return slice(&eui.git_panel.remote_input);
        }
        if self.git_https_focus_active() {
            return slice(&eui.git_panel.https_input);
        }
        if self.git_branch_create_focus_active() {
            return slice(&eui.git_panel.branch_create_input);
        }
        if self.git_author_focus_active() {
            return if eui.git_panel.author_email_focused {
                slice(&eui.git_panel.author_email_input)
            } else {
                slice(&eui.git_panel.author_name_input)
            };
        }
        if let Some(rename) = ui.layer_rename.as_ref() {
            return slice(&rename.input);
        }
        if ui.property_focus.is_some() || eui.effect_param_focus.is_some() {
            return slice(&ui.property_input);
        }
        if eui.variables_theme_rename_axis.is_some() || eui.variables_variant_rename_value.is_some()
        {
            return slice(&eui.variables_header_input);
        }
        if eui.variable_row_focus.is_some() {
            return slice(&eui.variable_row_input);
        }
        if eui.chat_model_picker.open {
            return slice(&eui.chat_model_picker_input);
        }
        None
    }

    // The keyboard routers publish the legacy caret mirror through
    // `op_editor_core::host_keyboard_transitions` directly, and the
    // variables-panel header renames now ride the shared press
    // transitions; this wrapper remains for the row-cell PRESS paths.

    pub(in crate::widget_host) fn sync_variable_row_input_legacy(&mut self, select_all: bool) {
        op_editor_core::host_keyboard_transitions::mirror_variable_row_input_legacy(
            &mut self.editor_state,
            select_all,
            self.now_ms,
        );
    }
}
