use super::*;
use op_editor_core::{ChatActivityStatus, ChatCompletion, ChatMessage, Locale};
use op_editor_host_core::chat::{apply_poll_to_message, ChatPoll};
use op_editor_host_core::design::{DesignCmdReq, DesignDelta};
use op_host_native::WidgetHostNative;
use op_orchestrator::{Progress, RunSummary, SubtaskOutcome};
use std::sync::mpsc;

fn failed_subtask(id: &str, label: &str) -> op_orchestrator::plan::Subtask {
    op_orchestrator::plan::Subtask {
        id: id.into(),
        label: label.into(),
        region: op_orchestrator::plan::Region {
            width: 375.0,
            height: 180.0,
        },
        id_prefix: id.into(),
        parent_frame_id: Some("root".into()),
        elements: None,
        screen: Some("Now".into()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    }
}

fn design_request_json() -> String {
    serde_json::to_string(&op_orchestrator::DesignRequest {
        prompt: "design a weather app".into(),
        model: Some("gemini-3.6-flash".into()),
        provider: Some("antigravity".into()),
        design_md: None,
        concurrency: 1,
        continuation_context: None,
        append_context: None,
        validation_enabled: true,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    })
    .unwrap()
}

fn failed_run_summary() -> RunSummary {
    RunSummary {
        root_frame_id: "root".into(),
        subtasks: vec![
            SubtaskOutcome {
                id: "hero".into(),
                node_count: 12,
                error: None,
                inserted_root_ids: vec!["hero-root".into()],
                subtask: None,
            },
            SubtaskOutcome {
                id: "sun_arc".into(),
                node_count: 0,
                error: Some("self-check failed".into()),
                inserted_root_ids: Vec::new(),
                subtask: Some(failed_subtask("sun_arc", "Sunrise & Sunset Arc")),
            },
        ],
        total_nodes: 12,
        unfilled_screens: Vec::new(),
    }
}

#[test]
fn companion_chat_disconnect_cannot_drop_validation_completion_or_retry_payload() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("design a weather app"));
    let mut primary = ChatMessage::assistant_streaming();
    primary.design_request_json_for_retry = Some(design_request_json());
    host.editor_state_mut().chat.messages.push(primary);

    for event in [
        Progress::Planned {
            subtasks: vec![
                ("hero".into(), "Weather Hero".into()),
                ("sun_arc".into(), "Sunrise & Sunset Arc".into()),
            ],
        },
        Progress::SubtaskDone {
            id: "hero".into(),
            node_count: 12,
        },
        Progress::SubtaskFailed {
            id: "sun_arc".into(),
            error: "self-check failed".into(),
        },
        Progress::CleanupDone,
        Progress::ValidationStarted,
    ] {
        delta_tx.send(DesignDelta::Progress(event)).unwrap();
    }
    assert!(pump_progress(&mut host, &mut current, None));

    // This is the real CLI-standard terminal ordering: its companion chat
    // sender drops after run_design_worker returns, while the design channel
    // already contains ValidationDone + Done. The app pumps chat first.
    let primary = host.editor_state_mut().chat.messages.last_mut().unwrap();
    apply_poll_to_message(
        primary,
        &ChatPoll {
            text: String::new(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            error: None,
            finished: true,
        },
    );
    assert!(!primary.streaming);

    delta_tx
        .send(DesignDelta::Progress(Progress::ValidationPreCheckDone {
            applied: 1,
            by_category: Default::default(),
        }))
        .unwrap();
    delta_tx
        .send(DesignDelta::Progress(Progress::ValidationDone {
            total_applied: 1,
        }))
        .unwrap();
    delta_tx
        .send(DesignDelta::Done(Ok(failed_run_summary())))
        .unwrap();

    assert!(pump_progress(&mut host, &mut current, None));
    assert!(current.is_none());
    let message = host.editor_state().chat.messages.last().unwrap();
    assert_eq!(
        message
            .activities
            .iter()
            .find(|activity| activity.id == "__validation")
            .unwrap()
            .status,
        ChatActivityStatus::Done
    );
    assert_eq!(
        message
            .activities
            .iter()
            .find(|activity| activity.id == "sun_arc")
            .unwrap()
            .status,
        ChatActivityStatus::Error
    );
    assert_eq!(
        message
            .activities
            .iter()
            .find(|activity| activity.id == "sun_arc")
            .unwrap()
            .detail
            .as_deref(),
        Some("Reason: self-check failed")
    );
    assert_eq!(message.failed_subtasks.len(), 1);
    assert_eq!(message.failed_subtasks[0].subtask_id, "sun_arc");
    assert_eq!(
        message.completion,
        Some(ChatCompletion {
            succeeded: 1,
            failed: 1,
            nodes: 12,
        })
    );
}

#[test]
fn first_design_pump_after_chat_disconnect_keeps_all_progress_and_retry_payload() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("design a weather app"));
    let mut primary = ChatMessage::assistant_streaming();
    primary.design_request_json_for_retry = Some(design_request_json());
    host.editor_state_mut().chat.messages.push(primary);

    for event in [
        Progress::Planning,
        Progress::Planned {
            subtasks: vec![
                ("hero".into(), "Weather Hero".into()),
                ("sun_arc".into(), "Sunrise & Sunset Arc".into()),
            ],
        },
        Progress::SubtaskDone {
            id: "hero".into(),
            node_count: 12,
        },
        Progress::SubtaskFailed {
            id: "sun_arc".into(),
            error: "self-check failed".into(),
        },
        Progress::CleanupDone,
        Progress::ValidationStarted,
        Progress::ValidationDone { total_applied: 0 },
    ] {
        delta_tx.send(DesignDelta::Progress(event)).unwrap();
    }
    delta_tx
        .send(DesignDelta::Done(Ok(failed_run_summary())))
        .unwrap();

    // A fast design can finish before the first design pump. The companion
    // chat is still pumped first, so the request marker must route the first
    // design progress event even though no activity exists yet.
    let primary = host.editor_state_mut().chat.messages.last_mut().unwrap();
    apply_poll_to_message(
        primary,
        &ChatPoll {
            text: String::new(),
            thinking: String::new(),
            tool_calls: Vec::new(),
            error: None,
            finished: true,
        },
    );
    assert!(!primary.streaming);

    assert!(pump_progress(&mut host, &mut current, None));
    assert!(current.is_none());
    let message = host.editor_state().chat.messages.last().unwrap();
    assert_eq!(
        message
            .activities
            .iter()
            .find(|activity| activity.id == "__validation")
            .unwrap()
            .status,
        ChatActivityStatus::Done
    );
    assert_eq!(
        message
            .activities
            .iter()
            .find(|activity| activity.id == "sun_arc")
            .unwrap()
            .status,
        ChatActivityStatus::Error
    );
    assert_eq!(message.failed_subtasks.len(), 1);
    assert_eq!(message.failed_subtasks[0].subtask_id, "sun_arc");
}

#[test]
fn unused_design_session_disconnect_does_not_finish_plain_chat_bubble() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    drop(delta_tx);
    drop(cmd_tx);
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("explain this"));
    host.editor_state_mut().chat.messages.push({
        let mut message = ChatMessage::assistant_streaming();
        // CLI-standard stashes this before route classification, including
        // for Chat and Modify turns. It is not proof that the parked
        // DesignSession owns the bubble.
        message.design_request_json_for_retry = Some(design_request_json());
        message
    });

    assert!(!pump_progress(&mut host, &mut current, None));
    assert!(current.is_none());
    assert!(
        host.editor_state().chat.messages.last().unwrap().streaming,
        "the unused parked DesignSession must not terminate the real ChatSession bubble"
    );
}
