//! OpenPencil shell core — platform-agnostic widget tree + render-backend trait.
//!
//! Per kickoff spec §1.2 (FROZEN 2026-05-02): this crate must compile on
//! wasm32-unknown-unknown. winit / accesskit_winit / skia-safe live in
//! `openpencil-shell-native`; wasm-bindgen / web-sys / CanvasKit live in
//! `openpencil-shell-web`.

pub fn placeholder() -> &'static str {
    "openpencil-shell-core skeleton"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_returns_string() {
        assert_eq!(placeholder(), "openpencil-shell-core skeleton");
    }
}
