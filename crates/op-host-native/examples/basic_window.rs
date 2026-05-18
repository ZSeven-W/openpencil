//! Spec v19 §1.2 acceptance #1 — basic_window demo.
//!
//! Phase C Task 4 deliverable: a minimal winit + `SharedSkiaContext` +
//! `NativeBackend` integration that paints chrome (rect / text / box
//! outline) on every frame.
//!
//! NOTE: this demo used to also route pointer events through Jian's
//! `jian_host_desktop::pointer::PointerTranslator`, but that crate is
//! pinned to the upstream `winit` package while the desktop host now
//! builds on the `casement` winit fork — the two `winit::event` types
//! are not interchangeable. The translator call was discarded smoke-
//! test code (`let _ = jian_event`), so it has simply been dropped.
//!
//! The macOS / Linux / Windows runtime is exercised by maintainers manually
//! (`notes/step-1a-{macos, linux, windows}-manual-smoke.md`); CI only
//! verifies that `cargo build --examples --workspace` compiles on every
//! desktop OS.
//!
//! Run with:
//! ```text
//! cargo run -p op-host-native --example basic_window
//! ```
//!
//! The window should display:
//! - White background (chrome canvas clear).
//! - Red filled rect at (50, 50) - 100x100.
//! - "Hello 你好" black text at (50, 200).
//! - Blue stroked rect outline at (200, 50) - 200x150.
//!
//! Closing the window must run `SharedSkiaContext::teardown` exactly
//! once (the `Drop` impl is the safety net) and exit cleanly.

#![cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]

use jian_core::scene::Color as JianColor;
use op_editor_ui::{Color, Point2D, Rect, TextLayout};
use op_host_native::{NativeBackend, SharedSkiaContext, SharedSkiaError};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

/// Per-frame chrome paint. Pulled into a free function so both the
/// initial `Resumed` paint and `RedrawRequested` redraws share the
/// exact same draw list (spec §5.2.1 frame-scoped backend pattern).
fn paint_chrome(ctx: &mut SharedSkiaContext, backend: &mut NativeBackend) {
    let rect_fill = Rect {
        origin: Point2D::new(50.0, 50.0),
        size: Point2D::new(100.0, 100.0),
    };
    let rect_outline = Rect {
        origin: Point2D::new(200.0, 50.0),
        size: Point2D::new(200.0, 150.0),
    };
    let text = TextLayout::single_run(
        "Hello 你好",
        "",
        24.0,
        JianColor::rgb(0, 0, 0),
        Point2D::new(50.0, 200.0),
    );

    ctx.begin_frame();
    ctx.with_frame(|canvas, _glow| {
        // White background — clear the framebuffer through Skia.
        canvas.clear(skia_safe::Color::WHITE);

        // 1. Filled red rectangle (acceptance #1 chrome rect).
        backend.fill_rect(canvas, rect_fill, Color::RED);

        // 2. Black "Hello 你好" — exercises CJK path through jian-skia
        //    `textlayout` ParagraphBuilder.
        backend.draw_text(canvas, &text, Point2D::ZERO);

        // 3. Blue stroked box (acceptance #1 chrome outline).
        backend.stroke_rect(canvas, rect_outline, Color::BLUE, 2.0);
    });
    ctx.present();
}

struct BasicWindowApp {
    window: Option<Window>,
    ctx: Option<SharedSkiaContext>,
    backend: Option<NativeBackend>,
    /// Fatal teardown / surface error captured for post-loop diagnosis.
    error: Option<SharedSkiaError>,
}

impl BasicWindowApp {
    fn new() -> Self {
        Self {
            window: None,
            ctx: None,
            backend: None,
            error: None,
        }
    }
}

impl ApplicationHandler for BasicWindowApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("OpenPencil — basic_window (Step 1a §1.2 acceptance #1)")
            .with_inner_size(winit::dpi::LogicalSize::new(800u32, 600u32));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => w,
            Err(err) => {
                eprintln!("basic_window: create_window failed: {err}");
                event_loop.exit();
                return;
            }
        };

        // Build the GL stack + Skia context bound to the new window.
        let dpi = window.scale_factor() as f32;
        match SharedSkiaContext::new_desktop(&window) {
            Ok(ctx) => {
                self.ctx = Some(ctx);
                self.backend = Some(NativeBackend::with_dpi(dpi));
            }
            Err(err) => {
                eprintln!("basic_window: SharedSkiaContext::new_desktop failed: {err}");
                self.error = Some(err);
                event_loop.exit();
                return;
            }
        }
        self.window = Some(window);

        // First paint so the window has visible content even before
        // the OS schedules a `RedrawRequested`. Subsequent redraws come
        // through the event match below.
        if let (Some(ctx), Some(backend)) = (self.ctx.as_mut(), self.backend.as_mut()) {
            paint_chrome(ctx, backend);
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
                        eprintln!("basic_window: resize failed: {err}");
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
                    paint_chrome(ctx, backend);
                }
            }
            _ => {}
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Idempotent teardown — the `Drop` impl on `SharedSkiaContext`
        // is the safety net, but explicit teardown gives the demo a
        // visible "no leaks" signal at exit (acceptance #6).
        if let Some(mut ctx) = self.ctx.take() {
            if let Err(err) = ctx.teardown() {
                eprintln!("basic_window: teardown failed: {err}");
            }
        }
        self.backend.take();
        self.window.take();
    }
}

fn main() {
    // Per-frame `#[instrument]` spans are emitted but no subscriber is
    // initialised by default — the demo deliberately stays free of
    // dev-only crates. Add `tracing_subscriber::fmt::try_init()` here
    // (and drop it as a dev-dep) if you want to inspect spans.

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(err) => {
            eprintln!("basic_window: EventLoop::new failed: {err}");
            std::process::exit(1);
        }
    };
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut app = BasicWindowApp::new();
    if let Err(err) = event_loop.run_app(&mut app) {
        eprintln!("basic_window: run_app exited with error: {err}");
        std::process::exit(1);
    }
    if let Some(err) = app.error {
        eprintln!("basic_window: fatal error during run: {err}");
        std::process::exit(1);
    }
}
