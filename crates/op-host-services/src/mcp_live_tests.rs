//! Live-MCP server tests. Split out of `mcp_live.rs` (inline `mod tests`)
//! to keep that file under the 800-line cap; still a child module of
//! `mcp_live`, so `super::` paths resolve unchanged.

use super::*;

#[test]
fn live_ping_response_carries_identity_mode_and_token() {
    let resp = live_ping_response("3", "abc-123");
    assert!(resp.contains(r#""server":"openpencil-mcp""#), "{resp}");
    assert!(resp.contains(r#""mode":"live""#), "{resp}");
    assert!(resp.contains(r#""token":"abc-123""#), "{resp}");
    assert!(resp.contains(r#""id":3"#), "{resp}");
}

#[test]
fn make_live_token_is_nonempty_and_structured() {
    let token = make_live_token();
    assert!(token.contains('-'), "{token}");
    assert!(!token.starts_with('-') && !token.ends_with('-'), "{token}");
}

#[test]
fn snapshot_request_wakes_ui_event_loop_after_queueing() {
    let (req_tx, req_rx) = mpsc::channel();
    let wake_count = Arc::new(AtomicUsize::new(0));
    let wake_ui: UiWake = {
        let wake_count = Arc::clone(&wake_count);
        Arc::new(move || {
            wake_count.fetch_add(1, Ordering::AcqRel);
        })
    };

    let requester = thread::spawn(move || request_snapshot(&req_tx, &wake_ui));
    let request = req_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("snapshot request should be queued");
    let wake_deadline = std::time::Instant::now() + Duration::from_secs(1);
    while wake_count.load(Ordering::Acquire) == 0 && std::time::Instant::now() < wake_deadline {
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(
        wake_count.load(Ordering::Acquire),
        1,
        "queueing a UI request must wake winit instead of relying on idle polling"
    );
    let UiRequest::Snapshot { ack } = request else {
        panic!("expected snapshot request");
    };
    ack.send(EditorState::new()).expect("ack snapshot");
    requester
        .join()
        .expect("requester thread should not panic")
        .expect("snapshot request should complete");
}

#[test]
fn set_active_page_fast_path_sends_apply_without_full_snapshot() {
    let (req_tx, req_rx) = mpsc::channel();
    let wake_ui: UiWake = Arc::new(|| {});
    let line = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"set_active_page","arguments":{"index":"1"}}}"#;

    let requester = thread::spawn(move || process_lightweight_live_tool(line, &req_tx, &wake_ui));
    let request = req_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("fast path should queue an apply request");
    let UiRequest::Apply {
        tool_name,
        cmd: EditorCommand::SetActivePage { index },
        ack,
    } = request
    else {
        panic!("set_active_page must not request a full EditorState snapshot");
    };
    assert_eq!(tool_name, "set_active_page");
    assert_eq!(index, 1);
    ack.send(ApplyAck { applied: true }).expect("ack apply");

    let response = requester
        .join()
        .expect("requester thread should not panic")
        .expect("fast dispatch should succeed")
        .expect("tool call should produce a response");
    assert!(response.contains(r#"\"wrote\":\"true\""#), "{response}");
}

#[test]
fn list_pages_fast_path_requests_only_page_metadata() {
    let (req_tx, req_rx) = mpsc::channel();
    let wake_ui: UiWake = Arc::new(|| {});
    let line = r#"{"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"list_pages","arguments":{}}}"#;

    let requester = thread::spawn(move || process_lightweight_live_tool(line, &req_tx, &wake_ui));
    let request = req_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("fast path should request page metadata");
    let UiRequest::ListPages { ack } = request else {
        panic!("list_pages must not request a full EditorState snapshot");
    };
    ack.send(op_mcp::ListPages {
        page_count: 70,
        active_page_index: 42,
        pages: vec![
            ("page-a".to_string(), "Intro".to_string()),
            ("page-b".to_string(), "Analysis".to_string()),
        ],
    })
    .expect("ack page metadata");

    let response = requester
        .join()
        .expect("requester thread should not panic")
        .expect("fast dispatch should succeed")
        .expect("tool call should produce a response");
    let result = crate::mcp_serve::tool_text(&response);
    let json: serde_json::Value = serde_json::from_str(&result).expect("list_pages result json");
    assert_eq!(json["pageCount"], 70);
    assert_eq!(json["activePageIndex"], 42);
    assert_eq!(json["pages"][1]["name"], "Analysis");
}

#[test]
fn pump_yields_after_ui_request_budget() {
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = McpLiveServer {
        port: 0,
        token: "test-token".to_string(),
        quit_flag: Arc::new(AtomicBool::new(false)),
        req_rx,
        stop_tx,
        client_identity: Arc::new(Mutex::new(None)),
        last_mcp_epoch: 0,
    };
    let mut state = EditorState::new();
    let mut acks = Vec::new();

    for index in 0..(UI_PUMP_REQUEST_BUDGET + 3) {
        let (ack_tx, ack_rx) = mpsc::sync_channel(1);
        req_tx
            .send(UiRequest::Apply {
                tool_name: "set_viewport".to_string(),
                cmd: EditorCommand::SetViewport {
                    pan_x: Some(index as i32),
                    pan_y: None,
                    zoom_percent: None,
                },
                ack: ack_tx,
            })
            .expect("queue apply request");
        acks.push(ack_rx);
    }

    let outcome = server.pump(&mut state);
    assert!(outcome.repaint);
    assert!(!outcome.layout_dirty);
    let acked = acks.iter().filter(|ack| ack.try_recv().is_ok()).count();
    assert_eq!(acked, UI_PUMP_REQUEST_BUDGET);
}

/// The UI pump renders queued screenshot requests off the live
/// state through the raster export pipeline — the scripted-channel
/// half of the live `debug_screenshot` roundtrip.
#[cfg(feature = "mcp-debug-tools")]
#[test]
fn pump_fulfills_screenshot_requests_off_the_live_state() {
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = McpLiveServer {
        port: 0,
        token: "test-token".to_string(),
        quit_flag: Arc::new(AtomicBool::new(false)),
        req_rx,
        stop_tx,
        client_identity: Arc::new(Mutex::new(None)),
        last_mcp_epoch: 0,
    };
    let mut state = EditorState::new();
    assert!(state.apply(EditorCommand::InsertNode {
        kind: "rect".into(),
        name: "Shot".into(),
        x: 5,
        y: 5,
        width: 20,
        height: 10,
        fill_hex: Some("#ff0000".into()),
        target_parent: op_editor_core::NodeId::NONE,
        page_id: None,
    }));

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Screenshot {
            spec: crate::export::screenshot::CaptureSpec {
                node_id: None,
                padding: 0.0,
                scale: 1.0,
            },
            ack: ack_tx,
        })
        .expect("queue screenshot request");
    let _ = server.pump(&mut state);
    let shot = ack_rx
        .try_recv()
        .expect("pump must ack the screenshot")
        .expect("capture must succeed for a filled rect");
    assert!(!shot.png_base64.is_empty());
    assert!(shot.actual_bounds.is_none(), "root capture has no bounds");
}

#[test]
fn pump_marks_layout_dirty_only_for_layout_affecting_commands() {
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = McpLiveServer {
        port: 0,
        token: "test-token".to_string(),
        quit_flag: Arc::new(AtomicBool::new(false)),
        req_rx,
        stop_tx,
        client_identity: Arc::new(Mutex::new(None)),
        last_mcp_epoch: 0,
    };
    let mut state = EditorState::new();
    state.selection.anchor = op_editor_core::NodeId::new("n1");
    state.selection.set = vec![op_editor_core::NodeId::new("n1")];

    let (selection_ack_tx, _selection_ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "clear_selection".to_string(),
            cmd: EditorCommand::ClearSelection,
            ack: selection_ack_tx,
        })
        .expect("queue selection request");
    let selection = server.pump(&mut state);
    assert!(selection.repaint);
    assert!(
        !selection.layout_dirty,
        "selection-only MCP commands must not rebuild layout"
    );

    let (insert_ack_tx, _insert_ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "insert_node".to_string(),
            cmd: EditorCommand::InsertNode {
                kind: "rect".to_string(),
                name: "Box".to_string(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
                target_parent: op_editor_core::NodeId::NONE,
                page_id: None,
            },
            ack: insert_ack_tx,
        })
        .expect("queue insert request");
    let insert = server.pump(&mut state);
    assert!(insert.repaint);
    assert!(
        insert.layout_dirty,
        "document-mutating MCP commands must rebuild layout"
    );
    assert!(
        !insert.document_replaced,
        "a plain Apply command is not a whole-document replace"
    );
}

/// `ReplaceDocument` swaps in a whole new `EditorState` and resets
/// `document_revision()` back to 0 (see `EditorState::replace_document`).
/// Callers that key a cache off `document_revision()` (the desktop
/// host's image-search scan gate) need an explicit signal to
/// distinguish this from an ordinary revision-bumping `Apply`, since a
/// fresh document's revision can alias a previously-scanned one.
#[test]
fn pump_flags_document_replaced_only_for_replace_document_requests() {
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = McpLiveServer {
        port: 0,
        token: "test-token".to_string(),
        quit_flag: Arc::new(AtomicBool::new(false)),
        req_rx,
        stop_tx,
        client_identity: Arc::new(Mutex::new(None)),
        last_mcp_epoch: 0,
    };
    let mut state = EditorState::new();
    let replacement_doc = op_pen_loader::load_canonical(
            r#"{"version":"1.0","children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[]}] }"#,
        )
        .expect("two-page replacement")
        .value;

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::ReplaceDocument {
            doc: Box::new(replacement_doc),
            editor_meta: op_pen_loader::EditorMeta {
                active_page_index: 1,
                preserve_authored_geometry: true,
            },
            ack: ack_tx,
        })
        .expect("queue replace-document request");
    let outcome = server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok(), "replace must ack");
    assert!(outcome.repaint);
    assert!(outcome.layout_dirty);
    assert_eq!(state.ui.active_page_index, 1);
    assert!(state.editor_ui.preserve_authored_geometry);
    assert!(
        outcome.document_replaced,
        "ReplaceDocument must flag document_replaced"
    );

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::UpdateEditorMeta {
            editor_meta: op_pen_loader::EditorMeta {
                active_page_index: 0,
                preserve_authored_geometry: false,
            },
            ack: ack_tx,
        })
        .expect("queue metadata-only request");
    let metadata = server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok(), "metadata update must ack");
    assert!(metadata.repaint);
    assert!(metadata.layout_dirty);
    assert!(!metadata.document_replaced);
    assert_eq!(state.ui.active_page_index, 0);
    assert!(!state.editor_ui.preserve_authored_geometry);
}

fn set_active_collaboration(state: &mut EditorState, role: op_editor_core::CollabUiRole) {
    assert!(state.editor_ui.collab.set_authenticated_session(
        op_editor_core::CollabConnectionPhase::Active,
        op_editor_core::AuthenticatedCollabSession {
            session_name: "Live session".to_string(),
            role,
            share_endpoint: None,
        },
        Vec::new(),
    ));
}

#[test]
fn pump_rejects_mcp_document_writes_during_active_collaboration() {
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();
    set_active_collaboration(&mut state, op_editor_core::CollabUiRole::Owner);
    let before = state.doc.clone();

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "insert_node".to_string(),
            cmd: EditorCommand::InsertNode {
                kind: "rect".to_string(),
                name: "Blocked".to_string(),
                x: 0,
                y: 0,
                width: 10,
                height: 10,
                fill_hex: None,
                target_parent: op_editor_core::NodeId::NONE,
                page_id: None,
            },
            ack: ack_tx,
        })
        .expect("queue MCP write");

    let outcome = server.pump(&mut state);
    assert!(!ack_rx.try_recv().expect("apply ack").applied);
    assert_eq!(state.doc, before);
    assert!(outcome.repaint);
    assert!(!outcome.layout_dirty);
    assert!(matches!(
        state.editor_ui.collab.notice.map(|notice| notice.kind),
        Some(op_editor_core::CollabNoticeKind::Reject(
            op_editor_core::CollabRejectUiCode::Unsupported
        ))
    ));
}

#[test]
fn pump_rejects_whole_document_sync_during_active_collaboration() {
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();
    set_active_collaboration(&mut state, op_editor_core::CollabUiRole::Owner);
    let before = state.doc.clone();
    let replacement = op_pen_loader::load_canonical(
        r#"{"version":"1.0","children":[],"pages":[{"id":"new","name":"New","children":[]}]}"#,
    )
    .expect("replacement")
    .value;

    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::ReplaceDocument {
            doc: Box::new(replacement),
            editor_meta: op_pen_loader::EditorMeta::default(),
            ack: ack_tx,
        })
        .expect("queue whole-document sync");

    let outcome = server.pump(&mut state);
    assert!(!ack_rx.try_recv().expect("replace ack"));
    assert_eq!(state.doc, before);
    assert!(!outcome.document_replaced);
    assert!(!outcome.layout_dirty);
    assert!(matches!(
        state.editor_ui.collab.notice.map(|notice| notice.kind),
        Some(op_editor_core::CollabNoticeKind::UnsupportedEdit(
            op_editor_core::CollabUnsupportedFeature::ReplaceDocument
        ))
    ));
}

// ── MCP-driven canvas generation indicators ─────────────────────────
//
// No dedicated lock around the shared `agent_indicators` registry —
// matches the existing convention in this crate
// (`design_agent_tools.rs`'s
// `execute_design_batch_design_registers_reveals_when_epoch_is_set`
// also touches it lock-free). Each test clears on entry AND exit so
// it starts and leaves the registry empty regardless of ordering.

fn fresh_mcp_server(req_rx: Receiver<UiRequest>, stop_tx: Sender<()>) -> McpLiveServer {
    McpLiveServer {
        port: 0,
        token: "test-token".to_string(),
        quit_flag: Arc::new(AtomicBool::new(false)),
        req_rx,
        stop_tx,
        client_identity: Arc::new(Mutex::new(None)),
        last_mcp_epoch: 0,
    }
}

/// A one-frame-one-child subtree, JSON-built (same shape as
/// `design_agent_tools.rs`'s own `batch_design` reveal test) so the
/// insert produces both a fresh top-level root AND a fresh
/// descendant — the frame tag and the reveal path are two distinct
/// code branches in `register_mcp_generation` and this exercises both.
fn root_frame_with_child(root_id: &str, child_id: &str) -> PenNode {
    serde_json::from_str(&format!(
            r#"{{"type":"frame","id":"{root_id}","name":"Root","width":200,"height":200,
                "children":[{{"type":"rectangle","id":"{child_id}","name":"Box","width":50,"height":50}}]}}"#
        ))
        .expect("valid frame json")
}

fn send_batch_design_insert(req_tx: &Sender<UiRequest>, node: PenNode) -> mpsc::Receiver<ApplyAck> {
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "batch_design".to_string(),
            // `InsertAuthoredSubtree`, not `InsertSubtree` — the
            // latter remaps every incoming id to a fresh editor id
            // (`command.rs`'s own doc: "the caller's ids are
            // structural placeholders only"), which would make this
            // test's assertions target whatever id landed rather
            // than the one it authored. Doesn't change what's under
            // test: `register_mcp_generation` diffs ids generically
            // and doesn't care which `EditorCommand` variant ran.
            cmd: EditorCommand::InsertAuthoredSubtree {
                nodes: vec![node],
                parent_id: op_editor_core::NodeId::NONE,
                page_id: None,
            },
            ack: ack_tx,
        })
        .expect("queue batch_design apply");
    ack_rx
}

#[test]
fn batch_design_apply_populates_the_scan_gate_frames_and_reveals() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();

    let ack_rx = send_batch_design_insert(&req_tx, root_frame_with_child("root-a", "box-a"));
    let outcome = server.pump(&mut state);
    assert!(ack_rx.try_recv().is_ok_and(|ack| ack.applied));
    assert!(outcome.repaint);

    // The canvas radar-scan (`canvas_generation_scan::generating_paint_sets`)
    // gates purely on `AgentIndicators.frames` being non-empty — this is
    // the exact fact the bug report needed and the CLI-provider path
    // already has covered (`chat_intent_host_tests.rs`'s
    // `cli_new_design_populates_frame_indicators_the_canvas_scan_gates_on`);
    // this is the MCP-tool-driven counterpart.
    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert!(
        snapshot.frames.contains_key("root-a"),
        "the new top-level root must be tagged: {:?}",
        snapshot.frames
    );
    assert!(
        snapshot.reveals.contains_key("root-a") && snapshot.reveals.contains_key("box-a"),
        "both the new root and its new child must have entrance reveals: {:?}",
        snapshot.reveals
    );

    op_editor_core::agent_indicators::clear();
}

#[test]
fn batch_design_apply_is_a_noop_when_nothing_new_landed() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();

    // A `batch_design` call whose command carries an EMPTY subtree —
    // shaped like a Direct-op property tweak that inserts nothing new.
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    req_tx
        .send(UiRequest::Apply {
            tool_name: "batch_design".to_string(),
            cmd: EditorCommand::InsertSubtree {
                nodes: vec![],
                parent_id: op_editor_core::NodeId::NONE,
                page_id: None,
            },
            ack: ack_tx,
        })
        .expect("queue batch_design apply");
    let _ = server.pump(&mut state);
    let _ = ack_rx.try_recv();

    assert_eq!(
        op_editor_core::agent_indicators::snapshot().frames.len(),
        0,
        "an apply that produced no new content must not begin an epoch at all"
    );
    assert_eq!(server.last_mcp_epoch, 0, "no epoch was ever minted");

    op_editor_core::agent_indicators::clear();
}

/// Locks in Track "batch_design 单批自成一轮 + 漂移窗口合并" — the whole
/// value of that design over the simpler "every call gets its own
/// fresh epoch" alternative: a second `batch_design` apply that lands
/// while the FIRST one's reveal queue is still draining must extend
/// the same epoch, not `begin()` a fresh one — `begin()` unconditionally
/// clears every map (`AgentIndicators::clear_maps`), so if this
/// coalescing didn't happen the first root's frame tag would be wiped
/// the instant the second call landed.
#[test]
fn back_to_back_batch_design_calls_coalesce_into_one_epoch_while_reveals_drain() {
    let _guard = op_editor_core::agent_indicators::test_guard();
    op_editor_core::agent_indicators::clear();
    let (req_tx, req_rx) = mpsc::channel();
    let (stop_tx, _stop_rx) = mpsc::channel();
    let mut server = fresh_mcp_server(req_rx, stop_tx);
    let mut state = EditorState::new();

    let first_ack = send_batch_design_insert(&req_tx, root_frame_with_child("root-a", "box-a"));
    server.pump(&mut state);
    assert!(first_ack.try_recv().is_ok_and(|ack| ack.applied));
    let epoch_after_first = server.last_mcp_epoch;
    assert_ne!(epoch_after_first, 0, "first call must have minted an epoch");
    // The first call's own reveal queue must still be mid-flight for
    // this test to prove anything — it always is here (`REVEAL_DURATION_MS`
    // is a full second; this whole test runs in microseconds).
    assert!(
        op_editor_core::agent_indicators::latest_reveal_end_ms(epoch_after_first)
            .is_some_and(|end_ms| reveal_now_millis_for_test() < end_ms),
        "test precondition: the first batch's reveal queue must still be draining"
    );

    // Second call, different root, arriving inside that drain window.
    let second_ack = send_batch_design_insert(&req_tx, root_frame_with_child("root-b", "box-b"));
    server.pump(&mut state);
    assert!(second_ack.try_recv().is_ok_and(|ack| ack.applied));

    assert_eq!(
        server.last_mcp_epoch, epoch_after_first,
        "the second call must REUSE the first call's still-draining epoch, not begin() a fresh one"
    );
    let snapshot = op_editor_core::agent_indicators::snapshot();
    assert!(
        snapshot.frames.contains_key("root-a") && snapshot.frames.contains_key("root-b"),
        "both roots must coexist in the SAME registry snapshot — if the second \
             call had begun a fresh epoch instead, begin()'s clear_maps() would have \
             wiped root-a's tag the instant root-b's call landed: {:?}",
        snapshot.frames
    );
    assert!(
        snapshot.reveals.contains_key("box-a") && snapshot.reveals.contains_key("box-b"),
        "both batches' reveals must coexist too: {:?}",
        snapshot.reveals
    );

    op_editor_core::agent_indicators::clear();
}

fn reveal_now_millis_for_test() -> u64 {
    crate::design_agent_tools::reveal_now_millis()
}
