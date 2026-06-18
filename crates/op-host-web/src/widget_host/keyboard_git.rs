//! Git-panel keyboard routing for the web widget host.

use super::WidgetHost;
use op_editor_core::{CloneField, GitBranchPickerMode, GitPanelAction};

impl WidgetHost {
    pub(in crate::widget_host) fn apply_git_text(&mut self, c: char) -> Option<bool> {
        if self.git_clone_input_active() {
            if c.is_control() {
                return Some(false);
            }
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                if !form.cloning {
                    let mut s = [0u8; 4];
                    match form.focus {
                        Some(CloneField::Url) => form
                            .url_input
                            .insert_str(c.encode_utf8(&mut s), self.now_ms),
                        Some(CloneField::Dest) => form
                            .dest_input
                            .insert_str(c.encode_utf8(&mut s), self.now_ms),
                        None => {}
                    }
                    form.error = None;
                }
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_commit_focus_active() {
            if c.is_control() {
                return Some(false);
            }
            let panel = &mut self.editor_state.editor_ui.git_panel;
            let mut s = [0u8; 4];
            panel
                .commit_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            panel.commit_no_changes = false;
            self.mark_dirty();
            return Some(true);
        }
        if self.git_remote_focus_active() {
            if c.is_control() {
                return Some(false);
            }
            let panel = &mut self.editor_state.editor_ui.git_panel;
            let mut s = [0u8; 4];
            panel
                .remote_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        if self.git_https_focus_active() {
            if c.is_control() {
                return Some(false);
            }
            let panel = &mut self.editor_state.editor_ui.git_panel;
            let mut s = [0u8; 4];
            panel
                .https_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        if self.git_author_focus_active() {
            if c.is_control() {
                return Some(false);
            }
            let panel = &mut self.editor_state.editor_ui.git_panel;
            let mut s = [0u8; 4];
            if panel.author_email_focused {
                panel
                    .author_email_input
                    .insert_str(c.encode_utf8(&mut s), self.now_ms);
            } else {
                panel
                    .author_name_input
                    .insert_str(c.encode_utf8(&mut s), self.now_ms);
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_branch_create_focus_active() {
            if c.is_control() {
                return Some(false);
            }
            let panel = &mut self.editor_state.editor_ui.git_panel;
            let mut s = [0u8; 4];
            panel
                .branch_create_input
                .insert_str(c.encode_utf8(&mut s), self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        None
    }

    pub(in crate::widget_host) fn apply_git_backspace(&mut self) -> Option<bool> {
        if self.git_clone_input_active() {
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                if !form.cloning {
                    match form.focus {
                        Some(CloneField::Url) => form.url_input.backspace(self.now_ms),
                        Some(CloneField::Dest) => form.dest_input.backspace(self.now_ms),
                        None => {}
                    }
                    form.error = None;
                }
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.commit_input.backspace(self.now_ms);
            panel.commit_no_changes = false;
            self.mark_dirty();
            return Some(true);
        }
        if self.git_remote_focus_active() {
            self.editor_state
                .editor_ui
                .git_panel
                .remote_input
                .backspace(self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        if self.git_https_focus_active() {
            self.editor_state
                .editor_ui
                .git_panel
                .https_input
                .backspace(self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        if self.git_author_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if panel.author_email_focused {
                panel.author_email_input.backspace(self.now_ms);
            } else {
                panel.author_name_input.backspace(self.now_ms);
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_branch_create_focus_active() {
            self.editor_state
                .editor_ui
                .git_panel
                .branch_create_input
                .backspace(self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        None
    }

    pub(in crate::widget_host) fn apply_git_send(&mut self) -> Option<bool> {
        if self.git_clone_input_active() {
            let submit = self
                .editor_state
                .editor_ui
                .git_panel
                .clone_form
                .as_ref()
                .is_some_and(|form| form.focus.is_some() && !form.cloning);
            if submit {
                self.editor_state.editor_ui.git_panel.pending_action =
                    Some(GitPanelAction::SubmitClone);
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.commit_input.text().trim().is_empty()
                && panel.changed_files.iter().any(|file| file.staged)
            {
                panel.pending_action = Some(GitPanelAction::Commit);
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.remote_input.text().trim().is_empty() {
                panel.pending_action = Some(GitPanelAction::SetRemote(
                    panel.remote_input.text().to_owned(),
                ));
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.https_input.text().trim().is_empty() {
                panel.pending_action = Some(GitPanelAction::SetHttpsAuth(
                    panel.https_input.text().to_owned(),
                ));
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_branch_create_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            let name = panel.branch_create_input.text().trim().to_string();
            if !name.is_empty() {
                panel.pending_action = Some(GitPanelAction::CreateBranch(name));
                panel.branch_picker_mode = GitBranchPickerMode::List;
                panel.branch_create_input.set_text("");
                panel.branch_create_focused = false;
                panel.branch_picker_open = false;
                panel.branch_picker_menu.hover = None;
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_author_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            if !panel.author_name_input.text().trim().is_empty()
                && panel.author_email_input.text().contains('@')
            {
                panel.pending_action = Some(GitPanelAction::SaveAuthor);
            }
            self.mark_dirty();
            return Some(true);
        }
        if self.git_ready_popover_open() {
            return Some(true);
        }
        None
    }

    pub(in crate::widget_host) fn apply_git_input_select_all(&mut self) -> Option<bool> {
        if self.git_clone_input_active() {
            if let Some(form) = self.editor_state.editor_ui.git_panel.clone_form.as_mut() {
                match form.focus {
                    Some(CloneField::Url) => {
                        form.url_input.select_all();
                        form.url_input.touch(self.now_ms);
                        self.mark_dirty();
                        return Some(true);
                    }
                    Some(CloneField::Dest) => {
                        form.dest_input.select_all();
                        form.dest_input.touch(self.now_ms);
                        self.mark_dirty();
                        return Some(true);
                    }
                    None => {}
                }
            }
            return Some(false);
        }
        if self.git_commit_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.commit_input.select_all();
            panel.commit_input.touch(self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        if self.git_remote_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.remote_input.select_all();
            panel.remote_input.touch(self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        if self.git_https_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.https_input.select_all();
            panel.https_input.touch(self.now_ms);
            self.mark_dirty();
            return Some(true);
        }
        if self.git_branch_create_focus_active() {
            let panel = &mut self.editor_state.editor_ui.git_panel;
            panel.branch_create_input.select_all();
            panel.branch_create_input.touch(self.now_ms);
            self.mark_dirty();
            return Some(true);
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
            return Some(true);
        }
        None
    }
}
