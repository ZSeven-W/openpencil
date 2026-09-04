//! Clipboard / undo / reorder keyboard handlers, split out of
//! `input.rs` to stay under the 800-line cap.

use super::WidgetHostNative;
use op_editor_core::host_keyboard_transitions as shared;
use op_editor_core::{figma_import_state::ImportSource, AgentSettingsTab, ReorderDirection};

impl WidgetHostNative {
    /// True when a non-chat text surface owns keyboard input. Plain-string
    /// host inputs are checked explicitly; `TextInputState` owners share the
    /// canonical editor-core resolver.
    pub fn non_chat_input_owns_keyboard_pub(&self) -> bool {
        let editor_ui = &self.editor_state.editor_ui;
        if self.preview.is_some()
            || editor_ui.collab_join_input_active()
            || editor_ui.font_picker.open
            || editor_ui.image_panel.search_open
            || editor_ui.image_panel.generate_open
            || self.variables_search_active()
            || editor_ui.preset_name_input_active()
            || editor_ui.icon_picker.open
            || editor_ui.prompt_center.open
            || editor_ui.scene_template_center.input_active()
            || editor_ui.component_browser_open
            // `active_text_input()` resolves chat before Git. Visible Git /
            // clone inputs must therefore claim ownership before the pointer
            // comparison below when a stale chat-focus bit coexists.
            || self.git_commit_focus_active()
            || self.git_remote_focus_active()
            || self.git_https_focus_active()
            || self.git_branch_create_focus_active()
            || self.git_author_focus_active()
            || self.git_clone_input_active()
        {
            return true;
        }
        if self.editor_state.chat.focused {
            self.editor_state
                .active_text_input()
                .is_some_and(|active| !std::ptr::eq(active, &self.editor_state.chat.input))
        } else {
            // The host predicate applies the visibility gates for Git /
            // clone inputs; the plain-string omissions are covered above.
            self.input_active()
        }
    }

    /// True only when the chat input, rather than a higher-priority
    /// non-chat surface, owns keyboard input.
    pub fn chat_input_owns_keyboard_pub(&self) -> bool {
        self.editor_state.chat.focused && !self.non_chat_input_owns_keyboard_pub()
    }

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
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ClipboardPaste,
            ),
        ) {
            return true;
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
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ClipboardPaste,
            ),
        ) {
            return true;
        }
        let result = if let Some(allocator) = self.collab_id_allocator.as_mut() {
            if self.editor_state.clipboard.is_empty() {
                Ok(Vec::new())
            } else {
                let snapshot = self.editor_state.snapshot_for_history();
                let result = self
                    .editor_state
                    .paste_clipboard_with_allocator(allocator, 10.0);
                if result.as_ref().is_ok_and(|ids| !ids.is_empty()) {
                    self.editor_state.history_push_past(snapshot);
                }
                result
            }
        } else {
            let pasted = shared::paste_clipboard_at_default_offset(
                &mut self.editor_state,
                &mut self.next_node_id,
            );
            Ok(if pasted {
                vec![self.editor_state.selection.anchor.clone()]
            } else {
                Vec::new()
            })
        };
        let pasted = match result {
            Ok(ids) => !ids.is_empty(),
            Err(error) => {
                self.show_collab_id_error(error);
                return true;
            }
        };
        if pasted {
            self.mark_dirty();
        }
        pasted
    }

    /// Cmd-A — select every top-level node on the active page.
    pub fn apply_select_all(&mut self) -> bool {
        // The font picker owns the keyboard while its search field is open.
        // It has no range-selection model, so consume Cmd/Ctrl+A instead of
        // selecting canvas nodes behind the overlay.
        if self.editor_state.editor_ui.font_picker.open {
            return true;
        }
        if self.apply_input_select_all() {
            return true;
        }
        // String-backed fields without a range-selection model still own the
        // chord; never select canvas nodes behind the collaboration panel.
        if self.input_active() {
            return true;
        }
        let ok = self.editor_state.select_all_top_level();
        if ok {
            self.mark_dirty();
        }
        ok
    }

    fn apply_input_select_all(&mut self) -> bool {
        if op_editor_core::save_name_keyboard::select_all(&mut self.editor_state, self.now_ms) {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.collab_join_input_active() {
            if op_editor_ui::widgets::collab_ui::join_address_select_all(
                &mut self.editor_state.editor_ui,
                self.now_ms,
            ) == Some(true)
            {
                self.mark_dirty();
            }
            return true;
        }
        if self.apply_image_panel_select_all() {
            return true;
        }
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
        // Rename → canvas text edit → property / effect-param →
        // variables header / row → icon picker → model picker →
        // component browser → chat input.
        if shared::select_all_focused_input(&mut self.editor_state, self.now_ms) {
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
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            return false;
        }
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if !self.collab_allows_user_action(op_editor_core::CollabGateAction::GlobalUndo) {
            return true;
        }
        let ok = self.editor_state.undo();
        if ok {
            self.mark_dirty();
            self.refresh_missing_fonts_after_history_change();
        }
        ok
    }

    pub fn apply_redo(&mut self) -> bool {
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            return false;
        }
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if !self.collab_allows_user_action(op_editor_core::CollabGateAction::GlobalRedo) {
            return true;
        }
        let ok = self.editor_state.redo();
        if ok {
            self.mark_dirty();
            self.refresh_missing_fonts_after_history_change();
        }
        ok
    }

    /// Cmd+G — wrap the current selection in a new Group node.
    pub fn apply_group(&mut self) -> bool {
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            return false;
        }
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::Group) {
            return true;
        }
        let snap = self.editor_state.snapshot_for_history();
        let result = if let Some(allocator) = self.collab_id_allocator.as_mut() {
            self.editor_state.group_selected_with_allocator(allocator)
        } else {
            Ok(self.editor_state.group_selected(&mut self.next_node_id))
        };
        let grouped = match result {
            Ok(id) => id.is_some(),
            Err(error) => {
                self.show_collab_id_error(error);
                return true;
            }
        };
        if grouped {
            self.editor_state.history_push_past(snap);
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cmd+Shift+G — unwrap a Group selection (children replace it).
    pub fn apply_ungroup(&mut self) -> bool {
        if self.editor_state.editor_ui.image_panel.search_open
            || self.editor_state.editor_ui.image_panel.generate_open
        {
            return false;
        }
        self.commit_variable_row_focus_if_any();
        if self.editor_state.ui.layer_rename.is_some() || self.editor_state.chat.focused {
            return false;
        }
        if self.editor_state.selection.is_empty() {
            return false;
        }
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::Ungroup) {
            return true;
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
        let ui = &mut self.editor_state.editor_ui;
        if !ui.code_property_tab_available() {
            if ui.set_property_tab(PropertyTab::Design) {
                self.mark_dirty();
            }
            return true;
        }
        let next = match ui.effective_property_tab() {
            PropertyTab::Design => PropertyTab::Code,
            PropertyTab::Interact => PropertyTab::Code,
            PropertyTab::Code => PropertyTab::Design,
        };
        if ui.set_property_tab(next) {
            self.mark_dirty();
        }
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
        self.close_image_popovers_for_higher_overlay();
        self.dispatch_toolbar_action(op_editor_ui::widgets::ToolbarAction::ToggleDesignPanel)
    }

    /// Cmd+Shift+K — toggle the component (UIKit) browser panel.
    pub fn apply_toggle_component_browser(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        self.close_image_popovers_for_higher_overlay();
        let ui = &mut self.editor_state.editor_ui;
        ui.component_browser_open = !ui.component_browser_open;
        self.mark_dirty();
        true
    }

    /// Cmd+Shift+F — open the Figma import modal.
    pub fn apply_open_figma_import(&mut self) -> bool {
        self.apply_open_import(ImportSource::Figma)
    }

    /// Cmd+Shift+H — open the HTML import modal.
    pub fn apply_open_html_import(&mut self) -> bool {
        self.apply_open_import(ImportSource::Html)
    }

    fn apply_open_import(&mut self, source: ImportSource) -> bool {
        // Keep the chord owned by the editor while an import is running, but
        // do not mutate the source/modal or disturb the focused input. An
        // existing higher modal must also keep ownership: opening the import
        // modal underneath it would look like a no-op, then surface later when
        // the original modal closes.
        if shared::import_modal_blocked_by_overlay(&self.editor_state.editor_ui)
            || self.git_commit_focus_active()
            || self.git_remote_focus_active()
            || self.git_https_focus_active()
            || self.git_author_focus_active()
            || self.git_branch_create_focus_active()
            || self.git_clone_input_active()
        {
            return true;
        }
        if !self.collab_allows_document_mutation_from(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::ExternalAssets,
            ),
            op_editor_core::CollabEditSource::Import,
        ) {
            return true;
        }

        // The import modal covers every editor text surface. Commit canvas /
        // layer editing, then reuse the canonical chrome-input blur path so a
        // hidden property, chat, or model-picker input cannot keep receiving
        // keyboard/IME events behind the scrim.
        let rename_mutation =
            self.editor_state
                .ui
                .layer_rename
                .as_ref()
                .map(|rename| match &rename.target {
                    op_editor_core::ui_draft::LayerContextTarget::Layer(_) => {
                        op_editor_core::CollabDocumentMutation::NodeProperty(
                            op_editor_core::CollabNodeField::Name,
                        )
                    }
                    op_editor_core::ui_draft::LayerContextTarget::Page(_) => {
                        op_editor_core::CollabDocumentMutation::Unsupported(
                            op_editor_core::CollabUnsupportedFeature::PageStructure,
                        )
                    }
                });
        if rename_mutation.is_some_and(|mutation| !self.collab_allows_document_mutation(mutation)) {
            let _ = self.editor_state.rename_cancel();
        }
        self.collab_blur_color_picker_inputs();
        shared::commit_editing_for_modal(&mut self.editor_state);
        self.blur_text_inputs_on_blank_press();
        self.close_image_popovers_for_higher_overlay();
        self.close_import_menu();
        shared::open_import_modal(&mut self.editor_state.editor_ui, source);
        self.mark_dirty();
        true
    }

    /// Cmd+Alt+K — promote the selected node to a component (same
    /// path as the property-panel Create Component button).
    pub fn apply_create_component(&mut self) -> bool {
        self.commit_variable_row_focus_if_any();
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::Components,
            ),
        ) {
            return true;
        }
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
        self.cancel_agent_settings_touch_gesture();
        self.commit_variable_row_focus_if_any();
        self.editor_state.editor_ui.blur_collab_join_input();
        let opening = !self.editor_state.editor_ui.agent_settings_open;
        if opening {
            // Settings covers the Property surface. Commit and release every
            // Property-owned input before the modal becomes visible so a
            // hidden field cannot keep receiving text / IME events.
            self.release_property_keyboard_owner();
            self.editor_state.editor_ui.agent_settings_open = true;
            // The settings modal is now topmost. Do not leave a property-panel
            // image input alive underneath it or text/IME would keep routing
            // to an invisible query field.
            self.close_image_popovers_for_higher_overlay();
            self.editor_state.chat.blur_input(self.now_ms);
        } else {
            // Cmd+, closes through the same commit/defocus path as the modal's
            // close button. Leaving either the form focus or the Fonts tab's
            // searchable picker alive after hiding the modal makes the input /
            // IME router treat an invisible field as owner.
            self.commit_settings_focus_if_any();
            let ui = &mut self.editor_state.editor_ui;
            if matches!(
                ui.font_picker_purpose,
                Some(op_editor_core::FontPickerPurpose::MissingFont {
                    surface: op_editor_core::MissingFontSurface::Settings,
                    ..
                })
            ) {
                ui.close_font_picker();
            }
            ui.agent_settings_open = false;
            ui.agent_settings_drag = None;
            ui.ime_preedit = None;
        }
        self.mark_dirty();
        true
    }

    /// Open the floating settings modal on a specific tab.
    ///
    /// Native menu commands use this instead of toggling so a command can
    /// reveal its live status even when Settings is already open elsewhere.
    pub fn apply_open_agent_settings_tab(&mut self, tab: AgentSettingsTab) -> bool {
        self.cancel_agent_settings_touch_gesture();
        self.commit_variable_row_focus_if_any();
        self.editor_state.editor_ui.blur_collab_join_input();
        self.release_property_keyboard_owner();
        self.commit_settings_focus_if_any();
        shared::commit_editing_for_modal(&mut self.editor_state);
        self.design_md_drag = None;
        self.component_browser_drag = None;
        self.icon_picker_drag = None;
        {
            let ui = &mut self.editor_state.editor_ui;
            ui.close_font_picker();
            ui.close_icon_picker();
            ui.design_md_panel.open = false;
            ui.design_md_panel.hover = None;
            ui.component_browser_open = false;
            ui.component_browser_kit_picker_open = false;
            ui.component_browser_confirm_delete_kit = None;
            ui.component_browser_hover = None;
            ui.layer_context_menu = None;
            ui.agent_settings_open = true;
            ui.agent_settings.tab = tab;
            ui.agent_settings.scroll_y.offset = 0.0;
            ui.agent_settings_drag = None;
            ui.ime_preedit = None;
        }
        self.editor_state.ui.path_anchor_menu = None;
        self.close_image_popovers_for_higher_overlay();
        self.editor_state.chat.blur_input(self.now_ms);
        self.mark_dirty();
        true
    }

    /// Single-key tool switch (V / R / O / L / T / F / P / H). Also
    /// DISCARDS any in-flight pen path (TS `onToolChange` resets the
    /// pen preview without committing, `skia-pen-tool.ts:38-50`).
    pub fn apply_set_tool(&mut self, tool: op_editor_core::Tool) {
        self.exit_image_crop_edit();
        self.commit_variable_row_focus_if_any();
        if !self.cancel_pen_on_tool_switch(tool) {
            return;
        }
        shared::set_active_tool(&mut self.editor_state, tool);
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
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeMove) {
            return true;
        }
        // Translate the shell-core reorder direction (kept as the
        // public API type) into the op-editor-core equivalent.
        let ec_dir = match direction {
            ReorderDirection::Up => op_editor_core::walkers::ReorderDirection::Up,
            ReorderDirection::Down => op_editor_core::walkers::ReorderDirection::Down,
        };
        let ok = shared::reorder_selection(&mut self.editor_state, ec_dir);
        if ok {
            self.mark_dirty();
        }
        ok
    }
}
