//! OpenPencil shell — native (desktop) backend.
//!
//! Per kickoff spec §1.2: this crate must NOT be linked into the wasm32-unknown-unknown
//! web bundle. Even though some deps (winit) compile silently on wasm32 via web-sys,
//! we use an explicit compile_error! guard to make accidental inclusion a hard error.

#[cfg(target_arch = "wasm32")]
compile_error!(
    "openpencil-shell-native must NOT be compiled for wasm32 targets. \
     Use openpencil-shell-web for browser builds (kickoff spec §1.2)."
);

pub fn placeholder() -> String {
    format!("openpencil-shell-native skeleton ({})", openpencil_shell_core::placeholder())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_links_core() {
        assert!(placeholder().contains("openpencil-shell-core"));
    }
}
