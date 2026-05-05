//! Jian re-export module (spec v19 §2 / §1.2).
//!
//! `openpencil-shell-core` is a thin Jian wrapper — it re-exports
//! `jian_core::render::{DrawOp, Paint, TextRun, …}` through the OP layer for
//! shell-native's internal translation. geometry / scene types get a `Jian*`
//! prefix to avoid clashing with OP's own `Rect` / `Color` (`render_backend.rs`).
//!
//! **Contract (spec §5.2.1, line 746)**: inside the NativeBackend impl, the
//! v19 wrapper **must not let widget code see `jian_core::render::DrawOp`
//! directly** — widgets only call OP `RenderBackend` methods. These re-exports
//! are for **shell-native internal** translation, not for widget code.

// ────────────────────────────────────────────────────────────────────────────
// jian_core::render — DrawOp command buffer + Paint / TextRun / other draw descriptors
// ────────────────────────────────────────────────────────────────────────────

pub use jian_core::render::{
    BorderRadii, DrawOp, GradientStop, ImageSource, LinearGradient, Paint, PathCommand,
    RadialGradient, RenderCommand, ShadowSpec, StrokeOp, TextAlign, TextRun,
};

// ────────────────────────────────────────────────────────────────────────────
// jian_core::geometry — `Jian*` prefix to avoid clashing with OP `Rect`
// ────────────────────────────────────────────────────────────────────────────

/// Jian's axis-aligned rectangle (`euclid::Rect<f32>`).
/// OP's own `crate::render_backend::Rect` is for the widget facade; shell-native
/// translates OP `Rect` → `JianRect` internally (`origin/size` → `euclid::Rect::new`).
pub type JianRect = jian_core::geometry::Rect;
pub type Size = jian_core::geometry::Size;
pub type JianPoint = jian_core::geometry::Point;
pub type Affine2 = jian_core::geometry::Affine2;

// ────────────────────────────────────────────────────────────────────────────
// jian_core::scene — Color (packed u32) gets a `Jian*` prefix
// ────────────────────────────────────────────────────────────────────────────

/// Jian's packed-RGBA color (`pub struct Color(pub u32)`); distinct from OP's
/// `crate::render_backend::Color` (RGBA f32 quad). NativeBackend bit-packs when
/// translating OP Color → JianColor.
pub type JianColor = jian_core::scene::Color;
