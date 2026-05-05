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
//! - event types are re-exported from `jian_core::gesture` directly (no
//!   OP-specific translation layer). Per user 2026-05-05 directive: OP's
//!   render engine + event types stay consistent with Jian; OP-specific
//!   differentiation lives at the canvas viewport / chrome layer
//!   (single-page + infinite canvas recommended, multi-page also supported).

pub mod jian;
pub mod render_backend;

// Re-export the primary API for upstream crates / widgets / tests.
pub use render_backend::{Color, Point2D, Rect, RenderBackend, TextLayout};

/// Re-exports of Jian gesture / event types so shell consumers can use the
/// canonical Jian types directly without an OP-specific translation layer.
/// Per user 2026-05-05 directive: render engine + event types stay
/// consistent with Jian; OP-specific differentiation is at the canvas
/// viewport / chrome layer (single-page + infinite canvas recommended,
/// multi-page also supported), not at event-type abstraction.
pub use jian_core::gesture::{
    Modifiers, MouseButtons, PointerEvent, PointerId, PointerKind, PointerPhase,
};
