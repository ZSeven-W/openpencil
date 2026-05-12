//! Keyboard / clipboard handlers on the web `WidgetHost`.
//! Pulled out of `widget_host.rs` so the spine file stays
//! under the 800-line ceiling. Mirrors the native shell's
//! `widget_host/input.rs` shape.

use super::WidgetHost;

impl WidgetHost {
    /// Step 5 P2: push a typed character into the focused chat
    /// input. Returns true if anything changed.
    pub fn apply_text(&mut self, c: char) -> bool {
        if self.document.ui.layer_rename.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            return self.document.rename_append(c.encode_utf8(&mut s));
        }
        if self.document.ui.text_editing.is_some() && !c.is_control() {
            let mut s = [0u8; 4];
            return self
                .document
                .text_edit_append(c.encode_utf8(&mut s), self.now_ms);
        }
        if !self.document.chat.focused {
            return false;
        }
        self.document.chat.input.push(c);
        true
    }

    pub fn apply_backspace(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some() {
            return self.document.rename_backspace();
        }
        if self.document.ui.text_editing.is_some() {
            return self.document.text_edit_backspace(self.now_ms);
        }
        if self.document.chat.focused {
            return self.document.chat.input.pop().is_some();
        }
        if !self.document.selected.is_real() {
            return false;
        }
        let snap = self.document.snapshot_for_history();
        if self.document.delete_selected() {
            self.document.history_push_past(snap);
            return true;
        }
        false
    }

    pub fn apply_send(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some() {
            return self.document.rename_commit();
        }
        if self.document.ui.text_editing.is_some() {
            return self.document.text_edit_commit();
        }
        if self.document.chat.input.trim().is_empty() {
            return false;
        }
        self.document.chat.send();
        true
    }

    /// Delete key — selected-node delete; never touches text
    /// drafts. Use this when the host wants Delete strictly
    /// destructive regardless of focus.
    pub fn apply_delete(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some() {
            return self.document.rename_backspace();
        }
        if self.document.ui.text_editing.is_some() {
            return self.document.text_edit_backspace(self.now_ms);
        }
        if self.document.chat.focused {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        let snap = self.document.snapshot_for_history();
        if self.document.delete_selected() {
            self.document.history_push_past(snap);
            return true;
        }
        false
    }

    /// Cmd/Ctrl+D — duplicate the selected node as a sibling
    /// offset by ~10 doc px. Selection follows the clone.
    pub fn apply_duplicate(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some()
            || self.document.ui.text_editing.is_some()
            || self.document.chat.focused
        {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.commit_history();
        self.document
            .duplicate_selected(&mut self.next_node_id, 10.0)
            .is_some()
    }

    /// Arrow-key nudge — translate the selected node by
    /// `(dx, dy)` document px. Shift-arrow callers pass 10 px;
    /// plain arrows pass 1 px.
    pub fn apply_nudge(&mut self, dx: f32, dy: f32) -> bool {
        if self.document.ui.layer_rename.is_some()
            || self.document.ui.text_editing.is_some()
            || self.document.chat.focused
        {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.commit_history();
        self.document.translate_selected(dx, dy);
        true
    }

    /// Cmd/Ctrl+A — replace selection with every top-level node
    /// on the active page (TS `setSelection(topLevelIds, …)`).
    pub fn apply_select_all(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some()
            || self.document.ui.text_editing.is_some()
            || self.document.chat.focused
        {
            return false;
        }
        self.document.select_all_top_level()
    }

    /// Cmd/Ctrl+C — copy the selection into the clipboard.
    pub fn apply_copy(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some()
            || self.document.ui.text_editing.is_some()
            || self.document.chat.focused
        {
            return false;
        }
        self.document.copy_selected()
    }

    /// Cmd/Ctrl+X — copy then delete the selection.
    pub fn apply_cut(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some()
            || self.document.ui.text_editing.is_some()
            || self.document.chat.focused
        {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.commit_history();
        self.document.cut_selected()
    }

    /// Cmd/Ctrl+V — paste the clipboard at the active page,
    /// offset by 10 doc px from the originals. Selection follows
    /// the new clones.
    pub fn apply_paste(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some()
            || self.document.ui.text_editing.is_some()
            || self.document.chat.focused
        {
            return false;
        }
        if self.document.clipboard.is_empty() {
            return false;
        }
        self.document.commit_history();
        !self
            .document
            .paste_clipboard(&mut self.next_node_id, 10.0)
            .is_empty()
    }

    pub fn apply_undo(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some() || self.document.chat.focused {
            return false;
        }
        self.document.undo()
    }

    pub fn apply_redo(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some() || self.document.chat.focused {
            return false;
        }
        self.document.redo()
    }

    /// `[` / `]` — bump the selected node down / up by one
    /// position in its parent's children vec (changing paint
    /// order).
    pub fn apply_reorder(
        &mut self,
        direction: openpencil_shell_core::document::ReorderDirection,
    ) -> bool {
        if self.document.ui.layer_rename.is_some()
            || self.document.ui.text_editing.is_some()
            || self.document.chat.focused
        {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.commit_history();
        self.document.reorder_selected(direction)
    }

    /// Escape — handles one layer per press, matching the
    /// native host's priority order. The property-focus tier is
    /// listed first for parity even though web hasn't wired
    /// property input editing yet — so when web grows that path,
    /// no Escape-priority reordering is needed.
    pub fn apply_escape(&mut self) -> bool {
        if self.document.rename_cancel() {
            return true;
        }
        if self.document.text_edit_commit() {
            return true;
        }
        if self.document.ui.property_focus.take().is_some() {
            self.document.ui.property_input_draft.clear();
            return true;
        }
        if self.document.ui.locale_picker_open {
            self.document.ui.locale_picker_open = false;
            return true;
        }
        if self.document.ui.shape_picker_open {
            self.document.ui.shape_picker_open = false;
            return true;
        }
        if self.document.ui.fill_type_picker_open {
            self.document.ui.fill_type_picker_open = false;
            return true;
        }
        if self.document.chat.focused {
            self.document.chat.focused = false;
            return true;
        }
        if self.document.selected.is_real() {
            self.document.deselect_all();
            return true;
        }
        false
    }

    /// Phase C2 IME forwarding stub — Step 5+ wires per-widget focus.
    pub fn apply_ime(&mut self, _event: &openpencil_shell_core::ImeEvent) { // glue:
    }

    /// Phase C2 keyboard forwarding stub.
    pub fn apply_key(&mut self, _event: &openpencil_shell_core::KeyEvent) { // glue:
    }
}
