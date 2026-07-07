//! Shared focused text-input helpers for the web widget host.

use super::WidgetHost;

impl WidgetHost {
    /// True iff a text-input surface owns the keyboard. Gates the
    /// editor shortcuts so typing into a focused input never
    /// duplicates / nudges / reorders nodes.
    pub(crate) fn input_active(&self) -> bool {
        let ui = &self.editor_state.ui;
        ui.layer_rename.is_some()
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
            || (self.editor_state.editor_ui.agent_settings_open
                && self.editor_state.editor_ui.agent_settings.focus.is_some())
            || self.editor_state.editor_ui.icon_picker.open
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

    pub(in crate::widget_host) fn sync_property_input_legacy(&mut self, select_all: bool) {
        let ui = &mut self.editor_state.ui;
        ui.property_input_draft = ui.property_input.text().to_owned();
        ui.property_caret_pos = ui.property_input.caret();
        ui.property_draft_select_all = select_all;
        ui.property_caret_anchor_ms = self.now_ms;
    }

    pub(in crate::widget_host) fn sync_variables_header_input_legacy(&mut self, select_all: bool) {
        self.editor_state.ui.property_input_draft = self
            .editor_state
            .editor_ui
            .variables_header_input
            .text()
            .to_owned();
        self.editor_state.ui.property_caret_pos =
            self.editor_state.editor_ui.variables_header_input.caret();
        self.editor_state.ui.property_draft_select_all = select_all;
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
    }

    pub(in crate::widget_host) fn sync_variable_row_input_legacy(&mut self, select_all: bool) {
        self.editor_state.ui.property_input_draft = self
            .editor_state
            .editor_ui
            .variable_row_input
            .text()
            .to_owned();
        self.editor_state.ui.property_caret_pos =
            self.editor_state.editor_ui.variable_row_input.caret();
        self.editor_state.ui.property_draft_select_all = select_all;
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
    }
}
