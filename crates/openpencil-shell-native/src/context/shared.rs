//! `SharedSkiaContext` — owns the GL stack + Skia DirectContext + Surface.
//!
//! Per spec v19 §3.2-§3.5 the context follows a strict `Option<>` field
//! pattern so `teardown` is idempotent (repeated calls collapse to no-op
//! via `Option::take`). Lifecycle hooks (`on_pause` / `on_resume` /
//! `on_low_memory`) match Android + iOS contracts; on desktop they are
//! no-ops but stay in the public API so Step 1f mobile entries land
//! without breaking changes.

use std::sync::Arc;

use crate::context::provider::{GlContextProvider, GlutinProvider, ProviderError};

/// Errors raised by [`SharedSkiaContext`].
#[derive(Debug, thiserror::Error)]
pub enum SharedSkiaError {
    /// Wrapped GL provider error (glutin / EGL / EAGL).
    #[error(transparent)]
    Provider(#[from] ProviderError),
    /// Skia could not load a GL `Interface` (driver did not expose
    /// `glGetIntegerv` etc).
    #[error("skia gl interface unavailable")]
    GlInterface,
    /// Skia could not build a `DirectContext` from the loaded interface.
    #[error("skia direct context construction failed")]
    DirectContext,
    /// Skia could not wrap the GL framebuffer into a render target /
    /// surface.
    #[error("skia surface construction failed")]
    Surface,
}

/// Result alias for fallible context operations.
pub type SharedSkiaResult<T> = Result<T, SharedSkiaError>;

/// Configuration for surface creation. `dpi` is logical→physical scale
/// (passed to `NativeBackend::dpi_scale`); `(width, height)` is the
/// initial physical-pixel size of the framebuffer.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceConfig {
    pub width: u32,
    pub height: u32,
    pub sample_count: usize,
    pub stencil_bits: usize,
    pub dpi: f32,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            sample_count: 0,
            stencil_bits: 8,
            dpi: 1.0,
        }
    }
}

/// Shared Skia + GL context (spec v19 §3.2).
///
/// All native handles live behind `Option<>` so `teardown` collapses
/// cleanly on repeated calls. The fields are private; consumers go
/// through `with_frame` / `present` / `resize` / `teardown` /
/// `lifecycle hooks`.
pub struct SharedSkiaContext {
    provider: Option<Box<dyn GlContextProvider>>,
    direct_context: Option<skia_safe::gpu::DirectContext>,
    surface: Option<skia_safe::Surface>,
    /// glow handle. **Spec mini-patch pending** (Codex Phase A Gate
    /// round 1 CONCERN 2): spec v19 lines 120-125 + 191 declare a
    /// non-Optional `Arc<glow::Context>`, but real lifecycle requires
    /// it to be droppable:
    ///
    ///   - `teardown` must release the handle so the loaded function
    ///     table can be GC'd (deterministic cleanup for the
    ///     100-iteration RSS test);
    ///   - `inert_for_lifecycle_test()` produces a context with no
    ///     GL backing whatsoever — exposing a phantom `Arc` would
    ///     just hide UB behind a `unimplemented!()` loader closure;
    ///   - Step 1f Android `on_pause` will need to drop the glow
    ///     handle alongside the surface (the EGL context behind it
    ///     becomes invalid the moment the activity backgrounds).
    ///
    /// Report back to controller to push v19 → v19.1 making
    /// `glow_handle` optional in §3.2 / §3.3.
    glow_handle: Option<Arc<glow::Context>>,
    config: SurfaceConfig,
}

impl SharedSkiaContext {
    /// Generic constructor accepting any `GlContextProvider`. Used by
    /// headless tests (EGL pbuffer provider) and mobile providers in
    /// Step 1f.
    #[tracing::instrument(skip_all)]
    pub fn new<P: GlContextProvider + 'static>(
        mut provider: P,
        config: SurfaceConfig,
    ) -> SharedSkiaResult<Self> {
        provider.make_current()?;

        let glow_handle = provider.glow();

        let interface =
            skia_safe::gpu::gl::Interface::new_native().ok_or(SharedSkiaError::GlInterface)?;
        let mut direct_context = skia_safe::gpu::direct_contexts::make_gl(interface, None)
            .ok_or(SharedSkiaError::DirectContext)?;

        let fboid = provider.default_framebuffer_id();
        let surface = build_gl_surface(&mut direct_context, &config, fboid)?;

        Ok(Self {
            provider: Some(Box::new(provider)),
            direct_context: Some(direct_context),
            surface: Some(surface),
            glow_handle: Some(glow_handle),
            config,
        })
    }

    /// Desktop convenience constructor — wraps `GlutinProvider` and
    /// derives the initial surface size from the window.
    #[tracing::instrument(skip_all)]
    pub fn new_desktop(window: &winit::window::Window) -> SharedSkiaResult<Self> {
        let provider = GlutinProvider::from_window(window)?;
        let inner = window.inner_size();
        let scale = window.scale_factor() as f32;
        let config = SurfaceConfig {
            width: inner.width.max(1),
            height: inner.height.max(1),
            dpi: if scale > 0.0 { scale } else { 1.0 },
            ..SurfaceConfig::default()
        };
        Self::new(provider, config)
    }

    /// Begin a frame. Pure marker hook for now — chrome / canvas viewport
    /// drawing happens inside [`with_frame`]. The tracing span lets
    /// observers see the per-frame boundary.
    #[tracing::instrument(skip(self))]
    pub fn begin_frame(&mut self) {
        // Emit an event so subscriber-based tests (`tracing-test`) can see
        // the call land — `#[instrument]` alone produces a span but no
        // log line until the body emits an event.
        tracing::trace!("begin_frame");
    }

    /// Borrow the Skia `Canvas` + `glow::Context` for the duration of
    /// the closure (spec v19 §3.3). Skia 0.97's canvas methods are all
    /// `&self`, so the closure can issue any draw calls + receive a
    /// shared `&Canvas` (matching `NativeBackend::fill_rect` etc).
    #[tracing::instrument(skip_all)]
    pub fn with_frame<F>(&mut self, f: F)
    where
        F: FnOnce(&skia_safe::Canvas, &glow::Context),
    {
        tracing::trace!("with_frame");
        if let (Some(surface), Some(glow)) = (self.surface.as_mut(), self.glow_handle.as_ref()) {
            let canvas = surface.canvas();
            f(canvas, glow.as_ref());
        }
    }

    /// Flush the recorded GPU commands and swap the back buffer. No-op
    /// when the context has been torn down.
    #[tracing::instrument(skip(self))]
    pub fn present(&mut self) {
        tracing::trace!("present");
        if let Some(dc) = self.direct_context.as_mut() {
            dc.flush_and_submit();
        }
        if let Some(p) = self.provider.as_mut() {
            // Best-effort swap; failure is logged but not propagated so
            // a single transient error doesn't kill the event loop.
            if let Err(err) = p.swap_buffers() {
                tracing::warn!(?err, "swap_buffers failed");
            }
        }
    }

    /// Resize the GL surface + rebuild the Skia surface to match.
    #[tracing::instrument(skip(self))]
    pub fn resize(&mut self, width: u32, height: u32) -> SharedSkiaResult<()> {
        tracing::trace!("resize");
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.config.width = width;
        self.config.height = height;

        if let Some(p) = self.provider.as_mut() {
            p.resize(width, height)?;
        }

        // Rebuild the Skia surface with the new framebuffer dimensions;
        // the `DirectContext` is reused (no GPU resource churn beyond
        // the wrapped FBO).
        let fboid = self
            .provider
            .as_ref()
            .map(|p| p.default_framebuffer_id())
            .unwrap_or(0);
        if let Some(dc) = self.direct_context.as_mut() {
            // Drop old surface before building the new one to release
            // its FBO wrapper.
            self.surface.take();
            let new_surface = build_gl_surface(dc, &self.config, fboid)?;
            self.surface = Some(new_surface);
        }
        Ok(())
    }

    /// Idempotent teardown (spec v19 §3.3 + §3.5).
    ///
    /// Order: surface → DirectContext → provider. Repeated calls hit
    /// the all-`None` path and are no-ops.
    #[tracing::instrument(skip(self))]
    pub fn teardown(&mut self) -> SharedSkiaResult<()> {
        tracing::trace!("teardown");
        if let Some(s) = self.surface.take() {
            drop(s);
        }
        if let Some(mut ctx) = self.direct_context.take() {
            ctx.abandon();
        }
        if let Some(mut p) = self.provider.take() {
            p.release()?;
            drop(p);
        }
        // Drop the glow handle last; it shares the loader function table
        // with the chrome stub but holds no GL state of its own.
        self.glow_handle.take();
        Ok(())
    }

    /// Optional accessor for the current glow handle. Mainly used by
    /// tests + headless GPU smoke (`with_frame` is the preferred path).
    pub fn glow(&self) -> Option<Arc<glow::Context>> {
        self.glow_handle.clone()
    }

    // ── Lifecycle hooks ────────────────────────────────────────────────

    /// Background entry. On Android / iOS the surface must be dropped
    /// before this returns; the system reclaims the underlying native
    /// window. Desktop is a no-op. Idempotent.
    #[tracing::instrument(skip(self))]
    pub fn on_pause(&mut self) -> SharedSkiaResult<()> {
        tracing::trace!("on_pause");
        if cfg!(any(target_os = "android", target_os = "ios")) {
            if let Some(s) = self.surface.take() {
                drop(s);
            }
        }
        Ok(())
    }

    /// Foreground re-entry. On Android / iOS the caller passes a fresh
    /// `HasWindowHandle` so the provider can attach the new native
    /// window; Step 1f wires the actual rebuild. Desktop is a no-op.
    /// Idempotent.
    #[tracing::instrument(skip(self, _new_window))]
    pub fn on_resume(
        &mut self,
        _new_window: Option<&dyn raw_window_handle::HasWindowHandle>,
    ) -> SharedSkiaResult<()> {
        tracing::trace!("on_resume");
        // Step 1f mobile implementation: provider.attach_window(handle)
        // + rebuild_surface. Desktop has nothing to restore.
        Ok(())
    }

    /// System-memory-pressure hook. Drops Skia's GPU resource cache to
    /// release texture memory; idempotent and safe across teardown.
    #[tracing::instrument(skip(self))]
    pub fn on_low_memory(&mut self) {
        tracing::trace!("on_low_memory");
        if let Some(dc) = self.direct_context.as_mut() {
            dc.free_gpu_resources();
        }
    }

    /// Test-only constructor producing an inert context — every
    /// `Option<>` field is `None`, so all methods take the "already
    /// torn down" branch. Used by `tests/teardown_idempotent.rs` and
    /// `tests/tracing_spans.rs` to pin lifecycle invariants on the
    /// post-teardown shape (the hardest path to get wrong).
    ///
    /// **Renamed from `inert_for_test` (Codex Phase A Gate round 1
    /// BLOCK 3)**: the old name implied "OK for any test", which let
    /// the 100-iteration RSS sanity test (`tests/memory_loop.rs`)
    /// pass against an all-`None` context — i.e. measure nothing.
    /// The new name makes it explicit that this constructor is for
    /// lifecycle idempotence checks only; tests that need to verify
    /// resource accounting must build a real context (raster path on
    /// macOS / Windows, EGL pbuffer on Linux).
    ///
    /// Behind `#[doc(hidden)]` because production callers must go
    /// through `new` / `new_desktop` (those are what allocate the
    /// driver-side handles whose lifecycle we care about).
    #[doc(hidden)]
    pub fn inert_for_lifecycle_test() -> Self {
        Self {
            provider: None,
            direct_context: None,
            surface: None,
            glow_handle: None,
            config: SurfaceConfig::default(),
        }
    }
}

impl Drop for SharedSkiaContext {
    fn drop(&mut self) {
        if let Err(err) = self.teardown() {
            tracing::error!(?err, "SharedSkiaContext drop fallback teardown failed");
        }
    }
}

/// Build a GL-backed Skia surface bound to the provider's default
/// framebuffer. Pulled out of `new` / `resize` to share the
/// (BackendRenderTarget → wrap_backend_render_target) chain.
fn build_gl_surface(
    direct_context: &mut skia_safe::gpu::DirectContext,
    config: &SurfaceConfig,
    fboid: u32,
) -> SharedSkiaResult<skia_safe::Surface> {
    use skia_safe::gpu::{backend_render_targets, gl, SurfaceOrigin};
    use skia_safe::{ColorType, Surface};

    let fb_info = gl::FramebufferInfo {
        fboid,
        // GL_RGBA8 (0x8058) — Skia's default color format for window
        // framebuffers; matches the alpha-bits / stencil request.
        format: 0x8058,
        ..Default::default()
    };

    let backend_rt = backend_render_targets::make_gl(
        (config.width as i32, config.height as i32),
        config.sample_count,
        config.stencil_bits,
        fb_info,
    );

    let surface = skia_safe::gpu::surfaces::wrap_backend_render_target(
        direct_context,
        &backend_rt,
        SurfaceOrigin::BottomLeft,
        ColorType::RGBA8888,
        None,
        None,
    )
    .ok_or(SharedSkiaError::Surface)?;

    Ok::<Surface, SharedSkiaError>(surface)
}
