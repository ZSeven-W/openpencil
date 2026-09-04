//! Keyboard / clipboard handlers on the web `WidgetHost`.
//! Pulled out of `widget_host.rs` so the spine file stays
//! under the 800-line ceiling. Mirrors the native shell's
//! `widget_host/input.rs` + `keyboard.rs` shape.
//!
//! `EditorState` is the host's source of truth: every focus / draft
//! / chat field is read + written on `editor_state`; mutations flag
//! the paint snapshot dirty.

use super::WidgetHost;
use op_editor_core::host_keyboard_transitions as shared;
use op_editor_core::host_preset_name_draft as preset_name;

impl WidgetHost {
    /// Push a typed character into the focused chat / settings input.
    /// Returns true if anything changed.
    pub fn apply_text(&mut self, c: char) -> bool {
        if let Some(changed) = shared::prompt_center_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        // Native parity (`op-host-native/widget_host/keyboard.rs`). Without
        // this the web Asset Center could not be typed into at all — a
        // character fell through to the canvas shortcuts, where a bare letter
        // switches tools behind the open gallery.
        if let Some(changed) = shared::scene_template_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if self.editor_state.editor_ui.collab_join_input_active() {
            let changed = op_editor_ui::widgets::collab_ui::join_address_text(
                &mut self.editor_state.editor_ui,
                c,
                self.now_ms,
            )
            .unwrap_or(false);
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if self.apply_image_panel_text(c) {
            return true;
        }
        // The font picker is the next-highest text input. Route to it before
        // any stale settings, property, or canvas-text focus can consume the
        // character behind the overlay.
        if self.editor_state.editor_ui.font_picker.open {
            if c.is_control() {
                return false;
            }
            let ui = &mut self.editor_state.editor_ui;
            ui.font_picker_search.push(c);
            ui.font_picker.scroll.offset = 0.0;
            ui.font_picker.hover = None;
            ui.font_picker_import_hover = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_text(c);
        }
        if let Some(consumed) = self.apply_git_text(c) {
            return consumed;
        }
        if let Some(changed) = shared::rename_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) = shared::text_edit_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Variables-panel search filter — live append.
        if shared::variables_search_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        // Variables-panel theme/variant header rename drafts.
        if shared::variables_header_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        // #20: the variables preset dropdown's save-as-name input types into
        // the flat `property_input_draft` the ThemePresetMenu widget paints.
        if preset_name::preset_name_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        // Variables-panel row/cell drafts — per-kind char gates
        // (numeric / free text / hex) live in the shared router.
        if let Some(changed) = shared::variable_row_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Property-panel + effect-param inputs share `ui.property_input`.
        if let Some(changed) = shared::property_input_text(&mut self.editor_state, c, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Icon-picker / component-browser search boxes own typing
        // while their panels are open (mirrors native routing order:
        // icon picker → chat model picker → component browser; see
        // `overlay_keys.rs`).
        if let Some(changed) = self.icon_picker_text(c) {
            return changed;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return self.apply_chat_model_picker_text(c);
        }
        if let Some(changed) = self.component_browser_text(c) {
            return changed;
        }
        if shared::chat_input_text(&mut self.editor_state, c, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_backspace(&mut self) -> bool {
        if let Some(changed) = shared::prompt_center_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if self.editor_state.editor_ui.collab_join_input_active() {
            let changed = op_editor_ui::widgets::collab_ui::join_address_backspace(
                &mut self.editor_state.editor_ui,
                self.now_ms,
            )
            .unwrap_or(false);
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if self.apply_image_panel_backspace() {
            return true;
        }
        // Swallow Backspace in the font picker even when the query is empty,
        // so it cannot mutate a background property or canvas selection.
        if self.editor_state.editor_ui.font_picker.open {
            let ui = &mut self.editor_state.editor_ui;
            let query_changed = ui.font_picker_search.pop().is_some();
            let hover_changed = ui.font_picker_import_hover;
            if query_changed || hover_changed {
                ui.font_picker.scroll.offset = 0.0;
                ui.font_picker.hover = None;
                ui.font_picker_import_hover = false;
                self.mark_dirty();
            }
            return true;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_backspace();
        }
        if let Some(consumed) = self.apply_git_backspace() {
            return consumed;
        }
        if let Some(changed) = shared::rename_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) = shared::text_edit_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Variables-panel search filter — pop one char.
        if let Some(changed) =
            shared::variables_search_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // #20: preset save-as-name input — delete before the caret (or
        // clear a select-all draft).
        if let Some(changed) =
            preset_name::preset_name_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.variables_header_rename_active() {
            let changed = shared::variables_header_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            let changed = shared::variable_row_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            let changed = shared::property_input_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) = self.icon_picker_backspace() {
            return changed;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return self.apply_chat_model_picker_backspace();
        }
        if let Some(changed) = self.component_browser_backspace() {
            return changed;
        }
        if let Some(changed) = shared::chat_input_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if shared::delete_selection_with_history(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_send(&mut self) -> bool {
        if self.editor_state.editor_ui.prompt_center.open {
            return true;
        }
        if self.editor_state.editor_ui.collab_join_input_active() {
            let queued = op_editor_ui::widgets::collab_ui::join_address_submit(
                &mut self.editor_state.editor_ui,
            )
            .unwrap_or(false);
            if queued {
                self.mark_dirty();
            }
            return true;
        }
        // The image popover is painted above every editor input. Submit or
        // swallow Enter before consulting any independently stale focus below.
        if self.apply_image_panel_send() {
            return true;
        }
        if self.exit_image_crop_edit() {
            return true;
        }
        // Enter belongs to the open font-search overlay. Swallow it before
        // any stale background focus or the chat composer can act on it.
        if self.editor_state.editor_ui.font_picker.open {
            return true;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            if op_editor_core::host_ui_transitions::settings_model_newline(
                &mut self.editor_state.editor_ui,
                self.now_ms,
            ) {
                self.mark_dirty();
                return true;
            }
            self.commit_settings_focus();
            return true;
        }
        if let Some(consumed) = self.apply_git_send() {
            return consumed;
        }
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_commit();
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            if self.editor_state.text_edit_insert("\n", self.now_ms) {
                self.mark_dirty();
            }
            return true;
        }
        // #20: Enter in the preset save-as-name input saves the preset
        // (native parity, `variable-theme-manager.tsx:298`).
        if self.commit_variables_preset_name_if_any() {
            return true;
        }
        // Enter in the variables search box just blurs it (the filter
        // is already live) — the same transition Escape runs.
        if self.editor_state.editor_ui.blur_variables_search() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.variables_header_rename_active() {
            self.commit_variables_panel_header_focus_if_any();
            return true;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            self.commit_variable_row_focus_if_any();
            return true;
        }
        if self.editor_state.editor_ui.effect_param_focus.is_some() {
            self.commit_effect_param_focus_if_any();
            return true;
        }
        if self.editor_state.ui.property_focus.is_some() {
            self.commit_property_focus_if_any();
            return true;
        }
        if self.editor_state.chat.available_models.is_empty() {
            return false;
        }
        // Real send with the AI transport (`codegen`); an honest
        // offline error on transport-less builds. See
        // `click.rs::begin_chat_send`.
        let sent = self.begin_chat_send();
        if sent {
            self.mark_dirty();
        }
        sent
    }

    /// Delete key — selected-node delete; never touches text
    /// drafts unless rename / text-edit owns the keyboard. Mirrors
    /// the native shell's `apply_delete`.
    pub fn apply_delete(&mut self) -> bool {
        if let Some(changed) =
            shared::prompt_center_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if self.apply_image_panel_delete() {
            return true;
        }
        // Forward deletion in the join input. A no-op falls through to
        // `delete_owned_by_chrome_input`, which still swallows the key
        // before it can reach the canvas selection.
        if self.editor_state.editor_ui.collab_join_input_active() {
            let changed = op_editor_ui::widgets::collab_ui::join_address_delete_forward(
                &mut self.editor_state.editor_ui,
                self.now_ms,
            )
            .unwrap_or(false);
            if changed {
                self.mark_dirty();
                return true;
            }
        }
        // The open font picker owns Delete. Its search draft handles
        // Backspace separately; forward-delete must never reach the canvas
        // selection behind the overlay.
        if self.editor_state.editor_ui.font_picker.open {
            return true;
        }
        // Settings-modal input owns Delete while focused (forward
        // deletion at the caret) — without this arm the keystroke fell
        // through and removed the selected node behind the modal.
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_delete_forward();
        }
        // The rename draft has no forward deletion — Delete pops the
        // char before the caret, same as Backspace.
        if let Some(changed) = shared::rename_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) = shared::text_edit_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // #20: the preset save-as-name draft takes forward-delete too
        // (native parity) — without this arm Delete fell through and
        // destroyed the selected node behind the open dropdown.
        if let Some(changed) =
            preset_name::preset_name_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Don't delete the selected node when a chrome text input or
        // search overlay owns the keyboard. The arms inside route
        // Delete into whichever draft is focused; everything else the
        // predicate covers simply swallows the key.
        if shared::delete_owned_by_chrome_input(&self.editor_state) {
            if self.editor_state.ui.property_focus.is_some()
                || self.editor_state.editor_ui.effect_param_focus.is_some()
            {
                let changed =
                    shared::property_input_delete_forward(&mut self.editor_state, self.now_ms);
                if changed {
                    self.mark_dirty();
                }
                return changed;
            }
            if self.editor_state.editor_ui.variables_header_rename_active() {
                let changed =
                    shared::variables_header_delete_forward(&mut self.editor_state, self.now_ms);
                if changed {
                    self.mark_dirty();
                }
                return changed;
            }
            if self.editor_state.editor_ui.variable_row_focus.is_some() {
                let changed =
                    shared::variable_row_delete_forward(&mut self.editor_state, self.now_ms);
                if changed {
                    self.mark_dirty();
                }
                return changed;
            }
            if self.editor_state.chat.focused
                && self.editor_state.chat.delete_input_selection(self.now_ms)
            {
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if shared::delete_selection_with_history(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Phase C2 keyboard forwarding stub. (No-op; the CanvasKit keydown handler
    /// dispatches per-key directly. Kept tested + ready.)
    #[allow(dead_code)]
    pub fn apply_key(&mut self, _event: &op_editor_ui::KeyEvent) -> bool {
        false
    }

    /// Route a printable character into the live preview runtime.
    /// Returns `true` when consumed. No-op (false) when not in preview.
    pub fn apply_preview_text(&mut self, text: &str) -> bool {
        self.preview_dispatch_text(text)
    }

    /// Dispatch a named key to the preview runtime.
    /// Returns `true` when consumed.
    pub fn apply_preview_key(&mut self, key: &str, shift: bool) -> bool {
        self.preview_dispatch_key(key, shift)
    }

    /// Advance focus in the preview. Returns `true` while in preview.
    pub fn apply_preview_focus(&mut self, shift: bool) -> bool {
        self.preview_focus(shift)
    }

    /// Check if preview is active. Used by keydown handler.
    pub fn is_preview_active(&self) -> bool {
        #[cfg(feature = "canvaskit")]
        {
            self.preview.is_some() && self.editor_state.editor_ui.preview.mode
        }
        #[cfg(not(feature = "canvaskit"))]
        {
            false
        }
    }

    /// Exit Preview mode. Delegates to the preview_frame module which handles
    /// Track M-1 animation logic.
    pub fn exit_preview(&mut self, viewport_width: f32, viewport_height: f32) {
        self.do_exit_preview(viewport_width, viewport_height);
    }
}
