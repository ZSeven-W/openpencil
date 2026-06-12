//! `NativeBackend` — frame-scoped Jian-DrawOp adapter for OP widget facade.
//!
//! Per spec v19 §5.2.1 the backend exposes the same method surface as
//! [`op_editor_ui::RenderBackend`] but takes
//! `canvas: &skia_safe::Canvas` as an explicit parameter on every drawing
//! call. This avoids the `&mut Canvas` borrow-conflict trap a previous
//! design (canvas-as-field with `'a` lifetime) hit (round 3 BLOCK-R3-3).
//! Step 1c+ may wrap `NativeBackend` inside a `WithCanvas<'a>` newtype so
//! the OP `RenderBackend` trait gets a real impl.

pub mod skia;

pub use skia::{enumerate_system_font_families, to_jian_color, to_jian_rect, NativeBackend};
