//! Tests for the connect-time provider probe — pure-parser and
//! string-builder coverage (no subprocess spawns).

use super::*;

/// Build an unsigned JWT with the given payload JSON.
fn jwt_with_payload(payload: &str) -> String {
    let engine = &base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!(
        "{}.{}.{}",
        engine.encode(r#"{"alg":"none"}"#),
        engine.encode(payload),
        engine.encode("sig")
    )
}

#[test]
fn codex_auth_info_reads_email_and_plan_from_jwt() {
    let token = jwt_with_payload(
        r#"{"email":"dev@example.com","https://api.openai.com/auth":{"chatgpt_plan_type":"pro"}}"#,
    );
    let auth = format!(r#"{{"auth_mode":"chatgpt","tokens":{{"id_token":"{token}"}}}}"#);
    assert_eq!(
        codex_connection_info_from_auth(Locale::EnUs, Some(&auth)),
        "Connected via pro (dev@example.com)"
    );
}

#[test]
fn codex_auth_info_falls_back_to_auth_mode_then_generic() {
    // Email present but no plan claim → auth_mode label.
    let token = jwt_with_payload(r#"{"email":"dev@example.com"}"#);
    let auth = format!(r#"{{"auth_mode":"chatgpt","tokens":{{"id_token":"{token}"}}}}"#);
    assert_eq!(
        codex_connection_info_from_auth(Locale::EnUs, Some(&auth)),
        "Connected via chatgpt (dev@example.com)"
    );
    // No tokens → bare auth_mode.
    assert_eq!(
        codex_connection_info_from_auth(Locale::EnUs, Some(r#"{"auth_mode":"api-key"}"#)),
        "Connected via api-key"
    );
    // No auth.json at all.
    assert_eq!(
        codex_connection_info_from_auth(Locale::EnUs, None),
        "Connected via Codex CLI"
    );
}

#[test]
fn jwt_decode_rejects_malformed_tokens() {
    assert!(decode_jwt_payload("only-one-part").is_none());
    assert!(decode_jwt_payload("a.b").is_none());
    assert!(decode_jwt_payload("a.!!!notbase64!!!.c").is_none());
}

#[test]
fn mask_key_mirrors_ts_slicing() {
    assert_eq!(mask_key("sk-proj-abcdefghij"), "sk-proj-...");
    assert_eq!(mask_key("short"), "***");
}

#[test]
fn opencode_summary_lists_first_three_providers() {
    let mk = |slug: &str| ModelEntry::new(AgentProvider::OpenCode, slug, slug);
    let models = vec![
        mk("anthropic/claude-sonnet-4-6"),
        mk("anthropic/claude-haiku-4-5"),
        mk("openai/gpt-5.5"),
        mk("google/gemini-2.5-pro"),
        mk("mistral/large"),
    ];
    assert_eq!(
        opencode_provider_summary(Locale::EnUs, &models),
        "Connected (anthropic, openai, google +1)"
    );
    assert_eq!(
        opencode_provider_summary(Locale::EnUs, &models[..3]),
        "Connected (anthropic, openai)"
    );
    assert_eq!(
        opencode_provider_summary(Locale::EnUs, &[]),
        "Connected via OpenCode server"
    );
}

#[test]
fn connected_probe_outcome_rejects_empty_model_list() {
    let outcome = connected_probe_outcome(
        Locale::EnUs,
        AgentProvider::CodexCli,
        Vec::new(),
        Some("Connected via Codex CLI".to_string()),
        None,
        None,
        None,
    );

    assert!(!outcome.connected);
    assert!(outcome.models.is_empty());
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|e| e.contains("No models found")),
        "empty model probes must surface a failure, got {outcome:?}"
    );
}

#[test]
fn copilot_connection_info_mirrors_ts_branches() {
    let full = CopilotAuth {
        login: Some("octocat".into()),
        auth_type: Some("oauth".into()),
        status_message: None,
    };
    assert_eq!(
        copilot_connection_info(Locale::EnUs, Some(&full)),
        "Connected as @octocat (oauth)"
    );
    let message_only = CopilotAuth {
        login: None,
        auth_type: None,
        status_message: Some("Signed in via device flow".into()),
    };
    assert_eq!(
        copilot_connection_info(Locale::EnUs, Some(&message_only)),
        "Signed in via device flow"
    );
    assert_eq!(
        copilot_connection_info(Locale::EnUs, None),
        "Connected via GitHub"
    );
}

#[test]
fn friendly_error_mappers_match_ts_tables() {
    assert!(
        friendly_claude_error(Locale::EnUs, "process exited with code 1").contains("claude login")
    );
    assert_eq!(
        friendly_claude_error(Locale::EnUs, "process exited with code 7"),
        "Unable to connect. Claude Code process exited unexpectedly."
    );
    assert!(friendly_copilot_error(Locale::EnUs, "spawn ENOENT").contains("not found"));
    assert!(
        friendly_copilot_error(Locale::EnUs, "not authenticated yet").contains("copilot login")
    );
    assert_eq!(
        friendly_copilot_error(Locale::EnUs, "Connection timed out"),
        // "timed out" also matches the auth branch's bare "auth"?
        // No — it doesn't contain "auth"; the timeout branch wins.
        "Connection timed out. Please try again."
    );
}

#[test]
fn install_commands_mirror_install_agent_ts() {
    assert_eq!(
        install_command(AgentProvider::ClaudeCode),
        "npm install -g @anthropic-ai/claude-code"
    );
    assert_eq!(
        install_command(AgentProvider::CodexCli),
        "npm install -g @openai/codex"
    );
    let antigravity = if cfg!(windows) {
        "irm https://antigravity.google/cli/install.ps1 | iex"
    } else {
        "curl -fsSL https://antigravity.google/cli/install.sh | bash"
    };
    let grok = if cfg!(windows) {
        "irm https://x.ai/cli/install.ps1 | iex"
    } else {
        "curl -fsSL https://x.ai/cli/install.sh | bash"
    };
    assert_eq!(install_command(AgentProvider::Antigravity), antigravity);
    assert_eq!(install_command(AgentProvider::GrokBuild), grok);
    // The verified dsh install hint: the npm global package.
    assert_eq!(
        install_command(AgentProvider::DeepSeekHarness),
        "npm install -g @deepseek-ai/dsh"
    );
}

#[test]
fn antigravity_and_grok_install_commands_are_platform_native() {
    assert_eq!(
        install_command_for_platform(AgentProvider::Antigravity, true, false),
        "irm https://antigravity.google/cli/install.ps1 | iex"
    );
    assert_eq!(
        install_command_for_platform(AgentProvider::GrokBuild, true, false),
        "irm https://x.ai/cli/install.ps1 | iex"
    );
    assert_eq!(
        install_command_for_platform(AgentProvider::Antigravity, false, false),
        "curl -fsSL https://antigravity.google/cli/install.sh | bash"
    );
    assert_eq!(
        install_command_for_platform(AgentProvider::GrokBuild, false, true),
        "curl -fsSL https://x.ai/cli/install.sh | bash"
    );
}

#[test]
fn not_installed_outcome_carries_guidance() {
    let outcome = ProbeOutcome::not_installed(
        AgentProvider::ClaudeCode,
        "Claude Code CLI not found".to_string(),
    );
    assert!(outcome.not_installed);
    assert!(!outcome.connected);
    assert_eq!(
        outcome.install_command.as_deref(),
        Some("npm install -g @anthropic-ai/claude-code")
    );
    assert_eq!(outcome.error.as_deref(), Some("Claude Code CLI not found"));
}

#[test]
fn version_failure_message_carries_the_cli_own_diagnostics() {
    // The GUI-launch failure signature: `codex` is a node-shebang script,
    // the Dock-inherited PATH has no node, execve fails with 127. The
    // card used to read "Codex CLI not responding", which pointed at the
    // wrong thing entirely.
    let failure = CliVersionFailure::Exited {
        status: "127".to_string(),
        tail: "env: node: No such file or directory".to_string(),
    };
    assert_eq!(
        cli_version_failure_message("Codex", &failure),
        "Codex CLI failed: env: node: No such file or directory"
    );
}

#[test]
fn version_failure_message_falls_back_when_the_cli_said_nothing() {
    assert_eq!(
        cli_version_failure_message(
            "Codex",
            &CliVersionFailure::Exited {
                status: "1".to_string(),
                tail: String::new(),
            }
        ),
        "Codex CLI exited with status 1 and no output"
    );
    assert_eq!(
        cli_version_failure_message(
            "Codex",
            &CliVersionFailure::Spawn("permission denied".into())
        ),
        "Codex CLI failed to start: permission denied"
    );
    assert_eq!(
        cli_version_failure_message(
            "Codex",
            &CliVersionFailure::TimedOut {
                seconds: 5,
                tail: String::new(),
            }
        ),
        "Codex CLI did not respond within 5s"
    );
}

/// Run a real subprocess without discovering an installed CLI or asking
/// the OS to launch a freshly written executable.
#[cfg(unix)]
fn cli_version_fixture(body: &str, timeout: Duration) -> Result<String, CliVersionFailure> {
    let mut command = std::process::Command::new("/bin/sh");
    command.args(["-c", body, "cli-version-fixture", "--version"]);
    cli_version_command(command, timeout)
}

#[cfg(unix)]
#[test]
fn cli_version_reports_stderr_from_a_nonzero_exit() {
    // A stand-in for the broken-shebang case: prints to stderr, exits
    // non-zero. The captured tail must survive into the failure.
    let failure = cli_version_fixture(
        "printf 'env: node: No such file or directory\n' >&2
exit 127
",
        Duration::from_secs(10),
    )
    .expect_err("a 127 exit is not a usable version");
    let CliVersionFailure::Exited { status, tail } = &failure else {
        panic!("expected a non-zero exit, got {failure:?}");
    };
    assert_eq!(status, "127");
    assert!(
        tail.contains("env: node: No such file or directory"),
        "stderr must survive into the card text, got {tail:?}"
    );
    assert!(cli_version_failure_message("Codex", &failure).contains("node"));
}

#[cfg(unix)]
#[test]
fn cli_version_surfaces_node_optional_dependency_root_cause() {
    // Codex 0.149.x on Node 24 reports the actionable cause first, then
    // enough loader frames that the old last-200-character snippet showed
    // only ModuleJob.run / tracePromise. Keep the first error line, redact
    // credentials in it, and still enforce the shared UI diagnostic cap.
    let secret = "sk-proj-issue-228-secret-value";
    let root = format!(
        "Error: Missing optional dependency @openai/codex-darwin-arm64. \
         Reinstall @openai/codex. OPENAI_API_KEY={secret} {}",
        "context".repeat(120)
    );
    let body = format!(
        "printf '%s\\n' '{root}' \
         '    at file:///opt/homebrew/lib/node_modules/@openai/codex/bin/codex.js:401:17' \
         '    at ModuleJob.run (node:internal/modules/esm/module_job:413:25)' \
         '    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:654:26)' >&2\n\
         exit 1\n"
    );
    let failure = cli_version_fixture(&body, Duration::from_secs(10))
        .expect_err("a missing platform package must fail the version gate");
    let message = cli_version_failure_message("Codex", &failure);

    assert!(
        message.contains("Missing optional dependency @openai/codex-darwin-arm64"),
        "the provider card must retain the actionable first line, got {message:?}"
    );
    assert!(
        !message.contains("ModuleJob.run") && !message.contains("tracePromise"),
        "loader stack frames must not replace the root cause, got {message:?}"
    );
    assert!(
        !message.contains(secret) && message.contains("OPENAI_API_KEY=<redacted>"),
        "the surfaced diagnostic must redact credentials, got {message:?}"
    );
    assert!(
        message.chars().count()
            <= "Codex CLI failed: ".chars().count() + op_util::cli_output::TAIL_MAX_CHARS,
        "the provider card diagnostic must stay bounded, got {} characters",
        message.chars().count()
    );
}

#[cfg(unix)]
#[test]
fn cli_version_does_not_wait_for_a_descendant_holding_its_pipes() {
    // The wrapper has already delivered a valid version and exited, but its
    // background helper inherits stdout/stderr. A blocking reader join would
    // turn this quick responsiveness gate into a 30-second wait.
    let started = Instant::now();
    let version = cli_version_fixture(
        "printf 'codex-cli 1.2.3\n'
sleep 30 &
exit 0
",
        Duration::from_secs(10),
    );
    let elapsed = started.elapsed();

    assert_eq!(version, Ok("codex-cli 1.2.3".to_string()));
    assert!(
        elapsed < std::time::Duration::from_secs(15),
        "the exited wrapper, not its descendant's pipe EOF, must finish the gate; took {elapsed:?}"
    );
}

#[cfg(unix)]
#[test]
fn cli_version_accepts_a_healthy_cli_and_bounds_a_hung_one() {
    let version = cli_version_fixture(
        "printf 'codex-cli 0.9.1\n'
",
        Duration::from_secs(5),
    )
    .expect("healthy CLI must yield a version");
    assert_eq!(version, "codex-cli 0.9.1");

    // Mid first-run OAuth: prints a prompt, then hangs past the budget.
    let budget = Duration::from_secs(4);
    let started = Instant::now();
    let failure = cli_version_fixture(
        "printf 'Sign in at https://x\n'
exec sleep 30
",
        budget,
    )
    .expect_err("a CLI still running at the deadline is not a usable version");
    assert!(
        started.elapsed() < budget * 4,
        "the deadline, not the hung CLI, must bound the probe"
    );
    let CliVersionFailure::TimedOut { seconds, tail } = &failure else {
        panic!("expected a timeout, got {failure:?}");
    };
    assert_eq!(*seconds, budget.as_secs());
    assert!(
        tail.contains("Sign in at"),
        "output printed before the kill must survive, got {tail:?}"
    );
}
