//! winit `ApplicationHandler` impl for `DesktopApp`. Split out of
//! `main.rs` to keep that file under the 800-line cap.

use crate::{
    chat_attachment, chat_session, cursor_icon, design_session, figma_import_session, frame,
    git_jobs, menu, persistence, settings_io, window_state, DesktopApp, INITIAL_VIEWPORT_H,
    INITIAL_VIEWPORT_W,
};
use op_host_native::{NativeBackend, ProviderError, SharedSkiaContext, SharedSkiaError};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{
    ElementState, KeyEvent, MouseButton, MouseScrollDelta, StartCause, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::window::{Window, WindowId};

impl ApplicationHandler for DesktopApp {
    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: StartCause) {
        // When the WaitUntil deadline fires, the next redraw paints
        // the next caret-blink phase. winit doesn't auto-redraw on
        // ResumeTimeReached, so we have to request it here.
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
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
        // Restore the window geometry from the previous session
        // (position / size / maximized). A missing or stale file
        // leaves the default attrs untouched.
        let saved_geometry = window_state::load();
        if let Some(saved) = saved_geometry.as_ref() {
            attrs = saved.apply_to(attrs);
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
            if persistence::is_supported_figma_import(&path) {
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

        // The auto-update probe spawned in `new()` is likely still
        // running. Wake the loop soon so its result is drained even
        // if the user never touches the freshly opened window.
        if self.update_probe.is_pending() {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(500),
            ));
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
        let now_ms = self.clock_start.elapsed().as_millis() as u64;
        self.host.set_now_ms(now_ms);
        // CursorMoved never changes persisted prefs — skip snapshot
        // on the trackpad hot path.
        let settings_before = match &event {
            WindowEvent::CursorMoved { .. } => None,
            _ => Some(settings_io::fingerprint(self.host.editor_state())),
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
                    settings_io::save(self.host.editor_state());
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
                if persistence::is_supported_figma_import(&path) {
                    figma_import_session::cancel(&mut self.host, &mut self.current_figma_import);
                    self.current_figma_import =
                        Some(figma_import_session::spawn(&mut self.host, path));
                    self.request_redraw(true);
                } else if persistence::is_supported_document(&path) {
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
                if chat_session::drain_new_chat_request(
                    &mut self.host,
                    &mut self.current_chat,
                    &mut self.current_design,
                ) {
                    self.redraw_dirty = true;
                }
                if chat_session::drain_stop_request(
                    &mut self.host,
                    &mut self.current_chat,
                    &mut self.current_design,
                ) {
                    self.redraw_dirty = true;
                }
                // Pump in-flight AI chat deltas into this frame.
                if chat_session::pump(&mut self.host, &mut self.current_chat) {
                    self.redraw_dirty = true;
                }
                // Drain a finished background `.fig` parse — applies
                // the imported document + clears the loading overlay
                // flag. Rebinds Git + window title on success
                // (matches the prior synchronous path's outcome).
                match figma_import_session::pump(
                    &mut self.host,
                    &mut self.current_figma_import,
                    &mut self.current_path,
                    self.window.as_ref(),
                ) {
                    figma_import_session::PumpOutcome::CompletedOk => {
                        self.rebind_git_session_for_current_path();
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
                if design_session::pump_progress(&mut self.host, &mut self.current_design) {
                    self.redraw_dirty = true;
                }
                self.image_search.enqueue_missing(self.host.editor_state());
                if self.image_search.poll_into(self.host.editor_state_mut()) {
                    self.host.mark_editor_state_dirty();
                    self.redraw_dirty = true;
                }
                // Drain background model discovery once it lands.
                if self.model_probe.poll_into(&mut self.host) {
                    self.redraw_dirty = true;
                }
                if self.drain_iconify_picker() {
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
                // Chat / design / Figma-import worker active → wake
                // ~10 fps to pump results and animate the loading
                // overlay's spinner. Chat + design need ~30 fps for
                // streaming deltas; Figma import is a one-shot result
                // but the overlay's spinner needs frames to animate.
                if self.current_chat.is_some() || self.current_design.is_some() {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + Duration::from_millis(33),
                    ));
                } else if self.current_figma_import.is_some() || self.mcp_server_active() {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + Duration::from_millis(100),
                    ));
                } else if let Some(deadline_ms) = self.host.next_animation_deadline_ms() {
                    let deadline = self.clock_start + Duration::from_millis(deadline_ms);
                    event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                } else if self.update_probe.is_pending()
                    || self.image_search.is_pending()
                    || self
                        .iconify_job
                        .as_ref()
                        .is_some_and(crate::iconify_host::IconifyJob::is_pending)
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
                    // Keep waking ~2 Hz until the background update
                    // probe / git pull / git status query lands so its
                    // result is drained even while the app is idle.
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
                // `cursor_hint` hit-tests the layout-resolved render
                // scene. A mutation since the last paint may have left
                // the scene stale (`editor_state_dirty`), so refresh it
                // first — otherwise a post-mutation / pre-paint cursor
                // move reads stale geometry and picks the wrong hint.
                let _ = self.host.layout_scene();
                if let Some(window) = self.window.as_ref() {
                    let viewport_w = window.inner_size().width as f32 / self.dpi;
                    let viewport_h = window.inner_size().height as f32 / self.dpi;
                    let hint =
                        self.host
                            .cursor_hint(self.cursor_x, self.cursor_y, viewport_w, viewport_h);
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
                                    settings_io::save(self.host.editor_state());
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
                if let Some(text) = self.host.editor_state_mut().chat.pending_copy_text.take() {
                    crate::clipboard::set_text(&text);
                }
                // A click on the chat Send button raises
                // `pending_send` — launch the provider turn.
                if chat_session::launch_if_pending(
                    &mut self.host,
                    &mut self.current_chat,
                    &mut self.current_design,
                ) {
                    self.request_redraw(true);
                }
                if chat_session::drain_new_chat_request(
                    &mut self.host,
                    &mut self.current_chat,
                    &mut self.current_design,
                ) {
                    self.request_redraw(true);
                }
                if chat_session::drain_stop_request(
                    &mut self.host,
                    &mut self.current_chat,
                    &mut self.current_design,
                ) {
                    self.request_redraw(true);
                }
                // A click on the attach button raises
                // `pending_attachment_pick` — open the file picker.
                if chat_attachment::drain_attachment_pick(&mut self.host) {
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
                        eui.file_menu_hover = None;
                        eui.export_dialog_open = true;
                        self.host.mark_editor_state_dirty();
                        self.request_redraw(true);
                    } else {
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
                            persistence::ActionOutcome::Saved => self.mark_document_saved(),
                            // User picked a `.fig`; spin up the worker
                            // session and let `pump` apply the document
                            // once parsing finishes. Cancel any prior
                            // in-flight session first so two imports
                            // in quick succession don't race.
                            persistence::ActionOutcome::FigmaImportStarted(path) => {
                                figma_import_session::cancel(
                                    &mut self.host,
                                    &mut self.current_figma_import,
                                );
                                self.current_figma_import =
                                    Some(figma_import_session::spawn(&mut self.host, path));
                                self.request_redraw(true);
                            }
                            persistence::ActionOutcome::Noop => {}
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
            // CJK composition: macOS / X11 / Wayland route the committed
            // candidate string through here. We don't paint the preedit
            // yet; only the final commit is pushed into the focused input.
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                let mut consumed = false;
                for ch in text.chars() {
                    if self.host.apply_text(ch) {
                        consumed = true;
                    }
                }
                if consumed {
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
            self.request_redraw(true);
        }
        if self.reconcile_mcp_cli_integrations(mcp_cli_before) {
            self.request_redraw(true);
        }
        if let Some(before) = settings_before {
            settings_io::save_if_changed(self.host.editor_state(), before);
        }
        // A Git-panel click or Enter may have queued an action
        // (Commit / Refresh / Pull) — run it after the event.
        self.drain_git_action();
        // A Design-MD panel click may have queued an import / export.
        self.drain_design_md_action();
        // A Component-Browser card click may have queued an insert —
        // run it against the current viewport centre. Schedule a
        // repaint on success so the new node lands visibly.
        if self
            .host
            .drain_component_browser_insert(self.viewport_width, self.viewport_height)
        {
            self.request_redraw(true);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // macOS Cmd+Q / Alt+F4 / WM-close can deliver `exiting` without
        // `CloseRequested`; flush MCP port draft before snapshotting so
        // a focused-but-uncommitted edit isn't silently dropped.
        self.host.flush_settings_input();
        settings_io::save(self.host.editor_state());
        if let Some(mut server) = self.mcp_server.take() {
            server.stop();
        }
        // Save window geometry for next launch. Guarded on a window
        // having existed — a failed startup reaches `exiting` with
        // unseeded geometry and would clobber the previous good save.
        if self.window.is_some() {
            window_state::save(&window_state::WindowState::from_window(
                self.win_pos,
                self.win_size,
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
