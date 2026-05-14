//! OpenPencil desktop runner — winit + skia-safe + WidgetHostNative.
//! Owns the event loop, GL surface, DPI, animation timer + cursor input.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

mod export;
mod pen_doc_adapter;
mod pen_doc_path_bounds;
mod persistence;
mod settings_io;

use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend};
use openpencil_shell_native::{
    NativeBackend, NativeFrameBackend, SharedSkiaContext, SharedSkiaError, WidgetHostNative,
};
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

/// Paint pass — clear, scale by DPI, dispatch to the widget host.
fn paint(
    ctx: &mut SharedSkiaContext,
    backend: &mut NativeBackend,
    host: &WidgetHostNative,
    viewport_width: f32,
    viewport_height: f32,
    dpi: f32,
) {
    ctx.begin_frame();
    ctx.with_frame(|canvas, _glow| {
        canvas.clear(skia_safe::Color::BLACK);
        canvas.reset_matrix();
        canvas.scale((dpi, dpi));
        let mut frame = NativeFrameBackend::new(backend, canvas);
        frame.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(viewport_width, viewport_height),
            },
            Color::BLACK,
        );
        host.paint(&mut frame, viewport_width, viewport_height);
    });
    ctx.present();
}

/// Build a 32 × 32 RGBA rotate-cursor bitmap by rendering the
/// lucide `RotateCw` icon through skia's stroke-path pipeline —
/// proper Skia AA + round caps/joins, matches the rest of the
/// icon set's visual language. Two passes: white halo for
/// legibility on any background, then a dark core on top.
fn make_rotate_cursor_rgba() -> (Vec<u8>, u16, u16, u16, u16) {
    const SIZE: i32 = 16;
    // Lucide rotate-cw d-strings (one per <path> element).
    const LUCIDE_PATHS: &[&str] = &[
        "M21 12a9 9 0 1 1-9-9c2.52 0 4.93 1 6.74 2.74L21 8",
        "M21 3v5h-5",
    ];

    // Off-screen raster surface; skia handles all AA + stroking.
    let mut surface = skia_safe::surfaces::raster_n32_premul((SIZE, SIZE))
        .expect("raster_n32_premul should produce a CPU surface for cursor");
    {
        let canvas = surface.canvas();
        canvas.clear(skia_safe::Color::TRANSPARENT);
        let scale = (SIZE as f32) / 24.0; // lucide viewBox is 24 × 24
        let mut paths = Vec::with_capacity(LUCIDE_PATHS.len());
        for d in LUCIDE_PATHS {
            if let Some(path) = skia_safe::utils::parse_path::from_svg(d) {
                let mut m = skia_safe::Matrix::new_identity();
                m.set_scale((scale, scale), None);
                paths.push(path.with_transform(&m));
            }
        }

        // Halo (white, slightly wider).
        let mut halo = skia_safe::Paint::new(skia_safe::Color4f::new(1.0, 1.0, 1.0, 1.0), None);
        halo.set_anti_alias(true);
        halo.set_stroke(true);
        halo.set_stroke_width(2.4);
        halo.set_stroke_cap(skia_safe::PaintCap::Round);
        halo.set_stroke_join(skia_safe::PaintJoin::Round);
        for p in &paths {
            canvas.draw_path(p, &halo);
        }

        // Core stroke — near-black for contrast.
        let mut core = skia_safe::Paint::new(skia_safe::Color4f::new(0.06, 0.06, 0.06, 1.0), None);
        core.set_anti_alias(true);
        core.set_stroke(true);
        core.set_stroke_width(1.2);
        core.set_stroke_cap(skia_safe::PaintCap::Round);
        core.set_stroke_join(skia_safe::PaintJoin::Round);
        for p in &paths {
            canvas.draw_path(p, &core);
        }
    }

    let row_bytes = (SIZE as usize) * 4;
    let mut bgra = vec![0u8; row_bytes * (SIZE as usize)];
    let info = skia_safe::ImageInfo::new(
        (SIZE, SIZE),
        skia_safe::ColorType::BGRA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    surface.read_pixels(&info, &mut bgra, row_bytes, (0, 0));
    // winit wants straight RGBA — swap BGRA → RGBA in place.
    let mut rgba = bgra;
    for px in rgba.chunks_exact_mut(4) {
        px.swap(0, 2);
    }

    let hotspot = (SIZE as u16) / 2;
    (rgba, SIZE as u16, SIZE as u16, hotspot, hotspot)
}

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
}

impl DesktopApp {
    fn new() -> Self {
        let mut host = WidgetHostNative::new();
        settings_io::load(host.document_mut()); // best-effort prefs restore
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
            shift_modifier: false,
            pending_cursor_move: None,
            redraw_pending: false,
            redraw_dirty: false,
            clock_start: Instant::now(),
            rotate_cursor: None,
            current_path: None,
            error: None,
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
            openpencil_shell_core::widgets::TOP_BAR_HEIGHT + 8.0 + 28.0 + 16.0,
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
        let (rgba, w, h, hx, hy) = make_rotate_cursor_rgba();
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
            paint(
                ctx,
                backend,
                &self.host,
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
            _ => Some(settings_io::fingerprint(self.host.document())),
        };
        match event {
            WindowEvent::CloseRequested => {
                self.host.flush_settings_input();
                settings_io::save(self.host.document());
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
                let should_paint = self.prepare_redraw();
                if should_paint {
                    if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
                        paint(
                            ctx,
                            backend,
                            &self.host,
                            self.viewport_width,
                            self.viewport_height,
                            self.dpi,
                        );
                    }
                }
                if let Some(deadline_ms) = self.host.next_animation_deadline_ms() {
                    let deadline = self.clock_start + Duration::from_millis(deadline_ms);
                    event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                } else {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x as f32 / self.dpi;
                self.cursor_y = position.y as f32 / self.dpi;
                if let Some(window) = self.window.as_ref() {
                    let viewport_w = window.inner_size().width as f32 / self.dpi;
                    let viewport_h = window.inner_size().height as f32 / self.dpi;
                    let hint =
                        self.host
                            .cursor_hint(self.cursor_x, self.cursor_y, viewport_w, viewport_h);
                    use openpencil_shell_native::CursorHint;
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
                // Coalesce: stash the latest cursor for the next
                // paint pass instead of running apply_cursor_move
                // (which mutates drag state + may rebuild widgets)
                // on every input event. Mouse poll rates can hit
                // 1000 Hz on modern trackpads — without this the
                // host runs ~16× more per-event work than the
                // display rate. Apply lands in RedrawRequested.
                self.pending_cursor_move = Some((self.cursor_x, self.cursor_y));
                self.request_redraw(false);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                // Drain queued cursor move so hover state is current before press lands.
                if self.drain_pending_cursor_move() { self.redraw_dirty = true; }
                let consumed = self.host.apply_press(self.cursor_x, self.cursor_y, self.viewport_width, self.viewport_height);
                if let Some(action) = self.host.document_mut().ui.pending_file_action.take() {
                    // ExportImage opens the picker dialog; close any
                    // source overlay first so its hit-test isn't
                    // shadowed (codex CONCERN). ExportImageConfirm +
                    // everything else falls through to run_action.
                    if matches!(action, openpencil_shell_core::document::FileAction::ExportImage) {
                        let ui = &mut self.host.document_mut().ui;
                        ui.file_menu_open = false; ui.file_menu_hover = None;
                        ui.export_dialog_open = true;
                        self.request_redraw(true);
                    } else {
                        persistence::run_action(action, &mut self.host, &mut self.current_path, self.window.as_ref());
                    }
                }
                if consumed { self.request_redraw(true); }
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
                // Drain any cursor moves queued since the last paint
                // BEFORE releasing — otherwise a drag that ended mid-
                // motion would commit using the previous frame's
                // cursor and lose the final position.
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
                // Figma-style routing: PixelDelta (trackpad 2-finger
                // swipe) pans, LineDelta (mouse wheel) zooms. Cmd /
                // Ctrl held promotes pixel-delta to zoom too, since
                // trackpad-only laptops need a way to zoom without
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
                use openpencil_shell_core::document::ReorderDirection;
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
                    // Named-key editor shortcuts only fire when no
                    // Cmd / Ctrl is held — Cmd+Backspace,
                    // Cmd+Delete, Cmd+Arrow, Cmd+Enter, Cmd+Escape
                    // are reserved for OS / browser bindings (move
                    // to trash, cursor-jump in text inputs, etc.)
                    // and shouldn't silently mutate editor state.
                    Key::Named(NamedKey::Backspace) if !self.zoom_modifier => {
                        consumed = self.host.apply_backspace();
                    }
                    Key::Named(NamedKey::Delete) if !self.zoom_modifier && !settings_focused => {
                        consumed = self.host.apply_delete();
                    }
                    Key::Named(NamedKey::Enter) if !self.zoom_modifier => {
                        consumed = self.host.apply_send();
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
                    Key::Named(NamedKey::ArrowRight) if !self.zoom_modifier && !settings_focused => {
                        consumed = self.host.apply_nudge(nudge, 0.0);
                    }
                    // Cmd/Ctrl-gated editor shortcuts. Match on the
                    // logical character to be insensitive to layout
                    // quirks while the modifier is held.
                    // Cmd/Ctrl-letter shortcuts, with NO shift. Shift-
                    // variants (Cmd+Shift+C "paste without formatting",
                    // Cmd+Shift+G "ungroup", etc) stay reserved for
                    // future bindings — TS parity with the
                    // `!e.shiftKey` guards in `use-clipboard-shortcuts`.
                    Key::Character(ref ch) if self.zoom_modifier && !self.shift_modifier => {
                        let lower = ch.to_lowercase();
                        match lower.as_str() {
                            // Cmd+, always allowed — it toggles the
                            // modal itself; closing while focused
                            // also commits via the close path.
                            "," => consumed = self.host.apply_toggle_agent_settings(),
                            "s" => consumed = persistence::handle_save(&mut self.host, &mut self.current_path, self.window.as_ref()),
                            "o" => consumed = persistence::handle_open(&mut self.host, &mut self.current_path, self.window.as_ref()),
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
                            "s" => consumed = persistence::handle_save_as(&mut self.host, &mut self.current_path, self.window.as_ref()),
                            "p" => {
                                persistence::run_action(openpencil_shell_core::document::FileAction::ExportImage, &mut self.host, &mut self.current_path, self.window.as_ref());
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
                            "v" => self
                                .host
                                .apply_set_tool(openpencil_shell_core::document::Tool::Select),
                            "r" => self
                                .host
                                .apply_set_tool(openpencil_shell_core::document::Tool::Rect),
                            "o" => self
                                .host
                                .apply_set_tool(openpencil_shell_core::document::Tool::Ellipse),
                            "l" => self
                                .host
                                .apply_set_tool(openpencil_shell_core::document::Tool::Line),
                            "t" => self
                                .host
                                .apply_set_tool(openpencil_shell_core::document::Tool::Text),
                            "f" => self
                                .host
                                .apply_set_tool(openpencil_shell_core::document::Tool::Frame),
                            "p" => self
                                .host
                                .apply_set_tool(openpencil_shell_core::document::Tool::Pen),
                            "h" => self
                                .host
                                .apply_set_tool(openpencil_shell_core::document::Tool::Hand),
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
                        "[" if !settings_focused => consumed = self.host.apply_reorder(ReorderDirection::Down),
                        "]" if !settings_focused => consumed = self.host.apply_reorder(ReorderDirection::Up),
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
            settings_io::save_if_changed(self.host.document(), before);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Belt-and-suspenders: macOS Cmd+Q / Alt+F4 / window-manager
        // close can deliver `exiting` without `CloseRequested`. Flush
        // any in-progress MCP port draft before snapshotting so a
        // focused-but-uncommitted edit isn't silently dropped.
        self.host.flush_settings_input();
        settings_io::save(self.host.document());
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
