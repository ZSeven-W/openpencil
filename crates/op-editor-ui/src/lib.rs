//! OpenPencil editor UI — platform-agnostic widget facade.
//!
//! Extracted out of `openpencil-shell-core` in the Phase 7 strangler
//! reorg. This crate must compile on wasm32-unknown-unknown — winit /
//! accesskit_winit / skia-safe live in `openpencil-shell-native`;
//! wasm-bindgen / web-sys / CanvasKit live in `openpencil-shell-web`.
//!
//! Modules:
//! - [`widgets`] — the `Widget` facade trait, render primitives, the
//!   editor-UI compositions (`LayerPanel` / `PropertyPanel` / `Toolbar`
//!   / `TopBar` / settings modal / pickers), the `CanvasViewport`
//!   center canvas, the lucide icon glyph drawer, and `editor_state_ext`
//!   (theme / translation derivations over `EditorUiState`).
//! - [`theme`] — shadcn-dark palette tokens.
//! - [`layout_scene`] / [`layout_scene_hit`] — a paint-only,
//!   layout-resolved render scene + its hit-test (input path).
//! - [`scene_vars`] — paint-time design-variable aggregation consumed
//!   by the `LayoutScene` scene builder.
//!
//! The `Color` / `Point2D` / `Rect` / `RenderBackend` / `TextLayout`
//! facade comes from `op_editor_core::render_backend`; gesture / event
//! types come from `jian_core::gesture`. Both are re-exported at the
//! crate root so widget code can use the short `crate::Color` form.

// i18n string tables — re-exported as `i18n` so `crate::i18n::translate`
// paths inside the widgets keep resolving.
pub use op_i18n as i18n;

// The wasm-clean RenderBackend trait + facade types live in
// op-editor-core; re-exported as `render_backend` and at the crate root.
pub use op_editor_core::render_backend;

pub mod accessibility;
pub mod layout_scene;
pub mod layout_scene_hit;
pub mod scene_vars;
pub mod theme;
pub mod widgets;

// Re-export the primary API for the hosts / widgets / tests.
pub use layout_scene::{SceneTextAlign, SceneTextVerticalAlign};
pub use op_editor_core::render_backend::{
    Color, ImageAdjustments, ImageDrawMode, Point2D, Rect, RenderBackend, TextLayout,
};
pub use theme::Theme;

/// Re-exports of Jian gesture / event types so widget code can use the
/// canonical Jian types directly via the short `crate::` form.
pub use jian_core::gesture::{
    FocusEvent, ImeEvent, ImeKind, KeyCode, KeyEvent, KeyLocation, KeyState, KeyValue, Modifiers,
    MouseButtons, NamedKey, PointerEvent, PointerId, PointerKind, PointerPhase, ScrollMode,
    WheelEvent,
};

#[cfg(test)]
pub(crate) mod agent_indicator_test_support {
    use std::sync::{LazyLock, Mutex, MutexGuard};

    static LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    pub(crate) fn lock() -> MutexGuard<'static, ()> {
        LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }
}
