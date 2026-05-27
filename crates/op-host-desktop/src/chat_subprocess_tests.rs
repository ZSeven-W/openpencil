use super::*;

#[test]
fn parse_line_text_delta() {
    match parse_line(r#"{"type":"text","delta":"Hello"}"#) {
        ChatDelta::TextDelta(s) => assert_eq!(s, "Hello"),
        other => panic!("expected TextDelta, got {other:?}"),
    }
}

#[test]
fn parse_line_thinking_delta() {
    match parse_line(r#"{"type":"thinking","delta":"reasoning..."}"#) {
        ChatDelta::Thinking(s) => assert_eq!(s, "reasoning..."),
        other => panic!("expected Thinking, got {other:?}"),
    }
}

#[test]
fn parse_line_tool_use() {
    match parse_line(r#"{"type":"tool_use","name":"bash","args":{"cmd":"ls"}}"#) {
        ChatDelta::ToolUse { name, args } => {
            assert_eq!(name, "bash");
            assert!(args.contains("ls"));
        }
        other => panic!("expected ToolUse, got {other:?}"),
    }
}

#[test]
fn parse_line_done_with_stop_reason() {
    match parse_line(r#"{"type":"done","stop_reason":"max_tokens"}"#) {
        ChatDelta::Done { stop_reason } => {
            assert!(matches!(stop_reason, StopReason::MaxTokens));
        }
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn parse_line_codex_agent_message_completed() {
    match parse_line(
        r#"{"type":"item.completed","item":{"id":"item_0","type":"agent_message","text":"hi"}}"#,
    ) {
        ChatDelta::TextDelta(s) => assert_eq!(s, "hi"),
        other => panic!("expected TextDelta, got {other:?}"),
    }
}

#[test]
fn parse_line_codex_turn_completed() {
    match parse_line(r#"{"type":"turn.completed","usage":{"input_tokens":1,"output_tokens":2}}"#) {
        ChatDelta::Done { stop_reason } => {
            assert!(matches!(stop_reason, StopReason::EndTurn));
        }
        other => panic!("expected Done, got {other:?}"),
    }
}

#[test]
fn for_cli_constructs_codex_exec_provider() {
    let provider = SubprocessProvider::for_cli(CliName::Codex);
    assert!(
        provider.is_some(),
        "Codex CLI should be wired through exec --json"
    );
}

#[test]
fn parse_line_error() {
    match parse_line(r#"{"type":"error","message":"rate limited"}"#) {
        ChatDelta::Error(s) => assert_eq!(s, "rate limited"),
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn parse_line_plain_text_falls_through_to_text_delta() {
    match parse_line("Just some plain text") {
        ChatDelta::TextDelta(s) => assert_eq!(s, "Just some plain text\n"),
        other => panic!("expected TextDelta, got {other:?}"),
    }
}

#[test]
fn parse_line_malformed_json_falls_through() {
    // Looks like JSON ({ ... }) but isn't valid → raw text.
    match parse_line("{not json") {
        ChatDelta::TextDelta(s) => assert_eq!(s, "{not json\n"),
        other => panic!("expected TextDelta, got {other:?}"),
    }
}

#[test]
fn parse_line_unknown_type_falls_through() {
    match parse_line(r#"{"type":"frobnicate","payload":42}"#) {
        ChatDelta::TextDelta(s) => assert!(s.contains("frobnicate")),
        other => panic!("expected TextDelta fallback, got {other:?}"),
    }
}

#[test]
fn for_cli_claude_code_seeds_print_stream_json_flags() {
    let p = SubprocessProvider::for_cli(CliName::ClaudeCode).unwrap();
    // `binary` is the resolved absolute path when claude is on
    // PATH (or one of the npm-global / nvm fallback locations);
    // otherwise it falls back to the bare name. Either way the
    // file name component must match.
    assert!(p.binary.ends_with("claude"), "binary={}", p.binary);
    assert!(p.args.iter().any(|a| a == "--print"));
    assert!(p.args.iter().any(|a| a == "--verbose"));
    assert!(p.args.iter().any(|a| a == "stream-json"));
    assert_eq!(p.label, "Claude Code");
    assert_eq!(p.prompt_mode, PromptMode::PositionalArg);
}

#[test]
fn for_cli_uses_default_binary_per_cli_name() {
    // Resolved path may be bare or absolute depending on what's
    // installed on the test host; check the basename only.
    let gemini = SubprocessProvider::for_cli(CliName::Gemini).unwrap();
    assert!(
        gemini.binary.ends_with("gemini"),
        "binary={}",
        gemini.binary
    );
    assert_eq!(gemini.prompt_mode, PromptMode::Stdin);
    let copilot = SubprocessProvider::for_cli(CliName::Copilot).unwrap();
    assert!(
        copilot.binary.ends_with("gh-copilot"),
        "binary={}",
        copilot.binary
    );
    assert_eq!(copilot.prompt_mode, PromptMode::Stdin);
}

#[test]
fn for_cli_rejects_opencode() {
    assert!(SubprocessProvider::for_cli(CliName::OpenCode).is_none());
}

#[test]
fn parse_line_malformed_structured_event_is_error() {
    // "text" with no "delta" → Error
    match parse_line(r#"{"type":"text"}"#) {
        ChatDelta::Error(s) => assert!(s.contains("malformed text")),
        other => panic!("expected Error, got {other:?}"),
    }
    // "tool_use" missing "name" → Error
    match parse_line(r#"{"type":"tool_use","args":{}}"#) {
        ChatDelta::Error(s) => assert!(s.contains("malformed tool_use")),
        other => panic!("expected Error, got {other:?}"),
    }
    // "error" missing "message" → Error
    match parse_line(r#"{"type":"error"}"#) {
        ChatDelta::Error(s) => assert!(s.contains("malformed error")),
        other => panic!("expected Error, got {other:?}"),
    }
}

#[test]
fn spawn_failure_surfaces_error_and_done() {
    // Use a binary path that's guaranteed not to exist on PATH.
    // Cross-platform expectation:
    //   - macOS / Linux: `Command::new` returns spawn-time Err
    //     because execvp fails → `ChatDelta::Error("spawn ...")`
    //     emitted before EOF.
    //   - Windows: `build_command` routes through `cmd /c`, which
    //     successfully spawns. The inner CLI then exits non-zero
    //     ("not recognized as an internal or external command")
    //     → `ChatDelta::Error("CLI exited with status N")` from
    //     the exit-status path.
    // Either way the bridge must emit at least one Error delta
    // plus a terminal Done.
    let p =
        SubprocessProvider::with_binary("definitely-not-a-binary-3kf9j2-xyz", Vec::new(), "bogus");
    let deltas: Vec<ChatDelta> = p
        .send(ChatRequest {
            system_prompt: String::new(),
            user_message: "hi".into(),
            max_output_tokens: 64,
            ..Default::default()
        })
        .collect();
    assert!(
        deltas.iter().any(|d| matches!(d, ChatDelta::Error(_))),
        "expected at least one Error delta, got {:?}",
        deltas
    );
    assert!(
        deltas.iter().any(|d| matches!(d, ChatDelta::Done { .. })),
        "expected terminal Done in deltas, got {:?}",
        deltas
    );
}
