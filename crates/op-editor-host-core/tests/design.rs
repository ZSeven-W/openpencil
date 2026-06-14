use std::sync::mpsc;
use std::thread;

use op_editor_core::{EditorCommand, EditorState};
use op_editor_host_core::design::{
    DesignCmdAck, DesignCmdOp, DesignCmdReq, DesignDelta, DesignSession, RemoteDocSink,
};
use op_orchestrator::{DocSink, Progress, RunSummary, SubtaskOutcome};

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
            }],
            total_nodes: 2,
        })))
        .expect("done");

    let (ack_tx, _ack_rx) = mpsc::sync_channel::<DesignCmdAck>(1);
    cmd_tx
        .send(DesignCmdReq {
            op: DesignCmdOp::Apply(EditorCommand::ClearSelection),
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
