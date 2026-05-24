//! Headless smoke runner for `op-orchestrator`.
//!
//! Drives one design turn against `AnthropicProvider` without the
//! desktop UI / `DesignSession` actor model — single-threaded
//! `block_on(Orchestrator::run)` against an inline `DocSink`, with every
//! progress event + every applied `EditorCommand` dumped to stderr.
//!
//! ## Usage
//!
//! ```sh
//! export OPENPENCIL_ANTHROPIC_API_KEY=sk-ant-...   # or ANTHROPIC_API_KEY
//! cargo run -p op-smoke -- "design a login screen"
//! ```
//!
//! Optional env overrides:
//! - `OPENPENCIL_ORCHESTRATOR_MODEL` — default `claude-sonnet-4-6`.
//!
//! ## What this verifies vs the desktop GUI smoke
//!
//! - LLM client construction (`AnthropicProvider::new` + auth).
//! - `Orchestrator::run` reaching the network (200 OK / 401 / 429 etc.
//!   surfaces as a `LlmError` in the streamed events).
//! - Planner → scaffold → subtask → cleanup transitions
//!   (`Progress::*` enum, every variant rendered to stderr).
//! - `EditorCommand` applied to the in-memory state, including
//!   `InsertSubtree` ID-remapping.
//! - Terminal `RunSummary` (subtask outcomes + total node count) or
//!   `OrchestratorError`.
//!
//! What this does NOT verify (run the desktop binary for those):
//! - Canvas rendering / paint correctness.
//! - chat panel rendering of progress lines / streaming bubble.
//! - Cross-session abort (mid-turn switch to chat — covered by
//!   `chat_session::launch_if_pending` host tests).
//! - Pre-validation fixes — smoke runs with `SkippedPreValidator` so
//!   the trace stays focused on orchestrator behaviour. The host
//!   binary uses `LintPreValidator`; smoke skips that layer.

use std::sync::Arc;

use agent::abort::AbortController;
use agent::provider::anthropic::AnthropicProvider;
use agent::provider::Provider;
use agent::query::QueryEngine;
use agent::stream::Event;
use futures::channel::mpsc;
use futures::StreamExt;
use op_editor_core::{EditorCommand, EditorState};
use op_orchestrator::{
    AbortFlag, CallRequest, DesignRequest, DocSink, LlmChunk, LlmClient, LlmError, Orchestrator,
    Progress, SkippedPreValidator, SkippedScreenshotProvider, SkippedVisionLlmClient,
    ValidationProviders,
};

/// `LlmClient` impl for the smoke runner — `AnthropicProvider` under a
/// `QueryEngine`, with every call spawned onto the current tokio runtime.
/// Mirrors `op-host-desktop::chat_orchestrator::DesktopLlmClient` but
/// uses `tokio::spawn` instead of a shared `Runtime::spawn` handle.
struct SmokeLlmClient {
    provider: Arc<dyn Provider>,
    default_model: String,
}

impl LlmClient for SmokeLlmClient {
    fn call(
        &self,
        req: CallRequest,
    ) -> futures::stream::BoxStream<'static, Result<LlmChunk, LlmError>> {
        let (tx, rx) = mpsc::unbounded::<Result<LlmChunk, LlmError>>();
        if req.abort.is_set() {
            let _ = tx.unbounded_send(Err(LlmError {
                message: "aborted".into(),
                aborted: true,
            }));
            return Box::pin(rx);
        }
        let provider = self.provider.clone();
        let model = req
            .model
            .clone()
            .unwrap_or_else(|| self.default_model.clone());
        let system = req.system_prompt.clone();
        let user = req.user_prompt.clone();

        eprintln!(
            "[LLM] call: model={model} system_len={} user_len={}",
            system.len(),
            user.len()
        );

        tokio::spawn(async move {
            let engine = QueryEngine::new(provider, model).with_system(system);
            let abort = AbortController::new();
            let stream = match engine.run(user, abort).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[LLM] engine.run error: {e}");
                    let _ = tx.unbounded_send(Err(LlmError {
                        message: e.to_string(),
                        aborted: false,
                    }));
                    return;
                }
            };
            let mut stream = stream;
            while let Some(item) = stream.next().await {
                let sent = match item {
                    Ok(Event::TextDelta { delta }) => tx.unbounded_send(Ok(LlmChunk::Text(delta))),
                    Ok(Event::Thinking { delta }) => {
                        tx.unbounded_send(Ok(LlmChunk::Thinking(delta)))
                    }
                    Ok(Event::Result { .. }) => break,
                    Ok(Event::Error { code, message }) => {
                        eprintln!("[LLM] event error: {code}: {message}");
                        tx.unbounded_send(Err(LlmError {
                            message: format!("{code}: {message}"),
                            aborted: false,
                        }))
                    }
                    Ok(_) => Ok(()),
                    Err(e) => {
                        eprintln!("[LLM] stream error: {e}");
                        tx.unbounded_send(Err(LlmError {
                            message: e.to_string(),
                            aborted: false,
                        }))
                    }
                };
                if sent.is_err() {
                    break;
                }
            }
        });

        Box::pin(rx)
    }
}

/// Inline `DocSink` — owns the canonical state directly, no channel hop.
/// Every `apply` echoes the command kind + result so the smoke trace
/// shows the orchestrator's mutations linearly.
struct InlineDocSink {
    state: EditorState,
}

impl DocSink for InlineDocSink {
    fn state(&self) -> &EditorState {
        &self.state
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        let label = describe_cmd(&cmd);
        let applied = self.state.apply(cmd);
        eprintln!("[CMD] {label} → applied={applied}");
        applied
    }

    fn begin_undo_batch(&mut self) {
        eprintln!("[UNDO] begin");
    }

    fn end_undo_batch(&mut self) {
        eprintln!("[UNDO] end");
    }
}

/// One-line label for an `EditorCommand` variant. We don't dump the full
/// payload (often kilobytes of node JSON) — just the variant + its key
/// identifying field so the trace stays readable.
fn describe_cmd(cmd: &EditorCommand) -> String {
    match cmd {
        EditorCommand::InsertSubtree { nodes, parent_id } => {
            format!("InsertSubtree(parent={parent_id:?}, nodes={})", nodes.len())
        }
        EditorCommand::UpdateNode { node_id, .. } => format!("UpdateNode({node_id:?})"),
        EditorCommand::DeleteNode { node_id } => format!("DeleteNode({node_id:?})"),
        EditorCommand::MoveNode { node_id, .. } => format!("MoveNode({node_id:?})"),
        EditorCommand::SetNodeLayoutProp {
            node_id, property, ..
        } => format!("SetNodeLayoutProp({node_id:?}, prop={property:?})"),
        EditorCommand::SetNodeStrokeHex { node_id, hex } => {
            format!("SetNodeStrokeHex({node_id:?}, {hex})")
        }
        EditorCommand::SetNodeStrokeWidth { node_id, .. } => {
            format!("SetNodeStrokeWidth({node_id:?})")
        }
        EditorCommand::SetNodeFillHex { node_id, hex } => {
            format!("SetNodeFillHex({node_id:?}, {hex})")
        }
        EditorCommand::RemoveNodeEffect { node_id, index } => {
            format!("RemoveNodeEffect({node_id:?}, [{index}])")
        }
        other => {
            let dbg = format!("{other:?}");
            // Truncate the Debug output so massive payloads don't blow up the trace.
            if dbg.len() > 120 {
                format!("{}...", &dbg[..117])
            } else {
                dbg
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::process::ExitCode {
    let prompt = match std::env::args().nth(1) {
        Some(p) if !p.is_empty() => p,
        _ => {
            eprintln!(
                "usage: op-smoke <prompt>\n\nexport OPENPENCIL_ANTHROPIC_API_KEY=... (or ANTHROPIC_API_KEY)"
            );
            return std::process::ExitCode::from(2);
        }
    };

    let api_key = std::env::var("OPENPENCIL_ANTHROPIC_API_KEY")
        .ok()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .filter(|k| !k.is_empty());
    let Some(api_key) = api_key else {
        eprintln!("error: neither OPENPENCIL_ANTHROPIC_API_KEY nor ANTHROPIC_API_KEY is set");
        return std::process::ExitCode::from(3);
    };

    let model = std::env::var("OPENPENCIL_ORCHESTRATOR_MODEL")
        .unwrap_or_else(|_| "claude-sonnet-4-6".into());

    eprintln!("[SMOKE] model={model}");
    eprintln!("[SMOKE] prompt={prompt:?}");

    let provider: Arc<dyn Provider> = Arc::new(AnthropicProvider::new(api_key));
    let llm = SmokeLlmClient {
        provider,
        default_model: model.clone(),
    };

    let mut sink = InlineDocSink {
        state: EditorState::new(),
    };
    let request = DesignRequest {
        prompt,
        model: Some(model),
        provider: None,
        design_md: sink.state.doc.design_md.clone(),
        append_context: None,
        concurrency: 1,
        validation_enabled: false,
        visual_ref_enabled: false,
    };
    let abort = AbortFlag::new();
    // Skip pre-validation in the smoke trace — keeps the orchestrator
    // signal clean. The desktop binary swaps this for `LintPreValidator`.
    let pre_validator = SkippedPreValidator;
    let screenshot = SkippedScreenshotProvider;
    let vision = SkippedVisionLlmClient;
    let providers = ValidationProviders {
        pre_validator: &pre_validator,
        screenshot: &screenshot,
        vision: &vision,
        system_prompt: String::new(),
    };

    let mut on_progress = |p: Progress| {
        eprintln!("[PROGRESS] {p:?}");
    };

    let started = std::time::Instant::now();
    let result = Orchestrator::new()
        .run(
            request,
            &mut sink,
            &llm,
            &mut on_progress,
            &abort,
            &providers,
        )
        .await;
    let elapsed = started.elapsed();

    match result {
        Ok(summary) => {
            eprintln!("[FINAL] Ok in {elapsed:?}");
            eprintln!("  root_frame_id = {:?}", summary.root_frame_id);
            eprintln!("  total_nodes   = {}", summary.total_nodes);
            eprintln!("  subtasks      = {}", summary.subtasks.len());
            for s in &summary.subtasks {
                eprintln!(
                    "    - {}: {} node(s){}",
                    s.id,
                    s.node_count,
                    s.error
                        .as_deref()
                        .map(|e| format!(" [error: {e}]"))
                        .unwrap_or_default()
                );
            }
            std::process::ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("[FINAL] Err in {elapsed:?}: {e}");
            std::process::ExitCode::from(1)
        }
    }
}
