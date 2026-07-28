//! Standard chat/design turn for the Rust web shell.
//!
//! The browser owns the immediate UI. This endpoint accepts an optional
//! request-scoped built-in credential and mirrors the desktop "standard mode"
//! route on the daemon side: classify the user's turn, then dispatch to plain
//! chat, design modification, or the orchestrator-backed new-design pipeline.
//! Host CLI and ACP providers are intentionally unavailable on the web route.

use std::io::Write;
use std::sync::{Arc, Mutex};

use base64::Engine as _;
#[cfg(test)]
use op_ai::chat_provider::StopReason;
use op_ai::chat_provider::{ChatAttachment, ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest};
use op_editor_core::chat::MAX_ATTACHMENT_BYTES;
use op_editor_core::{BuiltinAgentConfig, EditorCommand, EditorState, NodeId};
use op_orchestrator::{
    AbortFlag, DesignRequest, DocSink, Orchestrator, Progress, SkippedScreenshotProvider,
    SkippedVisionLlmClient, ValidationProviders,
};
use serde_json::Value;

use crate::ai_proxy::AiStreamRequest;
use crate::chat_provider_llm::ChatProviderLlmClient;
use crate::pre_validator::LintPreValidator;
use crate::web_canvas_server::{SseHub, WebCanvasState};

#[path = "web_chat_standard_error.rs"]
mod error;
use error::WebChatStandardError;

#[path = "web_chat_standard_events.rs"]
mod events;
use events::{
    progress_label, web_identity_seed, write_agent_identity_event, write_delta_event,
    write_done_event, write_error_event, write_thinking_event,
};

const STANDARD_MODIFY_STEP: &str =
    r#"<step title="Checking guidelines">Analyzing modification request...</step>"#;

pub struct WebStandardTurnRequest {
    pub ai: AiStreamRequest,
    document_json: Option<String>,
    editor_meta: Option<op_pen_loader::EditorMeta>,
    selected_ids: Vec<String>,
    active_page_id: Option<String>,
    agent_team_size: Option<u32>,
    history: Vec<(ChatHistoryRole, String)>,
    attachments: Vec<ChatAttachment>,
    transient_builtin: Option<BuiltinAgentConfig>,
}

pub fn parse_standard_turn_body(body: &str) -> Option<WebStandardTurnRequest> {
    let ai = crate::ai_proxy::parse_ai_stream_body(body)?;
    let value: Value = serde_json::from_str(body).ok()?;
    let obj = value.as_object()?;
    let document_json = obj.get("document").map(Value::to_string);
    let editor_meta = obj
        .get("editorMeta")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok());
    let selected_ids = obj
        .get("selectedIds")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let active_page_id = obj
        .get("activePageId")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let agent_team_size = obj
        .get("agent_team_size")
        .and_then(Value::as_u64)
        .map(|n| (n as u32).clamp(1, 6));
    let history = parse_chat_history(obj.get("history"));
    let attachments = parse_chat_attachments(obj.get("attachments"));
    let transient_builtin = match obj.get("credential") {
        None | Some(Value::Null) => None,
        Some(value) => Some(crate::web_credentials::parse_transient_builtin(value)?),
    };
    Some(WebStandardTurnRequest {
        ai,
        document_json,
        editor_meta,
        selected_ids,
        active_page_id,
        agent_team_size,
        history,
        attachments,
        transient_builtin,
    })
}

fn parse_chat_history(value: Option<&Value>) -> Vec<(ChatHistoryRole, String)> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let obj = entry.as_object()?;
            let role = match obj.get("role").and_then(Value::as_str) {
                Some("user") => ChatHistoryRole::User,
                Some("assistant") => ChatHistoryRole::Assistant,
                _ => return None,
            };
            let content = obj.get("content").and_then(Value::as_str)?.to_string();
            if content.trim().is_empty() {
                return None;
            }
            Some((role, content))
        })
        .collect()
}

fn parse_chat_attachments(value: Option<&Value>) -> Vec<ChatAttachment> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let obj = entry.as_object()?;
            let name = obj
                .get("name")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())?
                .to_string();
            let media_type = obj
                .get("media_type")
                .or_else(|| obj.get("mediaType"))
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())?
                .to_string();
            let encoded = obj
                .get("data_base64")
                .or_else(|| obj.get("dataBase64"))
                .and_then(Value::as_str)?;
            let data = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()?;
            if data.len() > MAX_ATTACHMENT_BYTES {
                return None;
            }
            Some(ChatAttachment {
                name,
                media_type,
                data,
            })
        })
        .collect()
}

pub fn stream_standard_turn<W: Write>(
    out: &mut W,
    req: WebStandardTurnRequest,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
    cors_origin: Option<&str>,
) -> std::io::Result<()> {
    crate::ai_proxy::write_sse_headers(out, cors_origin)?;

    let mut snapshot = match apply_request_snapshot(&req, state, hub) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            // `write_error_event` feeds `op-ai`'s `ChatDelta::Error(String)`
            // SSE frame; render the typed failure at that boundary only.
            return write_error_event(out, &error.to_string());
        }
    };

    let model = selected_model_id(&req.ai);
    if matches!(
        op_orchestrator::classify_intent(&req.ai.user),
        op_orchestrator::Intent::Design
    ) && clear_fresh_starter_frame_for_design(&mut snapshot)
    {
        let version = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(version) = clear_live_starter_frame_for_design(&mut guard) {
                snapshot = guard.editor.clone();
                Some(version)
            } else {
                None
            }
        };
        if let Some(version) = version {
            hub.broadcast(version);
        }
    }
    inject_transient_builtin(&mut snapshot, req.transient_builtin.as_ref());

    let credential_persistence = state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .credential_persistence;
    let providers = (|| -> Result<_, WebChatStandardError> {
        let resolve = |chat_session| {
            crate::ai_proxy::proxy_provider_for_request_with_chat_session(
                &snapshot,
                &req.ai,
                chat_session,
                credential_persistence,
            )
            // `ProxyProviderError` is transparent, so the sentence the SSE
            // `error` event carries is unchanged; this variant is flat
            // because the resolve step reports one sentence to the browser.
            .map_err(|error| WebChatStandardError::ProviderResolve(error.to_string()))?
            .ok_or(WebChatStandardError::NoModelConfigured)
        };
        Ok((resolve(false)?, resolve(true)?, resolve(false)?))
    })();
    let (classify_provider, chat_provider, design_provider) = match providers {
        Ok(providers) => providers,
        Err(error) => return write_error_event(out, &error.to_string()),
    };

    let classified = crate::chat_intent::classify_intent_for_standard_route(
        classify_provider.as_ref(),
        &snapshot,
        &req.ai.user,
        model.clone(),
    );
    let modify_plan = crate::chat_intent::build_modify_plan(&snapshot, &req.ai.user);
    let page_children_empty = snapshot.active_children().is_empty();
    let intent = resolve_standard_route(classified, page_children_empty, modify_plan.is_some());

    match intent {
        crate::chat_intent::DesignIntent::Chat => {
            stream_chat_route(out, &req, &snapshot, chat_provider.as_ref(), model)
        }
        crate::chat_intent::DesignIntent::Modify => {
            let plan = modify_plan.expect("route checked has_modify_plan");
            stream_modify_route(out, plan, design_provider.as_ref(), state, hub)
        }
        crate::chat_intent::DesignIntent::New => {
            stream_new_design_route(out, req, snapshot, design_provider, state, hub, model)
        }
    }
}

fn apply_request_snapshot(
    req: &WebStandardTurnRequest,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
) -> Result<EditorState, WebChatStandardError> {
    let mut broadcast_version = None;
    let mut snapshot = {
        let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(agent) = req.transient_builtin.as_ref() {
            if agent.model.trim() != req.ai.model.trim() {
                return Err(WebChatStandardError::TransientModelMismatch);
            }
            // `web_credentials` is outside this pass; carry its verdict text.
            crate::web_credentials::validate_web_provider_base_url(&agent.base_url)
                .map_err(|error| WebChatStandardError::EndpointRejected(error.to_string()))?;
            if !crate::web_credentials::public_demo_transient_endpoint_allowed(agent) {
                return Err(WebChatStandardError::EndpointNotAllowlisted);
            }
        }
        if let Some(doc_json) = req.document_json.as_deref() {
            let loaded = op_pen_loader::load_canonical(doc_json)
                .map_err(|e| WebChatStandardError::Document(e.to_string()))?;
            if guard.editor.doc != loaded.value {
                let version = guard.replace_document(loaded.value);
                broadcast_version = Some(version);
            }
        }
        if let Some(meta) = req.editor_meta {
            op_pen_loader::apply_editor_meta(&mut guard.editor, meta);
        }
        if let Some(size) = req.agent_team_size {
            guard.editor.chat.agent_team_size = size.clamp(1, 6);
        }
        guard.editor.selection.set = req.selected_ids.iter().map(NodeId::new).collect::<Vec<_>>();
        guard.editor.selection.anchor = guard
            .editor
            .selection
            .set
            .last()
            .cloned()
            .unwrap_or(NodeId::NONE);
        if let Some(page_id) = req.active_page_id.as_deref() {
            if let Some(index) = guard
                .editor
                .doc
                .pages
                .as_ref()
                .and_then(|pages| pages.iter().position(|p| p.id == page_id))
            {
                let _ = guard.editor.set_active_page(index);
            }
        }
        guard.editor.clone()
    };
    if let Some(version) = broadcast_version {
        hub.broadcast(version);
    }
    inject_transient_builtin(&mut snapshot, req.transient_builtin.as_ref());
    Ok(snapshot)
}

fn inject_transient_builtin(state: &mut EditorState, transient: Option<&BuiltinAgentConfig>) {
    let Some(transient) = transient else {
        return;
    };
    let agents = &mut state.editor_ui.agent_settings.builtin_agents;
    agents.retain(|agent| agent.id != transient.id);
    agents.insert(0, transient.clone());
    state.rebuild_chat_models();
}

fn selected_model_id(req: &AiStreamRequest) -> Option<String> {
    let model = req.model.trim();
    if model.is_empty() || model == "default" || model.starts_with("builtin:") {
        None
    } else {
        Some(model.to_string())
    }
}

fn clear_fresh_starter_frame_for_design(state: &mut EditorState) -> bool {
    if state.doc != EditorState::starter().doc {
        return false;
    }
    state.active_children_mut().clear();
    state.clear_selection();
    // Raw `active_children_mut()` bypasses the command/history path, so it
    // must advance the content revision explicitly. Save acknowledgements
    // use that revision to avoid marking newer edits as saved.
    state.mark_document_changed();
    true
}

fn clear_live_starter_frame_for_design(state: &mut WebCanvasState) -> Option<u64> {
    if !clear_fresh_starter_frame_for_design(&mut state.editor) {
        return None;
    }
    state.version += 1;
    Some(state.version)
}

fn resolve_standard_route(
    classified: crate::chat_intent::DesignIntent,
    page_children_empty: bool,
    has_modify_plan: bool,
) -> crate::chat_intent::DesignIntent {
    match classified {
        crate::chat_intent::DesignIntent::Modify if page_children_empty => {
            crate::chat_intent::DesignIntent::New
        }
        crate::chat_intent::DesignIntent::Modify if !has_modify_plan => {
            crate::chat_intent::DesignIntent::New
        }
        other => other,
    }
}

fn stream_chat_route<W: Write>(
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

fn stream_modify_route<W: Write>(
    out: &mut W,
    plan: crate::chat_intent::ModifyPlan,
    provider: &dyn ChatProvider,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
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
        let (applied, version) = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            let (count, mutated) = crate::chat_canvas_tools::apply_design_modification(
                &mut guard.editor,
                &nodes,
                &target_frame_ids,
            );
            let version = if mutated {
                guard.version += 1;
                Some(guard.version)
            } else {
                None
            };
            (count, version)
        };
        if let Some(version) = version {
            hub.broadcast(version);
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

fn stream_new_design_route<W: Write>(
    out: &mut W,
    req: WebStandardTurnRequest,
    snapshot: EditorState,
    provider: Box<dyn ChatProvider>,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
    model: Option<String>,
) -> std::io::Result<()> {
    let append_context = crate::chat_intent::detect_append_intent(&snapshot, &req.ai.user);
    let request = DesignRequest {
        prompt: req.ai.user,
        model: model.clone(),
        provider: None,
        design_md: snapshot.doc.design_md.clone(),
        append_context,
        concurrency: req
            .agent_team_size
            .unwrap_or(snapshot.chat.agent_team_size)
            .clamp(1, 6),
        validation_enabled: true,
        visual_ref_enabled: false,
    };
    // Share one provider Arc between the design LLM and (optionally) the
    // vision validator, so the real vision loop reuses the same auth/model
    // the user picked instead of needing a second key.
    let provider_arc: Arc<dyn ChatProvider> = Arc::from(provider);
    let llm = ChatProviderLlmClient::new(provider_arc.clone()).with_model(model.clone());
    let mut sink = WebDesignDocSink::new(state, hub, snapshot);
    let abort = AbortFlag::new();
    let pre_validator = LintPreValidator;

    // ── Class-C vision-validation provider selection (Track-1 Step 3) ──────────
    // REAL providers only when `OPENPENCIL_VISION_VALIDATION=1` (defaults OFF);
    // otherwise the no-op stubs keep `run_post_generation_validation` a
    // guaranteed short-circuit, so the default path is byte-for-byte unchanged.
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
    // The browser transcript learns the persona first. The daemon relay then
    // confirms that exact same identity, so the canvas cursor cannot appear
    // under a different name or colour than the visible assistant bubble.
    write_agent_identity_event(out, &identity)?;
    let epoch = op_editor_core::agent_indicators::begin();
    op_editor_core::agent_indicators::confirm_cursor_agent(epoch, &identity.color, &identity.name);
    let summary = {
        let out_ref = &mut *out;
        let mut on_progress = move |p: Progress| {
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
    // Natural completion drains the queued reveals gracefully; an
    // aborted turn tears the overlay down at once.
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

struct WebDesignDocSink<'a> {
    state: &'a Mutex<WebCanvasState>,
    hub: &'a SseHub,
    mirror: EditorState,
}

impl<'a> WebDesignDocSink<'a> {
    fn new(state: &'a Mutex<WebCanvasState>, hub: &'a SseHub, mirror: EditorState) -> Self {
        Self { state, hub, mirror }
    }
}

impl DocSink for WebDesignDocSink<'_> {
    fn state(&self) -> &EditorState {
        &self.mirror
    }

    fn apply(&mut self, cmd: EditorCommand) -> bool {
        let (applied, version, snapshot) = {
            let mut guard = self.state.lock().unwrap_or_else(|p| p.into_inner());
            let applied = guard.editor.apply(cmd);
            let version = if applied {
                crate::design_session::fit_design_viewport_to_content(
                    &mut guard.editor,
                    1440.0,
                    900.0,
                );
                guard.version += 1;
                Some(guard.version)
            } else {
                None
            };
            (applied, version, guard.editor.clone())
        };
        self.mirror = snapshot;
        if let Some(version) = version {
            self.hub.broadcast(version);
        }
        applied
    }

    fn begin_undo_batch(&mut self) {}

    fn end_undo_batch(&mut self) {}
}

#[cfg(test)]
#[path = "web_chat_standard_tests.rs"]
mod tests;
