//! Desktop codegen session — drives the pull-based `CodegenPipeline` on a
//! worker thread and streams progress into `editor_state.codegen`. Mirrors
//! `chat_session.rs` (worker thread + mpsc channel + per-frame pump +
//! `launch_if_pending`); like `design_session.rs` it carries a single
//! progress channel and never mutates the document.
//!
//! The pipeline is pull-based: `step()` returns `Dispatch(reqs)` until the
//! host has run each model request and fed the streamed text back via
//! `on_delta` / `on_complete` / `on_error`. The worker drains each request's
//! `ChatProvider::send` iterator (blocking) off the UI thread, then emits a
//! `Progress` delta so the panel can advance. Terminal `Done` / `Failed`
//! carry the assembled code / assets back to the UI pump.

#[cfg(test)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
#[cfg(test)]
use op_codegen::ai::types::CodegenInput;
#[cfg(test)]
use op_editor_host_core::codegen::framework_ext;
#[cfg(test)]
use op_editor_host_core::codegen_session::run_pipeline;
use op_editor_host_core::codegen_session::{
    drain_codegen_cancel_state, pump_codegen_state, retire_stale_codegen_session,
};
#[cfg(test)]
pub use op_editor_host_core::codegen_session::{CodegenDelta, CodegenResult};
pub use op_editor_host_core::codegen_session::{
    CodegenDocumentIdentity, CodegenResults, CodegenSession,
};
use op_host_native::WidgetHostNative;

use crate::chat_session::{provider_for_selected_model, selected_cli_model_id};

pub fn document_identity(host: &WidgetHostNative) -> CodegenDocumentIdentity {
    (
        host.document_epoch(),
        host.editor_state().document_generation(),
        host.editor_state().codegen.document_reset_epoch(),
    )
}

/// Pump the in-flight generation's deltas into `editor_state.codegen`.
/// Clears `current` once the turn finishes and parks the completed result
/// (asset bytes) in `results`. Returns true when state changed so the
/// caller can dirty the redraw.
pub fn pump(
    host: &mut WidgetHostNative,
    current: &mut Option<CodegenSession>,
    results: &mut CodegenResults,
) -> bool {
    let live_identity = document_identity(host);
    let changed = pump_codegen_state(
        &mut host.editor_state_mut().codegen,
        live_identity,
        current,
        results,
    );
    if changed {
        host.mark_editor_state_dirty();
    }
    changed
}

/// Drain a Generate / Regenerate request raised by the Code panel and launch
/// a worker turn. Clears the pending flags first, then resolves the input
/// (selection, else the whole active page) + provider; nothing to generate
/// from (empty page / dead selection) or an unconfigured model surfaces an
/// inline error instead of starting a turn. Returns true when state changed
/// (a turn launched OR an error was written).
pub fn launch_codegen_if_pending(
    host: &mut WidgetHostNative,
    current: &mut Option<CodegenSession>,
) -> bool {
    let live_document_identity = document_identity(host);
    retire_stale_codegen_session(current, live_document_identity);
    // A LIVE run blocks a new launch; a canceled run still draining its
    // dropped deltas does not — the fresh run replaces it (and gets a
    // strictly larger run epoch), TS parity: cancel + regenerate is
    // immediate.
    if current.as_ref().is_some_and(|s| !s.is_canceled()) {
        return false;
    }
    let cg = &host.editor_state().codegen;
    if !cg.pending_generate && !cg.pending_regenerate {
        return false;
    }
    // Clear the flags first so a failed launch doesn't re-fire every frame.
    {
        let cg = &mut host.editor_state_mut().codegen;
        cg.pending_generate = false;
        cg.pending_regenerate = false;
    }
    let Some((input, _raw)) = crate::codegen_input::build_codegen_input(host.editor_state()) else {
        let cg = &mut host.editor_state_mut().codegen;
        cg.error = Some("Select nodes to generate code".into());
        cg.phase = op_editor_core::codegen::CodegenPhase::Error;
        return true;
    };
    // Capture the target framework BEFORE `input` is moved into the worker.
    let framework = host.editor_state().codegen.framework;
    if let Some(error) = fixed_provider_launch_error(host) {
        let cg = &mut host.editor_state_mut().codegen;
        cg.error = Some(error);
        cg.phase = op_editor_core::codegen::CodegenPhase::Error;
        return true;
    }
    let model = selected_cli_model_id(host);
    let Some(provider) = provider_for_selected_model(host) else {
        let cg = &mut host.editor_state_mut().codegen;
        cg.error = Some("No model configured".into());
        cg.phase = op_editor_core::codegen::CodegenPhase::Error;
        return true;
    };
    // Keep this run's targets on the session until Done. A failed
    // regeneration can keep displaying the previous successful code, so
    // overwriting its snapshot at launch would create a mixed cache entry.
    let selection_snapshot: Vec<String> = host
        .editor_state()
        .selection
        .set
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();
    let session = match CodegenSession::try_start_with_model(provider, input, framework, model) {
        Ok(session) => session
            .with_document_identity(live_document_identity)
            .with_selection_snapshot(selection_snapshot),
        Err(error) => {
            let cg = &mut host.editor_state_mut().codegen;
            cg.error = Some(error.to_string());
            cg.phase = op_editor_core::codegen::CodegenPhase::Error;
            return true;
        }
    };
    let cg = &mut host.editor_state_mut().codegen;
    cg.progress = Default::default();
    cg.error = None;
    cg.phase = op_editor_core::codegen::CodegenPhase::Generating;
    *current = Some(session);
    true
}

/// Built-in and ACP selections carry their own ready-state and are validated
/// while constructing the provider. Fixed CLI providers must have completed
/// the connect probe; otherwise the default agent index would silently spawn
/// an unconfigured Claude turn and fail much later inside a chunk request.
fn fixed_provider_launch_error(host: &WidgetHostNative) -> Option<String> {
    use op_editor_core::agent_settings::ProviderConnectPhase;

    let state = host.editor_state();
    if state
        .chat
        .selected_model_entry()
        .is_some_and(|entry| entry.builtin_provider_id.is_some() || entry.acp_agent_id().is_some())
    {
        return None;
    }
    let provider = *op_editor_core::AgentProvider::ALL.get(state.editor_ui.chat_selected_agent)?;
    let settings = &state.editor_ui.agent_settings;
    if settings.provider_verified_connected(provider) {
        return None;
    }
    let idx = op_editor_core::agent_settings::AgentSettings::provider_index(provider);
    let connection = settings.provider_connection.get(idx)?;
    Some(match connection.phase {
        ProviderConnectPhase::Probing => format!(
            "{} is still connecting. Try code generation again when the connection check finishes.",
            provider.name()
        ),
        ProviderConnectPhase::Error => connection.error.clone().unwrap_or_else(|| {
            format!(
                "{} is not available. Reconnect it in Agent Settings before generating code.",
                provider.name()
            )
        }),
        ProviderConnectPhase::Idle | ProviderConnectPhase::Connected => format!(
            "Connect {} in Agent Settings before generating code.",
            provider.name()
        ),
    })
}

/// Drain a Cancel request raised by the Code panel (TS parity:
/// `abortRef.current?.abort()`). Raises the in-flight run's shared abort
/// flag — the worker stops at its next hook point — and leaves the
/// session parked so `pump` drops every delta the stale run still emits.
/// The UI phase was already flipped by the Cancel action itself
/// (idle, or complete when previous code exists).
pub fn drain_codegen_cancel_request(
    host: &mut WidgetHostNative,
    current: &mut Option<CodegenSession>,
) -> bool {
    if !drain_codegen_cancel_state(&mut host.editor_state_mut().codegen, current) {
        return false;
    }
    host.mark_editor_state_dirty();
    true
}

#[cfg(test)]
#[path = "codegen_session_tests.rs"]
mod tests;
