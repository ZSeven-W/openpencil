//! Keyboard / clipboard handlers on the web `WidgetHost`.
//! Pulled out of `widget_host.rs` so the spine file stays
//! under the 800-line ceiling. Mirrors the native shell's
//! `widget_host/input.rs` + `keyboard.rs` shape.
//!
//! `EditorState` is the host's source of truth: every focus / draft
//! / chat field is read + written on `editor_state`; mutations flag
//! the paint snapshot dirty.

use super::WidgetHost;

impl WidgetHost {
    /// Push a typed character into the focused chat / settings input.
    /// Returns true if anything changed.
    pub fn apply_text(&mut self, c: char) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_text(c);
        }
        if self.editor_state.ui.layer_rename.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            if self.editor_state.rename_append(c.encode_utf8(&mut s)) {
                if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
                    rename.input.touch(self.now_ms);
                }
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.text_editing.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            if self
                .editor_state
                .text_edit_insert(c.encode_utf8(&mut s), self.now_ms)
            {
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // Variables-panel search filter — live append (mirrors the
        // native host's append/pop discipline).
        if self.variables_search_active() && !c.is_control() {
            self.editor_state.editor_ui.variables_search.push(c);
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.editor_state.editor_ui.variables_scroll.offset = 0.0;
            self.mark_dirty();
            return true;
        }
        // Variables-panel theme/variant header rename drafts.
        if (self
            .editor_state
            .editor_ui
            .variables_theme_rename_axis
            .is_some()
            || self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .is_some())
            && !c.is_control()
        {
            let mut s = [0u8; 4];
            self.editor_state
                .editor_ui
                .variables_header_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.sync_variables_header_input_legacy(false);
            self.mark_dirty();
            return true;
        }
        // Variables-panel row/cell drafts — per-kind char gates
        // mirror the native host (numeric / free text / hex).
        if let Some(focus) = self.editor_state.editor_ui.variable_row_focus {
            use op_editor_core::editor_ui_state::VariableRowFocus;
            let input = &self.editor_state.editor_ui.variable_row_input;
            let replacing_all = input.is_select_all();
            let draft = input.text();
            let pos = if replacing_all {
                0
            } else {
                input.caret().min(draft.len())
            };
            let len_after_clear = if replacing_all { 0 } else { draft.len() };
            let allowed = match focus {
                VariableRowFocus::Name(_) => !c.is_control(),
                VariableRowFocus::Number(_) | VariableRowFocus::NumberCell { .. } => {
                    c.is_ascii_digit()
                        || (c == '-' && (replacing_all || (pos == 0 && !draft.starts_with('-'))))
                        || (c == '.' && (replacing_all || !draft.contains('.')))
                }
                VariableRowFocus::String(_) | VariableRowFocus::StringCell { .. } => {
                    !c.is_control()
                }
                // Inline color hex — `#` only at the front, hex digits
                // after, capped at `#rrggbb`.
                VariableRowFocus::ColorCell { .. } => {
                    if c == '#' {
                        len_after_clear == 0
                    } else {
                        c.is_ascii_hexdigit() && len_after_clear < 7
                    }
                }
            };
            if !allowed {
                return false;
            }
            let mut s = [0u8; 4];
            self.editor_state
                .editor_ui
                .variable_row_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.sync_variable_row_input_legacy(false);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            if c.is_control() {
                return false;
            }
            let input = &self.editor_state.ui.property_input;
            let replacing_all = input.is_select_all();
            let draft = input.text();
            let pos = if replacing_all {
                0
            } else {
                input.caret().min(draft.len())
            };
            let allowed = if let Some(focus) = self.editor_state.ui.property_focus {
                if focus.is_free_text() {
                    // Widget text rows (placeholder / value / label /
                    // icon names / bind key) take any non-control char.
                    !c.is_control()
                } else if focus.is_hex() {
                    (replacing_all || draft.len() < 7)
                        && (c.is_ascii_hexdigit()
                            || (c == '#' && pos == 0 && !draft.starts_with('#')))
                } else {
                    c.is_ascii_digit()
                        || (c == '-' && pos == 0 && (replacing_all || !draft.starts_with('-')))
                        || (c == '.'
                            && focus.accepts_decimal()
                            && (replacing_all || !draft.contains('.')))
                }
            } else {
                c.is_ascii_digit()
                    || (c == '-' && pos == 0 && (replacing_all || !draft.starts_with('-')))
                    || (c == '.' && (replacing_all || !draft.contains('.')))
            };
            if !allowed {
                return false;
            }
            let mut s = [0u8; 4];
            self.editor_state
                .ui
                .property_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.sync_property_input_legacy(false);
            self.mark_dirty();
            return true;
        }
        // Font-family picker search box (mirrors the native
        // font_picker_dispatch routing).
        if self.editor_state.editor_ui.font_picker.open {
            if c.is_control() {
                return false;
            }
            let ui = &mut self.editor_state.editor_ui;
            ui.font_picker_search.push(c);
            ui.font_picker.scroll.offset = 0.0;
            ui.font_picker.hover = None;
            self.mark_dirty();
            return true;
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
        if !self.editor_state.chat.focused {
            return false;
        }
        if c.is_control() {
            return false;
        }
        let mut s = [0u8; 4];
        self.editor_state
            .chat
            .insert_input_text(c.encode_utf8(&mut s), self.now_ms);
        self.mark_dirty();
        true
    }

    pub fn apply_backspace(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_backspace();
        }
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_backspace();
            if ok {
                if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
                    rename.input.touch(self.now_ms);
                }
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            let ok = self.editor_state.text_edit_backspace(self.now_ms);
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
        // Variables-panel search filter — pop one char.
        if self.variables_search_active() {
            if self.editor_state.editor_ui.variables_search.pop().is_some() {
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.editor_state.editor_ui.variables_scroll.offset = 0.0;
                self.mark_dirty();
                return true;
            }
            return false;
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
            let before = (
                self.editor_state
                    .editor_ui
                    .variables_header_input
                    .text()
                    .to_owned(),
                self.editor_state.editor_ui.variables_header_input.caret(),
            );
            self.editor_state
                .editor_ui
                .variables_header_input
                .backspace(self.now_ms);
            let after = (
                self.editor_state
                    .editor_ui
                    .variables_header_input
                    .text()
                    .to_owned(),
                self.editor_state.editor_ui.variables_header_input.caret(),
            );
            if after != before {
                self.sync_variables_header_input_legacy(false);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            let before = (
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .text()
                    .to_owned(),
                self.editor_state.editor_ui.variable_row_input.caret(),
            );
            self.editor_state
                .editor_ui
                .variable_row_input
                .backspace(self.now_ms);
            let after = (
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .text()
                    .to_owned(),
                self.editor_state.editor_ui.variable_row_input.caret(),
            );
            if after != before {
                self.sync_variable_row_input_legacy(false);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            let before = (
                self.editor_state.ui.property_input.text().to_owned(),
                self.editor_state.ui.property_input.caret(),
            );
            self.editor_state.ui.property_input.backspace(self.now_ms);
            let after = (
                self.editor_state.ui.property_input.text().to_owned(),
                self.editor_state.ui.property_input.caret(),
            );
            if after != before {
                self.sync_property_input_legacy(false);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // Font-family picker search box — swallow the key while the
        // picker is open even on an empty draft (no node deletion).
        if self.editor_state.editor_ui.font_picker.open {
            let ui = &mut self.editor_state.editor_ui;
            if ui.font_picker_search.pop().is_some() {
                ui.font_picker.scroll.offset = 0.0;
                ui.font_picker.hover = None;
                self.mark_dirty();
            }
            return true;
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
        if self.editor_state.chat.focused {
            if self.editor_state.chat.backspace_input(self.now_ms) {
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.delete_selected() {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

    pub fn apply_send(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            self.commit_settings_focus();
            return true;
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
        // Enter in the variables search box just blurs it (the
        // filter is already live).
        if self.variables_search_active() {
            self.editor_state.editor_ui.variables_search_focus = false;
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
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_backspace();
            if ok {
                if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
                    rename.input.touch(self.now_ms);
                }
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            let ok = self.editor_state.text_edit_backspace(self.now_ms);
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
        // Don't delete the selected node when a text input owns the
        // keyboard — the model-picker search, property focus, or chat
        // input. Their own backspace handlers run earlier; falling
        // through to `delete_selected` would drop the node behind the
        // focused field.
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
            || self.editor_state.editor_ui.variable_row_focus.is_some()
            || self.variables_search_active()
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
            || self.editor_state.editor_ui.chat_model_picker.open
            || self.editor_state.chat.focused
        {
            if self.editor_state.ui.property_focus.is_some()
                || self.editor_state.editor_ui.effect_param_focus.is_some()
            {
                let before = (
                    self.editor_state.ui.property_input.text().to_owned(),
                    self.editor_state.ui.property_input.caret(),
                );
                self.editor_state
                    .ui
                    .property_input
                    .delete_forward(self.now_ms);
                let after = (
                    self.editor_state.ui.property_input.text().to_owned(),
                    self.editor_state.ui.property_input.caret(),
                );
                if after != before {
                    self.sync_property_input_legacy(false);
                    self.mark_dirty();
                    return true;
                }
                return false;
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
                let before = (
                    self.editor_state
                        .editor_ui
                        .variables_header_input
                        .text()
                        .to_owned(),
                    self.editor_state.editor_ui.variables_header_input.caret(),
                );
                self.editor_state
                    .editor_ui
                    .variables_header_input
                    .delete_forward(self.now_ms);
                let after = (
                    self.editor_state
                        .editor_ui
                        .variables_header_input
                        .text()
                        .to_owned(),
                    self.editor_state.editor_ui.variables_header_input.caret(),
                );
                if after != before {
                    self.sync_variables_header_input_legacy(false);
                    self.mark_dirty();
                    return true;
                }
                return false;
            }
            if self.editor_state.editor_ui.variable_row_focus.is_some() {
                let before = (
                    self.editor_state
                        .editor_ui
                        .variable_row_input
                        .text()
                        .to_owned(),
                    self.editor_state.editor_ui.variable_row_input.caret(),
                );
                self.editor_state
                    .editor_ui
                    .variable_row_input
                    .delete_forward(self.now_ms);
                let after = (
                    self.editor_state
                        .editor_ui
                        .variable_row_input
                        .text()
                        .to_owned(),
                    self.editor_state.editor_ui.variable_row_input.caret(),
                );
                if after != before {
                    self.sync_variable_row_input_legacy(false);
                    self.mark_dirty();
                    return true;
                }
                return false;
            }
            if self.editor_state.chat.focused
                && self.editor_state.chat.delete_input_selection(self.now_ms)
            {
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.delete_selected() {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

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

    /// IME composition forwarding. Only the final COMMIT lands in the
    /// focused input — preedit text is not painted (matches the native
    /// host, which routes winit `Ime::Commit` through `apply_text`
    /// char-by-char in `app_handler.rs` and ignores `Ime::Preedit`).
    /// Routing therefore covers every `apply_text` focus branch: chat,
    /// rename, canvas text edit, property / settings drafts, and the
    /// picker search boxes. Returns true when any character landed.
    // IME composition entry — tested + ready to wire; the CanvasKit keydown
    // handler does per-key dispatch and hasn't wired IME composition yet.
    #[allow(dead_code)]
    pub fn apply_ime(&mut self, event: &op_editor_ui::ImeEvent) -> bool {
        if !matches!(event.kind, op_editor_ui::ImeKind::CompositionEnd) {
            return false;
        }
        self.apply_paste_text(&event.text)
    }

    /// Route a multi-character text payload (IME commit, clipboard
    /// paste) into whichever input owns the keyboard, char-by-char
    /// through `apply_text` so every focus branch + per-field filter
    /// (numeric / hex drafts) applies unchanged. Returns true when at
    /// least one character landed.
    pub fn apply_paste_text(&mut self, text: &str) -> bool {
        let mut consumed = false;
        for c in text.chars() {
            if !c.is_control() && self.apply_text(c) {
                consumed = true;
            }
        }
        consumed
    }

    /// Phase C2 keyboard forwarding stub. (No-op; the CanvasKit keydown handler
    /// dispatches per-key directly. Kept tested + ready.)
    #[allow(dead_code)]
    pub fn apply_key(&mut self, _event: &op_editor_ui::KeyEvent) -> bool {
        false
    }
}
