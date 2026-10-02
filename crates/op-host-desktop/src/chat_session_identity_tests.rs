use super::*;
use op_ai::chat_provider::{ChatDelta, ChatRequest, EchoProvider, StopReason};
use op_editor_core::{ChatMessage, ChatToolCall};

#[test]
fn attach_tool_result_targets_the_matching_agent_message() {
    let running_call = || ChatToolCall {
        name: "batch_design".into(),
        args: r#"{"status":"running"}"#.into(),
        content_offset: None,
    };
    let mut chat = ChatState::default();
    let mut parent = ChatMessage::assistant_streaming();
    parent.agent_name = Some("Fern".into());
    parent.agent_color = Some("#6C5CE7".into());
    parent.tool_calls.push(running_call());
    chat.messages.push(parent);
    let mut sub = ChatMessage::assistant_streaming();
    sub.agent_name = Some("Kiki".into());
    sub.agent_color = Some("#FF6B6B".into());
    sub.tool_calls.push(running_call());
    chat.messages.push(sub);
    let result = ChatToolResult {
        content: r#"{"success":true}"#.into(),
        is_error: false,
    };

    assert!(attach_tool_result_to_transcript(
        &mut chat,
        "batch_design",
        &result,
    ));
    assert!(attach_tool_result_to_transcript_with(
        &mut chat,
        "batch_design",
        &result,
        Some(("Kiki", "#FF6B6B")),
    ));

    let parent: serde_json::Value =
        serde_json::from_str(&chat.messages[0].tool_calls[0].args).unwrap();
    let sub: serde_json::Value =
        serde_json::from_str(&chat.messages[1].tool_calls[0].args).unwrap();
    assert_eq!(parent["status"], "done");
    assert_eq!(sub["status"], "done");
}

#[test]
fn parent_pump_does_not_write_into_trailing_sub_agent_message() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.messages.extend([
        ChatMessage::user("earlier request"),
        ChatMessage::assistant("earlier reply"),
        ChatMessage::user("current request"),
    ]);
    let mut parent = ChatMessage::assistant_streaming();
    parent.agent_name = Some("Fern".into());
    parent.agent_color = Some("#6C5CE7".into());
    host.editor_state_mut().chat.messages.push(parent);
    let mut sub = ChatMessage::assistant_streaming();
    sub.agent_name = Some("Kiki".into());
    sub.agent_color = Some("#FF6B6B".into());
    host.editor_state_mut().chat.messages.push(sub);

    let provider = Box::new(EchoProvider {
        script: vec![
            ChatDelta::TextDelta("parent reply".into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ],
    });
    let mut current = Some(ChatSession::start(
        provider,
        ChatRequest {
            user_message: "hi".into(),
            max_output_tokens: 64,
            ..Default::default()
        },
    ));
    for _ in 0..2000 {
        pump(&mut host, &mut current, None, None, (1200.0, 800.0));
        if current.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    let messages = &host.editor_state().chat.messages;
    assert_eq!(messages[1].content, "earlier reply");
    assert_eq!(messages[3].content, "parent reply");
    assert_eq!(messages[3].agent_name.as_deref(), Some("Fern"));
    assert_eq!(messages[3].agent_color.as_deref(), Some("#6C5CE7"));
    assert!(!messages[3].streaming);
    assert!(messages[4].content.is_empty());
    assert_eq!(messages[4].agent_name.as_deref(), Some("Kiki"));
}

#[test]
fn agent_message_routing_never_falls_back_to_a_previous_turn_or_another_agent() {
    let mut previous_worker = ChatMessage::assistant("previous worker reply");
    previous_worker.agent_name = Some("Kiki".into());
    previous_worker.agent_color = Some("#FF6B6B".into());
    let mut messages = vec![
        ChatMessage::user("previous request"),
        ChatMessage::assistant("previous primary reply"),
        previous_worker,
        ChatMessage::user("current request"),
    ];
    assert_eq!(agent_message_index(&messages, None), None);
    assert_eq!(
        agent_message_index(&messages, Some(("Kiki", "#FF6B6B"))),
        None
    );

    messages.push(ChatMessage::assistant_streaming());
    assert_eq!(agent_message_index(&messages, None), Some(4));
    assert_eq!(
        agent_message_index(&messages, Some(("Kiki", "#FF6B6B"))),
        None,
        "an unmatched worker must not claim the primary bubble"
    );
}

#[test]
fn primary_message_routing_skips_classic_design_worker_bubbles() {
    let mut worker = ChatMessage::assistant_streaming();
    worker.design_worker_group = Some(1);
    let messages = vec![
        ChatMessage::user("current request"),
        worker,
        ChatMessage::assistant_streaming(),
    ];
    assert_eq!(agent_message_index(&messages, None), Some(2));
}
