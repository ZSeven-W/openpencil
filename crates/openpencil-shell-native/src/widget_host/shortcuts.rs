//! Clipboard / undo / reorder keyboard handlers, split out of
//! `input.rs` to stay under the 800-line cap.

use super::WidgetHostNative;
use openpencil_shell_core::document::ReorderDirection;

impl WidgetHostNative {
    /// Cmd-C — copy selection to clipboard.
    pub fn apply_copy(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        self.document.copy_selected()
    }

    /// Cmd/Ctrl+X — copy then delete the selection.
    pub fn apply_cut(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.commit_history();
        self.document.cut_selected()
    }

    /// Cmd-V — paste clipboard at +10 doc px; select clones.
    pub fn apply_paste(&mut self) -> bool {
        if self.input_active() {
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

    /// Cmd-A — select every top-level node on the active page.
    pub fn apply_select_all(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        self.document.select_all_top_level()
    }

    /// Cmd-Z — undo the last transactional change. Allowed during
    /// text-edit so the user can roll back a typing burst without
    /// exiting edit mode (each pause-bounded burst is its own
    /// history entry).
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

    /// Cmd+G — wrap the current selection in a new Group node.
    pub fn apply_group(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some() || self.document.chat.focused {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        let snap = self.document.snapshot_for_history();
        if self
            .document
            .group_selected(&mut self.next_node_id)
            .is_some()
        {
            self.document.history_push_past(snap);
            return true;
        }
        false
    }

    /// Cmd+Shift+G — unwrap a Group selection (children replace it).
    pub fn apply_ungroup(&mut self) -> bool {
        if self.document.ui.layer_rename.is_some() || self.document.chat.focused {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        let snap = self.document.snapshot_for_history();
        if self.document.ungroup_selected() {
            self.document.history_push_past(snap);
            return true;
        }
        false
    }

    /// Cmd+J — focus / defocus the AI chat input.
    pub fn apply_toggle_chat(&mut self) -> bool {
        self.document.chat.focused = !self.document.chat.focused;
        if self.document.chat.focused {
            self.document.chat.caret_anchor_ms = self.now_ms;
        }
        true
    }

    /// Cmd+Shift+C — toggle the PropertyPanel between Design / Code.
    pub fn apply_toggle_code_panel(&mut self) -> bool {
        use openpencil_shell_core::document::PropertyTab;
        self.document.ui.property_tab = match self.document.ui.property_tab {
            PropertyTab::Design => PropertyTab::Code,
            PropertyTab::Code => PropertyTab::Design,
        };
        true
    }

    /// Cmd+, — open / close the floating agent-settings modal.
    pub fn apply_toggle_agent_settings(&mut self) -> bool {
        self.document.ui.agent_settings_open = !self.document.ui.agent_settings_open;
        if !self.document.ui.agent_settings_open {
            self.document.ui.agent_settings_drag = None;
        }
        true
    }

    /// Single-key tool switch (V / R / O / L / T / F / P / H). Also
    /// commits any in-flight pen path so switching away from Pen
    /// doesn't leave a dangling rubber-band.
    pub fn apply_set_tool(&mut self, tool: openpencil_shell_core::document::Tool) {
        let _ = self.document.finish_pen_path();
        self.document.tool = tool;
        if let openpencil_shell_core::document::Tool::Rect
        | openpencil_shell_core::document::Tool::Ellipse
        | openpencil_shell_core::document::Tool::Polygon
        | openpencil_shell_core::document::Tool::Line
        | openpencil_shell_core::document::Tool::Pen = tool
        {
            self.document.ui.shape_tool = tool;
        }
    }

    /// Public proxy for the keyboard router so it can gate single-
    /// letter shortcuts on whether any text-input surface owns the
    /// keyboard.
    pub fn input_active_pub(&self) -> bool {
        self.input_active()
    }

    /// `[` / `]` — bump selection up/down within parent siblings.
    pub fn apply_reorder(&mut self, direction: ReorderDirection) -> bool {
        if self.input_active() {
            return false;
        }
        if !self.document.selected.is_real() {
            return false;
        }
        self.document.commit_history();
        self.document.reorder_selected(direction)
    }
}
