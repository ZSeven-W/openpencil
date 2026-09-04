//! `apply_send` — Enter routing on `WidgetHostNative`: commit whichever
//! chrome input owns focus, else send the chat turn.
//!
//! Split out of `keyboard.rs` to keep every file under the repo's
//! 800-line cap.

use super::WidgetHostNative;

impl WidgetHostNative {
    pub fn apply_send(&mut self) -> bool {
        // Enter in the save-name dialog confirms (mobile keyboards send it
        // as the "done" action); a blank name swallows the key instead.
        if self.editor_state.editor_ui.save_name_dialog.open {
            if self
                .editor_state
                .editor_ui
                .save_name_dialog
                .request_confirm()
            {
                self.mark_dirty();
            }
            return true;
        }
        if self.editor_state.editor_ui.prompt_center.open {
            return true;
        }
        // Enter in the Scene Template Center submits the generate row when
        // that field has the caret, and is swallowed otherwise: the panel is
        // open over the canvas, so the key must never reach chat send.
        if self.editor_state.editor_ui.scene_template_center.open {
            if self.editor_state.editor_ui.scene_template_center.focus
                == op_editor_core::SceneTemplateFocus::Generate
                && self
                    .editor_state
                    .editor_ui
                    .scene_template_center
                    .request_generate()
            {
                self.mark_dirty();
            }
            return true;
        }
        // Preview mode: Enter goes to the focused runtime widget
        // (textarea newline / activation), never chat send.
        if self.preview.is_some() {
            return self.preview_dispatch_key("Enter", false);
        }
        if self.editor_state.editor_ui.collab_join_input_active() {
            let queued = op_editor_ui::widgets::collab_ui::join_address_submit(
                &mut self.editor_state.editor_ui,
            )
            .unwrap_or(false);
            if queued {
                self.mark_dirty();
            }
            return true;
        }
        // The image popover is painted above every editor input. Submit or
        // swallow Enter before consulting any independently stale focus below.
        if self.apply_image_panel_send() {
            return true;
        }
        if self.exit_image_crop_edit() {
            return true;
        }
        if self.editor_state.color_picker_hex_focused() {
            self.collab_blur_color_picker_inputs();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.color_picker_rgb_focused() {
            self.collab_blur_color_picker_inputs();
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.agent_settings.focus.is_some() {
            if op_editor_core::host_ui_transitions::settings_model_newline(
                &mut self.editor_state.editor_ui,
                self.now_ms,
            ) {
                self.mark_dirty();
                return true;
            }
            self.commit_settings_focus_if_any();
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
                panel.branch_picker_menu.hover = None;
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
            let mutation = match self
                .editor_state
                .ui
                .layer_rename
                .as_ref()
                .map(|rename| &rename.target)
            {
                Some(op_editor_core::ui_draft::LayerContextTarget::Layer(_)) => {
                    op_editor_core::CollabDocumentMutation::NodeProperty(
                        op_editor_core::CollabNodeField::Name,
                    )
                }
                Some(op_editor_core::ui_draft::LayerContextTarget::Page(_)) => {
                    op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::PageStructure,
                    )
                }
                None => return false,
            };
            if !self.collab_allows_document_mutation(mutation) {
                let _ = self.editor_state.rename_cancel();
                return true;
            }
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
            if !self.collab_allows_document_mutation(
                op_editor_core::CollabDocumentMutation::NodeProperty(
                    op_editor_core::CollabNodeField::Content,
                ),
            ) {
                return true;
            }
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
        // is already live) — the same transition Escape runs.
        if self.editor_state.editor_ui.blur_variables_search() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.editor_ui.variables_header_rename_active() {
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
}
