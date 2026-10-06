use super::*;
use op_ai::chat_provider::{ChatDelta, ChatRequest, EchoProvider, StopReason};
use op_editor_core::{ChatMessage, ChatToolCall};

#[test]
fn session_streams_echo_provider_deltas_to_completion() {
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
    // Drain to completion — poll in a bounded loop so a stuck
    // worker fails the test instead of hanging it.
    let mut acc = String::new();
    for _ in 0..1000 {
        let p = session.poll();
        acc.push_str(&p.text);
        if p.finished {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(session.finished(), "session must reach Done");
    assert_eq!(acc, "Hello");
}

#[test]
fn poll_splits_thinking_and_tool_calls_from_answer_text() {
    let provider = Box::new(EchoProvider {
        script: vec![
            ChatDelta::Thinking("let me think".into()),
            ChatDelta::ToolUse {
                name: "insert_node".into(),
                args: "{\"kind\":\"rect\"}".into(),
            },
            ChatDelta::TextDelta("here is the answer".into()),
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
    let mut text = String::new();
    let mut thinking = String::new();
    let mut tools = Vec::new();
    for _ in 0..1000 {
        let p = session.poll();
        text.push_str(&p.text);
        thinking.push_str(&p.thinking);
        tools.extend(p.tool_calls);
        if p.finished {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(text, "here is the answer", "answer text only");
    assert_eq!(thinking, "let me think", "thinking routed separately");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "insert_node");
    assert_eq!(tools[0].args, "{\"kind\":\"rect\"}");
}

#[test]
fn apply_poll_appends_text_thinking_tools_and_clears_streaming_on_finish() {
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
    assert!(msg.streaming, "still streaming until the turn finishes");

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
    assert_eq!(msg.content, "hi!", "text accumulates across polls");
    assert!(!msg.streaming, "finished clears the streaming flag");
}

#[test]
fn apply_poll_expands_modify_tool_process_like_ts_cards() {
    let mut msg = ChatMessage::assistant_streaming();
    assert!(msg.tools_collapsed, "assistant messages start collapsed");

    apply_poll_to_message(
        &mut msg,
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

    assert!(
        !msg.tools_collapsed,
        "TS opens modify/delete/orchestrate tool cards by default"
    );
}

#[test]
fn apply_poll_keeps_read_tool_process_collapsed_like_ts_cards() {
    let mut msg = ChatMessage::assistant_streaming();

    apply_poll_to_message(
        &mut msg,
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

    assert!(
        msg.tools_collapsed,
        "TS keeps read/create tool cards collapsed by default"
    );
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
fn drain_stop_request_drops_session_without_clearing_transcript() {
    // `drain_stop_request` retires the process-global active indicator epoch.
    // Serialize this test with every reveal/indicator test so a parallel Stop
    // cannot clear another test's epoch between its `begin()` and first
    // `batch_design` reveal registration.
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();

    let provider = Box::new(EchoProvider {
        script: vec![ChatDelta::TextDelta("late".into())],
    });
    let mut current = Some(ChatSession::start(
        provider,
        ChatRequest {
            user_message: "x".into(),
            max_output_tokens: 64,
            ..Default::default()
        },
    ));
    let mut current_design = None;
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());

    assert!(host.editor_state_mut().chat.stop_streaming());
    assert!(drain_stop_request(
        &mut host,
        &mut current,
        &mut current_design,
        None,
    ));

    assert!(current.is_none());
    assert!(!host.editor_state().chat.pending_stop_chat);
    assert_eq!(host.editor_state().chat.messages.len(), 1);
    assert!(!host.editor_state().chat.messages[0].streaming);
    op_editor_core::agent_indicators::clear();
}

#[test]
fn drain_stop_request_marks_a_generating_workspace_stopped() {
    // Serialize against the indicator tests (see the test above).
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();

    let mut current = None;
    let mut current_design = None;
    let mut host = WidgetHostNative::new();
    {
        let editor = host.editor_state_mut();
        editor
            .chat
            .messages
            .push(ChatMessage::assistant_streaming());
        editor.chat.stop_streaming();
        // A Home-launched generation just opened the workspace.
        editor.editor_ui.workspace.open_for_generation(
            op_editor_core::HomeFamily::AppUi,
            "取餐预约，3 个页面",
            op_editor_core::TaskDraft::default(),
            0,
            1_000,
            None,
        );
        editor.chat.pending_stop_chat = true;
    }

    assert!(drain_stop_request(
        &mut host,
        &mut current,
        &mut current_design,
        None,
    ));
    let workspace = &host.editor_state().editor_ui.workspace;
    assert_eq!(workspace.phase, op_editor_core::WorkspacePhase::Stopped);
    // The late idle edge can no longer flip it to Done or Failed.
    assert!(!host.editor_state_mut().editor_ui.workspace.mark_done(0));
    assert!(!host.editor_state_mut().editor_ui.workspace.mark_failed(0));
    assert_eq!(
        host.editor_state().editor_ui.workspace.phase,
        op_editor_core::WorkspacePhase::Stopped
    );
    op_editor_core::agent_indicators::clear();
}

#[test]
fn selected_builtin_model_routes_to_builtin_provider() {
    let mut host = WidgetHostNative::new();
    let id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Built-in Claude", "sk-test", "claude-sonnet-4-5");
    host.editor_state_mut().rebuild_chat_models();
    let idx = host
        .editor_state()
        .chat
        .available_models
        .iter()
        .position(|m| m.builtin_provider_id.as_deref() == Some(id.as_str()))
        .expect("built-in model should be selectable");
    host.editor_state_mut().select_chat_model(idx);

    let provider = provider_for_selected_model(&host).expect("built-in provider should build");
    assert_eq!(provider.provider_label(), "Built-in Claude");
}

#[test]
fn selected_acp_model_routes_to_acp_provider() {
    let mut host = WidgetHostNative::new();
    let id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_acp_agent_config(
            "Local ACP",
            op_editor_core::AcpConnectionType::Local,
            "test-acp-agent",
            Vec::new(),
            std::collections::BTreeMap::new(),
            None,
            true,
        );
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .apply_acp_agent_connect_outcome(
            &id,
            op_editor_core::AcpAgentConnectOutcome {
                connected: true,
                info: Some("Local ACP".into()),
                error: None,
            },
        );
    host.editor_state_mut().rebuild_chat_models();
    let idx = host
        .editor_state()
        .chat
        .available_models
        .iter()
        .position(|m| m.value == format!("acp:{id}"))
        .expect("ACP model should be selectable");
    host.editor_state_mut().select_chat_model(idx);

    let provider = provider_for_selected_model(&host).expect("ACP provider should build");
    assert_eq!(provider.provider_label(), "ACP: Local ACP");
}

#[test]
fn selected_cli_model_forwards_wire_id_for_matching_provider() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.available_models = vec![
        op_editor_core::ModelEntry::new(op_editor_core::AgentProvider::ClaudeCode, "opus", "Opus"),
        op_editor_core::ModelEntry::new(
            op_editor_core::AgentProvider::CodexCli,
            "gpt-5.5",
            "GPT-5.5",
        ),
    ];
    // select_chat_model syncs chat_selected_agent to the entry's
    // provider, so the routed CLI and the model id stay paired.
    host.editor_state_mut().select_chat_model(1);
    assert_eq!(selected_cli_model_id(&host).as_deref(), Some("gpt-5.5"));
    host.editor_state_mut().select_chat_model(0);
    assert_eq!(selected_cli_model_id(&host).as_deref(), Some("opus"));
}

#[test]
fn selected_cli_model_is_none_for_builtin_and_acp_entries() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.available_models = vec![
        op_editor_core::ModelEntry::builtin(
            op_editor_core::AgentProvider::ClaudeCode,
            "builtin-1",
            "builtin:builtin-1:claude-sonnet-4-5",
            "claude-sonnet-4-5",
        ),
        op_editor_core::ModelEntry::acp("acp-1", "Local ACP"),
    ];
    // Built-in providers carry their model in their own config;
    // ACP entries address an agent, not a model. Neither must
    // leak `entry.value` into a CLI transport.
    host.editor_state_mut().select_chat_model(0);
    assert!(selected_cli_model_id(&host).is_none());
    host.editor_state_mut().select_chat_model(1);
    assert!(selected_cli_model_id(&host).is_none());
}

#[test]
fn selected_cli_model_is_none_when_entry_provider_diverges_from_routed_agent() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.available_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::CodexCli,
        "gpt-5.5",
        "GPT-5.5",
    )];
    host.editor_state_mut().select_chat_model(0);
    // Force a divergence: route Antigravity while the selected entry
    // still belongs to Codex. The Codex model id must NOT be
    // passed to Antigravity.
    host.editor_state_mut().editor_ui.chat_selected_agent = 4;
    assert!(selected_cli_model_id(&host).is_none());
}

#[test]
fn selected_cli_model_is_none_for_blank_wire_id() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().chat.available_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::CodexCli,
        "   ",
        "Mystery",
    )];
    host.editor_state_mut().select_chat_model(0);
    // A blank wire id must collapse to None so no transport ever
    // emits an empty `--model` flag.
    assert!(selected_cli_model_id(&host).is_none());
}

#[test]
fn session_surfaces_provider_error() {
    let provider = Box::new(EchoProvider {
        script: vec![ChatDelta::Error("boom".into())],
    });
    let mut session = ChatSession::start(
        provider,
        ChatRequest {
            system_prompt: String::new(),
            user_message: "x".into(),
            max_output_tokens: 0,
            ..Default::default()
        },
    );
    let mut err = None;
    for _ in 0..1000 {
        let p = session.poll();
        if p.error.is_some() {
            err = p.error;
        }
        if p.finished {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(err.as_deref(), Some("boom"));
}

#[test]
fn selected_builtin_model_enables_canvas_tool_loop() {
    let mut host = WidgetHostNative::new();
    let id = host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Built-in Claude", "sk-test", "claude-sonnet-4-5");
    host.editor_state_mut().rebuild_chat_models();
    let idx = host
        .editor_state()
        .chat
        .available_models
        .iter()
        .position(|m| m.builtin_provider_id.as_deref() == Some(id.as_str()))
        .expect("built-in model should be selectable");
    host.editor_state_mut().select_chat_model(idx);

    let (provider, _tool_rx) =
        builtin_provider_with_tools(&host).expect("builtin entry must wire the tool loop");
    assert_eq!(provider.provider_label(), "Built-in Claude");

    // Non-builtin selections never wire tools.
    host.editor_state_mut().chat.available_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "opus",
        "Opus",
    )];
    host.editor_state_mut().select_chat_model(0);
    assert!(builtin_provider_with_tools(&host).is_none());
}

#[test]
fn attach_tool_result_updates_running_card_envelope() {
    let mut chat = op_editor_core::ChatState::default();
    let mut msg = ChatMessage::assistant_streaming();
    msg.tool_calls.push(ChatToolCall {
        name: "update_node".into(),
        args: r#"{"level":"modify","args":{"nodeId":"n1"},"status":"running"}"#.into(),
        content_offset: None,
    });
    chat.messages.push(msg);

    let changed = attach_tool_result_to_transcript(
        &mut chat,
        "update_node",
        &ChatToolResult {
            content: r#"{"success":true,"data":{"wrote":"true"}}"#.into(),
            is_error: false,
        },
    );
    assert!(changed);
    let args = &chat.messages[0].tool_calls[0].args;
    let v: serde_json::Value = serde_json::from_str(args).unwrap();
    assert_eq!(v["status"], "done");
    assert_eq!(v["result"]["success"], true);

    // A second attach finds no running card left — no-op.
    let changed = attach_tool_result_to_transcript(
        &mut chat,
        "update_node",
        &ChatToolResult {
            content: r#"{"success":false,"error":"x"}"#.into(),
            is_error: true,
        },
    );
    assert!(!changed);
}

#[test]
fn attach_tool_result_marks_error_status() {
    let mut chat = op_editor_core::ChatState::default();
    let mut msg = ChatMessage::assistant_streaming();
    msg.tool_calls.push(ChatToolCall {
        name: "delete_node".into(),
        args: r#"{"level":"delete","args":{"nodeId":"n9"},"status":"running"}"#.into(),
        content_offset: None,
    });
    chat.messages.push(msg);
    attach_tool_result_to_transcript(
        &mut chat,
        "delete_node",
        &ChatToolResult {
            content: r#"{"success":false,"error":"unknown node"}"#.into(),
            is_error: true,
        },
    );
    let v: serde_json::Value = serde_json::from_str(&chat.messages[0].tool_calls[0].args).unwrap();
    assert_eq!(v["status"], "error");
    assert_eq!(v["result"]["error"], "unknown node");
}

#[test]
fn pump_executes_scripted_tool_call_against_live_state() {
    // Reproduce both sends landing between the two channel observations. The
    // request must not overtake the still-unpolled ToolUse card.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());
    let before = host.editor_state().active_children().len();
    let (delta_tx, delta_rx) = std::sync::mpsc::channel();
    let (tool_tx, tool_rx) = std::sync::mpsc::channel();
    let (ack_tx, ack_rx) = std::sync::mpsc::sync_channel(1);
    let mut current = Some(ChatSession::from_channels(delta_rx, Some(tool_rx)));
    let args = r##"{"kind":"rect","name":"Card","x":"5","y":"5","width":"50","height":"50","fill_hex":"#00ff00"}"##;
    pump_with_channel_interleave(&mut host, &mut current, None, None, (1200.0, 800.0), || {
        delta_tx
            .send(ChatDelta::ToolUse {
                name: "insert_node".into(),
                args: op_host_services::chat_agent_loop::tool_card_envelope("create", args),
            })
            .unwrap();
        tool_tx
            .send(op_editor_host_core::chat::ChatToolRequest {
                name: "insert_node".into(),
                args_json: args.into(),
                ack: ack_tx,
            })
            .unwrap();
    });
    pump(&mut host, &mut current, None, None, (1200.0, 800.0));

    ack_rx
        .try_recv()
        .expect("the second pump must execute and acknowledge the tool");
    delta_tx
        .send(ChatDelta::TextDelta("tool said: ok".into()))
        .unwrap();
    delta_tx
        .send(ChatDelta::Done {
            stop_reason: StopReason::EndTurn,
        })
        .unwrap();
    pump(&mut host, &mut current, None, None, (1200.0, 800.0));
    assert!(current.is_none(), "turn must finish");

    assert_eq!(host.editor_state().active_children().len(), before + 1);
    use op_editor_core::PenNodeExt;
    assert!(host
        .editor_state()
        .active_children()
        .iter()
        .any(|n| n.base().name.as_deref() == Some("Card")));

    let msg = host.editor_state().chat.messages.last().unwrap();
    assert_eq!(msg.tool_calls.len(), 1);
    let v: serde_json::Value = serde_json::from_str(&msg.tool_calls[0].args).unwrap();
    assert_eq!(v["status"], "done");
    assert_eq!(v["result"]["success"], true);
    assert!(msg.content.contains("tool said:"));
    assert!(!msg.streaming);
}
