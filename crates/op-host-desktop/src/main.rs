//! OpenPencil desktop runner — winit + skia-safe + WidgetHostNative.
//! Owns the event loop, GL surface, DPI, animation timer + cursor input.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

mod app_handler;
mod chat_acp;
mod chat_attachment;
mod chat_builtin_http;
mod chat_claude;
mod chat_copilot;
mod chat_http_server;
mod chat_provider_llm;
mod chat_runtime;
mod chat_session;
mod chat_subprocess;
mod clipboard;
mod cursor_icon;
mod design_md_host;
mod design_session;
mod export;
mod export_pdf;
mod figma_import_session;
mod frame;
mod git_host;
mod git_jobs;
mod git_session;
mod iconify_host;
mod image_search_session;
mod keyboard_input;
mod macos_app;
mod mcp_serve;
mod menu;
mod model_discovery;
mod persistence;
mod persistence_image;
mod pre_validator;
mod settings_io;
mod update_check;
mod window_state;

use op_host_native::{NativeBackend, SharedSkiaContext, SharedSkiaError, WidgetHostNative};
use std::path::PathBuf;
use std::time::Instant;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window};

const INITIAL_VIEWPORT_W: f32 = 1440.0;
const INITIAL_VIEWPORT_H: f32 = 900.0;

struct DesktopApp {
    window: Option<Window>,
    ctx: Option<SharedSkiaContext>,
    backend: Option<NativeBackend>,
    host: WidgetHostNative,
    /// Cached LOGICAL viewport size (refreshed on Resumed + Resized).
    viewport_width: f32,
    viewport_height: f32,
    /// Last cursor position (logical, top-left origin).
    cursor_x: f32,
    cursor_y: f32,
    /// Cached scale factor (refreshed on Resumed + ScaleFactorChanged).
    dpi: f32,
    /// Cmd / Ctrl held — promotes scroll to zoom + gates editor shortcuts.
    zoom_modifier: bool,
    alt_modifier: bool,
    /// Shift held — arrow-key nudge 1→10 px.
    shift_modifier: bool,
    /// Cursor moves coalesced between paints; drained on RedrawRequested
    /// and right before apply_press/release so drag-end frames aren't lost.
    pending_cursor_move: Option<(f32, f32)>,
    /// True iff a `request_redraw` is already in flight.
    redraw_pending: bool,
    /// True when the pending redraw needs a paint even if cursor coalescing drained to no-op.
    redraw_dirty: bool,
    /// Monotonic clock anchor — `Instant.elapsed().as_millis()`
    /// from this is fed into `WidgetHostNative::set_now_ms` so
    /// `jian_core::anim::blink_visible` can drive the caret blink
    /// (and any future time-based UI animation).
    clock_start: Instant,
    /// Cached custom rotate cursor — built once at `Resumed` and
    /// reused for every CursorHint::Rotate to avoid re-decoding
    /// the bitmap every move. None until the event loop is ready.
    rotate_cursor: Option<winit::window::CustomCursor>,
    /// Path of the currently-open .pen/.op document; None when unsaved.
    current_path: Option<PathBuf>,
    error: Option<SharedSkiaError>,
    /// In-flight AI chat turn, if any. `chat.begin_send` raises
    /// `chat.pending_send`; the event loop drains that into a
    /// `ChatSession` here and pumps deltas into the transcript.
    current_chat: Option<chat_session::ChatSession>,
    /// In-flight design-orchestrator turn, if any. Mutually exclusive
    /// with `current_chat`: `chat_session::launch_if_pending` classifies
    /// the user's message via `op_orchestrator::classify_intent` and
    /// routes `Intent::Design` here (when an `agent::Provider` is
    /// available), `Intent::Chat` to `current_chat`.
    current_design: Option<design_session::DesignSession>,
    /// In-flight `.fig` import — worker thread that parses on a
    /// background thread so the editor UI keeps repainting. The pump
    /// in `RedrawRequested` swaps in the parsed document when the
    /// worker finishes.
    current_figma_import: Option<figma_import_session::FigmaImportSession>,
    /// Background AI-model discovery — probes the installed CLIs
    /// on a worker thread; its result is drained into
    /// `chat.available_models` on a later frame.
    model_probe: model_discovery::ModelProbe,
    /// Background auto-search jobs that replace generated empty image
    /// nodes with freely licensed remote images.
    image_search: image_search_session::ImageSearchSession,
    iconify_job: Option<iconify_host::IconifyJob>,
    /// Document to open once the window is ready — set from argv by
    /// the file-association launch path (`openpencil-desktop X.op`).
    initial_file: Option<PathBuf>,
    /// Native menu bar — kept alive for the process lifetime;
    /// `None` until `resumed` builds it (and always `None` on Linux,
    /// where there is no native menu).
    app_menu: Option<menu::AppMenu>,
    /// Background auto-update probe — checks the GitHub releases API
    /// on a worker thread; its result is drained into
    /// `editor_ui.update_status` on a later frame.
    update_probe: update_check::UpdateProbe,
    /// Gates the update-available dialog to once per check.
    update_prompt_shown: bool,
    /// Last *windowed* (non-maximized) outer position, physical px.
    /// Persisted on exit so a restart restores window placement.
    win_pos: Option<(i32, i32)>,
    /// Last *windowed* inner size, physical px.
    win_size: Option<(u32, u32)>,
    /// Whether the window is currently maximized.
    win_maximized: bool,
    /// Document fingerprint captured at the last save / open / new.
    /// `document_is_dirty` compares the live fingerprint against this
    /// to drive the unsaved-changes prompt on close.
    saved_doc_fingerprint: u64,
    /// In-app Git — the repository bound to the open document.
    /// Rebound whenever the document path changes; read by the
    /// window title and the Git panel.
    git_session: git_session::GitSession,
    /// In-flight background `git pull`, if any — keeps the
    /// network-bound pull off the UI thread.
    git_pull_job: Option<git_jobs::GitPullJob>,
    /// In-flight background `git push`, if any.
    git_push_job: Option<git_jobs::GitPushJob>,
    /// Document fingerprint captured when a `git pull` was spawned.
    /// The post-pull reload compares against it to detect edits made
    /// *during* the async pull — which the spawn-time confirm did
    /// not cover — and re-confirm before discarding them.
    git_pull_doc_baseline: Option<u64>,
    /// In-flight background Git status query, if any.
    git_status_job: Option<git_jobs::GitStatusJob>,
    /// In-flight background Git diff (`git diff` / `git show`), if any.
    git_diff_job: Option<git_jobs::GitDiffJob>,
    /// When the Git panel was last re-snapshotted — drives the
    /// periodic refresh that keeps an open panel current against
    /// external repository changes.
    last_git_refresh: Instant,
}

impl DesktopApp {
    fn new(initial_file: Option<PathBuf>) -> Self {
        let mut host = WidgetHostNative::new();
        let fit_blank_frame = initial_file.is_none();
        // Best-effort prefs restore onto the host's `EditorState`.
        settings_io::load(host.editor_state_mut());
        if fit_blank_frame {
            host.fit_content_to_viewport(INITIAL_VIEWPORT_W, INITIAL_VIEWPORT_H);
        }
        host.mark_editor_state_dirty();
        // Baseline for the unsaved-changes prompt — the fresh,
        // empty document is by definition "saved" (nothing to lose).
        let saved_doc_fingerprint = persistence::document_fingerprint(host.editor_state());
        let update_probe = update_check::UpdateProbe::for_auto_check(
            host.editor_state()
                .editor_ui
                .agent_settings
                .auto_update_enabled,
        );
        Self {
            window: None,
            ctx: None,
            backend: None,
            host,
            viewport_width: INITIAL_VIEWPORT_W,
            viewport_height: INITIAL_VIEWPORT_H,
            cursor_x: 0.0,
            cursor_y: 0.0,
            dpi: 1.0,
            zoom_modifier: false,
            alt_modifier: false,
            shift_modifier: false,
            pending_cursor_move: None,
            redraw_pending: false,
            redraw_dirty: false,
            clock_start: Instant::now(),
            rotate_cursor: None,
            current_path: None,
            error: None,
            current_chat: None,
            current_design: None,
            current_figma_import: None,
            model_probe: model_discovery::ModelProbe::spawn(),
            image_search: image_search_session::ImageSearchSession::new(),
            iconify_job: None,
            initial_file,
            app_menu: None,
            update_probe,
            update_prompt_shown: false,
            win_pos: None,
            win_size: None,
            win_maximized: false,
            saved_doc_fingerprint,
            git_session: git_session::GitSession::new(),
            git_pull_job: None,
            git_push_job: None,
            git_pull_doc_baseline: None,
            git_status_job: None,
            git_diff_job: None,
            last_git_refresh: Instant::now(),
        }
    }

    /// Snapshot the current document as the saved baseline — called
    /// after every successful load / save / new so `document_is_dirty`
    /// only reports edits made *since* that point. Also rebinds the
    /// Git session (the document path may have changed).
    fn mark_document_saved(&mut self) {
        // Any successful Save / Open / New replaced the document. If
        // a background Figma import is still running, its result
        // would later overwrite this fresh document in `pump` —
        // drop the session here so the worker's `send` becomes a
        // silent no-op when it finishes.
        figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
        self.image_search.reset();
        self.saved_doc_fingerprint = persistence::document_fingerprint(self.host.editor_state());
        self.rebind_git_session_for_current_path();
    }

    /// Rebind the Git session to `current_path`, retitle the window
    /// and refresh an open Git panel — WITHOUT touching the
    /// unsaved-changes baseline. `mark_document_saved` calls this
    /// after a real save; a Figma import calls it directly: the
    /// import changed the document path (so the old repo binding is
    /// stale) but the imported design is unsaved work, so
    /// `saved_doc_fingerprint` must stay put or close would skip the
    /// save prompt.
    fn rebind_git_session_for_current_path(&mut self) {
        // The empty-state "Init" card is gated on the doc having a path.
        self.host
            .editor_state_mut()
            .editor_ui
            .git_panel
            .has_saved_file = self.current_path.is_some();
        let prev_repo = self.git_session.repo().map(|r| r.workdir().to_path_buf());
        let prev_tracked = self.git_session.tracked_file().map(|p| p.to_path_buf());
        self.git_session.rebind(self.current_path.as_deref());
        let new_repo = self.git_session.repo().map(|r| r.workdir().to_path_buf());
        let new_tracked = self.git_session.tracked_file().map(|p| p.to_path_buf());
        if prev_tracked != new_tracked {
            // The tracked document changed — a half-typed commit
            // message was authored for the *previous* document (a
            // commit acts on whatever document is tracked now), so
            // drop the draft and its focus. This fires on any
            // document switch, including between two files in the
            // same repository.
            let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
            panel.commit_message.clear();
            panel.commit_focused = false;
        }
        if prev_repo != new_repo {
            // The bound repository changed — any in-flight git job is
            // for the *previous* repo; drop both (and the transient
            // `pulling` flag) so a stale result can never land on the
            // new binding, even with the panel closed during the
            // switch. The panel goes into a `loading` state so it
            // shows "Loading…" rather than the old repo's data until
            // the new snapshot lands.
            self.git_status_job = None;
            self.git_pull_job = None;
            self.git_push_job = None;
            self.git_pull_doc_baseline = None;
            self.git_diff_job = None;
            let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
            panel.pulling = false;
            panel.pushing = false;
            // A diff / merge-resolution view is for the previous
            // repository — close it.
            panel.diff = None;
            panel.merge_resolve = None;
            if panel.open {
                panel.loading = true;
            }
        }
        self.refresh_window_title();
        // Keep an open Git panel current with the (possibly new)
        // document + repository.
        if self.host.editor_state().editor_ui.git_panel.open {
            self.refresh_git_panel();
        }
    }

    /// Set the window title to `<file> (<branch>) — OpenPencil`, with
    /// the branch shown only when the document is in a git repository.
    fn refresh_window_title(&self) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        let name = self
            .current_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned());
        let title = match (name, self.git_session.current_branch()) {
            (Some(name), Some(branch)) => format!("{name} ({branch}) — OpenPencil"),
            (Some(name), None) => format!("{name} — OpenPencil"),
            (None, _) => "OpenPencil".to_string(),
        };
        window.set_title(&title);
    }

    /// Whether the document carries edits since the last save / open
    /// / new.
    fn document_is_dirty(&self) -> bool {
        persistence::document_fingerprint(self.host.editor_state()) != self.saved_doc_fingerprint
    }

    /// Open documents macOS delivered through the open-documents
    /// Apple event — a Finder double-click, `open file.op`, or a file
    /// dropped on the Dock icon. macOS routes these out-of-band of
    /// argv; the `casement` winit fork captures them and this drains
    /// the buffer. The single-window editor opens the first supported
    /// document and logs any extras. Returns `true` when a document
    /// was opened. A no-op on non-macOS (the buffer is always empty).
    fn drain_opened_files(&mut self) -> bool {
        #[cfg(target_os = "macos")]
        {
            let mut opened = false;
            for path in winit::platform::macos::drain_opened_file_urls() {
                let is_op = persistence::is_supported_document(&path);
                let is_fig = persistence::is_supported_figma_import(&path);
                if !is_op && !is_fig {
                    continue;
                }
                if is_fig
                    && self
                        .current_figma_import
                        .as_ref()
                        .is_some_and(|sess| sess.path() == path.as_path())
                {
                    opened = true;
                    continue;
                }
                if opened {
                    eprintln!(
                        "openpencil-desktop: ignoring extra opened file \
                         (single-window editor): {}",
                        path.display()
                    );
                    continue;
                }
                if is_fig {
                    // `.fig` → background import. Mark `opened` true
                    // so further drops in this batch are skipped, but
                    // don't run `mark_document_saved` (the document is
                    // still pending; pump applies it when the worker
                    // finishes).
                    figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
                    self.current_figma_import =
                        Some(figma_import_session::spawn(&mut self.host, path));
                    self.request_redraw(true);
                    opened = true;
                } else if persistence::open_path(
                    &mut self.host,
                    path,
                    &mut self.current_path,
                    self.window.as_ref(),
                ) {
                    self.mark_document_saved();
                    opened = true;
                }
            }
            opened
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    /// Show the save-changes prompt when the document has unsaved
    /// edits. Returns `true` when it is safe to close — no edits, or
    /// the user chose Save (which succeeded) or Don't Save — and
    /// `false` to abort the close (Cancel, or a Save that failed or
    /// was itself cancelled). Called from the cancellable close
    /// paths: the window close button and the Quit menu item.
    /// Guard a document-reloading Git action (branch switch, merge
    /// abort / complete) against unsaved in-memory edits — the reload
    /// would silently discard them. Returns `true` to proceed (no
    /// edits, or the user chose Save / Discard), `false` to abort
    /// (Cancel, or a Save that failed / was itself cancelled).
    fn confirm_document_reload(&mut self) -> bool {
        // Flush any in-progress text-input draft into the document
        // first — otherwise an unflushed draft would not count toward
        // `document_is_dirty` and the reload would drop it silently.
        self.host.commit_pending_input_pub();
        if !self.document_is_dirty() {
            return true;
        }
        let locale = self.host.editor_state().editor_ui.locale;
        let choice = rfd::MessageDialog::new()
            .set_title(op_i18n::translate(locale, "git.reload.confirmTitle"))
            .set_description(op_i18n::translate(locale, "git.reload.confirmBody"))
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match choice {
            rfd::MessageDialogResult::Yes => {
                self.host.commit_variable_row_focus_if_any_pub();
                if persistence::handle_save(
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                ) {
                    self.mark_document_saved();
                    true
                } else {
                    false
                }
            }
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    fn confirm_close(&mut self) -> bool {
        if !self.document_is_dirty() {
            return true;
        }
        let locale = self.host.editor_state().editor_ui.locale;
        let name = self
            .current_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| op_i18n::translate(locale, "dialog.untitledDocument").to_string());
        let body = op_i18n::translate(locale, "dialog.closeBody").replace("{{name}}", &name);
        let choice = rfd::MessageDialog::new()
            .set_title(op_i18n::translate(locale, "dialog.unsavedTitle"))
            .set_description(&body)
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match choice {
            rfd::MessageDialogResult::Yes => {
                // Save, then close only if the document actually
                // persisted (a cancelled Save-As must abort the close).
                self.host.commit_variable_row_focus_if_any_pub();
                if persistence::handle_save(
                    &mut self.host,
                    &mut self.current_path,
                    self.window.as_ref(),
                ) {
                    self.mark_document_saved();
                    true
                } else {
                    false
                }
            }
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    /// Drain the background auto-update probe into `update_status`.
    /// When the probe reports a newer release, offer to open the
    /// download page — once per check.
    fn poll_update_probe(&mut self) -> bool {
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
    fn poll_git_pull_job(&mut self) -> bool {
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
                            persistence::document_fingerprint(self.host.editor_state()) != base
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
    fn poll_git_push_job(&mut self) -> bool {
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
    fn show_git_op_error_dialog(&self, op: &str, err: &op_git::GitError) {
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
        rfd::MessageDialog::new()
            .set_title(op_i18n::translate(locale, title_key))
            .set_description(format!(
                "{}\n\n{}",
                op_i18n::translate(locale, body_key),
                detail,
            ))
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
    }

    /// Dispatch a native-menu selection onto the matching host
    /// action — the same calls the keyboard shortcuts make.
    fn handle_menu_action(&mut self, action: menu::MenuAction, event_loop: &ActiveEventLoop) {
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
            A::Cut => self.host.apply_cut(),
            A::Copy => self.host.apply_copy(),
            A::Paste => self.host.apply_paste(),
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
                    let panel = &mut self.host.editor_state_mut().editor_ui.git_panel;
                    panel.commit_focused = false;
                    panel.remote_focused = false;
                    panel.https_focused = false;
                    panel.diff = None;
                    panel.merge_resolve = None;
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

    fn request_redraw(&mut self, dirty: bool) -> bool {
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

    fn drain_pending_cursor_move(&mut self) -> bool {
        if let Some((cx, cy)) = self.pending_cursor_move.take() {
            let hover_changed =
                self.host
                    .update_layer_hover(cx, cy, self.viewport_width, self.viewport_height);
            let cursor_changed = self.host.apply_cursor_move(cx, cy);
            hover_changed || cursor_changed
        } else {
            false
        }
    }

    fn prepare_redraw(&mut self) -> bool {
        let tracked_request = self.redraw_pending;
        self.redraw_pending = false;
        let mut should_paint = !tracked_request || self.redraw_dirty;
        self.redraw_dirty = false;
        should_paint |= self.drain_pending_cursor_move();
        should_paint
    }
}

/// Scan argv for a document to open on launch. This is the
/// file-association entry point: once the `.op` / `.pen` association
/// is registered (see `Cargo.toml`'s `[package.metadata.bundle]`),
/// the OS launches this binary with the document path in argv —
/// double-click on Windows / Linux, or `open file.op` from a shell
/// on any platform. The first existing `.op` / `.pen` / `.fig`
/// argument wins; flags (`--mcp`, …) never match the extension
/// filter. `.fig` routes through the Figma import worker once the
/// window is up (see `DesktopApp::apply_initial_file`).
fn initial_file_from_argv() -> Option<PathBuf> {
    std::env::args_os().skip(1).map(PathBuf::from).find(|p| {
        (persistence::is_supported_document(p) || persistence::is_supported_figma_import(p))
            && p.is_file()
    })
}

/// Pop a native dialog offering to open the download page when a
/// newer release is found. Yes opens the GitHub releases page.
fn prompt_update_available(locale: op_editor_core::Locale, version: &str) {
    let body = op_i18n::translate(locale, "dialog.updateBody")
        .replace("{{version}}", version)
        .replace("{{current}}", env!("CARGO_PKG_VERSION"));
    let choice = rfd::MessageDialog::new()
        .set_title(op_i18n::translate(locale, "dialog.updateTitle"))
        .set_description(&body)
        .set_level(rfd::MessageLevel::Info)
        .set_buttons(rfd::MessageButtons::YesNo)
        .show();
    if matches!(choice, rfd::MessageDialogResult::Yes) {
        update_check::open_url(&update_check::releases_url());
    }
}

fn main() {
    // `--mcp` / `--mcp-http` swap the GUI for an MCP server mode;
    // when one of those ran, exit instead of opening a window.
    if mcp_serve::run_cli_if_requested() {
        return;
    }
    let initial_file = initial_file_from_argv();
    let mut event_loop_builder = EventLoop::builder();
    #[cfg(target_os = "macos")]
    {
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        event_loop_builder.with_activation_policy(ActivationPolicy::Regular);
    }
    let event_loop = match event_loop_builder.build() {
        Ok(el) => el,
        Err(err) => {
            eprintln!("openpencil-desktop: EventLoop::new failed: {err}");
            std::process::exit(1);
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    // Give the non-bundled binary a proper Dock name + icon.
    macos_app::apply();
    let mut app = DesktopApp::new(initial_file);
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("openpencil-desktop: run_app exited with error: {err}");
        std::process::exit(1);
    }
    if let Some(err) = app.error {
        eprintln!("openpencil-desktop: fatal error during run: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod main_tests;
