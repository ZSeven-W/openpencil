use super::*;
use op_editor_core::EditorCommand;
use op_orchestrator::{RunSummary, SubtaskOutcome};
use std::time::{Duration, Instant};

/// `RemoteDocSink::apply` blocks until UI acks. When the UI side
/// drops the receiver, `apply` returns false instead of hanging.
#[test]
fn remote_doc_sink_returns_false_when_ui_channel_closed() {
    let (tx, rx) = mpsc::channel::<DesignCmdReq>();
    let mut sink = RemoteDocSink::new(tx, EditorState::new());
    drop(rx); // simulate UI session dropped before the worker called apply
    let applied = sink.apply(EditorCommand::ClearSelection);
    assert!(!applied, "apply on closed channel must return false");
}

/// Happy-path round-trip: worker sends an apply request; UI thread
/// acks with an updated state snapshot; worker's mirror reflects it.
#[test]
fn remote_doc_sink_updates_mirror_on_ack() {
    let (tx, rx) = mpsc::channel::<DesignCmdReq>();
    let initial = EditorState::new();
    let mut sink = RemoteDocSink::new(tx, initial.clone());

    // Spawn UI-side faker that acks one request with a modified state.
    let ui_thread = thread::spawn(move || {
        let req = rx.recv().expect("worker should send one request");
        let mut new_state = initial.clone();
        // Mutate something the test can observe — viewport zoom.
        new_state.viewport.zoom = 2.0;
        let ack = DesignCmdAck {
            applied: true,
            new_state,
        };
        req.ack.send(ack).expect("ack must reach worker");
    });

    let applied = sink.apply(EditorCommand::ClearSelection);
    ui_thread.join().expect("ui thread must finish");
    assert!(applied, "ack reported applied=true");
    assert_eq!(
        sink.state().viewport.zoom,
        2.0,
        "mirror should reflect ack snapshot"
    );
}

/// `BeginUndoBatch` and `EndUndoBatch` are forwarded as their own
/// `DesignCmdOp` variants so the UI can route them through the
/// real `History::begin_batch` / `end_batch` once wired.
#[test]
fn undo_batch_signals_are_distinguishable_on_the_wire() {
    let (tx, rx) = mpsc::channel::<DesignCmdReq>();
    let mut sink = RemoteDocSink::new(tx, EditorState::new());
    let ui = thread::spawn(move || {
        let mut kinds = Vec::new();
        while let Ok(req) = rx.recv() {
            let label = match req.op {
                DesignCmdOp::Apply(_) => "apply",
                DesignCmdOp::BeginUndoBatch => "begin",
                DesignCmdOp::EndUndoBatch => "end",
            };
            kinds.push(label.to_string());
            let _ = req.ack.send(DesignCmdAck {
                applied: true,
                new_state: EditorState::new(),
            });
        }
        kinds
    });
    sink.begin_undo_batch();
    sink.apply(EditorCommand::ClearSelection);
    sink.end_undo_batch();
    drop(sink); // close the channel so the ui-side recv loop exits
    let kinds = ui.join().expect("ui thread finishes");
    assert_eq!(kinds, vec!["begin", "apply", "end"]);
}

/// End-to-end smoke through `pump_commands` + `pump_progress`:
/// a fake worker thread drives a `RemoteDocSink` against
/// real-looking channels, the UI loop drains both pumps, and we
/// assert that the chat bubble carries the rendered progress +
/// terminal summary line, and that the session clears itself
/// after `Done`.
///
/// This is the host-side complement to the orchestrator's own
/// end-to-end tests — it exercises the actor seam without
/// requiring an `agent::Provider` / `ANTHROPIC_API_KEY`. Task #28
/// covers the live LLM smoke separately.
#[test]
fn end_to_end_pump_round_trips_apply_and_progress_via_actor_channels() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    // Seed a streaming assistant bubble — `chat.begin_send`
    // creates one in production; the pumps fold the worker's
    // progress + summary into it.
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());

    // Fake worker — emits one progress event, asks UI to apply
    // ClearSelection, then a successful `Done`.
    let fake_worker = thread::spawn(move || {
        // Progress first so the bubble starts streaming text
        // before the doc mutation.
        let _ = delta_tx.send(DesignDelta::Progress(Progress::Planning));
        let mut sink = RemoteDocSink::new(cmd_tx, EditorState::new());
        sink.apply(EditorCommand::ClearSelection);
        let _ = delta_tx.send(DesignDelta::Done(Ok(RunSummary {
            root_frame_id: "root".into(),
            subtasks: vec![SubtaskOutcome {
                id: "s1".into(),
                node_count: 3,
                error: None,
            }],
            total_nodes: 3,
        })));
        // Hold the sink so its channel survives until the UI has
        // had a chance to drain (the test polls until `Done`).
        sink
    });

    // UI drives the pumps until the session clears (mirrors the
    // event-loop `RedrawRequested` block). Bound the loop with a
    // timeout so a hung worker fails the test instead of hanging.
    let deadline = Instant::now() + Duration::from_secs(5);
    while current.is_some() && Instant::now() < deadline {
        let _ = pump_commands(&mut host, &mut current, 1440.0, 900.0);
        let _ = pump_progress(&mut host, &mut current);
        if current.is_none() {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    // Worker can join now — the sink it returned (and thus its
    // cmd_tx) drops at this scope end.
    let _ = fake_worker.join().expect("fake worker exits cleanly");

    assert!(
        current.is_none(),
        "session must clear after Done — leaving it set would keep the\
         event loop ticking and pump_progress retrying"
    );
    let bubble = host
        .editor_state()
        .chat
        .messages
        .last()
        .expect("seeded bubble survives");
    assert!(
        bubble.thinking.contains("Planning"),
        "progress line should render in the process block, got thinking={:?}",
        bubble.thinking
    );
    assert!(
        !bubble.content.contains("Planning"),
        "final answer should not be polluted by progress lines, got: {:?}",
        bubble.content
    );
    assert!(
        bubble.content.contains("1 subtask"),
        "summary should report 1 subtask succeeded, got: {:?}",
        bubble.content
    );
    assert!(
        !bubble.streaming,
        "summary path must clear streaming so the chat panel stops the animation"
    );
}

#[test]
fn fit_design_viewport_centers_and_fits_mobile_root() {
    let mut state = EditorState::new();
    state.doc.children = vec![mobile_root()];

    assert!(fit_design_viewport_to_content(&mut state, 1440.0, 900.0));

    let bounds = active_content_bounds(&state).expect("root bounds");
    let (canvas_w, canvas_h) = design_canvas_size(&state, 1440.0, 900.0);
    let left = state.viewport.pan_x + bounds.x as f32 * state.viewport.zoom;
    let top = state.viewport.pan_y + bounds.y as f32 * state.viewport.zoom;
    let right = left + bounds.w as f32 * state.viewport.zoom;
    let bottom = top + bounds.h as f32 * state.viewport.zoom;
    let center_x = (left + right) / 2.0;
    let center_y = (top + bottom) / 2.0;

    assert!(left >= 0.0, "left edge should be visible, got {left}");
    assert!(top >= 0.0, "top edge should be visible, got {top}");
    assert!(
        right <= canvas_w,
        "right edge should be visible: {right} > {canvas_w}"
    );
    assert!(
        bottom <= canvas_h,
        "bottom edge should be visible: {bottom} > {canvas_h}"
    );
    assert!((center_x - canvas_w / 2.0).abs() < 0.5);
    assert!((center_y - canvas_h / 2.0).abs() < 0.5);
}

#[test]
fn pump_commands_refits_viewport_after_design_insert() {
    let (_delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().doc.children.clear();
    let before = host.editor_state().viewport;

    let (ack_tx, ack_rx) = mpsc::sync_channel::<DesignCmdAck>(1);
    cmd_tx
        .send(DesignCmdReq {
            op: DesignCmdOp::Apply(EditorCommand::InsertSubtree {
                nodes: vec![mobile_root()],
                parent_id: op_editor_core::NodeId::NONE,
            }),
            ack: ack_tx,
        })
        .expect("request should queue");

    assert!(pump_commands(&mut host, &mut current, 1440.0, 900.0));
    let ack = ack_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("pump should ack apply request");
    assert!(ack.applied);
    assert!(
        !ack.new_state.doc.children.is_empty(),
        "ack snapshot should include inserted root"
    );
    assert_eq!(
        host.editor_state().doc.children.len(),
        1,
        "host state should receive inserted root"
    );

    let after = host.editor_state().viewport;
    assert_ne!(before, after, "design insert should refit viewport");
    assert!(
        (after.zoom - 0.905).abs() < 0.01,
        "mobile root should fit viewport height, got zoom {}",
        after.zoom
    );
}

#[test]
fn fit_design_viewport_uses_resolved_layout_for_fit_content_root() {
    let mut state = EditorState::new();
    state.doc.children = vec![mobile_fit_content_root()];

    assert!(fit_design_viewport_to_content(&mut state, 1440.0, 900.0));

    let bounds = active_content_bounds(&state).expect("resolved root bounds");
    assert!(
        (bounds.h - 844.0).abs() < 1.0,
        "fit_content root should resolve to full mobile height, got {}",
        bounds.h
    );
    assert!(
        (state.viewport.zoom - 0.905).abs() < 0.01,
        "full mobile root should remain fully visible, got zoom {}",
        state.viewport.zoom
    );
}

fn mobile_root() -> jian_ops_schema::node::PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "x": 80,
        "y": 40,
        "width": 390,
        "height": 844,
        "children": []
    }))
    .expect("mobile root fixture parses")
}

fn mobile_fit_content_root() -> jian_ops_schema::node::PenNode {
    serde_json::from_value(serde_json::json!({
        "type": "frame",
        "id": "root",
        "name": "Mobile Root",
        "x": 80,
        "y": 40,
        "width": 390,
        "height": "fit_content",
        "layout": "vertical",
        "gap": 0,
        "children": [
            {"type": "frame", "id": "status", "name": "Status Bar", "width": "fill_container", "height": 32},
            {"type": "frame", "id": "header", "name": "Header", "width": "fill_container", "height": 92},
            {"type": "frame", "id": "search", "name": "Search", "width": "fill_container", "height": 104},
            {"type": "frame", "id": "promo", "name": "Promo", "width": "fill_container", "height": 132},
            {"type": "frame", "id": "categories", "name": "Categories", "width": "fill_container", "height": 86},
            {"type": "frame", "id": "restaurants", "name": "Restaurants", "width": "fill_container", "height": 314},
            {"type": "frame", "id": "bottom-nav", "name": "Bottom Nav", "width": "fill_container", "height": 84}
        ]
    }))
    .expect("fit_content mobile root fixture parses")
}
