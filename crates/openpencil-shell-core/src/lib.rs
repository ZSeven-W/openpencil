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

pub mod agent_settings_state;
pub mod chat_models;
pub mod chat_provider;
pub mod codegen;
pub mod document;
pub mod figma;
pub mod i18n;
pub mod jian;
pub mod mcp;
#[cfg(test)] mod mcp_tests;
pub mod render_backend;
pub mod theme;
pub mod widgets;

// Re-export the primary API for upstream crates / widgets / tests.
pub use render_backend::{Color, Point2D, Rect, RenderBackend, TextLayout};
pub use theme::Theme;

/// Re-exports of Jian gesture / event types so shell consumers can use the
/// canonical Jian types directly without an OP-specific translation layer.
/// Per user 2026-05-05 directive: render engine + event types stay
/// consistent with Jian; OP-specific differentiation is at the canvas
/// viewport / chrome layer (single-page + infinite canvas recommended,
/// multi-page also supported), not at event-type abstraction.
///
/// Step 1b §2.4 additions (P0.5A jian PR):
/// - `KeyEvent` + supporting `KeyValue` / `NamedKey` / `KeyCode` /
///   `KeyLocation` / `KeyState` for browser + winit keyboard input.
/// - `ImeEvent` + `ImeKind` for IME composition (UTF-8 byte-indexed
///   selection per spec §2.4).
/// - `FocusEvent` for window-level / widget-level focus transitions
///   (separate from `FocusManager`'s internal Tab-ring chain).
/// - `WheelEvent` + `ScrollMode` for W3C `WheelEvent.deltaMode` parity
///   on browser + native winit hosts.
pub use jian_core::gesture::{
    FocusEvent, ImeEvent, ImeKind, KeyCode, KeyEvent, KeyLocation, KeyState, KeyValue, Modifiers,
    MouseButtons, NamedKey, PointerEvent, PointerId, PointerKind, PointerPhase, ScrollMode,
    WheelEvent,
};
