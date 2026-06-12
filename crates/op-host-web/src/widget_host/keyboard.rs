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
        // Font-family picker search box (mirrors the native
        // font_picker_dispatch routing).
        if self.editor_state.editor_ui.font_family_picker_open {
            if c.is_control() {
                return false;
            }
            let ui = &mut self.editor_state.editor_ui;
            ui.font_picker_search.push(c);
            ui.font_picker_scroll = 0.0;
            ui.font_picker_hover = None;
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
        if self.editor_state.editor_ui.chat_model_picker_open {
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
            .insert_input_text(c.encode_utf8(&mut s));
        self.editor_state.chat.caret_anchor_ms = self.now_ms;
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
        // Font-family picker search box — swallow the key while the
        // picker is open even on an empty draft (no node deletion).
        if self.editor_state.editor_ui.font_family_picker_open {
            let ui = &mut self.editor_state.editor_ui;
            if ui.font_picker_search.pop().is_some() {
                ui.font_picker_scroll = 0.0;
                ui.font_picker_hover = None;
                self.mark_dirty();
            }
            return true;
        }
        if let Some(changed) = self.icon_picker_backspace() {
            return changed;
        }
        if self.editor_state.editor_ui.chat_model_picker_open {
            return self.apply_chat_model_picker_backspace();
        }
        if let Some(changed) = self.component_browser_backspace() {
            return changed;
        }
        if self.editor_state.chat.focused {
            if self.editor_state.chat.backspace_input() {
                self.editor_state.chat.caret_anchor_ms = self.now_ms;
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
            let ok = self.editor_state.text_edit_commit();
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.chat.input.trim().is_empty() {
            return false;
        }
        // Real send with the AI transport (`codegen`); an honest
        // offline error on transport-less builds. See
        // `click.rs::begin_chat_send`.
        self.begin_chat_send();
        self.mark_dirty();
        true
    }

    /// Delete key — selected-node delete; never touches text
    /// drafts unless rename / text-edit owns the keyboard. Mirrors
    /// the native shell's `apply_delete`.
    pub fn apply_delete(&mut self) -> bool {
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_backspace();
            if ok {
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
            || self.editor_state.editor_ui.chat_model_picker_open
            || self.editor_state.chat.focused
        {
            if self.editor_state.chat.focused && self.editor_state.chat.delete_input_selection() {
                self.editor_state.chat.caret_anchor_ms = self.now_ms;
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
            self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
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
        if let Some(focus) = self.editor_state.editor_ui.agent_settings.focus {
            let ui = &mut self.editor_state.editor_ui;
            ui.settings_input_select_all = true;
            ui.settings_input_caret = Some(ui.settings_input_draft.len());
            ui.settings_input_caret_focus = Some(focus);
            ui.settings_input_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
            rename.select_all = true;
            rename.caret = rename.draft.chars().count();
            self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.text_editing.is_some() {
            self.editor_state.ui.text_edit_select_all = true;
            self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.property_focus.is_some() {
            let ui = &mut self.editor_state.ui;
            ui.property_draft_select_all = true;
            ui.property_caret_pos = ui.property_input_draft.len();
            ui.property_caret_anchor_ms = self.now_ms;
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
        if self.editor_state.editor_ui.chat_model_picker_open {
            let ui = &mut self.editor_state.editor_ui;
            ui.chat_model_picker_select_all = true;
            ui.chat_model_picker_caret = Some(ui.chat_model_picker_search.len());
            ui.chat_model_picker_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.chat.focused {
            self.editor_state.chat.input_select_all = true;
            self.editor_state.chat.input_selection =
                Some(op_editor_core::chat::ChatInputSelection {
                    anchor: 0,
                    focus: self.editor_state.chat.input.len(),
                });
            self.editor_state.chat.caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        false
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
            self.editor_state.editor_ui.settings_input_draft.clear();
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
            self.editor_state.editor_ui.file_menu_hover = None;
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
        if self.editor_state.ui.property_focus.take().is_some() {
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.font_family_picker_open {
            let ui = &mut self.editor_state.editor_ui;
            ui.font_family_picker_open = false;
            ui.font_picker_search.clear();
            ui.font_picker_scroll = 0.0;
            ui.font_picker_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.locale_picker_open {
            self.editor_state.editor_ui.locale_picker_open = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.shape_picker_open {
            self.editor_state.editor_ui.shape_picker_open = false;
            self.editor_state.editor_ui.shape_picker_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.icon_picker_open {
            self.editor_state.editor_ui.icon_picker_open = false;
            self.editor_state.editor_ui.icon_picker_replace_selection = false;
            self.editor_state.editor_ui.icon_picker_search.clear();
            self.editor_state.editor_ui.icon_picker_select_all = false;
            self.editor_state.editor_ui.icon_picker_hover = None;
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
        if self.editor_state.editor_ui.fill_type_picker_open {
            self.editor_state.editor_ui.fill_type_picker_open = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.image_fill_popover_open {
            self.editor_state.editor_ui.image_fill_popover_open = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.chat_model_picker_open {
            self.editor_state.editor_ui.chat_model_picker_open = false;
            self.editor_state.editor_ui.chat_model_picker_scroll = 0.0;
            self.editor_state.editor_ui.chat_model_picker_search.clear();
            self.editor_state.editor_ui.chat_model_picker_caret = None;
            self.editor_state.editor_ui.chat_model_picker_select_all = false;
            self.editor_state.editor_ui.chat_model_picker_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.chat.focused {
            self.editor_state.chat.focused = false;
            self.editor_state.chat.input_select_all = false;
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

    /// True iff a text-input surface owns the keyboard. Gates the
    /// editor shortcuts so typing into a focused input never
    /// duplicates / nudges / reorders nodes.
    pub(in crate::widget_host) fn input_active(&self) -> bool {
        let ui = &self.editor_state.ui;
        ui.layer_rename.is_some()
            || ui.text_editing.is_some()
            || ui.property_focus.is_some()
            || self.editor_state.editor_ui.agent_settings.focus.is_some()
            || self.editor_state.editor_ui.icon_picker_open
            || self.editor_state.editor_ui.chat_model_picker_open
            || self.editor_state.editor_ui.component_browser_open
            || self.editor_state.chat.focused
    }

    /// IME composition forwarding. Only the final COMMIT lands in the
    /// focused input — preedit text is not painted (matches the native
    /// host, which routes winit `Ime::Commit` through `apply_text`
    /// char-by-char in `app_handler.rs` and ignores `Ime::Preedit`).
    /// Routing therefore covers every `apply_text` focus branch: chat,
    /// rename, canvas text edit, property / settings drafts, and the
    /// picker search boxes. Returns true when any character landed.
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

    /// Phase C2 keyboard forwarding stub.
    pub fn apply_key(&mut self, _event: &op_editor_ui::KeyEvent) { // glue:
    }

    /// Commit the in-progress settings-modal input draft (currently
    /// just the MCP server port). Parses u16, clamps ≥1024, writes
    /// back, clears focus + draft. Mirrors the native helper.
    pub(super) fn commit_settings_focus(&mut self) {
        use op_editor_core::agent_settings::{
            AcpAgentField, BuiltinAgentField, ImageGenField, SettingsFocus,
        };
        let Some(focus) = self.editor_state.editor_ui.agent_settings.focus.take() else {
            return;
        };
        let draft = std::mem::take(&mut self.editor_state.editor_ui.settings_input_draft);
        self.clear_settings_caret();
        match focus {
            SettingsFocus::McpPort => {
                if let Ok(port) = draft.trim().parse::<u16>() {
                    self.editor_state.editor_ui.agent_settings.mcp_server.port = port.max(1024);
                }
            }
            SettingsFocus::ImageSearch(field) => match field {
                op_editor_core::agent_settings::ImageSearchField::ClientId => {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .openverse_client_id = draft.trim().to_string();
                }
                op_editor_core::agent_settings::ImageSearchField::ClientSecret => {
                    self.editor_state
                        .editor_ui
                        .agent_settings
                        .openverse_client_secret = draft.trim().to_string();
                }
            },
            SettingsFocus::BuiltinAgent { index, field } => {
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agents
                    .get_mut(index)
                {
                    match field {
                        BuiltinAgentField::DisplayName => {
                            if !draft.trim().is_empty() {
                                agent.display_name = draft.trim().to_string();
                            }
                        }
                        BuiltinAgentField::ApiKey => agent.api_key = draft.trim().to_string(),
                        BuiltinAgentField::Model => agent.model = draft.trim().to_string(),
                        BuiltinAgentField::BaseUrl => {
                            agent.base_url = if draft.trim().is_empty() {
                                agent.kind.default_base_url().to_string()
                            } else {
                                draft.trim().to_string()
                            };
                        }
                    }
                    self.editor_state.rebuild_chat_models();
                }
            }
            SettingsFocus::BuiltinAgentDraft(field) => {
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .builtin_agent_draft
                    .as_mut()
                {
                    match field {
                        BuiltinAgentField::DisplayName => {
                            if !draft.trim().is_empty() {
                                agent.display_name = draft.trim().to_string();
                            }
                        }
                        BuiltinAgentField::ApiKey => agent.api_key = draft.trim().to_string(),
                        BuiltinAgentField::Model => agent.model = draft.trim().to_string(),
                        BuiltinAgentField::BaseUrl => {
                            agent.base_url = if draft.trim().is_empty() {
                                agent.kind.default_base_url().to_string()
                            } else {
                                draft.trim().to_string()
                            };
                        }
                    }
                }
            }
            SettingsFocus::ImageGenProfile { index, field } => {
                if let Some(profile) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .image_gen_profiles
                    .get_mut(index)
                {
                    match field {
                        ImageGenField::Name => profile.name = draft.trim().to_string(),
                        ImageGenField::ApiKey => profile.api_key = draft.trim().to_string(),
                        ImageGenField::Model => profile.model = draft.trim().to_string(),
                        ImageGenField::BaseUrl => {
                            profile.base_url = if draft.trim().is_empty() {
                                None
                            } else {
                                Some(draft.trim().to_string())
                            };
                        }
                    }
                }
            }
            SettingsFocus::AcpAgent { index, field } => {
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agents
                    .get_mut(index)
                {
                    match field {
                        AcpAgentField::DisplayName => {
                            if !draft.trim().is_empty() {
                                agent.display_name = draft.trim().to_string();
                            }
                        }
                        AcpAgentField::Command => {
                            agent.command = draft.trim().to_string();
                            agent.connected = false;
                        }
                        AcpAgentField::Args => {
                            agent.set_args_text(&draft);
                            agent.connected = false;
                        }
                        AcpAgentField::Env => {
                            agent.set_env_text(&draft);
                            agent.connected = false;
                        }
                        AcpAgentField::Url => {
                            agent.url = if draft.trim().is_empty() {
                                None
                            } else {
                                Some(draft.trim().to_string())
                            };
                            agent.connected = false;
                        }
                    }
                    self.editor_state.rebuild_chat_models();
                }
            }
            SettingsFocus::AcpAgentDraft(field) => {
                if let Some(agent) = self
                    .editor_state
                    .editor_ui
                    .agent_settings
                    .acp_agent_draft
                    .as_mut()
                {
                    match field {
                        AcpAgentField::DisplayName => {
                            if !draft.trim().is_empty() {
                                agent.display_name = draft.trim().to_string();
                            }
                        }
                        AcpAgentField::Command => {
                            agent.command = draft.trim().to_string();
                            agent.connected = false;
                        }
                        AcpAgentField::Args => {
                            agent.set_args_text(&draft);
                            agent.connected = false;
                        }
                        AcpAgentField::Env => {
                            agent.set_env_text(&draft);
                            agent.connected = false;
                        }
                        AcpAgentField::Url => {
                            agent.url = if draft.trim().is_empty() {
                                None
                            } else {
                                Some(draft.trim().to_string())
                            };
                            agent.connected = false;
                        }
                    }
                }
            }
        }
        self.mark_dirty();
    }
}
