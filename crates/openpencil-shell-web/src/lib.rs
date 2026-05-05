//! OpenPencil shell — web (wasm32) bundle entry.
//!
//! Per kickoff spec §1.2: this crate is the web bundle entry. CI invariant
//! requires `cargo check --target wasm32-unknown-unknown -p openpencil-shell-web
//! --no-default-features --features web` to pass on every PR.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn placeholder() -> String {
    format!(
        "openpencil-shell-web skeleton ({})",
        openpencil_shell_core::placeholder()
    )
}
