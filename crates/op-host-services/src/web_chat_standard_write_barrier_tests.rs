//! Write-barrier (shutdown admission) regressions for the standard turn.
//!
//! A closed barrier must leave the document, editor metadata, starter
//! frame, and active page exactly as the flush will find them while the
//! reply still streams; an open barrier applies every write normally.
//!
//! Split out of `web_chat_standard_tests.rs` (800-line file ceiling).

use super::*;
use op_ai::chat_provider::{ChatDelta, ChatProvider, ChatRequest, StopReason};
use std::sync::Mutex;

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
    // `EditorMeta::from_state` persists `active_page_index` and
    // `preserve_authored_geometry`, so both need admission even when the
    // document is unchanged. Shutdown must leave the flushed snapshot intact.
    //
    // Skip incidental metadata writes so plain chat can still reply.
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
