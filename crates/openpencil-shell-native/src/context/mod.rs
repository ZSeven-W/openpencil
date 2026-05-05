//! Shared GL + Skia context module (spec v19 §3).
//!
//! - [`provider`] — `GlContextProvider` trait + `GlutinProvider` (desktop)
//!   + iOS / Android stubs (Step 1f).
//! - [`shared`] — `SharedSkiaContext` owning the GL stack +
//!   `skia_safe::DirectContext` + `skia_safe::Surface`. Frame-scoped
//!   `with_frame` callback + idempotent teardown.

pub mod provider;
pub mod shared;

pub use provider::{GlContextProvider, GlutinProvider, ProviderError, ProviderResult};
pub use shared::{SharedSkiaContext, SharedSkiaError, SharedSkiaResult, SurfaceConfig};
