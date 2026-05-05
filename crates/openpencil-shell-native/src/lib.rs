//! OpenPencil shell — native (desktop) backend.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate must NOT be linked into
//! the wasm32-unknown-unknown web bundle. Even though some deps (winit) compile
//! silently on wasm32 via web-sys, we use an explicit compile_error! guard to
//! make accidental inclusion a hard error.
//!
//! Step 1a Task 1 状态：仅装 deps（P0-pinned GL stack + jian-skia +
//! jian-host-desktop）；SharedSkiaContext / NativeBackend 实现在 Task 2 加。

#[cfg(target_arch = "wasm32")]
compile_error!(
    "openpencil-shell-native must NOT be compiled for wasm32 targets. \
     Use openpencil-shell-web for browser builds (spec v19 §1.2)."
);

/// Skeleton placeholder until Task 2 lands `SharedSkiaContext` / `NativeBackend`.
///
/// 用 OP `Color::RED` 命名常量证明 shell-core re-export 链通了；这是最小
/// link-check （Step 1a Task 1 不实现 Skia surface 与 GL provider，那些
/// 是 Task 2 的范围）。
pub fn placeholder() -> &'static str {
    let _red = openpencil_shell_core::Color::RED;
    "openpencil-shell-native skeleton (Task 1: deps wired)"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_links_core() {
        assert!(placeholder().contains("Task 1"));
    }
}
