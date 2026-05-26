//! Clipboard / undo / reorder keyboard handlers, split out of
//! `input.rs` to stay under the 800-line cap.

use super::WidgetHostNative;
use op_editor_core::ReorderDirection;

impl WidgetHostNative {
    /// Cmd-C — copy selection to clipboard.
    pub fn apply_copy(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        // Clipboard is transient editor state — no paint change.
        self.editor_state.copy_selected()
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
        let ok = self.editor_state.cut_selected();
        if ok {
            self.mark_dirty();
        }
        ok
    }

    /// Cmd-V — paste clipboard at +10 doc px; select clones.
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

    /// Cmd-A — select every top-level node on the active page.
    pub fn apply_select_all(&mut self) -> bool {
        if self.input_active() {
            return false;
        }
        let ok = self.editor_state.select_all_top_level();
        if ok {
            self.mark_dirty();
        }
        ok
    }

    /// Cmd-Z — undo the last transactional change. Allowed during
    /// text-edit so the user can roll back a typing burst without
    /// exiting edit mode (each pause-bounded burst is its own
    /// history entry).
    pub fn apply_undo(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        let ok = self.editor_state.undo();
        if ok {
            self.mark_dirty();
        }
        ok
    }

    pub fn apply_redo(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        let ok = self.editor_state.redo();
        if ok {
            self.mark_dirty();
        }
        ok
    }

    /// Cmd+G — wrap the current selection in a new Group node.
    pub fn apply_group(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self
            .editor_state
            .group_selected(&mut self.next_node_id)
            .is_some()
        {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd+Shift+G — unwrap a Group selection (children replace it).
    pub fn apply_ungroup(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        let snap = self.editor_state.snapshot_for_history();
        if self.editor_state.ungroup_selected() {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd+J — focus / defocus the AI chat input.
    pub fn apply_toggle_chat(&mut self) -> bool {
        // Codex stop-gate: shifting focus to chat must commit
        // any pending variable-row edit first so subsequent
        // keystrokes don't keep routing into the variable draft.
        self.commit_variable_row_focus_if_any();
        self.editor_state.chat.focused = !self.editor_state.chat.focused;
        if self.editor_state.chat.focused {
            self.editor_state.chat.caret_anchor_ms = self.now_ms;
        }
        self.mark_dirty();
        true
    }

    /// Cmd+Shift+C — toggle the PropertyPanel between Design / Code.
    pub fn apply_toggle_code_panel(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        use op_editor_core::PropertyTab;
        self.editor_state.editor_ui.property_tab = match self.editor_state.editor_ui.property_tab {
            PropertyTab::Design => PropertyTab::Code,
            PropertyTab::Code => PropertyTab::Design,
        };
        self.mark_dirty();
        true
    }

    /// Cmd+Shift+V / toolbar `{}` — open / close the Variables panel.
    pub fn apply_toggle_variables_panel(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        let ui = &mut self.editor_state.editor_ui;
        ui.variables_panel_open = !ui.variables_panel_open;
        self.mark_dirty();
        true
    }

    /// Cmd+Shift+D / toolbar book — open / close the Design.md panel.
    pub fn apply_toggle_design_md_panel(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        let ui = &mut self.editor_state.editor_ui;
        ui.design_md_panel_open = !ui.design_md_panel_open;
        self.mark_dirty();
        true
    }

    /// Cmd+, — open / close the floating agent-settings modal.
    pub fn apply_toggle_agent_settings(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        self.editor_state.editor_ui.agent_settings_open =
            !self.editor_state.editor_ui.agent_settings_open;
        if !self.editor_state.editor_ui.agent_settings_open {
            self.editor_state.editor_ui.agent_settings_drag = None;
        }
        self.mark_dirty();
        true
    }

    /// Single-key tool switch (V / R / O / L / T / F / P / H). Also
    /// commits any in-flight pen path so switching away from Pen
    /// doesn't leave a dangling rubber-band.
    pub fn apply_set_tool(&mut self, tool: op_editor_core::Tool) {
        self.commit_variable_row_focus_if_any();
        let _ = self.editor_state.finish_pen_path();
        let ec_tool = tool;
        self.editor_state.tool = ec_tool;
        if let op_editor_core::Tool::Rect
        | op_editor_core::Tool::Ellipse
        | op_editor_core::Tool::Polygon
        | op_editor_core::Tool::Line
        | op_editor_core::Tool::Pen = tool
        {
            self.editor_state.editor_ui.shape_tool = ec_tool;
        }
        self.mark_dirty();
    }

    /// Public proxy for the keyboard router so it can gate single-
    /// letter shortcuts on whether any text-input surface owns the
    /// keyboard.
    pub fn input_active_pub(&self) -> bool {
        self.input_active()
    }

    /// Public proxy for the variable-row inline editor commit.
    /// External call sites in the desktop binary (Cmd+S / Cmd+O /
    /// Cmd+Shift+S / Cmd+Shift+P) call this before persistence /
    /// export actions so the typed value lands before the file
    /// op runs.
    pub fn commit_variable_row_focus_if_any_pub(&mut self) {
        self.commit_variables_panel_header_focus_if_any();
        self.commit_variable_row_focus_if_any();
    }

    /// Public proxy: commit every pending text-input draft — a
    /// half-typed property-panel field or variable-row value — into
    /// the document. The desktop binary calls this before a
    /// reload-style git op (pull / branch switch / merge) so an
    /// in-progress edit is captured by dirty-detection instead of
    /// being silently dropped by the reload. `commit_property_focus_if_any`
    /// already chains the variable-row commit.
    pub fn commit_pending_input_pub(&mut self) {
        self.commit_property_focus_if_any();
    }

    /// `[` / `]` — bump selection up/down within parent siblings.
    pub fn apply_reorder(&mut self, direction: ReorderDirection) -> bool {
        if self.input_active() {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        self.editor_state.commit_history();
        // Translate the shell-core reorder direction (kept as the
        // public API type) into the op-editor-core equivalent.
        let ec_dir = match direction {
            ReorderDirection::Up => op_editor_core::walkers::ReorderDirection::Up,
            ReorderDirection::Down => op_editor_core::walkers::ReorderDirection::Down,
        };
        let ok = self.editor_state.reorder_selected(ec_dir);
        if ok {
            self.mark_dirty();
        }
        ok
    }
}
