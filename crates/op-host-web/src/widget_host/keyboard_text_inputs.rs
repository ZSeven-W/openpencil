//! Shared focused text-input helpers for the web widget host.

use super::WidgetHost;

impl WidgetHost {
    /// True iff a text-input surface owns the keyboard. Gates the
    /// editor shortcuts so typing into a focused input never
    /// duplicates / nudges / reorders nodes.
    pub(in crate::widget_host) fn input_active(&self) -> bool {
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
            || self.variables_search_active()
            || self.editor_state.editor_ui.agent_settings.focus.is_some()
            || self.editor_state.editor_ui.icon_picker.open
            || self.editor_state.editor_ui.chat_model_picker.open
            || self.editor_state.editor_ui.component_browser_open
            || self.editor_state.chat.focused
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
