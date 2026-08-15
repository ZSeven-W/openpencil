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
            pinned_style_guide: None,
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

#[test]
fn a_closed_write_barrier_leaves_the_document_untouched() {
    // `/api/ai/standard` used to be dispatched before the write admission, so
    // its document commits ran during shutdown — after the flush had already
    // snapshotted the document.
    use crate::web_canvas_server::WriteBarrier;

    let barrier = WriteBarrier::default();
    barrier.close();

    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let before = state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .document_version_for_test();

    let req = parse_standard_turn_body(&standard_body_with_document()).expect("request parses");
    let error = apply_request_snapshot(&req, &state, &SseHub::default(), Some(&barrier))
        .expect_err("a closed barrier must refuse the commit");
    assert!(
        matches!(error, WebChatStandardError::ShuttingDown),
        "{error:?}"
    );
    assert_eq!(
        state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .document_version_for_test(),
        before,
        "the refused turn must not have changed the document"
    );
}

#[test]
fn an_open_write_barrier_applies_the_turn_normally() {
    use crate::web_canvas_server::WriteBarrier;
    let barrier = WriteBarrier::default();
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let req = parse_standard_turn_body(&standard_body_with_document()).expect("request parses");
    apply_request_snapshot(&req, &state, &SseHub::default(), Some(&barrier))
        .expect("an open barrier admits the commit");
    assert!(
        state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .document_version_for_test()
            > 0,
        "the turn must have applied"
    );
}

/// A standard-turn body that carries a document, so `apply_request_snapshot`
/// reaches its `replace_document` commit.
fn standard_body_with_document() -> String {
    serde_json::json!({
        "message": "hello",
        "document": {
            "version": "1.0.0",
            "children": [{
                "id": "n1", "type": "rectangle", "name": "from-turn",
                "x": 0, "y": 0, "width": 4, "height": 4,
            }],
        },
    })
    .to_string()
}

#[test]
fn a_closed_write_barrier_skips_the_editor_metadata_write() {
    // `EditorMeta::from_state` serialises `active_page_index` and
    // `preserve_authored_geometry` into the tenant's persisted snapshot, so
    // both are document writes for admission purposes even when the document
    // itself is unchanged. A turn arriving after the flush snapshotted the
    // document must not move them.
    //
    // Skipped, not refused: a plain chat turn carries this metadata
    // incidentally, so it still gets its reply.
    use crate::web_canvas_server::WriteBarrier;

    let body = serde_json::json!({
        "model": "default",
        "user": "hello",
        "editorMeta": { "activePageIndex": 1, "preserveAuthoredGeometry": true },
    })
    .to_string();
    let req = parse_standard_turn_body(&body).expect("request parses");

    let barrier = WriteBarrier::default();
    barrier.close();
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));

    let snapshot = apply_request_snapshot(&req, &state, &SseHub::default(), Some(&barrier))
        .expect("a metadata-only turn still gets its snapshot");

    assert_eq!(snapshot.ui.active_page_index, 0);
    assert!(!snapshot.editor_ui.preserve_authored_geometry);
    let live = state.lock().unwrap_or_else(|p| p.into_inner());
    assert_eq!(
        live.editor.ui.active_page_index, 0,
        "a closed barrier must leave the persisted active page where the flush found it"
    );
    assert!(
        !live.editor.editor_ui.preserve_authored_geometry,
        "a closed barrier must leave the persisted geometry flag alone"
    );
}

#[test]
fn a_closed_write_barrier_skips_the_starter_frame_clear() {
    // The design route clears the starter frame before generating. That clear
    // is a document commit, so during shutdown it is skipped and the starter
    // frame survives into the flushed snapshot.
    use crate::web_canvas_server::WriteBarrier;

    let body =
        serde_json::json!({ "model": "default", "user": "design a landing page" }).to_string();
    let req = parse_standard_turn_body(&body).expect("request parses");

    let closed = WriteBarrier::default();
    closed.close();
    let state = Mutex::new(WebCanvasState::new(EditorState::starter(), 3100));
    let before = state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .editor
        .active_children()
        .len();
    assert!(before > 0, "the starter document has a frame to clear");

    let mut out = Vec::new();
    stream_standard_turn(
        &mut out,
        req,
        &state,
        &SseHub::default(),
        Some(&closed),
        None,
    )
    .expect("the turn still answers");

    assert_eq!(
        state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .editor
            .active_children()
            .len(),
        before,
        "a closed barrier must leave the starter frame in place"
    );
}

/// A provider that replays one fixed text response.
struct ScriptedProvider {
    response: String,
}

impl ChatProvider for ScriptedProvider {
    fn provider_label(&self) -> &str {
        "scripted"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        let text = self.response.clone();
        Box::new(
            [
                ChatDelta::TextDelta(text),
                ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                },
            ]
            .into_iter(),
        )
    }
}

/// A canvas holding one frame the modify route can target.
fn modify_target_state() -> WebCanvasState {
    let doc_json = serde_json::json!({
        "version": "1.0.0",
        "children": [{
            "id": "n217", "type": "frame", "name": "Card",
            "x": 0, "y": 0, "width": 100, "height": 100, "children": [],
        }],
    })
    .to_string();
    let loaded = op_pen_loader::load_canonical(&doc_json).expect("fixture document loads");
    WebCanvasState::new(EditorState::from_document(loaded.value), 3100)
}

fn modify_plan() -> crate::chat_intent::ModifyPlan {
    crate::chat_intent::ModifyPlan {
        user_message: "rename the card".into(),
        system_prompt: String::new(),
        target_frame_ids: vec!["n217".to_string()],
    }
}

/// The model's reply, renaming the targeted frame.
const MODIFY_RESPONSE: &str = r#"[{"type":"frame","id":"n217","name":"Renamed","x":0,"y":0,"width":100,"height":100,"children":[]}]"#;

#[test]
fn a_closed_write_barrier_still_streams_the_modify_reply_without_writing() {
    // The modify route writes a batch straight into the editor, so during
    // shutdown it must degrade to `(0, false)` — no nodes applied, no version
    // bump — while still streaming the model's answer back to the user. A
    // refusal here would turn a shutdown into a visible chat error.
    use crate::web_canvas_server::WriteBarrier;

    let barrier = WriteBarrier::default();
    barrier.close();
    let state = Mutex::new(modify_target_state());
    let before = state.lock().unwrap_or_else(|p| p.into_inner()).version;
    let provider = ScriptedProvider {
        response: MODIFY_RESPONSE.to_string(),
    };
    let mut out = Vec::new();

    stream_modify_route(
        &mut out,
        modify_plan(),
        &provider,
        &state,
        &SseHub::default(),
        Some(&barrier),
    )
    .expect("the modify turn still answers");

    let streamed = String::from_utf8(out).expect("utf8 sse");
    assert!(
        streamed.contains("Renamed"),
        "the reply text must still reach the user: {streamed}"
    );
    let live = state.lock().unwrap_or_else(|p| p.into_inner());
    assert_eq!(
        live.version, before,
        "a closed barrier must not bump the document version"
    );
    let node = serde_json::to_value(&live.editor.active_children()[0]).expect("node serialises");
    assert_eq!(
        node["name"],
        serde_json::json!("Card"),
        "a closed barrier must leave the node as the flush found it"
    );
}

#[test]
fn a_closed_write_barrier_makes_the_design_doc_sink_ack_false() {
    // Every generated command is its own commit, so each needs its own instant
    // of admission. A closed barrier acks `false`, which the generator already
    // treats as "not applied" — the alternative would be a write landing after
    // the flush snapshotted the document.
    use crate::web_canvas_server::WriteBarrier;

    let barrier = WriteBarrier::default();
    barrier.close();
    let state = Mutex::new(WebCanvasState::new(EditorState::new(), 3100));
    let hub = SseHub::default();
    let mirror = state
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .editor
        .clone();
    let mut sink = WebDesignDocSink::new(&state, &hub, Some(&barrier), mirror);

    assert!(
        !sink.apply(EditorCommand::InsertNode {
            kind: "rect".into(),
            name: "Generated".into(),
            x: 10,
            y: 20,
            width: 100,
            height: 50,
            fill_hex: Some("#ff0000".into()),
            target_parent: NodeId::NONE,
            page_id: None,
        }),
        "a closed barrier must ack false"
    );
    let live = state.lock().unwrap_or_else(|p| p.into_inner());
    assert_eq!(live.version, 0, "no version bump for an unapplied command");
    assert!(
        live.editor.active_children().is_empty(),
        "no node may reach the document"
    );
    assert!(
        sink.state().active_children().is_empty(),
        "the generator's mirror must not advertise a node that was never applied"
    );
}

/// A two-page daemon document, so switching the active page is possible.
fn multi_page_state() -> WebCanvasState {
    let doc_json = serde_json::json!({
        "version": "1.0.0",
        "children": [],
        "pages": [
            {"id": "p1", "name": "One", "children": []},
            {"id": "p2", "name": "Two", "children": []},
        ],
    })
    .to_string();
    let loaded = op_pen_loader::load_canonical(&doc_json).expect("fixture document loads");
    WebCanvasState::new(EditorState::from_document(loaded.value), 3100)
}

/// A turn that asks for a different active page and carries no document.
fn active_page_body(page_id: &str) -> String {
    serde_json::json!({
        "model": "default",
        "user": "hello",
        "activePageId": page_id,
    })
    .to_string()
}

#[test]
fn a_closed_write_barrier_leaves_the_active_page_where_the_flush_found_it() {
    // `active_page_index` is serialised into the tenant's persisted snapshot by
    // `EditorMeta::from_state`, so moving it after the flush has snapshotted
    // the document loses the move — or worse, persists a page the flushed
    // document does not describe.
    //
    // This covers the SECOND admission guard specifically. The `editorMeta`
    // test above passes even if this one's guard is deleted, because the two
    // fields arrive on different request fields.
    use crate::web_canvas_server::WriteBarrier;

    let barrier = WriteBarrier::default();
    barrier.close();
    let state = Mutex::new(multi_page_state());
    assert_eq!(
        state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .editor
            .ui
            .active_page_index,
        0,
        "the fixture starts on the first page"
    );

    let req = parse_standard_turn_body(&active_page_body("p2")).expect("request parses");
    let snapshot = apply_request_snapshot(&req, &state, &SseHub::default(), Some(&barrier))
        .expect("a page-switch turn still gets its reply");

    assert_eq!(
        snapshot.ui.active_page_index, 0,
        "the turn snapshot must not advertise a page the flush will not persist"
    );
    assert_eq!(
        state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .editor
            .ui
            .active_page_index,
        0,
        "a closed barrier must leave the persisted active page alone"
    );
}

#[test]
fn an_open_write_barrier_still_switches_the_active_page() {
    // The counterpart, so the guard above is proven to be what stops it rather
    // than the fixture simply being unable to switch pages at all.
    use crate::web_canvas_server::WriteBarrier;

    let barrier = WriteBarrier::default();
    let state = Mutex::new(multi_page_state());

    let req = parse_standard_turn_body(&active_page_body("p2")).expect("request parses");
    apply_request_snapshot(&req, &state, &SseHub::default(), Some(&barrier))
        .expect("an open barrier admits the switch");

    assert_eq!(
        state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .editor
            .ui
            .active_page_index,
        1,
        "an open barrier must apply the requested page switch"
    );
}
