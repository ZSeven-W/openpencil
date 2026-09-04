//! Host-neutral Code panel session state folding and result cache.

use op_codegen::ai::types::AssetFile;
use op_editor_core::codegen::{CodegenPhase, CodegenState, Framework};

use crate::codegen_session::{CodegenDelta, CodegenSession};

/// Identity of the editor document a generation belongs to.
///
/// The components are the concrete host's whole-state epoch,
/// [`op_editor_core::EditorState`]'s document generation, and
/// [`op_editor_core::codegen::CodegenState::document_reset_epoch`]. The third
/// value covers collaboration installs/rollbacks that intentionally preserve
/// the broader editor document generation.
pub type CodegenDocumentIdentity = (u64, u64, u64);

/// The completed result kept host-side for Download.
#[derive(Default, Clone)]
pub struct CodegenResult {
    pub code: String,
    pub framework_ext: String,
    pub assets: Vec<AssetFile>,
}

/// Completed generation payloads keyed by document identity and framework.
///
/// UI state carries lightweight asset metadata, while the raw bytes remain in
/// this host-side cache for Download. Changing documents invalidates the
/// entire cache; switching framework tabs only selects another entry.
#[derive(Default)]
pub struct CodegenResults {
    document_identity: Option<CodegenDocumentIdentity>,
    entries: Vec<(Framework, CodegenResult)>,
}

impl CodegenResults {
    pub fn insert(
        &mut self,
        document_identity: CodegenDocumentIdentity,
        framework: Framework,
        result: CodegenResult,
    ) {
        if self.document_identity != Some(document_identity) {
            self.document_identity = Some(document_identity);
            self.entries.clear();
        }
        if let Some((_, cached)) = self
            .entries
            .iter_mut()
            .find(|(cached_framework, _)| *cached_framework == framework)
        {
            *cached = result;
        } else {
            self.entries.push((framework, result));
        }
    }

    pub fn get(
        &self,
        document_identity: CodegenDocumentIdentity,
        framework: Framework,
    ) -> Option<&CodegenResult> {
        if self.document_identity != Some(document_identity) {
            return None;
        }
        self.entries
            .iter()
            .find(|(cached_framework, _)| *cached_framework == framework)
            .map(|(_, result)| result)
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Cancel and retire a session captured against a superseded document.
///
/// Returns `true` only when a stale session was removed. This is useful both
/// before a new launch and from render-free mobile background pumps.
pub fn retire_stale_codegen_session(
    current: &mut Option<CodegenSession>,
    live_identity: CodegenDocumentIdentity,
) -> bool {
    if current
        .as_ref()
        .is_none_or(|session| session.document_identity == live_identity)
    {
        return false;
    }
    if let Some(stale) = current.take() {
        stale.cancel();
    }
    true
}

/// Fold queued worker deltas into transport-free editor state.
///
/// Raw completed assets are parked in `results`. Canceled or stale sessions
/// never mutate UI state and never become downloadable. Returns `true` only
/// when `state` changed; concrete hosts remain responsible for scheduling a
/// redraw when that happens.
pub fn pump_codegen_state(
    state: &mut CodegenState,
    live_identity: CodegenDocumentIdentity,
    current: &mut Option<CodegenSession>,
    results: &mut CodegenResults,
) -> bool {
    if retire_stale_codegen_session(current, live_identity) {
        return false;
    }
    let Some(session) = current.as_mut() else {
        return false;
    };
    if session.is_canceled() {
        // A canceled run may still have queued progress or a terminal event.
        // Drop all of it so late deltas cannot revive or overwrite the UI.
        loop {
            match session.rx.try_recv() {
                Ok(CodegenDelta::Progress(_)) => {}
                Ok(CodegenDelta::Done { .. }) | Ok(CodegenDelta::Failed(_)) => {
                    session.finished = true;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    session.finished = true;
                    break;
                }
            }
        }
        if session.finished {
            *current = None;
        }
        return false;
    }

    let mut changed = false;
    loop {
        match session.rx.try_recv() {
            Ok(CodegenDelta::Progress(progress)) => {
                state.progress = progress;
                state.phase = CodegenPhase::Generating;
                changed = true;
            }
            Ok(CodegenDelta::Done {
                code,
                degraded,
                assets,
            }) => {
                let selection_snapshot = std::mem::take(&mut session.selection_snapshot);
                let metas = assets
                    .iter()
                    .map(|asset| op_editor_core::codegen::AssetMeta {
                        relative_path: asset.relative_path.clone(),
                        byte_len: asset.bytes.len(),
                    })
                    .collect();
                state.code = code.clone();
                state.code_scroll.offset = 0.0;
                state.code_selection = None;
                state.degraded = degraded;
                state.assets = metas;
                state.selection_snapshot = selection_snapshot;
                state.phase = CodegenPhase::Complete;
                state.pending_generate = false;
                state.pending_regenerate = false;
                results.insert(
                    session.document_identity,
                    session.framework,
                    CodegenResult {
                        code,
                        framework_ext: crate::codegen::framework_ext(session.framework).into(),
                        assets,
                    },
                );
                session.finished = true;
                changed = true;
            }
            Ok(CodegenDelta::Failed(error)) => {
                state.error = Some(error);
                state.phase = CodegenPhase::Error;
                state.pending_generate = false;
                state.pending_regenerate = false;
                session.finished = true;
                changed = true;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                if !session.finished {
                    state.error = Some("Code generation ended unexpectedly".into());
                    state.phase = CodegenPhase::Error;
                    state.pending_generate = false;
                    state.pending_regenerate = false;
                    changed = true;
                }
                session.finished = true;
                break;
            }
        }
    }
    if session.finished {
        *current = None;
    }
    changed
}

/// Drain the Code panel's Cancel intent into the active worker token.
///
/// The property action has already selected the UI phase to display. The
/// session stays parked until [`pump_codegen_state`] drops any late deltas
/// and observes a terminal event or disconnect.
pub fn drain_codegen_cancel_state(
    state: &mut CodegenState,
    current: &mut Option<CodegenSession>,
) -> bool {
    if !std::mem::take(&mut state.pending_cancel) {
        return false;
    }
    if let Some(session) = current.as_ref() {
        session.cancel();
    }
    true
}
