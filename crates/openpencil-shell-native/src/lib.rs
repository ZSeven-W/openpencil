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

// `placeholder()` from Task 1 was removed by Codex Phase A Gate round 1
// NIT 1 — Task 2's full re-export chain (`SharedSkiaContext`,
// `NativeBackend`, etc.) already proves the shell-core ↔ shell-native
// link, and Task 3's `AppShell::run_desktop` will be the canonical
// entry. Keeping the dead helper just to satisfy a removed link-check
// is YAGNI.
