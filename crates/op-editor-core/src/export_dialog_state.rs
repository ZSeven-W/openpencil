//! State-layer mirror of the modal export dialog's buttons.
//!
//! `op_editor_ui::widgets::export_dialog::ExportDialogHit` lives in the
//! widget crate, so `ExportDialogButton` mirrors it here for the hover
//! state stored on `EditorUiState.export_dialog_hover` — same wasm32-
//! clean discipline as the other `*_state` mirrors. The `Format`
//! variant carries the canonical [`crate::ExportFormat`], so no extra
//! conversion is needed on the state side.

use crate::ExportFormat;

/// Which modal-export-dialog button the cursor is over. `None` on
/// `EditorUiState.export_dialog_hover` = no hover wash. Covers the
/// format + scale radio pills and the Cancel / Export footer buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportDialogButton {
    /// A format radio pill (PNG / JPEG / WEBP / SVG / PDF).
    Format(ExportFormat),
    /// A scale radio pill (1× / 2× / 3×), as `1 | 2 | 3`.
    Scale(u8),
    /// The Cancel footer button.
    Cancel,
    /// The Export footer button.
    Export,
}
