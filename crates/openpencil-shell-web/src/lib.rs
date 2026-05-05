//! OpenPencil shell — web (wasm32) bundle entry.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate is the web bundle entry.
//! CI invariant requires `cargo check --target wasm32-unknown-unknown -p
//! openpencil-shell-web --no-default-features --features web` to pass on
//! every PR.
//!
//! Step 1a Task 1 status: only link-checks the shell-core re-export; the
//! CanvasKit WebBackend lands in Step 1b.

use wasm_bindgen::prelude::*;

/// Skeleton placeholder until Step 1b lands `WebCanvasKitBackend`.
///
/// Uses the OP `Color::TRANSPARENT` named constant to prove the shell-core
/// re-export links on wasm32 — this is the minimal link-check (shell-core
/// must stay wasm32-clean per spec §1.2).
#[wasm_bindgen]
pub fn placeholder() -> String {
    let _t = openpencil_shell_core::Color::TRANSPARENT;
    "openpencil-shell-web skeleton (Task 1: deps wired)".to_string()
}
