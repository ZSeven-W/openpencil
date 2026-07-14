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
        apply_request_snapshot(&req, &state, &SseHub::default()).expect("request snapshot");

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

    let error = apply_request_snapshot(&req, &state, &SseHub::default())
        .expect_err("mismatched credential must be rejected");

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

        let error = apply_request_snapshot(&req, &state, &SseHub::default())
            .expect_err("public demo must reject custom endpoint");

        assert!(error.contains("endpoint"));
        assert_eq!(
            before,
            crate::settings_io::fingerprint(&state.lock().unwrap().editor)
        );
    }
}

#[test]
fn server_persistence_does_not_allow_an_unapproved_public_transient_endpoint() {
    let body = standard_body_with_credential_at("sk-transient", "https://attacker.example/v1");
    let req = parse_standard_turn_body(&body).expect("credential shape parses");
    let state = Mutex::new(WebCanvasState::new_with_policy(
        EditorState::new(),
        3100,
        crate::web_credential_policy::WebCredentialPersistence::Server,
    ));

    let error = apply_request_snapshot(&req, &state, &SseHub::default())
        .expect_err("persistence must not authorize an arbitrary public provider endpoint");
    assert!(error.contains("endpoint"), "unexpected error: {error}");
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
            provider: None,
            model: "claude-sonnet".into(),
            skills: Vec::new(),
            user: "current request".into(),
            max_output_tokens: 2048,
            thinking: ThinkingMode::Adaptive,
            effort: EffortLevel::Low,
            transient_builtin: None,
        },
        document_json: None,
        selected_ids: Vec::new(),
        active_page_id: None,
        agent_team_size: None,
        history: history.clone(),
        attachments: attachments.clone(),
        transient_builtin: None,
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
