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
    // TS codex-client.ts argv: exec --json --skip-git-repo-check
    // --sandbox read-only … - (prompt via stdin).
    let provider = SubprocessProvider::for_cli(CliName::Codex).expect("codex wired");
    assert_eq!(
        provider.args,
        vec![
            "exec",
            "--json",
            "--skip-git-repo-check",
            "--sandbox",
            "read-only"
        ]
    );
    assert_eq!(provider.prompt_mode, PromptMode::Stdin);
    assert_eq!(provider.tail_args, vec!["-"]);
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
    // TS gemini-client.ts streamGeminiExec argv: `-o stream-json
    // --approval-mode plan … -p ' '` (prompt via stdin).
    assert_eq!(
        gemini.args,
        vec!["-o", "stream-json", "--approval-mode", "plan"]
    );
    assert_eq!(gemini.tail_args, vec!["-p", " "]);
}

#[test]
fn for_cli_rejects_opencode_and_copilot() {
    // OpenCode chats over its HTTP server (chat_http_server.rs);
    // Copilot's routed transport is the official SDK
    // (chat_copilot.rs) — the old `gh-copilot suggest` argv was a
    // stale dead end, so the subprocess bridge refuses both.
    assert!(SubprocessProvider::for_cli(CliName::OpenCode).is_none());
    assert!(SubprocessProvider::for_cli(CliName::Copilot).is_none());
}

/// `ChatRequest` carrying only a model selection (knobs defaulted).
fn request_with_model(model: Option<&str>) -> ChatRequest {
    ChatRequest {
        model: model.map(str::to_string),
        ..Default::default()
    }
}

#[test]
fn codex_turn_args_append_model_flag_when_selected() {
    let p = SubprocessProvider::for_cli(CliName::Codex).unwrap();
    let args = p.turn_args(&request_with_model(Some("gpt-5.5")));
    let pos = args
        .iter()
        .position(|a| a == "--model")
        .expect("--model flag present");
    assert_eq!(args[pos + 1], "gpt-5.5");
    // Model flags come after the base template so the exec subcommand
    // shape is untouched; the `-` stdin marker stays last (TS order).
    assert_eq!(args[0], "exec");
    assert_eq!(args.last().map(String::as_str), Some("-"));
}

#[test]
fn codex_turn_args_omit_model_flag_when_unset_or_blank() {
    let p = SubprocessProvider::for_cli(CliName::Codex).unwrap();
    // None → CLI default model, no flag.
    let args = p.turn_args(&request_with_model(None));
    assert!(!args.iter().any(|a| a == "--model"), "args={args:?}");
    // Blank → treated as unset; never emit a bare `--model`.
    let args = p.turn_args(&request_with_model(Some("   ")));
    assert!(!args.iter().any(|a| a == "--model"), "args={args:?}");
}

#[test]
fn codex_turn_args_map_effort_to_reasoning_config() {
    let p = SubprocessProvider::for_cli(CliName::Codex).unwrap();
    // Defaulted knobs (Adaptive + Low) emit no config — the CLI
    // keeps its own default reasoning effort.
    let args = p.turn_args(&ChatRequest::default());
    assert!(!args.iter().any(|a| a == "--config"), "args={args:?}");
    // High passes through; Max folds to Codex's top tier "high";
    // disabled thinking forces "low" (TS resolveCodexEffort parity).
    let cases = [
        (ThinkingMode::Adaptive, EffortLevel::High, "high"),
        (ThinkingMode::Adaptive, EffortLevel::Max, "high"),
        (ThinkingMode::Adaptive, EffortLevel::Medium, "medium"),
        (ThinkingMode::Disabled, EffortLevel::Max, "low"),
        (ThinkingMode::Enabled, EffortLevel::Low, "low"),
    ];
    for (thinking, effort, expected) in cases {
        let args = p.turn_args(&ChatRequest {
            thinking,
            effort,
            ..Default::default()
        });
        let pos = args
            .iter()
            .position(|a| a == "--config")
            .unwrap_or_else(|| panic!("--config missing for {thinking:?}/{effort:?}: {args:?}"));
        assert_eq!(args[pos + 1], format!("model_reasoning_effort={expected}"));
    }
}

#[test]
fn gemini_turn_args_append_m_flag_when_selected() {
    let p = SubprocessProvider::for_cli(CliName::Gemini).unwrap();
    let args = p.turn_args(&request_with_model(Some("gemini-2.5-pro")));
    let pos = args
        .iter()
        .position(|a| a == "-m")
        .expect("-m flag present");
    assert_eq!(args[pos + 1], "gemini-2.5-pro");
    // Gemini has no native effort knob — never a --config arg.
    assert!(!args.iter().any(|a| a == "--config"));
    // The `-p ' '` stdin marker trails the model flag (TS order).
    assert_eq!(args[args.len() - 2], "-p");
    assert_eq!(args[args.len() - 1], " ");
}

#[test]
fn gemini_turn_args_omit_m_flag_when_unset() {
    let p = SubprocessProvider::for_cli(CliName::Gemini).unwrap();
    let args = p.turn_args(&request_with_model(None));
    assert!(!args.iter().any(|a| a == "-m"), "args={args:?}");
}

#[test]
fn claude_subprocess_template_ignores_model() {
    // Claude Code's model rides the SDK adapter (chat_claude.rs); its
    // subprocess template must not grow a model flag here.
    let p = SubprocessProvider::for_cli(CliName::ClaudeCode).unwrap();
    let base = p.args.clone();
    let args = p.turn_args(&request_with_model(Some("some-model")));
    assert_eq!(args, base, "Claude Code args must be unchanged");
}

#[test]
fn with_binary_provider_ignores_model() {
    // Custom binaries have no known model flag; the selection is
    // dropped rather than guessed.
    let p = SubprocessProvider::with_binary("custom-cli", vec!["run".into()], "custom");
    let args = p.turn_args(&request_with_model(Some("m1")));
    assert_eq!(args, vec!["run".to_string()]);
}

#[test]
fn codex_reasoning_effort_table_matches_ts_resolver() {
    // TS codex-client.ts::resolveCodexEffort parity.
    assert_eq!(
        codex_reasoning_effort(ThinkingMode::Disabled, EffortLevel::High),
        Some("low")
    );
    assert_eq!(
        codex_reasoning_effort(ThinkingMode::Adaptive, EffortLevel::Max),
        Some("high")
    );
    assert_eq!(
        codex_reasoning_effort(ThinkingMode::Adaptive, EffortLevel::Medium),
        Some("medium")
    );
    assert_eq!(
        codex_reasoning_effort(ThinkingMode::Enabled, EffortLevel::Low),
        Some("low")
    );
    assert_eq!(
        codex_reasoning_effort(ThinkingMode::Adaptive, EffortLevel::Low),
        None
    );
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
