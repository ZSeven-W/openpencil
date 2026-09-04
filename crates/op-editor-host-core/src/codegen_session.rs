//! Shared code-generation session worker.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
use op_codegen::ai::types::{AssetFile, CodegenInput, PipelineStep, RequestKind};
use op_codegen::ai::CodegenPipeline;
use op_editor_core::codegen::{CodeGenProgress, Framework};

pub use crate::codegen_runtime_state::{
    drain_codegen_cancel_state, pump_codegen_state, retire_stale_codegen_session,
    CodegenDocumentIdentity, CodegenResult, CodegenResults,
};

/// Streamed from the worker to the UI pump.
pub enum CodegenDelta {
    Progress(CodeGenProgress),
    Done {
        code: String,
        degraded: bool,
        assets: Vec<AssetFile>,
    },
    Failed(String),
}

static NEXT_RUN_EPOCH: AtomicU64 = AtomicU64::new(1);

/// Failure to launch the background code-generation worker.
#[derive(Debug, thiserror::Error)]
pub enum CodegenStartError {
    #[error("Could not start code generation: {source}")]
    ThreadSpawn {
        #[source]
        source: std::io::Error,
    },
}

type CodegenWorker = Box<dyn FnOnce() + Send + 'static>;

/// An in-flight generation. Host pumps own the UI-specific folding of deltas.
pub struct CodegenSession {
    pub rx: Receiver<CodegenDelta>,
    pub finished: bool,
    pub framework: Framework,
    /// Whole-document identity captured by the host when this run launched.
    /// Hosts compare it before folding any delta so an Open/New/import/sync
    /// replacement cannot receive a completion from the previous document.
    pub document_identity: CodegenDocumentIdentity,
    /// Node ids captured when this run launched. The host commits these to
    /// `CodegenState::selection_snapshot` only when the matching terminal
    /// `Done` is applied; failed/canceled runs simply drop them.
    pub selection_snapshot: Vec<String>,
    /// CLI model selected when this run launched. Built-in and ACP
    /// providers keep this `None` because their model lives in the provider
    /// configuration rather than on each request.
    pub model: Option<String>,
    pub cancel: Arc<AtomicBool>,
    pub run_epoch: u64,
}

impl CodegenSession {
    /// Bind a newly-started session to its host/document/codegen lifetime.
    ///
    /// The shared worker does not own a host, so constructors use the initial
    /// identity `(0, 0, 0)` and concrete hosts stamp their live identity before
    /// publishing the session.
    pub fn with_document_identity(mut self, document_identity: CodegenDocumentIdentity) -> Self {
        self.document_identity = document_identity;
        self
    }

    /// Bind the generation targets captured alongside the input.
    pub fn with_selection_snapshot(mut self, selection_snapshot: Vec<String>) -> Self {
        self.selection_snapshot = selection_snapshot;
        self
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn is_canceled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    /// Spawn a worker that drives the pipeline against `provider`.
    pub fn start(
        provider: Box<dyn ChatProvider>,
        input: CodegenInput,
        framework: Framework,
    ) -> Self {
        Self::start_with_model(provider, input, framework, None)
    }

    /// Fallible production entry point for starting a code-generation worker.
    pub fn try_start(
        provider: Box<dyn ChatProvider>,
        input: CodegenInput,
        framework: Framework,
    ) -> Result<Self, CodegenStartError> {
        Self::try_start_with_model(provider, input, framework, None)
    }

    /// Spawn a worker and forward the selected CLI model to every planning,
    /// chunk, and assembly request. Codegen uses several independent model
    /// turns, so applying the selection only to the first request is not
    /// sufficient.
    pub fn start_with_model(
        provider: Box<dyn ChatProvider>,
        input: CodegenInput,
        framework: Framework,
        model: Option<String>,
    ) -> Self {
        let fallback_model = model.clone();
        Self::try_start_with_model(provider, input, framework, model)
            .unwrap_or_else(|error| Self::failed_start_session(framework, fallback_model, error))
    }

    /// Fallible worker launch that forwards the selected model to every
    /// planning, chunk, and assembly request.
    pub fn try_start_with_model(
        provider: Box<dyn ChatProvider>,
        input: CodegenInput,
        framework: Framework,
        model: Option<String>,
    ) -> Result<Self, CodegenStartError> {
        Self::try_start_with_model_and_spawner(provider, input, framework, model, |worker| {
            std::thread::Builder::new()
                .name("op-codegen-turn".into())
                .spawn(worker)
                .map(drop)
        })
    }

    fn try_start_with_model_and_spawner<F>(
        provider: Box<dyn ChatProvider>,
        input: CodegenInput,
        framework: Framework,
        model: Option<String>,
        spawner: F,
    ) -> Result<Self, CodegenStartError>
    where
        F: FnOnce(CodegenWorker) -> std::io::Result<()>,
    {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        let worker_model = model.clone();
        let worker: CodegenWorker = Box::new(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_pipeline_with_model_and_cancel_token(
                    provider.as_ref(),
                    input,
                    worker_model.as_deref(),
                    &tx,
                    &worker_cancel,
                    Some(Arc::clone(&worker_cancel)),
                );
            }));
            if outcome.is_err() {
                let _ = tx.send(CodegenDelta::Failed(
                    "Code generation failed unexpectedly".into(),
                ));
            }
        });
        spawner(worker).map_err(|source| CodegenStartError::ThreadSpawn { source })?;
        Ok(CodegenSession {
            rx,
            finished: false,
            framework,
            document_identity: (0, 0, 0),
            selection_snapshot: Vec::new(),
            model,
            cancel,
            run_epoch: NEXT_RUN_EPOCH.fetch_add(1, Ordering::Relaxed),
        })
    }

    fn failed_start_session(
        framework: Framework,
        model: Option<String>,
        error: CodegenStartError,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = tx.send(CodegenDelta::Failed(error.to_string()));
        drop(tx);
        CodegenSession {
            rx,
            finished: false,
            framework,
            document_identity: (0, 0, 0),
            selection_snapshot: Vec::new(),
            model,
            cancel: Arc::new(AtomicBool::new(false)),
            run_epoch: NEXT_RUN_EPOCH.fetch_add(1, Ordering::Relaxed),
        }
    }
}

impl Drop for CodegenSession {
    fn drop(&mut self) {
        // Teardown and document replacement must stop the worker even when a
        // concrete host forgets to call `cancel` first. The worker owns only a
        // clone of this token, so setting it is non-blocking and never joins.
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Drive the pipeline to completion against `provider`, emitting deltas on
/// `tx`. Runs on the worker thread or synchronously in tests.
pub fn run_pipeline(
    provider: &dyn ChatProvider,
    input: CodegenInput,
    tx: &Sender<CodegenDelta>,
    cancel: &AtomicBool,
) {
    run_pipeline_with_model(provider, input, None, tx, cancel);
}

/// Drive the pipeline with an optional selected CLI model. Provider failures
/// are retained as bounded diagnostics and appended to a terminal pipeline
/// error, so the UI does not replace the actionable transport error with the
/// generic "All chunks failed" message.
pub fn run_pipeline_with_model(
    provider: &dyn ChatProvider,
    input: CodegenInput,
    model: Option<&str>,
    tx: &Sender<CodegenDelta>,
    cancel: &AtomicBool,
) {
    run_pipeline_with_model_and_cancel_token(provider, input, model, tx, cancel, None);
}

fn run_pipeline_with_model_and_cancel_token(
    provider: &dyn ChatProvider,
    input: CodegenInput,
    model: Option<&str>,
    tx: &Sender<CodegenDelta>,
    cancel: &AtomicBool,
    provider_cancel: Option<Arc<AtomicBool>>,
) {
    let provider_label = provider.provider_label().to_string();
    let framework = input.framework.as_wire();
    tracing::info!(
        target: "op_codegen_runtime",
        provider = %provider_label,
        model = model.unwrap_or("<provider-default>"),
        framework,
        "code generation started"
    );
    let mut pipe = CodegenPipeline::new(input);
    let mut diagnostics = Vec::<String>::new();
    let mut last_progress: Option<CodeGenProgress> = None;
    loop {
        if cancel.load(Ordering::Relaxed) {
            pipe.cancel();
        }
        let step = pipe.step();
        // Settled requests are resolved inside `step()`. Publish after that
        // transition; the former post-`on_complete` snapshot was one step
        // stale and could hide the final Failed/Done chunk states.
        if !send_progress_if_changed(&pipe, tx, cancel, &mut last_progress) {
            return;
        }
        match step {
            PipelineStep::Dispatch(reqs) => {
                for req in reqs {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let request_label = request_kind_label(&req.kind);
                    let system = codegen_system_prompt(&req.skills);
                    let chat_req = ChatRequest {
                        system_prompt: system,
                        user_message: req.user_message.clone(),
                        history: Vec::new(),
                        max_output_tokens: req.max_output_tokens,
                        thinking: req.thinking,
                        effort: req.effort,
                        attachments: Vec::new(),
                        model: model.map(str::to_string),
                    };
                    let mut errored: Option<String> = None;
                    let mut response_bytes = 0usize;
                    let mut thinking_bytes = 0usize;
                    let mut stop_reason = None;
                    let response_byte_limit = response_byte_limit(req.max_output_tokens);
                    tracing::debug!(
                        target: "op_codegen_runtime",
                        provider = %provider_label,
                        model = model.unwrap_or("<provider-default>"),
                        request = %request_label,
                        max_output_tokens = req.max_output_tokens,
                        "dispatching code generation request"
                    );
                    let stream = match provider_cancel.as_ref() {
                        Some(token) => provider.send_cancellable(chat_req, Arc::clone(token)),
                        None => provider.send(chat_req),
                    };
                    for delta in stream {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        match delta {
                            ChatDelta::TextDelta(t) => {
                                if response_bytes.saturating_add(t.len()) > response_byte_limit {
                                    errored = Some(format!(
                                        "provider response exceeded the {response_byte_limit}-byte safety limit"
                                    ));
                                    break;
                                }
                                response_bytes = response_bytes.saturating_add(t.len());
                                pipe.on_delta(req.id, &t);
                            }
                            ChatDelta::Thinking(t) => {
                                thinking_bytes = thinking_bytes.saturating_add(t.len());
                            }
                            ChatDelta::Error(e) => {
                                tracing::warn!(
                                    target: "op_codegen_runtime",
                                    provider = %provider_label,
                                    model = model.unwrap_or("<provider-default>"),
                                    request = %request_label,
                                    error = %e,
                                    "code generation provider request failed"
                                );
                                errored = Some(bounded_message(&e));
                                break;
                            }
                            ChatDelta::Done {
                                stop_reason: reason,
                            } => {
                                stop_reason = Some(reason);
                                break;
                            }
                            ChatDelta::ToolUse { name, .. } => {
                                errored = Some(format!(
                                    "provider attempted tool call {name:?} instead of returning code"
                                ));
                                break;
                            }
                        }
                    }
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    if errored.is_none() {
                        errored = stop_reason_error(stop_reason);
                    }
                    if errored.is_none() && response_bytes == 0 {
                        errored = Some(format!(
                            "provider returned no text (stop={}; thinking_bytes={thinking_bytes})",
                            stop_reason_label(stop_reason)
                        ));
                    }
                    if let Some(detail) = errored.as_deref() {
                        remember_diagnostic(
                            &mut diagnostics,
                            format!("{request_label}: {}", bounded_message(detail)),
                        );
                        tracing::warn!(
                            target: "op_codegen_runtime",
                            provider = %provider_label,
                            model = model.unwrap_or("<provider-default>"),
                            request = %request_label,
                            stop_reason = stop_reason_label(stop_reason),
                            thinking_bytes,
                            error = %detail,
                            "code generation request did not complete with usable text"
                        );
                    }
                    match errored {
                        Some(e) => pipe.on_error(req.id, e),
                        None => pipe.on_complete(req.id),
                    }
                }
            }
            PipelineStep::Waiting => {}
            PipelineStep::Done {
                code,
                degraded,
                assets,
            } => {
                tracing::info!(
                    target: "op_codegen_runtime",
                    provider = %provider_label,
                    model = model.unwrap_or("<provider-default>"),
                    framework,
                    degraded,
                    code_bytes = code.len(),
                    asset_count = assets.len(),
                    "code generation completed"
                );
                let _ = tx.send(CodegenDelta::Done {
                    code,
                    degraded,
                    assets,
                });
                return;
            }
            PipelineStep::Failed { message } => {
                let message = with_diagnostics(
                    message,
                    &provider_label,
                    model.unwrap_or("provider default"),
                    &diagnostics,
                );
                tracing::error!(
                    target: "op_codegen_runtime",
                    provider = %provider_label,
                    model = model.unwrap_or("<provider-default>"),
                    framework,
                    error = %message,
                    "code generation failed"
                );
                let _ = tx.send(CodegenDelta::Failed(message));
                return;
            }
        }
    }
}

fn send_progress_if_changed(
    pipe: &CodegenPipeline,
    tx: &Sender<CodegenDelta>,
    cancel: &AtomicBool,
    last_progress: &mut Option<CodeGenProgress>,
) -> bool {
    if cancel.load(Ordering::Relaxed) {
        return true;
    }
    let progress = pipe.progress();
    if progress == CodeGenProgress::default() && last_progress.is_none() {
        return true;
    }
    if last_progress.as_ref() == Some(&progress) {
        return true;
    }
    *last_progress = Some(progress.clone());
    tx.send(CodegenDelta::Progress(progress)).is_ok()
}

fn codegen_system_prompt(skills: &[&str]) -> String {
    let mut prompt = op_ai_skills::compose_system_prompt(skills, 0);
    if !prompt.is_empty() {
        prompt.push_str("\n\n");
    }
    prompt.push_str(
        "This is a text-only code-generation request. NEVER use tools for this request. Return the requested JSON or source code directly in the response text.",
    );
    prompt
}

fn request_kind_label(kind: &RequestKind) -> String {
    match kind {
        RequestKind::Planning => "planning".to_string(),
        RequestKind::Chunk { chunk_id } => format!("chunk {chunk_id}"),
        RequestKind::Assembly => "assembly".to_string(),
    }
}

fn stop_reason_label(reason: Option<op_ai::chat_provider::StopReason>) -> &'static str {
    use op_ai::chat_provider::StopReason;
    match reason {
        Some(StopReason::EndTurn) => "end_turn",
        Some(StopReason::Aborted) => "aborted",
        Some(StopReason::MaxTokens) => "max_tokens",
        Some(StopReason::ToolUse) => "tool_use",
        None => "stream_ended",
    }
}

fn stop_reason_error(reason: Option<op_ai::chat_provider::StopReason>) -> Option<String> {
    use op_ai::chat_provider::StopReason;
    let detail = match reason {
        Some(StopReason::EndTurn) => return None,
        Some(StopReason::Aborted) => "provider aborted the request",
        Some(StopReason::MaxTokens) => "provider reached the output-token limit",
        Some(StopReason::ToolUse) => "provider stopped for a tool call instead of returning code",
        None => "provider stream ended without a terminal stop reason",
    };
    Some(detail.to_string())
}

/// Providers are expected to honor the token cap, but an adapter bug or a
/// malformed stream must not grow the worker buffer without bound. Sixteen
/// bytes per requested token is deliberately generous for source code and
/// UTF-8, with a useful floor for small test/custom budgets and a hard ceiling.
fn response_byte_limit(max_output_tokens: u32) -> usize {
    const MIN_BYTES: usize = 64 * 1024;
    const MAX_BYTES: usize = 2 * 1024 * 1024;
    (max_output_tokens as usize)
        .saturating_mul(16)
        .clamp(MIN_BYTES, MAX_BYTES)
}

fn bounded_message(message: &str) -> String {
    const MAX_CHARS: usize = 500;
    let trimmed = message.trim();
    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed.to_string();
    }
    let mut out: String = trimmed.chars().take(MAX_CHARS).collect();
    out.push('…');
    out
}

fn remember_diagnostic(diagnostics: &mut Vec<String>, detail: String) {
    const MAX_DIAGNOSTICS: usize = 6;
    if diagnostics.iter().any(|existing| existing == &detail) {
        return;
    }
    if diagnostics.len() == MAX_DIAGNOSTICS {
        diagnostics.remove(0);
    }
    diagnostics.push(detail);
}

fn with_diagnostics(
    message: String,
    provider_label: &str,
    model_label: &str,
    diagnostics: &[String],
) -> String {
    // Put the actionable request failure first: the property panel has
    // limited width and ellipsizes this line. Provider/model identity remains
    // available at the end for logs and support diagnostics.
    let mut details = diagnostics.to_vec();
    details.push(format!(
        "provider={}; model={}",
        bounded_message(provider_label),
        bounded_message(model_label)
    ));
    format!("{message}\nDetails: {}", details.join(" | "))
}

#[cfg(test)]
#[path = "codegen_session_start_tests.rs"]
mod start_tests;
