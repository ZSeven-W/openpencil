//! State-layer enum for the floating zoom status bar's controls.
//!
//! `StatusBarButton` is the hit-test result of `StatusBar::hit_test`
//! and the value stored on `EditorUiState.statusbar_hover` for the
//! per-control `theme.button_hover` wash. It lives in `op-editor-core`
//! (not the widget crate) so the state struct can hold it while the
//! crate stays wasm32-clean — same discipline as `topbar_state`.

/// Which status-bar control the cursor is over / clicked. The pill
/// hosts a search-to-fit button and a `[- N% +]` zoom cluster.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarButton {
    /// Magnifier — frames the page content within the viewport.
    Search,
    /// Minus — zooms the canvas out one step.
    ZoomOut,
    /// Plus — zooms the canvas in one step.
    ZoomIn,
}
