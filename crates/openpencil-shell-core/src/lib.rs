//! OpenPencil shell core — platform-agnostic widget facade + RenderBackend trait.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate must compile on
//! wasm32-unknown-unknown. winit / accesskit_winit / skia-safe live in
//! `openpencil-shell-native`; wasm-bindgen / web-sys / CanvasKit live in
//! `openpencil-shell-web`.
//!
//! v19 pivot — 该 crate 是 Jian 薄 wrapper：
//! - [`jian`] 模块 re-export `jian_core::render::{DrawOp, Paint, TextRun, …}`
//!   + geometry/scene aliases，给 shell-native 内部翻译用（widget code 看不到）。
//! - [`render_backend`] 模块定义 OP 自有 widget-facing facade
//!   (`RenderBackend` trait + `Rect` / `Color` / `TextLayout`，spec §5.2)。

pub mod jian;
pub mod render_backend;

// Re-export 主要 API 给上层 crate / widgets / tests 用。
pub use render_backend::{Color, Point2D, Rect, RenderBackend, TextLayout};
