//! Preview contract DTOs — the dependency-light types shared across the
//! preview stack's module boundaries (R3/R4 contracts).
//!
//! This crate is a LEAF: it depends only on `serde` / `serde_json` /
//! `thiserror` and contains no runtime, layout, or UI code. It exists so
//! `op-preview-core`, `op-editor-ui`, the FFI layers, and the host
//! adapters can share one frozen contract without creating the existing
//! Core → UI dependency cycle.
//!
//! Capabilities and activation ids (R4) live here now; effect DTOs and
//! the platform-support authoring table (R3), plus debug/trace DTOs
//! (R9), extend this crate in their own tasks.

mod activation;
mod capability;

pub use activation::UserActivationId;
pub use capability::{PreviewCapability, PreviewHostCapabilities};
