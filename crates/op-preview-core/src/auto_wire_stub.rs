//! Mobile stub for the orchestrator-backed preview auto-wire pass.
//!
//! `auto_wire_for_preview` runs the op-orchestrator's Track A screen/nav
//! wiring over a preview-only document clone. The mobile player builds do
//! not link the orchestrator (tokio/QuickJS stack); without the pass a
//! hand-drawn multi-screen document enters App Mode without the automatic
//! screen/nav wiring — authored `screen` markers still work.

#![cfg(not(feature = "gl-host"))]

/// Returns `None` — the auto-wire pass is desktop-only; documents with
/// authored `screen` markers are unaffected.
#[allow(dead_code)]
pub(crate) fn auto_wire_for_preview(
    _doc: &jian_ops_schema::PenDocument,
    _active_page_index: usize,
) -> Option<jian_ops_schema::PenDocument> {
    None
}
