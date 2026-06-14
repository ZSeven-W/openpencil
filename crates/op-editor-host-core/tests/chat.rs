use op_ai::chat_provider::{
    ChatDelta, ChatRequest, ChatToolExecutor, ChatToolResult, EchoProvider, StopReason,
};
use op_editor_core::{ChatMessage, ChatToolCall};
use op_editor_host_core::chat::{apply_poll_to_message, chat_tool_channel, ChatPoll, ChatSession};

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
