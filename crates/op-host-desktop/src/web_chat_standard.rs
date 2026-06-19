//! Standard chat/design turn for the Rust web shell.
//!
//! The browser owns the immediate UI, but it cannot spawn CLI providers or
//! hold API keys. This endpoint mirrors the desktop "standard mode" route on
//! the daemon side: classify the user's turn, then dispatch to plain chat,
//! design modification, or the orchestrator-backed new-design pipeline.

use std::io::Write;
use std::sync::{Arc, Mutex};

use base64::Engine as _;
use op_ai::chat_provider::{
    ChatAttachment, ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, StopReason,
};
use op_editor_core::chat::MAX_ATTACHMENT_BYTES;
use op_editor_core::{EditorCommand, EditorState, NodeId};
use op_orchestrator::{
    AbortFlag, DesignRequest, DocSink, Orchestrator, Progress, SkippedScreenshotProvider,
    SkippedVisionLlmClient, ValidationProviders,
};
use serde_json::Value;

use op_web_daemon::ai_proxy::AiStreamRequest;
use op_web_daemon::chat_provider_llm::ChatProviderLlmClient;
use op_web_daemon::pre_validator::LintPreValidator;
use crate::web_canvas_server::{SseHub, WebCanvasState};

const STANDARD_MODIFY_STEP: &str =
    r#"<step title="Checking guidelines">Analyzing modification request...</step>"#;

pub(crate) struct WebStandardTurnRequest {
    pub ai: AiStreamRequest,
    document_json: Option<String>,
    selected_ids: Vec<String>,
    active_page_id: Option<String>,
    agent_team_size: Option<u32>,
    history: Vec<(ChatHistoryRole, String)>,
    attachments: Vec<ChatAttachment>,
}

pub(crate) fn parse_standard_turn_body(body: &str) -> Option<WebStandardTurnRequest> {
    let ai = op_web_daemon::ai_proxy::parse_ai_stream_body(body)?;
    let value: Value = serde_json::from_str(body).ok()?;
    let obj = value.as_object()?;
    let document_json = obj.get("document").map(Value::to_string);
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
    Some(WebStandardTurnRequest {
        ai,
        document_json,
        selected_ids,
        active_page_id,
        agent_team_size,
        history,
        attachments,
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

pub(crate) fn stream_standard_turn<W: Write>(
    out: &mut W,
    req: WebStandardTurnRequest,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
) -> std::io::Result<()> {
    op_web_daemon::ai_proxy::write_sse_headers(out)?;

    let mut snapshot = match apply_request_snapshot(&req, state, hub) {
        Ok(snapshot) => snapshot,
        Err(message) => {
            return write_error_event(out, &message);
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
            if guard.editor.doc == EditorState::starter().doc {
                guard.editor.active_children_mut().clear();
                guard.editor.clear_selection();
                guard.version += 1;
                snapshot = guard.editor.clone();
                Some(guard.version)
            } else {
                None
            }
        };
        if let Some(version) = version {
            hub.broadcast(version);
        }
    }

    let Some(classify_provider) =
        op_web_daemon::ai_proxy::proxy_provider_with_chat_session(&snapshot, &req.ai.model, false)
    else {
        return write_error_event(out, "no model configured");
    };
    let Some(chat_provider) =
        op_web_daemon::ai_proxy::proxy_provider_with_chat_session(&snapshot, &req.ai.model, true)
    else {
        return write_error_event(out, "no model configured");
    };
    let Some(design_provider) =
        op_web_daemon::ai_proxy::proxy_provider_with_chat_session(&snapshot, &req.ai.model, false)
    else {
        return write_error_event(out, "no model configured");
    };

    let classified = op_web_daemon::chat_intent::classify_intent_llm(
        classify_provider.as_ref(),
        &req.ai.user,
        model.clone(),
    );
    let modify_plan = op_web_daemon::chat_intent::build_modify_plan(&snapshot, &req.ai.user);
    let page_children_empty = snapshot.active_children().is_empty();
    let intent = resolve_standard_route(classified, page_children_empty, modify_plan.is_some());

    match intent {
        op_web_daemon::chat_intent::DesignIntent::Chat => {
            stream_chat_route(out, &req, &snapshot, chat_provider.as_ref(), model)
        }
        op_web_daemon::chat_intent::DesignIntent::Modify => {
            let plan = modify_plan.expect("route checked has_modify_plan");
            stream_modify_route(out, plan, design_provider.as_ref(), state, hub)
        }
        op_web_daemon::chat_intent::DesignIntent::New => {
            stream_new_design_route(out, req, snapshot, design_provider, state, hub, model)
        }
    }
}

fn apply_request_snapshot(
    req: &WebStandardTurnRequest,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
) -> Result<EditorState, String> {
    let mut broadcast_version = None;
    let snapshot = {
        let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(doc_json) = req.document_json.as_deref() {
            let loaded = op_pen_loader::load_canonical(doc_json).map_err(|e| e.to_string())?;
            if guard.editor.doc != loaded.value {
                let version = guard.replace_document(loaded.value);
                broadcast_version = Some(version);
            }
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
    Ok(snapshot)
}

fn selected_model_id(req: &AiStreamRequest) -> Option<String> {
    let model = req.model.trim();
    if model.is_empty() || model == "default" {
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
    true
}

fn resolve_standard_route(
    classified: op_web_daemon::chat_intent::DesignIntent,
    page_children_empty: bool,
    has_modify_plan: bool,
) -> op_web_daemon::chat_intent::DesignIntent {
    match classified {
        op_web_daemon::chat_intent::DesignIntent::Modify if page_children_empty => {
            op_web_daemon::chat_intent::DesignIntent::New
        }
        op_web_daemon::chat_intent::DesignIntent::Modify if !has_modify_plan => {
            op_web_daemon::chat_intent::DesignIntent::New
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
        system_prompt: op_web_daemon::chat_system_prompt::build_chat_system_prompt(state, &req.ai.user),
        user_message: req.ai.user.clone(),
        history: req.history.clone(),
        max_output_tokens: req.ai.max_output_tokens,
        thinking: req.ai.thinking,
        effort: req.ai.effort,
        attachments: req.attachments.clone(),
        model,
    };
    for delta in provider.send(chat_req) {
        out.write_all(op_web_daemon::ai_proxy::delta_to_sse(&delta).as_bytes())?;
        out.flush()?;
        if matches!(delta, ChatDelta::Done { .. } | ChatDelta::Error(_)) {
            break;
        }
    }
    Ok(())
}

fn stream_modify_route<W: Write>(
    out: &mut W,
    plan: op_web_daemon::chat_intent::ModifyPlan,
    provider: &dyn ChatProvider,
    state: &Mutex<WebCanvasState>,
    hub: &SseHub,
) -> std::io::Result<()> {
    write_delta_event(out, STANDARD_MODIFY_STEP)?;
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

    let nodes = op_orchestrator::parse::parse_nodes(&full_response).unwrap_or_default();
    if !nodes.is_empty() {
        write_delta_event(out, &format!("\n{full_response}"))?;
        let node_values = nodes
            .iter()
            .map(|n| serde_json::to_value(n).unwrap_or(Value::Null))
            .collect::<Vec<_>>();
        let (applied, version) = {
            let mut guard = state.lock().unwrap_or_else(|p| p.into_inner());
            let (count, mutated) = op_web_daemon::chat_canvas_tools::apply_design_modification(
                &mut guard.editor,
                &node_values,
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
    let append_context = op_web_daemon::chat_intent::detect_append_intent(&snapshot, &req.ai.user);
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
    let llm = ChatProviderLlmClient::new(Arc::from(provider)).with_model(model);
    let mut sink = WebDesignDocSink::new(state, hub, snapshot);
    let abort = AbortFlag::new();
    let pre_validator = LintPreValidator;
    let screenshot = SkippedScreenshotProvider;
    let vision = SkippedVisionLlmClient;
    let providers = ValidationProviders {
        pre_validator: &pre_validator,
        screenshot: &screenshot,
        vision: &vision,
        system_prompt: String::new(),
    };
    let epoch = op_editor_core::agent_indicators::begin();
    let summary = {
        let out_ref = &mut *out;
        let mut on_progress = move |p: Progress| {
            let _ = write_thinking_event(out_ref, &format!("\n{}", progress_label(&p)));
        };
        op_web_daemon::chat_runtime::shared_runtime().block_on(
            Orchestrator::new().with_indicator_epoch(epoch).run(
                request,
                &mut sink,
                &llm,
                &mut on_progress,
                &abort,
                &providers,
            ),
        )
    };
    op_editor_core::agent_indicators::end_if_epoch(epoch);
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
                op_web_daemon::design_session::fit_design_viewport_to_content(
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

fn write_delta_event<W: Write>(out: &mut W, text: &str) -> std::io::Result<()> {
    out.write_all(
        op_web_daemon::ai_proxy::delta_to_sse(&ChatDelta::TextDelta(text.to_string())).as_bytes(),
    )?;
    out.flush()
}

fn write_thinking_event<W: Write>(out: &mut W, text: &str) -> std::io::Result<()> {
    out.write_all(
        op_web_daemon::ai_proxy::delta_to_sse(&ChatDelta::Thinking(text.to_string())).as_bytes(),
    )?;
    out.flush()
}

fn write_done_event<W: Write>(out: &mut W) -> std::io::Result<()> {
    out.write_all(
        op_web_daemon::ai_proxy::delta_to_sse(&ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        })
        .as_bytes(),
    )?;
    out.flush()
}

fn write_error_event<W: Write>(out: &mut W, message: &str) -> std::io::Result<()> {
    out.write_all(
        op_web_daemon::ai_proxy::delta_to_sse(&ChatDelta::Error(message.to_string())).as_bytes(),
    )?;
    out.flush()
}

fn progress_label(p: &Progress) -> String {
    match p {
        Progress::Planning => "• Planning…".into(),
        Progress::ScaffoldDone => "• Scaffold ready".into(),
        Progress::SubtaskStarted { id, label } => format!("• Subtask `{id}` — {label}"),
        Progress::SubtaskDone { id, node_count } => {
            format!("• Subtask `{id}` done ({node_count} nodes)")
        }
        Progress::SubtaskFailed { id, error } => format!("• Subtask `{id}` failed: {error}"),
        Progress::CleanupDone => "• Cleanup done".into(),
        Progress::ValidationStarted => "• Validation started".into(),
        Progress::ValidationPreCheckDone { applied, .. } => {
            format!("• Pre-validation applied {applied} fix(es)")
        }
        Progress::ValidationRoundStarted { round } => format!("• Vision round {round} started"),
        Progress::ValidationRoundDone {
            round,
            applied,
            quality_score,
        } => {
            format!("• Vision round {round} done — {applied} fix(es), quality {quality_score}/100")
        }
        Progress::ValidationDone { total_applied } => {
            format!("• Validation done — {total_applied} fix(es) total")
        }
        Progress::VisualRefStarted => "• Visual-ref pipeline started".into(),
        Progress::VisualRefDesignSystem { var_count } => {
            format!("• Design system ready — {var_count} variable(s) seeded")
        }
        Progress::VisualRefHtmlGenerated { byte_len } => {
            format!("• Visual-ref HTML generated ({byte_len} bytes)")
        }
        Progress::VisualRefScreenshotReady { skipped } => {
            if *skipped {
                "• Visual-ref screenshot skipped".into()
            } else {
                "• Visual-ref screenshot captured".into()
            }
        }
        Progress::VisualRefFallback { reason } => format!("• Visual-ref fallback: {reason}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_ai::chat_provider::{EffortLevel, ThinkingMode};

    struct CaptureProvider {
        seen: Arc<Mutex<Option<ChatRequest>>>,
    }

    impl ChatProvider for CaptureProvider {
        fn provider_label(&self) -> &str {
            "capture"
        }

        fn send(&self, request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
            *self.seen.lock().expect("seen lock") = Some(request);
            Box::new(std::iter::once(ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            }))
        }
    }

    #[test]
    fn parse_standard_turn_body_reads_canvas_snapshot_fields() {
        let body = serde_json::json!({
            "model": "claude-sonnet",
            "user": "design a page",
            "document": { "version": "1.0.0", "children": [] },
            "selectedIds": ["n1", "", "n2"],
            "activePageId": "page-1",
            "agent_team_size": 9,
            "history": [
                { "role": "user", "content": "previous request" },
                { "role": "assistant", "content": "previous answer" },
                { "role": "system", "content": "ignored" },
                { "role": "user", "content": "" }
            ],
            "attachments": [
                {
                    "name": "a.png",
                    "media_type": "image/png",
                    "data_base64": "AQID"
                },
                {
                    "name": "bad.txt",
                    "media_type": "text/plain",
                    "data_base64": "not base64"
                }
            ]
        })
        .to_string();

        let req = parse_standard_turn_body(&body).expect("request parses");

        assert_eq!(req.ai.model, "claude-sonnet");
        assert_eq!(req.ai.user, "design a page");
        assert!(req.document_json.is_some());
        assert_eq!(req.selected_ids, vec!["n1".to_string(), "n2".to_string()]);
        assert_eq!(req.active_page_id.as_deref(), Some("page-1"));
        assert_eq!(req.agent_team_size, Some(6));
        assert_eq!(
            req.history,
            vec![
                (
                    op_ai::chat_provider::ChatHistoryRole::User,
                    "previous request".into()
                ),
                (
                    op_ai::chat_provider::ChatHistoryRole::Assistant,
                    "previous answer".into()
                ),
            ]
        );
        assert_eq!(req.attachments.len(), 1);
        assert_eq!(req.attachments[0].name, "a.png");
        assert_eq!(req.attachments[0].media_type, "image/png");
        assert_eq!(req.attachments[0].data, vec![1, 2, 3]);
    }

    #[test]
    fn stream_chat_route_passes_history_and_attachments_to_provider() {
        let history = vec![
            (ChatHistoryRole::User, "previous request".to_string()),
            (ChatHistoryRole::Assistant, "previous answer".to_string()),
        ];
        let attachments = vec![op_ai::chat_provider::ChatAttachment {
            name: "a.png".to_string(),
            media_type: "image/png".to_string(),
            data: vec![1, 2, 3],
        }];
        let req = WebStandardTurnRequest {
            ai: AiStreamRequest {
                model: "claude-sonnet".into(),
                skills: Vec::new(),
                user: "current request".into(),
                max_output_tokens: 2048,
                thinking: ThinkingMode::Adaptive,
                effort: EffortLevel::Low,
            },
            document_json: None,
            selected_ids: Vec::new(),
            active_page_id: None,
            agent_team_size: None,
            history: history.clone(),
            attachments: attachments.clone(),
        };
        let seen = Arc::new(Mutex::new(None));
        let provider = CaptureProvider { seen: seen.clone() };
        let mut out = Vec::new();

        stream_chat_route(&mut out, &req, &EditorState::new(), &provider, None)
            .expect("stream chat");

        let captured = seen
            .lock()
            .expect("seen lock")
            .clone()
            .expect("provider saw request");
        assert_eq!(captured.user_message, "current request");
        assert_eq!(captured.history, history);
        assert_eq!(captured.attachments, attachments);
    }

    #[test]
    fn design_doc_sink_applies_and_bumps_version() {
        let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
        let hub = SseHub::default();
        let sub = hub.subscribe();
        let mirror = state.lock().unwrap().editor.clone();
        let mut sink = WebDesignDocSink::new(&state, &hub, mirror);

        assert!(sink.apply(EditorCommand::InsertNode {
            kind: "rect".into(),
            name: "Generated".into(),
            x: 10,
            y: 20,
            width: 100,
            height: 50,
            fill_hex: Some("#ff0000".into()),
            target_parent: NodeId::NONE,
            page_id: None,
        }));

        assert_eq!(state.lock().unwrap().version, 1);
        assert_eq!(sub.recv().unwrap(), 1);
        assert_eq!(sink.state().active_children().len(), 1);
    }

    #[test]
    fn progress_labels_match_desktop_design_session_bullets() {
        assert_eq!(progress_label(&Progress::Planning), "• Planning…");
        assert_eq!(
            progress_label(&Progress::SubtaskStarted {
                id: "brand".into(),
                label: "Brand Header".into(),
            }),
            "• Subtask `brand` — Brand Header"
        );
    }
}
