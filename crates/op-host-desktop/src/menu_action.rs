//! Native-menu action dispatch for [`DesktopApp`].
//!
//! Split out of the runner spine (`main.rs`) to keep that file under the
//! 800-line cap. Behaviour is unchanged — these are the same host calls
//! the keyboard shortcuts make.

use winit::event_loop::ActiveEventLoop;
use winit::window::Fullscreen;

use crate::{menu, persistence, update_check, DesktopApp};

impl DesktopApp {
    /// Dispatch a native-menu selection onto the matching host action —
    /// the same calls the keyboard shortcuts make.
    pub(crate) fn handle_menu_action(
        &mut self,
        action: menu::MenuAction,
        event_loop: &ActiveEventLoop,
    ) {
        use menu::MenuAction as A;
        let consumed = match action {
            A::New => {
                if persistence::run_action(
                    op_editor_core::editor_ui_state::FileAction::New,
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                ) == persistence::ActionOutcome::Saved
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
            A::Save => {
                self.host.commit_variable_row_focus_if_any_pub();
                let ok = persistence::handle_save(
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                );
                if ok {
                    self.mark_document_saved();
                }
                ok
            }
            A::SaveAs => {
                self.host.commit_variable_row_focus_if_any_pub();
                let ok = persistence::handle_save_as(
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                );
                if ok {
                    self.mark_document_saved();
                }
                ok
            }
            A::Export => {
                self.host.commit_variable_row_focus_if_any_pub();
                persistence::run_action(
                    op_editor_core::editor_ui_state::FileAction::ExportImage,
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                );
                true
            }
            A::Undo => self.host.apply_undo(),
            A::Redo => self.host.apply_redo(),
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
                let opening = !ui.design_md_panel_open;
                if opening {
                    // Centre the floating panel on the viewport the
                    // first time it opens (and re-centre on reopen so
                    // it never strands off-screen after a resize).
                    ui.design_md_panel_pos = Some((
                        ((self.viewport_width - op_editor_ui::widgets::DESIGN_MD_PANEL_W) / 2.0)
                            .max(0.0),
                        ((self.viewport_height - op_editor_ui::widgets::DESIGN_MD_PANEL_H) / 2.0)
                            .max(0.0),
                    ));
                }
                ui.design_md_panel_open = opening;
                self.host.mark_editor_state_dirty();
                true
            }
            A::ToggleComponentBrowserPanel => {
                let ui = &mut self.host.editor_state_mut().editor_ui;
                let opening = !ui.component_browser_open;
                if opening {
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
        if consumed {
            self.request_redraw(true);
        }
    }
}
