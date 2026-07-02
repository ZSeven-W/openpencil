//! GL context provider abstraction (spec v19 §3.1).
//!
//! Trait abstraction over the GL stack so `SharedSkiaContext` is reusable
//! across desktop (glutin) and mobile (EAGL / Android EGL — Step 1f). The
//! trait deliberately does **not** carry a `Send` bound: glutin's
//! `PossiblyCurrentContext` is `!Send` (thread-current binding), and the
//! shell event loop runs on the main thread.
//!
//! Step 1a ships only [`GlutinProvider`] (desktop). iOS / Android stubs
//! exist as compile-time placeholders so the public API surface is frozen
//! before Step 1f real implementations land.

#[cfg(feature = "gl-host")]
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
use std::error::Error;
use std::sync::Arc;

/// Errors raised by GL context providers.
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    /// Underlying glutin / EGL / EAGL error wrapped as a string for
    /// cross-platform uniformity (each backend uses different error types).
    #[error("GL context provider failed: {0}")]
    Failure(String),
    /// The platform has created a window, but its content surface is
    /// still the placeholder startup size. Retrying after the first
    /// real resize/redraw avoids entering native GL with an invalid
    /// drawable.
    #[error("GL window surface is not ready: {width}x{height}")]
    SurfaceNotReady { width: u32, height: u32 },
}

impl ProviderError {
    /// Wrap any `Error` impl into a `ProviderError::Failure`. Used by the
    /// desktop `GlutinProvider` to convert glutin / glutin-winit errors;
    /// `cfg(any(...))`-gated to silence `dead_code` on iOS / Android where
    /// no in-tree caller exists yet (Step 1f mobile providers will use it).
    #[cfg(feature = "gl-host")]
    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    pub(crate) fn from_error<E: Error>(err: E) -> Self {
        Self::Failure(err.to_string())
    }

    /// Construct a `ProviderError::Failure` from a free-form message.
    ///
    /// `pub` (not `pub(crate)`) so out-of-tree provider implementations
    /// — notably the Linux EGL pbuffer test helper in
    /// `tests/common/mod.rs` — can produce typed provider errors with
    /// the same diagnostic shape as `from_error`. Mobile providers
    /// landing in Step 1f will rely on this for `unimplemented!()` →
    /// real-error transitions too. (Codex Phase A Gate round 1
    /// BLOCK 1: previously `pub(crate)` blocked Linux integration
    /// test compilation.)
    pub fn from_msg(msg: impl Into<String>) -> Self {
        Self::Failure(msg.into())
    }
}

/// Result alias used by all provider methods.
pub type ProviderResult<T> = Result<T, ProviderError>;

/// Cross-platform abstraction over a current GL context + presentation
/// surface. Implementations must be safe to call `make_current` /
/// `swap_buffers` repeatedly on the same thread.
///
/// **No `Send` bound** — `PossiblyCurrentContext` and EAGL contexts are
/// thread-bound; `SharedSkiaContext` is intended to live on the main
/// thread alongside winit's `EventLoop` (or the iOS/Android UI thread).
pub trait GlContextProvider {
    /// Make this provider's GL context current on the calling thread.
    fn make_current(&mut self) -> ProviderResult<()>;

    /// Swap the back buffer to the screen.
    fn swap_buffers(&mut self) -> ProviderResult<()>;

    /// A shared `glow::Context` for direct GL state mutation by the
    /// chrome / canvas viewport stub. The `Arc` lets callers cheaply
    /// hand the handle off to long-lived components without reloading
    /// function pointers.
    fn glow(&self) -> Arc<glow::Context>;

    /// Resize the underlying surface (window / pbuffer). Called from
    /// `SharedSkiaContext::resize` before the Skia surface is rebuilt.
    ///
    /// v19.1 mini-patch landed (spec frozen at openpencil-docs
    /// commit 526791f, §3.1): mobile surface-recreate (Step 1f Android
    /// `on_resume`) and desktop window resize both require the GL
    /// surface to be rebuilt before Skia rewraps the FBO.
    fn resize(&mut self, width: u32, height: u32) -> ProviderResult<()>;

    /// The default framebuffer object id of the presentation surface.
    /// `0` = the GL window default FBO; non-zero = an FBO that Skia
    /// must wrap when constructing its `BackendRenderTarget`.
    ///
    /// v19.1 mini-patch landed (spec frozen at openpencil-docs
    /// commit 526791f, §3.1). **Required method, no default body**
    /// (Phase A Gate round 2 CONCERN 1 fix): Step 1f mobile
    /// implementations must explicitly specify — iOS EAGL needs the
    /// non-zero CAEAGLLayer-backed FBO; an inherited default would
    /// silently route Skia's `wrap_backend_render_target` to FBO 0
    /// and produce a black screen at runtime.
    fn default_framebuffer_id(&self) -> u32;

    /// Cleanup hook invoked by `SharedSkiaContext::teardown` before the
    /// provider is dropped. Intentionally explicit: glutin's `Drop` only
    /// unbinds the thread-current context and lets the driver free
    /// asynchronously, so an explicit release path makes the
    /// 100-iteration RSS sanity test deterministic.
    fn release(&mut self) -> ProviderResult<()>;

    /// Resolve a GL symbol through the API that created this context
    /// (EGL / GLX / WGL / CGL). Skia consumes this via
    /// `Interface::new_load_with` so symbol resolution matches the
    /// context's API — `Interface::new_native()` dlopens libGL/GLX on
    /// Linux, which fails when the current context is EGL (the
    /// LINUX_GPU_SKIA_LOADER_TBD gap). `None` = this provider has no
    /// loader; `SharedSkiaContext::new` falls back to `new_native()`.
    fn gl_proc_address(&self, symbol: &std::ffi::CStr) -> Option<*const std::ffi::c_void> {
        let _ = symbol;
        None
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Desktop: GlutinProvider (cfg-gated to macOS / Linux / Windows; the
// glutin / winit / skia-safe dep stack is desktop-only per spec §11
// invariant 1).
// ────────────────────────────────────────────────────────────────────────────

/// Desktop GL provider built on top of `glutin 0.32` + `glutin-winit 0.5`.
///
/// All non-`Send` glutin handles live in [`Option`]s so `release` can
/// drop them in a defined order (surface → context → display) without
/// requiring `&mut self` to consume `self`.
#[cfg(feature = "gl-host")]
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub struct GlutinProvider {
    /// Currently-current context. `None` after `release`.
    context: Option<glutin::context::PossiblyCurrentContext>,
    /// The window surface attached to the GL context.
    surface: Option<glutin::surface::Surface<glutin::surface::WindowSurface>>,
    /// glow handle — shared via `Arc` so the same loaded function table
    /// reaches both Skia (via `gl::Interface::new_load_with`) and the
    /// chrome stub.
    glow: Arc<glow::Context>,
}

#[cfg(feature = "gl-host")]
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
impl GlutinProvider {
    /// Construct from an existing winit window. Builds a glutin display
    /// directly from the window's display handle (bypassing the sealed
    /// `glutin_winit::GlutinEventLoop` trait), picks a GL config, creates
    /// a context + window surface, and loads glow function pointers.
    ///
    /// This routine performs the minimal handshake required by Skia's
    /// GL backend: a current context with a default framebuffer that
    /// matches the window's pixel format and stencil bits.
    #[tracing::instrument(skip_all)]
    pub fn from_window(window: &winit::window::Window) -> ProviderResult<Self> {
        use glutin::config::ConfigTemplateBuilder;
        use glutin::context::ContextAttributesBuilder;
        use glutin::display::{Display, GetGlDisplay};
        use glutin::prelude::*;
        use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
        use std::ffi::CString;

        let raw_window_handle = window
            .window_handle()
            .map_err(ProviderError::from_error)?
            .as_raw();
        let raw_display_handle = window
            .display_handle()
            .map_err(ProviderError::from_error)?
            .as_raw();

        // Per-platform display preference (glutin-winit applies the same
        // matrix internally, but we go direct to avoid the sealed trait).
        let preference = pick_display_api(raw_window_handle);

        let gl_display = unsafe {
            Display::new(raw_display_handle, preference).map_err(ProviderError::from_error)?
        };

        let template = ConfigTemplateBuilder::new()
            .with_alpha_size(8)
            .with_stencil_size(8)
            .compatible_with_native_window(raw_window_handle)
            .build();

        let gl_config = unsafe {
            gl_display
                .find_configs(template)
                .map_err(ProviderError::from_error)?
                .reduce(|best, cfg| {
                    if cfg.num_samples() > best.num_samples() {
                        cfg
                    } else {
                        best
                    }
                })
                .ok_or_else(|| ProviderError::from_msg("no GL config matched the window"))?
        };

        let context_attributes = ContextAttributesBuilder::new().build(Some(raw_window_handle));

        let not_current = unsafe {
            gl_config
                .display()
                .create_context(&gl_config, &context_attributes)
                .map_err(ProviderError::from_error)?
        };

        // GL window-surface attributes, built directly off the
        // window's raw handle + inner size. This is what
        // `glutin_winit::GlWindow::build_surface_attributes` does
        // internally; we inline it so the crate stays off
        // `glutin-winit` (which is pinned to the upstream `winit`
        // package). During macOS startup, winit can briefly expose a
        // placeholder 1×1 drawable; reject that state before entering
        // native GL so the caller can retry after the real resize.
        let inner = window.inner_size();
        let (surface_width, surface_height) = window_surface_size(inner)?;
        let surface_attrs =
            glutin::surface::SurfaceAttributesBuilder::<glutin::surface::WindowSurface>::new()
                .build(raw_window_handle, surface_width, surface_height);
        let surface = unsafe {
            gl_config
                .display()
                .create_window_surface(&gl_config, &surface_attrs)
                .map_err(ProviderError::from_error)?
        };

        let context = not_current
            .make_current(&surface)
            .map_err(ProviderError::from_error)?;

        // Load glow function pointers from the now-current context.
        let display_for_loader = gl_config.display();
        let glow_ctx = unsafe {
            glow::Context::from_loader_function(|sym| {
                let cstr = CString::new(sym).expect("GL symbol UTF-8");
                display_for_loader.get_proc_address(cstr.as_c_str())
            })
        };

        Ok(Self {
            context: Some(context),
            surface: Some(surface),
            glow: Arc::new(glow_ctx),
        })
    }
}

#[cfg(feature = "gl-host")]
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
fn window_surface_size(
    inner: winit::dpi::PhysicalSize<u32>,
) -> ProviderResult<(std::num::NonZeroU32, std::num::NonZeroU32)> {
    if inner.width <= 1 || inner.height <= 1 {
        return Err(ProviderError::SurfaceNotReady {
            width: inner.width,
            height: inner.height,
        });
    }
    let width = std::num::NonZeroU32::new(inner.width).expect("surface width is > 1");
    let height = std::num::NonZeroU32::new(inner.height).expect("surface height is > 1");
    Ok((width, height))
}

#[cfg(feature = "gl-host")]
#[cfg(target_os = "macos")]
fn pick_display_api(
    _raw: raw_window_handle::RawWindowHandle,
) -> glutin::display::DisplayApiPreference {
    glutin::display::DisplayApiPreference::Cgl
}

#[cfg(feature = "gl-host")]
#[cfg(target_os = "windows")]
fn pick_display_api(
    raw: raw_window_handle::RawWindowHandle,
) -> glutin::display::DisplayApiPreference {
    glutin::display::DisplayApiPreference::WglThenEgl(Some(raw))
}

#[cfg(feature = "gl-host")]
#[cfg(target_os = "linux")]
fn pick_display_api(
    _raw: raw_window_handle::RawWindowHandle,
) -> glutin::display::DisplayApiPreference {
    // Prefer EGL on Linux: works under both X11 (via Mesa EGL) and Wayland,
    // and lines up with the EGL-pbuffer headless path used by GPU smoke tests.
    glutin::display::DisplayApiPreference::Egl
}

#[cfg(feature = "gl-host")]
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
impl GlContextProvider for GlutinProvider {
    #[tracing::instrument(skip(self))]
    fn make_current(&mut self) -> ProviderResult<()> {
        use glutin::prelude::*;
        let ctx = self
            .context
            .as_ref()
            .ok_or_else(|| ProviderError::from_msg("provider already released"))?;
        let surface = self
            .surface
            .as_ref()
            .ok_or_else(|| ProviderError::from_msg("provider already released"))?;
        ctx.make_current(surface).map_err(ProviderError::from_error)
    }

    #[tracing::instrument(skip(self))]
    fn swap_buffers(&mut self) -> ProviderResult<()> {
        use glutin::prelude::*;
        let ctx = self
            .context
            .as_ref()
            .ok_or_else(|| ProviderError::from_msg("provider already released"))?;
        let surface = self
            .surface
            .as_ref()
            .ok_or_else(|| ProviderError::from_msg("provider already released"))?;
        surface.swap_buffers(ctx).map_err(ProviderError::from_error)
    }

    fn glow(&self) -> Arc<glow::Context> {
        self.glow.clone()
    }

    /// Desktop GL: window default framebuffer = FBO 0.
    /// Phase A Gate round 2 CONCERN 1 fix — explicit override (trait
    /// no longer provides a default body).
    fn default_framebuffer_id(&self) -> u32 {
        0
    }

    /// Route Skia's symbol lookups through the same glutin display the
    /// glow function table was loaded from, so EGL contexts resolve via
    /// eglGetProcAddress instead of `new_native()`'s GLX path.
    fn gl_proc_address(&self, symbol: &std::ffi::CStr) -> Option<*const std::ffi::c_void> {
        use glutin::display::GetGlDisplay;
        use glutin::prelude::*;
        self.context
            .as_ref()
            .map(|ctx| ctx.display().get_proc_address(symbol))
    }

    #[tracing::instrument(skip(self))]
    fn resize(&mut self, width: u32, height: u32) -> ProviderResult<()> {
        use glutin::prelude::*;
        use std::num::NonZeroU32;
        let ctx = self
            .context
            .as_ref()
            .ok_or_else(|| ProviderError::from_msg("provider already released"))?;
        let surface = self
            .surface
            .as_ref()
            .ok_or_else(|| ProviderError::from_msg("provider already released"))?;
        let w = NonZeroU32::new(width.max(1)).expect("width clamped to >= 1");
        let h = NonZeroU32::new(height.max(1)).expect("height clamped to >= 1");
        surface.resize(ctx, w, h);
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    fn release(&mut self) -> ProviderResult<()> {
        // glutin 0.32 release semantics (spec §3.5): drop in surface →
        // context order; the driver finishes resource cleanup
        // asynchronously. The explicit `Option::take` chain makes
        // `release` idempotent.
        if let Some(s) = self.surface.take() {
            drop(s);
        }
        if let Some(c) = self.context.take() {
            drop(c);
        }
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Mobile: stubs (Step 1f real implementations)
// ────────────────────────────────────────────────────────────────────────────

/// iOS GL provider — placeholder. Step 1f wires CAEAGLLayer or Metal-translated.
#[cfg(target_os = "ios")]
pub struct EaglProvider;

#[cfg(target_os = "ios")]
impl GlContextProvider for EaglProvider {
    fn make_current(&mut self) -> ProviderResult<()> {
        unimplemented!("Step 1f")
    }
    fn swap_buffers(&mut self) -> ProviderResult<()> {
        unimplemented!("Step 1f")
    }
    fn glow(&self) -> Arc<glow::Context> {
        unimplemented!("Step 1f")
    }
    fn resize(&mut self, _width: u32, _height: u32) -> ProviderResult<()> {
        unimplemented!("Step 1f")
    }
    /// CAEAGLLayer-backed FBO is non-zero; Step 1f must thread the
    /// real FBO id (Phase A Gate round 2 CONCERN 1 fix — explicit
    /// override required, no trait default body).
    fn default_framebuffer_id(&self) -> u32 {
        unimplemented!("Step 1f")
    }
    fn release(&mut self) -> ProviderResult<()> {
        unimplemented!("Step 1f")
    }
}

/// Android GL provider — placeholder. Step 1f wires ANativeWindow + EGL.
#[cfg(target_os = "android")]
pub struct AndroidEglProvider;

#[cfg(target_os = "android")]
impl GlContextProvider for AndroidEglProvider {
    fn make_current(&mut self) -> ProviderResult<()> {
        unimplemented!("Step 1f")
    }
    fn swap_buffers(&mut self) -> ProviderResult<()> {
        unimplemented!("Step 1f")
    }
    fn glow(&self) -> Arc<glow::Context> {
        unimplemented!("Step 1f")
    }
    fn resize(&mut self, _width: u32, _height: u32) -> ProviderResult<()> {
        unimplemented!("Step 1f")
    }
    /// EGL window default FBO = 0 (per spec §3.1 line 136).
    /// Phase A Gate round 2 CONCERN 1 fix — explicit override required,
    /// no trait default body.
    fn default_framebuffer_id(&self) -> u32 {
        0
    }
    fn release(&mut self) -> ProviderResult<()> {
        unimplemented!("Step 1f")
    }
}

#[cfg(all(test, feature = "gl-host"))]
mod tests {
    use super::*;

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn window_surface_size_rejects_tiny_startup_extents() {
        let err = window_surface_size(winit::dpi::PhysicalSize::new(1, 1)).unwrap_err();

        assert!(matches!(
            err,
            ProviderError::SurfaceNotReady {
                width: 1,
                height: 1
            }
        ));
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn window_surface_size_keeps_usable_extents() {
        let (width, height) =
            window_surface_size(winit::dpi::PhysicalSize::new(1440, 900)).unwrap();

        assert_eq!(width.get(), 1440);
        assert_eq!(height.get(), 900);
    }
}
