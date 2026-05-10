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
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

const INSPECTOR_WIDTH: f32 = 280.0;

/// Paint pass — clear to white, then dispatch the inspector widgets
/// through `WidgetHostNative`. Pulled into a free function so the
/// initial `Resumed` paint and `RedrawRequested` redraws share the
/// exact same draw list (same pattern as `basic_window`).
fn paint_inspector(
    ctx: &mut SharedSkiaContext,
    backend: &mut NativeBackend,
    host: &WidgetHostNative,
) {
    ctx.begin_frame();
    ctx.with_frame(|canvas, _glow| {
        // Clear via Skia first so the framebuffer alpha is reset
        // before widget paints sit on top.
        canvas.clear(skia_safe::Color::WHITE);

        let mut frame = NativeFrameBackend::new(backend, canvas);
        // Belt-and-braces: also clear via the trait fill_rect so the
        // shell-web and shell-native paint orders match exactly. The
        // canvas.clear above is a no-op-equivalent prequel.
        frame.fill_rect(
            Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(960.0, 640.0),
            },
            Color::WHITE,
        );
        host.paint(&mut frame, INSPECTOR_WIDTH);
    });
    ctx.present();
}

struct InspectorApp {
    window: Option<Window>,
    ctx: Option<SharedSkiaContext>,
    backend: Option<NativeBackend>,
    host: WidgetHostNative,
    error: Option<SharedSkiaError>,
}

impl InspectorApp {
    fn new() -> Self {
        Self {
            window: None,
            ctx: None,
            backend: None,
            host: WidgetHostNative::new(),
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
            .with_title("OpenPencil — inspector_window (Step 1b §1.4 native)")
            .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(err) => {
                eprintln!("inspector_window: create_window failed: {err}");
                event_loop.exit();
                return;
            }
        };

        let dpi = window.scale_factor() as f32;
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
            paint_inspector(ctx, backend, &self.host);
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
                if let Some(window) = self.window.as_ref() {
                    window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(backend) = self.backend.as_mut() {
                    backend.set_dpi(scale_factor as f32);
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
                    paint_inspector(ctx, backend, &self.host);
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
