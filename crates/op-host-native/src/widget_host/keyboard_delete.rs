//! Backspace / Delete / Duplicate on `WidgetHostNative` — the
//! focused-input-first ladder that keeps a keystroke from deleting the
//! selection out from under a caret.
//!
//! Split out of `keyboard.rs` to keep every file under the repo's
//! 800-line cap.

use super::WidgetHostNative;
use op_editor_core::host_keyboard_transitions as shared;
use op_editor_core::host_preset_name_draft as preset_name;

impl WidgetHostNative {
    pub fn apply_backspace(&mut self) -> bool {
        // Save-name dialog first — same modal priority as `apply_text`.
        if let Some(changed) =
            op_editor_core::save_name_keyboard::backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if let Some(changed) = shared::prompt_center_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if let Some(changed) = shared::scene_template_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        // Preview mode: Backspace edits the focused runtime widget, not
        // the editor selection.
        if self.preview.is_some() {
            return self.preview_dispatch_key("Backspace", false);
        }
        if self.editor_state.editor_ui.collab_join_input_active() {
            let changed = op_editor_ui::widgets::collab_ui::join_address_backspace(
                &mut self.editor_state.editor_ui,
                self.now_ms,
            )
            .unwrap_or(false);
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if self.apply_image_panel_backspace() {
            return true;
        }
        if self.editor_state.color_picker_hex_focused() {
            if !self.collab_allows_color_picker_mutation() {
                return true;
            }
            self.editor_state.color_picker_hex_backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
        if self.editor_state.color_picker_rgb_focused() {
            if !self.collab_allows_color_picker_mutation() {
                return true;
            }
            self.editor_state.color_picker_rgb_backspace(self.now_ms);
            self.mark_dirty();
            return true;
        }
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
            // Editing the message (delete or cut, not just typing) clears
            // the stale "no changes to commit" hint, matching `apply_text`.
            panel.commit_no_changes = false;
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
        if let Some(changed) = shared::rename_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.ui.text_editing.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodeProperty(
                    op_editor_core::CollabNodeField::Content,
                ),
            )
        {
            return true;
        }
        if let Some(changed) = shared::text_edit_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Variables-panel search filter — pop one char.
        if let Some(changed) =
            shared::variables_search_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.variables_header_rename_active() {
            let changed = shared::variables_header_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) =
            preset_name::preset_name_backspace(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            let changed = shared::variable_row_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            let changed = shared::property_input_backspace(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Font-family picker search box (font_picker_dispatch.rs).
        if self.apply_font_picker_backspace() {
            return true;
        }
        if self.editor_state.editor_ui.icon_picker.open {
            if self.editor_state.editor_ui.icon_picker_select_all {
                self.editor_state.editor_ui.icon_picker_search.clear();
                self.editor_state.editor_ui.icon_picker_select_all = false;
                self.editor_state.editor_ui.icon_picker.hover = None;
                self.editor_state.editor_ui.icon_picker.pressed = None;
                // Filter changed → scroll the list back to the top.
                self.editor_state.editor_ui.icon_picker.scroll.offset = 0.0;
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
                self.editor_state.editor_ui.icon_picker.hover = None;
                self.editor_state.editor_ui.icon_picker.pressed = None;
                // Filter changed → scroll the list back to the top.
                self.editor_state.editor_ui.icon_picker.scroll.offset = 0.0;
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
        if let Some(changed) = shared::chat_input_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Pen authoring: Backspace pops the last anchor (`pen_press.rs`).
        if self.apply_pen_backspace() {
            return true;
        }
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeDelete)
        {
            return true;
        }
        if shared::delete_selection_with_history(&mut self.editor_state) {
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Delete — pops a char from rename / text-edit when active;
    /// otherwise deletes the selected node.
    pub fn apply_delete(&mut self) -> bool {
        if let Some(changed) =
            op_editor_core::save_name_keyboard::delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if let Some(changed) =
            shared::prompt_center_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        if let Some(changed) =
            shared::scene_template_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return true;
        }
        // Preview mode: Delete edits the focused runtime widget, never
        // the editor selection.
        if self.preview.is_some() {
            return self.preview_dispatch_key("Delete", false);
        }
        if self.apply_image_panel_delete() {
            return true;
        }
        // Forward deletion in the join input. A no-op falls through to
        // `delete_owned_by_chrome_input`, which still swallows the key
        // before it can reach the canvas selection.
        if self.editor_state.editor_ui.collab_join_input_active() {
            let changed = op_editor_ui::widgets::collab_ui::join_address_delete_forward(
                &mut self.editor_state.editor_ui,
                self.now_ms,
            )
            .unwrap_or(false);
            if changed {
                self.mark_dirty();
                return true;
            }
        }
        // The open font picker owns Delete. Its search draft handles
        // Backspace separately; forward-delete must never reach the canvas
        // selection behind the overlay.
        if self.editor_state.editor_ui.font_picker.open {
            return true;
        }
        // Settings-modal input owns Delete while focused (forward
        // deletion at the caret) — without this arm the keystroke fell
        // through and removed the selected node behind the modal.
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            return self.apply_settings_delete_forward();
        }
        if self.editor_state.editor_ui.variables_header_rename_active() {
            let changed =
                shared::variables_header_delete_forward(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if let Some(changed) =
            preset_name::preset_name_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.editor_ui.variable_row_focus.is_some() {
            let changed = shared::variable_row_delete_forward(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // The rename draft has no forward deletion — Delete pops the
        // char before the caret, same as Backspace.
        if let Some(changed) = shared::rename_backspace(&mut self.editor_state, self.now_ms) {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        // Delete is FORWARD deletion at the caret (or removes the
        // active selection) — textarea parity.
        if self.editor_state.ui.text_editing.is_some()
            && !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodeProperty(
                    op_editor_core::CollabNodeField::Content,
                ),
            )
        {
            return true;
        }
        if let Some(changed) = shared::text_edit_delete_forward(&mut self.editor_state, self.now_ms)
        {
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.ui.property_focus.is_some()
            || self.editor_state.editor_ui.effect_param_focus.is_some()
        {
            let changed =
                shared::property_input_delete_forward(&mut self.editor_state, self.now_ms);
            if changed {
                self.mark_dirty();
            }
            return changed;
        }
        if self.editor_state.chat.focused
            && self.editor_state.chat.delete_input_selection(self.now_ms)
        {
            self.mark_dirty();
            return true;
        }
        // Don't delete the selected node when a chrome text input or
        // search overlay owns the keyboard. Those branches ran above
        // (or deliberately swallow Delete); falling through here would
        // silently drop the node behind the focused field.
        if shared::delete_owned_by_chrome_input(&self.editor_state) {
            return false;
        }
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeDelete)
        {
            return true;
        }
        if shared::delete_selection_with_history(&mut self.editor_state) {
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
        if !self.collab_allows_document_mutation(
            op_editor_core::CollabDocumentMutation::Unsupported(
                op_editor_core::CollabUnsupportedFeature::Duplicate,
            ),
        ) {
            return true;
        }
        let result = if let Some(allocator) = self.collab_id_allocator.as_mut() {
            if self.editor_state.selection.is_empty() {
                Ok(None)
            } else {
                let snapshot = self.editor_state.snapshot_for_history();
                let result = self
                    .editor_state
                    .duplicate_selected_with_allocator(allocator, 10.0);
                if result.as_ref().is_ok_and(Option::is_some) {
                    self.editor_state.history_push_past(snapshot);
                }
                result
            }
        } else {
            Ok(
                shared::duplicate_selection(&mut self.editor_state, &mut self.next_node_id)
                    .then_some(self.editor_state.selection.anchor.clone()),
            )
        };
        let dup = match result {
            Ok(id) => id.is_some(),
            Err(error) => {
                self.show_collab_id_error(error);
                return true;
            }
        };
        if dup {
            self.mark_dirty();
        }
        dup
    }
}
