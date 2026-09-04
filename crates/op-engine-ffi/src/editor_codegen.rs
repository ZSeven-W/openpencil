//! Native mobile Code panel runtime.
//!
//! This is the mobile host arm around the transport-free codegen session:
//! it resolves the selected built-in model, launches/cancels the worker,
//! folds deltas on the engine owner thread, and freezes Download / AI Bundle
//! bytes for the platform export surface. No desktop CLI, ACP process, GPU,
//! or shell UI is touched here, so the same pump can continue while a mobile
//! drawing surface is suspended.

use std::collections::VecDeque;

use op_editor_core::codegen::{CodeGenProgress, CodegenPhase};
use op_editor_host_core::codegen::build_codegen_input;
pub(crate) use op_editor_host_core::codegen_export::CodegenArtifact;
use op_editor_host_core::codegen_export::{generated_artifact, live_bundle_artifact};
use op_editor_host_core::codegen_session::{
    drain_codegen_cancel_state, pump_codegen_state, retire_stale_codegen_session,
    CodegenDocumentIdentity, CodegenResults, CodegenSession,
};
use op_host_native::WidgetHostNative;

use crate::editor_builtin_provider::MobileBuiltinProvider;
use crate::lifecycle::Session;

/// Match the foreground chat/codegen streaming cadence (~30 fps).
const CODEGEN_POLL_INTERVAL_MS: u64 = 33;
/// Download + Bundle can be raised in the same synthetic frame. Retain both,
/// but never let repeated forged actions accumulate unbounded zip bytes.
const MAX_STAGED_ARTIFACTS: usize = 2;

#[derive(Default)]
pub(crate) struct MobileCodegenHost {
    current: Option<CodegenSession>,
    results: CodegenResults,
    artifacts: VecDeque<CodegenArtifact>,
    document_identity: Option<CodegenDocumentIdentity>,
}

impl Drop for MobileCodegenHost {
    fn drop(&mut self) {
        if let Some(session) = self.current.take() {
            session.cancel();
        }
    }
}

impl MobileCodegenHost {
    pub(crate) fn has_background_work(&self, host: &WidgetHostNative) -> bool {
        let state = host.editor_state();
        self.current.is_some()
            || state.codegen.pending_generate
            || state.codegen.pending_regenerate
            || state.codegen.pending_cancel
    }

    /// Drain Code panel requests and fold worker deltas. Returns the next
    /// owner-thread deadline while generation remains live.
    pub(crate) fn pump(&mut self, host: &mut WidgetHostNative, now_ms: u64) -> Option<u64> {
        let identity = document_identity(host);
        self.rotate_document(identity);

        let mut changed =
            drain_codegen_cancel_state(&mut host.editor_state_mut().codegen, &mut self.current);
        changed |= self.guard_framework(host);
        changed |= self.launch_if_pending(host, identity);
        changed |= pump_codegen_state(
            &mut host.editor_state_mut().codegen,
            identity,
            &mut self.current,
            &mut self.results,
        );
        changed |= self.stage_pending_artifacts(host, identity);
        if changed {
            host.mark_editor_state_dirty();
        }

        self.current
            .as_ref()
            .map(|_| now_ms.saturating_add(CODEGEN_POLL_INTERVAL_MS))
    }

    /// OS background expiry/cancel path. Unlike the Code panel's normal
    /// cancel intent, this retires the session immediately so the platform
    /// can report no remaining background work in the same tick.
    pub(crate) fn cancel_background_work(&mut self, host: &mut WidgetHostNative) -> bool {
        self.rotate_document(document_identity(host));
        let had_session = if let Some(session) = self.current.take() {
            session.cancel();
            true
        } else {
            false
        };
        let state = &mut host.editor_state_mut().codegen;
        let had_pending = state.pending_generate
            || state.pending_regenerate
            || state.pending_cancel
            || state.phase == CodegenPhase::Generating;
        state.pending_generate = false;
        state.pending_regenerate = false;
        state.pending_cancel = false;
        if state.phase == CodegenPhase::Generating {
            state.phase = if state.code.is_empty() {
                CodegenPhase::Idle
            } else {
                CodegenPhase::Complete
            };
        }
        if had_session || had_pending {
            host.mark_editor_state_dirty();
            true
        } else {
            false
        }
    }

    /// Pop one frozen artifact for the existing platform export staging
    /// protocol. A whole-document replacement invalidates queued bytes before
    /// they can be handed to another document's shell action.
    pub(crate) fn drain_artifact(&mut self, host: &WidgetHostNative) -> Option<CodegenArtifact> {
        self.rotate_document(document_identity(host));
        self.artifacts.pop_front()
    }

    fn rotate_document(&mut self, identity: CodegenDocumentIdentity) {
        if self.document_identity == Some(identity) {
            return;
        }
        if let Some(session) = self.current.take() {
            session.cancel();
        }
        self.results = CodegenResults::default();
        self.artifacts.clear();
        self.document_identity = Some(identity);
    }

    /// Synthetic/framework changes cannot relabel a run. The normal UI
    /// disables framework tabs while generating; this host-side guard also
    /// covers stale/synthetic actions and keeps late output out of the wrong
    /// framework cache.
    fn guard_framework(&mut self, host: &mut WidgetHostNative) -> bool {
        let mismatched = self.current.as_ref().is_some_and(|session| {
            !session.is_canceled() && session.framework != host.editor_state().codegen.framework
        });
        if !mismatched {
            return false;
        }
        if let Some(session) = self.current.take() {
            session.cancel();
        }
        let state = &mut host.editor_state_mut().codegen;
        state.pending_generate = false;
        state.pending_regenerate = false;
        state.error =
            Some("The code target changed while generation was running. Generate it again".into());
        state.phase = CodegenPhase::Error;
        true
    }

    fn launch_if_pending(
        &mut self,
        host: &mut WidgetHostNative,
        identity: CodegenDocumentIdentity,
    ) -> bool {
        retire_stale_codegen_session(&mut self.current, identity);
        if self
            .current
            .as_ref()
            .is_some_and(|session| !session.is_canceled())
        {
            return false;
        }
        let requested = {
            let state = host.editor_state();
            state.codegen.pending_generate || state.codegen.pending_regenerate
        };
        if !requested {
            return false;
        }
        {
            let state = &mut host.editor_state_mut().codegen;
            state.pending_generate = false;
            state.pending_regenerate = false;
        }

        let Some((input, _raw)) = build_codegen_input(host.editor_state()) else {
            set_inline_error(host, "Select nodes to generate code");
            return true;
        };
        let provider = match MobileBuiltinProvider::from_selected_model(host.editor_state()) {
            Ok(provider) => provider,
            Err(error) => {
                set_inline_error(host, error.to_string());
                return true;
            }
        };
        let framework = host.editor_state().codegen.framework;
        let selection_snapshot = host
            .editor_state()
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect();
        let session = match CodegenSession::try_start(Box::new(provider), input, framework) {
            Ok(session) => session
                .with_document_identity(identity)
                .with_selection_snapshot(selection_snapshot),
            Err(error) => {
                set_inline_error(host, error.to_string());
                return true;
            }
        };
        {
            let state = &mut host.editor_state_mut().codegen;
            state.progress = CodeGenProgress::default();
            state.error = None;
            state.phase = CodegenPhase::Generating;
        }
        self.current = Some(session);
        true
    }

    fn stage_pending_artifacts(
        &mut self,
        host: &mut WidgetHostNative,
        identity: CodegenDocumentIdentity,
    ) -> bool {
        let (download, bundle, framework) = {
            let state = &mut host.editor_state_mut().codegen;
            (
                std::mem::take(&mut state.pending_download),
                std::mem::take(&mut state.pending_export_bundle),
                state.framework,
            )
        };
        if !download && !bundle {
            return false;
        }

        if download {
            match self
                .results
                .get(identity, framework)
                .map(generated_artifact)
            {
                Some(Ok(Some(artifact))) => self.queue_artifact(artifact),
                Some(Err(error)) => set_inline_error(host, error.to_string()),
                Some(Ok(None)) | None => set_inline_error(
                    host,
                    "Generated code is no longer available. Generate it again",
                ),
            }
        }
        if bundle {
            match live_bundle_artifact(host.editor_state()) {
                Ok(Some(artifact)) => self.queue_artifact(artifact),
                Ok(None) => set_inline_error(host, "Select nodes to export an AI bundle"),
                Err(error) => set_inline_error(host, error.to_string()),
            }
        }
        true
    }

    fn queue_artifact(&mut self, artifact: CodegenArtifact) {
        if self.artifacts.len() == MAX_STAGED_ARTIFACTS {
            self.artifacts.pop_front();
        }
        self.artifacts.push_back(artifact);
    }
}

fn document_identity(host: &WidgetHostNative) -> CodegenDocumentIdentity {
    (
        host.document_epoch(),
        host.editor_state().document_generation(),
        host.editor_state().codegen.document_reset_epoch(),
    )
}

fn set_inline_error(host: &mut WidgetHostNative, message: impl Into<String>) {
    let state = &mut host.editor_state_mut().codegen;
    state.error = Some(message.into());
    state.phase = CodegenPhase::Error;
    state.pending_generate = false;
    state.pending_regenerate = false;
}

impl Session {
    pub(crate) fn pump_editor_codegen(&mut self, now_ms: u64) -> Option<u64> {
        let Session {
            editor, codegen, ..
        } = self;
        editor.as_mut().and_then(|host| codegen.pump(host, now_ms))
    }
}

#[cfg(test)]
#[path = "editor_codegen_tests.rs"]
mod tests;
