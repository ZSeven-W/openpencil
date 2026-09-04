//! Initial `.op` ingestion shared by viewer and editor session creation.

use crate::error::{FfiError, FfiResult};
use crate::OpStatus;
use op_editor_core::EditorState;

pub(crate) fn load_initial_state(source: &str) -> FfiResult<EditorState> {
    // Canonical load: strict schema with legacy-major compatibility,
    // exactly the path `op-host-services::doc_io` uses for the desktop editor.
    let meta = op_pen_loader::extract_editor_meta_with_report(source).map(|value| value.meta);
    let loaded =
        op_pen_loader::payload::load_canonical_with_compatibility(source).map_err(|error| {
            FfiError::new(
                OpStatus::BadDocument,
                format!("document rejected by the canonical schema: {error}"),
            )
        })?;
    let mut state = EditorState::from_document(loaded.loaded.value);
    op_pen_loader::apply_editor_meta_or_legacy_fallback(&mut state, meta);
    Ok(state)
}
