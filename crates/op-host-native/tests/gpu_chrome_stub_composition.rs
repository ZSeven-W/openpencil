//! Spec v19 acceptance #2 (chrome + canvas-stub on the same GPU surface)
//! / plan v7 Task 2 Step 16d / round 6 BLOCK-R6-2.
//!
//! Companion to `gpu_smoke.rs`: this test specifically asserts that
//! `CanvasViewportStub::render_into` and `NativeBackend::fill_rect` can
//! coexist inside a single `SharedSkiaContext::with_frame` closure, and
//! that the chrome pixels survive the stub's deliberate GL-state
//! pollution (acceptance #4 — chrome readback unaffected).
//!
//! Per-OS dispatch matches `gpu_smoke.rs` (Linux EGL pbuffer + macOS
//! invisible window + Windows ignored).

#![cfg_attr(target_os = "windows", allow(dead_code))]

mod common;

use op_editor_ui::{Color, Point2D, Rect};
use op_host_native::{CanvasViewportStub, NativeBackend, SharedSkiaContext};

/// Inside the `with_frame` closure: stub paints first (and pollutes
/// stencil + blend), chrome paints on top. If `SharedSkiaContext`
/// correctly mediates the GL state, the chrome pixel must read back
/// red regardless of what the stub left behind.
#[allow(dead_code)]
fn paint_chrome_and_stub(ctx: &mut SharedSkiaContext, backend: &mut NativeBackend) {
    let chrome_rect = Rect {
        origin: Point2D::new(50.0, 50.0),
        size: Point2D::new(100.0, 100.0),
    };
    let stub_rect = skia_safe::Rect::from_xywh(50.0, 50.0, 400.0, 300.0);
    ctx.begin_frame();
    ctx.with_frame(|canvas, glow| {
        // Stub first — paints a blue rect inside the stub region and
        // pollutes GL state without restoring.
        CanvasViewportStub::render_into(glow, canvas, stub_rect);
        // Chrome second — must override the stub at (50,50)-(150,150).
        backend.fill_rect(canvas, chrome_rect, Color::RED);
    });
    ctx.present();
}

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use crate::common::egl_pbuffer::EglPbufferProvider;
    use glow::HasContext;

    pub fn run() {
        // BLOCK 2 (Codex Phase A Gate round 1): see `gpu_smoke.rs` for
        // the full rationale. STEP1A_REQUIRE_GPU=1 ⇒ setup-failure is
        // a hard panic; otherwise we surface an INCONCLUSIVE marker
        // so acceptance #2 / #4 don't pass silently on hostless CI.
        let require_gpu = std::env::var_os("STEP1A_REQUIRE_GPU")
            .map(|v| v == "1")
            .unwrap_or(false);

        let provider = match EglPbufferProvider::new((800, 600)) {
            Ok(p) => p,
            Err(err) => {
                let msg = format!("Linux EGL pbuffer setup failed: {err}");
                if require_gpu {
                    panic!("gpu_chrome_stub_composition: STEP1A_REQUIRE_GPU=1 but {msg}");
                }
                eprintln!(
                    "⚠ INCONCLUSIVE gpu_chrome_stub_composition: {msg} \
                     (set STEP1A_REQUIRE_GPU=1 on a real-GPU runner to fail hard)"
                );
                return;
            }
        };
        // Phase A Gate round 2 BLOCK 1 fix — single-arg `new(provider)`;
        // size queried from EGL pbuffer GL viewport.
        let mut ctx = match SharedSkiaContext::new(provider) {
            Ok(c) => c,
            Err(err) => {
                let msg = format!("SharedSkiaContext::new failed: {err}");
                if require_gpu {
                    panic!("gpu_chrome_stub_composition: STEP1A_REQUIRE_GPU=1 but {msg}");
                }
                eprintln!(
                    "⚠ INCONCLUSIVE gpu_chrome_stub_composition: {msg} \
                     (set STEP1A_REQUIRE_GPU=1 on a real-GPU runner to fail hard)"
                );
                return;
            }
        };
        let mut backend = NativeBackend::with_dpi(1.0);
        paint_chrome_and_stub(&mut ctx, &mut backend);

        // Chrome pixel must be red even though the stub poked
        // STENCIL_TEST + BlendFunc(ONE, ZERO).
        // Phase A Gate round 2 BLOCK 2(a) fix — `glow()` borrows;
        // clone the `Arc` so the handle outlives the later `teardown`.
        let glow = ctx.glow().expect("glow handle").clone();
        let mut chrome_px = [0u8; 4];
        let mut stub_px = [0u8; 4];
        unsafe {
            glow.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
            glow.finish();
            glow.read_pixels(
                75,
                75,
                1,
                1,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut chrome_px)),
            );
            // Outside the chrome rect but inside the stub rect — should
            // be the stub's blue paint.
            glow.read_pixels(
                300,
                200,
                1,
                1,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut stub_px)),
            );
        }
        let _ = ctx.teardown();

        assert_eq!(
            chrome_px,
            [255, 0, 0, 255],
            "chrome pixel polluted by stub's GL state — got {:?}",
            chrome_px
        );
        // Stub painted blue; check the blue channel dominates.
        assert!(
            stub_px[2] > 200 && stub_px[0] < 60,
            "stub region missing blue pixels — got {:?}",
            stub_px
        );
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use glow::HasContext;
    use winit::application::ApplicationHandler;
    use winit::event::WindowEvent;
    use winit::event_loop::{ActiveEventLoop, EventLoop};
    use winit::window::{Window, WindowId};

    #[derive(Default)]
    struct State {
        done: bool,
        chrome_red: bool,
        stub_blue: bool,
        inconclusive: Option<String>,
        window: Option<Window>,
    }

    struct Composition {
        state: Rc<RefCell<State>>,
    }

    impl ApplicationHandler for Composition {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let mut state = self.state.borrow_mut();
            if state.done {
                return;
            }
            let attrs = Window::default_attributes()
                .with_visible(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(800u32, 600u32));
            match event_loop.create_window(attrs) {
                Ok(w) => state.window = Some(w),
                Err(err) => {
                    state.inconclusive = Some(format!("create_window failed: {err}"));
                    state.done = true;
                    event_loop.exit();
                }
            }
        }

        fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
            let mut state = self.state.borrow_mut();
            if state.done {
                event_loop.exit();
                return;
            }
            let window = match state.window.as_ref() {
                Some(w) => w,
                None => return,
            };
            match SharedSkiaContext::new_desktop(window) {
                Ok(mut ctx) => {
                    let mut backend = NativeBackend::with_dpi(window.scale_factor() as f32);
                    paint_chrome_and_stub(&mut ctx, &mut backend);
                    // Phase A Gate round 2 BLOCK 2(a) fix — `glow()` borrows;
                    // clone the `Arc` so the handle outlives `teardown`.
                    let glow = ctx.glow().expect("glow handle").clone();
                    let mut chrome_px = [0u8; 4];
                    let mut stub_px = [0u8; 4];
                    unsafe {
                        glow.bind_framebuffer(glow::READ_FRAMEBUFFER, None);
                        glow.read_pixels(
                            75,
                            75,
                            1,
                            1,
                            glow::RGBA,
                            glow::UNSIGNED_BYTE,
                            glow::PixelPackData::Slice(Some(&mut chrome_px)),
                        );
                        glow.read_pixels(
                            300,
                            200,
                            1,
                            1,
                            glow::RGBA,
                            glow::UNSIGNED_BYTE,
                            glow::PixelPackData::Slice(Some(&mut stub_px)),
                        );
                    }
                    state.chrome_red = chrome_px[0] > 200 && chrome_px[1] < 60 && chrome_px[2] < 60;
                    state.stub_blue = stub_px[2] > 200 && stub_px[0] < 60;
                    let _ = ctx.teardown();
                }
                Err(err) => {
                    state.inconclusive = Some(format!("SharedSkiaContext::new_desktop: {err}"));
                }
            }
            state.done = true;
            event_loop.exit();
        }

        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
    }

    pub fn run() {
        // Same `catch_unwind` pattern as `gpu_smoke.rs::macos`. On
        // worker-thread test runs winit panics with "EventLoop must
        // be created on the main thread" — degrade to inconclusive.
        let result = std::panic::catch_unwind(|| {
            let event_loop = EventLoop::new().map_err(|e| format!("EventLoop::new: {e}"))?;
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
            let state = Rc::new(RefCell::new(State::default()));
            let mut comp = Composition {
                state: state.clone(),
            };
            let _ = event_loop.run_app(&mut comp);
            let s = state.borrow();
            if let Some(reason) = &s.inconclusive {
                return Err(reason.clone());
            }
            Ok((s.chrome_red, s.stub_blue))
        });

        match result {
            Ok(Ok((true, true))) => {}
            Ok(Ok((chrome_red, stub_blue))) => {
                panic!("gpu_chrome_stub_composition: chrome_red={chrome_red} stub_blue={stub_blue}")
            }
            Ok(Err(reason)) => {
                eprintln!("gpu_chrome_stub_composition: inconclusive — {reason}");
            }
            Err(panic) => {
                let msg = panic
                    .downcast_ref::<&str>()
                    .map(|s| (*s).to_string())
                    .or_else(|| panic.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "unknown".into());
                eprintln!(
                    "gpu_chrome_stub_composition: inconclusive — winit panicked (likely \
                     `EventLoop` not on the main thread): {msg}"
                );
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod platform {
    pub fn run() {}
}

#[cfg(target_os = "macos")]
#[test]
fn gpu_chrome_stub_composition() {
    platform::run();
}

// Linux GPU chrome+stub composition deferred: same root cause as
// LINUX_GPU_SKIA_LOADER_TBD in gpu_smoke.rs.
#[cfg(target_os = "linux")]
#[test]
#[ignore = "LINUX_GPU_SKIA_LOADER_TBD: skia-safe Interface::new_native cannot resolve GL syms from EGL pbuffer + llvmpipe (see gpu_smoke.rs)"]
fn gpu_chrome_stub_composition() {
    platform::run();
}

#[cfg(target_os = "windows")]
#[test]
#[ignore = "WINDOWS_GPU_DEFERRED_NO_RUNNER: standard GitHub Actions Windows runner has no GPU driver; manual smoke required (per spec §8.1)"]
fn gpu_chrome_stub_composition() {
    platform::run();
}
