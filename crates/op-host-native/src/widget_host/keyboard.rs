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

    /// Typed-char router: settings → rename → text-edit → variable
    /// row → property → chat.
    pub fn apply_text(&mut self, c: char) -> bool {
        // Settings input owns the keyboard while focused.
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_text(c);
        }
        // The inline clone wizard owns the keyboard while it is open: a
        // focused URL / destination field takes the character (unless a
        // clone is already running), and every other key is swallowed so
        // nothing reaches the canvas.
        if self.git_clone_input_active() {
            if c.is_control() {
                return false;
            }
            let now = self.now_ms;
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                if !form.cloning {
                    let mut s = [0u8; 4];
                    match form.focus {
                        Some(op_editor_core::CloneField::Url) => {
                            form.url_input.insert_str(c.encode_utf8(&mut s), now)
                        }
                        Some(op_editor_core::CloneField::Dest) => {
                            form.dest_input.insert_str(c.encode_utf8(&mut s), now)
                        }
                        None => {}
                    }
                    form.error = None;
                }
            }
            self.mark_dirty();
            return true;
        }
        // Git panel's commit-message input owns the keyboard next.
        if self.git_commit_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel.commit_input.insert_str(c.encode_utf8(&mut s), now);
                panel.commit_no_changes = false;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the Git panel's remote-URL input.
        if self.git_remote_focus_active() {
            if !c.is_control() {
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .remote_input
                    .insert_str(c.encode_utf8(&mut s), self.now_ms);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the Git panel's HTTPS-credential input.
        if self.git_https_focus_active() {
            if !c.is_control() {
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .https_input
                    .insert_str(c.encode_utf8(&mut s), self.now_ms);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // …then the commit-signature form's name / email inputs.
        if self.git_author_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                if panel.author_email_focused {
                    panel
                        .author_email_input
                        .insert_str(c.encode_utf8(&mut s), now);
                } else {
                    panel
                        .author_name_input
                        .insert_str(c.encode_utf8(&mut s), now);
                }
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.git_branch_create_focus_active() {
            if !c.is_control() {
                let now = self.now_ms;
                let panel = &mut self.editor_state.editor_ui.git_panel;
                let mut s = [0u8; 4];
                panel
                    .branch_create_input
                    .insert_str(c.encode_utf8(&mut s), now);
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.ui.layer_rename.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            let _ = self.editor_state.rename_append(c.encode_utf8(&mut s));
            if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
                rename.input.touch(self.now_ms);
            }
            self.mark_dirty();
            return true;
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
        // Variables-panel search filter — live append, no draft /
        // commit machinery (TS controlled `<input>`; same append/pop
        // discipline as the font-picker search).
        if self.variables_search_active() && !c.is_control() {
            self.editor_state.editor_ui.variables_search.push(c);
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            // A narrower list invalidates the scroll offset — the
            // widget clamps, but reset for a stable reveal-from-top.
            self.editor_state.editor_ui.variables_scroll = 0.0;
            self.mark_dirty();
            return true;
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
            let mut s = [0u8; 4];
            self.editor_state
                .editor_ui
                .variables_header_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.sync_variables_header_input_legacy(false);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.preset_name_input_active() && !c.is_control() {
            let replace_selection = self.editor_state.ui.property_draft_select_all;
            let pos = if replace_selection {
                0
            } else {
                text_boundary_at_or_before(
                    &self.editor_state.ui.property_input_draft,
                    self.editor_state.ui.property_caret_pos,
                )
            };
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
        if let Some(focus) = self.editor_state.editor_ui.variable_row_focus {
            let input = &self.editor_state.editor_ui.variable_row_input;
            let replacing_all = input.is_select_all();
            let draft = input.text();
            let pos = if replacing_all {
                0
            } else {
                input.caret().min(draft.len())
            };
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
                // Inline color hex — `#` only at the front, hex
                // digits after, capped at `#rrggbb` (same gating as
                // the property panel's FillHex draft).
                VariableRowFocus::ColorCell { .. } => {
                    let len_after_clear = if replacing_all { 0 } else { draft.len() };
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
        if self.editor_state.editor_ui.effect_param_focus.is_some() {
            // Effect-param value box — numeric, caret-aware insert
            // into the shared text-input state (same as a numeric property).
            let input = &self.editor_state.ui.property_input;
            let replacing_all = input.is_select_all();
            let draft = input.text();
            let pos = if replacing_all {
                0
            } else {
                input.caret().min(draft.len())
            };
            let allowed = c.is_ascii_digit()
                || (c == '-' && pos == 0 && (replacing_all || !draft.starts_with('-')))
                || (c == '.' && (replacing_all || !draft.contains('.')));
            if !allowed {
                return false;
            }
            let mut s = [0u8; 4];
            self.editor_state
                .ui
                .property_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.editor_state.ui.property_input_draft =
                self.editor_state.ui.property_input.text().to_owned();
            self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
            self.editor_state.ui.property_draft_select_all = false;
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        if let Some(focus) = self.editor_state.ui.property_focus {
            let input = &self.editor_state.ui.property_input;
            let replacing_all = input.is_select_all();
            let is_hex_focus = focus.is_hex();
            // Caret byte-index — drafts are ASCII so it is also the
            // char index. `-` / `#` are gated on the caret being at
            // the start, NOT on the draft being empty: typing `-` at
            // the head of an existing `40` is a valid edit (`-40`).
            let draft = input.text();
            let pos = if replacing_all {
                0
            } else {
                input.caret().min(draft.len())
            };
            let allowed = if is_hex_focus {
                // Cap at 7 chars (`#RRGGBB`) — per-stop alpha is
                // preserved at commit time so the user never types
                // raw alpha digits.
                (replacing_all || draft.len() < 7)
                    && (c.is_ascii_hexdigit() || (c == '#' && pos == 0 && !draft.starts_with('#')))
            } else {
                c.is_ascii_digit()
                    || (c == '-' && pos == 0 && (replacing_all || !draft.starts_with('-')))
                    || (c == '.'
                        && focus.accepts_decimal()
                        && (replacing_all || !draft.contains('.')))
            };
            if !allowed {
                return false;
            }
            // Insert at the caret and advance it.
            let mut s = [0u8; 4];
            self.editor_state
                .ui
                .property_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.editor_state.ui.property_input_draft =
                self.editor_state.ui.property_input.text().to_owned();
            self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
            self.editor_state.ui.property_draft_select_all = false;
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        // Font-family picker search box (font_picker_dispatch.rs).
        if self.apply_font_picker_text(c) {
            return true;
        }
        // Image-node Search / Generate popover inputs.
        if self.apply_image_panel_text(c) {
            return true;
        }
        if self.editor_state.editor_ui.icon_picker_open && !c.is_control() {
            if self.editor_state.editor_ui.icon_picker_select_all {
                self.editor_state.editor_ui.icon_picker_search.clear();
                self.editor_state.editor_ui.icon_picker_select_all = false;
            }
            self.editor_state.editor_ui.icon_picker_search.push(c);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return self.apply_chat_model_picker_text(c);
        }
        if self.editor_state.editor_ui.component_browser_open && !c.is_control() {
            if self.editor_state.editor_ui.component_browser_select_all {
                self.editor_state.editor_ui.component_browser_search.clear();
                self.editor_state.editor_ui.component_browser_select_all = false;
            }
            self.editor_state.editor_ui.component_browser_search.push(c);
            self.mark_dirty();
            return true;
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
        self.editor_state.chat.insert_input_text(text, self.now_ms);
        self.mark_dirty();
        true
    }

    /// Paste clipboard `text` into whichever text input currently owns
    /// the keyboard — the clone-wizard URL / destination, the git commit
    /// message, the remote / HTTPS draft, or a settings field. Each
    /// character is routed through [`Self::apply_text`], so per-input
    /// filtering (e.g. digits-only for the MCP port, the clone field's
    /// `!cloning` lock) still applies; control characters / newlines are
    /// dropped since these inputs are single-line. Returns `true` if
    /// anything was inserted.
    pub fn apply_input_paste(&mut self, text: &str) -> bool {
        let mut inserted = false;
        for c in text.chars() {
            if c.is_control() {
                continue;
            }
            if self.apply_text(c) {
                inserted = true;
            }
        }
        inserted
    }

    /// Cut the focused chat input — returns its text and empties the
    /// buffer. `None` when the chat input is not focused or already
    /// empty. The desktop host writes the returned text to the OS
    /// clipboard on Cmd+X.
    pub fn chat_input_cut(&mut self) -> Option<String> {
        if !self.editor_state.chat.focused || self.editor_state.chat.input.text().is_empty() {
            return None;
        }
        if let Some(selected) = self
            .editor_state
            .chat
            .selected_input_text()
            .map(str::to_string)
        {
            self.editor_state.chat.delete_input_selection(self.now_ms);
            self.mark_dirty();
            return Some(selected);
        }
        let taken = self.editor_state.chat.input.text().to_owned();
        self.editor_state.chat.set_input_text("");
        self.editor_state.chat.input.touch(self.now_ms);
        self.mark_dirty();
        Some(taken)
    }

    pub fn apply_backspace(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_backspace();
        }
        if self.git_clone_input_active() {
            // Swallow Backspace whenever the wizard is open so it can
            // never delete a selected node; pop a char only from a
            // focused field that isn't mid-clone.
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                if !form.cloning {
                    match form.focus {
                        Some(op_editor_core::CloneField::Url) => {
                            form.url_input.backspace(self.now_ms)
                        }
                        Some(op_editor_core::CloneField::Dest) => {
                            form.dest_input.backspace(self.now_ms)
                        }
                        None => {}
                    }
                    form.error = None;
                }
            }
            self.mark_dirty();
            return true;
        }
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.commit_input.backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.remote_input.backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.https_input.backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_author_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if panel.author_email_focused {
                panel.author_email_input.backspace(self.now_ms);
            } else {
                panel.author_name_input.backspace(self.now_ms);
            }
            self.mark_dirty();
            return true;
        }
        if self.git_branch_create_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.branch_create_input.backspace(self.now_ms);
            self.mark_dirty();
            return true;
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
                self.editor_state.editor_ui.variables_scroll = 0.0;
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
        if self.editor_state.editor_ui.preset_name_input_active() {
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
                self.editor_state.ui.property_input_draft =
                    self.editor_state.ui.property_input.text().to_owned();
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.ui.property_input.caret();
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // Font-family picker search box (font_picker_dispatch.rs).
        if self.apply_font_picker_backspace() {
            return true;
        }
        // Image-node Search / Generate popover inputs.
        if self.apply_image_panel_backspace() {
            return true;
        }
        if self.editor_state.editor_ui.icon_picker_open {
            if self.editor_state.editor_ui.icon_picker_select_all {
                self.editor_state.editor_ui.icon_picker_search.clear();
                self.editor_state.editor_ui.icon_picker_select_all = false;
                self.mark_dirty();
                return true;
            }
            if self
                .editor_state
                .editor_ui
                .icon_picker_search
                .pop()
                .is_some()
            {
                self.mark_dirty();
                return true;
            }
            return false;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            return self.apply_chat_model_picker_backspace();
        }
        if self.editor_state.editor_ui.component_browser_open {
            if self.editor_state.editor_ui.component_browser_select_all {
                self.editor_state.editor_ui.component_browser_search.clear();
                self.editor_state.editor_ui.component_browser_select_all = false;
                self.mark_dirty();
                return true;
            }
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
            if self.editor_state.chat.backspace_input(self.now_ms) {
                self.mark_dirty();
                return true;
            }
            return false;
        }
        // Pen authoring: Backspace pops the last anchor (`pen_press.rs`).
        if self.apply_pen_backspace() {
            return true;
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
        if self.editor_state.editor_ui.preset_name_input_active() {
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
            // Delete is FORWARD deletion at the caret (or removes the
            // active selection) — textarea parity.
            let ok = self.editor_state.text_edit_delete_forward(self.now_ms);
            if ok {
                self.mark_dirty();
            }
            return ok;
        }
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
                self.editor_state.ui.property_input_draft =
                    self.editor_state.ui.property_input.text().to_owned();
                self.editor_state.ui.property_caret_pos =
                    self.editor_state.ui.property_input.caret();
                self.editor_state.ui.property_draft_select_all = false;
                self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
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
        // Don't delete the selected node when any text input owns
        // the keyboard — property focus, effect-param focus, or the
        // chat input. The text-input branches above handle their
        // own backspace; falling through to `delete_selected` here
        // would silently drop the node behind the focused field.
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
            || self.editor_state.editor_ui.icon_picker_open
            || self.editor_state.editor_ui.chat_model_picker.open
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

    /// Cmd-D — duplicate selection as a sibling at +10 doc px.
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

    /// Up / Down arrow on a focused numeric property input — steps
    /// the value by `delta` and commits it (like a `−` / `+`
    /// stepper). Returns `false` when no numeric property input is
    /// focused, so the caller falls back to nudging the selection.
    pub fn apply_property_step(&mut self, delta: f32) -> bool {
        // Effect-parameter focus: step the value, commit via
        // `SetEffectParam`, and reflect it back into the draft.
        if let Some(ef) = self.editor_state.editor_ui.effect_param_focus {
            let current: f32 = self
                .editor_state
                .ui
                .property_input
                .text()
                .trim()
                .parse()
                .unwrap_or(0.0);
            let next = current + delta;
            let id = self.editor_state.selection.anchor.clone();
            if id.is_real() {
                self.editor_state.commit_history();
                let _ = self
                    .editor_state
                    .apply(op_editor_core::EditorCommand::SetEffectParam {
                        node_id: id,
                        index: ef.effect as u32,
                        field: ef.field,
                        value: next,
                    });
            }
            let next_text = if next.fract() == 0.0 {
                format!("{}", next as i64)
            } else {
                format!("{next}")
            };
            self.editor_state
                .ui
                .property_input
                .set_text(next_text.clone());
            self.editor_state.ui.property_input.touch(self.now_ms);
            self.editor_state.ui.property_input_draft = next_text;
            self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
            return true;
        }
        let Some(focus) = self.editor_state.ui.property_focus else {
            return false;
        };
        // Hex colour fields aren't numerically steppable.
        if focus.is_hex() {
            return false;
        }
        let current: f32 = self
            .editor_state
            .ui
            .property_input
            .text()
            .trim()
            .parse()
            .unwrap_or(0.0);
        let next = current + delta;
        // Instance-write redirect (GAP #10) — see property_dispatch
        // for the choke-point note.
        let instance_scope = self.editor_state.begin_instance_write_for_anchor();
        let _ = self.editor_state.commit_property_edit(focus, next);
        if let Some(scope) = instance_scope {
            self.editor_state.finish_instance_write(scope);
        }
        // Reflect the committed value back into the draft so the
        // field shows it and a further step builds on the new value.
        let next_text = if next.fract() == 0.0 {
            format!("{}", next as i64)
        } else {
            format!("{next}")
        };
        self.editor_state
            .ui
            .property_input
            .set_text(next_text.clone());
        self.editor_state.ui.property_input.touch(self.now_ms);
        self.editor_state.ui.property_input_draft = next_text;
        self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
        self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
        self.mark_dirty();
        true
    }

    /// Left / Right arrow during an inline rename — moves the rename
    /// caret one character. Returns `false` when no rename is active,
    /// so the caller falls back to the property caret / node-nudge.
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

    /// Left / Right arrow on a focused property input — moves the
    /// text caret one character. Returns `false` when no property
    /// input is focused, so the caller falls back to node-nudge.
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
            // #20: preset-name input shares the rename draft.
            && !self.editor_state.editor_ui.preset_name_input_active()
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
            self.editor_state.ui.property_input_draft =
                self.editor_state.ui.property_input.text().to_owned();
            self.editor_state.ui.property_caret_pos = self.editor_state.ui.property_input.caret();
            self.editor_state.ui.property_draft_select_all = false;
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
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
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
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
            return true;
        }
        let draft = &self.editor_state.ui.property_input_draft;
        let pos = text_boundary_at_or_before(draft, self.editor_state.ui.property_caret_pos);
        self.editor_state.ui.property_draft_select_all = false;
        let next = if forward {
            next_text_boundary(draft, pos)
        } else {
            previous_text_boundary(draft, pos)
        };
        if next != self.editor_state.ui.property_caret_pos {
            self.editor_state.ui.property_caret_pos = next;
            self.editor_state.ui.property_caret_anchor_ms = self.now_ms;
            self.mark_dirty();
        }
        // Consumed regardless — an arrow over a focused input must
        // never fall through to nudging the selected node.
        true
    }

    /// Arrow-key nudge — translate selection by (dx, dy) doc px.
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

    pub fn apply_send(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            self.commit_settings_focus_if_any();
            return true;
        }
        // Image-search popover: Enter submits the query; the
        // generate popover swallows Enter (TS textarea).
        if self.apply_image_panel_send() {
            return true;
        }
        // Font-family picker: swallow Enter so it can't leak into
        // chat send / property commit while the overlay is open.
        if self.editor_state.editor_ui.font_picker.open {
            return true;
        }
        // Enter is owned by the clone wizard whenever it is open: a
        // focused field (not mid-clone) requests the clone; otherwise the
        // key is simply swallowed so it can't fall through to chat send
        // or any other action.
        if self.git_clone_input_active() {
            let submit = self
                .editor_state
                .editor_ui
                .git_panel
                .clone_form
                .as_ref()
                .is_some_and(|f| f.focus.is_some() && !f.cloning);
            if submit {
                self.editor_state.editor_ui.git_panel.pending_action =
                    Some(op_editor_core::GitPanelAction::SubmitClone);
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the Git commit input requests a commit — needs a
        // message and a staged file (the commit is the staged set).
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.commit_input.text().trim().is_empty()
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
            if !panel.remote_input.text().trim().is_empty() {
                panel.pending_action = Some(op_editor_core::GitPanelAction::SetRemote(
                    panel.remote_input.text().to_owned(),
                ));
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the Git HTTPS-credential input stores it.
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.https_input.text().trim().is_empty() {
                panel.pending_action = Some(op_editor_core::GitPanelAction::SetHttpsAuth(
                    panel.https_input.text().to_owned(),
                ));
            }
            self.mark_dirty();
            return true;
        }
        if self.git_branch_create_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            let name = panel.branch_create_input.text().trim().to_string();
            if !name.is_empty() {
                panel.pending_action = Some(op_editor_core::GitPanelAction::CreateBranch(name));
                panel.branch_picker_mode = op_editor_core::GitBranchPickerMode::List;
                panel.branch_create_input.set_text("");
                panel.branch_create_focused = false;
                panel.branch_picker_open = false;
            }
            self.mark_dirty();
            return true;
        }
        // Enter in the commit-signature form submits it when valid; swallowed
        // either way so it never falls through to the global chat send.
        if self.git_author_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.author_name_input.text().trim().is_empty()
                && panel.author_email_input.text().contains('@')
            {
                panel.pending_action = Some(op_editor_core::GitPanelAction::SaveAuthor);
            }
            self.mark_dirty();
            return true;
        }
        // While a ready-state popover (branch picker / overflow menu) is
        // actually visible with no focused input, swallow Enter so it can't
        // fall through to the global chat send below. (Focused inputs already
        // submitted above; the helper requires the ready view so a stale flag
        // on a closed / merging / diff panel can't eat global Enter.)
        if self.git_ready_popover_open() {
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
            // Enter INSERTS a newline (TS textarea parity) — only
            // Escape / outside click commit the session. Swallow the
            // key either way so it never falls through to chat send.
            if self.editor_state.text_edit_insert("\n", self.now_ms) {
                self.mark_dirty();
            }
            return true;
        }
        if let Some(ok) = self.apply_pen_enter() {
            return ok;
        }
        // #20: Enter in the preset-name input saves the preset
        // (variable-theme-manager.tsx:298).
        if self.commit_variables_preset_name_if_any() {
            return true;
        }
        // Enter in the variables search box just blurs it (the filter
        // is already live).
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
            self.clear_settings_caret();
            self.mark_dirty();
            return true;
        }
        // #20: Escape closes the preset-name input only — the
        // preset dropdown stays open (variable-theme-manager.tsx:299).
        if self.escape_variables_preset_name() {
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
        // Anchor-menu close, then pen CANCEL (TS Escape discards).
        if self.apply_pen_escape() {
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
            self.editor_state.editor_ui.variable_row_input.set_text("");
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
            self.editor_state.ui.property_input.set_text("");
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.property_focus.take().is_some() {
            self.editor_state.ui.property_input_draft.clear();
            self.editor_state.ui.property_draft_select_all = false;
            self.editor_state.ui.property_input.set_text("");
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
        if self.editor_state.editor_ui.icon_picker_open {
            self.editor_state.editor_ui.icon_picker_open = false;
            self.editor_state.editor_ui.icon_picker_replace_selection = false;
            self.editor_state.editor_ui.icon_picker_search.clear();
            self.editor_state.editor_ui.icon_picker_select_all = false;
            self.editor_state.editor_ui.icon_picker_hover = None;
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.chat_model_picker.open {
            self.editor_state.editor_ui.close_chat_model_picker();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.component_browser_open {
            // One layer per press: an open kit-filter popover closes
            // before the panel itself does.
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
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            self.editor_state.editor_ui.image_panel.close_popovers();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.font_picker.open {
            self.close_font_picker();
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
