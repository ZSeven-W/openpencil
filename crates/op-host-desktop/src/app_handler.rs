//! `impl ApplicationHandler for DesktopApp` — the winit event-loop
//! body (new_events / resumed / window_event). Split out of `main.rs`
//! to keep that file under the 800-line cap; `main.rs` keeps the
//! `DesktopApp` struct, its helper `impl`, and `fn main`.

use crate::{
    chat_attachment, chat_session, cursor_icon, frame, git_jobs, menu, persistence, settings_io,
    window_state, DesktopApp, INITIAL_VIEWPORT_H, INITIAL_VIEWPORT_W,
};
use op_host_native::{NativeBackend, SharedSkiaContext};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, StartCause, WindowEvent};
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
            ));
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
        match SharedSkiaContext::new_desktop(&window) {
            Ok(ctx) => {
                self.ctx = Some(ctx);
                self.backend = Some(NativeBackend::with_dpi(dpi));
            }
            Err(err) => {
                eprintln!("openpencil-desktop: SharedSkiaContext::new_desktop failed: {err}");
                self.error = Some(err);
                event_loop.exit();
                return;
            }
        }
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
        // via argv now that the host + window are ready.
        if let Some(path) = self.initial_file.take() {
            if persistence::open_path(
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
            WindowEvent::DroppedFile(path) => {
                // Drag-and-drop open. Only `.op` / `.pen` documents
                // are accepted; anything else is ignored silently so
                // a stray drop can't disrupt the current document.
                if persistence::is_supported_document(&path) {
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
                        "openpencil-desktop: ignored dropped file (not .op / .pen): {}",
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
                // Pump in-flight AI chat deltas into this frame.
                if chat_session::pump(&mut self.host, &mut self.current_chat) {
                    self.redraw_dirty = true;
                }
                // Drain background model discovery once it lands.
                if self.model_probe.poll_into(&mut self.host) {
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
                // Chat turn streaming → wake ~30 fps to pump deltas.
                if self.current_chat.is_some() {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + Duration::from_millis(33),
                    ));
                } else if let Some(deadline_ms) = self.host.next_animation_deadline_ms() {
                    let deadline = self.clock_start + Duration::from_millis(deadline_ms);
                    event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                } else if self.update_probe.is_pending()
                    || self.git_pull_job.as_ref().is_some_and(git_jobs::GitPullJob::is_pending)
                    || self.git_push_job.as_ref().is_some_and(git_jobs::GitPushJob::is_pending)
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
                let consumed = self.host.apply_press(
                    self.cursor_x,
                    self.cursor_y,
                    self.viewport_width,
                    self.viewport_height,
                );
                // A click on the chat Send button raises
                // `pending_send` — launch the provider turn.
                if chat_session::launch_if_pending(&mut self.host, &mut self.current_chat) {
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
                    } else if persistence::run_action(
                        action,
                        &mut self.host,
                        &mut self.current_path,
                        self.window.as_ref(),
                    ) {
                        self.mark_document_saved();
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
        // Belt-and-suspenders: macOS Cmd+Q / Alt+F4 / window-manager
        // close can deliver `exiting` without `CloseRequested`. Flush
        // any in-progress MCP port draft before snapshotting so a
        // focused-but-uncommitted edit isn't silently dropped.
        self.host.flush_settings_input();
        settings_io::save(self.host.editor_state());
        // Persist the window geometry so the next launch restores
        // where the user left the window. Guarded on a window having
        // existed: a failed startup (create_window / Skia init error)
        // reaches `exiting` with unseeded geometry, and saving that
        // would clobber the previous session's good geometry.
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
