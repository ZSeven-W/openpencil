use std::sync::mpsc;
use std::thread;

use op_editor_core::{EditorCommand, EditorState};
use op_editor_host_core::design::{
    DesignCmdAck, DesignCmdOp, DesignCmdReq, DesignDelta, DesignSession, RemoteDocSink,
};
use op_orchestrator::{AbortFlag, DocSink, Progress, RunSummary, SubtaskOutcome};

#[test]
fn remote_doc_sink_returns_false_when_ui_channel_closed() {
    let (tx, rx) = mpsc::channel::<DesignCmdReq>();
    let mut sink = RemoteDocSink::new(tx, EditorState::new());
    drop(rx);

    assert!(!sink.apply(EditorCommand::ClearSelection));
}

#[test]
fn remote_doc_sink_updates_mirror_on_ack() {
    let (tx, rx) = mpsc::channel::<DesignCmdReq>();
    let initial = EditorState::new();
    let mut sink = RemoteDocSink::new(tx, initial.clone());

    let ui_thread = thread::spawn(move || {
        let req = rx.recv().expect("request");
        assert_eq!(req.target_page_id.as_deref(), Some("0"));
        let mut new_state = initial.clone();
        new_state.viewport.zoom = 2.0;
        req.ack
            .send(DesignCmdAck {
                applied: true,
                new_state,
            })
            .expect("ack");
    });

    assert!(sink.apply(EditorCommand::ClearSelection));
    ui_thread.join().expect("ui thread");
    assert_eq!(sink.state().viewport.zoom, 2.0);
}

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
    drop(sink);

    assert_eq!(ui.join().expect("ui thread"), vec!["begin", "apply", "end"]);
}

#[test]
fn design_session_drains_progress_and_command_requests() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut session = DesignSession::from_channels(delta_rx, cmd_rx);

    delta_tx
        .send(DesignDelta::Progress(Progress::Planning))
        .expect("progress");
    delta_tx
        .send(DesignDelta::Done(Ok(RunSummary {
            root_frame_id: "root".into(),
            subtasks: vec![SubtaskOutcome {
                id: "s1".into(),
                node_count: 2,
                error: None,
                inserted_root_ids: Vec::new(),
                subtask: None,
            }],
            total_nodes: 2,
            unfilled_screens: Vec::new(),
        })))
        .expect("done");

    let (ack_tx, _ack_rx) = mpsc::sync_channel::<DesignCmdAck>(1);
    cmd_tx
        .send(DesignCmdReq {
            op: DesignCmdOp::Apply(EditorCommand::ClearSelection),
            target_page_id: None,
            ack: ack_tx,
        })
        .expect("cmd");

    let poll = session.poll_progress();
    assert_eq!(poll.progress.len(), 1);
    assert!(poll.summary.is_some());
    assert!(poll.finished);

    let reqs = session.drain_cmd_requests();
    assert_eq!(reqs.len(), 1);
}

#[test]
fn design_session_drains_progress_queued_after_done_before_finishing() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut session = DesignSession::from_channels(delta_rx, cmd_rx);

    delta_tx
        .send(DesignDelta::Progress(Progress::ValidationStarted))
        .expect("validation started");
    delta_tx
        .send(DesignDelta::Done(Ok(RunSummary {
            root_frame_id: "root".into(),
            subtasks: Vec::new(),
            total_nodes: 0,
            unfilled_screens: Vec::new(),
        })))
        .expect("done");
    delta_tx
        .send(DesignDelta::Progress(Progress::ValidationDone {
            total_applied: 0,
        }))
        .expect("validation done");

    let poll = session.poll_progress();

    assert!(poll.finished);
    assert!(poll.summary.is_some());
    assert_eq!(poll.progress.len(), 2);
    assert!(matches!(
        poll.progress.first(),
        Some(Progress::ValidationStarted)
    ));
    assert!(matches!(
        poll.progress.get(1),
        Some(Progress::ValidationDone { total_applied: 0 })
    ));
}

#[test]
fn design_session_preserves_worker_scoped_progress_context() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut session = DesignSession::from_channels(delta_rx, cmd_rx);
    let identity = op_orchestrator::agent_identity::AgentIdentity {
        color: "#4ECDC4".into(),
        name: "Mochi".into(),
    };
    delta_tx
        .send(DesignDelta::Progress(Progress::worker_scoped(
            2,
            "Profile",
            identity.clone(),
            Progress::SubtaskStarted {
                id: "profile-body".into(),
                label: "Profile body".into(),
            },
        )))
        .expect("worker progress send");

    let poll = session.poll_progress();

    assert_eq!(poll.progress.len(), 1);
    let Progress::WorkerScoped(worker) = &poll.progress[0] else {
        panic!("worker envelope must survive the design transport");
    };
    assert_eq!(worker.group_idx, 2);
    assert_eq!(worker.screen, "Profile");
    assert_eq!(worker.identity, identity);
    assert!(matches!(
        worker.event.as_ref(),
        Progress::SubtaskStarted { id, .. } if id == "profile-body"
    ));
}

#[test]
fn dropping_unfinished_design_session_sets_shared_abort_flag() {
    let (_delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let abort = AbortFlag::new();
    let session =
        DesignSession::from_channels_with_epoch_and_abort(delta_rx, cmd_rx, 0, abort.clone());

    assert!(!abort.is_set());
    drop(session);
    assert!(abort.is_set());
}

#[test]
fn explicit_design_session_abort_sets_shared_flag() {
    let (_delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let abort = AbortFlag::new();
    let session =
        DesignSession::from_channels_with_epoch_and_abort(delta_rx, cmd_rx, 0, abort.clone());

    session.abort();
    assert!(abort.is_set());
}

#[test]
fn dropping_naturally_finished_design_session_does_not_abort_worker() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let abort = AbortFlag::new();
    let mut session =
        DesignSession::from_channels_with_epoch_and_abort(delta_rx, cmd_rx, 0, abort.clone());
    delta_tx
        .send(DesignDelta::Done(Ok(RunSummary {
            root_frame_id: "root".into(),
            subtasks: Vec::new(),
            total_nodes: 0,
            unfilled_screens: Vec::new(),
        })))
        .expect("done");

    assert!(session.poll_progress().finished);
    drop(session);
    assert!(!abort.is_set());
}
