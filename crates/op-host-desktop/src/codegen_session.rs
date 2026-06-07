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

use std::sync::mpsc::{Receiver, Sender};

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
use op_codegen::ai::types::{AssetFile, CodegenInput, PipelineStep};
use op_codegen::ai::CodegenPipeline;
use op_editor_core::codegen::CodeGenProgress;
use op_host_native::WidgetHostNative;

use crate::chat_session::provider_for_selected_model;

/// File extension for the active framework's generated component file.
/// Mirrors the TS download naming (`component.<ext>`).
fn framework_ext(fw: op_editor_core::codegen::Framework) -> &'static str {
    use op_editor_core::codegen::Framework;
    match fw {
        Framework::React | Framework::ReactNative => "tsx",
        Framework::Vue => "vue",
        Framework::Svelte => "svelte",
        Framework::Html => "html",
        Framework::Flutter => "dart",
        Framework::SwiftUi => "swift",
        Framework::Compose => "kt",
    }
}

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

/// An in-flight generation. The UI pump drains `rx` each frame.
pub struct CodegenSession {
    pub(crate) rx: Receiver<CodegenDelta>,
    pub(crate) finished: bool,
    /// Raw (pre-asset-sanitization) selected-nodes JSON captured at launch —
    /// carried here so the terminal `Done` can build the COMPLETE result
    /// (the pipeline doesn't echo the raw selection back).
    pub(crate) raw: String,
    /// Target framework captured at launch, for the file extension.
    pub(crate) framework: op_editor_core::codegen::Framework,
}

/// The completed result kept HOST-SIDE for Download / Export Bundle — asset
/// bytes are not carried in the wasm-clean `editor_state`.
#[derive(Default, Clone)]
pub struct CodegenResult {
    pub code: String,
    /// File extension for the active framework (e.g. "tsx", "vue", "html").
    pub framework_ext: String,
    pub assets: Vec<AssetFile>,
    /// Raw (pre-asset-sanitization) selected-nodes JSON, for the bundle.
    pub raw_nodes_json: String,
    /// Sanitized selected-nodes JSON (asset data-URLs replaced), for the bundle.
    pub sanitized_nodes_json: String,
}

/// Drive the pipeline to completion against `provider`, emitting deltas on
/// `tx`. Runs on the worker thread (or synchronously in tests). Each model
/// request is run sequentially; the streamed text deltas are fed back into
/// the pipeline so the next `step()` can advance.
pub(crate) fn run_pipeline(
    provider: &dyn ChatProvider,
    input: CodegenInput,
    tx: &Sender<CodegenDelta>,
) {
    let mut pipe = CodegenPipeline::new(input);
    loop {
        match pipe.step() {
            PipelineStep::Dispatch(reqs) => {
                for req in reqs {
                    // Expand the pipeline's skill NAMES into the final system
                    // prompt (budget 0 = no truncation cap).
                    let system = op_ai_skills::compose_system_prompt(&req.skills, 0);
                    let chat_req = ChatRequest {
                        system_prompt: system,
                        user_message: req.user_message.clone(),
                        max_output_tokens: req.max_output_tokens,
                        thinking: req.thinking,
                        effort: req.effort,
                        attachments: Vec::new(),
                    };
                    // Drain the blocking provider iterator; the first `Error`
                    // ends this request and is reported back to the pipeline.
                    let mut errored: Option<String> = None;
                    for delta in provider.send(chat_req) {
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
                    match errored {
                        Some(e) => pipe.on_error(req.id, e),
                        None => pipe.on_complete(req.id),
                    }
                    // Stream progress after each request so the panel advances.
                    if tx.send(CodegenDelta::Progress(pipe.progress())).is_err() {
                        return; // panel went away — stop early
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

impl CodegenSession {
    /// Spawn a worker that drives the pipeline against `provider`. Returns
    /// immediately — the model turns run off the UI thread. `raw` (the raw
    /// selected-nodes JSON) and `framework` are stashed on the session so the
    /// terminal `Done` can assemble the COMPLETE `CodegenResult`.
    pub fn start(
        provider: Box<dyn ChatProvider>,
        input: CodegenInput,
        raw: String,
        framework: op_editor_core::codegen::Framework,
    ) -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("op-codegen-turn".into())
            .spawn(move || {
                // Guard against a panic in the pipeline / provider so the UI
                // receives a terminal error instead of hanging in `Generating`
                // (the receiver would otherwise only observe a channel
                // disconnect — see pump's Disconnected branch).
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run_pipeline(provider.as_ref(), input, &tx);
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
            raw,
            framework,
        }
    }
}

/// Pump the in-flight generation's deltas into `editor_state.codegen`.
/// Clears `current` once the turn finishes and parks the completed result
/// (asset bytes) in `last_result`. Returns true when state changed so the
/// caller can dirty the redraw.
pub fn pump(
    host: &mut WidgetHostNative,
    current: &mut Option<CodegenSession>,
    last_result: &mut Option<CodegenResult>,
) -> bool {
    let Some(session) = current.as_mut() else {
        return false;
    };
    let mut changed = false;
    loop {
        match session.rx.try_recv() {
            Ok(CodegenDelta::Progress(p)) => {
                let cg = &mut host.editor_state_mut().codegen;
                cg.progress = p;
                cg.phase = op_editor_core::codegen::CodegenPhase::Generating;
                changed = true;
            }
            Ok(CodegenDelta::Done {
                code,
                degraded,
                assets,
            }) => {
                let metas = assets
                    .iter()
                    .map(|a| op_editor_core::codegen::AssetMeta {
                        relative_path: a.relative_path.clone(),
                        byte_len: a.bytes.len(),
                    })
                    .collect();
                {
                    let cg = &mut host.editor_state_mut().codegen;
                    cg.code = code.clone();
                    cg.degraded = degraded;
                    cg.assets = metas;
                    cg.phase = op_editor_core::codegen::CodegenPhase::Complete;
                    cg.pending_generate = false;
                    cg.pending_regenerate = false;
                }
                // `extract_codegen_assets` returns (sanitized_json, assets);
                // we only need the sanitized JSON here — the Done assets
                // already carry the extracted bytes.
                let sanitized_nodes_json =
                    op_codegen::ai::assets::extract_codegen_assets(&session.raw).0;
                *last_result = Some(CodegenResult {
                    code,
                    framework_ext: framework_ext(session.framework).into(),
                    assets,
                    raw_nodes_json: session.raw.clone(),
                    sanitized_nodes_json,
                });
                session.finished = true;
                changed = true;
            }
            Ok(CodegenDelta::Failed(e)) => {
                let cg = &mut host.editor_state_mut().codegen;
                cg.error = Some(e);
                cg.phase = op_editor_core::codegen::CodegenPhase::Error;
                cg.pending_generate = false;
                cg.pending_regenerate = false;
                session.finished = true;
                changed = true;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => break,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                // The worker dropped its sender without a terminal Done/Failed
                // (e.g. an unexpected early exit). Surface an error rather than
                // leaving the UI stuck in `Generating` with no live session.
                if !session.finished {
                    let cg = &mut host.editor_state_mut().codegen;
                    cg.error = Some("Code generation ended unexpectedly".into());
                    cg.phase = op_editor_core::codegen::CodegenPhase::Error;
                    cg.pending_generate = false;
                    cg.pending_regenerate = false;
                    changed = true;
                }
                session.finished = true;
                break;
            }
        }
    }
    if changed {
        host.mark_editor_state_dirty();
    }
    if session.finished {
        *current = None;
    }
    changed
}

/// Drain a Generate / Regenerate request raised by the Code panel and launch
/// a worker turn. Clears the pending flags first, then resolves the input +
/// provider; a missing selection or unconfigured model surfaces an inline
/// error instead of starting a turn. Returns true when state changed (a turn
/// launched OR an error was written).
pub fn launch_codegen_if_pending(
    host: &mut WidgetHostNative,
    current: &mut Option<CodegenSession>,
) -> bool {
    if current.is_some() {
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
    let Some((input, raw)) = crate::codegen_input::build_codegen_input(host.editor_state()) else {
        let cg = &mut host.editor_state_mut().codegen;
        cg.error = Some("Select nodes to generate code".into());
        cg.phase = op_editor_core::codegen::CodegenPhase::Error;
        return true;
    };
    // Capture the target framework BEFORE `input` is moved into the worker.
    let framework = host.editor_state().codegen.framework;
    let Some(provider) = provider_for_selected_model(host) else {
        let cg = &mut host.editor_state_mut().codegen;
        cg.error = Some("No model configured".into());
        cg.phase = op_editor_core::codegen::CodegenPhase::Error;
        return true;
    };
    {
        let cg = &mut host.editor_state_mut().codegen;
        cg.progress = Default::default();
        cg.error = None;
        cg.phase = op_editor_core::codegen::CodegenPhase::Generating;
    }
    *current = Some(CodegenSession::start(provider, input, raw, framework));
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::chat_provider::StopReason;
    use op_editor_core::codegen::Framework;

    /// Test-only provider that replays a DIFFERENT scripted turn each time
    /// `send` is called — `EchoProvider` replays the same script, which can't
    /// satisfy the pipeline's three distinct phases (planning → chunk →
    /// assembly). Interior mutability lets it pop one script per request.
    struct ScriptedProvider {
        scripts: std::sync::Mutex<std::collections::VecDeque<Vec<ChatDelta>>>,
    }

    impl ChatProvider for ScriptedProvider {
        fn provider_label(&self) -> &str {
            "scripted"
        }
        fn send(&self, _r: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            let next = self.scripts.lock().unwrap().pop_front().unwrap_or_default();
            Box::new(next.into_iter())
        }
    }

    #[test]
    fn framework_ext_maps_every_framework() {
        assert_eq!(framework_ext(Framework::React), "tsx");
        assert_eq!(framework_ext(Framework::ReactNative), "tsx");
        assert_eq!(framework_ext(Framework::Vue), "vue");
        assert_eq!(framework_ext(Framework::Svelte), "svelte");
        assert_eq!(framework_ext(Framework::Html), "html");
        assert_eq!(framework_ext(Framework::Flutter), "dart");
        assert_eq!(framework_ext(Framework::SwiftUi), "swift");
        assert_eq!(framework_ext(Framework::Compose), "kt");
    }

    fn turn(text: &str) -> Vec<ChatDelta> {
        vec![
            ChatDelta::TextDelta(text.into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ]
    }

    #[test]
    fn run_pipeline_drives_three_phases_to_done() {
        // Phase 1: planning JSON (one chunk targeting node `n1`).
        let plan = r#"{"chunks":[{"id":"c1","name":"Root","nodeIds":["n1"],"role":"r","suggestedComponentName":"Root","dependencies":[]}],"sharedStyles":[],"rootLayout":{"direction":"column","gap":0,"responsive":false}}"#;
        // Phase 2: chunk code + contract.
        let chunk =
            "export default function Root(){}\n---CONTRACT---\n{\"componentName\":\"Root\"}";
        // Phase 3: assembly references the chunk component.
        let assembly = "export default function App(){ return <Root/> }";

        let provider = ScriptedProvider {
            scripts: std::sync::Mutex::new(std::collections::VecDeque::from(vec![
                turn(plan),
                turn(chunk),
                turn(assembly),
            ])),
        };

        let input = CodegenInput {
            nodes_json: "[{\"type\":\"frame\",\"id\":\"n1\",\"children\":[]}]".to_string(),
            framework: Framework::React,
            variables_json: None,
            max_output_tokens: 4096,
            thinking: op_ai::chat_provider::ThinkingMode::Adaptive,
            effort: op_ai::chat_provider::EffortLevel::Low,
        };

        let (tx, rx) = std::sync::mpsc::channel();
        run_pipeline(&provider, input, &tx);
        drop(tx);

        let deltas: Vec<CodegenDelta> = rx.into_iter().collect();
        assert!(!deltas.is_empty(), "pipeline must emit at least one delta");
        match deltas.last().expect("a terminal delta") {
            CodegenDelta::Done { code, .. } => {
                // The assembled output wires in the assembly turn's App shell.
                assert!(
                    code.contains("App"),
                    "final code should contain the assembled App component, got: {code}"
                );
            }
            CodegenDelta::Failed(message) => {
                panic!("pipeline failed instead of completing: {message}");
            }
            CodegenDelta::Progress(_) => {
                panic!("last delta should be terminal Done, not Progress");
            }
        }
    }
}
