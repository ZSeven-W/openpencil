//! Preview contract DTOs — the dependency-light types shared across the
//! preview stack's module boundaries (R3/R4 contracts).
//!
//! This crate is a LEAF: it depends only on `serde` / `serde_json` /
//! `thiserror` and contains no runtime, layout, or UI code. It exists so
//! `op-preview-core`, `op-editor-ui`, the FFI layers, and the host
//! adapters can share one frozen contract without creating the existing
//! Core → UI dependency cycle.
//!
//! Capabilities, activation ids, effect DTOs, and the platform-support
//! authoring table (R3/R4) live here now; debug/trace DTOs (R9) extend
//! this crate in their own task.

mod activation;
mod capability;
mod debug;
mod effect;
mod platform_support;
#[cfg(test)]
mod tests;

pub use activation::UserActivationId;
pub use capability::{PreviewCapability, PreviewHostCapabilities};
pub use debug::{
    PreviewDebugSnapshot, PreviewDiagnostic, PreviewQueueCounts, PreviewRunState,
    PreviewStateProvenance, PreviewStateRow, PreviewStateScope, PreviewTraceEntry,
    PreviewTraceKind,
};
pub use effect::{
    EffectSource, HapticStyle, PreviewEffect, PreviewEffectFailure, PreviewEffectFailureCode,
    PreviewEffectResult, SharePayload,
};
pub use platform_support::{platform_support, HostSupport, PreviewInteraction, PreviewPlatform};
