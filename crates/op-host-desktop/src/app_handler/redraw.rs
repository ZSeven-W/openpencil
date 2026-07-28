//! The `RedrawRequested` frame pump for `DesktopApp` — worker drains,
//! indicator updates, paint and the next-wake schedule. Carved out of
//! the `app_handler.rs` spine to keep it under the 800-line cap; pure
//! code motion.

use crate::{
    chat_session, codegen_session, design_session, figma_import_session, frame,
    html_import_session, DesktopApp,
};
use std::time::{Duration, Instant};
use winit::event_loop::ActiveEventLoop;

impl DesktopApp {
    /// Returns `false` when the frame bailed before doing any work (no
    /// render context yet) — the dispatcher then skips its post-event
    /// epilogue, exactly as the inlined `return` did.
    pub(super) fn on_redraw_requested(&mut self, event_loop: &ActiveEventLoop) -> bool {
        if (self.ctx.is_none() || self.backend.is_none())
            && !self.try_init_render_context(event_loop)
        {
            self.redraw_pending = false;
            self.redraw_dirty = true;
            return false;
        }
        if self.collab_runtime.refresh_availability(&mut self.host) {
            self.redraw_dirty = true;
        }
        let collaboration_transition_pending = matches!(
            self.host
                .editor_state()
                .editor_ui
                .collab
                .pending_action
                .as_ref(),
            Some(
                op_editor_core::CollabUiAction::Start
                    | op_editor_core::CollabUiAction::JoinDiscovered { .. }
                    | op_editor_core::CollabUiAction::JoinAddress { .. }
                    | op_editor_core::CollabUiAction::Retry
            )
        );
        let collaboration_transition_ready = if collaboration_transition_pending {
            self.settle_document_io_before_collaboration();
            self.settle_git_before_collaboration_transition()
        } else {
            true
        };
        if collaboration_transition_ready && self.collab_runtime.drain_ui_action(&mut self.host) {
            self.redraw_dirty = true;
        }
        if self.collab_runtime.take_save_as_fork_request() {
            self.request_background_save_as();
            self.redraw_dirty = true;
        }
        if self.collab_runtime.poll(&mut self.host) {
            self.redraw_dirty = true;
        }
        for status in self.collab_runtime.drain_status_events() {
            eprintln!("[collab] {status:?}");
        }
        let collab_cursor = self.host.canvas_doc_point(
            self.cursor_x,
            self.cursor_y,
            self.viewport_width,
            self.viewport_height,
        );
        if self
            .collab_runtime
            .publish_local_presence(&mut self.host, collab_cursor)
        {
            self.redraw_dirty = true;
        }
        // Route any accessibility action requests (#67) from the
        // screen reader back into host state before painting, so a
        // Focus / activation reflects in this frame.
        if let Some(a11y) = self.a11y.as_mut() {
            let actions = a11y.drain_actions();
            for action in actions {
                if self.host.apply_a11y_action(action.target, action.is_focus) {
                    self.redraw_dirty = true;
                }
            }
        }
        if self.drain_new_chat() {
            self.redraw_dirty = true;
        }
        if self.drain_stop_chat() {
            self.redraw_dirty = true;
        }
        if self.drain_close_chat_tab() {
            self.redraw_dirty = true;
        }
        // Pump in-flight AI chat deltas into this frame. The deltas
        // land in the tab the turn was bound to (MT.3 session-per-tab),
        // not whichever tab is active now.
        if chat_session::pump(
            &mut self.host,
            &mut self.current_chat,
            self.chat_running_tab,
            None,
            (self.viewport_width, self.viewport_height),
        ) {
            self.redraw_dirty = true;
        }
        // Update design-loop canvas indicators (frame glows + N/M
        // header) while an OPENPENCIL_DESIGN_AGENT_LOOP turn runs.
        crate::design_loop_indicator::pump_indicator(
            &mut self.design_loop_indicator,
            &self.current_chat,
            self.host.editor_state_mut(),
        );
        // Sub-agent design loops (Task 3.1): launch any specs the
        // parent loop just stashed via `spawn_agents`, then pump the
        // active sub SEQUENTIALLY. Runs AFTER `pump_indicator` so the
        // parent indicator teardown can't clobber the sub N/M count.
        // Borrow-clean: launch + pump each borrow `host` and
        // `sub_agents` separately (and `pump_sub_agents` takes the
        // active session out before pumping).
        if let Some(specs) = crate::sub_agent_session::take_pending_spawn() {
            // Guard against a parent calling `spawn_agents` a SECOND
            // time while batch A is still running: overwriting
            // `self.sub_agents` would drop batch A's in-flight sessions
            // and leak its active indicator epoch (forever-glow). The
            // nested guard only covers a sub re-spawning, not the
            // parent — so drop the re-spawn while a batch is live (the
            // stash is still consumed above, so it can't fire later).
            if self.sub_agents.is_empty() {
                self.sub_agents =
                    crate::sub_agent_session::launch_sub_agents(&mut self.host, specs);
                self.active_sub_agent = 0;
                if !self.sub_agents.is_empty() {
                    self.redraw_dirty = true;
                }
            }
        }
        if crate::sub_agent_session::pump_sub_agents(
            &mut self.host,
            &mut self.sub_agents,
            &mut self.active_sub_agent,
            self.chat_running_tab,
        ) {
            self.redraw_dirty = true;
        }
        // Drain a pending Cancel from the Code panel FIRST so a
        // canceled run is flagged before launch (a same-frame
        // Regenerate replaces it) and before pump (its remaining
        // deltas are dropped, never resurrecting the panel).
        if codegen_session::drain_codegen_cancel_request(&mut self.host, &mut self.current_codegen)
        {
            self.redraw_dirty = true;
        }
        // Launch a pending Generate / Regenerate from the Code
        // panel, then pump the in-flight codegen pipeline's
        // progress into `editor_state.codegen` this frame.
        if codegen_session::launch_codegen_if_pending(&mut self.host, &mut self.current_codegen) {
            self.redraw_dirty = true;
        }
        if codegen_session::pump(
            &mut self.host,
            &mut self.current_codegen,
            &mut self.codegen_results,
        ) {
            self.redraw_dirty = true;
        }
        if self.poll_design_md_generation() {
            self.redraw_dirty = true;
        }
        // Drain a pending Download / Export-Bundle from the Code
        // panel — pops a native save dialog + writes the file(s).
        if crate::codegen_export::drain_codegen_file_actions(&mut self.host, &self.codegen_results)
        {
            self.redraw_dirty = true;
        }
        // Drain a finished background `.fig` parse — applies
        // the imported document + clears the loading overlay
        // flag. Rebinds Git + window title on success
        // (matches the prior synchronous path's outcome).
        // Drain a finished Figma CLIPBOARD paste decode.
        if self.pump_figma_clipboard_paste() {
            self.redraw_dirty = true;
        }
        // Drain a finished HTML clipboard paste decode.
        if self.pump_html_clipboard_paste() {
            self.redraw_dirty = true;
        }
        match figma_import_session::pump(
            &mut self.host,
            &mut self.current_figma_import,
            &mut self.current_path,
            self.window.as_ref(),
        ) {
            figma_import_session::PumpOutcome::CompletedOk
            | figma_import_session::PumpOutcome::CompletedSaved => {
                self.rebind_git_session_for_current_path();
                // The import installed a fresh `EditorState` whose
                // revision restarts at 0 AND whose node ids can
                // collide with ids from the document just replaced.
                // A gate-only invalidation isn't enough: stale
                // `in_flight`/`completed` node ids could suppress a
                // same-id target in the new document, or a
                // still-pending job could apply its stale result to
                // a same-id node once it resolves. `reset()` drops
                // the whole session (sets + in-flight jobs + the
                // scan gate) so the new document starts clean.
                self.image_search.reset();
                self.redraw_dirty = true;
            }
            figma_import_session::PumpOutcome::CompletedErr => {
                self.redraw_dirty = true;
            }
            figma_import_session::PumpOutcome::SelectionReady
            | figma_import_session::PumpOutcome::Cancelled => {
                self.redraw_dirty = true;
            }
            figma_import_session::PumpOutcome::StillPending
            | figma_import_session::PumpOutcome::Idle => {}
        }
        match html_import_session::pump(
            &mut self.host,
            &mut self.current_html_import,
            &mut self.current_path,
            self.window.as_ref(),
        ) {
            figma_import_session::PumpOutcome::CompletedOk
            | figma_import_session::PumpOutcome::CompletedSaved => {
                self.rebind_git_session_for_current_path();
                // Same fresh-EditorState reasoning as the Figma
                // pump above: drop stale image-search state.
                self.image_search.reset();
                self.redraw_dirty = true;
            }
            figma_import_session::PumpOutcome::CompletedErr => {
                self.redraw_dirty = true;
            }
            figma_import_session::PumpOutcome::SelectionReady
            | figma_import_session::PumpOutcome::Cancelled => {}
            figma_import_session::PumpOutcome::Idle
            | figma_import_session::PumpOutcome::StillPending => {}
        }
        // A failed subtask row's "Retry" click raised
        // `chat.pending_subtask_retry` — launch the single-subtask
        // worker (failed-subtask remediation, manual layer) before
        // this same frame's pump drains its first commands/progress.
        if self.launch_subtask_retry_if_pending() {
            self.redraw_dirty = true;
        }
        // Drain orchestrator apply requests + progress events
        // for any in-flight design turn (orchestrator runs off
        // the UI thread; `RemoteDocSink` forwards mutations
        // here each frame).
        if design_session::pump_commands(
            &mut self.host,
            &mut self.current_design,
            self.viewport_width,
            self.viewport_height,
        ) {
            self.redraw_dirty = true;
        }
        if design_session::pump_progress(
            &mut self.host,
            &mut self.current_design,
            self.chat_running_tab,
        ) {
            self.redraw_dirty = true;
        }
        // Update design-orchestrator canvas indicators (frame glows
        // + scan) — the `current_design` counterpart to
        // `pump_indicator` above. Runs AFTER `pump_commands` /
        // `pump_progress` so a same-frame turn-finish (which clears
        // `current_design`) is observed here, mirroring how
        // `pump_indicator` runs after `chat_session::pump`.
        crate::design_loop_indicator::pump_design_session_indicator(
            &mut self.design_session_indicator,
            &self.current_design,
            self.host.editor_state_mut(),
            self.chat_running_tab,
        );
        // Each pump retires its session when the turn finishes — once
        // no chat / design / sub-agent run remains in flight, the tab
        // binding is stale, so clear it (a fresh turn re-captures the
        // active tab at launch).
        if self.current_chat.is_none()
            && self.current_design.is_none()
            && self.sub_agents.is_empty()
        {
            self.chat_running_tab = None;
        }
        // Starter ghost: painted from the moment a design prompt
        // clears the blank starter until the generated design's root
        // lands (or the turn dies with nothing produced).
        let session_running = self.current_chat.is_some() || self.current_design.is_some();
        self.persist_connection_changes();
        self.persist_ui_pref_changes();
        if crate::chat_session::reconcile_starter_ghost(
            self.host.editor_state_mut(),
            session_running,
        ) {
            self.host.mark_editor_state_dirty();
            self.redraw_dirty = true;
        }
        let (editor_state, layout_scene) = self.host.editor_state_mut_and_layout_scene();
        self.image_search
            .enqueue_missing_with_scene(editor_state, layout_scene);
        if self
            .image_search
            .poll_into_with_scene(editor_state, layout_scene)
        {
            self.host.mark_editor_state_dirty();
            self.redraw_dirty = true;
        }
        // Property-panel image section: asset check + Search /
        // Generate popover jobs.
        if self.image_panel.pump(&mut self.host, &self.current_path) {
            self.redraw_dirty = true;
        }
        // Drain paint-recorded remote image misses → spawn
        // fetches; store landed bytes into the painter's
        // shared cache so this frame (or the next) draws them.
        if self.remote_images.pump() {
            self.redraw_dirty = true;
        }
        if self.collab_avatars.pump() {
            self.redraw_dirty = true;
        }
        if let Some(backend) = self.backend.as_mut() {
            if self.image_decodes.pump(backend) {
                self.redraw_dirty = true;
            }
        }
        // Drain background model discovery once it lands.
        if self.model_probe.poll_into(self.host.editor_state_mut()) {
            self.host.mark_editor_state_dirty();
            self.redraw_dirty = true;
        }
        // #20 theme presets — persist the app-level preset
        // list when a save / delete / rename marked it dirty.
        crate::theme_preset_host::persist_if_dirty(self.host.editor_state_mut());
        if self.drain_iconify_picker() {
            self.redraw_dirty = true;
        }
        // Drain the connect-time provider probe (Settings →
        // Agents → Connect) — spawn requested probes, land
        // finished ones into agent_settings + the model
        // catalog.
        if self.drain_provider_connect() {
            self.redraw_dirty = true;
        }
        if self.drain_acp_agent_connect() {
            self.redraw_dirty = true;
        }
        // Drain the background auto-update probe.
        if self.poll_update_probe() {
            self.redraw_dirty = true;
        }
        // Drain the browser device-login flow (status + browser
        // opens land here).
        if self.poll_auth_flow() {
            self.redraw_dirty = true;
        }
        // Drain a finished background `git pull`.
        if self.poll_git_pull_job() {
            self.redraw_dirty = true;
        }
        // Drain a finished background `git push`.
        if self.poll_git_push_job() {
            self.redraw_dirty = true;
        }
        // Drain a finished background Git status query.
        if self.poll_git_status_job() {
            self.redraw_dirty = true;
        }
        // Drain a finished background Git diff.
        if self.poll_git_diff_job() {
            self.redraw_dirty = true;
        }
        // Drain a finished background `git clone`.
        if self.poll_git_clone_job() {
            self.redraw_dirty = true;
        }
        // Drain live MCP requests. Write tools must apply on the
        // UI-owned EditorState so canvas state, history and
        // selection stay canonical.
        if self.poll_mcp_server() {
            self.redraw_dirty = true;
        }
        // A token-authed `op stop` asked the live MCP server to quit
        // — exit the event loop cleanly (runs `exiting()`, saving
        // window state + settings and removing the discovery file).
        if self.mcp_shutdown_requested() {
            // Finalize-lifecycle invariant (0718-1-k3-1 postmortem)
            // — see `chat_session::finalize_design_session_if_needed`'s
            // doc comment.
            chat_session::finalize_design_session_if_needed(
                &mut self.host,
                &self.current_chat,
                "teardown-backstop",
            );
            event_loop.exit();
        }
        // Keep an open Git panel fresh against external repo
        // changes — re-request a snapshot at most every 2 s.
        // The query runs on a worker thread, so this never
        // blocks the UI, however large the repository.
        if self.host.editor_state().editor_ui.git_panel.open
            && self.last_git_refresh.elapsed() >= Duration::from_secs(2)
        {
            self.last_git_refresh = Instant::now();
            self.refresh_git_panel();
        }
        // Refresh the fullscreen flag every frame — the
        // macOS fullscreen-exit transition can land its
        // final `Resized` before `window.fullscreen()`
        // flips, so the `Resized` handler alone could miss
        // the exit. Polling here self-corrects so the
        // TopBar's traffic-light reservation is restored.
        let fullscreen = self
            .window
            .as_ref()
            .is_some_and(|w| w.fullscreen().is_some());
        if self.host.editor_state().editor_ui.window_fullscreen != fullscreen {
            self.host.editor_state_mut().editor_ui.window_fullscreen = fullscreen;
            self.host.mark_editor_state_dirty();
            self.redraw_dirty = true;
        }
        let should_paint = self.prepare_redraw();
        if should_paint {
            self.refresh_host_clock();
            let mut painted = false;
            // `OP_SLOW_FRAME_LOG=1` prints every paint slower
            // than ~2 frames with pan-cache attribution, so a
            // felt stutter can be pinned to blit / scroll /
            // full-paint work on a live session.
            let slow_log = std::env::var_os("OP_SLOW_FRAME_LOG").is_some();
            let frame_start = slow_log.then(Instant::now);
            let stats_before = slow_log.then(|| self.host.pan_cache_stats());
            if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
                frame::paint(
                    ctx,
                    backend,
                    &mut self.host,
                    self.viewport_width,
                    self.viewport_height,
                    self.dpi,
                );
                painted = true;
            }
            if let (Some(start), Some((blits0, scrolls0, builds0))) = (frame_start, stats_before) {
                let elapsed = start.elapsed();
                if elapsed.as_millis() >= 32 {
                    let (blits, scrolls, builds) = self.host.pan_cache_stats();
                    eprintln!(
                        "[slow-frame] {:>4}ms degrade={} blit={} scroll={} build={}",
                        elapsed.as_millis(),
                        self.host.interaction_degrade_active(),
                        blits - blits0,
                        scrolls - scrolls0,
                        builds - builds0,
                    );
                }
            }
            if painted {
                let page = self.active_page_paint_identity();
                if self.last_painted_page.as_ref() != Some(&page) {
                    self.last_painted_page = Some(page);
                    crate::heap_pressure::schedule_relief("page first paint");
                }
            }
            // Paint publishes exact input geometry. Refresh the OS
            // candidate anchor now, before any future Preedit event.
            self.sync_native_ime();
            // Republish the accessibility tree alongside the
            // painted frame so the screen reader's view tracks
            // the visible editor state (#67). The tree build
            // (including the O(nodes) `LayerPanel` walk) is
            // deferred into this closure, which `DesktopA11y`
            // only invokes from inside `update_if_active` — i.e.
            // only when assistive tech is actually attached, so
            // ordinary painted frames skip the walk entirely.
            if let Some(a11y) = self.a11y.as_mut() {
                let viewport_width = self.viewport_width;
                let viewport_height = self.viewport_height;
                let host = &mut self.host;
                a11y.push(move || host.accessibility_tree_update(viewport_width, viewport_height));
            }
        }
        self.schedule_next_wake(event_loop);
        true
    }
}
