//! Web `WidgetHost::apply_escape` — the Escape-key overlay-dismiss
//! cascade (one layer per press). Split from `keyboard.rs` to keep each
//! file under the 800-line ceiling.
//!
//! What each rung *does* is single-sourced in
//! `op_editor_core::host_escape_transitions`; the ORDER is host-side
//! because the browser bundle has no preview mode, pen tool, or Git
//! sub-forms to step through. The two documented behavioural
//! divergences from the native ladder are commented inline below.

use super::WidgetHost;
use op_editor_core::host_escape_transitions as escape;

impl WidgetHost {
    /// Escape — handles one layer per press.
    pub fn apply_escape(&mut self) -> bool {
        // Slideshow presentation consumes Escape to exit. This must run
        // before all the property/panel/modal escapes so presentation gets
        // priority when it is active. Use the cached viewport dimensions,
        // which are updated on every press/paint.
        if self.preview_slideshow_active() {
            self.do_exit_preview(self.last_viewport_w, self.last_viewport_h);
            return true;
        }
        if self.editor_state.editor_ui.escape_scene_template_center() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_prompt_center() {
            self.mark_dirty();
            return true;
        }
        // The property-panel image popover owns Escape before any stale input
        // underneath. Keep the image selected; only dismiss the top layer.
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            self.clear_image_input_selection_drag();
            self.editor_state.editor_ui.image_panel.close_popovers();
            self.mark_dirty();
            return true;
        }
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
        if self.editor_state.editor_ui.escape_export_dialog() {
            self.mark_dirty();
            return true;
        }
        // No Cancel post-back here: the multi-page Figma picker only
        // runs in the native shell (see `keyboard.rs::apply_escape`).
        if self.editor_state.editor_ui.escape_import_modal() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_file_menu() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_export_quick_menu() {
            self.mark_dirty();
            return true;
        }
        // The slides rail's export dropdown — same rung as the TopBar
        // dropdown it mirrors, and the same rung native puts it on.
        if self.editor_state.editor_ui.slides_panel.close_export_menu() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_layer_context_menu() {
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
        if self.editor_state.editor_ui.close_corner_expand() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_effect_add_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.compositing_picker.open {
            self.close_compositing_picker();
            self.mark_dirty();
            return true;
        }
        if escape::escape_effect_param_focus(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        if escape::escape_property_focus(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        // Escape blurs the variables search box, keeping the filter.
        if self.editor_state.editor_ui.blur_variables_search() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_variables_row_menu() {
            self.mark_dirty();
            return true;
        }
        if let Some(consumed) = self.apply_git_escape() {
            return consumed;
        }
        // Divergence kept on purpose: the web ladder COMMITS the
        // variables header rename + row drafts, while the native ladder
        // DISCARDS them (`escape::escape_variable_row_focus`). Both are
        // covered by their host's own tests; the web behavior is the
        // one the browser chrome ships.
        if self.editor_state.editor_ui.variables_header_rename_active() {
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
        if self.editor_state.editor_ui.escape_agent_settings_modal() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.import_menu_open {
            self.close_import_menu();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_locale_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_shape_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_icon_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_component_browser() {
            self.mark_dirty();
            return true;
        }
        if self
            .editor_state
            .editor_ui
            .escape_instance_component_picker()
        {
            self.mark_dirty();
            return true;
        }
        // Escape closes the colour-variable popup before the fill-type
        // dropdown underneath it (one layer per press).
        if self
            .editor_state
            .editor_ui
            .property_color_variable_picker_open
            .is_some()
        {
            self.editor_state.editor_ui.close_color_variable_picker();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_fill_type_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_image_fill_popover() {
            self.mark_dirty();
            return true;
        }
        if self.exit_image_crop_edit() {
            return true;
        }
        if self.editor_state.editor_ui.escape_chat_model_picker() {
            self.mark_dirty();
            return true;
        }
        if escape::escape_chat_focus(&mut self.editor_state, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if escape::escape_selection(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        false
    }
}
