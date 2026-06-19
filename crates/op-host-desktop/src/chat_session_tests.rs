use super::*;
use op_web_daemon::chat_system_prompt::{build_chat_system_prompt, chat_history_from_transcript};
use op_ai::chat_history::{trim_chat_history, DEFAULT_MAX_CHARS, DEFAULT_MAX_MESSAGES};
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
        &mut current_design
    ));

    assert!(current.is_none());
    assert!(!host.editor_state().chat.pending_stop_chat);
    assert_eq!(host.editor_state().chat.messages.len(), 1);
    assert!(!host.editor_state().chat.messages[0].streaming);
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
    // Force a divergence: route Gemini while the selected entry
    // still belongs to Codex. The Codex model id must NOT be
    // passed to the Gemini CLI.
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

/// Scripted tool-looping provider: emits a ToolUse card, blocks on the
/// injected executor (the UI tool channel), then streams the result
/// as text — the provider side of GAP #32 without any network.
struct ToolLoopProvider {
    executor: std::sync::Arc<dyn op_ai::chat_provider::ChatToolExecutor>,
    tool: &'static str,
    args: &'static str,
}

struct ToolLoopIter {
    executor: std::sync::Arc<dyn op_ai::chat_provider::ChatToolExecutor>,
    tool: &'static str,
    args: &'static str,
    step: u8,
}

impl Iterator for ToolLoopIter {
    type Item = ChatDelta;
    fn next(&mut self) -> Option<ChatDelta> {
        self.step += 1;
        match self.step {
            1 => Some(ChatDelta::ToolUse {
                name: self.tool.into(),
                args: crate::chat_agent_loop::tool_card_envelope("create", self.args),
            }),
            2 => {
                // Blocks until the UI thread (the test's pump loop)
                // executes the call and acks.
                let result = self.executor.execute(self.tool, self.args);
                Some(ChatDelta::TextDelta(format!(
                    "tool said: {}",
                    result.content
                )))
            }
            3 => Some(ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            }),
            _ => None,
        }
    }
}

impl op_ai::chat_provider::ChatProvider for ToolLoopProvider {
    fn provider_label(&self) -> &str {
        "tool-loop"
    }
    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(ToolLoopIter {
            executor: self.executor.clone(),
            tool: self.tool,
            args: self.args,
            step: 0,
        })
    }
}

#[test]
fn pump_executes_scripted_tool_call_against_live_state() {
    // End-to-end-ish host test: a scripted insert_node tool call flows
    // worker → tool channel → pump → execute_chat_tool → apply path,
    // mutating the live document and updating the transcript card.
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());
    let before = host.editor_state().active_children().len();

    let (executor, tool_rx) = op_web_daemon::chat_canvas_tools::chat_tool_channel();
    let provider = Box::new(ToolLoopProvider {
        executor: std::sync::Arc::new(executor),
        tool: "insert_node",
        args: r##"{"kind":"rect","name":"Card","x":"5","y":"5","width":"50","height":"50","fill_hex":"#00ff00"}"##,
    });
    let mut current = Some(ChatSession::start_with_tools(
        provider,
        ChatRequest {
            user_message: "add a card".into(),
            max_output_tokens: 64,
            ..Default::default()
        },
        Some(tool_rx),
    ));

    for _ in 0..2000 {
        pump(&mut host, &mut current);
        if current.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(current.is_none(), "turn must finish");

    // The tool call mutated the live document through the apply path.
    assert_eq!(host.editor_state().active_children().len(), before + 1);
    use op_editor_core::PenNodeExt;
    assert!(host
        .editor_state()
        .active_children()
        .iter()
        .any(|n| n.base().name.as_deref() == Some("Card")));

    // Transcript: card recorded, result attached, answer text landed.
    let msg = host.editor_state().chat.messages.last().unwrap();
    assert_eq!(msg.tool_calls.len(), 1);
    let v: serde_json::Value = serde_json::from_str(&msg.tool_calls[0].args).unwrap();
    assert_eq!(v["status"], "done");
    assert_eq!(v["result"]["success"], true);
    assert!(msg.content.contains("tool said:"));
    assert!(!msg.streaming);
}

#[test]
fn launch_populates_system_prompt_and_history() {
    // The per-turn request assembly (GAP #31): transcript → trimmed
    // history; the system prompt resolves per-turn. Verified through
    // the helpers launch_if_pending composes.
    let mut host = WidgetHostNative::new();
    let chat = &mut host.editor_state_mut().chat;
    chat.messages.push(ChatMessage::user("first question"));
    chat.messages.push(ChatMessage::assistant("first answer"));
    chat.messages.push(ChatMessage::user("make it red"));
    chat.messages.push(ChatMessage::assistant_streaming());

    let history = trim_chat_history(
        &chat_history_from_transcript(&host.editor_state().chat.messages),
        DEFAULT_MAX_MESSAGES,
        DEFAULT_MAX_CHARS,
    );
    assert_eq!(
        history.len(),
        2,
        "prior turns only — in-flight turn excluded"
    );
    assert_eq!(history[0].1, "first question");
    assert_eq!(history[1].1, "first answer");

    let system = build_chat_system_prompt(host.editor_state(), "make it red");
    assert!(system.starts_with("You are a design assistant for OpenPencil"));
}

#[test]
fn design_turn_clears_fresh_starter_frame() {
    let mut state = op_editor_core::EditorState::starter();

    assert!(clear_fresh_starter_frame_for_design(&mut state));
    assert!(state.active_children().is_empty());
    assert!(state.selection.is_empty());
}

#[test]
fn design_turn_preserves_non_starter_documents() {
    let mut state = op_editor_core::EditorState::starter();
    state.active_children_mut().clear();

    assert!(!clear_fresh_starter_frame_for_design(&mut state));
    assert!(state.active_children().is_empty());
}
