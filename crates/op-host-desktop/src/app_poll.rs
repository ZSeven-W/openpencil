//! Background probe / job drains and the redraw scheduler for
//! `DesktopApp`. Carved out of the `main.rs` spine to keep it under the
//! 800-line cap; pure code motion.

use crate::{prompt_update_available, update_check, DesktopApp};

impl DesktopApp {
    /// Drain the browser device-login flow: fold bridge status polls
    /// into UI state and open the verification page (exactly once per
    /// flow) in the system browser.
    pub(crate) fn poll_auth_flow(&mut self) -> bool {
        let changed = self.host.poll_auth();
        if let Some(url) = self.host.take_pending_browser_url() {
            update_check::open_url(&url);
        }
        changed
    }

    /// Drain the background auto-update probe into `update_status`.
    /// When the probe reports a newer release, offer to open the
    /// download page — once per check.
    pub(crate) fn poll_update_probe(&mut self) -> bool {
        let Some(status) = self.update_probe.poll() else {
            return false;
        };
        let available = matches!(status, op_editor_core::UpdateStatus::Available { .. });
        self.host.editor_state_mut().editor_ui.update_status = status.clone();
        self.host.mark_editor_state_dirty();
        if available && !self.update_prompt_shown {
            self.update_prompt_shown = true;
            if let op_editor_core::UpdateStatus::Available { version } = &status {
                let locale = self.host.editor_state().editor_ui.locale;
                prompt_update_available(locale, version);
            }
        }
        true
    }

    /// Drain a finished background `git pull` into the Git panel.
    /// Returns `true` when a result was just drained.
    pub(crate) fn poll_git_pull_job(&mut self) -> bool {
        let Some(job) = self.git_pull_job.as_mut() else {
            return false;
        };
        let Some(result) = job.poll() else {
            return false;
        };
        self.git_pull_job = None;
        let baseline = self.git_pull_doc_baseline.take();
        self.host.editor_state_mut().editor_ui.git_panel.pulling = false;
        match &result {
            Ok(outcome) => {
                // A fast-forward / merge rewrote the tracked document
                // on disk — reload it so the editor reflects the
                // pulled state. A conflict leaves markers that would
                // not parse (the panel shows merge-in-progress
                // instead); an up-to-date pull changes nothing.
                if matches!(
                    outcome,
                    op_git::MergeOutcome::FastForward | op_git::MergeOutcome::Merge
                ) {
                    // Flush any in-progress input draft into the
                    // document so an edit made during the pull is seen
                    // by the comparison below — not silently dropped.
                    self.host.commit_pending_input_pub();
                    // If the user edited the document *while the pull
                    // ran*, the spawn-time confirm did not cover those
                    // edits — re-confirm before the reload discards
                    // them. An unchanged document reloads silently.
                    let edited_during_pull = baseline
                        .map(|base| {
                            let state = self.host.editor_state();
                            (
                                self.host.document_epoch(),
                                state.document_generation(),
                                state.document_revision(),
                            ) != base
                        })
                        .unwrap_or(false);
                    if !edited_during_pull || self.confirm_document_reload() {
                        self.reload_tracked_document();
                    }
                }
            }
            Err(err) => {
                self.show_git_op_error_dialog("pull", err);
            }
        }
        self.refresh_git_panel();
        true
    }

    /// Drain a finished background `git push` into the Git panel.
    /// Returns `true` when a result was just drained.
    pub(crate) fn poll_git_push_job(&mut self) -> bool {
        let Some(job) = self.git_push_job.as_mut() else {
            return false;
        };
        let Some(result) = job.poll() else {
            return false;
        };
        self.git_push_job = None;
        self.host.editor_state_mut().editor_ui.git_panel.pushing = false;
        if let Err(err) = &result {
            // A failed push must be visible — stderr is invisible in
            // a packaged GUI build.
            self.show_git_op_error_dialog("push", err);
        }
        self.refresh_git_panel();
        true
    }

    /// Report a failed git op (pull / push / commit) in a dialog —
    /// the panel otherwise just returns to idle with no signal.
    pub(crate) fn show_git_op_error_dialog(&self, op: &str, err: &op_git::GitError) {
        let locale = self.host.editor_state().editor_ui.locale;
        let (title_key, body_key) = match op {
            "push" => ("git.error.pushTitle", "git.error.pushBody"),
            "commit" => ("git.error.commitTitle", "git.error.commitBody"),
            _ => ("git.error.pullTitle", "git.error.pullBody"),
        };
        // The translated variant message keeps the actionable git
        // output via its `{{detail}}` slot (stderr / path / IO text).
        let detail =
            op_i18n::translate(locale, err.i18n_key()).replace("{{detail}}", &err.i18n_detail());
        crate::message_dialog::alert(
            op_i18n::translate(locale, title_key),
            &format!("{}\n\n{}", op_i18n::translate(locale, body_key), detail),
            rfd::MessageLevel::Error,
        );
    }

    pub(crate) fn request_redraw(&mut self, dirty: bool) -> bool {
        if dirty {
            self.redraw_dirty = true;
        }
        if self.redraw_pending {
            return false;
        }
        self.redraw_pending = true;
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
        true
    }

    pub(crate) fn drain_pending_cursor_move(&mut self) -> bool {
        if let Some((cx, cy)) = self.pending_cursor_move.take() {
            let model_picker_open = self.host.editor_state().editor_ui.chat_model_picker.open;
            let over_layer_panel = !model_picker_open
                && self.host.cursor_over_layer_panel(
                    cx,
                    cy,
                    self.viewport_width,
                    self.viewport_height,
                );
            let hover_changed = !model_picker_open
                && self
                    .host
                    .update_layer_hover(cx, cy, self.viewport_width, self.viewport_height);
            // A top-most menu (file / import / locale / shape / layer context /
            // chat model) paints OVER the layer panel, so when one is open the
            // cursor must still reach `apply_cursor_move` (which updates its
            // hover) even inside the panel's x-range. Otherwise the overlay's
            // left half — overlapping the sidebar — is short-circuited here and
            // its rows never highlight (only the right half, clear of the
            // sidebar, did).
            let overlay_open = {
                let eui = &self.host.editor_state().editor_ui;
                eui.file_menu_open
                    || eui.import_menu_open
                    || eui.locale_picker.open
                    || eui.shape_picker.open
                    || eui.layer_context_menu.is_some()
                    || eui.chat_model_picker.open
            };
            // Side-panel resize starts on the gutter but must keep receiving
            // cursor moves after the pointer crosses back into the layer rail.
            let cursor_changed = if over_layer_panel
                && !self.host.layer_drag_in_progress()
                && !self.host.is_resizing_panel()
                && !overlay_open
            {
                false
            } else {
                self.host.apply_cursor_move(cx, cy)
            };
            hover_changed || cursor_changed
        } else {
            false
        }
    }

    pub(crate) fn prepare_redraw(&mut self) -> bool {
        let tracked_request = self.redraw_pending;
        self.redraw_pending = false;
        let mut should_paint = !tracked_request || self.redraw_dirty;
        self.redraw_dirty = false;
        should_paint |= self.drain_pending_cursor_move();
        // A template chosen in the Scene Template Center replaces the
        // document here rather than inside the press handler: loading is a
        // host capability, and the panel deliberately only records the
        // request.
        let now_ms = self.clock_start.elapsed().as_millis() as u64;
        should_paint |= crate::scene_template_open::drain_pending_scene_template(
            &mut self.host,
            &mut self.current_path,
            self.window.as_ref(),
            now_ms,
        );
        // A topic typed into the same panel's generate row replaces the
        // document too, then queues a chat turn on it. The launch has to
        // happen here rather than waiting for the next pointer / key event:
        // nothing else is guaranteed to arrive, so a queued turn would sit
        // unsent until the user happened to touch the window again.
        // The Styles tab's DESIGN.md import: a file dialog, and the disk half
        // of adding or removing a guide the panel has already put in (or taken
        // out of) the runtime catalogue.
        should_paint |= crate::style_import_host::drain_pending_style_import(self);
        // The Templates tab's saved-template delete: the disk half of removing
        // a template the panel has already taken out of the runtime registry.
        should_paint |=
            crate::user_template_store::drain_pending_template_delete(&mut self.host, now_ms);
        if crate::scene_template_generate::drain_pending_scene_template_generate(
            &mut self.host,
            &mut self.current_path,
            self.window.as_ref(),
        ) {
            self.launch_chat_if_pending();
            should_paint = true;
        }
        should_paint
    }
}
