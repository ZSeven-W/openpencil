//! Shared test helpers for `openpencil-shell-native`.
//!
//! Spec v19 §9 / plan v7 Task 2 Step 16a-16f: this module supplies
//! - `setup_headless_context()` for tests that don't actually need a
//!   live GPU surface (teardown idempotence, tracing, RSS sanity);
//! - `egl_pbuffer::EglPbufferProvider` (Linux only) for the GPU-smoke
//!   tests, providing an off-screen GL context via Mesa softpipe.

#![allow(dead_code)]

use openpencil_shell_native::SharedSkiaContext;

/// Returns a `SharedSkiaContext` whose every `Option<>` field is
/// `None`. The tests that exercise idempotence + tracing don't care
/// what's behind the handles — they care that the public API behaves
/// the same on a torn-down context. Avoids the per-test cost of
/// spinning up an EGL pbuffer / winit window.
pub fn setup_headless_context() -> SharedSkiaContext {
    SharedSkiaContext::inert_for_test()
}

#[cfg(target_os = "linux")]
pub mod egl_pbuffer {
    //! `EglPbufferProvider` — a `GlContextProvider` backed by a Mesa
    //! softpipe EGL pbuffer. No display server / X11 / Wayland needed,
    //! so the test runs on hosted Linux runners (GitHub Actions
    //! `ubuntu-latest` ships Mesa). Spec §9.2 + plan v7 Task 2 Step
    //! 16f Linux path.
    //!
    //! The provider is tiny on purpose — it only does what the GPU
    //! smoke + chrome-stub composition tests need, and it intentionally
    //! does not try to unify with the desktop `GlutinProvider` API
    //! beyond the `GlContextProvider` trait.
    use std::ffi::c_void;
    use std::sync::Arc;

    use khronos_egl as egl;
    use openpencil_shell_native::{GlContextProvider, ProviderError, ProviderResult};

    type Egl = egl::DynamicInstance<egl::EGL1_4>;

    pub struct EglPbufferProvider {
        egl: Arc<Egl>,
        display: egl::Display,
        context: egl::Context,
        surface: egl::Surface,
        glow: Arc<glow::Context>,
        size: (u32, u32),
        released: bool,
    }

    impl EglPbufferProvider {
        pub fn new(size: (u32, u32)) -> ProviderResult<Self> {
            let egl: Arc<Egl> = unsafe {
                Arc::new(
                    egl::DynamicInstance::<egl::EGL1_4>::load_required()
                        .map_err(|e| ProviderError::from_msg(format!("libEGL load: {e}")))?,
                )
            };

            let display = egl
                .get_display(egl::DEFAULT_DISPLAY)
                .ok_or_else(|| ProviderError::from_msg("no default EGL display"))?;
            egl.initialize(display)
                .map_err(|e| ProviderError::from_msg(format!("EGL init: {e}")))?;
            egl.bind_api(egl::OPENGL_API)
                .map_err(|e| ProviderError::from_msg(format!("EGL bind_api: {e}")))?;

            let attribs = [
                egl::SURFACE_TYPE,
                egl::PBUFFER_BIT,
                egl::RENDERABLE_TYPE,
                egl::OPENGL_BIT,
                egl::RED_SIZE,
                8,
                egl::GREEN_SIZE,
                8,
                egl::BLUE_SIZE,
                8,
                egl::ALPHA_SIZE,
                8,
                egl::DEPTH_SIZE,
                0,
                egl::STENCIL_SIZE,
                8,
                egl::NONE,
            ];
            let config = egl
                .choose_first_config(display, &attribs)
                .map_err(|e| ProviderError::from_msg(format!("choose_config: {e}")))?
                .ok_or_else(|| ProviderError::from_msg("no matching EGL config"))?;

            let context_attribs = [egl::NONE];
            let context = egl
                .create_context(display, config, None, &context_attribs)
                .map_err(|e| ProviderError::from_msg(format!("create_context: {e}")))?;

            let pbuffer_attribs = [
                egl::WIDTH,
                size.0 as i32,
                egl::HEIGHT,
                size.1 as i32,
                egl::NONE,
            ];
            let surface = egl
                .create_pbuffer_surface(display, config, &pbuffer_attribs)
                .map_err(|e| ProviderError::from_msg(format!("create_pbuffer: {e}")))?;

            egl.make_current(display, Some(surface), Some(surface), Some(context))
                .map_err(|e| ProviderError::from_msg(format!("make_current: {e}")))?;

            let egl_for_loader = Arc::clone(&egl);
            let glow_ctx = unsafe {
                glow::Context::from_loader_function(move |sym| {
                    egl_for_loader
                        .get_proc_address(sym)
                        .map(|p| p as *const c_void)
                        .unwrap_or(std::ptr::null())
                })
            };

            Ok(Self {
                egl,
                display,
                context,
                surface,
                glow: Arc::new(glow_ctx),
                size,
                released: false,
            })
        }
    }

    impl GlContextProvider for EglPbufferProvider {
        fn make_current(&mut self) -> ProviderResult<()> {
            self.egl
                .make_current(
                    self.display,
                    Some(self.surface),
                    Some(self.surface),
                    Some(self.context),
                )
                .map_err(|e| ProviderError::from_msg(format!("make_current: {e}")))
        }

        fn swap_buffers(&mut self) -> ProviderResult<()> {
            // Pbuffers don't have a real "swap" — the EGL spec defines
            // it as a no-op. Treat the call as success so the present
            // path still runs to completion.
            self.egl
                .swap_buffers(self.display, self.surface)
                .map_err(|e| ProviderError::from_msg(format!("swap_buffers: {e}")))
        }

        fn glow(&self) -> Arc<glow::Context> {
            self.glow.clone()
        }

        fn resize(&mut self, _w: u32, _h: u32) -> ProviderResult<()> {
            // Pbuffer can't be resized once created — tests build the
            // surface at the size they need.
            Ok(())
        }

        fn size(&self) -> (u32, u32) {
            self.size
        }

        fn release(&mut self) -> ProviderResult<()> {
            if self.released {
                return Ok(());
            }
            // Make-not-current first; some Mesa builds segfault if the
            // pbuffer surface drops while still bound.
            let _ = self.egl.make_current(self.display, None, None, None);
            let _ = self.egl.destroy_surface(self.display, self.surface);
            let _ = self.egl.destroy_context(self.display, self.context);
            // Keep the EGL display open for other tests in the same
            // process; eglTerminate is process-global on Mesa and a
            // shared display is normal.
            self.released = true;
            Ok(())
        }
    }

    impl Drop for EglPbufferProvider {
        fn drop(&mut self) {
            let _ = self.release();
        }
    }
}
