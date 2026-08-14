use super::*;
use op_editor_core::{ChatActivity, ChatActivityStatus, ChatMessage, ChatRole, Locale};
use op_editor_host_core::design::{DesignCmdReq, DesignDelta};
use op_host_native::WidgetHostNative;
use op_orchestrator::agent_identity::AgentIdentity;
use op_orchestrator::{AbortFlag, Progress, RunSummary, SubtaskOutcome};
use std::sync::mpsc;

fn identity(name: &str, color: &str) -> AgentIdentity {
    AgentIdentity {
        name: name.into(),
        color: color.into(),
    }
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
        screen: Some("Profile".into()),
        generated_root_id: None,
        existing_section_labels: None,
        retry_feedback: None,
    })
    .unwrap()
}

fn persisted_request_json() -> String {
    serde_json::to_string(&op_orchestrator::DesignRequest {
        prompt: "design profile".into(),
        model: None,
        provider: None,
        design_md: None,
        concurrency: 2,
        continuation_context: None,
        append_context: None,
        validation_enabled: false,
        visual_ref_enabled: false,
        pinned_style_guide: None,
    })
    .unwrap()
}

fn activity(id: &str, status: ChatActivityStatus) -> ChatActivity {
    ChatActivity {
        id: id.into(),
        title: id.into(),
        detail: None,
        status,
        content_offset: None,
    }
}

#[test]
fn worker_scoped_progress_builds_one_stable_message_per_screen_group() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("design three screens"));
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());

    delta_tx
        .send(DesignDelta::Progress(Progress::Planned {
            subtasks: vec![
                ("trip-plan".into(), "Trip plan".into()),
                ("explore-feed".into(), "Explore feed".into()),
                ("saved-grid".into(), "Saved grid".into()),
            ],
        }))
        .unwrap();
    delta_tx
        .send(DesignDelta::Progress(Progress::ConcurrentGroupsStarted {
            group_count: 3,
            workers: 2,
        }))
        .unwrap();
    // Deliberately arrive out of group order. Stable group metadata, not
    // arrival order or the two semaphore slots, owns each message.
    for (group, screen, name, color, id) in [
        (2, "Saved", "Pixel", "#5B8DEF", "saved-grid"),
        (0, "Trips", "Fern", "#FF6B6B", "trip-plan"),
        (1, "Explore", "Mochi", "#4ECDC4", "explore-feed"),
    ] {
        delta_tx
            .send(DesignDelta::Progress(Progress::worker_scoped(
                group,
                screen,
                identity(name, color),
                Progress::SubtaskStarted {
                    id: id.into(),
                    label: id.into(),
                },
            )))
            .unwrap();
    }

    assert!(pump_progress(&mut host, &mut current, None));

    let messages = &host.editor_state().chat.messages;
    let primary = messages
        .iter()
        .find(|message| {
            message.design_worker_group.is_none() && message.role == ChatRole::Assistant
        })
        .expect("primary message doubles as group zero");
    assert_eq!(primary.agent_name.as_deref(), Some("Fern"));
    assert_eq!(primary.design_worker_screen.as_deref(), Some("Trips"));
    assert!(primary.content.contains("3 screen groups · 2 workers"));
    assert_eq!(
        primary
            .activities
            .iter()
            .map(|activity| activity.id.as_str())
            .collect::<Vec<_>>(),
        vec!["trip-plan"],
        "non-primary rows must migrate out of the global Planned checklist"
    );
    let workers: Vec<_> = messages
        .iter()
        .filter(|message| message.design_worker_group.is_some())
        .collect();
    assert_eq!(workers.len(), 2, "primary + two workers = three personas");
    for (group, screen, name, id) in [
        (1, "Explore", "Mochi", "explore-feed"),
        (2, "Saved", "Pixel", "saved-grid"),
    ] {
        let message = workers
            .iter()
            .copied()
            .find(|message| message.design_worker_group == Some(group))
            .expect("stable group message");
        assert_eq!(message.design_worker_screen.as_deref(), Some(screen));
        assert_eq!(message.agent_name.as_deref(), Some(name));
        assert!(message.content.contains(screen));
        assert_eq!(message.activities.len(), 1);
        assert_eq!(message.activities[0].id, id);
    }

    // A later event for group 1 updates that same message rather than
    // appending another worker bubble.
    delta_tx
        .send(DesignDelta::Progress(Progress::worker_scoped(
            1,
            "Explore",
            identity("Mochi", "#4ECDC4"),
            Progress::SubtaskDone {
                id: "explore-feed".into(),
                node_count: 12,
            },
        )))
        .unwrap();
    assert!(pump_progress(&mut host, &mut current, None));
    let messages = &host.editor_state().chat.messages;
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.design_worker_group.is_some())
            .count(),
        2
    );
    let explore = messages
        .iter()
        .find(|message| message.design_worker_group == Some(1))
        .unwrap();
    assert_eq!(explore.activities[0].status, ChatActivityStatus::Done);
}

#[test]
fn worker_summary_finishes_all_messages_and_keeps_retry_on_owning_worker() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("design profile"));
    let mut primary = ChatMessage::assistant_streaming();
    primary.design_request_json_for_retry = Some(persisted_request_json());
    host.editor_state_mut().chat.messages.push(primary);

    delta_tx
        .send(DesignDelta::Progress(Progress::worker_scoped(
            1,
            "Profile",
            identity("Mochi", "#4ECDC4"),
            Progress::SubtaskFailed {
                id: "hero".into(),
                error: "empty content".into(),
            },
        )))
        .unwrap();
    let subtask: op_orchestrator::plan::Subtask =
        serde_json::from_str(&persisted_subtask_json()).unwrap();
    delta_tx
        .send(DesignDelta::Done(Ok(RunSummary {
            root_frame_id: "root".into(),
            subtasks: vec![SubtaskOutcome {
                id: "hero".into(),
                node_count: 0,
                error: Some("empty content".into()),
                inserted_root_ids: Vec::new(),
                subtask: Some(subtask),
            }],
            total_nodes: 0,
            unfilled_screens: Vec::new(),
        })))
        .unwrap();

    assert!(pump_progress(&mut host, &mut current, None));
    assert!(current.is_none());

    let messages = &host.editor_state().chat.messages;
    assert!(messages
        .iter()
        .filter(|message| message.role == ChatRole::Assistant)
        .all(|message| !message.streaming));
    let primary = messages
        .iter()
        .find(|message| {
            message.design_worker_group.is_none() && message.role == ChatRole::Assistant
        })
        .unwrap();
    assert_eq!(primary.completion.unwrap().failed, 1);
    assert!(primary.failed_subtasks.is_empty());
    let worker = messages
        .iter()
        .find(|message| message.design_worker_group == Some(1))
        .unwrap();
    assert_eq!(
        worker.design_request_json_for_retry,
        Some(persisted_request_json())
    );
    assert_eq!(worker.failed_subtasks.len(), 1);
    assert_eq!(worker.failed_subtasks[0].subtask_id, "hero");
    assert_eq!(worker.activities[0].status, ChatActivityStatus::Error);
    assert!(worker
        .content
        .contains("failed sections are expanded with their reasons"));
}

#[test]
fn terminal_design_error_stops_primary_and_every_worker_message() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());
    for (group, screen, name, color, id) in [
        (0, "Trips", "Fern", "#FF6B6B", "trip-plan"),
        (1, "Explore", "Mochi", "#4ECDC4", "explore-feed"),
    ] {
        delta_tx
            .send(DesignDelta::Progress(Progress::worker_scoped(
                group,
                screen,
                identity(name, color),
                Progress::SubtaskStarted {
                    id: id.into(),
                    label: id.into(),
                },
            )))
            .unwrap();
    }
    delta_tx
        .send(DesignDelta::Done(Err(
            op_orchestrator::OrchestratorError::Internal("boom".into()),
        )))
        .unwrap();

    assert!(pump_progress(&mut host, &mut current, None));

    let messages = &host.editor_state().chat.messages;
    assert!(messages.iter().all(|message| !message.streaming));
    assert!(messages[0].content.contains("error:"));
    assert_eq!(messages[0].activities[0].status, ChatActivityStatus::Error);
    assert!(messages[0].activities[0]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.starts_with("Reason:") && detail.contains("boom")));
    let worker = messages
        .iter()
        .find(|message| message.design_worker_group == Some(1))
        .unwrap();
    assert!(worker.content.contains("Stopped designing"));
    assert_eq!(worker.activities[0].status, ChatActivityStatus::Error);
    assert!(worker.activities[0]
        .detail
        .as_deref()
        .is_some_and(|detail| detail.starts_with("Reason:") && detail.contains("boom")));
}

#[test]
fn disconnected_session_marks_active_worker_rows_error_before_stopping() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());
    delta_tx
        .send(DesignDelta::Progress(Progress::worker_scoped(
            1,
            "Explore",
            identity("Mochi", "#4ECDC4"),
            Progress::SubtaskStarted {
                id: "explore-feed".into(),
                label: "Explore feed".into(),
            },
        )))
        .unwrap();
    drop(delta_tx);

    assert!(pump_progress(&mut host, &mut current, None));

    let worker = host
        .editor_state()
        .chat
        .messages
        .iter()
        .find(|message| message.design_worker_group == Some(1))
        .unwrap();
    assert!(!worker.streaming);
    assert_eq!(worker.activities[0].status, ChatActivityStatus::Error);
    assert_eq!(
        worker.activities[0].detail.as_deref(),
        Some("The agent connection closed before this section returned a result.")
    );
}

#[test]
fn unscoped_manual_retry_updates_owning_worker_and_finishes_on_disconnect() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("design profile"));
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant("Finished the primary screen."));
    let mut worker = ChatMessage::assistant("Designing **Profile**…");
    worker.design_worker_group = Some(1);
    worker.design_worker_screen = Some("Profile".into());
    worker.streaming = true;
    worker.activities.push(ChatActivity {
        id: "hero".into(),
        title: "Hero".into(),
        detail: None,
        status: ChatActivityStatus::Running,
        content_offset: None,
    });
    host.editor_state_mut().chat.messages.push(worker);

    delta_tx
        .send(DesignDelta::Progress(Progress::SubtaskStarted {
            id: "hero".into(),
            label: "Hero".into(),
        }))
        .unwrap();
    delta_tx
        .send(DesignDelta::Progress(Progress::SubtaskDone {
            id: "hero".into(),
            node_count: 9,
        }))
        .unwrap();
    drop(delta_tx);

    assert!(pump_progress(&mut host, &mut current, None));
    assert!(current.is_none());
    let primary = &host.editor_state().chat.messages[1];
    let worker = &host.editor_state().chat.messages[2];
    assert_eq!(primary.content, "Finished the primary screen.");
    assert_eq!(worker.activities[0].status, ChatActivityStatus::Done);
    assert_eq!(worker.activities[0].detail.as_deref(), Some("9 elements"));
    assert!(!worker.streaming);
    assert!(
        !worker.content.contains("Stopped designing"),
        "a normal retry channel close is not an interrupted worker"
    );
}

#[test]
fn group_one_retry_attempt_and_done_never_touch_primary() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("design home and profile"));
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());

    delta_tx
        .send(DesignDelta::Progress(Progress::Planned {
            subtasks: vec![
                ("home".into(), "Home".into()),
                ("profile".into(), "Profile".into()),
            ],
        }))
        .unwrap();
    delta_tx
        .send(DesignDelta::Progress(Progress::worker_scoped(
            0,
            "Home",
            identity("Fern", "#FF6B6B"),
            Progress::SubtaskStarted {
                id: "home".into(),
                label: "Home".into(),
            },
        )))
        .unwrap();
    delta_tx
        .send(DesignDelta::Progress(Progress::worker_scoped(
            1,
            "Profile",
            identity("Mochi", "#4ECDC4"),
            Progress::SubtaskRetry {
                id: "profile".into(),
                attempt: 4,
                reason: "empty output".into(),
            },
        )))
        .unwrap();

    assert!(pump_progress(&mut host, &mut current, None));
    let primary = &host.editor_state().chat.messages[1];
    assert_eq!(primary.activities.len(), 1);
    assert_eq!(primary.activities[0].id, "home");
    assert_eq!(primary.activities[0].status, ChatActivityStatus::Running);
    assert!(primary.activities[0].detail.is_none());
    let worker = host
        .editor_state()
        .chat
        .messages
        .iter()
        .find(|message| message.design_worker_group == Some(1))
        .unwrap();
    assert_eq!(worker.activities.len(), 1);
    assert_eq!(worker.activities[0].id, "profile");
    assert_eq!(worker.activities[0].status, ChatActivityStatus::Running);
    assert_eq!(
        worker.activities[0].detail.as_deref(),
        Some("Retrying · attempt 4")
    );

    delta_tx
        .send(DesignDelta::Progress(Progress::worker_scoped(
            1,
            "Profile",
            identity("Mochi", "#4ECDC4"),
            Progress::SubtaskDone {
                id: "profile".into(),
                node_count: 12,
            },
        )))
        .unwrap();
    assert!(pump_progress(&mut host, &mut current, None));

    let messages = &host.editor_state().chat.messages;
    let primary = &messages[1];
    assert_eq!(primary.activities.len(), 1);
    assert_eq!(primary.activities[0].id, "home");
    assert_eq!(primary.activities[0].status, ChatActivityStatus::Running);
    let workers: Vec<_> = messages
        .iter()
        .filter(|message| message.design_worker_group == Some(1))
        .collect();
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].activities.len(), 1);
    assert_eq!(workers[0].activities[0].status, ChatActivityStatus::Done);
    assert_eq!(
        workers[0].activities[0].detail.as_deref(),
        Some("12 elements")
    );
}

#[test]
fn partial_summary_marks_omitted_active_rows_error() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("design three screens"));
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());
    delta_tx
        .send(DesignDelta::Progress(Progress::Planned {
            subtasks: vec![
                ("trips".into(), "Trips".into()),
                ("profile".into(), "Profile".into()),
                ("saved".into(), "Saved".into()),
            ],
        }))
        .unwrap();
    for (group, screen, id) in [(0, "Trips", "trips"), (1, "Profile", "profile")] {
        delta_tx
            .send(DesignDelta::Progress(Progress::worker_scoped(
                group,
                screen,
                identity(if group == 0 { "Fern" } else { "Mochi" }, "#4ECDC4"),
                Progress::SubtaskStarted {
                    id: id.into(),
                    label: id.into(),
                },
            )))
            .unwrap();
    }
    delta_tx
        .send(DesignDelta::Progress(Progress::worker_scoped(
            2,
            "Saved",
            identity("Pixel", "#5B8DEF"),
            Progress::Planned {
                subtasks: vec![("saved".into(), "Saved".into())],
            },
        )))
        .unwrap();
    assert!(pump_progress(&mut host, &mut current, None));

    delta_tx
        .send(DesignDelta::Done(Ok(RunSummary {
            root_frame_id: "root".into(),
            subtasks: vec![SubtaskOutcome {
                id: "trips".into(),
                node_count: 7,
                error: None,
                inserted_root_ids: Vec::new(),
                subtask: None,
            }],
            total_nodes: 7,
            unfilled_screens: Vec::new(),
        })))
        .unwrap();
    assert!(pump_progress(&mut host, &mut current, None));

    let messages = &host.editor_state().chat.messages;
    let row = |id: &str| {
        messages
            .iter()
            .flat_map(|message| message.activities.iter())
            .find(|activity| activity.id == id)
            .expect("activity row")
    };
    assert_eq!(row("trips").status, ChatActivityStatus::Done);
    assert_eq!(row("profile").status, ChatActivityStatus::Error);
    assert_eq!(row("saved").status, ChatActivityStatus::Error);
    for id in ["profile", "saved"] {
        assert_eq!(
            row(id).detail.as_deref(),
            Some("The agent stopped before returning a result for this section."),
            "omitted summary row {id} needs a concrete terminal reason"
        );
    }
    assert!(messages
        .iter()
        .filter(|message| message.role == ChatRole::Assistant)
        .all(|message| !message.streaming));
    for group in [1, 2] {
        let worker = messages
            .iter()
            .find(|message| message.design_worker_group == Some(group))
            .unwrap();
        assert!(worker.content.contains("Stopped designing"));
    }
    let primary = &messages[1];
    assert_eq!(primary.completion.unwrap().succeeded, 1);
    assert_eq!(primary.completion.unwrap().failed, 2);
    assert!(primary.content.contains("2 failed"));
    assert!(primary
        .content
        .contains("failed sections below show the exact reasons"));
}

#[test]
fn unscoped_manual_retry_on_old_worker_turn_routes_and_finishes_owner() {
    let (delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("original design"));
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant("PRIMARY-SENTINEL"));
    let mut worker = ChatMessage::assistant("Designing **Profile**…");
    worker.design_worker_group = Some(1);
    worker.design_worker_screen = Some("Profile".into());
    worker.streaming = true;
    worker
        .activities
        .push(activity("hero", ChatActivityStatus::Running));
    host.editor_state_mut().chat.messages.push(worker);
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("a later unrelated turn"));
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant("LATEST-SENTINEL"));

    delta_tx
        .send(DesignDelta::Progress(Progress::SubtaskStarted {
            id: "hero".into(),
            label: "Hero".into(),
        }))
        .unwrap();
    delta_tx
        .send(DesignDelta::Progress(Progress::SubtaskDone {
            id: "hero".into(),
            node_count: 9,
        }))
        .unwrap();
    drop(delta_tx);

    assert!(pump_progress(&mut host, &mut current, None));
    assert!(current.is_none());
    let messages = &host.editor_state().chat.messages;
    assert_eq!(messages[1].content, "PRIMARY-SENTINEL");
    assert_eq!(messages[4].content, "LATEST-SENTINEL");
    let worker = &messages[2];
    assert_eq!(worker.activities[0].status, ChatActivityStatus::Done);
    assert_eq!(worker.activities[0].detail.as_deref(), Some("9 elements"));
    assert!(!worker.streaming);
    assert!(!worker.content.contains("Stopped designing"));
}

#[test]
fn explicit_stop_after_stop_streaming_finalizes_current_worker_bubbles() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    op_editor_core::agent_indicators::clear();

    let (_delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let abort = AbortFlag::new();
    let mut current_design = Some(DesignSession::from_channels_with_epoch_and_abort(
        delta_rx,
        cmd_rx,
        0,
        abort.clone(),
    ));
    let mut current_chat = None;
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    let mut old_worker = ChatMessage::assistant("OLD-WORKER-SENTINEL");
    old_worker.design_worker_group = Some(9);
    old_worker.design_worker_screen = Some("Old".into());
    old_worker
        .activities
        .push(activity("old", ChatActivityStatus::Done));
    host.editor_state_mut().chat.messages.push(old_worker);
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("design current screens"));
    let mut primary = ChatMessage::assistant_streaming();
    primary
        .activities
        .push(activity("home", ChatActivityStatus::Running));
    host.editor_state_mut().chat.messages.push(primary);
    for (group, screen, id, status) in [
        (1, "Profile", "profile", ChatActivityStatus::Pending),
        (2, "Saved", "saved", ChatActivityStatus::Running),
    ] {
        let mut worker = ChatMessage::assistant_streaming();
        worker.design_worker_group = Some(group);
        worker.design_worker_screen = Some(screen.into());
        worker.activities.push(activity(id, status));
        host.editor_state_mut().chat.messages.push(worker);
    }

    assert!(host.editor_state_mut().chat.stop_streaming());
    assert!(host
        .editor_state()
        .chat
        .messages
        .iter()
        .skip(2)
        .all(|message| !message.streaming));
    assert!(crate::chat_session::drain_stop_request(
        &mut host,
        &mut current_chat,
        &mut current_design,
        None,
    ));

    assert!(abort.is_set());
    assert!(current_design.is_none());
    assert!(!host.editor_state().chat.pending_stop_chat);
    let messages = &host.editor_state().chat.messages;
    assert_eq!(messages[0].content, "OLD-WORKER-SENTINEL");
    assert_eq!(messages[0].activities[0].status, ChatActivityStatus::Done);
    for message in messages.iter().skip(2) {
        assert_eq!(message.activities[0].status, ChatActivityStatus::Error);
        if message.design_worker_group.is_some() {
            assert_eq!(message.content.matches("Stopped designing").count(), 1);
        }
    }
    op_editor_core::agent_indicators::clear();
}

#[test]
fn explicit_stop_falls_back_to_active_retry_before_later_completed_workers() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    op_editor_core::agent_indicators::clear();

    let (_delta_tx, delta_rx) = mpsc::channel::<DesignDelta>();
    let (_cmd_tx, cmd_rx) = mpsc::channel::<DesignCmdReq>();
    let mut current_design = Some(DesignSession::from_channels(delta_rx, cmd_rx));
    let mut current_chat = None;
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.locale = Locale::EnUs;
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("original design"));
    let mut retry = ChatMessage::assistant_streaming();
    retry.design_worker_group = Some(1);
    retry.design_worker_screen = Some("Profile".into());
    retry
        .activities
        .push(activity("hero", ChatActivityStatus::Running));
    host.editor_state_mut().chat.messages.push(retry);
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::user("later design"));
    let mut completed = ChatMessage::assistant("LATER-WORKER-SENTINEL");
    completed.design_worker_group = Some(2);
    completed.design_worker_screen = Some("Saved".into());
    completed
        .activities
        .push(activity("saved", ChatActivityStatus::Done));
    host.editor_state_mut().chat.messages.push(completed);

    assert!(host.editor_state_mut().chat.stop_streaming());
    assert!(crate::chat_session::drain_stop_request(
        &mut host,
        &mut current_chat,
        &mut current_design,
        None,
    ));

    let retry = &host.editor_state().chat.messages[1];
    assert_eq!(retry.activities[0].status, ChatActivityStatus::Error);
    assert_eq!(retry.content.matches("Stopped designing").count(), 1);
    let completed = &host.editor_state().chat.messages[3];
    assert_eq!(completed.content, "LATER-WORKER-SENTINEL");
    assert_eq!(completed.activities[0].status, ChatActivityStatus::Done);
    op_editor_core::agent_indicators::clear();
}
