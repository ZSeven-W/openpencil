//! OpenPencil desktop runner — winit + skia-safe + WidgetHostNative.
//! Owns the event loop, GL surface, DPI, animation timer + cursor input.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

mod chat_claude;
mod chat_copilot;
mod chat_http_server;
mod chat_runtime;
mod chat_session;
mod chat_subprocess;
mod cursor_icon;
mod export;
mod export_pdf;
mod frame;
mod mcp_serve;
mod model_discovery;
mod persistence;
mod settings_io;

use op_host_native::{NativeBackend, SharedSkiaContext, SharedSkiaError, WidgetHostNative};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{
    ElementState, KeyEvent, MouseButton, MouseScrollDelta, StartCause, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

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
    /// Background AI-model discovery — probes the installed CLIs
    /// on a worker thread; its result is drained into
    /// `chat.available_models` on a later frame.
    model_probe: model_discovery::ModelProbe,
}

impl DesktopApp {
    fn new() -> Self {
        let mut host = WidgetHostNative::new();
        // Best-effort prefs restore onto the host's `EditorState`.
        settings_io::load(host.editor_state_mut());
        host.mark_editor_state_dirty();
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
            model_probe: model_discovery::ModelProbe::spawn(),
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
            let hover_changed = self.host.update_layer_hover(cx, cy, self.viewport_height);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_only_redraw_without_visible_state_change_skips_present() {
        let mut app = DesktopApp::new();
        app.redraw_pending = true;
        app.pending_cursor_move = Some((1200.0, 20.0));

        assert!(!app.prepare_redraw());
        assert!(!app.redraw_pending);
        assert!(app.pending_cursor_move.is_none());
    }

    #[test]
    fn consumed_press_dirties_existing_cursor_redraw_without_second_request() {
        let mut app = DesktopApp::new();
        app.redraw_pending = true;

        assert!(!app.request_redraw(true));
        assert!(app.prepare_redraw());
    }

    #[test]
    fn cursor_redraw_still_paints_when_layer_hover_changes() {
        let mut app = DesktopApp::new();
        app.redraw_pending = true;
        app.pending_cursor_move = Some((
            20.0,
            op_editor_ui::widgets::TOP_BAR_HEIGHT + 8.0 + 28.0 + 16.0,
        ));

        assert!(app.prepare_redraw());
    }
}

impl ApplicationHandler for DesktopApp {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        // When the WaitUntil deadline fires, the next redraw paints
        // the next caret-blink phase. winit doesn't auto-redraw on
        // ResumeTimeReached, so we have to request it here.
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            self.request_redraw(true);
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("OpenPencil")
            .with_inner_size(winit::dpi::LogicalSize::new(
                INITIAL_VIEWPORT_W as u32,
                INITIAL_VIEWPORT_H as u32,
            ));
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
                self.host.flush_settings_input();
                settings_io::save(self.host.editor_state());
                event_loop.exit();
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
                self.request_redraw(true);
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
                        persistence::run_action(
                            action,
                            &mut self.host,
                            &mut self.current_path,
                            self.window.as_ref(),
                        );
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
            WindowEvent::Ime(ime) => {
                // CJK composition: macOS / X11 / Wayland route the
                // committed candidate string through here. We don't
                // paint the preedit yet; only the final commit is
                // pushed into the focused input.
                if let winit::event::Ime::Commit(text) = ime {
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
                use op_editor_core::ReorderDirection;
                let mut consumed = false;
                let nudge = if self.shift_modifier { 10.0 } else { 1.0 };
                // While a settings-modal input owns the keyboard, the
                // ONLY allowed paths are text / backspace / send /
                // escape. Editor shortcuts (Cmd+D, Cmd+G, Cmd+Z,
                // arrow nudges, Delete, [ / ], single-letter tool
                // switches, …) would otherwise silently mutate the
                // document while the user thinks they're typing a port.
                let settings_focused = self.host.settings_focus_active();
                match logical_key {
                    // Named-key shortcuts fire only when no Cmd/Ctrl is held.
                    Key::Named(NamedKey::Backspace) if !self.zoom_modifier => {
                        consumed = self.host.apply_backspace();
                    }
                    Key::Named(NamedKey::Delete) if !self.zoom_modifier && !settings_focused => {
                        consumed = self.host.apply_delete();
                    }
                    Key::Named(NamedKey::Enter) if !self.zoom_modifier => {
                        consumed = self.host.apply_send();
                        // apply_send may raise pending_send (chat send).
                        if chat_session::launch_if_pending(&mut self.host, &mut self.current_chat) {
                            self.request_redraw(true);
                        }
                    }
                    Key::Named(NamedKey::Escape) if !self.zoom_modifier => {
                        consumed = self.host.apply_escape();
                    }
                    Key::Named(NamedKey::ArrowUp) if !self.zoom_modifier && !settings_focused => {
                        consumed = self.host.apply_nudge(0.0, -nudge);
                    }
                    Key::Named(NamedKey::ArrowDown) if !self.zoom_modifier && !settings_focused => {
                        consumed = self.host.apply_nudge(0.0, nudge);
                    }
                    Key::Named(NamedKey::ArrowLeft) if !self.zoom_modifier && !settings_focused => {
                        consumed = self.host.apply_nudge(-nudge, 0.0);
                    }
                    Key::Named(NamedKey::ArrowRight)
                        if !self.zoom_modifier && !settings_focused =>
                    {
                        consumed = self.host.apply_nudge(nudge, 0.0);
                    }
                    // Cmd/Ctrl+Alt+U/S/I/X — path boolean ops (Paper.js parity).
                    Key::Character(ref ch)
                        if self.zoom_modifier && self.alt_modifier && !self.shift_modifier =>
                    {
                        use op_editor_core::BooleanOp;
                        match ch.to_lowercase().as_str() {
                            "u" => consumed = self.host.apply_boolean_op(BooleanOp::Union),
                            "s" => consumed = self.host.apply_boolean_op(BooleanOp::Subtract),
                            "i" => consumed = self.host.apply_boolean_op(BooleanOp::Intersect),
                            "x" => consumed = self.host.apply_boolean_op(BooleanOp::Exclude),
                            _ => {}
                        }
                    }
                    Key::Character(ref ch) if self.zoom_modifier && !self.shift_modifier => {
                        let lower = ch.to_lowercase();
                        match lower.as_str() {
                            // Cmd+, always allowed — it toggles the
                            // modal itself; closing while focused
                            // also commits via the close path.
                            "," => consumed = self.host.apply_toggle_agent_settings(),
                            "s" => {
                                // Codex stop-gate: commit any pending
                                // variable-row inline edit before save
                                // so the typed value lands on disk.
                                self.host.commit_variable_row_focus_if_any_pub();
                                consumed = persistence::handle_save(
                                    &mut self.host,
                                    &mut self.current_path,
                                    self.window.as_ref(),
                                );
                            }
                            "o" => {
                                self.host.commit_variable_row_focus_if_any_pub();
                                consumed = persistence::handle_open(
                                    &mut self.host,
                                    &mut self.current_path,
                                    self.window.as_ref(),
                                );
                            }
                            _ if settings_focused => {}
                            "d" => consumed = self.host.apply_duplicate(),
                            "a" => consumed = self.host.apply_select_all(),
                            "c" => consumed = self.host.apply_copy(),
                            "x" => consumed = self.host.apply_cut(),
                            "v" => consumed = self.host.apply_paste(),
                            "z" => consumed = self.host.apply_undo(),
                            "y" => consumed = self.host.apply_redo(),
                            "g" => consumed = self.host.apply_group(),
                            "j" => consumed = self.host.apply_toggle_chat(),
                            _ => {}
                        }
                    }
                    Key::Character(ref ch) if self.zoom_modifier && self.shift_modifier => {
                        match ch.to_lowercase().as_str() {
                            // Cmd+Shift+S = Save As; always allowed.
                            "s" => {
                                self.host.commit_variable_row_focus_if_any_pub();
                                consumed = persistence::handle_save_as(
                                    &mut self.host,
                                    &mut self.current_path,
                                    self.window.as_ref(),
                                );
                            }
                            "p" => {
                                self.host.commit_variable_row_focus_if_any_pub();
                                persistence::run_action(
                                    op_editor_core::editor_ui_state::FileAction::ExportImage,
                                    &mut self.host,
                                    &mut self.current_path,
                                    self.window.as_ref(),
                                );
                                consumed = true;
                            }
                            _ if settings_focused => {}
                            "z" => consumed = self.host.apply_redo(),
                            "g" => consumed = self.host.apply_ungroup(),
                            "c" => consumed = self.host.apply_toggle_code_panel(),
                            _ => {}
                        }
                    }
                    // Single-letter tool switches (no modifier). Only
                    // fire when no input is focused so typing in a
                    // text node / chat / rename doesn't switch tools.
                    Key::Character(ref ch)
                        if !self.zoom_modifier && !self.host.input_active_pub() =>
                    {
                        let lower = ch.to_lowercase();
                        let mut handled = true;
                        match lower.as_str() {
                            "v" => self.host.apply_set_tool(op_editor_core::Tool::Select),
                            "r" => self.host.apply_set_tool(op_editor_core::Tool::Rect),
                            "o" => self.host.apply_set_tool(op_editor_core::Tool::Ellipse),
                            "l" => self.host.apply_set_tool(op_editor_core::Tool::Line),
                            "t" => self.host.apply_set_tool(op_editor_core::Tool::Text),
                            "f" => self.host.apply_set_tool(op_editor_core::Tool::Frame),
                            "p" => self.host.apply_set_tool(op_editor_core::Tool::Pen),
                            "h" => self.host.apply_set_tool(op_editor_core::Tool::Hand),
                            "[" => {
                                consumed = self.host.apply_reorder(ReorderDirection::Down);
                                handled = false;
                            }
                            "]" => {
                                consumed = self.host.apply_reorder(ReorderDirection::Up);
                                handled = false;
                            }
                            _ => handled = false,
                        }
                        if handled {
                            consumed = true;
                        }
                    }
                    // `[` / `]` — z-order reorder when an input is focused (still gated by apply_reorder internally).
                    Key::Character(ref ch) if !self.zoom_modifier => match ch.as_str() {
                        "[" if !settings_focused => {
                            consumed = self.host.apply_reorder(ReorderDirection::Down)
                        }
                        "]" if !settings_focused => {
                            consumed = self.host.apply_reorder(ReorderDirection::Up)
                        }
                        _ => {
                            if let Some(s) = text.as_deref() {
                                for c in s.chars() {
                                    if !c.is_control() && self.host.apply_text(c) {
                                        consumed = true;
                                    }
                                }
                            }
                        }
                    },
                    _ => {
                        // Suppress apply_text whenever Cmd / Ctrl
                        // is held — Cmd-anything that isn't bound
                        // above must NOT type into a focused chat
                        // / property input. Otherwise Cmd+Shift+D
                        // (and other unbound chords) would inject
                        // "D" into the focused input.
                        if !self.zoom_modifier {
                            if let Some(s) = text.as_deref() {
                                for c in s.chars() {
                                    if !c.is_control() && self.host.apply_text(c) {
                                        consumed = true;
                                    }
                                }
                            }
                        }
                    }
                }
                if consumed {
                    self.request_redraw(true);
                }
            }
            _ => {}
        }
        if let Some(before) = settings_before {
            settings_io::save_if_changed(self.host.editor_state(), before);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Belt-and-suspenders: macOS Cmd+Q / Alt+F4 / window-manager
        // close can deliver `exiting` without `CloseRequested`. Flush
        // any in-progress MCP port draft before snapshotting so a
        // focused-but-uncommitted edit isn't silently dropped.
        self.host.flush_settings_input();
        settings_io::save(self.host.editor_state());
        if let Some(mut ctx) = self.ctx.take() {
            if let Err(err) = ctx.teardown() {
                eprintln!("openpencil-desktop: teardown failed: {err}");
            }
        }
        self.backend.take();
        self.window.take();
    }
}

fn main() {
    // `--mcp <path>` swaps the GUI for a JSON-RPC stdio MCP
    // server backed by that .op file. External CLIs (Claude
    // Code / Codex / Gemini / Copilot) spawn the binary in this
    // mode to drive the Rust editor the same way they drive TS
    // pen-mcp.
    let mut args = std::env::args().skip(1);
    if let Some(first) = args.next() {
        if first == "--mcp" {
            let Some(path) = args.next() else {
                eprintln!("openpencil-desktop --mcp: missing <path> arg");
                std::process::exit(2);
            };
            match mcp_serve::run(PathBuf::from(path)) {
                Ok(()) => return,
                Err(e) => {
                    eprintln!("openpencil-desktop --mcp: {e}");
                    std::process::exit(1);
                }
            }
        }
        // Unknown leading arg → fall through to GUI mode for now
        // (a future patch may add `--help` etc).
    }
    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(err) => {
            eprintln!("openpencil-desktop: EventLoop::new failed: {err}");
            std::process::exit(1);
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = DesktopApp::new();
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("openpencil-desktop: run_app exited with error: {err}");
        std::process::exit(1);
    }
    if let Some(err) = app.error {
        eprintln!("openpencil-desktop: fatal error during run: {err}");
        std::process::exit(1);
    }
}
