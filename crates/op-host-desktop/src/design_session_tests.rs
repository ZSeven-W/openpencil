use super::*;
use op_editor_core::{EditorCommand, EditorState, Locale};
use op_editor_host_core::design::{DesignCmdReq, DesignDelta, RemoteDocSink};
use op_host_services::design_session::{
    active_content_bounds, design_canvas_size, fit_design_viewport_to_content,
};
use op_orchestrator::{DocSink, RunSummary, SubtaskOutcome};
use std::sync::mpsc;
use std::thread;
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
/// assert that the chat bubble carries ordered narration, typed activity and
/// structured terminal metadata, and that the session clears itself
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
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
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
                inserted_root_ids: Vec::new(),
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
        let _ = pump_progress(&mut host, &mut current, None);
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
        bubble.thinking.is_empty(),
        "typed progress stays out of thinking"
    );
    assert!(bubble.content.contains("mapping the request"));
    assert!(bubble.content.contains("Done —"));
    assert_eq!(bubble.activities.len(), 1);
    assert_eq!(bubble.activities[0].title, "Planning the design");
    assert!(bubble.activities[0].content_offset.is_some());
    assert_eq!(
        bubble.activities[0].status,
        op_editor_core::ChatActivityStatus::Done
    );
    assert_eq!(
        bubble.completion,
        Some(op_editor_core::ChatCompletion {
            succeeded: 1,
            failed: 0,
            nodes: 3,
        })
    );
    assert!(
        !bubble.content.contains("subtask(s) succeeded"),
        "completion metadata must not depend on parsing the visible summary"
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
                page_id: None,
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

#[test]
fn typed_progress_merges_one_subtask_and_hides_scheduler_details() {
    use op_orchestrator::{Progress, SkillBrief};
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    let events = vec![
        Progress::Planned {
            subtasks: vec![("header".into(), "Greeting header".into())],
        },
        Progress::SubtaskStarted {
            id: "header".into(),
            label: "Greeting header".into(),
        },
        Progress::SubtaskSkills {
            id: "header".into(),
            included: vec![SkillBrief {
                name: "mobile-app".into(),
                token_count: 600,
                truncated: false,
            }],
            dropped: vec![("examples".into(), "budget".into())],
            budget_used: 5200,
            budget_max: 8000,
        },
        Progress::SubtaskNodes {
            id: "header".into(),
            nodes_so_far: 3,
        },
        Progress::SubtaskDone {
            id: "header".into(),
            node_count: 3,
        },
    ];

    assert!(super::apply_progress(&mut message, &events, Locale::EnUs));
    assert_eq!(message.activities.len(), 1);
    assert_eq!(message.activities[0].title, "Greeting header");
    assert_eq!(message.activities[0].detail.as_deref(), Some("3 elements"));
    assert!(message.activities[0].content_offset.is_some());
    assert_eq!(
        message.activities[0].status,
        op_editor_core::ChatActivityStatus::Done
    );
    assert!(message.thinking.is_empty());
    let visible = format!(
        "{} {:?}",
        message.activities[0].title, message.activities[0].detail
    );
    for internal in ["skills", "5200/8000", "dropped", "examples"] {
        assert!(
            !visible.contains(internal),
            "leaked internal field: {internal}"
        );
    }
    assert!(message.content.contains("mapped the page into 1 section"));
}

#[test]
fn cli_progress_builds_an_ordered_narration_timeline_and_plain_summary() {
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    assert!(super::apply_progress(
        &mut message,
        &[
            Progress::Planning,
            Progress::Planned {
                subtasks: vec![("header".into(), "Greeting header".into())],
            },
            Progress::CleanupDone,
            Progress::ValidationStarted,
            Progress::ValidationDone { total_applied: 0 },
        ],
        Locale::EnUs,
    ));

    let offsets: Vec<usize> = message
        .activities
        .iter()
        .map(|activity| activity.content_offset.expect("timeline offset") as usize)
        .collect();
    assert!(offsets.windows(2).all(|pair| pair[0] <= pair[1]));
    assert!(message.content.contains("mapping the request"));
    assert!(message.content.contains("polishing spacing"));
    assert!(message.content.contains("checking overflow"));
    assert!(!message.content.contains("skills"));

    assert!(super::append_completion_narration(
        &mut message,
        1,
        0,
        Locale::EnUs
    ));
    assert!(message.content.ends_with(
        "Done — the planned section is in place and the final layout has been checked."
    ));
}

#[test]
fn typed_progress_updates_retry_without_duplicating_the_activity() {
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    let events = vec![
        Progress::SubtaskStarted {
            id: "header".into(),
            label: "Greeting header".into(),
        },
        Progress::SubtaskRetry {
            id: "header".into(),
            attempt: 2,
            reason: "zero nodes generated".into(),
        },
    ];

    assert!(super::apply_progress(&mut message, &events, Locale::EnUs));
    assert_eq!(message.activities.len(), 1);
    assert_eq!(
        message.activities[0].detail.as_deref(),
        Some("Retrying · attempt 2")
    );
    assert_eq!(
        message.activities[0].status,
        op_editor_core::ChatActivityStatus::Running
    );
    assert!(!format!("{:?}", message.activities).contains("zero nodes"));
}

#[test]
fn cli_progress_uses_the_editor_locale_for_visible_process_and_summary() {
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    assert!(super::apply_progress(
        &mut message,
        &[
            Progress::Planning,
            Progress::Planned {
                subtasks: vec![("header".into(), "问候页头".into())],
            },
            Progress::CleanupDone,
            Progress::ValidationStarted,
            Progress::ValidationPreCheckDone {
                applied: 0,
                by_category: Default::default(),
            },
            Progress::ValidationDone { total_applied: 0 },
        ],
        Locale::ZhCn,
    ));
    assert!(super::append_completion_narration(
        &mut message,
        1,
        0,
        Locale::ZhCn,
    ));

    assert!(message.content.contains("需求整理成清晰的页面结构"));
    assert!(message.content.contains("润色间距"));
    assert!(message.content.contains("检查溢出"));
    assert!(message.content.contains("已完成——"));
    assert!(message
        .activities
        .iter()
        .any(|activity| activity.title == "检查设计"));
    assert!(!message.content.contains("Planning"));
    assert!(!message.content.contains("Done"));
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
