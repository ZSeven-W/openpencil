//! Clipboard / undo / reorder keyboard handlers, split out of
//! `input.rs` to stay under the 800-line cap.

use super::WidgetHostNative;
use op_editor_core::ReorderDirection;

impl WidgetHostNative {
    /// Cmd-C — copy selection to clipboard.
    pub fn apply_copy(&mut self) -> bool {
        if self.editor_state.chat.focused {
            if let Some(text) = self
                .editor_state
                .chat
                .selected_input_text()
                .map(str::to_string)
            {
                self.editor_state.chat.queue_copy_text(text);
                return true;
            }
            return false;
        }
        if self.input_active() {
            return false;
        }
        if let Some(text) = self.editor_state.codegen.selected_code_text() {
            self.editor_state.chat.queue_copy_text(text.to_string());
            return true;
        }
        if let Some(text) = self
            .editor_state
            .chat
            .selected_transcript_text()
            .map(str::to_string)
        {
            self.editor_state.chat.queue_copy_text(text);
            return true;
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
        if self.apply_input_select_all() {
            return true;
        }
        let ok = self.editor_state.select_all_top_level();
        if ok {
            self.mark_dirty();
        }
        ok
    }

    fn apply_input_select_all(&mut self) -> bool {
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            let ui = &mut self.editor_state.editor_ui;
            ui.settings_input.select_all();
            ui.settings_input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_clone_input_active() {
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                match form.focus {
                    Some(op_editor_core::CloneField::Url) => {
                        form.url_input.select_all();
                        form.url_input.touch(self.now_ms);
                        self.mark_dirty();
                        return true;
                    }
                    Some(op_editor_core::CloneField::Dest) => {
                        form.dest_input.select_all();
                        form.dest_input.touch(self.now_ms);
                        self.mark_dirty();
                        return true;
                    }
                    None => {}
                }
            }
            return false;
        }
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.commit_input.select_all();
            panel.commit_input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.remote_input.select_all();
            panel.remote_input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.https_input.select_all();
            panel.https_input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_branch_create_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.branch_create_input.select_all();
            panel.branch_create_input.touch(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.git_author_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if panel.author_email_focused {
                panel.author_email_input.select_all();
                panel.author_email_input.touch(self.now_ms);
            } else {
                panel.author_name_input.select_all();
                panel.author_name_input.touch(self.now_ms);
            }
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
        if variable_header_focus
            || self.editor_state.editor_ui.variable_row_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
            || self.editor_state.ui.property_focus.is_some()
        {
            if self.editor_state.ui.property_focus.is_some()
                || self.editor_state.editor_ui.effect_param_focus.is_some()
            {
                let input = &mut self.editor_state.ui.property_input;
                input.select_all();
                input.touch(self.now_ms);
                let ui = &mut self.editor_state.ui;
                ui.property_input_draft = ui.property_input.text().to_owned();
                ui.property_caret_pos = ui.property_input.caret();
                ui.property_draft_select_all = true;
                ui.property_caret_anchor_ms = self.now_ms;
            } else if variable_header_focus {
                let input = &mut self.editor_state.editor_ui.variables_header_input;
                input.select_all();
                input.touch(self.now_ms);
                self.sync_variables_header_input_legacy(true);
            } else {
                let input = &mut self.editor_state.editor_ui.variable_row_input;
                input.select_all();
                input.touch(self.now_ms);
                self.sync_variable_row_input_legacy(true);
            }
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.icon_picker.open {
            self.editor_state.editor_ui.icon_picker_select_all = true;
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
        if self.editor_state.editor_ui.component_browser_open {
            self.editor_state.editor_ui.component_browser_select_all = true;
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
        if self.editor_state.chat.focused {
            self.editor_state.chat.blur_input(self.now_ms);
        } else {
            self.editor_state.chat.focus_input_at_end(self.now_ms);
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
            PropertyTab::Interact => PropertyTab::Code,
            PropertyTab::Code => PropertyTab::Design,
        };
        self.mark_dirty();
        true
    }

    /// Cmd+Shift+V — toggle the floating Variables panel (same
    /// bookkeeping as the toolbar Braces button).
    pub fn apply_toggle_variables_panel(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        self.dispatch_toolbar_action(op_editor_ui::widgets::ToolbarAction::ToggleVariablesPanel)
    }

    /// Cmd+Shift+D — toggle the floating Design-MD panel.
    pub fn apply_toggle_design_md_panel(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        self.dispatch_toolbar_action(op_editor_ui::widgets::ToolbarAction::ToggleDesignPanel)
    }

    /// Cmd+Shift+K — toggle the component (UIKit) browser panel.
    pub fn apply_toggle_component_browser(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        let ui = &mut self.editor_state.editor_ui;
        ui.component_browser_open = !ui.component_browser_open;
        self.mark_dirty();
        true
    }

    /// Cmd+Shift+F — open the Figma import modal.
    pub fn apply_open_figma_import(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        self.editor_state.editor_ui.figma_import_open = true;
        self.mark_dirty();
        true
    }

    /// Cmd+Alt+K — promote the selected node to a component (same
    /// path as the property-panel Create Component button).
    pub fn apply_create_component(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        let id = self.editor_state.selection.anchor.clone();
        let acted = id.is_real() && self.editor_state.create_component_from_node_name(&id);
        if acted {
            self.mark_dirty();
        }
        acted
    }

    /// Space held / released — transient canvas pan (TS space-drag
    /// parity): while held, a canvas press pans regardless of tool.
    pub fn set_space_pan(&mut self, held: bool) {
        self.space_pan = held;
    }

    /// Cmd+, — open / close the floating agent-settings modal.
    pub fn apply_toggle_agent_settings(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        self.editor_state.editor_ui.agent_settings_open =
            !self.editor_state.editor_ui.agent_settings_open;
        if self.editor_state.editor_ui.agent_settings_open {
            self.editor_state.chat.blur_input(self.now_ms);
        } else {
            self.editor_state.editor_ui.agent_settings_drag = None;
        }
        self.mark_dirty();
        true
    }

    /// Single-key tool switch (V / R / O / L / T / F / P / H). Also
    /// DISCARDS any in-flight pen path (TS `onToolChange` resets the
    /// pen preview without committing, `skia-pen-tool.ts:38-50`).
    pub fn apply_set_tool(&mut self, tool: op_editor_core::Tool) {
        // Leaving Select must drop the hover outline immediately —
        // cursor moves stop updating it for other tools.
        self.editor_state.editor_ui.canvas_hover_node = None;
        self.commit_variable_row_focus_if_any();
        self.cancel_pen_on_tool_switch(tool);
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
