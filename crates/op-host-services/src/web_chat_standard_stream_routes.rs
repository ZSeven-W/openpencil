//! The three `DesignIntent` dispatch targets of the standard turn: plain
//! chat streaming, modify-route application, and the orchestrator-backed
//! new-design pipeline.
//!
//! Split out of the `web_chat_standard` spine (800-line file ceiling); the
//! spine re-imports them, so call sites are unchanged.

use std::io::Write;
use std::sync::{Arc, Mutex};

use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest};
use op_editor_core::EditorState;
use op_orchestrator::{
    AbortFlag, Orchestrator, Progress, SkippedScreenshotProvider, SkippedVisionLlmClient,
    ValidationProviders,
};

use super::events::{
    progress_label, web_identity_seed, write_agent_identity_event, write_delta_event,
    write_done_event, write_error_event, write_thinking_event,
};
use super::quality;
use super::reference;
use super::sink::WebDesignDocSink;
use super::{admit_document_write, design_request_for, CanvasWriteTarget, WebStandardTurnRequest};
use crate::chat_provider_llm::ChatProviderLlmClient;
use crate::pre_validator::LintPreValidator;
use crate::web_canvas_server::{SseHub, WebCanvasState};

const STANDARD_MODIFY_STEP: &str =
    r#"<step title="Checking guidelines">Analyzing modification request...</step>"#;

pub(super) fn stream_chat_route<W: Write>(
    out: &mut W,
    req: &WebStandardTurnRequest,
    state: &EditorState,
    provider: &dyn ChatProvider,
    model: Option<String>,
) -> std::io::Result<()> {
    let chat_req = ChatRequest {
        system_prompt: crate::chat_system_prompt::build_chat_system_prompt(state, &req.ai.user),
        user_message: req.ai.user.clone(),
        history: req.history.clone(),
        max_output_tokens: req.ai.max_output_tokens,
        thinking: req.ai.thinking,
        effort: req.ai.effort,
        attachments: req.attachments.clone(),
        model,
    };
    for delta in provider.send(chat_req) {
        out.write_all(crate::ai_proxy::delta_to_sse(&delta).as_bytes())?;
        out.flush()?;
        if matches!(delta, ChatDelta::Done { .. } | ChatDelta::Error(_)) {
            break;
        }
    }
    Ok(())
}

pub(super) fn stream_modify_route<W: Write>(
    out: &mut W,
    plan: crate::chat_intent::ModifyPlan,
    provider: &dyn ChatProvider,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
    write_barrier: Option<&crate::web_canvas_server::WriteBarrier>,
) -> std::io::Result<()> {
    write_delta_event(out, STANDARD_MODIFY_STEP)?;
    let target_frame_ids = plan.target_frame_ids;
    let request = ChatRequest {
        system_prompt: plan.system_prompt,
        user_message: plan.user_message,
        max_output_tokens: 8192,
        ..Default::default()
    };
    let mut full_response = String::new();
    let mut stream_error: Option<String> = None;
    for delta in provider.send(request) {
        match delta {
            ChatDelta::TextDelta(s) => full_response.push_str(&s),
            ChatDelta::Thinking(_) | ChatDelta::ToolUse { .. } => {}
            ChatDelta::Error(msg) => {
                stream_error = Some(msg);
                break;
            }
            ChatDelta::Done { .. } => break,
        }
    }

    let nodes = crate::chat_intent::parse_modify_nodes(&full_response);
    if !nodes.is_empty() {
        write_delta_event(out, &format!("\n{full_response}"))?;
        let (applied, tick) = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            // `apply_design_modification` writes a batch straight into the
            // editor, so the gate runs before it rather than per command.
            if guard
                .gate_daemon_mutation(
                    op_editor_core::CollabGateAction::Document(
                        op_editor_core::CollabDocumentMutation::NodePropertyBatch,
                    ),
                    op_editor_core::CollabEditSource::Ai,
                )
                .is_err()
            {
                (0, None)
            } else {
                // Shutting down: the reply still streams, the document is left
                // exactly as the flush will find it.
                let admitted = admit_document_write(write_barrier).ok();
                let (count, mutated) = if admitted.is_none() {
                    (0, false)
                } else {
                    crate::chat_canvas_tools::apply_design_modification(
                        &mut guard.editor,
                        &nodes,
                        &target_frame_ids,
                    )
                };
                let tick = if mutated {
                    guard.version += 1;
                    Some(guard.sse_tick())
                } else {
                    None
                };
                (count, tick)
            }
        };
        if let Some(tick) = tick {
            hub.broadcast(tick);
        }
        if applied > 0 {
            write_delta_event(out, "\n\n<!-- APPLIED -->")?;
        }
        return write_done_event(out);
    }

    let message = if let Some(err) = stream_error {
        err
    } else {
        let trimmed = full_response.trim();
        let hint = if trimmed.is_empty() {
            "The model returned an empty response.".to_string()
        } else {
            let preview: String = trimmed.chars().take(150).collect();
            let ellipsis = if full_response.chars().count() > 150 {
                "…"
            } else {
                ""
            };
            format!("Model output: \"{preview}{ellipsis}\"")
        };
        format!("Could not parse design nodes from model response. {hint}")
    };
    write_error_event(out, &message)
}

pub(super) fn stream_new_design_route<W: Write>(
    out: &mut W,
    req: WebStandardTurnRequest,
    snapshot: EditorState,
    provider: Box<dyn ChatProvider>,
    model: Option<String>,
    target: CanvasWriteTarget<'_>,
) -> std::io::Result<()> {
    let prompt = req.ai.user.clone();
    let mut request = design_request_for(&req, &snapshot, model.clone());
    let provider_arc: Arc<dyn ChatProvider> = Arc::from(provider);
    let llm = ChatProviderLlmClient::new(provider_arc.clone()).with_model(model.clone());
    let mut sink = WebDesignDocSink::new(target.state, target.hub, target.write_barrier, snapshot);
    let abort = AbortFlag::new();
    let image_model = request.model.clone();
    let image_provider = request.provider.clone();
    let reference_result = crate::chat_runtime::block_on_anywhere(
        crate::reference_context::resolve_reference_context(
            &llm,
            &prompt,
            request.model.clone(),
            request.provider.clone(),
            &abort,
        ),
    );
    match reference_result {
        Ok(Some(context)) => reference::apply_reference_context(&mut request, &mut sink, context),
        Ok(None) => reference::apply_image_reference(
            &mut request,
            &mut sink,
            &mut *out,
            reference::resolve_image_reference(
                &req.attachments,
                &provider_arc,
                &prompt,
                image_model.clone(),
                image_provider.clone(),
            ),
        )?,
        Err(error) => {
            let notice = format!("reference page could not be used: {error}");
            eprintln!("[web-chat] {notice}");
            write_thinking_event(out, &format!("\n{notice}"))?;
            reference::apply_image_reference(
                &mut request,
                &mut sink,
                &mut *out,
                reference::resolve_image_reference(
                    &req.attachments,
                    &provider_arc,
                    &prompt,
                    image_model,
                    image_provider,
                ),
            )?;
        }
    }
    let pre_validator = LintPreValidator;

    let use_real_vision = crate::validation_providers::vision_validation_enabled();
    let stub_screenshot = SkippedScreenshotProvider;
    let stub_vision = SkippedVisionLlmClient;
    let real_screenshot = crate::validation_providers::RealScreenshotProvider;
    let real_vision = crate::validation_providers::ChatVisionLlmClient::new(provider_arc.clone())
        .with_model(model.clone());
    let (screenshot, vision, system_prompt): (
        &dyn op_orchestrator::ScreenshotProvider,
        &dyn op_orchestrator::VisionLlmClient,
        String,
    ) = if use_real_vision {
        (
            &real_screenshot,
            &real_vision,
            crate::validation_providers::validation_system_prompt(),
        )
    } else {
        (&stub_screenshot, &stub_vision, String::new())
    };
    let providers = ValidationProviders {
        pre_validator: &pre_validator,
        screenshot,
        vision,
        system_prompt,
    };
    let identity =
        op_orchestrator::agent_identity::assign_agent_identities_seeded(1, web_identity_seed())
            .into_iter()
            .next()
            .expect("one requested agent identity");
    write_agent_identity_event(out, &identity)?;
    let epoch = op_editor_core::agent_indicators::begin();
    op_editor_core::agent_indicators::confirm_cursor_agent(epoch, &identity.color, &identity.name);
    let mut folded_quality = None;
    let quality_brief = request.prompt.clone();
    let summary = {
        let out_ref = &mut *out;
        let quality_ref = &mut folded_quality;
        let mut on_progress = move |p: Progress| {
            crate::run_quality::fold_quality_event(quality_ref, &p);
            let _ = write_thinking_event(out_ref, &format!("\n{}", progress_label(&p)));
        };
        crate::chat_runtime::block_on_anywhere(Orchestrator::new().with_indicator_epoch(epoch).run(
            request,
            &mut sink,
            &llm,
            &mut on_progress,
            &abort,
            &providers,
        ))
    };
    if abort.is_set() {
        op_editor_core::agent_indicators::end_if_epoch(epoch);
    } else {
        op_editor_core::agent_indicators::finish_if_epoch(epoch);
    }
    match summary {
        Ok(summary) => {
            let ok = summary
                .subtasks
                .iter()
                .filter(|o| o.error.is_none())
                .count();
            let failed = summary.subtasks.len() - ok;
            if let Some(report) =
                quality::finished_run_report(folded_quality, target.state, &summary, &quality_brief)
            {
                quality::write_quality_report_event(out, &report)?;
            }
            write_delta_event(
                out,
                &format!(
                    "\n\nDone — {} subtask(s) succeeded, {} failed, {} node(s) total.",
                    ok, failed, summary.total_nodes
                ),
            )?;
            write_done_event(out)
        }
        Err(e) => write_error_event(out, &e.to_string()),
    }
}
