//! P0 probe runner — runs verification (2) cross-API GL state visibility +
//! verification (3) full readback chain. Invoked as a subprocess from
//! `tests/p0_probe.rs` because winit on macOS requires `EventLoop::new()` on
//! the OS main thread (cargo test puts test fns on a worker thread).
//!
//! TRANSIENT (Step 1a P0 CI gate). Reverted before Task 1.
//!
//! Usage: `cargo run --example p0_probe -- <stencil|readback>`
//! Exit code 0 = PASS; non-zero with stderr message = FAIL.

use std::ffi::CString;
use std::num::NonZeroU32;
use std::process::ExitCode;

use glow::HasContext;
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, NotCurrentGlContext};
use glutin::display::{GetGlDisplay, GlDisplay};
use glutin::surface::{GlSurface, SurfaceAttributesBuilder, WindowSurface};
use glutin_winit::DisplayBuilder;
use raw_window_handle::HasWindowHandle;
use skia_safe::gpu::{
    backend_render_targets, direct_contexts, gl as skgl, surfaces, SurfaceOrigin,
};
use skia_safe::{Color, ColorType, Paint, Rect};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const WIDTH: i32 = 64;
const HEIGHT: i32 = 64;

#[derive(Clone, Copy)]
enum Mode {
    StencilVisibility,
    Readback,
}

struct Probe {
    mode: Mode,
    result: Option<Result<(), String>>,
}

impl ApplicationHandler for Probe {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.result = Some(match self.mode {
            Mode::StencilVisibility => run_stencil_visibility(event_loop),
            Mode::Readback => run_readback(event_loop),
        });
        event_loop.exit();
    }

    fn window_event(&mut self, _event_loop: &ActiveEventLoop, _id: WindowId, _ev: WindowEvent) {}
}

struct GlStack {
    _window: Window,
    gl_surface: glutin::surface::Surface<WindowSurface>,
    gl_context: glutin::context::PossiblyCurrentContext,
    glow: glow::Context,
    skia: skia_safe::gpu::DirectContext,
    surface: skia_safe::Surface,
    fb_info: skgl::FramebufferInfo,
}

fn build_stack(event_loop: &ActiveEventLoop) -> Result<GlStack, String> {
    let window_attrs = WindowAttributes::default()
        .with_visible(false)
        .with_inner_size(winit::dpi::PhysicalSize::new(WIDTH as u32, HEIGHT as u32))
        .with_title("p0-probe");

    let template = ConfigTemplateBuilder::new()
        .with_alpha_size(8)
        .with_stencil_size(8);

    let (window, gl_config) = DisplayBuilder::new()
        .with_window_attributes(Some(window_attrs))
        .build(event_loop, template, |configs| {
            configs.into_iter().next().expect("no GL config available")
        })
        .map_err(|e| format!("DisplayBuilder::build failed: {e}"))?;

    let window = window.ok_or("DisplayBuilder returned no window")?;
    let raw_window_handle = window
        .window_handle()
        .map_err(|e| format!("window_handle: {e}"))?
        .as_raw();
    let gl_display = gl_config.display();

    let context_attrs = ContextAttributesBuilder::new()
        .with_context_api(ContextApi::OpenGl(None))
        .build(Some(raw_window_handle));
    let not_current = unsafe {
        gl_display
            .create_context(&gl_config, &context_attrs)
            .map_err(|e| format!("create_context: {e}"))?
    };

    let surface_attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
        raw_window_handle,
        NonZeroU32::new(WIDTH as u32).unwrap(),
        NonZeroU32::new(HEIGHT as u32).unwrap(),
    );
    let gl_surface = unsafe {
        gl_display
            .create_window_surface(&gl_config, &surface_attrs)
            .map_err(|e| format!("create_window_surface: {e}"))?
    };
    let gl_context = not_current
        .make_current(&gl_surface)
        .map_err(|e| format!("make_current: {e}"))?;

    let glow = unsafe {
        glow::Context::from_loader_function(|s| {
            let cs = CString::new(s).unwrap();
            gl_display.get_proc_address(&cs)
        })
    };

    let interface = skgl::Interface::new_load_with(|s| {
        if s == "eglGetCurrentDisplay" {
            return std::ptr::null();
        }
        let cs = CString::new(s).unwrap();
        gl_display.get_proc_address(&cs)
    })
    .ok_or("skia gl::Interface::new_load_with returned None")?;

    let mut skia =
        direct_contexts::make_gl(interface, None).ok_or("DirectContext::make_gl returned None")?;

    let fb_info = skgl::FramebufferInfo {
        fboid: 0,
        // glow::RGBA8 is u32; skia's gl::Enum aliases GrGLenum = c_uint = u32
        // on every platform in `deny.toml`'s target list. Use `as _` instead
        // of `.try_into().unwrap()` so clippy::useless_conversion stays quiet.
        format: glow::RGBA8 as _,
        protected: skia_safe::gpu::Protected::No,
    };
    let backend_rt = backend_render_targets::make_gl((WIDTH, HEIGHT), 0, 8, fb_info);
    let surface = surfaces::wrap_backend_render_target(
        &mut skia,
        &backend_rt,
        SurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
    .ok_or("wrap_backend_render_target returned None")?;

    Ok(GlStack {
        _window: window,
        gl_surface,
        gl_context,
        glow,
        skia,
        surface,
        fb_info,
    })
}

fn run_stencil_visibility(event_loop: &ActiveEventLoop) -> Result<(), String> {
    let mut stack = build_stack(event_loop)?;
    unsafe { stack.glow.enable(glow::STENCIL_TEST) };

    let canvas = stack.surface.canvas();
    canvas.clear(Color::TRANSPARENT);
    let mut paint = Paint::default();
    paint.set_color(Color::RED);
    canvas.draw_rect(
        Rect::from_xywh(0.0, 0.0, WIDTH as f32, HEIGHT as f32),
        &paint,
    );
    stack.skia.flush_and_submit();

    let still_enabled = unsafe { stack.glow.is_enabled(glow::STENCIL_TEST) };
    unsafe { stack.glow.disable(glow::STENCIL_TEST) };
    let _ = stack.gl_surface.swap_buffers(&stack.gl_context);

    if !still_enabled {
        return Err("STENCIL_TEST flag lost across Skia flush — context state NOT shared".into());
    }
    Ok(())
}

fn run_readback(event_loop: &ActiveEventLoop) -> Result<(), String> {
    let mut stack = build_stack(event_loop)?;

    // c0. save current READ_FRAMEBUFFER binding
    let prev_read_fb = unsafe { stack.glow.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING) };

    // c1. draw red into Skia surface
    let canvas = stack.surface.canvas();
    canvas.clear(Color::from_argb(255, 0, 0, 0));
    let mut red = Paint::default();
    red.set_color(Color::RED);
    red.set_anti_alias(false);
    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, WIDTH as f32, HEIGHT as f32), &red);
    stack.skia.flush_and_submit();
    unsafe { stack.glow.finish() };

    // c2-c3. resolve Skia FBO id (we wrapped fboid=0 = window default FBO)
    let fboid = stack.fb_info.fboid;
    let target_fb = NonZeroU32::new(fboid).map(glow::NativeFramebuffer);

    // c4. RAII restore prev binding (panic-safe)
    let glow_ref = &stack.glow;
    let _restore = scopeguard::guard(prev_read_fb, |prev| unsafe {
        let nfb = NonZeroU32::new(prev as u32).map(glow::NativeFramebuffer);
        glow_ref.bind_framebuffer(glow::READ_FRAMEBUFFER, nfb);
    });

    // c5. bind Skia FBO + finish
    unsafe {
        stack
            .glow
            .bind_framebuffer(glow::READ_FRAMEBUFFER, target_fb)
    };
    unsafe { stack.glow.finish() };

    // c6. readback
    let mut buf = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
    unsafe {
        stack.glow.read_pixels(
            0,
            0,
            WIDTH,
            HEIGHT,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut buf)),
        );
    }

    // c7. assert center pixel is red
    let cx = WIDTH / 2;
    let cy = HEIGHT / 2;
    let i = ((cy * WIDTH + cx) * 4) as usize;
    let center = (buf[i], buf[i + 1], buf[i + 2], buf[i + 3]);
    drop(_restore);
    let _ = stack.gl_surface.swap_buffers(&stack.gl_context);

    if center.0 < 200 || center.1 > 80 || center.2 > 80 {
        return Err(format!(
            "center pixel not red — got rgba({}, {}, {}, {}); buf[0..4]={:?}",
            center.0,
            center.1,
            center.2,
            center.3,
            &buf[..4]
        ));
    }
    Ok(())
}

fn main() -> ExitCode {
    let mode = match std::env::args().nth(1).as_deref() {
        Some("stencil") => Mode::StencilVisibility,
        Some("readback") => Mode::Readback,
        other => {
            eprintln!("usage: readback <stencil|readback>; got {:?}", other);
            return ExitCode::from(2);
        }
    };

    let event_loop = match EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            eprintln!("EventLoop::new failed: {e}");
            return ExitCode::from(3);
        }
    };
    let mut probe = Probe { mode, result: None };
    if let Err(e) = event_loop.run_app(&mut probe) {
        eprintln!("run_app failed: {e}");
        return ExitCode::from(4);
    }
    match probe.result {
        Some(Ok(())) => ExitCode::SUCCESS,
        Some(Err(e)) => {
            eprintln!("FAIL: {e}");
            ExitCode::from(1)
        }
        None => {
            eprintln!("FAIL: handler never produced a result (window never resumed?)");
            ExitCode::from(5)
        }
    }
}
