//! Shared code-generation session worker.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
use op_codegen::ai::types::{AssetFile, CodegenInput, PipelineStep};
use op_codegen::ai::CodegenPipeline;
use op_editor_core::codegen::CodeGenProgress;

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

/// An in-flight generation. Host pumps own the UI-specific folding of deltas.
pub struct CodegenSession {
    pub rx: Receiver<CodegenDelta>,
    pub finished: bool,
    pub framework: op_editor_core::codegen::Framework,
    pub cancel: Arc<AtomicBool>,
    pub run_epoch: u64,
}

impl CodegenSession {
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
        framework: op_editor_core::codegen::Framework,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel);
        std::thread::Builder::new()
            .name("op-codegen-turn".into())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_pipeline(provider.as_ref(), input, &tx, &worker_cancel);
                }));
                if outcome.is_err() {
                    let _ = tx.send(CodegenDelta::Failed(
                        "Code generation failed unexpectedly".into(),
                    ));
                }
            })
            .expect("spawn op-codegen-turn worker");
        CodegenSession {
            rx,
            finished: false,
            framework,
            cancel,
            run_epoch: NEXT_RUN_EPOCH.fetch_add(1, Ordering::Relaxed),
        }
    }
}

/// The completed result kept host-side for Download.
#[derive(Default, Clone)]
pub struct CodegenResult {
    pub code: String,
    pub framework_ext: String,
    pub assets: Vec<AssetFile>,
}

/// Drive the pipeline to completion against `provider`, emitting deltas on
/// `tx`. Runs on the worker thread or synchronously in tests.
pub fn run_pipeline(
    provider: &dyn ChatProvider,
    input: CodegenInput,
    tx: &Sender<CodegenDelta>,
    cancel: &AtomicBool,
) {
    let mut pipe = CodegenPipeline::new(input);
    loop {
        if cancel.load(Ordering::Relaxed) {
            pipe.cancel();
        }
        match pipe.step() {
            PipelineStep::Dispatch(reqs) => {
                for req in reqs {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let system = op_ai_skills::compose_system_prompt(&req.skills, 0);
                    let chat_req = ChatRequest {
                        system_prompt: system,
                        user_message: req.user_message.clone(),
                        history: Vec::new(),
                        max_output_tokens: req.max_output_tokens,
                        thinking: req.thinking,
                        effort: req.effort,
                        attachments: Vec::new(),
                        model: None,
                    };
                    let mut errored: Option<String> = None;
                    for delta in provider.send(chat_req) {
                        if cancel.load(Ordering::Relaxed) {
                            break;
                        }
                        match delta {
                            ChatDelta::TextDelta(t) => pipe.on_delta(req.id, &t),
                            ChatDelta::Error(e) => {
                                errored = Some(e);
                                break;
                            }
                            ChatDelta::Done { .. } => break,
                            _ => {}
                        }
                    }
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    match errored {
                        Some(e) => pipe.on_error(req.id, e),
                        None => pipe.on_complete(req.id),
                    }
                    if tx.send(CodegenDelta::Progress(pipe.progress())).is_err() {
                        return;
                    }
                }
            }
            PipelineStep::Waiting => {}
            PipelineStep::Done {
                code,
                degraded,
                assets,
            } => {
                let _ = tx.send(CodegenDelta::Done {
                    code,
                    degraded,
                    assets,
                });
                return;
            }
            PipelineStep::Failed { message } => {
                let _ = tx.send(CodegenDelta::Failed(message));
                return;
            }
        }
    }
}
