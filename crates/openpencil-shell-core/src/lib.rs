//! OpenPencil shell core — re-export shim.
//!
//! Per spec v19 §1.2 (FROZEN 2026-05-04): this crate must compile on
//! wasm32-unknown-unknown. winit / accesskit_winit / skia-safe live in
//! `openpencil-shell-native`; wasm-bindgen / web-sys / CanvasKit live in
//! `openpencil-shell-web`.
//!
//! Phase 7 strangler reorg: the widget facade, theme, layout-resolved
//! render scene, and design-variable aggregation were extracted into the
//! `op-editor-ui` crate; i18n into `op-i18n`; the `RenderBackend` trait
//! into `op-editor-core`; the MCP / codegen / figma / AI-chat modules
//! into their own `op-*` crates. What remains here is a thin re-export
//! shim so the hosts (`openpencil-shell-native` / `openpencil-shell-web`
//! / `openpencil-desktop`) keep resolving `openpencil_shell_core::*`
//! paths unchanged until the Task 7.3 host rename dissolves this crate.
//!
//! The [`jian`] module re-exports `jian_core::render::{DrawOp, Paint,
//! TextRun, …}` + geometry/scene aliases for shell-native's internal
//! translation (widget code never sees them).

pub mod jian;

// Phase 3 strangler reorg: i18n extracted into op-i18n. Re-exported as
// `i18n` so `crate::i18n::translate` / `crate::i18n::Locale` resolve.
pub use op_i18n as i18n;

// Phase 4 strangler reorg: the wasm-clean RenderBackend trait moved into
// op-editor-core. Re-exported as `render_backend`.
pub use op_editor_core::render_backend;

// Phase 7 strangler reorg: the widget facade + theme + layout scene +
// scene-var aggregation moved into op-editor-ui. Re-exported so
// `openpencil_shell_core::widgets` / `::theme` / `::layout_scene` /
// `::layout_scene_hit` / `::scene_vars` paths still resolve.
pub use op_editor_ui::{layout_scene, layout_scene_hit, scene_vars, theme, widgets};

// Re-export the primary API for upstream crates / widgets / tests.
pub use op_editor_core::render_backend::{Color, Point2D, Rect, RenderBackend, TextLayout};
pub use op_editor_ui::Theme;

/// Re-exports of Jian gesture / event types so shell consumers can use the
/// canonical Jian types directly without an OP-specific translation layer.
pub use jian_core::gesture::{
    FocusEvent, ImeEvent, ImeKind, KeyCode, KeyEvent, KeyLocation, KeyState, KeyValue, Modifiers,
    MouseButtons, NamedKey, PointerEvent, PointerId, PointerKind, PointerPhase, ScrollMode,
    WheelEvent,
};
