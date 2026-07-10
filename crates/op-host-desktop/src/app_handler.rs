//! winit `ApplicationHandler` impl for `DesktopApp`. Split out of
//! `main.rs` to keep that file under the 800-line cap.

use crate::{
    a11y, chat_attachment, chat_session, codegen_session, cursor_icon, design_session,
    figma_import_session, frame, git_jobs, menu, persistence, window_state, DesktopApp,
    DesktopEvent, INITIAL_VIEWPORT_H, INITIAL_VIEWPORT_W,
};
use op_host_native::{NativeBackend, ProviderError, SharedSkiaContext, SharedSkiaError};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{
    ElementState, KeyEvent, MouseButton, MouseScrollDelta, StartCause, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

impl ApplicationHandler<DesktopEvent> for DesktopApp {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        self.refresh_host_clock();
        // Live MCP requests must not wait for `RedrawRequested`. Window systems
        // may throttle redraws while a window is occluded or while a previous
        // paint is expensive, but CLI/MCP snapshot/apply acks should still be
        // drained as soon as the event loop wakes. Applies mark the document
        // dirty and schedule a paint; snapshots just ack with no repaint.
        if self.poll_mcp_server() {
            self.request_redraw(true);
        }
        if self.mcp_shutdown_requested() {
            event_loop.exit();
            return;
        }
        // When a WaitUntil deadline fires, only timed UI activity needs a paint
        // (caret blink, streaming chat, imports, background jobs). Live MCP uses
        // `DesktopEvent::McpWake`, so an idle server no longer creates timer
        // ticks just to poll for possible requests.
        if self.timed_wake_needs_redraw(&cause) && self.resume_time_needs_redraw() {
            self.request_redraw(true);
        }
        // Native-menu selections arrive on `muda`'s global channel.
        // A menu click wakes the event loop, so draining here — at
        // the top of each loop iteration — picks them up promptly.
        while let Some(action) = menu::poll() {
            self.handle_menu_action(action, event_loop);
        }
        // macOS open-documents Apple event — a Finder double-click /
        // `open file` / Dock drop on the already-running app. The
        // AppKit event wakes the loop, so draining here is prompt.
        if self.drain_opened_files() {
            self.request_redraw(true);
        }
        // Keep the native File ▸ Open Recent submenu in sync with the recent
        // list every iteration — cheap (rebuilds only on change) and catches
        // in-canvas File-menu opens/saves that never reach `handle_menu_action`.
        self.refresh_recent_menu();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let mut attrs = Window::default_attributes()
            .with_title("OpenPencil")
            .with_inner_size(winit::dpi::LogicalSize::new(
                INITIAL_VIEWPORT_W as u32,
                INITIAL_VIEWPORT_H as u32,
            ))
            .with_min_inner_size(winit::dpi::LogicalSize::new(640.0f64, 400.0f64));
        // Hide the title bar but keep the platform's own window
        // controls — the Electron `titleBarStyle: 'hidden'` recipe.
        //
        // macOS: a transparent, emptied title bar over a normal
        // `NSWindow` — the native traffic-light buttons stay (with
        // their native hover glyphs + green-button tiling menu),
        // and rounded corners / shadow / edge-resize / key-window
        // responsiveness all come for free. The TopBar insets its
        // left cluster so the app icons clear the native buttons.
        // Windows / Linux drop decorations (custom dots cover them).
        #[cfg(target_os = "macos")]
        {
            use winit::platform::macos::WindowAttributesExtMacOS;
            // Push the traffic lights down so their centre lines up with
            // the `TopBar`'s app icon GLYPHS. The icon buttons paint their
            // glyph centred at `TOP_BAR_HEIGHT / 2` = 20 px below the
            // window top (the 28 px hit box top+8 is wider than the glyph
            // and is NOT the visual centre). AppKit's default button centre
            // for a fullsize-content / transparent titlebar window is
            // `casement`'s `reposition_traffic_lights` lowers the button
            // by `inset` points from AppKit's default baseline. The
            // geometric centre is ~6, but by eye the user wants more top
            // margin so the dots drop onto the icon row. The casement
            // fork now applies this on `windowDidBecomeKey` (it was a
            // no-op at window creation — the buttons didn't exist yet,
            // so the inset only took effect after a resize). 4 px lands
            // the dots on the icon-glyph row (tuned by eye with the user).
            attrs = attrs
                .with_titlebar_transparent(true)
                .with_fullsize_content_view(true)
                .with_title_hidden(true)
                .with_traffic_light_inset(4.0);
        }
        #[cfg(not(target_os = "macos"))]
        {
            attrs = attrs.with_decorations(false);
        }
        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::{Color, CornerPreference, WindowAttributesExtWindows};
            attrs = attrs
                .with_corner_preference(CornerPreference::Round)
                .with_border_color(Some(Color::from_rgb(0x31, 0x31, 0x31)));
        }
        // Restore the window geometry from the previous session
        // (position / size / maximized). A missing or stale file
        // leaves the default attrs untouched.
        let saved_geometry = window_state::load();
        if let Some(saved) = saved_geometry.as_ref() {
            attrs = saved.apply_to(attrs);
        }
        // Windows only: create the window hidden so the accessibility
        // subclassing adapter (#67) can attach BEFORE the HWND is shown —
        // `accesskit_windows::SubclassingAdapter::new` panics on an already
        // visible window. `set_visible(true)` runs once the adapter is in.
        // macOS / Linux create visible as before (their adapters don't care).
        #[cfg(target_os = "windows")]
        {
            attrs = attrs.with_visible(false);
        }
        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(err) => {
                eprintln!("openpencil-desktop: create_window failed: {err}");
                event_loop.exit();
                return;
            }
        };
        // Enable IME so macOS / X11 / Wayland route Chinese / Japanese /
        // Korean composition through `WindowEvent::Ime` instead of
        // dropping the keystrokes.
        window.set_ime_allowed(true);

        let dpi = window.scale_factor() as f32;
        self.dpi = dpi;
        // Build the curved-arrow rotate cursor once and cache it.
        let (rgba, w, h, hx, hy) = cursor_icon::make_rotate_cursor_rgba();
        match winit::window::CustomCursor::from_rgba(rgba, w, h, hx, hy) {
            Ok(source) => {
                self.rotate_cursor = Some(event_loop.create_custom_cursor(source));
            }
            Err(err) => {
                eprintln!("openpencil-desktop: rotate cursor build failed: {err}");
            }
        }

        self.window = Some(window);

        if let Some(window) = self.window.as_ref() {
            let size = window.inner_size();
            self.viewport_width = size.width as f32 / self.dpi;
            self.viewport_height = size.height as f32 / self.dpi;
        }
        self.fit_initial_blank_frame_to_actual_viewport();

        // Build + attach the native menu bar now that the NSApp /
        // window exists. macOS attaches to the shared NSApp;
        // Windows to this window; Linux is a no-op.
        if let Some(window) = self.window.as_ref() {
            self.app_menu = Some(menu::AppMenu::install(window));
            // Seed File ▸ Open Recent from the recent list loaded at startup.
            self.refresh_recent_menu();
        }

        // Build the OS accessibility bridge (#67) from the window's raw
        // handle and publish the initial tree so a screen reader sees the
        // editor's regions immediately (subsequent frames push fresh
        // trees from `RedrawRequested`).
        if let Some(window) = self.window.as_ref() {
            // Same cross-thread wake-up mechanism live MCP requests use
            // (`mcp_runtime.rs::mcp_wake_callback`): the activation
            // handler may run off the render thread and can't repaint
            // directly, so it sends a `DesktopEvent` that
            // `user_event` turns into `request_redraw(true)`.
            let wake_proxy = self.mcp_wake_proxy.clone();
            let wake = move || {
                if let Some(proxy) = wake_proxy.as_ref() {
                    let _ = proxy.send_event(DesktopEvent::A11yActivated);
                }
            };
            let mut a11y = a11y::DesktopA11y::new(window, wake);
            let viewport_width = self.viewport_width;
            let viewport_height = self.viewport_height;
            let host = &mut self.host;
            a11y.push(move || host.accessibility_tree_update(viewport_width, viewport_height));
            self.a11y = Some(a11y);
        }

        // Windows: now that the a11y adapter is attached to the hidden HWND,
        // reveal the window (see the `with_visible(false)` note above).
        #[cfg(target_os = "windows")]
        if let Some(window) = self.window.as_ref() {
            window.set_visible(true);
        }

        // Seed the window-geometry tracking. A restored maximized
        // window keeps the saved *windowed* position / size so
        // un-maximizing later lands somewhere sensible; otherwise
        // the tracking starts from the live window.
        // Prefer the saved maximized flag over `Window::is_maximized`:
        // a `with_maximized(true)` attr can apply asynchronously, so
        // the live window may still report `false` right after
        // creation.
        self.win_maximized = match saved_geometry.as_ref() {
            Some(saved) => {
                self.win_pos = saved.pos();
                self.win_size = saved.size();
                saved.maximized()
            }
            None => self.window.as_ref().is_some_and(Window::is_maximized),
        };
        if !self.win_maximized {
            if let Some(window) = self.window.as_ref() {
                if let Ok(pos) = window.outer_position() {
                    self.win_pos = Some((pos.x, pos.y));
                }
                let size = window.inner_size();
                self.win_size = Some((size.width, size.height));
            }
        }

        // Guard a restored position against a monitor layout change:
        // a window saved on a since-disconnected display can reopen
        // off-screen. If it has, recentre it on the primary monitor.
        let restored_position = saved_geometry
            .as_ref()
            .and_then(window_state::WindowState::pos)
            .is_some();
        if restored_position && !self.win_maximized {
            if let Some(window) = self.window.as_ref() {
                let monitors: Vec<window_state::MonitorRect> = event_loop
                    .available_monitors()
                    .map(|m| {
                        let p = m.position();
                        let s = m.size();
                        ((p.x, p.y), (s.width, s.height))
                    })
                    .collect();
                let size = window.outer_size();
                if let Ok(pos) = window.outer_position() {
                    let visible = window_state::rect_visible_on_monitors(
                        (pos.x, pos.y),
                        (size.width, size.height),
                        &monitors,
                    );
                    if !visible {
                        if let Some(primary) = event_loop
                            .primary_monitor()
                            .or_else(|| event_loop.available_monitors().next())
                        {
                            let mp = primary.position();
                            let ms = primary.size();
                            let centered = window_state::centered_on(
                                (size.width, size.height),
                                ((mp.x, mp.y), (ms.width, ms.height)),
                            );
                            window.set_outer_position(winit::dpi::PhysicalPosition::new(
                                centered.0, centered.1,
                            ));
                            self.win_pos = Some(centered);
                        }
                    }
                }
            }
        }

        // File-association launch path: open the document handed in
        // via argv now that the host + window are ready. Routes `.op`
        // / `.pen` through `open_path`; `.fig` goes through the
        // background Figma import worker so the launch doesn't freeze
        // on a multi-second parse.
        if let Some(path) = self.initial_file.take() {
            if op_host_services::doc_io::is_supported_figma_import(&path) {
                figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
                self.current_figma_import = Some(figma_import_session::spawn(&mut self.host, path));
                self.request_redraw(true);
            } else if persistence::open_path(
                &mut self.host,
                path,
                &mut self.current_path,
                self.window.as_ref(),
            ) {
                self.mark_document_saved();
            }
        }
        // macOS Finder-launch path: a double-clicked document arrives
        // through the open-documents Apple event (captured by the
        // `casement` winit fork), not argv — drain it before the
        // first paint so the launch document shows immediately.
        self.drain_opened_files();

        // `op start` launches us with `--live-mcp[=port]`. The force flag is
        // honored directly by `reconcile_mcp_server_from_settings` (via
        // `force_live_mcp_port`) WITHOUT mutating or persisting the user's
        // MCP settings — mirroring TS's always-on editor MCP sync — so a
        // one-off `op start` never rewrites the user's saved settings.
        let bootstrap_changed = self.bootstrap_mcp_runtime_from_settings();
        if bootstrap_changed && self.force_live_mcp_port.is_none() {
            op_host_services::settings_io::save(self.host.editor_state());
        }
        // Publish (or clean up) the live MCP discovery file now that the
        // launch-time server state is settled, so `op` can find this canvas.
        self.publish_live_mcp_port();

        if self.try_init_render_context(event_loop) {
            if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
                frame::paint(
                    ctx,
                    backend,
                    &mut self.host,
                    self.viewport_width,
                    self.viewport_height,
                    self.dpi,
                );
            }
        }

        // Startup background probes are likely still running. Wake
        // the loop soon so their results are drained even if the user
        // never touches the freshly opened window.
        if self.update_probe.is_pending() || self.model_probe.is_pending() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(500),
            ));
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: DesktopEvent) {
        match event {
            DesktopEvent::McpWake => {
                if self.poll_mcp_server() {
                    self.request_redraw(true);
                }
                if self.mcp_shutdown_requested() {
                    event_loop.exit();
                }
            }
            DesktopEvent::ForwardedFileReady => {
                if self.drain_forwarded_files() {
                    self.request_redraw(true);
                }
                // Raise the window even for a bare ping (no path) so a second
                // launch surfaces the running editor.
                self.raise_window();
            }
            DesktopEvent::A11yActivated => {
                // Assistive tech just attached (see `a11y.rs`'s
                // `CachedTreeActivation::request_initial_tree`, which sent
                // this event). The app may have been fully idle
                // (`ControlFlow::Wait`, no dirty frame), so force a real
                // repaint here — the `RedrawRequested` handler's a11y push
                // then republishes a current, full tree.
                self.request_redraw(true);
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Refresh the host's monotonic clock at the top of every
        // WindowEvent so `apply_press` / `apply_text` /
        // `apply_backspace` etc. stamp `caret_anchor_ms` with the
        // CURRENT timestamp, not the one captured at the previous
        // RedrawRequested.
        self.refresh_host_clock();
        // CursorMoved never changes persisted prefs — skip snapshot
        // on the trackpad hot path.
        let settings_before = match &event {
            WindowEvent::CursorMoved { .. } => None,
            _ => Some(op_host_services::settings_io::fingerprint(
                self.host.editor_state(),
            )),
        };
        let mcp_cli_before = match &event {
            WindowEvent::CursorMoved { .. } => None,
            _ => {
                let settings = &self.host.editor_state().editor_ui.agent_settings;
                Some((settings.mcp_cli_enabled, settings.mcp_server.port))
            }
        };
        match event {
            WindowEvent::CloseRequested => {
                // The unsaved-changes prompt can abort the close.
                if self.confirm_close() {
                    self.host.flush_settings_input();
                    op_host_services::settings_io::save(self.host.editor_state());
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(ctx) = self.ctx.as_mut() {
                    if let Err(err) = ctx.resize(size.width, size.height) {
                        eprintln!("openpencil-desktop: resize failed: {err}");
                        self.error = Some(err);
                        event_loop.exit();
                    }
                } else {
                    self.try_init_render_context(event_loop);
                }
                self.viewport_width = size.width as f32 / self.dpi;
                self.viewport_height = size.height as f32 / self.dpi;
                // Re-lay-out the preview runtime (if active) against the
                // new canvas region so the live scene tracks the resize.
                self.host
                    .preview_resize(self.viewport_width, self.viewport_height);
                // Track geometry for window-state persistence. Only a
                // *windowed* (non-maximized) size is remembered so a
                // restart restores a sensible un-maximize size.
                self.win_maximized = self.window.as_ref().is_some_and(Window::is_maximized);
                if !self.win_maximized {
                    self.win_size = Some((size.width, size.height));
                }
                // Fullscreen enter / exit arrives as a `Resized` on
                // macOS — refresh the flag so the TopBar drops its
                // traffic-light reservation when the native lights
                // hide in fullscreen.
                let fullscreen = self
                    .window
                    .as_ref()
                    .is_some_and(|w| w.fullscreen().is_some());
                if self.host.editor_state().editor_ui.window_fullscreen != fullscreen {
                    self.host.editor_state_mut().editor_ui.window_fullscreen = fullscreen;
                    self.host.mark_editor_state_dirty();
                }
                self.request_redraw(true);
            }
            WindowEvent::Moved(pos) => {
                // Remember the windowed position for window-state
                // persistence; skip while maximized so the restored
                // position is the user's chosen spot.
                if !self.window.as_ref().is_some_and(Window::is_maximized) {
                    self.win_pos = Some((pos.x, pos.y));
                }
            }
            WindowEvent::HoveredFile(_path) => {
                // A file is being dragged over the window — show the
                // full-canvas drop overlay so the target is obvious.
                if !self.host.editor_state().editor_ui.file_drop_active {
                    self.host.editor_state_mut().editor_ui.file_drop_active = true;
                    self.host.mark_editor_state_dirty();
                    self.request_redraw(true);
                }
            }
            WindowEvent::HoveredFileCancelled => {
                // The drag left the window without dropping — hide it.
                if self.host.editor_state().editor_ui.file_drop_active {
                    self.host.editor_state_mut().editor_ui.file_drop_active = false;
                    self.host.mark_editor_state_dirty();
                    self.request_redraw(true);
                }
            }
            WindowEvent::DroppedFile(path) => {
                // Clear the drag overlay now that the drop has landed.
                self.host.editor_state_mut().editor_ui.file_drop_active = false;
                self.host.mark_editor_state_dirty();
                // Drag-and-drop open. `.op` / `.pen` documents route
                // through the canonical loader; `.fig` Figma exports
                // route through the background Figma import worker
                // (the parse + layout pass takes seconds for large
                // dashboards, so doing it inline would freeze the
                // window). Anything else is ignored silently so a
                // stray drop can't disrupt the current document.
                if op_host_services::doc_io::is_supported_figma_import(&path) {
                    figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
                    self.current_figma_import =
                        Some(figma_import_session::spawn(&mut self.host, path));
                    self.request_redraw(true);
                } else if op_host_services::doc_io::is_supported_document(&path) {
                    if persistence::open_path(
                        &mut self.host,
                        path,
                        &mut self.current_path,
                        self.window.as_ref(),
                    ) {
                        self.mark_document_saved();
                        self.request_redraw(true);
                    }
                } else {
                    eprintln!(
                        "openpencil-desktop: ignored dropped file (not .op / .pen / .fig): {}",
                        path.display()
                    );
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.dpi = scale_factor as f32;
                if let Some(backend) = self.backend.as_mut() {
                    backend.set_dpi(scale_factor as f32);
                }
                // Logical = physical / dpi; refresh to keep input + paint coords in sync after a DPI flip.
                if let Some(window) = self.window.as_ref() {
                    let size = window.inner_size();
                    self.viewport_width = size.width as f32 / self.dpi;
                    self.viewport_height = size.height as f32 / self.dpi;
                }
                self.request_redraw(true);
            }
            WindowEvent::RedrawRequested => {
                if (self.ctx.is_none() || self.backend.is_none())
                    && !self.try_init_render_context(event_loop)
                {
                    self.redraw_pending = false;
                    self.redraw_dirty = true;
                    return;
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
                if codegen_session::drain_codegen_cancel_request(
                    &mut self.host,
                    &mut self.current_codegen,
                ) {
                    self.redraw_dirty = true;
                }
                // Launch a pending Generate / Regenerate from the Code
                // panel, then pump the in-flight codegen pipeline's
                // progress into `editor_state.codegen` this frame.
                if codegen_session::launch_codegen_if_pending(
                    &mut self.host,
                    &mut self.current_codegen,
                ) {
                    self.redraw_dirty = true;
                }
                if codegen_session::pump(
                    &mut self.host,
                    &mut self.current_codegen,
                    &mut self.codegen_last_result,
                ) {
                    self.redraw_dirty = true;
                }
                if self.poll_design_md_generation() {
                    self.redraw_dirty = true;
                }
                // Drain a pending Download / Export-Bundle from the Code
                // panel — pops a native save dialog + writes the file(s).
                if crate::codegen_export::drain_codegen_file_actions(
                    &mut self.host,
                    &self.codegen_last_result,
                ) {
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
                match figma_import_session::pump(
                    &mut self.host,
                    &mut self.current_figma_import,
                    &mut self.current_path,
                    self.window.as_ref(),
                ) {
                    figma_import_session::PumpOutcome::CompletedOk => {
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
                    figma_import_session::PumpOutcome::StillPending
                    | figma_import_session::PumpOutcome::Idle => {}
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
                self.image_search.enqueue_missing(self.host.editor_state());
                if self.image_search.poll_into(self.host.editor_state_mut()) {
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
                    if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
                        frame::paint(
                            ctx,
                            backend,
                            &mut self.host,
                            self.viewport_width,
                            self.viewport_height,
                            self.dpi,
                        );
                    }
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
                        a11y.push(move || {
                            host.accessibility_tree_update(viewport_width, viewport_height)
                        });
                    }
                }
                // Chat / design / Figma-import worker active → wake
                // ~10 fps to pump results and animate the loading
                // overlay's spinner. Chat + design need ~30 fps for
                // streaming deltas; Figma import is a one-shot result
                // but the overlay's spinner needs frames to animate.
                if self.current_chat.is_some()
                    || self.current_design.is_some()
                    || self.current_codegen.is_some()
                    || self.current_design_md.is_some()
                    || !self.sub_agents.is_empty()
                {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + Duration::from_millis(33),
                    ));
                } else if self.current_figma_import.is_some() || self.pending_figma_paste.is_some()
                {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + Duration::from_millis(100),
                    ));
                } else if let Some(deadline_ms) = self.host.next_animation_deadline_ms() {
                    let deadline = self.clock_start + Duration::from_millis(deadline_ms);
                    event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                } else if self.update_probe.is_pending()
                    || self.model_probe.is_pending()
                    || self.image_search.is_pending()
                    || self.image_panel.is_pending()
                    // Remote-image fetches in flight, or fresh misses
                    // the paint above just recorded (the next pump
                    // picks them up).
                    || self.remote_images.is_pending()
                    || op_editor_ui::widgets::canvas_viewport_image::has_pending_remote_image_requests()
                    || self
                        .iconify_job
                        .as_ref()
                        .is_some_and(crate::iconify_host::IconifyJob::is_pending)
                    || self.provider_connect_pending()
                    || self.acp_agent_connect_pending()
                    || self
                        .git_pull_job
                        .as_ref()
                        .is_some_and(git_jobs::GitPullJob::is_pending)
                    || self
                        .git_push_job
                        .as_ref()
                        .is_some_and(git_jobs::GitPushJob::is_pending)
                    || self
                        .git_status_job
                        .as_ref()
                        .is_some_and(git_jobs::GitStatusJob::is_pending)
                    || self
                        .git_diff_job
                        .as_ref()
                        .is_some_and(git_jobs::GitDiffJob::is_pending)
                {
                    // Keep waking ~2 Hz until background probes/jobs
                    // land so results are drained even while the app
                    // is idle.
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + Duration::from_millis(500),
                    ));
                } else if self.host.editor_state().editor_ui.git_panel.open {
                    // While the Git panel is open, wake every 2 s for
                    // the periodic repository re-snapshot above.
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + Duration::from_secs(2),
                    ));
                } else {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x as f32 / self.dpi;
                self.cursor_y = position.y as f32 / self.dpi;
                let over_layer_panel = self.host.cursor_over_layer_panel(
                    self.cursor_x,
                    self.cursor_y,
                    self.viewport_width,
                    self.viewport_height,
                );
                if let Some(window) = self.window.as_ref() {
                    // Borderless (Windows / Linux) windows have no OS-provided
                    // edge-resize band — synthesize a resize cursor over the
                    // outer ring, ahead of every panel / canvas hint. macOS keeps
                    // its native decorations, so it's left untouched.
                    #[cfg(not(target_os = "macos"))]
                    if !self.host.is_dragging_node() && !window.is_maximized() {
                        let vw = window.inner_size().width as f32 / self.dpi;
                        let vh = window.inner_size().height as f32 / self.dpi;
                        if let Some(dir) = crate::window_resize::window_resize_direction(
                            self.cursor_x,
                            self.cursor_y,
                            vw,
                            vh,
                        ) {
                            window.set_cursor(winit::window::CursorIcon::from(dir));
                            self.pending_cursor_move = Some((self.cursor_x, self.cursor_y));
                            self.request_redraw(false);
                            return;
                        }
                    }
                    if self.host.is_dragging_node() || over_layer_panel {
                        window.set_cursor(winit::window::CursorIcon::Default);
                    } else {
                        // `cursor_hint` hit-tests the layout-resolved render
                        // scene. A mutation since the last paint may have left
                        // the scene stale (`editor_state_dirty`), so refresh it
                        // only when the pointer is outside the LayerPanel and a
                        // canvas/overlay hint could need scene geometry.
                        let _ = self.host.layout_scene();
                        let viewport_w = window.inner_size().width as f32 / self.dpi;
                        let viewport_h = window.inner_size().height as f32 / self.dpi;
                        let hint = self.host.cursor_hint(
                            self.cursor_x,
                            self.cursor_y,
                            viewport_w,
                            viewport_h,
                        );
                        use op_host_native::CursorHint;
                        if matches!(hint, CursorHint::Rotate) {
                            if let Some(c) = self.rotate_cursor.as_ref() {
                                window.set_cursor(winit::window::Cursor::Custom(c.clone()));
                            } else {
                                window.set_cursor(winit::window::CursorIcon::Grabbing);
                            }
                        } else {
                            let icon = match hint {
                                CursorHint::Default => winit::window::CursorIcon::Default,
                                CursorHint::Pointer => winit::window::CursorIcon::Pointer,
                                CursorHint::NotAllowed => winit::window::CursorIcon::NotAllowed,
                                CursorHint::Move => winit::window::CursorIcon::Move,
                                CursorHint::Grab => winit::window::CursorIcon::Grab,
                                CursorHint::Grabbing => winit::window::CursorIcon::Grabbing,
                                CursorHint::Crosshair => winit::window::CursorIcon::Crosshair,
                                CursorHint::Text => winit::window::CursorIcon::Text,
                                CursorHint::ResizeEw => winit::window::CursorIcon::EwResize,
                                CursorHint::ResizeNs => winit::window::CursorIcon::NsResize,
                                CursorHint::ResizeNwse => winit::window::CursorIcon::NwseResize,
                                CursorHint::ResizeNesw => winit::window::CursorIcon::NeswResize,
                                CursorHint::Rotate => unreachable!(),
                            };
                            window.set_cursor(icon);
                        }
                    }
                }
                // Coalesce cursor moves — apply once per redraw, not per 1000 Hz input event.
                self.pending_cursor_move = Some((self.cursor_x, self.cursor_y));
                self.request_redraw(false);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                // Drain queued cursor move so hover state is current before press lands.
                if self.drain_pending_cursor_move() {
                    self.redraw_dirty = true;
                }
                // Borderless-window edge resize (Windows / Linux). A press on the
                // outer ring hands the drag to the OS, ahead of the TopBar
                // window-drag and every app hit-test. macOS keeps native edges.
                #[cfg(not(target_os = "macos"))]
                if let Some(w) = self.window.as_ref() {
                    if !w.is_maximized() {
                        let vw = w.inner_size().width as f32 / self.dpi;
                        let vh = w.inner_size().height as f32 / self.dpi;
                        if let Some(dir) = crate::window_resize::window_resize_direction(
                            self.cursor_x,
                            self.cursor_y,
                            vw,
                            vh,
                        ) {
                            let _ = w.drag_resize_window(dir);
                            return;
                        }
                    }
                }
                // Custom window chrome — the native title bar is
                // hidden, so a press on the TopBar's window-control
                // dots drives the window, and a press on the bar's
                // blank area starts a window drag.
                if self.cursor_y < op_editor_ui::widgets::TOP_BAR_HEIGHT {
                    use op_editor_ui::widgets::{TopBar, WindowControl};
                    use op_editor_ui::{Point2D, Rect};
                    let tb_rect = Rect {
                        origin: Point2D::new(0.0, 0.0),
                        size: Point2D::new(
                            self.viewport_width,
                            op_editor_ui::widgets::TOP_BAR_HEIGHT,
                        ),
                    };
                    let tb = TopBar::for_editor_ui(&self.host.editor_state().editor_ui);
                    let p = Point2D::new(self.cursor_x, self.cursor_y);
                    if let Some(ctl) = tb.window_control_at(tb_rect, p) {
                        match ctl {
                            WindowControl::Close => {
                                // Mirror `WindowEvent::CloseRequested`:
                                // the unsaved-changes prompt can abort,
                                // and settings flush + save before exit
                                // — a bare `exit()` would drop work.
                                if self.confirm_close() {
                                    self.host.flush_settings_input();
                                    op_host_services::settings_io::save(self.host.editor_state());
                                    event_loop.exit();
                                }
                            }
                            WindowControl::Minimize => {
                                if let Some(w) = self.window.as_ref() {
                                    w.set_minimized(true);
                                }
                            }
                            WindowControl::Maximize => {
                                if let Some(w) = self.window.as_ref() {
                                    w.set_maximized(!w.is_maximized());
                                }
                            }
                        }
                        return;
                    }
                    // A press on the bar that hits none of the app's
                    // own buttons is a window-drag grab.
                    if tb.hit_test(tb_rect, p).is_none() {
                        if let Some(w) = self.window.as_ref() {
                            let _ = w.drag_window();
                        }
                        return;
                    }
                }
                let consumed = self.host.apply_press(
                    self.cursor_x,
                    self.cursor_y,
                    self.viewport_width,
                    self.viewport_height,
                );
                // TopBar fullscreen button raised an intent — toggle the
                // real window through the shared menu-action path.
                if self.host.editor_state().editor_ui.pending_fullscreen_toggle {
                    self.host
                        .editor_state_mut()
                        .editor_ui
                        .pending_fullscreen_toggle = false;
                    self.handle_menu_action(crate::menu::MenuAction::ToggleFullscreen, event_loop);
                }
                if let Some(text) = self.host.editor_state_mut().chat.pending_copy_text.take() {
                    crate::clipboard::set_text(&text);
                }
                // A click on the chat Send button raises
                // `pending_send` — launch the provider turn.
                if self.launch_chat_if_pending() {
                    self.request_redraw(true);
                }
                if self.drain_new_chat() {
                    self.request_redraw(true);
                }
                if self.drain_stop_chat() {
                    self.request_redraw(true);
                }
                if self.drain_close_chat_tab() {
                    self.request_redraw(true);
                }
                // A click on the attach button raises
                // `pending_attachment_pick` — open the file picker.
                if chat_attachment::drain_attachment_pick(&mut self.host) {
                    self.request_redraw(true);
                }
                // #20 theme presets — drain the preset dropdown's
                // pending `.optheme` import / export (blocking rfd
                // dialog, like `persistence::run_action` below).
                if crate::theme_preset_host::drain_preset_io(&mut self.host) {
                    self.request_redraw(true);
                }
                // Font import / removal raised by the property-panel
                // font picker — open the rfd dialog / run FontStore IO,
                // then refresh the picker's imported-family snapshot.
                if crate::font_import_host::drain_font_requests(&mut self.host) {
                    self.request_redraw(true);
                }
                if let Some(action) = self
                    .host
                    .editor_state_mut()
                    .editor_ui
                    .pending_file_action
                    .take()
                {
                    // ExportImage → close source overlay + open picker dialog.
                    if matches!(
                        action,
                        op_editor_core::editor_ui_state::FileAction::ExportImage
                    ) {
                        let eui = &mut self.host.editor_state_mut().editor_ui;
                        eui.file_menu_open = false;
                        eui.file_menu.hover = None;
                        eui.export_dialog_open = true;
                        self.host.mark_editor_state_dirty();
                        self.request_redraw(true);
                    } else {
                        // The file menu just closed (file_menu_open=false set
                        // in dispatch_file_menu_press), but `run_action` opens
                        // a BLOCKING native rfd dialog. `request_redraw` only
                        // defers a repaint to the next frame, which never
                        // arrives before the modal — so paint one synchronous
                        // frame here to actually dismiss the menu before the
                        // dialog covers it.
                        if let (Some(ctx), Some(backend)) =
                            (self.ctx.as_mut(), self.backend.as_mut())
                        {
                            frame::paint(
                                ctx,
                                backend,
                                &mut self.host,
                                self.viewport_width,
                                self.viewport_height,
                                self.dpi,
                            );
                        }
                        match persistence::run_action(
                            action,
                            &mut self.host,
                            &mut self.current_path,
                            self.window.as_ref(),
                        ) {
                            // `mark_document_saved` cancels any
                            // in-flight Figma import internally, so a
                            // stale worker can't overwrite the fresh
                            // document when its result lands.
                            op_host_services::doc_io::ActionOutcome::Saved => {
                                self.mark_document_saved()
                            }
                            // User picked a `.fig`; spin up the worker
                            // session and let `pump` apply the document
                            // once parsing finishes. Cancel any prior
                            // in-flight session first so two imports
                            // in quick succession don't race.
                            op_host_services::doc_io::ActionOutcome::FigmaImportStarted(path) => {
                                figma_import_session::cancel(
                                    &mut self.host,
                                    &mut self.current_figma_import,
                                );
                                self.current_figma_import =
                                    Some(figma_import_session::spawn(&mut self.host, path));
                                self.request_redraw(true);
                            }
                            op_host_services::doc_io::ActionOutcome::Noop => {}
                        }
                    }
                }
                if consumed {
                    self.request_redraw(true);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                if self.drain_pending_cursor_move() {
                    self.redraw_dirty = true;
                }
                let consumed = self.host.apply_right_press(
                    self.cursor_x,
                    self.cursor_y,
                    self.viewport_width,
                    self.viewport_height,
                );
                if consumed {
                    self.request_redraw(true);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } => {
                if self.drain_pending_cursor_move() {
                    self.redraw_dirty = true;
                }
                // Middle-button drag pans regardless of the active
                // tool (TS parity: e.button === 1 starts a pan).
                if self.host.apply_pan_press(self.cursor_x, self.cursor_y) {
                    self.request_redraw(true);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Middle,
                ..
            } => {
                if self.drain_pending_cursor_move() {
                    self.redraw_dirty = true;
                }
                let consumed = self
                    .host
                    .apply_release_with_viewport(self.viewport_width, self.viewport_height);
                if consumed {
                    self.request_redraw(true);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                // Drain pending cursor moves before release so drag-end commits final position.
                if self.drain_pending_cursor_move() {
                    self.redraw_dirty = true;
                }
                let consumed = self
                    .host
                    .apply_release_with_viewport(self.viewport_width, self.viewport_height);
                if consumed {
                    self.request_redraw(true);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Figma-style routing: pixel-delta pans, line-delta zooms; Cmd promotes pixel→zoom for trackpad-only laptops
                // a pinch sensor.
                let consumed = match delta {
                    MouseScrollDelta::LineDelta(_, y) => self.host.apply_wheel(
                        self.cursor_x,
                        self.cursor_y,
                        y * 16.0,
                        self.viewport_width,
                        self.viewport_height,
                    ),
                    MouseScrollDelta::PixelDelta(p) => {
                        let dx = p.x as f32 / self.dpi;
                        let dy = p.y as f32 / self.dpi;
                        if self.zoom_modifier {
                            self.host.apply_wheel(
                                self.cursor_x,
                                self.cursor_y,
                                dy,
                                self.viewport_width,
                                self.viewport_height,
                            )
                        } else {
                            self.host.apply_pan_gesture(
                                self.cursor_x,
                                self.cursor_y,
                                dx,
                                dy,
                                self.viewport_width,
                                self.viewport_height,
                            )
                        }
                    }
                };
                if consumed {
                    self.request_redraw(true);
                }
            }
            WindowEvent::PinchGesture { delta, .. } => {
                let consumed = self.host.apply_wheel(
                    self.cursor_x,
                    self.cursor_y,
                    delta as f32 * 400.0,
                    self.viewport_width,
                    self.viewport_height,
                );
                if consumed {
                    self.request_redraw(true);
                }
            }
            // CJK composition: preedit updates paint through the
            // shared overlay (host.apply_ime_preedit) and the OS
            // candidate window anchors to the focused input via
            // set_ime_cursor_area; the committed candidate lands
            // through apply_ime_commit -> apply_text.
            WindowEvent::Ime(winit::event::Ime::Preedit(text, cursor)) => {
                let changed = self.host.apply_ime_preedit(&text, cursor);
                if let Some(rect) = self
                    .host
                    .ime_anchor_rect(self.viewport_width, self.viewport_height)
                {
                    if let Some(window) = self.window.as_ref() {
                        let dpi = window.scale_factor();
                        window.set_ime_cursor_area(
                            winit::dpi::PhysicalPosition::new(
                                (rect.origin.x as f64) * dpi,
                                ((rect.origin.y + rect.size.y) as f64) * dpi,
                            ),
                            winit::dpi::PhysicalSize::new(
                                (rect.size.x as f64) * dpi,
                                (rect.size.y as f64) * dpi,
                            ),
                        );
                    }
                }
                if changed {
                    self.request_redraw(true);
                }
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                if self.host.apply_ime_commit(&text) {
                    self.request_redraw(true);
                }
            }
            WindowEvent::Ime(winit::event::Ime::Disabled) => {
                // Focus left the IME-capable surface mid-composition —
                // drop the preedit so the bubble doesn't linger.
                if self.host.apply_ime_preedit("", None) {
                    self.request_redraw(true);
                }
            }
            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                self.zoom_modifier = state.super_key() || state.control_key();
                self.shift_modifier = state.shift_key();
                self.alt_modifier = state.alt_key();
                // Forward shift state into the host so the next
                // mouse press can branch on shift+click for
                // multi-select.
                self.host.set_modifier_shift(self.shift_modifier);
                self.host.set_modifier_alt(self.alt_modifier);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Released,
                        logical_key: Key::Named(NamedKey::Space),
                        ..
                    },
                ..
            } => {
                // End transient space-pan (the press side lives in
                // `keyboard_input.rs`).
                self.host.set_space_pan(false);
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        text,
                        ..
                    },
                ..
            } => {
                self.handle_key_pressed(&logical_key, text.as_deref());
            }
            _ => {}
        }
        if self.reconcile_mcp_server_from_settings() {
            self.publish_live_mcp_port();
            self.request_redraw(true);
        }
        if self.reconcile_mcp_cli_integrations(mcp_cli_before) {
            self.request_redraw(true);
        }
        if let Some(before) = settings_before {
            op_host_services::settings_io::save_if_changed(self.host.editor_state(), before);
        }
        // A Git-panel click or Enter may have queued an action
        // (Commit / Refresh / Pull) — run it after the event.
        self.drain_git_action();
        // A Design-MD panel click may have queued an import / export.
        if self.drain_design_md_action() {
            self.request_redraw(true);
        }
        // A Component-Browser card click may have queued an insert —
        // run it against the current viewport centre. Schedule a
        // repaint on success so the new node lands visibly.
        if self
            .host
            .drain_component_browser_insert(self.viewport_width, self.viewport_height)
        {
            self.request_redraw(true);
        }
        // Component-Browser kit Import / Export + uikits.json flush.
        self.drain_kit_io();
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // macOS Cmd+Q / Alt+F4 / WM-close can deliver `exiting` without
        // `CloseRequested`; flush MCP port draft before snapshotting so
        // a focused-but-uncommitted edit isn't silently dropped.
        self.host.flush_settings_input();
        op_host_services::settings_io::save(self.host.editor_state());
        if let Some(mut server) = self.mcp_server.take() {
            server.stop();
            crate::mcp_port_file::remove();
        }
        // Save window geometry for next launch. Guarded on a window
        // having existed — a failed startup reaches `exiting` with
        // unseeded geometry and would clobber the previous good save.
        if self.window.is_some() {
            window_state::save(&window_state::WindowState::from_window_physical(
                self.win_pos,
                self.win_size,
                self.dpi,
                self.win_maximized,
            ));
        }
        if let Some(mut ctx) = self.ctx.take() {
            if let Err(err) = ctx.teardown() {
                eprintln!("openpencil-desktop: teardown failed: {err}");
            }
        }
        self.backend.take();
        self.window.take();
    }
}

fn render_surface_not_ready(err: &SharedSkiaError) -> bool {
    matches!(
        err,
        SharedSkiaError::Provider(ProviderError::SurfaceNotReady { .. })
    )
}

impl DesktopApp {
    fn refresh_host_clock(&mut self) {
        let now_ms = self.clock_start.elapsed().as_millis() as u64;
        self.host.set_now_ms(now_ms);
    }

    fn timed_wake_needs_redraw(&self, cause: &StartCause) -> bool {
        match cause {
            StartCause::ResumeTimeReached { .. } => true,
            StartCause::WaitCancelled {
                requested_resume: Some(deadline),
                ..
            } => Instant::now() >= *deadline,
            _ => false,
        }
    }

    pub(crate) fn resume_time_needs_redraw(&self) -> bool {
        self.current_chat.is_some()
            || self.current_design.is_some()
            || self.current_codegen.is_some()
            || self.current_design_md.is_some()
            || !self.sub_agents.is_empty()
            || self.current_figma_import.is_some()
            || self.pending_figma_paste.is_some()
            || self.host.next_animation_deadline_ms().is_some()
            || self.update_probe.is_pending()
            || self.model_probe.is_pending()
            || self.image_search.is_pending()
            || self.image_panel.is_pending()
            || self
                .iconify_job
                .as_ref()
                .is_some_and(crate::iconify_host::IconifyJob::is_pending)
            || self.provider_connect_pending()
            || self.acp_agent_connect_pending()
            || self
                .git_pull_job
                .as_ref()
                .is_some_and(git_jobs::GitPullJob::is_pending)
            || self
                .git_push_job
                .as_ref()
                .is_some_and(git_jobs::GitPushJob::is_pending)
            || self
                .git_status_job
                .as_ref()
                .is_some_and(git_jobs::GitStatusJob::is_pending)
            || self
                .git_diff_job
                .as_ref()
                .is_some_and(git_jobs::GitDiffJob::is_pending)
            || self.host.editor_state().editor_ui.git_panel.open
    }

    fn try_init_render_context(&mut self, event_loop: &ActiveEventLoop) -> bool {
        if self.ctx.is_some() && self.backend.is_some() {
            return true;
        }
        let Some(window) = self.window.as_ref() else {
            return false;
        };
        let size = window.inner_size();
        if size.width <= 1 || size.height <= 1 {
            self.defer_render_context_init(event_loop);
            return false;
        }

        let dpi = window.scale_factor() as f32;
        match SharedSkiaContext::new_desktop(window) {
            Ok(ctx) => {
                self.dpi = dpi;
                self.ctx = Some(ctx);
                self.backend = Some(NativeBackend::with_dpi(dpi));
                true
            }
            Err(err) if render_surface_not_ready(&err) => {
                self.defer_render_context_init(event_loop);
                false
            }
            Err(err) => {
                eprintln!("openpencil-desktop: SharedSkiaContext::new_desktop failed: {err}");
                self.error = Some(err);
                event_loop.exit();
                false
            }
        }
    }

    fn defer_render_context_init(&self, event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(
                INITIAL_VIEWPORT_W as f64,
                INITIAL_VIEWPORT_H as f64,
            ));
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(50),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_init_retries_only_surface_not_ready() {
        let not_ready = op_host_native::SharedSkiaError::Provider(
            op_host_native::ProviderError::SurfaceNotReady {
                width: 1,
                height: 1,
            },
        );
        assert!(render_surface_not_ready(&not_ready));

        assert!(!render_surface_not_ready(
            &op_host_native::SharedSkiaError::GlInterface
        ));
    }
}
