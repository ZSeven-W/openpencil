//! `DesktopApp` construction plus document / window / session
//! bookkeeping — title, dirty tracking, Git rebinding, file-open and
//! close confirmation. Carved out of the `main.rs` spine to keep it
//! under the 800-line cap (same pattern as `menu_action.rs`); pure code
//! motion.

use crate::{
    collab_avatar_host, figma_import_session, git_session, html_import_session, image_decode_host,
    image_panel_host, image_search_session, ime_window, init_auth_runtime, kit_persistence,
    persistence, remote_image_host, save_session, single_instance, theme_preset_host, update_check,
    DesktopApp, PaintedPageIdentity, INITIAL_VIEWPORT_H, INITIAL_VIEWPORT_W,
};
use op_host_native::WidgetHostNative;
use std::path::PathBuf;
use std::time::Instant;

impl DesktopApp {
    pub(crate) fn new(initial_file: Option<PathBuf>) -> Self {
        // (The brand-logo catalog is registered once in `main` before any render
        // path — GUI / `--render-shots` / MCP — so it is already loaded here.)
        let mut host = WidgetHostNative::new();
        let fit_blank_frame = initial_file.is_none();
        // Best-effort prefs restore onto the host's `EditorState`.
        op_host_services::settings_io::load(host.editor_state_mut());
        // Zode is a desktop-local integration. Keep it out of the shared
        // settings loader so `--serve-web` never exposes machine-local Zode
        // providers that the browser settings UI cannot manage.
        op_host_services::zode_import::import_zode_builtin_agents(host.editor_state_mut());
        // Imported UIKits + browser-open flag (`uikits.json`). Skipped
        // under test like the update / model probes — unit tests must
        // not see a developer machine's kit store.
        if !cfg!(test) {
            kit_persistence::load(host.editor_state_mut());
            // #20: saved theme presets (`theme-presets.json`).
            theme_preset_host::load(host.editor_state_mut());
            // Seed the font picker's imported-family snapshot from the
            // registry that `fonts::FontStore::rescan_and_register`
            // repopulated in `main`, so restored fonts show at once.
            host.refresh_imported_fonts();
        }
        // Desktop is the host that drains the import / remove requests,
        // so it advertises the capability (unconditionally, incl. tests)
        // — the picker paints the Import row + imported group here, and
        // web leaves the default `false` so those controls stay hidden.
        host.editor_state_mut().editor_ui.font_import_supported = true;
        // Same rationale for the File-menu batch-export row: desktop
        // owns the directory picker + offscreen exporter it needs.
        host.editor_state_mut()
            .editor_ui
            .batch_frame_export_supported = true;
        // Account gate + session restore. The bridge links the proprietary
        // auth library when a prebuilt exists for this target; stub builds
        // keep every account entry point hidden unless the dev fake-login
        // env is set. Skipped under test like the update / model probes.
        if !cfg!(test) {
            init_auth_runtime(&mut host);
        }
        let kit_browser_open_persisted = Some(host.editor_state().editor_ui.component_browser_open);
        if fit_blank_frame {
            host.fit_content_to_viewport(INITIAL_VIEWPORT_W, INITIAL_VIEWPORT_H);
        }
        host.editor_state_mut().mark_saved_revision();
        host.mark_editor_state_dirty();
        let update_probe = if cfg!(test) {
            update_check::UpdateProbe::idle()
        } else {
            update_check::UpdateProbe::for_auto_check(
                host.editor_state()
                    .editor_ui
                    .agent_settings
                    .auto_update_enabled,
            )
        };
        let model_probe = if cfg!(test) {
            op_host_services::model_discovery::ModelProbe::idle()
        } else {
            let connected = host.editor_state().editor_ui.agent_settings.connected;
            op_host_services::model_discovery::ModelProbe::spawn_for_connected(connected)
        };
        Self {
            window: None,
            a11y: None,
            ctx: None,
            backend: None,
            host,
            viewport_width: INITIAL_VIEWPORT_W,
            viewport_height: INITIAL_VIEWPORT_H,
            pending_initial_blank_frame_fit: fit_blank_frame,
            cursor_x: 0.0,
            cursor_y: 0.0,
            dpi: 1.0,
            ime_window_sync: ime_window::ImeWindowSync::default(),
            zoom_modifier: false,
            alt_modifier: false,
            shift_modifier: false,
            pending_cursor_move: None,
            redraw_pending: false,
            redraw_dirty: false,
            last_painted_page: None,
            clock_start: Instant::now(),
            rotate_cursor: None,
            current_path: None,
            save_session: save_session::SaveSession::new(),
            collab_fork_saves: Vec::new(),
            error: None,
            design_loop_indicator: None,
            design_session_indicator: None,
            sub_agents: Vec::new(),
            active_sub_agent: 0,
            current_chat: None,
            chat_running_tab: None,
            current_design: None,
            current_codegen: None,
            current_design_md: None,
            codegen_results: Default::default(),
            #[cfg(test)]
            design_md_test_provider: None,
            current_figma_import: None,
            current_html_import: None,
            pending_figma_paste: None,
            pending_html_paste: None,
            model_probe,
            image_search: image_search_session::ImageSearchSession::new(),
            image_panel: image_panel_host::ImagePanelJobs::new(),
            remote_images: remote_image_host::RemoteImageSession::new(),
            collab_avatars: collab_avatar_host::CollabAvatarHost::new(),
            image_decodes: image_decode_host::ImageDecodeHost::new(),
            mcp_wake_proxy: None,
            collab_runtime: crate::collab_runtime::DesktopCollabRuntime::new(),
            forwarded_files: single_instance::ForwardQueue::default(),
            iconify_job: None,
            kit_browser_open_persisted,
            provider_connect_job: None,
            provider_reconnect_queue: Vec::new(),
            remembered_connections: [false; 6],
            last_seen_provider_phase: Default::default(),
            hovered_image_drop: false,
            drop_cursor: None,
            last_saved_pencil_cursor: None,
            acp_agent_connect_job: None,
            initial_file,
            app_menu: None,
            recent_menu_labels: Vec::new(),
            recent_menu_paths: Vec::new(),
            update_probe,
            update_prompt_shown: false,
            win_pos: None,
            win_size: None,
            win_maximized: false,
            git_session: git_session::GitSession::new(),
            git_pull_job: None,
            git_push_job: None,
            git_pull_doc_baseline: None,
            git_status_job: None,
            git_diff_job: None,
            git_clone_job: None,
            git_clone_origin: None,
            last_git_refresh: Instant::now(),
            mcp_server: None,
            force_live_mcp_port: None,
            mcp_integrations_home: None,
        }
    }

    pub(crate) fn active_page_paint_identity(&self) -> PaintedPageIdentity {
        let document_epoch = self.host.document_epoch();
        let state = self.host.editor_state();
        let Some(pages) = state.doc.pages.as_ref().filter(|pages| !pages.is_empty()) else {
            return PaintedPageIdentity {
                document_epoch,
                page_id: "__document_root__".to_string(),
                duplicate_index: None,
            };
        };
        let index = state.ui.active_page_index.min(pages.len() - 1);
        let page_id = pages[index].id.to_string();
        let duplicate = pages
            .iter()
            .filter(|page| page.id.as_str() == page_id.as_str())
            .take(2)
            .count()
            > 1;
        PaintedPageIdentity {
            document_epoch,
            page_id,
            duplicate_index: duplicate.then_some(index),
        }
    }

    pub(crate) fn fit_initial_blank_frame_to_actual_viewport(&mut self) -> bool {
        if !self.pending_initial_blank_frame_fit {
            return false;
        }
        if self.viewport_width <= 0.0 || self.viewport_height <= 0.0 {
            return false;
        }
        self.pending_initial_blank_frame_fit = false;
        if self.host.editor_state().doc != op_editor_core::EditorState::starter().doc {
            return false;
        }
        self.host
            .fit_content_to_viewport(self.viewport_width, self.viewport_height);
        self.host.mark_editor_state_dirty();
        true
    }

    /// Mark the live revision as the saved baseline — called after a
    /// synchronous load / save / new. The editor's monotonic revision token
    /// avoids serializing the whole document merely to decide whether it is
    /// dirty.
    pub(crate) fn mark_document_saved(&mut self) {
        // Any successful Save / Open / New replaced the document. If
        // a background Figma import is still running, its result
        // would later overwrite this fresh document in `pump` —
        // drop the session here so the worker's `send` becomes a
        // silent no-op when it finishes.
        figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
        html_import_session::cancel(&mut self.host, &mut self.current_html_import);
        // Pending clipboard-paste decodes are NOT cancelled here: this
        // runs on plain Save too, and a save must not discard a paste
        // the user is mid-way through. Attribution to the right
        // document is handled by the document-epoch guard in
        // `pump_*_clipboard_paste` — a paste decoded for a document
        // that a later Open / New / import replaced is dropped there.
        self.image_search.reset();
        self.host.editor_state_mut().mark_saved_revision();
        self.rebind_git_session_for_current_path();
    }

    /// Rebind the Git session to `current_path`, retitle the window
    /// and refresh an open Git panel — WITHOUT touching the
    /// unsaved-changes baseline. `mark_document_saved` calls this after a real
    /// save; import paths call it directly after explicitly marking their new
    /// state saved or dirty.
    pub(crate) fn rebind_git_session_for_current_path(&mut self) {
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
            panel.commit_input.set_text("");
            panel.defocus_commit_input(0);
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
    pub(crate) fn document_is_dirty(&self) -> bool {
        self.host.editor_state().is_dirty()
    }

    /// Confirm the adjacent `.op` destination before disturbing any active
    /// import. Cancel therefore leaves the current worker and document intact.
    pub(crate) fn begin_figma_import(&mut self, path: PathBuf) -> bool {
        if !self.host.gate_collaboration_action(
            op_editor_core::CollabGateAction::ReplaceDocument,
            op_editor_core::CollabEditSource::Import,
        ) {
            return false;
        }
        if self
            .current_figma_import
            .as_ref()
            .is_some_and(|session| session.path() == path.as_path())
        {
            return true;
        }
        let Some(output_mode) = figma_import_session::prompt_output_mode(&self.host, &path) else {
            return false;
        };
        figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
        html_import_session::cancel(&mut self.host, &mut self.current_html_import);
        self.current_figma_import = Some(figma_import_session::spawn_approved(
            &mut self.host,
            path,
            output_mode,
        ));
        self.request_redraw(true);
        true
    }

    /// Start an HTML/ZIP whole-document import after a current-phase gate.
    pub(crate) fn begin_html_import(&mut self, path: PathBuf) -> bool {
        if !self.host.gate_collaboration_action(
            op_editor_core::CollabGateAction::ReplaceDocument,
            op_editor_core::CollabEditSource::Import,
        ) {
            return false;
        }
        figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
        html_import_session::cancel(&mut self.host, &mut self.current_html_import);
        self.current_html_import = Some(html_import_session::spawn(&mut self.host, path));
        self.request_redraw(true);
        true
    }

    /// Open documents macOS delivered through the open-documents
    /// Apple event — a Finder double-click, `open file.op`, or a file
    /// dropped on the Dock icon. macOS routes these out-of-band of
    /// argv; the `casement` winit fork captures them and this drains
    /// the buffer. The single-window editor opens the first supported
    /// document and logs any extras. Returns `true` when a document
    /// was opened. A no-op on non-macOS (the buffer is always empty).
    pub(crate) fn drain_opened_files(&mut self) -> bool {
        #[cfg(target_os = "macos")]
        {
            let mut opened = false;
            for path in winit::platform::macos::drain_opened_file_urls() {
                let is_op = op_host_services::doc_io::is_supported_document(&path);
                let is_fig = op_host_services::doc_io::is_supported_figma_import(&path);
                let is_html = op_host_services::doc_io::is_supported_html_import(&path);
                if !is_op && !is_fig && !is_html {
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
                    // `.fig` → background import. An accepted destination
                    // marks this batch handled; cancelling the dialog leaves
                    // the current import untouched and allows the next path.
                    // `pump` applies accepted imports when the worker finishes.
                    opened = self.begin_figma_import(path);
                } else if is_html {
                    opened = self.begin_html_import(path);
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

    /// Drain documents forwarded by second-launch processes
    /// (`single_instance`) and open them in this window. Cross-platform
    /// analogue of `drain_opened_files` (which only covers the macOS
    /// Apple-event path). Returns true when a document was opened.
    pub(crate) fn drain_forwarded_files(&mut self) -> bool {
        let paths: Vec<PathBuf> = match self.forwarded_files.lock() {
            Ok(mut queue) => queue.drain(..).collect(),
            Err(_) => return false,
        };
        let mut opened = false;
        for path in paths {
            let is_op = op_host_services::doc_io::is_supported_document(&path);
            let is_fig = op_host_services::doc_io::is_supported_figma_import(&path);
            let is_html = op_host_services::doc_io::is_supported_html_import(&path);
            if (!is_op && !is_fig && !is_html) || !path.is_file() {
                continue;
            }
            // Single-window editor: the first forwarded document wins, the
            // rest are ignored (mirrors `drain_opened_files`).
            if opened {
                continue;
            }
            if is_fig {
                opened = self.begin_figma_import(path);
            } else if is_html {
                opened = self.begin_html_import(path);
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

    /// Bring the editor window to the foreground — used when a second launch
    /// forwards (or just pings) this instance so the user sees the document
    /// surface in the running window.
    pub(crate) fn raise_window(&self) {
        if let Some(window) = self.window.as_ref() {
            window.set_minimized(false);
            window.focus_window();
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
    pub(crate) fn confirm_document_reload(&mut self) -> bool {
        // Flush any in-progress text-input draft into the document
        // first — otherwise an unflushed draft would not count toward
        // `document_is_dirty` and the reload would drop it silently.
        self.host.commit_pending_input_pub();
        if self.save_session.is_active() && !self.finish_background_saves() {
            return false;
        }
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
                self.request_background_save() && self.finish_background_saves()
            }
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }

    pub(crate) fn confirm_close(&mut self) -> bool {
        // A Save-As may be writing an otherwise-clean document to a new path.
        // Finish it before the process can exit and abandon the worker.
        if self.save_session.is_active() && !self.finish_background_saves() {
            return false;
        }
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
                self.request_background_save() && self.finish_background_saves()
            }
            rfd::MessageDialogResult::No => true,
            _ => false,
        }
    }
}
