//! Native-menu action dispatch for [`DesktopApp`].
//!
//! Split out of the runner spine (`main.rs`) to keep that file under the
//! 800-line cap. Behaviour is unchanged — these are the same host calls
//! the keyboard shortcuts make.

use winit::event_loop::ActiveEventLoop;
use winit::window::Fullscreen;

use crate::{menu, persistence, update_check, DesktopApp};

/// File-menu label for a recent path — the file name, falling back to the
/// full path when it has no final component.
fn recent_menu_label(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::to_string)
        .unwrap_or_else(|| path.to_string())
}

impl DesktopApp {
    /// Sync the native File ▸ Open Recent submenu with the host's current
    /// recent-file list (file names, newest first). Cheap to call every loop
    /// iteration: the muda submenu is rebuilt only when the labels actually
    /// changed, so it stays current no matter which path (native menu,
    /// in-canvas File menu, or Finder open) touched the recent list. No-op
    /// off macOS.
    pub(crate) fn refresh_recent_menu(&mut self) {
        // Menu-item labels are baked at build time, so a locale switch
        // needs a full rebuild (replaces the NSApp main menu). Cheap
        // equality check per loop iteration; rebuilds only on change.
        let locale = self.host.editor_state().editor_ui.locale;
        if let (Some(menu), Some(window)) = (self.app_menu.as_ref(), self.window.as_ref()) {
            if menu.locale() != locale {
                self.app_menu = Some(menu::AppMenu::install_with_locale(window, locale));
                // Force a reseed of the fresh menu's recent submenu below.
                self.recent_menu_paths.clear();
                self.recent_menu_labels.clear();
            }
        }
        let Some(menu) = self.app_menu.as_ref() else {
            return;
        };
        let recent = &self.host.editor_state().editor_ui.recent_files;
        // Allocation-free change check first — this runs on EVERY loop
        // iteration, and building the label strings each time (path
        // parsing + per-entry allocations) showed up in gesture-time
        // profiles. Labels are derived only when the paths changed.
        if recent.len() == self.recent_menu_paths.len()
            && recent
                .iter()
                .zip(self.recent_menu_paths.iter())
                .all(|(r, cached)| r.path == *cached)
        {
            return;
        }
        let labels: Vec<String> = recent.iter().map(|r| recent_menu_label(&r.path)).collect();
        self.recent_menu_paths = recent.iter().map(|r| r.path.clone()).collect();
        if labels != self.recent_menu_labels {
            menu.set_recent_files(&labels);
            self.recent_menu_labels = labels;
        }
    }

    /// Dispatch a native-menu selection onto the matching host action —
    /// the same calls the keyboard shortcuts make.
    pub(crate) fn handle_menu_action(
        &mut self,
        action: menu::MenuAction,
        event_loop: &ActiveEventLoop,
    ) {
        use menu::MenuAction as A;
        // Native menu accelerators bypass winit key delivery on macOS and
        // Windows, so each menu dispatch gets the same one-shot transaction
        // boundary as a keyboard command.
        let transaction = !matches!(action, A::Undo | A::Redo)
            && self.collab_runtime.begin_local_edit(&mut self.host);
        let consumed = match action {
            A::New => {
                if persistence::run_action(
                    op_editor_core::editor_ui_state::FileAction::New,
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                ) == op_host_services::doc_io::ActionOutcome::Saved
                {
                    self.mark_document_saved();
                }
                true
            }
            A::Open => {
                let ok = persistence::handle_open(
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                );
                if ok {
                    self.mark_document_saved();
                }
                ok
            }
            A::OpenRecent(i) => {
                let opened = persistence::run_action(
                    op_editor_core::editor_ui_state::FileAction::OpenRecent(i),
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                ) == op_host_services::doc_io::ActionOutcome::Saved;
                if opened {
                    self.mark_document_saved();
                }
                opened
            }
            A::Save => {
                self.host.commit_variable_row_focus_if_any_pub();
                self.request_background_save()
            }
            A::SaveAs => {
                self.host.commit_variable_row_focus_if_any_pub();
                self.request_background_save_as()
            }
            A::Export => {
                self.host.commit_variable_row_focus_if_any_pub();
                self.host
                    .editor_state_mut()
                    .editor_ui
                    .image_panel
                    .close_popovers();
                persistence::run_action(
                    op_editor_core::editor_ui_state::FileAction::ExportImage,
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                );
                true
            }
            A::ExportAllFrames => {
                self.host.commit_variable_row_focus_if_any_pub();
                persistence::run_action(
                    op_editor_core::editor_ui_state::FileAction::ExportAllFrames,
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                );
                true
            }
            A::Undo => self.collab_runtime.request_undo(&mut self.host) || self.host.apply_undo(),
            A::Redo => self.collab_runtime.reject_redo(&mut self.host) || self.host.apply_redo(),
            // Route through the input-aware dispatch (not the raw
            // `host.apply_*`) so Cmd+X / C / V cut, copy, and paste work
            // inside focused text fields. On macOS / Windows the Edit-menu
            // accelerator owns these chords (AppKit / Win32 consume the key
            // event, so the winit keydown in `handle_key_pressed` never
            // fires) — the same single-path-per-platform contract the
            // existing Undo / Duplicate menu items rely on, which would
            // otherwise double-fire on every press. Linux has no native
            // menu, so there `handle_key_pressed` is the only path.
            A::Cut => self.handle_cmd_cut(),
            A::Copy => self.handle_cmd_copy(),
            A::Paste => self.handle_cmd_paste(),
            A::SelectAll => self.host.apply_select_all(),
            A::Duplicate => self.host.apply_duplicate(),
            A::Group => self.host.apply_group(),
            A::Ungroup => self.host.apply_ungroup(),
            A::ToggleFullscreen => {
                if let Some(window) = self.window.as_ref() {
                    let next = match window.fullscreen() {
                        Some(_) => None,
                        None => Some(Fullscreen::Borderless(None)),
                    };
                    window.set_fullscreen(next);
                }
                false
            }
            A::ToggleGitPanel => {
                let opening = !self.host.editor_state().editor_ui.git_panel.open;
                if opening {
                    self.host
                        .editor_state_mut()
                        .editor_ui
                        .image_panel
                        .close_popovers();
                    // Show "Loading…" until the first snapshot lands,
                    // then request the snapshot from the git session.
                    self.host.editor_state_mut().editor_ui.git_panel.loading = true;
                    self.refresh_git_panel();
                } else {
                    // Closing — release the commit input's focus and
                    // discard any open diff view so a later reopen
                    // starts clean on the status list. Drop the diff
                    // job too: a result landing post-close must not
                    // repopulate the now-hidden panel.
                    self.git_diff_job = None;
                    // Abandon an in-flight clone + close its wizard so a
                    // result can't bind onto a hidden panel and a rapid
                    // close→reopen can't resurface a stale cloning form.
                    self.git_clone_job = None;
                    let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
                    panel.defocus_commit_input(0);
                    panel.remote_focused = false;
                    panel.https_focused = false;
                    panel.diff = None;
                    panel.merge_resolve = None;
                    panel.clone_form = None;
                }
                self.host.editor_state_mut().editor_ui.git_panel.open = opening;
                self.host.mark_editor_state_dirty();
                true
            }
            A::ToggleDesignMdPanel => {
                let ui = &mut self.host.editor_state_mut().editor_ui;
                let opening = !ui.design_md_panel.open;
                if opening {
                    ui.image_panel.close_popovers();
                    // Centre the floating panel on the viewport the
                    // first time it opens (and re-centre on reopen so
                    // it never strands off-screen after a resize).
                    ui.design_md_panel.pos = Some((
                        ((self.viewport_width - op_editor_ui::widgets::DESIGN_MD_PANEL_W) / 2.0)
                            .max(0.0),
                        ((self.viewport_height - op_editor_ui::widgets::DESIGN_MD_PANEL_H) / 2.0)
                            .max(0.0),
                    ));
                }
                ui.design_md_panel.open = opening;
                self.host.mark_editor_state_dirty();
                true
            }
            A::ToggleComponentBrowserPanel => {
                let ui = &mut self.host.editor_state_mut().editor_ui;
                let opening = !ui.component_browser_open;
                if opening {
                    ui.image_panel.close_popovers();
                    ui.component_browser_pos = Some((
                        ((self.viewport_width - op_editor_ui::widgets::COMPONENT_BROWSER_PANEL_W)
                            / 2.0)
                            .max(0.0),
                        ((self.viewport_height - op_editor_ui::widgets::COMPONENT_BROWSER_PANEL_H)
                            / 2.0)
                            .max(0.0),
                    ));
                }
                ui.component_browser_open = opening;
                self.host.mark_editor_state_dirty();
                true
            }
            A::Quit => {
                // Route through the unsaved-changes prompt — Cancel
                // there aborts the quit.
                if self.confirm_close() {
                    event_loop.exit();
                }
                false
            }
            A::CheckUpdates => {
                // Re-run the probe; the System tab reflects `Checking`
                // immediately and the result lands on a later frame.
                // Skip when a probe is already in flight so repeated
                // menu clicks can't stack untracked worker threads.
                if self.update_probe.is_pending() {
                    false
                } else {
                    self.host.editor_state_mut().editor_ui.update_status =
                        op_editor_core::UpdateStatus::Checking;
                    self.host.mark_editor_state_dirty();
                    self.update_probe = update_check::UpdateProbe::spawn();
                    self.update_prompt_shown = false;
                    true
                }
            }
            A::OpenGithub => {
                update_check::open_url("https://github.com/ZSeven-W/openpencil");
                false
            }
        };
        if transaction {
            self.collab_runtime.finish_local_edit(&mut self.host);
        }
        if consumed {
            self.request_redraw(true);
        }
    }
}
