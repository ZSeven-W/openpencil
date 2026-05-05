//! OpenPencil shell — native (desktop) backend.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate must NOT be linked into
//! the wasm32-unknown-unknown web bundle. Even though some deps (winit) compile
//! silently on wasm32 via web-sys, we use an explicit compile_error! guard to
//! make accidental inclusion a hard error.
//!
//! Step 1a Task 2: this crate now ships [`SharedSkiaContext`] (own GL stack +
//! Skia DirectContext + Surface, idempotent teardown, lifecycle hooks),
//! [`CanvasViewportStub`] (GL-state-isolation probe), and [`NativeBackend`]
//! (frame-scoped widget facade that translates OP `RenderBackend` calls into
//! Jian `DrawOp`s and submits via `jian_skia::SkiaBackend::draw_on_canvas`).
//!
//! Module layout (spec v19 §2 / §3 / §5.2.1):
//! - [`context`] — `GlContextProvider` trait + `GlutinProvider` desktop impl
//!   + iOS / Android stubs; `SharedSkiaContext` owning the GL stack.
//! - [`canvas_view_stub`] — `CanvasViewportStub::render_into` (deliberately
//!   pollutes GL state to verify chrome paint isolation).
//! - [`backend`] — `NativeBackend` exposing frame-scoped methods mirroring the
//!   OP `RenderBackend` trait surface (no direct trait impl in Step 1a; see
//!   spec §5.2.1).

#[cfg(target_arch = "wasm32")]
compile_error!(
    "openpencil-shell-native must NOT be compiled for wasm32 targets. \
     Use openpencil-shell-web for browser builds (spec v19 §1.2)."
);

pub mod backend;
pub mod canvas_view_stub;
pub mod context;

pub use backend::{to_jian_color, to_jian_rect, NativeBackend};
pub use canvas_view_stub::CanvasViewportStub;
pub use context::{
    GlContextProvider, GlutinProvider, ProviderError, ProviderResult, SharedSkiaContext,
    SharedSkiaError, SharedSkiaResult, SurfaceConfig,
};

/// Skeleton placeholder retained from Task 1 — proves shell-core re-export
/// chain still links after Task 2 lands. Removed in Task 3 once
/// `AppShell::run_desktop` becomes the documented entry.
#[doc(hidden)]
pub fn placeholder() -> &'static str {
    let _red = openpencil_shell_core::Color::RED;
    "openpencil-shell-native skeleton (Task 1+2 wired)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_links_core() {
        assert!(placeholder().contains("Task"));
    }
}
