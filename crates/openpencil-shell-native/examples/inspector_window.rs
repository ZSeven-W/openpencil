//! Step 1b §1.4 native inspector demo — paints the four shell-core
//! widgets (Tree / PropertyRow / Dropdown / TextInput) through
//! `WidgetHostNative` + `NativeFrameBackend`. Visually mirrors the
//! shell-web `mount()` first frame so the cross-platform widget
//! claim from spec §1.4 is concrete: same widget code, same paint
//! output on macOS / Linux / Windows desktop and on
//! wasm32-unknown-unknown browsers.
//!
//! ### Run (desktop)
//! ```text
//! cargo run -p openpencil-shell-native --example inspector_window
//! ```
//!
//! ### Mobile (iOS / Android) — Step 1f
//! This example is desktop-only (winit + `SharedSkiaContext::
//! new_desktop`). Mobile shells will land their own runners using
//! the platform `GlContextProvider` (`EaglProvider` on iOS,
//! `AndroidEglProvider` on Android — both zero-sized placeholders
//! in shell-native today; spec §11 + 2026-05-10 user directive
//! "安卓和ios 不需要 ipc / 本地 cli — 只需要 custom provider").
//! The widget glue (`WidgetHostNative` + `NativeFrameBackend`) is
//! platform-agnostic; mobile runners reuse the same
//! `host.paint(&mut frame, width)` once their provider ships real
//! `make_current` / `swap_buffers` impls.
//!
//! The window should display a 280 px column on the left containing
//! a 3-row tree (Frame / Title / Button — first row selected blue),
//! a "Width 960" property row, a "Normal" dropdown, and a "Frame 1"
//! text input. CI verifies `cargo build --examples` only — visual
//! verification is the Phase E manual smoke responsibility.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

use openpencil_shell_core::{Color, Point2D, Rect, RenderBackend};
use openpencil_shell_native::{
    NativeBackend, NativeFrameBackend, SharedSkiaContext, SharedSkiaError, WidgetHostNative,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Default starting viewport — matches `with_inner_size` below so
/// the first frame has the right layout. Resize is handled in the
/// `Resized` arm: we re-cache the host viewport via `set_viewport`
/// so the next paint matches the new physical size.
const INITIAL_VIEWPORT_W: f32 = 1440.0;
const INITIAL_VIEWPORT_H: f32 = 900.0;

/// Paint pass — clear to white, then dispatch the editor-UI
/// composition through `WidgetHostNative`. Pulled into a free
/// function so the initial `Resumed` paint and `RedrawRequested`
/// redraws share the exact same draw list (same pattern as
/// `basic_window`).
fn paint_inspector(
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
        // Skia canvas matrix is stateful across `with_frame`
        // invocations — without an explicit reset, every redraw
        // applies `scale(dpi, dpi)` on top of the previous frame's
        // matrix, doubling content size per click. Reset to
        // identity so the per-frame scale is absolute, not
        // cumulative. (Step 4 stop-hook fix: "click → 突然放大".)
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

struct InspectorApp {
    window: Option<Window>,
    ctx: Option<SharedSkiaContext>,
    backend: Option<NativeBackend>,
    host: WidgetHostNative,
    /// Cached LOGICAL viewport size — refreshed on Resumed +
    /// Resized so the host paints into the right rect at logical
    /// (DPI-independent) coordinates. The canvas is scaled by DPI
    /// inside `paint_inspector`, so widget paint coords are
    /// logical px and the surface (set up by `SharedSkiaContext`)
    /// is physical.
    viewport_width: f32,
    viewport_height: f32,
    /// Last cursor position (logical, top-left origin). winit
    /// `CursorMoved` reports a `PhysicalPosition`; we divide by
    /// the cached DPI so `WidgetHostNative::apply_click` matches
    /// the same logical coordinate space the widget paints used.
    cursor_x: f32,
    cursor_y: f32,
    /// Cached scale factor — refreshed on Resumed +
    /// `ScaleFactorChanged`. Used to scale the canvas + convert
    /// physical cursor positions to logical.
    dpi: f32,
    /// Cmd / Ctrl held — promotes 2-finger swipe from pan to zoom.
    zoom_modifier: bool,
    error: Option<SharedSkiaError>,
}

impl InspectorApp {
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
            error: None,
        }
    }
}

impl ApplicationHandler for InspectorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("OpenPencil — inspector_window (Step 3 native)")
            .with_inner_size(winit::dpi::LogicalSize::new(
                INITIAL_VIEWPORT_W as u32,
                INITIAL_VIEWPORT_H as u32,
            ));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(err) => {
                eprintln!("inspector_window: create_window failed: {err}");
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
                eprintln!("inspector_window: SharedSkiaContext::new_desktop failed: {err}");
                self.error = Some(err);
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);

        if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
            paint_inspector(
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
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(ctx) = self.ctx.as_mut() {
                    if let Err(err) = ctx.resize(size.width, size.height) {
                        eprintln!("inspector_window: resize failed: {err}");
                        self.error = Some(err);
                        event_loop.exit();
                    }
                }
                // Cache LOGICAL size — divide physical by DPI so
                // widget paint coords stay DPI-independent.
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
            }
            WindowEvent::RedrawRequested => {
                if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
                    paint_inspector(
                        ctx,
                        backend,
                        &self.host,
                        self.viewport_width,
                        self.viewport_height,
                        self.dpi,
                    );
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                // winit `position` is PHYSICAL — convert to logical
                // so widget hit-tests + drag deltas match the
                // coordinate space widgets paint in.
                self.cursor_x = position.x as f32 / self.dpi;
                self.cursor_y = position.y as f32 / self.dpi;
                if self.host.apply_cursor_move(self.cursor_x, self.cursor_y) {
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
                let consumed = self.host.apply_release_with_viewport(
                    self.viewport_width,
                    self.viewport_height,
                );
                if consumed {
                    if let Some(window) = self.window.as_ref() {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // Figma-style routing: PixelDelta (trackpad
                // 2-finger swipe) pans, LineDelta (mouse wheel)
                // zooms. Cmd / Ctrl held promotes a pixel-delta
                // gesture to zoom too, since trackpad-only laptops
                // need a way to zoom without a pinch sensor.
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
                // macOS pinch — emit as zoom centered on cursor.
                // `delta` is a unitless ratio per pinch tick; scale
                // it up so it feels comparable to wheel zoom.
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
                // Track Cmd / Ctrl so 2-finger swipe can be
                // promoted to zoom while held.
                let state = mods.state();
                self.zoom_modifier =
                    state.super_key() || state.control_key();
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
                let mut consumed = false;
                match logical_key {
                    Key::Named(NamedKey::Backspace) => {
                        consumed = self.host.apply_backspace();
                    }
                    Key::Named(NamedKey::Enter) => {
                        consumed = self.host.apply_send();
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
                eprintln!("inspector_window: teardown failed: {err}");
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
            eprintln!("inspector_window: EventLoop::new failed: {err}");
            std::process::exit(1);
        }
    };
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut app = InspectorApp::new();
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("inspector_window: run_app exited with error: {err}");
        std::process::exit(1);
    }
    if let Some(err) = app.error {
        eprintln!("inspector_window: fatal error during run: {err}");
        std::process::exit(1);
    }
}
