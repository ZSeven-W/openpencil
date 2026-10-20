use super::events::progress_label;
use super::*;
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, EffortLevel, ThinkingMode};
use op_orchestrator::Progress;
use std::sync::{Arc, Mutex};

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

fn standard_body_with_credential(api_key: &str) -> String {
    standard_body_with_credential_at(api_key, "https://api.openai.com/v1")
}

fn standard_body_with_credential_at(api_key: &str, base_url: &str) -> String {
    serde_json::json!({
        "model": "private-model",
        "user": "hello",
        "credential": {
            "id": "builtin-web-1",
            "preset": "custom",
            "display_name": "Private",
            "kind": "openai-compat",
            "api_key": api_key,
            "model": "private-model",
            "base_url": base_url,
            "enabled": true
        }
    })
    .to_string()
}

#[test]
fn transient_credential_is_applied_only_to_the_turn_snapshot() {
    let body = standard_body_with_credential("sk-transient");
    let req = parse_standard_turn_body(&body).expect("request parses");
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));

    let snapshot =
        apply_request_snapshot(&req, &state, &SseHub::default(), None).expect("request snapshot");

    assert_eq!(
        snapshot.editor_ui.agent_settings.builtin_agents[0].api_key,
        "sk-transient"
    );
    assert!(crate::ai_proxy::proxy_provider(&snapshot, "private-model").is_some());
    assert!(state
        .lock()
        .unwrap()
        .editor
        .editor_ui
        .agent_settings
        .builtin_agents
        .is_empty());
}

#[test]
fn invalid_transient_credential_is_rejected() {
    let oversized = "x".repeat(16 * 1024 + 1);
    let body = standard_body_with_credential(&oversized);

    assert!(parse_standard_turn_body(&body).is_none());
}

#[test]
fn transient_credential_model_must_match_the_requested_model() {
    let mut body: serde_json::Value =
        serde_json::from_str(&standard_body_with_credential("sk-transient")).unwrap();
    body["model"] = serde_json::Value::String("different-model".into());
    let req = parse_standard_turn_body(&body.to_string()).expect("credential shape parses");
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));

    let error = apply_request_snapshot(&req, &state, &SseHub::default(), None)
        .expect_err("mismatched credential must be rejected")
        .to_string();

    assert!(error.contains("model does not match"));
    assert!(state
        .lock()
        .unwrap()
        .editor
        .editor_ui
        .agent_settings
        .builtin_agents
        .is_empty());
}

#[test]
fn browser_only_demo_rejects_custom_or_loopback_transient_endpoints_without_mutation() {
    for base_url in ["http://127.0.0.1:8080/v1", "https://example.test/v1"] {
        let body = standard_body_with_credential_at("sk-transient", base_url);
        let req = parse_standard_turn_body(&body).expect("credential shape parses");
        let state = Mutex::new(WebCanvasState::new_with_policy(
            EditorState::new(),
            3100,
            crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
        ));
        let before = crate::settings_io::fingerprint(&state.lock().unwrap().editor);

        let error = apply_request_snapshot(&req, &state, &SseHub::default(), None)
            .expect_err("public demo must reject custom endpoint")
            .to_string();

        assert!(error.contains("endpoint"));
        assert_eq!(
            before,
            crate::settings_io::fingerprint(&state.lock().unwrap().editor)
        );
    }
}

#[test]
fn server_persistence_does_not_allow_a_reserved_transient_endpoint() {
    let body = standard_body_with_credential_at("sk-transient", "http://169.254.169.254/v1");
    let req = parse_standard_turn_body(&body).expect("credential shape parses");
    let state = Mutex::new(WebCanvasState::new_with_policy(
        EditorState::new(),
        3100,
        crate::web_credential_policy::WebCredentialPersistence::Server,
    ));

    let error = apply_request_snapshot(&req, &state, &SseHub::default(), None)
        .expect_err("persistence must not authorize a reserved provider endpoint")
        .to_string();
    assert!(error.contains("endpoint"), "unexpected error: {error}");
}

#[test]
fn parse_standard_turn_body_reads_canvas_snapshot_fields() {
    let body = serde_json::json!({
        "model": "claude-sonnet",
        "user": "design a page",
        "document": { "version": "1.0.0", "children": [] },
        "editorMeta": {
            "activePageIndex": 3,
            "preserveAuthoredGeometry": true
        },
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
    assert_eq!(
        req.editor_meta,
        Some(op_pen_loader::EditorMeta {
            active_page_index: 3,
            preserve_authored_geometry: true,
            scenario: None,
            work_family: None,
            pinned_style_guide: None,
            share_recipe: None,
            imported_from: None,
        })
    );
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
            hosted_route_id: None,
            history: Vec::new(),
            attachments: Vec::new(),
            provider: None,
            builtin_provider_id: None,
            model: "claude-sonnet".into(),
            skills: Vec::new(),
            user: "current request".into(),
            max_output_tokens: 2048,
            thinking: ThinkingMode::Adaptive,
            effort: EffortLevel::Low,
            transient_builtin: None,
        },
        document_json: None,
        editor_meta: None,
        selected_ids: Vec::new(),
        workspace_visible: false,
        workspace_selected: 0,
        active_page_id: None,
        agent_team_size: None,
        history: history.clone(),
        attachments: attachments.clone(),
        transient_builtin: None,
        launch_route: op_editor_core::LaunchRoute::Auto,
        locale: None,
    };
    let seen = Arc::new(Mutex::new(None));
    let provider = CaptureProvider { seen: seen.clone() };
    let mut out = Vec::new();

    stream_chat_route(&mut out, &req, &EditorState::new(), &provider, None).expect("stream chat");

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
fn request_snapshot_restores_editor_meta_before_design_response_updates() {
    let body = serde_json::json!({
        "model": "default",
        "user": "update the design",
        "document": {
            "version": "1.0.0",
            "children": [],
            "pages": [
                {"id":"p1","name":"One","children":[]},
                {"id":"p2","name":"Two","children":[]}
            ]
        },
        "editorMeta": {
            "activePageIndex": 1,
            "preserveAuthoredGeometry": true
        }
    })
    .to_string();
    let req = parse_standard_turn_body(&body).expect("request parses");
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));

    let snapshot =
        apply_request_snapshot(&req, &state, &SseHub::default(), None).expect("request snapshot");

    assert_eq!(snapshot.ui.active_page_index, 1);
    assert!(snapshot.editor_ui.preserve_authored_geometry);
    let live = state.lock().expect("live state");
    assert_eq!(live.editor.ui.active_page_index, 1);
    assert!(live.editor.editor_ui.preserve_authored_geometry);
}

#[test]
fn design_doc_sink_applies_and_bumps_version() {
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let hub = SseHub::default();
    let sub = hub.subscribe();
    let mirror = state.lock().unwrap().editor.clone();
    let mut sink = WebDesignDocSink::new(&state, &hub, None, mirror);

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
    assert_eq!(sub.pending().expect("published").version, 1);
    assert_eq!(sink.state().active_children().len(), 1);
}

#[test]
fn starter_clear_marks_content_dirty_after_stale_save_ack() {
    let mut state = EditorState::starter();
    state.mark_saved_revision();
    let generation = state.document_generation();
    let stale_revision = state.document_revision();
    assert!(!state.is_dirty());

    assert!(clear_fresh_starter_frame_for_design(&mut state));

    assert!(state.active_children().is_empty());
    assert!(state.document_revision() > stale_revision);
    assert!(state.is_dirty());
    assert!(state.mark_saved_revision_at(generation, stale_revision));
    assert!(
        state.is_dirty(),
        "an acknowledgement for the pre-clear snapshot must not mark the cleared document saved"
    );
}

#[test]
fn live_starter_clear_bumps_server_version_once_and_marks_document_dirty() {
    let mut state = WebCanvasState::new(EditorState::starter(), 3100);
    state.version = 41;
    state.editor.mark_saved_revision();
    let initial_revision = state.editor.document_revision();

    assert_eq!(clear_live_starter_frame_for_design(&mut state), Some(42));
    assert_eq!(state.version, 42);
    assert!(state.editor.is_dirty());
    assert!(state.editor.document_revision() > initial_revision);

    let cleared_revision = state.editor.document_revision();
    assert_eq!(clear_live_starter_frame_for_design(&mut state), None);
    assert_eq!(state.version, 42);
    assert_eq!(state.editor.document_revision(), cleared_revision);
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

// Lock the web progress_label output for SubtaskSkills and SubtaskRetry
// against the cluster-C contract — byte-identical to the desktop formatter
// in op-host-desktop/src/design_session.rs.
#[test]
fn web_progress_label_matches_desktop_skill_block_format() {
    use op_orchestrator::SkillBrief;

    let p = Progress::SubtaskSkills {
        id: "header".into(),
        included: vec![SkillBrief {
            name: "cjk-typography".into(),
            token_count: 800,
            truncated: true,
        }],
        dropped: vec![("examples".into(), "budget".into())],
        budget_used: 5200,
        budget_max: 8000,
    };
    let s = progress_label(&p);
    assert!(
        s.contains("• Subtask `header`  ·  1 skills · 5200/8000 tok · 1 dropped"),
        "summary line mismatch: {s}"
    );
    assert!(
        s.contains("\n  ▸ skills: cjk-typography (truncated)"),
        "skills sub-line mismatch: {s}"
    );
    assert!(
        s.contains("\n  ▸ dropped: examples (budget)"),
        "dropped sub-line mismatch: {s}"
    );
}

#[test]
fn web_progress_label_subtask_retry_format() {
    assert_eq!(
        progress_label(&Progress::SubtaskRetry {
            id: "header".into(),
            attempt: 2,
            reason: "zero nodes generated".into(),
        }),
        "  ▸ retry #2: zero nodes generated"
    );
}
