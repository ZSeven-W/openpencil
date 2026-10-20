//! Account-lifetime guards discard queued Hosted output before UI mutation.

use op_ai::chat_provider::{ChatDelta, StopReason};
use op_editor_core::{ChatMessage, EditorCommand};
use op_editor_host_core::design::{DesignCmdOp, DesignCmdReq, DesignDelta, DesignSession};
use op_host_native::WidgetHostNative;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

#[test]
fn local_provider_lifetimes_do_not_depend_on_sso() {
    assert!(super::hosted_epoch_is_current(None));
    assert!(!super::hosted_epoch_is_current(Some(0)));
}

#[test]
fn invalidated_hosted_chat_drops_queued_text_and_stops_worker() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());
    let (tx, rx) = mpsc::channel();
    tx.send(ChatDelta::TextDelta("stale account output".into()))
        .unwrap();
    tx.send(ChatDelta::Done {
        stop_reason: StopReason::EndTurn,
    })
    .unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let mut current = Some(
        super::ChatSession::from_channels_with_cancel(rx, None, cancel.clone())
            .with_hosted_session_epoch(Some(0)),
    );

    assert!(super::pump(
        &mut host,
        &mut current,
        None,
        None,
        (800.0, 600.0)
    ));
    assert!(current.is_none());
    assert!(cancel.load(Ordering::Relaxed));
    let reply = host.editor_state().chat.messages.last().unwrap();
    assert!(reply.content.is_empty());
    assert!(!reply.streaming);
}

#[test]
fn invalidated_hosted_design_never_acknowledges_queued_mutations() {
    let mut host = WidgetHostNative::new();
    let selected = op_editor_core::NodeId::new("selected-before-logout");
    host.editor_state_mut()
        .set_single_selection(selected.clone());
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());
    let (_delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (ack_tx, ack_rx) = mpsc::sync_channel(1);
    cmd_tx
        .send(DesignCmdReq {
            op: DesignCmdOp::Apply(EditorCommand::ClearSelection),
            target_page_id: None,
            ack: ack_tx,
        })
        .unwrap();
    let mut current =
        Some(DesignSession::from_channels(delta_rx, cmd_rx).with_hosted_session_epoch(Some(0)));

    assert!(!crate::design_session::pump_commands(
        &mut host,
        &mut current,
        800.0,
        600.0
    ));
    assert!(matches!(
        ack_rx.try_recv(),
        Err(mpsc::TryRecvError::Disconnected)
    ));
    assert!(host.editor_state().selection.set.contains(&selected));
    assert!(crate::design_session::pump_progress(
        &mut host,
        &mut current,
        None
    ));
    assert!(current.is_none());
    assert!(!host.editor_state().chat.messages.last().unwrap().streaming);
}

#[test]
fn invalidated_hosted_codegen_drops_queued_completion() {
    use crate::codegen_session::{CodegenResults, CodegenSession};
    use op_editor_core::codegen::{CodeGenProgress, CodegenPhase, Framework};
    use op_editor_host_core::codegen_session::CodegenDelta;

    let mut host = WidgetHostNative::new();
    host.editor_state_mut().codegen.phase = CodegenPhase::Generating;
    let (tx, rx) = mpsc::channel();
    tx.send(CodegenDelta::Progress(CodeGenProgress::default()))
        .unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let mut current = Some(CodegenSession {
        rx,
        finished: false,
        framework: Framework::React,
        document_identity: crate::codegen_session::document_identity(&host),
        selection_snapshot: Vec::new(),
        model: None,
        cancel: cancel.clone(),
        run_epoch: 1,
        hosted_session_epoch: Some(0),
    });
    let mut results = CodegenResults::default();
    assert!(crate::codegen_session::pump(
        &mut host,
        &mut current,
        &mut results
    ));
    assert!(current.is_none());
    assert!(cancel.load(Ordering::Relaxed));
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Error);
}

#[test]
fn stale_hosted_codegen_does_not_write_an_auth_error_into_a_replaced_document() {
    use crate::codegen_session::{CodegenResults, CodegenSession};
    use op_editor_core::codegen::{CodegenPhase, Framework};

    let mut host = WidgetHostNative::new();
    let (_tx, rx) = mpsc::channel();
    let mut current = Some(CodegenSession {
        rx,
        finished: false,
        framework: Framework::React,
        document_identity: crate::codegen_session::document_identity(&host),
        selection_snapshot: Vec::new(),
        model: None,
        cancel: Arc::new(AtomicBool::new(false)),
        run_epoch: 1,
        hosted_session_epoch: Some(0),
    });
    host.editor_state_mut()
        .replace_document(op_editor_core::EditorState::new().doc);
    let mut results = CodegenResults::default();
    assert!(!crate::codegen_session::pump(
        &mut host,
        &mut current,
        &mut results
    ));
    assert!(current.is_none());
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Idle);
    assert!(host.editor_state().codegen.error.is_none());
}
