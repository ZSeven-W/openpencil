//! Keyboard input handlers on `WidgetHostNative` — text input,
//! delete / duplicate / nudge, send, escape. Click routing +
//! marquee / layer-drag commit live in the sibling `click.rs`.
//!
//! `EditorState` is the host's source of truth: every focus / draft
//! / chat field is read + written on `editor_state`; mutations flag
//! the paint snapshot dirty.

use super::WidgetHostNative;
use op_editor_core::editor_ui_state::VariableRowFocus;

impl WidgetHostNative {
    /// Typed-char router: settings → rename → text-edit → variable
    /// row → property → chat.
    pub fn apply_text(&mut self, c: char) -> bool {
        // Settings input owns the keyboard while focused.
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            if c.is_ascii_digit() && self.editor_state.editor_ui.settings_input_draft.len() < 5 {
                self.editor_state.editor_ui.settings_input_draft.push(c);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // Git panel's commit-message input owns the keyboard next.
        if self.git_commit_focus_active() {
            if !c.is_control() {
                self.editor_state.editor_ui.git_panel.commit_message.push(c);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the Git panel's remote-URL input.
        if self.git_remote_focus_active() {
            if !c.is_control() {
                self.editor_state.editor_ui.git_panel.remote_draft.push(c);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the Git panel's HTTPS-credential input.
        if self.git_https_focus_active() {
            if !c.is_control() {
                self.editor_state.editor_ui.git_panel.https_draft.push(c);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.layer_rename.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            let _ = self.editor_state.rename_append(c.encode_utf8(&mut s));
            self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.text_editing.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            if self
                .editor_state
                .text_edit_append(c.encode_utf8(&mut s), self.now_ms)
            {
                self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
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
            if self.editor_state.ui.property_draft_select_all {
                self.editor_state.ui.property_input_draft.clear();
                self.editor_state.ui.property_caret_pos = 0;
            }
            self.editor_state.ui.property_draft_select_all = false;
            let pos = text_boundary_at_or_before(
                &self.editor_state.ui.property_input_draft,
                self.editor_state.ui.property_caret_pos,
            );
            self.editor_state.ui.property_input_draft.insert(pos, c);
            self.editor_state.ui.property_caret_pos = pos + c.len_utf8();
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if let Some(focus) = self.editor_state.editor_ui.variable_row_focus {
            let replace_selection = self.editor_state.ui.property_draft_select_all;
            let draft = &self.editor_state.ui.property_input_draft;
            let pos = if replace_selection {
                0
            } else {
                text_boundary_at_or_before(draft, self.editor_state.ui.property_caret_pos)
            };
            let allowed = match focus {
                VariableRowFocus::Name(_) => !c.is_control(),
                VariableRowFocus::Number(_) | VariableRowFocus::NumberCell { .. } => {
                    c.is_ascii_digit()
                        || (c == '-'
                            && (replace_selection || (pos == 0 && !draft.starts_with('-'))))
                        || (c == '.' && (replace_selection || !draft.contains('.')))
                }
                VariableRowFocus::String(_) | VariableRowFocus::StringCell { .. } => {
                    !c.is_control()
                }
            };
            if !allowed {
                return false;
            }
            if replace_selection {
                self.editor_state.ui.property_input_draft.clear();
                self.editor_state.ui.property_caret_pos = 0;
            }
            self.editor_state.ui.property_draft_select_all = false;
            self.editor_state.ui.property_input_draft.insert(pos, c);
            self.editor_state.ui.property_caret_pos = pos + c.len_utf8();
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.effect_param_focus.is_some() {
            // Effect-param value box — numeric, caret-aware insert
            // into the shared draft (same as a numeric property).
            self.editor_state.ui.property_draft_select_all = false;
            let draft = &self.editor_state.ui.property_input_draft;
            let pos = self.editor_state.ui.property_caret_pos.min(draft.len());
            let allowed = c.is_ascii_digit()
                || (c == '-' && pos == 0 && !draft.starts_with('-'))
                || (c == '.' && !draft.contains('.'));
            if !allowed {
                return false;
            }
            let draft = &mut self.editor_state.ui.property_input_draft;
            draft.insert(pos, c);
            self.editor_state.ui.property_caret_pos = pos + 1;
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if let Some(focus) = self.editor_state.ui.property_focus {
            self.editor_state.ui.property_draft_select_all = false;
            let is_hex_focus = focus.is_hex();
            // Caret byte-index — drafts are ASCII so it is also the
            // char index. `-` / `#` are gated on the caret being at
            // the start, NOT on the draft being empty: typing `-` at
            // the head of an existing `40` is a valid edit (`-40`).
            let draft = &self.editor_state.ui.property_input_draft;
            let pos = self.editor_state.ui.property_caret_pos.min(draft.len());
            let allowed = if is_hex_focus {
                // Cap at 7 chars (`#RRGGBB`) — per-stop alpha is
                // preserved at commit time so the user never types
                // raw alpha digits.
                draft.len() < 7
                    && (c.is_ascii_hexdigit() || (c == '#' && pos == 0 && !draft.starts_with('#')))
            } else {
                c.is_ascii_digit()
                    || (c == '-' && pos == 0 && !draft.starts_with('-'))
                    || (c == '.' && focus.accepts_decimal() && !draft.contains('.'))
            };
            if !allowed {
                return false;
            }
            // Insert at the caret and advance it.
            let draft = &mut self.editor_state.ui.property_input_draft;
            draft.insert(pos, c);
            self.editor_state.ui.property_caret_pos = pos + 1;
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.icon_picker_open && !c.is_control() {
            self.editor_state.editor_ui.icon_picker_search.push(c);
            self.editor_state.editor_ui.icon_picker_caret_anchor_ms = self.now_ms;
            self.editor_state.editor_ui.icon_picker_scroll = 0.0;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.component_browser_open && !c.is_control() {
            self.editor_state.editor_ui.component_browser_search.push(c);
            self.mark_dirty();
            return true;
        }
        if !self.editor_state.chat.focused {
            return false;
        }
        self.editor_state.chat.input.push(c);
        self.editor_state.chat.caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    /// Paste `text` into the focused chat input — appended at the
    /// caret (always the buffer end). Newlines are kept so a
    /// multi-line clipboard paste survives; the input widget wraps
    /// and honours `\n`. Returns `false` (no-op) when the chat input
    /// is not focused or `text` is empty. The desktop host calls
    /// this with the OS clipboard's contents on Cmd+V.
    pub fn chat_input_paste(&mut self, text: &str) -> bool {
        if !self.editor_state.chat.focused || text.is_empty() {
            return false;
        }
        self.editor_state.chat.input.push_str(text);
        self.editor_state.chat.caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    /// Cut the focused chat input — returns its text and empties the
    /// buffer. `None` when the chat input is not focused or already
    /// empty. The desktop host writes the returned text to the OS
    /// clipboard on Cmd+X.
    pub fn chat_input_cut(&mut self) -> Option<String> {
        if !self.editor_state.chat.focused || self.editor_state.chat.input.is_empty() {
            return None;
        }
        let taken = std::mem::take(&mut self.editor_state.chat.input);
        self.editor_state.chat.caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        Some(taken)
    }

    pub fn apply_backspace(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            self.editor_state.editor_ui.settings_input_draft.pop();
            self.mark_dirty();
            return true;
        }
        if self.git_commit_focus_active() {
            self.editor_state.editor_ui.git_panel.commit_message.pop();
            self.mark_dirty();
            return true;
        }
        if self.git_remote_focus_active() {
            self.editor_state.editor_ui.git_panel.remote_draft.pop();
            self.mark_dirty();
            return true;
        }
        if self.git_https_focus_active() {
            self.editor_state.editor_ui.git_panel.https_draft.pop();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_backspace();
            if ok {
                self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            let ok = self.editor_state.text_edit_backspace(self.now_ms);
            if ok {
                self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
            return ok;
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
            if self.editor_state.ui.property_draft_select_all {
                self.editor_state.ui.property_input_draft.clear();
                self.editor_state.ui.property_caret_pos = 0;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            let pos = text_boundary_at_or_before(
                &self.editor_state.ui.property_input_draft,
                self.editor_state.ui.property_caret_pos,
            );
            if pos > 0 {
                let prev = previous_text_boundary(&self.editor_state.ui.property_input_draft, pos);
                self.editor_state.ui.property_input_draft.drain(prev..pos);
                self.editor_state.ui.property_caret_pos = prev;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            if self.editor_state.ui.property_draft_select_all {
                self.editor_state.ui.property_input_draft.clear();
                self.editor_state.ui.property_caret_pos = 0;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            self.editor_state.ui.property_draft_select_all = false;
            let pos = text_boundary_at_or_before(
                &self.editor_state.ui.property_input_draft,
                self.editor_state.ui.property_caret_pos,
            );
            if pos > 0 {
                let prev = previous_text_boundary(&self.editor_state.ui.property_input_draft, pos);
                self.editor_state.ui.property_input_draft.drain(prev..pos);
                self.editor_state.ui.property_caret_pos = prev;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            self.editor_state.ui.property_draft_select_all = false;
            // Delete the char before the caret, then pull it back.
            let draft = &mut self.editor_state.ui.property_input_draft;
            let pos = self.editor_state.ui.property_caret_pos.min(draft.len());
            if pos > 0 {
                draft.remove(pos - 1);
                self.editor_state.ui.property_caret_pos = pos - 1;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.editor_ui.icon_picker_open {
            if self
                .editor_state
                .editor_ui
                .icon_picker_search
                .pop()
                .is_some()
            {
                self.editor_state.editor_ui.icon_picker_caret_anchor_ms = self.now_ms;
                self.editor_state.editor_ui.icon_picker_scroll = 0.0;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.editor_ui.component_browser_open {
            if self
                .editor_state
                .editor_ui
                .component_browser_search
                .pop()
                .is_some()
            {
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.chat.focused {
            if self.editor_state.chat.input.pop().is_some() {
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

    /// Delete — pops a char from rename / text-edit when active;
    /// otherwise deletes the selected node.
    pub fn apply_delete(&mut self) -> bool {
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
            if self.editor_state.ui.property_draft_select_all {
                self.editor_state.ui.property_input_draft.clear();
                self.editor_state.ui.property_caret_pos = 0;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            let pos = text_boundary_at_or_before(
                &self.editor_state.ui.property_input_draft,
                self.editor_state.ui.property_caret_pos,
            );
            if pos < self.editor_state.ui.property_input_draft.len() {
                let next = next_text_boundary(&self.editor_state.ui.property_input_draft, pos);
                self.editor_state.ui.property_input_draft.drain(pos..next);
                self.editor_state.ui.property_caret_pos = pos;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            if self.editor_state.ui.property_draft_select_all {
                self.editor_state.ui.property_input_draft.clear();
                self.editor_state.ui.property_caret_pos = 0;
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            self.editor_state.ui.property_draft_select_all = false;
            let pos = text_boundary_at_or_before(
                &self.editor_state.ui.property_input_draft,
                self.editor_state.ui.property_caret_pos,
            );
            if pos < self.editor_state.ui.property_input_draft.len() {
                let next = next_text_boundary(&self.editor_state.ui.property_input_draft, pos);
                self.editor_state.ui.property_input_draft.drain(pos..next);
                self.editor_state.ui.property_caret_pos = pos;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.layer_rename.is_some() {
            let ok = self.editor_state.rename_backspace();
            if ok {
                self.editor_state.editor_ui.rename_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
            return ok;
        }
        if self.editor_state.ui.text_editing.is_some() {
            let ok = self.editor_state.text_edit_backspace(self.now_ms);
            if ok {
                self.editor_state.ui.text_edit_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
            }
            return ok;
        }
        // Don't delete the selected node when any text input owns
        // the keyboard — property focus, effect-param focus, or the
        // chat input. The text-input branches above handle their
        // own backspace; falling through to `delete_selected` here
        // would silently drop the node behind the focused field.
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
            || self.editor_state.editor_ui.icon_picker_open
            || self.editor_state.editor_ui.component_browser_open
            || self.editor_state.chat.focused
        {
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
            self.commit_settings_focus_if_any();
            return true;
        }
        // Enter in the Git commit input requests a commit — needs a
        // message and a staged file (the commit is the staged set).
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.commit_message.trim().is_empty()
                && panel.changed_files.iter().any(|f| f.staged)
            {
                panel.pending_action = Some(op_editor_core::GitPanelAction::Commit);
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the Git remote-URL input sets `origin`.
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.remote_draft.trim().is_empty() {
                panel.pending_action = Some(op_editor_core::GitPanelAction::SetRemote(
                    panel.remote_draft.clone(),
                ));
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the Git HTTPS-credential input stores it.
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.https_draft.trim().is_empty() {
                panel.pending_action = Some(op_editor_core::GitPanelAction::SetHttpsAuth(
                    panel.https_draft.clone(),
                ));
            }
            self.mark_dirty();
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
        if self.editor_state.ui.pen_in_progress.is_some() {
            let ok = self.editor_state.finish_pen_path();
            if ok {
                self.mark_dirty();
            }
            return ok;
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
        // `begin_send` itself gates on (text OR staged attachments) —
        // an attachment-only turn is valid, so don't short-circuit on
        // empty text here.
        // Real provider turn — raises `chat.pending_send`.
        let sent = self.editor_state.chat.begin_send();
        if sent {
            self.mark_dirty();
        }
        sent
    }

    /// Escape — priority cascade: rename → property → pickers →
    /// chat → selection. One layer per press.
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
            self.mark_dirty();
            return true;
        }
        // Escape defocuses the Git commit input (the panel stays open).
        if self.git_commit_focus_active() {
            self.editor_state.editor_ui.git_panel.commit_focused = false;
            self.mark_dirty();
            return true;
        }
        // …and the Git remote-URL input.
        if self.git_remote_focus_active() {
            self.editor_state.editor_ui.git_panel.remote_focused = false;
            self.mark_dirty();
            return true;
        }
        // …and the Git HTTPS-credential input.
        if self.git_https_focus_active() {
            self.editor_state.editor_ui.git_panel.https_focused = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.agent_settings_open {
            self.editor_state.editor_ui.agent_settings_open = false;
            self.editor_state.editor_ui.agent_settings_drag = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.export_dialog_open {
            self.editor_state.editor_ui.export_dialog_open = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.figma_import_open {
            self.editor_state.editor_ui.figma_import_open = false;
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
        if self.editor_state.finish_pen_path() {
            self.mark_dirty();
            return true;
        }
        if self
            .editor_state
            .editor_ui
            .variables_theme_rename_axis
            .take()
            .is_some()
            || self
                .editor_state
                .editor_ui
                .variables_variant_rename_value
                .take()
                .is_some()
        {
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.mark_dirty();
            return true;
        }
        if self
            .editor_state
            .editor_ui
            .variable_row_focus
            .take()
            .is_some()
        {
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
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
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.property_focus.take().is_some() {
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.locale_picker_open {
            self.editor_state.editor_ui.locale_picker_open = false;
            self.editor_state.editor_ui.locale_picker_hover = None;
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
            self.editor_state.editor_ui.icon_picker_scroll = 0.0;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.component_browser_open {
            self.editor_state.editor_ui.component_browser_open = false;
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
        if self.editor_state.chat.focused {
            self.editor_state.chat.focused = false;
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

fn text_boundary_at_or_before(value: &str, pos: usize) -> usize {
    let mut clipped = pos.min(value.len());
    while clipped > 0 && !value.is_char_boundary(clipped) {
        clipped -= 1;
    }
    clipped
}

fn previous_text_boundary(value: &str, pos: usize) -> usize {
    let pos = text_boundary_at_or_before(value, pos);
    value[..pos]
        .char_indices()
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(0)
}

fn next_text_boundary(value: &str, pos: usize) -> usize {
    let pos = text_boundary_at_or_before(value, pos);
    if pos >= value.len() {
        return value.len();
    }
    pos + value[pos..].chars().next().map(char::len_utf8).unwrap_or(0)
}
