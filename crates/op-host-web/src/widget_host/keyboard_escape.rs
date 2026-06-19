//! Web `WidgetHost::apply_escape` — the Escape-key overlay-dismiss
//! cascade (one layer per press, native priority order). Split from
//! `keyboard.rs` to keep each file under the 800-line ceiling.

use super::WidgetHost;

impl WidgetHost {
    /// Escape — handles one layer per press, matching the native
    /// host's priority order.
    pub fn apply_escape(&mut self) -> bool {
        if self
            .editor_state
            .editor_ui
            .agent_settings
            .focus
            .take()
            .is_some()
        {
            self.clear_settings_caret();
            self.mark_dirty();
            return true;
        }
        // #20: Escape closes just the preset save-as-name input; the preset
        // dropdown stays open (native parity, `variable-theme-manager.tsx:299`).
        if self.escape_variables_preset_name() {
            return true;
        }
        // Modal overlays close one per press (mirrors native order:
        // export dialog → figma import → file menu).
        if self.editor_state.editor_ui.export_dialog_open {
            self.editor_state.editor_ui.export_dialog_open = false;
            self.editor_state.editor_ui.export_dialog_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.figma_import_open {
            self.editor_state.editor_ui.figma_import_open = false;
            self.editor_state.editor_ui.figma_import_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.file_menu_open {
            self.editor_state.editor_ui.file_menu_open = false;
            self.editor_state.editor_ui.file_menu.hover = None;
            self.mark_dirty();
            return true;
        }
        // Escape closes an open layer/page right-click context menu
        // (layer-context-menu.tsx:101 — keydown Escape → onClose).
        if self
            .editor_state
            .editor_ui
            .layer_context_menu
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.rename_cancel() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.text_edit_commit() {
            self.mark_dirty();
            return true;
        }
        if self
            .editor_state
            .editor_ui
            .effect_param_focus
            .take()
            .is_some()
        {
            self.editor_state.ui.property_input.set_text("");
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.property_focus.take().is_some() {
            self.editor_state.ui.property_input.set_text("");
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.mark_dirty();
            return true;
        }
        // Escape blurs the variables search box (the typed filter is
        // kept — clearing it would surprise mid-search).
        if self.variables_search_active() {
            self.editor_state.editor_ui.variables_search_focus = false;
            self.mark_dirty();
            return true;
        }
        // Escape closes an open variable-row `⋯` menu.
        if self
            .editor_state
            .editor_ui
            .variables_row_menu
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        if let Some(consumed) = self.apply_git_escape() {
            return consumed;
        }
        // Escape COMMITS variables header renames + row drafts
        // (mirrors the native host's escape behavior).
        if self
            .editor_state
            .editor_ui
            .variables_theme_rename_axis
            .is_some()
            || self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .is_some()
        {
            self.commit_variables_panel_header_focus_if_any();
            return true;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            self.commit_variable_row_focus_if_any();
            return true;
        }
        if self.editor_state.editor_ui.font_picker.open {
            let ui = &mut self.editor_state.editor_ui;
            ui.close_font_picker();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.locale_picker.open {
            self.editor_state.editor_ui.locale_picker.open = false;
            self.editor_state.editor_ui.locale_picker.hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.shape_picker.open {
            self.editor_state.editor_ui.shape_picker.open = false;
            self.editor_state.editor_ui.shape_picker.hover = None;
            self.editor_state.editor_ui.shape_picker.pressed = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.icon_picker.open {
            self.editor_state.editor_ui.close_icon_picker();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.component_browser_open {
            // One layer per press: an open kit-filter popover closes
            // before the panel itself does (mirrors the native host).
            if self
                .editor_state
                .editor_ui
                .component_browser_kit_picker_open
            {
                self.editor_state
                    .editor_ui
                    .component_browser_kit_picker_open = false;
                self.mark_dirty();
                return true;
            }
            self.editor_state.editor_ui.component_browser_open = false;
            self.editor_state.editor_ui.component_browser_select_all = false;
            self.editor_state.editor_ui.component_browser_hover = None;
            self.editor_state
                .editor_ui
                .component_browser_confirm_delete_kit = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.fill_type_picker.open {
            self.editor_state.editor_ui.close_fill_type_picker();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.image_fill_popover_open {
            self.editor_state.editor_ui.image_fill_popover_open = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            self.editor_state.editor_ui.close_chat_model_picker();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.chat.focused {
            self.editor_state.chat.blur_input(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if !self.editor_state.selection.is_empty() {
            self.editor_state.deselect_all();
            self.mark_dirty();
            return true;
        }
        false
    }
}
