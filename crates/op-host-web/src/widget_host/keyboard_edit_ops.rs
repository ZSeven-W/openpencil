//! Editor keyboard shortcuts (duplicate / nudge / clipboard / undo /
//! reorder) on the web host — split from `keyboard.rs` to honor the
//! 800-line cap. Mirrors the native `widget_host/keyboard.rs` ops.

use super::WidgetHost;

impl WidgetHost {
    /// Cmd/Ctrl+D — duplicate the selected node as a sibling
    /// offset by ~10 doc px. Selection follows the clone.
    pub fn apply_duplicate(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        let dup = self
            .editor_state
            .duplicate_selected(&mut self.next_node_id, 10.0)
            .is_some();
        if dup {
            self.mark_dirty();
        }
        dup
    }

    /// Left / Right arrow during an inline rename — moves the rename
    /// caret one character. Returns whether a rename is active, so the
    /// caller falls back to node-nudge when it isn't.
    pub fn apply_rename_caret(&mut self, forward: bool) -> bool {
        let moved = if forward {
            self.editor_state.rename_caret_right()
        } else {
            self.editor_state.rename_caret_left()
        };
        if moved {
            if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
                rename.input.touch(self.now_ms);
            }
            self.mark_dirty();
        }
        moved
    }

    /// Arrow-key nudge — translate the selected node by
    /// `(dx, dy)` document px. Shift-arrow callers pass 10 px;
    /// plain arrows pass 1 px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.translate_selected(dx as f64, dy as f64) {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd/Ctrl+A — replace selection with every top-level node
    /// on the active page (TS `setSelection(topLevelIds, …)`).
    pub fn apply_select_all(&mut self) -> bool {
        if self.apply_input_select_all() {
            return true;
        }
        if self.editor_state.select_all_top_level() {
            self.mark_dirty();
            return true;
        }
        false
    }

    fn apply_input_select_all(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            let ui = &mut self.editor_state.editor_ui;
            ui.settings_input.select_all();
            ui.settings_input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
            rename.input.select_all();
            rename.input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.text_editing.is_some() {
            let _ = self.editor_state.text_edit_select_all_now(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            self.editor_state.ui.property_input.select_all();
            self.editor_state.ui.property_input.touch(self.now_ms);
            self.sync_property_input_legacy(true);
            self.mark_dirty();
            return true;
        }
        let variable_header_focus = self
            .editor_state
            .editor_ui
            .variables_theme_rename_axis
            .is_some()
            || self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .is_some();
        if variable_header_focus || self.editor_state.editor_ui.variable_row_focus.is_some() {
            if variable_header_focus {
                self.editor_state
                    .editor_ui
                    .variables_header_input
                    .select_all();
                self.editor_state
                    .editor_ui
                    .variables_header_input
                    .touch(self.now_ms);
                self.sync_variables_header_input_legacy(true);
            } else {
                self.editor_state.editor_ui.variable_row_input.select_all();
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .touch(self.now_ms);
                self.sync_variable_row_input_legacy(true);
            }
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.icon_picker_open {
            self.editor_state.editor_ui.icon_picker_select_all = true;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.component_browser_open {
            self.editor_state.editor_ui.component_browser_select_all = true;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            let ui = &mut self.editor_state.editor_ui;
            ui.chat_model_picker_input.select_all();
            ui.chat_model_picker_input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.chat.focused {
            self.editor_state.chat.select_all_input(self.now_ms);
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_property_caret(&mut self, forward: bool) -> bool {
        if self.editor_state.ui.property_focus.is_none()
            && self.editor_state.editor_ui.effect_param_focus.is_none()
            && self
                .editor_state
                .editor_ui
                .variables_theme_rename_axis
                .is_none()
            && self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .is_none()
            && self.editor_state.editor_ui.variable_row_focus.is_none()
        {
            return false;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            if forward {
                self.editor_state
                    .ui
                    .property_input
                    .move_right(false, self.now_ms);
            } else {
                self.editor_state
                    .ui
                    .property_input
                    .move_left(false, self.now_ms);
            }
            self.sync_property_input_legacy(false);
            self.mark_dirty();
            return true;
        }
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
            if forward {
                self.editor_state
                    .editor_ui
                    .variables_header_input
                    .move_right(false, self.now_ms);
            } else {
                self.editor_state
                    .editor_ui
                    .variables_header_input
                    .move_left(false, self.now_ms);
            }
            self.sync_variables_header_input_legacy(false);
            self.mark_dirty();
            return true;
        }
        if forward {
            self.editor_state
                .editor_ui
                .variable_row_input
                .move_right(false, self.now_ms);
        } else {
            self.editor_state
                .editor_ui
                .variable_row_input
                .move_left(false, self.now_ms);
        }
        self.sync_variable_row_input_legacy(false);
        self.mark_dirty();
        true
    }

    /// Cmd/Ctrl+C — copy the selection into the clipboard.
    pub fn apply_copy(&mut self) -> bool {
        if self.editor_state.chat.focused {
            if let Some(text) = self
                .editor_state
                .chat
                .selected_input_text()
                .map(str::to_string)
            {
                #[cfg(feature = "codegen")]
                crate::web_clipboard::copy_text(&text);
                #[cfg(not(feature = "codegen"))]
                self.editor_state.chat.queue_copy_text(text);
                return true;
            }
            return false;
        }
        if self.input_active() {
            return false;
        }
        if let Some(text) = self.editor_state.codegen.selected_code_text() {
            #[cfg(feature = "codegen")]
            crate::web_clipboard::copy_text(text);
            #[cfg(not(feature = "codegen"))]
            let _ = text;
            return true;
        }
        if let Some(text) = self
            .editor_state
            .chat
            .selected_transcript_text()
            .map(str::to_string)
        {
            #[cfg(feature = "codegen")]
            crate::web_clipboard::copy_text(&text);
            #[cfg(not(feature = "codegen"))]
            self.editor_state.chat.queue_copy_text(text);
            return true;
        }
        if self.editor_state.copy_selected() {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd/Ctrl+X — copy then delete the selection.
    pub fn apply_cut(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        if self.editor_state.cut_selected() {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd/Ctrl+V — paste the clipboard at the active page,
    /// offset by 10 doc px from the originals. Selection follows
    /// the new clones.
    pub fn apply_paste(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.clipboard.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        let pasted = !self
            .editor_state
            .paste_clipboard(&mut self.next_node_id, 10.0)
            .is_empty();
        if pasted {
            self.mark_dirty();
        }
        pasted
    }

    pub fn apply_undo(&mut self) -> bool {
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.undo() {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_redo(&mut self) -> bool {
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.redo() {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd+Shift+K — toggle the component (UIKit) browser panel.
    /// Mirrors the native host's `apply_toggle_component_browser`
    /// (TS `editor-layout.tsx` Cmd+Shift+K → `toggleBrowser`); the
    /// open-position default is the viewport centre via
    /// `component_browser_panel_rect`'s `None`-pos fallback.
    pub fn apply_toggle_component_browser(&mut self) -> bool {
        let ui = &mut self.editor_state.editor_ui;
        ui.component_browser_open = !ui.component_browser_open;
        if !ui.component_browser_open {
            ui.component_browser_kit_picker_open = false;
            ui.component_browser_confirm_delete_kit = None;
            ui.component_browser_hover = None;
        }
        self.mark_dirty();
        true
    }

    /// `[` / `]` — bump the selected node down / up by one
    /// position in its parent's children vec (changing paint
    /// order).
    pub fn apply_reorder(&mut self, direction: op_editor_core::ReorderDirection) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        if self.editor_state.reorder_selected(direction) {
            self.mark_dirty();
            return true;
        }
        false
    }
}
