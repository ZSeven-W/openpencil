//! OpenPencil shell core — platform-agnostic widget facade + RenderBackend trait.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate must compile on
//! wasm32-unknown-unknown. winit / accesskit_winit / skia-safe live in
//! `openpencil-shell-native`; wasm-bindgen / web-sys / CanvasKit live in
//! `openpencil-shell-web`.
//!
//! v19 pivot — this crate is a thin Jian wrapper:
//! - the [`jian`] module re-exports `jian_core::render::{DrawOp, Paint, TextRun, …}`
//!   + geometry/scene aliases for shell-native's internal translation (widget code never sees them).
//! - the [`render_backend`] module defines OP's own widget-facing facade
//!   (`RenderBackend` trait + `Rect` / `Color` / `TextLayout`, spec §5.2).
//! - the [`event`] module declares OP's `ShellEvent` enum + sub-types
//!   (spec §5.1). Widget code consumes these on every platform; the
//!   desktop Jian/winit → ShellEvent mapper lives in
//!   `openpencil-shell-native::event` (target-gated to desktop today;
//!   Step 1f extends to mobile).

pub mod event;
pub mod jian;
pub mod render_backend;

// Re-export the primary API for upstream crates / widgets / tests.
pub use event::{
    ElementState, KeyCode, Modifiers, MouseButton, PointerId, ScrollDelta, ShellEvent, TouchForce,
    TouchId, TouchPhase, WindowEventKind,
};
pub use render_backend::{Color, Point2D, Rect, RenderBackend, TextLayout};
