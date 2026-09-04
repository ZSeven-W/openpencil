//! Copy / cut / paste against the focused chrome text input on
//! `WidgetHostNative`.
//!
//! Split out of `keyboard.rs` to keep every file under the repo's
//! 800-line cap.

use super::WidgetHostNative;

impl WidgetHostNative {
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
    /// message, the remote / HTTPS draft, or a settings field. The built-in
    /// Model list preserves normalized newlines; every other settings field
    /// remains single-line. Other inputs route each character through
    /// [`Self::apply_text`], so their per-input filtering (e.g. the clone
    /// field's `!cloning` lock) still applies. Returns `true` if anything was
    /// inserted.
    pub fn apply_input_paste(&mut self, text: &str) -> bool {
        // The save-name dialog is modal and filters characters a file name
        // cannot carry, so it takes the paste as a unit.
        if let Some(changed) =
            op_editor_core::save_name_keyboard::paste(&mut self.editor_state, text, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        // The Asset Center takes a paste as a unit: its style-import box
        // receives a whole DESIGN.md, and the char-by-char route below drops
        // control characters, which would flatten the markdown to one line.
        if let Some(changed) = op_editor_core::host_keyboard_transitions::scene_template_paste(
            &mut self.editor_state,
            text,
            self.now_ms,
        ) {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        // The join field takes a pasted invite code as a whole-field
        // replacement: char-by-char append silently concatenated a new code
        // onto a stale one, producing an invalid join target.
        if self.editor_state.editor_ui.collab_join_input_active() {
            let changed = op_editor_ui::widgets::collab_ui::join_address_paste(
                &mut self.editor_state.editor_ui,
                text,
                self.now_ms,
            )
            .unwrap_or(false);
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_text_payload(text);
        }
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

    /// Highlighted slice of whichever `TextInputState`-backed input
    /// currently owns the keyboard — settings / git (commit, remote,
    /// HTTPS, branch, author, clone) / rename / property / variables /
    /// model-picker / canvas text editor. `None` when no such input is
    /// focused or it has no selection. The desktop host writes the
    /// returned slice to the OS clipboard on Cmd+C; chat-input copy is
    /// handled separately (its own whole-buffer path). Routes through
    /// the shared `EditorState::active_text_input` resolver so every
    /// focused field is covered with one priority order.
    pub fn input_copy_text(&self) -> Option<String> {
        let state = self.editor_state.active_text_input()?;
        let (start, end) = state.highlight_range()?;
        Some(state.text().get(start..end)?.to_string())
    }

    /// Cut the highlighted slice of the focused `TextInputState` input:
    /// returns the slice and deletes it. `None` when no such input is
    /// focused or it has no selection. The delete reuses
    /// [`Self::apply_backspace`] so it follows each input's own backspace
    /// routing (per-input dirty / hint bookkeeping included); with a live
    /// selection `backspace` removes the whole highlighted range
    /// (`TextInputState::consume_pending`). Backs Cmd+X for every editor
    /// text field except the chat input (`chat_input_cut`).
    pub fn input_cut_text(&mut self) -> Option<String> {
        let text = self.input_copy_text()?;
        if text.is_empty() {
            return None;
        }
        self.apply_backspace();
        Some(text)
    }
}
