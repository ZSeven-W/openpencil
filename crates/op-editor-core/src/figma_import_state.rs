//! State-layer mirror of the Figma-import modal's interactive targets.
//!
//! `op_editor_ui::widgets::figma_import::FigmaImportHit` lives in the
//! widget crate, so this `Copy` mirror captures the two hoverable
//! targets — the close `✕` and the browse drop-zone — for the hover
//! state on `EditorUiState.figma_import_hover`. Same wasm32-clean
//! discipline as the other `*_state` mirrors.

/// Which Figma-import-modal target the cursor is over. `None` =
/// no hover wash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigmaImportButton {
    /// The close `✕` in the modal header.
    Close,
    /// The dashed drop-zone (click to browse for a `.fig` file).
    DropZone,
}
