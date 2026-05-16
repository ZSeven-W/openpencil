//! Canonical `.op` (`PenDocument`) → shell `Document` loader / adapter.
//!
//! Extracted from `openpencil-desktop` so library crates (notably
//! `openpencil-shell-native`) can reuse the conversion without
//! depending on a binary crate.
//!
//! Two layers:
//!
//! - [`adapter`] bridges `jian_ops_schema::PenDocument` → the private
//!   [`payload::DocPayload`], running `jian-core::LayoutEngine` +
//!   `jian_skia::SkiaMeasure` to bake flex layout into AABB rects.
//! - [`payload`] holds the serde DTOs + `apply_payload` (payload →
//!   `Document` builder) + [`pen_document_to_document`] — the clean
//!   public entry point.
//!
//! This crate is desktop-side and may depend on skia; it does NOT
//! need to be wasm-clean. The `rfd` Save/Open dialogs + error
//! dialogs stay in `openpencil-desktop/src/persistence.rs`.

mod adapter;
mod effects;
mod path_bounds;

pub mod payload;
pub mod variables;

/// Canonical entry point — convert a parsed `PenDocument` straight
/// into a shell-core `Document`.
pub use payload::pen_document_to_document;

// Re-exports so `openpencil-desktop`'s existing call sites change
// minimally.
pub use adapter::{build_var_table, pen_document_to_payload, LoadedDoc};
pub use effects::{effects_from_payload, effects_to_payload, shadows_from_canonical, ShadowPayload};
pub use payload::{
    apply_payload, load_canonical, to_payload, DocPayload, NodePayload, PagePayload, StrokePayload,
};
pub use variables::{var_table_from_payload, var_table_to_payload, VarTablePayload};
