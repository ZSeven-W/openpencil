//! State-layer mirror of the design-md panel's buttons.
//!
//! Mirrors the hoverable subset of `DesignMdHit` (the widget crate's
//! click enum) for the hover wash stored on
//! `EditorUiState.design_md_hover`. Drag-header / inside hits never
//! hover. Same wasm32-clean discipline as the other `*_state` mirrors.

/// Which design-md-panel button the cursor is over. `None` = no hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignMdButton {
    /// The header `✕` close button.
    Close,
    /// The header import button.
    Import,
    /// The header / empty-state auto-generate button.
    AutoGenerate,
    /// The header export button.
    Export,
    /// The footer "remove" link.
    Remove,
    /// A collapsible section header row, by section index (0..6).
    ToggleSection(u8),
}
