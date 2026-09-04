use op_ai::chat_provider::{
    ChatDelta, ChatHistoryRole, ChatProvider, ChatRequest, ChatToolExecutor, ChatToolResult,
    EchoProvider, StopReason,
};
use op_editor_core::{ChatActivity, ChatActivityStatus, ChatCompletion, ChatMessage, ChatToolCall};
use op_editor_host_core::chat::{
    apply_poll_to_message, apply_poll_to_message_with, chat_history_from_transcript,
    chat_tool_channel, ChatPoll, ChatSession,
};

fn drain_session(session: &mut ChatSession) -> (String, String, Vec<ChatToolCall>, Option<String>) {
    let mut text = String::new();
    let mut thinking = String::new();
    let mut tools = Vec::new();
    let mut error = None;
    for _ in 0..1000 {
        let poll = session.poll();
        text.push_str(&poll.text);
        thinking.push_str(&poll.thinking);
        tools.extend(poll.tool_calls);
        if poll.error.is_some() {
            error = poll.error;
        }
        if poll.finished {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    (text, thinking, tools, error)
}

#[test]
fn session_streams_provider_deltas_to_completion() {
    let provider = Box::new(EchoProvider {
        script: vec![
            ChatDelta::TextDelta("Hel".into()),
            ChatDelta::TextDelta("lo".into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ],
    });
    let mut session = ChatSession::start(
        provider,
        ChatRequest {
            system_prompt: String::new(),
            user_message: "hi".into(),
            max_output_tokens: 256,
            ..Default::default()
        },
    );

    let (text, _, _, _) = drain_session(&mut session);
    assert!(session.finished());
    assert_eq!(text, "Hello");
}

struct SilentCancellableProvider {
    canceled_tx: std::sync::mpsc::Sender<()>,
}

impl ChatProvider for SilentCancellableProvider {
    fn provider_label(&self) -> &str {
        "silent-cancellable"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        panic!("ChatSession must launch providers through send_cancellable")
    }

    fn supports_cancellable_send(&self) -> bool {
        true
    }

    fn send_cancellable(
        &self,
        _request: ChatRequest,
        cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(CancelWaitIter {
            cancel,
            canceled_tx: Some(self.canceled_tx.clone()),
        })
    }
}

struct CancelWaitIter {
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    canceled_tx: Option<std::sync::mpsc::Sender<()>>,
}

impl Iterator for CancelWaitIter {
    type Item = ChatDelta;

    fn next(&mut self) -> Option<Self::Item> {
        while !self.cancel.load(std::sync::atomic::Ordering::Acquire) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        if let Some(tx) = self.canceled_tx.take() {
            let _ = tx.send(());
        }
        None
    }
}

#[test]
fn dropping_session_cancels_a_silent_provider() {
    let (canceled_tx, canceled_rx) = std::sync::mpsc::channel();
    let session = ChatSession::start(
        Box::new(SilentCancellableProvider { canceled_tx }),
        ChatRequest::default(),
    );

    drop(session);

    canceled_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("dropping Stop/New Chat session must cancel a silent provider");
}

#[test]
fn dropping_channel_backed_session_sets_external_cancel_flag() {
    let (_tx, rx) = std::sync::mpsc::channel();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let session = ChatSession::from_channels_with_cancel(rx, None, std::sync::Arc::clone(&cancel));

    drop(session);

    assert!(cancel.load(std::sync::atomic::Ordering::Acquire));
}

#[test]
fn poll_splits_thinking_tools_errors_and_answer_text() {
    let provider = Box::new(EchoProvider {
        script: vec![
            ChatDelta::Thinking("let me think".into()),
            ChatDelta::ToolUse {
                name: "insert_node".into(),
                args: "{\"kind\":\"rect\"}".into(),
            },
            ChatDelta::TextDelta("answer".into()),
            ChatDelta::Error("boom".into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ],
    });
    let mut session = ChatSession::start(
        provider,
        ChatRequest {
            user_message: "x".into(),
            max_output_tokens: 64,
            ..Default::default()
        },
    );

    let (text, thinking, tools, error) = drain_session(&mut session);
    assert_eq!(text, "answer");
    assert_eq!(thinking, "let me think");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "insert_node");
    assert_eq!(tools[0].args, "{\"kind\":\"rect\"}");
    assert_eq!(error.as_deref(), Some("boom"));
}

#[test]
fn apply_poll_appends_content_and_clears_streaming_on_finish() {
    let mut msg = ChatMessage::assistant_streaming();
    apply_poll_to_message(
        &mut msg,
        &ChatPoll {
            text: "hi".into(),
            thinking: "reasoning".into(),
            tool_calls: vec![ChatToolCall {
                name: "t".into(),
                args: "{}".into(),
                content_offset: None,
            }],
            error: None,
            finished: false,
        },
    );
    assert_eq!(msg.content, "hi");
    assert_eq!(msg.thinking, "reasoning");
    assert_eq!(msg.tool_calls.len(), 1);
    assert!(msg.streaming);

    apply_poll_to_message(
        &mut msg,
        &ChatPoll {
            text: "!".into(),
            thinking: String::new(),
            tool_calls: vec![],
            error: None,
            finished: true,
        },
    );
    assert_eq!(msg.content, "hi!");
    assert!(!msg.streaming);
}

#[test]
fn design_loop_streams_narration_as_visible_prose_with_offsets() {
    // Pencil parity: design-loop narration is first-class transcript prose
    // (it used to fold into the collapsed thinking area, leaving the panel
    // showing nothing but "Thinking / N tool calls"). Tool calls stamp the
    // content offset where they landed so the transcript interleaves.
    let mut design = ChatMessage::assistant_streaming();
    apply_poll_to_message_with(
        &mut design,
        &ChatPoll {
            text: "Let me build the header".into(),
            thinking: "raw".into(),
            tool_calls: vec![ChatToolCall {
                name: "batch_design".into(),
                args: "{}".into(),
                content_offset: None,
            }],
            error: None,
            finished: false,
        },
        true,
    );
    assert_eq!(
        design.content, "Let me build the header",
        "narration streams into the visible bubble"
    );
    assert!(design.thinking.contains("raw"));
    assert!(!design.thinking.contains("Let me build the header"));
    assert_eq!(
        design.tool_calls[0].content_offset,
        Some("Let me build the header".len() as u32),
        "the call lands AFTER the narration that preceded it"
    );

    let mut chat = ChatMessage::assistant_streaming();
    apply_poll_to_message_with(
        &mut chat,
        &ChatPoll {
            text: "Sure, here you go".into(),
            thinking: String::new(),
            tool_calls: vec![],
            error: None,
            finished: false,
        },
        false,
    );
    assert_eq!(
        chat.content, "Sure, here you go",
        "plain chat keeps narration visible"
    );

    let mut errd = ChatMessage::assistant_streaming();
    apply_poll_to_message_with(
        &mut errd,
        &ChatPoll {
            text: "ignored".into(),
            thinking: String::new(),
            tool_calls: vec![],
            error: Some("boom".into()),
            finished: true,
        },
        true,
    );
    assert_eq!(
        errd.content, "error: boom",
        "errors always surface in content"
    );
}

#[test]
fn design_loop_preserves_text_tool_text_order_within_one_poll() {
    let (tx, rx) = std::sync::mpsc::channel();
    tx.send(ChatDelta::TextDelta("before".into())).unwrap();
    tx.send(ChatDelta::ToolUse {
        name: "batch_design".into(),
        args: "{}".into(),
    })
    .unwrap();
    tx.send(ChatDelta::TextDelta("after".into())).unwrap();
    drop(tx);

    let mut session = ChatSession::from_channels(rx, None).into_design_loop();
    let poll = session.poll();
    assert_eq!(poll.text, "beforeafter");
    assert_eq!(poll.tool_calls.len(), 1);
    assert_eq!(
        poll.tool_calls[0].content_offset,
        Some("before".len() as u32),
        "the poll records the call where it appeared, not after all batched text"
    );

    let mut message = ChatMessage::assistant_streaming();
    apply_poll_to_message_with(&mut message, &poll, true);

    assert_eq!(message.content, "beforeafter");
    assert_eq!(
        message.tool_calls[0].content_offset,
        Some("before".len() as u32),
        "the design transcript interleaves the tool between before and after"
    );
}

#[test]
fn apply_poll_opens_modify_tools_but_keeps_read_tools_collapsed() {
    let mut modify = ChatMessage::assistant_streaming();
    apply_poll_to_message(
        &mut modify,
        &ChatPoll {
            text: String::new(),
            thinking: String::new(),
            tool_calls: vec![ChatToolCall {
                name: "batch_design".into(),
                args: "{}".into(),
                content_offset: None,
            }],
            error: None,
            finished: false,
        },
    );
    assert!(!modify.tools_collapsed);

    let mut read = ChatMessage::assistant_streaming();
    apply_poll_to_message(
        &mut read,
        &ChatPoll {
            text: String::new(),
            thinking: String::new(),
            tool_calls: vec![ChatToolCall {
                name: "snapshot_layout".into(),
                args: "{}".into(),
                content_offset: None,
            }],
            error: None,
            finished: false,
        },
    );
    assert!(read.tools_collapsed);
}

#[test]
fn apply_poll_error_replaces_content_and_ends_stream() {
    let mut msg = ChatMessage::assistant_streaming();
    msg.content = "partial answer".into();
    apply_poll_to_message(
        &mut msg,
        &ChatPoll {
            text: String::new(),
            thinking: String::new(),
            tool_calls: vec![],
            error: Some("rate limited".into()),
            finished: true,
        },
    );
    assert_eq!(msg.content, "error: rate limited");
    assert!(!msg.streaming);
}

#[test]
fn chat_history_from_transcript_excludes_current_streaming_turn() {
    let messages = vec![
        ChatMessage::user("previous request"),
        ChatMessage::assistant("previous answer"),
        ChatMessage::user("current request"),
        ChatMessage::assistant_streaming(),
    ];

    let history = chat_history_from_transcript(&messages);

    assert_eq!(
        history,
        vec![
            (ChatHistoryRole::User, "previous request".into()),
            (ChatHistoryRole::Assistant, "previous answer".into()),
        ]
    );
}

#[test]
fn chat_history_from_transcript_skips_blank_messages() {
    let messages = vec![
        ChatMessage::user("first"),
        ChatMessage::assistant("   "),
        ChatMessage::assistant("answer"),
    ];

    let history = chat_history_from_transcript(&messages);

    assert_eq!(
        history,
        vec![
            (ChatHistoryRole::User, "first".into()),
            (ChatHistoryRole::Assistant, "answer".into()),
        ]
    );
}

#[test]
fn chat_history_folds_multi_screen_worker_projections_into_one_assistant_turn() {
    let mut primary = ChatMessage::assistant("The design is complete.");
    primary.design_worker_screen = Some("Explore".into());
    primary.activities.push(ChatActivity {
        id: "explore-feed".into(),
        title: "Explore feed".into(),
        detail: None,
        status: ChatActivityStatus::Done,
        content_offset: Some(0),
    });
    primary.completion = Some(ChatCompletion {
        succeeded: 2,
        failed: 0,
        nodes: 24,
    });

    let mut worker = ChatMessage::assistant("Profile is ready.");
    worker.design_worker_group = Some(1);
    worker.design_worker_screen = Some("Profile".into());
    worker.agent_name = Some("Mochi".into());
    worker.activities.push(ChatActivity {
        id: "profile-body".into(),
        title: "Profile body".into(),
        detail: None,
        status: ChatActivityStatus::Done,
        content_offset: Some(0),
    });
    let messages = vec![
        ChatMessage::user("design two screens"),
        primary,
        worker,
        ChatMessage::user("make both denser"),
        ChatMessage::assistant_streaming(),
    ];

    let history = chat_history_from_transcript(&messages);

    assert_eq!(history.len(), 2);
    assert_eq!(
        history[0],
        (ChatHistoryRole::User, "design two screens".into())
    );
    assert_eq!(history[1].0, ChatHistoryRole::Assistant);
    let assistant = &history[1].1;
    assert!(assistant.contains("Screen Explore"), "{assistant}");
    assert!(assistant.contains("Explore feed"), "{assistant}");
    assert!(assistant.contains("Screen Profile"), "{assistant}");
    assert!(assistant.contains("Profile body"), "{assistant}");
    assert!(assistant.contains("2 succeeded, 0 failed"), "{assistant}");
    assert_eq!(
        history
            .iter()
            .filter(|(role, _)| *role == ChatHistoryRole::Assistant)
            .count(),
        1,
        "primary + worker projections are one provider assistant turn"
    );
}

#[test]
fn chat_history_preserves_structured_design_completion_for_follow_up() {
    let mut completed = ChatMessage::assistant("");
    completed.activities.push(ChatActivity {
        id: "recent".into(),
        title: "Recently Played".into(),
        detail: None,
        status: ChatActivityStatus::Done,
        content_offset: None,
    });
    completed.completion = Some(ChatCompletion {
        succeeded: 1,
        failed: 0,
        nodes: 18,
    });
    let messages = vec![
        ChatMessage::user("design a music home"),
        completed,
        ChatMessage::user("make the cards smaller"),
        ChatMessage::assistant_streaming(),
    ];

    let history = chat_history_from_transcript(&messages);

    assert_eq!(history.len(), 2);
    assert_eq!(
        history[0],
        (ChatHistoryRole::User, "design a music home".into())
    );
    assert_eq!(history[1].0, ChatHistoryRole::Assistant);
    assert!(history[1].1.contains("1 succeeded, 0 failed"));
    assert!(history[1].1.contains("Recently Played"));
}

#[test]
fn chat_history_keeps_visible_narration_and_structured_sections() {
    let mut completed = ChatMessage::assistant("Done — the layout has been checked.");
    completed.activities.push(ChatActivity {
        id: "header".into(),
        title: "Greeting Header".into(),
        detail: None,
        status: ChatActivityStatus::Done,
        content_offset: Some(0),
    });
    completed.activities.push(ChatActivity {
        id: "__validation".into(),
        title: "Checking the design".into(),
        detail: None,
        status: ChatActivityStatus::Done,
        content_offset: Some(0),
    });
    completed.completion = Some(ChatCompletion {
        succeeded: 1,
        failed: 0,
        nodes: 1,
    });

    let history = chat_history_from_transcript(&[
        ChatMessage::user("design it"),
        completed,
        ChatMessage::user("make the header smaller"),
        ChatMessage::assistant_streaming(),
    ]);

    assert!(history[1].1.contains("Done — the layout has been checked."));
    assert!(history[1]
        .1
        .contains("Design work completed: 1 succeeded, 0 failed."));
    assert!(history[1].1.contains("Sections: Greeting Header."));
    assert!(!history[1].1.contains("Checking the design"));
    assert!(!history[1].1.contains("nodes"));
}

#[test]
fn tool_executor_forwards_request_and_waits_for_ack() {
    let (executor, rx) = chat_tool_channel();
    let worker = std::thread::spawn(move || executor.execute("insert_node", "{\"kind\":\"rect\"}"));

    let req = rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("tool request");
    assert_eq!(req.name, "insert_node");
    assert_eq!(req.args_json, "{\"kind\":\"rect\"}");
    req.ack
        .send(ChatToolResult {
            content: r#"{"success":true}"#.into(),
            is_error: false,
        })
        .expect("ack");

    let result = worker.join().expect("worker joins");
    assert_eq!(result.content, r#"{"success":true}"#);
    assert!(!result.is_error);
}
