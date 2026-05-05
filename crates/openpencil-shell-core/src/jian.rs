//! Jian re-export module (spec v19 §2 / §1.2).
//!
//! `openpencil-shell-core` 是 Jian 薄 wrapper —— 把
//! `jian_core::render::{DrawOp, Paint, TextRun, …}` 经 OP 层 re-export 给
//! shell-native 内部翻译用。geometry / scene 类型加 `Jian*` 前缀，避免
//! 与 OP 自有 `Rect` / `Color` (`render_backend.rs`) 冲突。
//!
//! **契约（spec §5.2.1 §746 行）**：v19 wrapper 在 NativeBackend impl 内
//! **不让 widget code 直接看到 `jian_core::render::DrawOp`** —— widget 只
//! 调 OP `RenderBackend` method。这些 re-export 是给 **shell-native 内部**
//! 翻译用，不是给 widget code 用。

// ────────────────────────────────────────────────────────────────────────────
// jian_core::render —— DrawOp 命令缓冲 + Paint / TextRun / 其它绘图描述符
// ────────────────────────────────────────────────────────────────────────────

pub use jian_core::render::{
    BorderRadii, DrawOp, GradientStop, ImageSource, LinearGradient, Paint, PathCommand,
    RadialGradient, RenderCommand, ShadowSpec, StrokeOp, TextAlign, TextRun,
};

// ────────────────────────────────────────────────────────────────────────────
// jian_core::geometry —— 加 Jian* 前缀避免与 OP `Rect` 冲突
// ────────────────────────────────────────────────────────────────────────────

/// Jian 的 axis-aligned 矩形 (`euclid::Rect<f32>`)。
/// OP 自有 `crate::render_backend::Rect` 是 widget facade 用；shell-native
/// 内部翻译时把 OP `Rect` → `JianRect`（`origin/size` → `euclid::Rect::new`）。
pub type JianRect = jian_core::geometry::Rect;
pub type Size = jian_core::geometry::Size;
pub type JianPoint = jian_core::geometry::Point;
pub type Affine2 = jian_core::geometry::Affine2;

// ────────────────────────────────────────────────────────────────────────────
// jian_core::scene —— Color (packed u32) 加 Jian* 前缀
// ────────────────────────────────────────────────────────────────────────────

/// Jian 的 packed-RGBA 颜色 (`pub struct Color(pub u32)`)；与 OP
/// `crate::render_backend::Color` (RGBA f32 quad) 不同。NativeBackend 内部
/// 翻译 OP Color → JianColor 时用位打包。
pub type JianColor = jian_core::scene::Color;
