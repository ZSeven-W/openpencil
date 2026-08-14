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
                subtask: None,
            }],
            total_nodes: 3,
            unfilled_screens: Vec::new(),
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

/// Failed-subtask remediation (manual layer): a `RunSummary` outcome
/// carrying a zero-node failure's persisted `subtask` must land on
/// `ChatMessage.failed_subtasks`, JSON-encoded, keyed by the same id the
/// row's `ChatActivity` carries — the exact lookup
/// `ChatState::begin_subtask_retry` performs on a "Retry" click.
#[test]
fn pump_progress_captures_failed_subtask_specs_for_manual_retry() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant_streaming());

    let subtask = op_orchestrator::plan::Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: op_orchestrator::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    };
    let fake_worker = thread::spawn(move || {
        let sink = RemoteDocSink::new(cmd_tx, EditorState::new());
        let _ = delta_tx.send(DesignDelta::Done(Ok(RunSummary {
            root_frame_id: "root".into(),
            subtasks: vec![SubtaskOutcome {
                id: "hero".into(),
                node_count: 0,
                error: Some("empty content from provider".into()),
                inserted_root_ids: Vec::new(),
                subtask: Some(subtask),
            }],
            total_nodes: 0,
            unfilled_screens: Vec::new(),
        })));
        sink
    });

    let deadline = Instant::now() + Duration::from_secs(5);
    while current.is_some() && Instant::now() < deadline {
        let _ = pump_commands(&mut host, &mut current, 1440.0, 900.0);
        let _ = pump_progress(&mut host, &mut current, None);
        if current.is_none() {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    let _ = fake_worker.join().expect("fake worker exits cleanly");

    let msg = host
        .editor_state()
        .chat
        .messages
        .last()
        .expect("seeded bubble survives");
    assert_eq!(msg.activities.len(), 1);
    assert_eq!(msg.activities[0].title, "Hero");
    assert_eq!(
        msg.activities[0].detail.as_deref(),
        Some("Reason: empty content from provider")
    );
    assert_eq!(msg.failed_subtasks.len(), 1, "{:?}", msg.failed_subtasks);
    assert_eq!(msg.failed_subtasks[0].subtask_id, "hero");
    let restored: op_orchestrator::plan::Subtask =
        serde_json::from_str(&msg.failed_subtasks[0].subtask_json)
            .expect("persisted spec must round-trip as valid Subtask JSON");
    assert_eq!(restored.label, "Hero");
    assert_eq!(restored.region.width, 1200.0);
}

fn persisted_subtask_json() -> String {
    serde_json::to_string(&op_orchestrator::plan::Subtask {
        id: "hero".into(),
        label: "Hero".into(),
        region: op_orchestrator::plan::Region {
            width: 1200.0,
            height: 400.0,
        },
        id_prefix: "hero".into(),
        parent_frame_id: None,
        elements: None,
        screen: None,
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    })
    .unwrap()
}

fn persisted_request_json() -> String {
    serde_json::to_string(&op_orchestrator::DesignRequest {
        prompt: "p".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    })
    .unwrap()
}

#[test]
fn launch_subtask_retry_if_pending_is_a_noop_without_a_pending_flag() {
    let mut host = WidgetHostNative::new();
    let mut current_design = None;
    assert!(!launch_subtask_retry_if_pending(
        &mut host,
        &mut current_design
    ));
    assert!(current_design.is_none());
}

/// The default selected agent (index 0, Claude Code CLI subprocess)
/// constructs unconditionally, so an out-of-range `chat_selected_agent` is
/// the only SAFE way to force "no provider" in a unit test — using the real
/// default provider here would spawn an actual `claude` subprocess as a test
/// side effect. The launch must write an honest inline error (not silently
/// do nothing) and consume the pending flag either way.
#[test]
fn launch_subtask_retry_if_pending_writes_an_error_when_no_model_is_configured() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.chat_selected_agent = 99;
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant("done"));
    {
        let msg = &mut host.editor_state_mut().chat.messages[0];
        msg.design_request_json_for_retry = Some(persisted_request_json());
        msg.failed_subtasks
            .push(op_editor_core::PendingSubtaskRetry {
                subtask_id: "hero".into(),
                subtask_json: persisted_subtask_json(),
            });
    }
    host.editor_state_mut().chat.pending_subtask_retry = Some((0, "hero".into()));
    let mut current_design = None;

    let changed = launch_subtask_retry_if_pending(&mut host, &mut current_design);

    assert!(changed);
    assert!(
        current_design.is_none(),
        "no provider configured — no session should launch"
    );
    assert!(
        host.editor_state().chat.pending_subtask_retry.is_none(),
        "the flag must be consumed even on the error path"
    );
    assert!(
        host.editor_state().chat.messages[0]
            .content
            .contains("no model configured"),
        "{}",
        host.editor_state().chat.messages[0].content
    );
}

/// A retry click with no matching `failed_subtasks` entry (e.g. a stale
/// flag from a message that was later pruned) is a silent no-op — nothing
/// was ever promised for that id.
#[test]
fn launch_subtask_retry_if_pending_noops_when_the_subtask_id_has_no_persisted_spec() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(op_editor_core::ChatMessage::assistant("done"));
    host.editor_state_mut().chat.messages[0].design_request_json_for_retry =
        Some(persisted_request_json());
    host.editor_state_mut().chat.pending_subtask_retry = Some((0, "not-persisted".into()));
    let mut current_design = None;

    let changed = launch_subtask_retry_if_pending(&mut host, &mut current_design);

    assert!(changed, "the flag is still consumed");
    assert!(current_design.is_none());
    assert_eq!(
        host.editor_state().chat.messages[0].content,
        "done",
        "no error text is appended for a stale/unmatched id"
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
            target_page_id: None,
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
fn pump_commands_keeps_a_design_turn_bound_to_its_start_page() {
    use jian_ops_schema::page::PenPage;

    let (_delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().doc.children.clear();
    host.editor_state_mut().doc.pages = Some(vec![
        PenPage {
            id: "page-design".into(),
            name: "Design".into(),
            children: Vec::new(),
            background_color: None,
            state: None,
            lifecycle: None,
        },
        PenPage {
            id: "page-user".into(),
            name: "User".into(),
            children: Vec::new(),
            background_color: None,
            state: None,
            lifecycle: None,
        },
    ]);
    host.editor_state_mut().ui.active_page_index = 1;

    let (ack_tx, ack_rx) = mpsc::sync_channel::<DesignCmdAck>(1);
    cmd_tx
        .send(DesignCmdReq {
            op: DesignCmdOp::Apply(EditorCommand::InsertSubtree {
                nodes: vec![mobile_root()],
                parent_id: op_editor_core::NodeId::NONE,
                page_id: None,
            }),
            target_page_id: Some("page-design".into()),
            ack: ack_tx,
        })
        .expect("request should queue");

    assert!(pump_commands(&mut host, &mut current, 1440.0, 900.0));
    let ack = ack_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("pump should ack apply request");
    assert!(ack.applied);
    assert_eq!(ack.new_state.ui.active_page_index, 0);
    let pages = host.editor_state().doc.pages.as_ref().unwrap();
    assert_eq!(pages[0].children.len(), 1);
    assert!(pages[1].children.is_empty());
    assert_eq!(host.editor_state().ui.active_page_index, 1);
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
fn failed_subtask_keeps_the_actionable_error_in_the_activity() {
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    assert!(super::apply_progress(
        &mut message,
        &[
            Progress::SubtaskStarted {
                id: "hero".into(),
                label: "Hero".into(),
            },
            Progress::SubtaskFailed {
                id: "hero".into(),
                error: "InsertSubtree rejected: parent_id=root status=missing".into(),
            },
        ],
        Locale::EnUs,
    ));
    let detail = message.activities[0].detail.as_deref().unwrap();
    assert!(detail.starts_with("Reason:"), "{detail}");
    assert!(detail.contains("parent_id=root status=missing"), "{detail}");
}

#[test]
fn failed_subtask_uses_localized_reason_label() {
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    assert!(super::apply_progress(
        &mut message,
        &[Progress::SubtaskFailed {
            id: "customer-table".into(),
            error: "parent_id=dashboard was not found".into(),
        }],
        Locale::ZhCn,
    ));

    assert_eq!(
        message.activities[0].detail.as_deref(),
        Some("失败原因：parent_id=dashboard was not found")
    );
}

#[test]
fn failed_subtask_without_provider_detail_reports_the_missing_diagnostic() {
    assert_eq!(
        super::subtask_failure_detail(Locale::ZhCn, " \n\t"),
        "Agent 执行失败，但没有返回错误说明。"
    );
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
