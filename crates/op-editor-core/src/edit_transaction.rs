//! Explicit editor transaction boundaries for collaboration adapters.
//!
//! Existing editor mutators intentionally keep their standalone undo/history
//! behavior. This module adds an orthogonal boundary that captures the
//! canonical document before a local gesture and reports the exact
//! before/after pair when that gesture finishes. Collaboration hosts can diff
//! that pair without guessing from legacy history pushes.

use crate::history::{EditorSnapshot, History};
use crate::state::EditorState;
use jian_ops_schema::PenDocument;
use std::collections::BTreeMap;

/// Source of a document edit or install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOrigin {
    /// A user-authored edit produced by this editor.
    Local,
    /// An authoritative commit received from another collaborator.
    RemoteCommit,
    /// A document rebuilt by replaying already-authoritative operations.
    Replay,
    /// A complete authoritative document snapshot.
    Snapshot,
}

/// Opaque checkpoint captured before one local editor gesture.
///
/// The host owns this value while the gesture is active. Its presence is also
/// the natural gate for queueing remote document commits until the local edit
/// has either completed or rolled back.
#[must_use = "a local edit capture must be ended or discarded explicitly"]
#[derive(Debug)]
pub struct LocalEditCapture {
    generation: u64,
    rollback: RollbackState,
}

/// Result of ending a local edit.
#[derive(Debug)]
pub enum LocalEditOutcome {
    /// The canonical document did not change.
    NoChange,
    /// The document changed and can be diffed or atomically rolled back.
    Changed(CompletedLocalEdit),
}

/// Exact before/after documents for one completed local gesture.
///
/// The rollback checkpoint remains private so callers cannot partially
/// restore editor internals. If diffing, replay, or hash verification fails,
/// pass the value back to [`EditorState::rollback_local_edit`].
#[must_use = "a completed edit must be accepted or rolled back by its caller"]
#[derive(Debug)]
pub struct CompletedLocalEdit {
    inner: Box<CompletedLocalEditData>,
}

#[derive(Debug)]
struct CompletedLocalEditData {
    generation: u64,
    before: PenDocument,
    after: PenDocument,
    rollback: RollbackState,
}

impl CompletedLocalEdit {
    /// Canonical document before the local gesture.
    pub fn before(&self) -> &PenDocument {
        &self.inner.before
    }

    /// Canonical document after the local gesture.
    pub fn after(&self) -> &PenDocument {
        &self.inner.after
    }

    /// Consume a successfully validated edit without changing editor state.
    pub fn accept(self) {}
}

#[derive(Debug)]
struct RollbackState {
    snapshot: EditorSnapshot,
    history: History,
    active_theme: BTreeMap<String, String>,
}

/// Typed failure while ending or rolling back a local edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalEditError {
    /// A whole-document replacement happened while the gesture was active.
    DocumentGenerationChanged { expected: u64, actual: u64 },
    /// The live document changed after the completed edit was returned.
    DocumentChangedAfterEdit,
}

impl std::fmt::Display for LocalEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DocumentGenerationChanged { expected, actual } => write!(
                f,
                "document generation changed during local edit (expected {expected}, got {actual})"
            ),
            Self::DocumentChangedAfterEdit => {
                f.write_str("document changed after the local edit completed")
            }
        }
    }
}

impl std::error::Error for LocalEditError {}

impl EditorState {
    /// Capture the exact document and rollback state before one local edit.
    ///
    /// This does not modify history, revision, or any UI state.
    pub fn begin_local_edit(&self) -> LocalEditCapture {
        LocalEditCapture {
            generation: self.document_generation,
            rollback: RollbackState {
                snapshot: self.snapshot_for_history(),
                history: self.history.clone(),
                active_theme: self.ui.variables.active_theme.clone(),
            },
        }
    }

    /// Finish one local edit and return its exact document delta.
    ///
    /// A no-op removes any legacy history/revision bookkeeping accidentally
    /// produced during the gesture, while preserving selection and view state.
    pub fn end_local_edit(
        &mut self,
        capture: LocalEditCapture,
    ) -> Result<LocalEditOutcome, LocalEditError> {
        self.ensure_local_edit_generation(capture.generation)?;
        let before = capture.rollback.snapshot.doc.materialize();
        if before == self.doc {
            self.history = capture.rollback.history;
            self.revision = capture.rollback.snapshot.revision;
            self.sync_dirty_flag();
            return Ok(LocalEditOutcome::NoChange);
        }

        if self.revision == capture.rollback.snapshot.revision {
            self.mark_document_changed();
        }
        let after = self.doc.clone();
        Ok(LocalEditOutcome::Changed(CompletedLocalEdit {
            inner: Box::new(CompletedLocalEditData {
                generation: capture.generation,
                before,
                after,
                rollback: capture.rollback,
            }),
        }))
    }

    /// Restore the editor checkpoint for a completed but unsupported edit.
    ///
    /// Legacy history entries and document-keyed drafts created by the edit
    /// are discarded. The revision allocator is deliberately not restored:
    /// the next edit receives a never-before-used revision even though the
    /// visible revision returns to the checkpoint's exact value.
    pub fn rollback_local_edit(
        &mut self,
        completed: CompletedLocalEdit,
    ) -> Result<(), LocalEditError> {
        let CompletedLocalEditData {
            generation,
            after,
            rollback,
            ..
        } = *completed.inner;
        self.ensure_local_edit_generation(generation)?;
        if self.doc != after {
            return Err(LocalEditError::DocumentChangedAfterEdit);
        }

        let RollbackState {
            snapshot,
            history,
            active_theme,
        } = rollback;
        self.editor_ui.clear_document_derived();
        self.restore(snapshot);
        self.history = history;
        self.clear_document_drafts(active_theme);
        self.codegen.reset_for_document_replacement();
        self.instance_write_virtual_anchor = None;
        jian_ops_schema::image_thumbs::activate_for_document(&self.doc);
        Ok(())
    }

    fn ensure_local_edit_generation(&self, expected: u64) -> Result<(), LocalEditError> {
        let actual = self.document_generation;
        if expected == actual {
            Ok(())
        } else {
            Err(LocalEditError::DocumentGenerationChanged { expected, actual })
        }
    }
}
