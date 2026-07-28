//! winit `ApplicationHandler` impl for `DesktopApp`. Split out of
//! `main.rs` to keep that file under the 800-line cap.
//!
//! The `window_event` dispatcher's per-event-family bodies live in the
//! sibling modules under `app_handler/`; this file keeps the
//! `ApplicationHandler` impl plus the dispatch skeleton.

mod keyboard_events;
mod pointer_events;
mod redraw;
mod scheduling;
mod window_events;

use crate::{
    a11y, chat_session, cursor_icon, frame, menu, persistence, window_state, DesktopApp,
    DesktopEvent, INITIAL_VIEWPORT_H, INITIAL_VIEWPORT_W,
};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, StartCause, WindowEvent};
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
        if self.collab_runtime.poll(&mut self.host) {
            self.request_redraw(true);
        }
        if self.mcp_shutdown_requested() {
            // Finalize-lifecycle invariant (0718-1-k3-1 postmortem) — see
            // `chat_session::finalize_design_session_if_needed`'s doc
            // comment. MCP-driven shutdown can fire while a chat-launched
            // design loop is still in flight in the same process.
            chat_session::finalize_design_session_if_needed(
                &mut self.host,
                &self.current_chat,
                "teardown-backstop",
            );
            event_loop.exit();
            return;
        }
        // When a WaitUntil deadline fires, only timed UI activity needs a paint
        // (caret blink, streaming chat, imports, background jobs). Live MCP uses
        // `DesktopEvent::McpWake`, so an idle server no longer creates timer
        // ticks just to poll for possible requests.
        if self.timed_wake_needs_redraw(&cause) {
            if self.resume_time_needs_redraw() {
                self.request_redraw(true);
            } else {
                // A timed wake with nothing to repaint must still RESET
                // the control flow: leaving the elapsed `WaitUntil` in
                // place made casement re-fire the timer immediately —
                // an idle spin that burned ~40% of the main thread in
                // CFRunLoop timer arming (measured via `sample`).
                self.schedule_next_wake(event_loop);
            }
        }
        // A file drag owns the pointer, so the drop-target ring has to be
        // re-probed on each wake rather than driven by cursor events.
        if self.refresh_image_drop_hover() {
            self.request_redraw(true);
        }
        // A file drag owns the pointer, so the drop-target ring has to be
        // re-probed on each wake rather than driven by cursor events.
        if self.refresh_image_drop_hover() {
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
            .with_title(op_editor_ui::PRODUCT_NAME)
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
        if !crate::macos_app::configure_srgb_window(&window) {
            tracing::warn!("could not configure the AppKit window colour space as sRGB");
        }
        // IME starts disabled. Once a logical text input takes focus,
        // `sync_native_ime` publishes its caret area before enabling IME so
        // the first macOS candidate window never observes the zero anchor.
        window.set_ime_allowed(false);

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
        // Replay last session's CLI connections (silent probes, one at a
        // time) so the user isn't greeted by five "Connect" buttons on
        // every launch.
        self.restore_remembered_connections();
        if let Some(style) = crate::ui_prefs::load_pencil_cursor() {
            self.host.editor_state_mut().editor_ui.pencil_cursor_style = style;
        }

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
                let _ = self.begin_figma_import(path);
            } else if op_host_services::doc_io::is_supported_html_import(&path) {
                let _ = self.begin_html_import(path);
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
            }
            DesktopEvent::CollabWake => {
                if self.collab_runtime.poll(&mut self.host) {
                    self.request_redraw(true);
                }
            }
            DesktopEvent::ImageDecodeReady => {
                self.request_redraw(true);
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
            DesktopEvent::SaveReady => {
                if self.poll_background_save() {
                    self.request_redraw(true);
                }
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
            WindowEvent::CloseRequested => self.on_close_requested(event_loop),
            WindowEvent::Resized(size) => self.on_resized(event_loop, size),
            WindowEvent::Moved(pos) => self.on_moved(pos),
            WindowEvent::HoveredFile(path) => self.on_hovered_file(&path),
            WindowEvent::HoveredFileCancelled => self.on_hovered_file_cancelled(),
            WindowEvent::DroppedFile(path) => self.on_dropped_file(path),
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.on_scale_factor_changed(scale_factor)
            }
            WindowEvent::RedrawRequested => {
                if !self.on_redraw_requested(event_loop) {
                    return;
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if !self.on_cursor_moved(position) {
                    return;
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.on_left_press(event_loop) {
                    return;
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => self.on_right_press(),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Middle,
                ..
            } => self.on_middle_press(),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Middle,
                ..
            } => self.on_middle_release(),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.on_left_release(),
            WindowEvent::MouseWheel { delta, .. } => self.on_mouse_wheel(delta),
            WindowEvent::PinchGesture { delta, .. } => self.on_pinch_gesture(delta),
            // CJK composition: preedit updates paint through the
            // shared overlay (host.apply_ime_preedit) and the OS
            // candidate window anchors to the focused input via
            // set_ime_cursor_area; the committed candidate lands
            // through apply_ime_commit -> apply_text.
            WindowEvent::Ime(winit::event::Ime::Preedit(text, cursor)) => {
                self.on_ime_preedit(&text, cursor)
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => self.on_ime_commit(&text),
            WindowEvent::Ime(winit::event::Ime::Disabled) => self.on_ime_disabled(),
            WindowEvent::ModifiersChanged(mods) => self.on_modifiers_changed(mods),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Released,
                        logical_key: Key::Named(NamedKey::Space),
                        ..
                    },
                ..
            } => self.on_space_released(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key,
                        text,
                        ..
                    },
                ..
            } => self.on_key_pressed(&logical_key, text.as_deref()),
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
        // Queue an authenticated Bye before transport workers are stopped.
        self.collab_runtime.leave(&mut self.host);
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
