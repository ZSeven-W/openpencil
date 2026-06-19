//! Shared GL + Skia context module (spec v19 §3).
//!
//! - [`provider`] — `GlContextProvider` trait (cross-platform) +
//!   `GlutinProvider` (desktop) + iOS / Android stubs (Step 1f). The
//!   trait is exposed on every non-wasm target per spec §11 invariant 2.
//! - [`shared`] — `SharedSkiaContext` owning the GL stack +
//!   `skia_safe::DirectContext` + `skia_safe::Surface`. Frame-scoped
//!   `with_frame` callback + idempotent teardown. Desktop-only — the
//!   skia-safe / jian-skia deps that back it aren't fetched on
//!   iOS / Android (Cargo.toml target-gates them).

pub mod provider;

// Cross-platform: trait + error types, importable on every non-wasm target.
pub use provider::{GlContextProvider, ProviderError, ProviderResult};

// Per-platform provider implementations.
#[cfg(target_os = "android")]
pub use provider::AndroidEglProvider;
#[cfg(target_os = "ios")]
pub use provider::EaglProvider;
#[cfg(feature = "gl-host")]
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use provider::GlutinProvider;

// `SharedSkiaContext` is desktop-only — depends on `skia_safe` + `winit`
// (cfg-gated dependencies). Step 1f mobile wiring will introduce a mobile
// twin (or generalize this one) once iOS / Android providers go from
// `unimplemented!()` placeholders to real GL contexts.
#[cfg(feature = "gl-host")]
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod shared;
#[cfg(feature = "gl-host")]
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
pub use shared::{SharedSkiaContext, SharedSkiaError, SharedSkiaResult, SurfaceConfig};
