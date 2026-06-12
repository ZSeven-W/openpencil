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
    "op-host-native must NOT be compiled for wasm32 targets. \
     Use op-host-web for browser builds (spec v19 §1.2)."
);

// Cross-platform context module: re-exports the `GlContextProvider` trait
// + `ProviderError` / `ProviderResult` on every (non-wasm) target so spec
// §11 invariant 2 holds — mobile callers can name the trait. Internal
// cfg-gates select between `GlutinProvider` (desktop), `EaglProvider` (iOS)
// and `AndroidEglProvider` (Android), and `SharedSkiaContext` is only
// compiled in on desktop where the GL + Skia stack is available.
pub mod context;

// Desktop-only modules — pull `skia_safe` / `jian_skia` / `glutin` types
// that aren't fetched on iOS / Android (see Cargo.toml target-gated deps).
// Spec §11 invariants 1 & 3: mobile builds compile shell-native without
// these modules at all; mobile widget rendering lands in Step 1f.
// Cross-platform render stack: `backend` (NativeBackend) and
// `widget_host` (NativeFrameBackend + WidgetHostNative) compile on
// every target where `skia-safe` + `jian-skia` compile — desktop
// trio + iOS + Android. Per the 2026-05-10 user directive
// (jian will eventually need iOS and Android) these used to be
// desktop-only; lifting the gate is the first half of the mobile
// extension. The other half (real `EaglProvider` / `AndroidEglProvider`
// impls + Metal/Vulkan integration) lands in Step 1f.
//
// `canvas_view_stub` stays desktop-only — it pulls `glow` for a
// GL-state-isolation probe that has no mobile equivalent.
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android"
))]
pub mod backend;
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android"
))]
pub mod boolean_ops;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub mod canvas_view_stub;
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android"
))]
pub mod widget_host;

#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android"
))]
pub use backend::{to_jian_rect, NativeBackend};
#[cfg(any(
    target_os = "macos",
    target_os = "linux",
    target_os = "windows",
    target_os = "ios",
    target_os = "android"
))]
pub use widget_host::{CursorHint, NativeFrameBackend, WidgetHostNative};
// canvas_view_stub stays desktop-only (uses glow GL-isolation probe).
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use canvas_view_stub::CanvasViewportStub;

// Cross-platform re-exports — visible on every (non-wasm) target.
pub use context::{GlContextProvider, ProviderError, ProviderResult};

// Desktop-only re-exports.
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use context::{
    GlutinProvider, SharedSkiaContext, SharedSkiaError, SharedSkiaResult, SurfaceConfig,
};

// Mobile stub re-exports — Step 1f real impls; today they're zero-sized
// placeholder structs whose `GlContextProvider` impls `unimplemented!()`.
#[cfg(target_os = "android")]
pub use context::AndroidEglProvider;
#[cfg(target_os = "ios")]
pub use context::EaglProvider;

// `placeholder()` from Task 1 was removed by Codex Phase A Gate round 1
// NIT 1 — Task 2's full re-export chain (`SharedSkiaContext`,
// `NativeBackend`, etc.) already proves the shell-core ↔ shell-native
// link, and Task 3's `AppShell::run_desktop` will be the canonical
// entry. Keeping the dead helper just to satisfy a removed link-check
// is YAGNI.
