//! `apply_escape` — the one-layer-per-press Escape ladder on
//! `WidgetHostNative`.
//!
//! Split out of `keyboard.rs` to keep every file under the repo's
//! 800-line cap.

use super::WidgetHostNative;
use op_editor_core::host_escape_transitions as escape;

impl WidgetHostNative {
    /// Escape — priority cascade: rename → property → pickers →
    /// chat → selection. One layer per press.
    pub fn apply_escape(&mut self) -> bool {
        // Transient pointer capture is the topmost interaction layer. Escape
        // cancels it without replaying a delayed tap or committing a reorder.
        if self.editor_state.editor_ui.touch_chrome() && self.cancel_native_touch_gestures() {
            self.mark_dirty();
            return true;
        }
        // Escape EXITS preview mode (top priority) — drops the runtime
        // and returns to the design surface.
        if self.preview.is_some() {
            self.exit_preview();
            return true;
        }
        // The save-name dialog is modal: Escape cancels it and nothing else.
        if self.editor_state.editor_ui.save_name_dialog.open {
            self.editor_state.editor_ui.save_name_dialog.close();
            self.mark_dirty();
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
        if self.editor_state.color_picker_hex_focused() {
            self.collab_blur_color_picker_inputs();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.color_picker_rgb_focused() {
            self.collab_blur_color_picker_inputs();
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
        // #20: Escape closes the preset-name input only — the
        // preset dropdown stays open (variable-theme-manager.tsx:299).
        if self.escape_variables_preset_name() {
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
        // Escape steps out of the clone wizard: first defocus the active
        // field, then (on a second press) close the wizard back to the
        // empty state.
        if self.git_clone_input_active() {
            let defocused = {
                let form = self
                    .editor_state
                    .editor_ui
                    .git_panel
                    .clone_form
                    .as_mut()
                    .unwrap();
                let url_caret = form.url_input.caret();
                form.url_input.set_caret(url_caret, self.now_ms);
                let dest_caret = form.dest_input.caret();
                form.dest_input.set_caret(dest_caret, self.now_ms);
                form.focus.take().is_some()
            };
            if !defocused {
                self.editor_state.editor_ui.git_panel.clone_form = None;
            }
            self.mark_dirty();
            return true;
        }
        // A branch-picker sub-mode (create / merge) takes Escape priority
        // OVER the Git input fields: step it back to the branch list (the
        // dropdown stays open). Driven off the mode, not input focus, so a
        // stale commit / remote / https focus can't intercept it, and merge
        // mode (which has no focused input) exits too.
        if self.editor_state.editor_ui.git_panel.branch_picker_open
            && self.editor_state.editor_ui.git_panel.branch_picker_mode
                != op_editor_core::GitBranchPickerMode::List
        {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.branch_picker_mode = op_editor_core::GitBranchPickerMode::List;
            panel.branch_picker_menu.hover = None;
            panel.branch_create_input.set_text("");
            panel.branch_create_focused = false;
            self.mark_dirty();
            return true;
        }
        // Escape dismisses the commit-signature form (TS form cancel) without
        // committing — checked before the input-focus handlers so a focused
        // name/email field doesn't swallow it.
        if self.editor_state.editor_ui.git_panel.author_prompt {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.author_prompt = false;
            panel.author_name_focused = false;
            panel.author_email_focused = false;
            let caret = panel.author_name_input.caret();
            panel.author_name_input.set_caret(caret, self.now_ms);
            let caret = panel.author_email_input.caret();
            panel.author_email_input.set_caret(caret, self.now_ms);
            self.mark_dirty();
            return true;
        }
        // Escape defocuses the Git commit input (the panel stays open).
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.defocus_commit_input(self.now_ms);
            self.mark_dirty();
            return true;
        }
        // …and the Git remote-URL input.
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.remote_focused = false;
            let caret = panel.remote_input.caret();
            panel.remote_input.set_caret(caret, self.now_ms);
            self.mark_dirty();
            return true;
        }
        // …and the Git HTTPS-credential input.
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.https_focused = false;
            let caret = panel.https_input.caret();
            panel.https_input.set_caret(caret, self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.font_picker.open
            && matches!(
                self.editor_state.editor_ui.font_picker_purpose,
                Some(op_editor_core::FontPickerPurpose::MissingFont { .. })
            )
        {
            self.close_font_picker();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_agent_settings_modal() {
            self.cancel_agent_settings_touch_gesture();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_export_dialog() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.figma_import_open {
            // Divergence kept on purpose: only the native host runs the
            // multi-page Figma picker, so only it has a Cancel
            // selection to post back before the shared close.
            if self.editor_state.editor_ui.figma_import_pages.len() > 1 {
                self.editor_state.editor_ui.pending_file_action = Some(
                    op_editor_core::editor_ui_state::FileAction::FinishFigmaImport(
                        op_editor_core::FigmaImportSelection::Cancel,
                    ),
                );
            }
            self.editor_state.editor_ui.escape_import_modal();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.import_menu_open {
            self.close_import_menu();
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
        // dropdown it mirrors, so Escape dismisses whichever one is open
        // before it reaches the selection below.
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
        // Anchor-menu close, then pen CANCEL (TS Escape discards).
        if self.apply_pen_escape() {
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
        if escape::escape_variable_row_focus(&mut self.editor_state) {
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
        if self.editor_state.editor_ui.escape_chat_model_picker() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.escape_component_browser() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            self.clear_image_input_selection_drag();
            self.editor_state.editor_ui.image_panel.close_popovers();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.font_picker.open {
            self.close_font_picker();
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
        if self.editor_state.editor_ui.interaction_menu_open {
            self.editor_state.editor_ui.close_interaction_menu();
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
        if escape::escape_chat_focus(&mut self.editor_state, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if escape::escape_selection(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        // TS Escape order (use-tool-shortcuts.ts:38-49): clearing the
        // selection comes first; the NEXT Escape steps out of the
        // entered frame/group.
        if self
            .editor_state
            .editor_ui
            .entered_container
            .take()
            .is_some()
        {
            self.mark_dirty();
            return true;
        }
        false
    }
}
