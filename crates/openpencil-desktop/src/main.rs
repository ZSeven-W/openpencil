//! OpenPencil desktop runner.
//!
//! winit + skia-safe binary that wires `WidgetHostNative` (from
//! `openpencil-shell-native`) into a real OS window. Owns the
//! event loop, GL surface, DPI tracking, animation timer, and the
//! cursor / input plumbing — everything that's specific to "OS
//! window with skia GL surface" and not shared with the wasm32
//! browser host or the (Step 1f) mobile shells.
//!
//! ### Run
//!
//! ```text
//! cargo run -p openpencil-desktop --release
//! ```
//!
//! ### Layout
//!
//! - `WidgetHostNative` (in `openpencil-shell-native`) owns the
//!   `Document`, paints widgets, and routes input — platform-free
//!   beyond the `RenderBackend` impl.
//! - `SharedSkiaContext` + `NativeBackend` (also in shell-native)
//!   wrap jian-skia + glutin so the same widget code paints on
//!   macOS / Linux / Windows.
//! - This binary glues winit's `ApplicationHandler` events
//!   (Resumed / Resized / RedrawRequested / CursorMoved /
//!   MouseInput / MouseWheel / PinchGesture / KeyboardInput /
//!   ModifiersChanged / ScaleFactorChanged) onto the host's
//!   `apply_*` methods + `paint`.
//!
//! ### Mobile (iOS / Android) — Step 1f
//!
//! The desktop crate is gated to macOS / Linux / Windows. Mobile
//! shells will live in their own crates with platform-specific
//! `GlContextProvider` impls (`EaglProvider` on iOS,
//! `AndroidEglProvider` on Android — both already stubbed in
//! shell-native; spec §11 + 2026-05-10 directive). The widget glue
//! `WidgetHostNative` is platform-agnostic; mobile runners reuse
//! `host.paint(&mut frame, w, h)` once their provider ships real
//! `make_current` / `swap_buffers`.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend};
use openpencil_shell_native::{
    NativeBackend, NativeFrameBackend, SharedSkiaContext, SharedSkiaError, WidgetHostNative,
};
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{
    ElementState, KeyEvent, MouseButton, MouseScrollDelta, StartCause, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

/// Default starting viewport — matches `with_inner_size` below so
/// the first frame has the right layout. Resize is handled in the
/// `Resized` arm: we cache the LOGICAL viewport so the next paint
/// matches the new size at DPI-independent coordinates.
const INITIAL_VIEWPORT_W: f32 = 1440.0;
const INITIAL_VIEWPORT_H: f32 = 900.0;

/// Paint pass — clear, scale by DPI, dispatch to the widget host.
/// The matrix `reset_matrix` is required because skia's canvas
/// matrix is stateful across `with_frame` invocations; without it
/// `scale(dpi, dpi)` would compound per redraw.
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
    /// Cached LOGICAL viewport size — refreshed on Resumed +
    /// Resized so the host paints at DPI-independent coordinates.
    /// The canvas is scaled by DPI inside `paint`; the underlying
    /// skia surface (set up by `SharedSkiaContext`) is physical.
    viewport_width: f32,
    viewport_height: f32,
    /// Last cursor position (logical, top-left origin). winit's
    /// `CursorMoved` reports a `PhysicalPosition`; we divide by
    /// the cached DPI so `WidgetHostNative::apply_*` receives
    /// the same logical coordinate space the widgets paint in.
    cursor_x: f32,
    cursor_y: f32,
    /// Cached scale factor — refreshed on Resumed +
    /// `ScaleFactorChanged`. Drives the DPI scale on the canvas
    /// and the physical→logical cursor conversion.
    dpi: f32,
    /// Cmd / Ctrl held — promotes 2-finger swipe from pan to zoom
    /// AND gates editor shortcuts (Cmd+D duplicate, etc.).
    zoom_modifier: bool,
    /// Shift held — promotes arrow-key nudge from 1 px to 10 px.
    shift_modifier: bool,
    /// Monotonic clock anchor — `Instant.elapsed().as_millis()`
    /// from this is fed into `WidgetHostNative::set_now_ms` so
    /// `jian_core::anim::blink_visible` can drive the caret blink
    /// (and any future time-based UI animation).
    clock_start: Instant,
    /// Cached custom rotate cursor — built once at `Resumed` and
    /// reused for every CursorHint::Rotate to avoid re-decoding
    /// the bitmap every move. None until the event loop is ready.
    rotate_cursor: Option<winit::window::CustomCursor>,
    error: Option<SharedSkiaError>,
}

impl DesktopApp {
    fn new() -> Self {
        Self {
            window: None,
            ctx: None,
            backend: None,
            host: WidgetHostNative::new(),
            viewport_width: INITIAL_VIEWPORT_W,
            viewport_height: INITIAL_VIEWPORT_H,
            cursor_x: 0.0,
            cursor_y: 0.0,
            dpi: 1.0,
            zoom_modifier: false,
            shift_modifier: false,
            clock_start: Instant::now(),
            rotate_cursor: None,
            error: None,
        }
    }
}

impl ApplicationHandler for DesktopApp {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        // When the WaitUntil deadline fires, the next redraw paints
        // the next caret-blink phase. winit doesn't auto-redraw on
        // ResumeTimeReached, so we have to request it here.
        if matches!(cause, StartCause::ResumeTimeReached { .. }) {
            if let Some(window) = self.window.as_ref() {
                window.request_redraw();
            }
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
        match event {
            WindowEvent::CloseRequested => {
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
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.dpi = scale_factor as f32;
                if let Some(backend) = self.backend.as_mut() {
                    backend.set_dpi(scale_factor as f32);
                }
                // Refresh the cached logical viewport from the
                // window's current physical size — the physical
                // size doesn't change when DPI flips on the same
                // monitor move, but the logical (physical/dpi)
                // conversion does, so input + paint coordinate
                // spaces would otherwise diverge until the user
                // resized the window.
                if let Some(window) = self.window.as_ref() {
                    let size = window.inner_size();
                    self.viewport_width = size.width as f32 / self.dpi;
                    self.viewport_height = size.height as f32 / self.dpi;
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
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
                let hover_changed = self.host.update_layer_hover(
                    self.cursor_x,
                    self.cursor_y,
                    self.viewport_height,
                );
                if self.host.apply_cursor_move(self.cursor_x, self.cursor_y) || hover_changed {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                let consumed = self.host.apply_press(
                    self.cursor_x,
                    self.cursor_y,
                    self.viewport_width,
                    self.viewport_height,
                );
                if consumed {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                let consumed = self
                    .host
                    .apply_release_with_viewport(self.viewport_width, self.viewport_height);
                if consumed {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
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
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
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
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
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
                    Key::Named(NamedKey::Delete) if !self.zoom_modifier => {
                        consumed = self.host.apply_delete();
                    }
                    Key::Named(NamedKey::Enter) if !self.zoom_modifier => {
                        consumed = self.host.apply_send();
                    }
                    Key::Named(NamedKey::Escape) if !self.zoom_modifier => {
                        consumed = self.host.apply_escape();
                    }
                    Key::Named(NamedKey::ArrowUp) if !self.zoom_modifier => {
                        consumed = self.host.apply_nudge(0.0, -nudge);
                    }
                    Key::Named(NamedKey::ArrowDown) if !self.zoom_modifier => {
                        consumed = self.host.apply_nudge(0.0, nudge);
                    }
                    Key::Named(NamedKey::ArrowLeft) if !self.zoom_modifier => {
                        consumed = self.host.apply_nudge(-nudge, 0.0);
                    }
                    Key::Named(NamedKey::ArrowRight) if !self.zoom_modifier => {
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
                            "d" => consumed = self.host.apply_duplicate(),
                            "a" => consumed = self.host.apply_select_all(),
                            "c" => consumed = self.host.apply_copy(),
                            "x" => consumed = self.host.apply_cut(),
                            "v" => consumed = self.host.apply_paste(),
                            _ => {}
                        }
                    }
                    // `[` / `]` — z-order reorder (no modifier).
                    Key::Character(ref ch) if !self.zoom_modifier => match ch.as_str() {
                        "[" => consumed = self.host.apply_reorder(ReorderDirection::Down),
                        "]" => consumed = self.host.apply_reorder(ReorderDirection::Up),
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
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
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
