//! OpenPencil shell — web (wasm32) bundle entry.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate is the web bundle entry.
//! CI invariant requires `cargo check --target wasm32-unknown-unknown -p
//! openpencil-shell-web --no-default-features --features web` to pass on
//! every PR.
//!
//! Step 1a Task 1 状态：仅 link-check shell-core re-export；CanvasKit
//! WebBackend 在 Step 1b 加。

use wasm_bindgen::prelude::*;

/// Skeleton placeholder until Step 1b lands `WebCanvasKitBackend`.
///
/// 用 OP `Color::TRANSPARENT` 命名常量证明 shell-core re-export 在 wasm32
/// 链通了；这是最小 link-check（shell-core 必须 wasm32-clean per spec §1.2）。
#[wasm_bindgen]
pub fn placeholder() -> String {
    let _t = openpencil_shell_core::Color::TRANSPARENT;
    "openpencil-shell-web skeleton (Task 1: deps wired)".to_string()
}
