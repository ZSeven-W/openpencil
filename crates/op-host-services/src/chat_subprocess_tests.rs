use super::*;
// `ThinkingMode` is no longer named in the spine (the Codex reasoning
// mapper moved to `chat_subprocess_quirks`), so the effort-table case
// imports it directly rather than leaning on the glob.
use op_ai::chat_provider::ThinkingMode;

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
    // The base shape stays stable whether this test host has a new, old, or
    // missing Codex binary. Fake-CLI tests below cover both gate outcomes.
    let provider = SubprocessProvider::for_cli(CliName::Codex).expect("codex wired");
    let args_without_ephemeral: Vec<_> = provider
        .args
        .iter()
        .filter(|arg| arg.as_str() != "--ephemeral")
        .map(String::as_str)
        .collect();
    assert_eq!(
        args_without_ephemeral,
        [
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

#[cfg(unix)]
struct CodexHelpStub {
    dir: std::path::PathBuf,
    binary: std::path::PathBuf,
}

#[cfg(unix)]
impl CodexHelpStub {
    fn new(help_line: &str) -> Self {
        use std::os::unix::fs::PermissionsExt;
        use std::sync::atomic::{AtomicU64, Ordering};

        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "openpencil-codex-help-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("create Codex help stub dir");
        // Model the real npm launcher: Codex is an `env node` wrapper, and
        // the matching Node runtime lives beside it. If the subprocess PATH
        // stops leading with this directory, the host machine's Node will try
        // to parse the shell fixture below as JavaScript and the probe fails.
        let node = dir.join("node");
        std::fs::write(&node, "#!/bin/sh\nexec /bin/sh \"$@\"\n").expect("write sibling Node stub");
        std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o755))
            .expect("make sibling Node stub executable");
        let binary = dir.join("codex");
        let script = format!(
            "#!/usr/bin/env node\n\
             if [ \"$1\" != 'exec' ] || [ \"$2\" != '--help' ]; then exit 64; fi\n\
             printf '%s\\n' '{}'\n",
            help_line
        );
        std::fs::write(&binary, script).expect("write Codex help stub");
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755))
            .expect("make Codex help stub executable");
        Self { dir, binary }
    }
}

#[cfg(unix)]
impl Drop for CodexHelpStub {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[cfg(unix)]
#[test]
fn codex_ephemeral_flag_is_capability_gated_and_cached() {
    // The gate result is negative-cached per binary path, and the probe
    // behind it (`codex exec --help` through the real `env node` wrapper
    // chain) is wall-clock bounded. On a saturated machine — parallel
    // `cargo test` plus external load; measured locally with the whole
    // suite running concurrently — the stub's spawn chain can outlive
    // even a generous probe budget, and that one slow probe would pin
    // "unsupported" for the stub's path forever. So when the flag fails
    // to appear, retry with a FRESH stub (fresh path = fresh cache slot)
    // instead of re-asking the poisoned entry. A real gating regression
    // fails every attempt deterministically, so nothing the test proves
    // is weakened; the retries only absorb scheduler starvation. The
    // attempt count is generous because it is nearly free where it
    // matters: a regressed gate completes each probe in well under a
    // second (the stub exits immediately), so all attempts together
    // still fail fast — only a starved machine pays a probe budget per
    // attempt, and that is exactly the case the retries exist to
    // outlast (the suite's own spawn-storm phase can starve a fresh
    // child for tens of seconds; measured 10+ consecutive 10s probe
    // timeouts with three test binaries running concurrently).
    let supported = (0..12)
        .find_map(|_| {
            let stub = CodexHelpStub::new("      --ephemeral  Do not persist session files");
            let mut args = vec!["exec".into(), "--json".into()];
            quirks::append_codex_ephemeral_arg(&stub.binary, &mut args);
            (args == ["exec", "--ephemeral", "--json"]).then_some(stub)
        })
        .expect("a Codex advertising --ephemeral must have the flag gated in");

    // Removing the stand-in proves the second lookup uses the path cache.
    std::fs::remove_file(&supported.binary).expect("remove supported stub");
    let mut cached = vec!["exec".into(), "--json".into()];
    quirks::append_codex_ephemeral_arg(&supported.binary, &mut cached);
    assert_eq!(cached, ["exec", "--ephemeral", "--json"]);

    // No retry needed here: a probe failure and a genuine "not
    // advertised" verdict both leave the flag out, which is exactly what
    // the assertion requires — it cannot flake, only miss a regression
    // that the retried positive case above would catch.
    let unsupported = CodexHelpStub::new("Usage: codex exec [OPTIONS]");
    let mut old_args = vec!["exec".into(), "--json".into()];
    quirks::append_codex_ephemeral_arg(&unsupported.binary, &mut old_args);
    assert_eq!(old_args, ["exec", "--json"]);
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
fn for_cli_rejects_opencode_and_copilot() {
    // OpenCode chats over its HTTP server (chat_http_server.rs);
    // Copilot's routed transport is the official SDK
    // (chat_copilot.rs) — the old `gh-copilot suggest` argv was a
    // stale dead end, so the subprocess bridge refuses both.
    assert!(SubprocessProvider::for_cli(CliName::OpenCode).is_none());
    assert!(SubprocessProvider::for_cli(CliName::Copilot).is_none());
}

#[test]
fn for_cli_dsh_constructs_the_one_shot_headless_bridge() {
    // Verified dsh interface: `dsh --profile headless "<prompt>"` —
    // prompt as a bare trailing argv element, no model selector.
    let dsh = SubprocessProvider::for_cli(CliName::Dsh).expect("dsh wired");
    assert!(dsh.binary.ends_with("dsh"), "binary={}", dsh.binary);
    assert_eq!(dsh.args, ["--profile", "headless"]);
    assert_eq!(dsh.prompt_mode, PromptMode::BareArg);
    assert_eq!(dsh.model_flag, None);
    assert_eq!(dsh.label, "DeepSeek Harness");
    // No stdin transport: the one-shot CLI gets a closed stdin.
    let turn = dsh.turn_args(&ChatRequest {
        user_message: "hi".into(),
        ..ChatRequest::default()
    });
    assert_eq!(turn, vec!["--profile", "headless"]);
}

#[test]
fn dsh_chat_routes_through_the_fail_closed_environment() {
    let mut actual = child_env_for_cli(Some(CliName::Dsh));
    let mut expected = safety::child_env(Some(CliName::Dsh)).expect("DSH guarded env");
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected);
}

#[test]
fn subprocess_providers_advertise_cooperative_cancellation() {
    let provider = SubprocessProvider::for_cli(CliName::Codex).expect("codex provider");
    assert!(provider.supports_cancellable_send());
}

#[cfg(unix)]
#[test]
fn terminal_reap_obeys_its_deadline() {
    crate::chat_runtime::block_on_anywhere(async {
        let pid_file = std::env::temp_dir().join(format!(
            "openpencil-terminal-descendant-{}.pid",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&pid_file);
        let args = vec![
            "-c".to_string(),
            "sleep 30 & echo $! > \"$1\"; wait".to_string(),
            "openpencil-test".to_string(),
            pid_file.to_string_lossy().into_owned(),
        ];
        let mut child =
            LineStreamChild::spawn_command(build_command("/bin/sh", &args)).expect("spawn sleeper");
        // Liveness bound, not the property under test: a loaded machine
        // can take whole seconds to schedule the stub far enough to write
        // its pid file, so this wait is generous. It waits for the
        // newline-terminated payload rather than existence — the shell's
        // `>` redirection creates the file before `echo` writes into it,
        // and an existence check raced that into reading an empty pid.
        let spawn_deadline = std::time::Instant::now() + Duration::from_secs(30);
        loop {
            let written =
                std::fs::read_to_string(&pid_file).is_ok_and(|content| content.ends_with('\n'));
            if written || std::time::Instant::now() >= spawn_deadline {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let descendant: i32 = std::fs::read_to_string(&pid_file)
            .expect("descendant pid file")
            .trim()
            .parse()
            .expect("descendant pid");
        let (tx, _rx) = mpsc::channel(1);
        let started = std::time::Instant::now();
        let status = wait_for_terminal_exit(
            &mut child,
            tokio::time::Instant::now() + Duration::from_millis(25),
            &tx,
        )
        .await;
        assert!(
            status.is_some(),
            "deadline must force-kill and reap the child"
        );
        // Far below the 30s child lifetime (the property: the deadline,
        // not the child, ends the reap), yet wide enough that scheduler
        // starvation under a loaded parallel run cannot fail it.
        assert!(started.elapsed() < Duration::from_secs(10));
        // Generous liveness bound for kill delivery + init's zombie reap
        // under load; the assert below still fails if the descendant
        // genuinely survives the tree reap.
        let reap_deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            // SAFETY: signal 0 is a read-only existence probe for the exact
            // positive pid written by this test's own child.
            if unsafe { libc::kill(descendant, 0) } != 0
                || std::time::Instant::now() >= reap_deadline
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_ne!(
            unsafe { libc::kill(descendant, 0) },
            0,
            "terminal reap must not leave a descendant alive"
        );
        let _ = std::fs::remove_file(pid_file);
    });
}

#[cfg(unix)]
#[test]
fn terminal_reap_obeys_receiver_cancellation() {
    crate::chat_runtime::block_on_anywhere(async {
        let args = vec!["-c".to_string(), "sleep 30".to_string()];
        let mut child =
            LineStreamChild::spawn_command(build_command("/bin/sh", &args)).expect("spawn sleeper");
        let (tx, rx) = mpsc::channel(1);
        drop(rx);
        let started = std::time::Instant::now();
        let status = wait_for_terminal_exit(
            &mut child,
            tokio::time::Instant::now() + Duration::from_secs(30),
            &tx,
        )
        .await;
        assert!(
            status.is_some(),
            "cancellation must force-kill and reap the child"
        );
        // Same rationale as the deadline case above: well under the 30s
        // child lifetime, tolerant of load-induced scheduling lag.
        assert!(started.elapsed() < Duration::from_secs(10));
    });
}

#[test]
fn antigravity_and_grok_use_documented_one_shot_flags() {
    let antigravity = SubprocessProvider::for_cli(CliName::Antigravity).unwrap();
    assert!(antigravity.binary.ends_with("agy"));
    assert_eq!(antigravity.args, ["--sandbox", "--print-timeout", "90s"]);
    assert_eq!(antigravity.prompt_mode, PromptMode::FlagArg("-p"));
    assert_eq!(antigravity.model_flag, Some("--model"));

    let grok = SubprocessProvider::for_cli(CliName::GrokBuild).unwrap();
    assert!(grok.binary.ends_with("grok"));
    assert!(grok
        .args
        .windows(2)
        .any(|pair| pair == ["--permission-mode", "dontAsk"]));
    assert!(grok
        .args
        .windows(2)
        .any(|pair| pair == ["--allow", safety::GROK_MCP_ALLOW]));
    assert!(grok
        .args
        .windows(2)
        .any(|pair| pair == ["--sandbox", "strict"]));
    assert!(grok
        .args
        .windows(2)
        .any(|pair| pair == ["--tools", safety::GROK_READ_TOOLS]));
    for forbidden in ["run_terminal_cmd", "search_replace", "web_search"] {
        assert!(!grok.args.iter().any(|arg| arg == forbidden));
    }
    assert!(grok.args.iter().any(|arg| arg == "--no-subagents"));
    assert!(grok.args.iter().any(|arg| arg == "--disable-web-search"));
    assert_eq!(grok.prompt_mode, PromptMode::PromptFile("--prompt-file"));
    assert_eq!(grok.model_flag, Some("-m"));
}

#[test]
fn generation_mode_disables_canvas_mcp_but_keeps_cli_containment() {
    let grok = SubprocessProvider::for_cli_generation(CliName::GrokBuild).unwrap();
    assert_eq!(grok.turn_purpose, safety::TurnPurpose::Generation);
    assert!(!grok.args.iter().any(|arg| arg == safety::GROK_MCP_ALLOW));
    let tools = grok.args.iter().position(|arg| arg == "--tools").unwrap();
    assert_eq!(grok.args[tools + 1], "");
    assert!(grok
        .args
        .windows(2)
        .any(|pair| pair == ["--sandbox", "strict"]));
    assert!(grok.args.iter().any(|arg| arg == "--disable-web-search"));

    let antigravity = SubprocessProvider::for_cli_generation(CliName::Antigravity).unwrap();
    assert_eq!(antigravity.turn_purpose, safety::TurnPurpose::Generation);
    assert_eq!(
        antigravity.args,
        ["--sandbox", "--print-timeout", "90s", "--mode", "plan"]
    );
}

#[test]
fn grok_default_model_keeps_cli_default_but_named_model_uses_m_flag() {
    let grok = SubprocessProvider::for_cli(CliName::GrokBuild).unwrap();
    let default_args = grok.turn_args(&request_with_model(Some("default")));
    assert!(!default_args.iter().any(|arg| arg == "-m"));

    let selected_args = grok.turn_args(&request_with_model(Some("grok-code-fast-1")));
    let flag = selected_args.iter().position(|arg| arg == "-m").unwrap();
    assert_eq!(selected_args[flag + 1], "grok-code-fast-1");
}

#[test]
fn antigravity_default_model_keeps_cli_default_but_named_model_uses_model_flag() {
    let antigravity = SubprocessProvider::for_cli(CliName::Antigravity).unwrap();
    let default_args = antigravity.turn_args(&request_with_model(Some("default")));
    assert!(!default_args.iter().any(|arg| arg == "--model"));

    let selected_args = antigravity.turn_args(&request_with_model(Some("Gemini 3.5 Flash (High)")));
    let flag = selected_args
        .iter()
        .position(|arg| arg == "--model")
        .unwrap();
    assert_eq!(selected_args[flag + 1], "Gemini 3.5 Flash (High)");
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
