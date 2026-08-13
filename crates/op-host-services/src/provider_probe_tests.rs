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
fn copilot_auth_status_parser_picks_id3() {
    assert!(
        parse_copilot_auth_status(r#"{"jsonrpc":"2.0","id":2,"result":{"models":[]}}"#).is_none()
    );
    let auth = parse_copilot_auth_status(
        r#"{"jsonrpc":"2.0","id":3,"result":{"login":"octocat","authType":"oauth"}}"#,
    )
    .expect("id:3 parses");
    assert_eq!(auth.login.as_deref(), Some("octocat"));
    assert_eq!(auth.auth_type.as_deref(), Some("oauth"));
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

/// Write an executable `/bin/sh` stand-in for a CLI, so the version gate
/// can be driven through a real subprocess without depending on which
/// coding CLIs happen to be installed on the machine running the tests.
#[cfg(unix)]
fn fake_cli(name: &str, body: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("op-cli-version-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let exe = dir.join(name);
    std::fs::write(&exe, format!("#!/bin/sh\n{body}")).expect("write fake cli");
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    (dir, exe)
}

/// [`cli_version`] with a bounded retry for the /tmp write→exec race
/// that surfaces as ETXTBSY ("Text file busy") on CI runners' overlay
/// filesystems. A fake CLI written and executed back-to-back can still be
/// mid-copy-up when the exec lands, so a single immediate retry keeps the
/// test's intent (a real subprocess round-trip) without papering over
/// genuine spawn failures.
#[cfg(unix)]
fn cli_version_retry(
    exe: &std::path::Path,
    timeout: std::time::Duration,
) -> Result<String, CliVersionFailure> {
    for attempt in 0..3 {
        match cli_version(exe, timeout) {
            Err(CliVersionFailure::Spawn(message))
                if attempt < 2 && message.contains("Text file busy") =>
            {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            result => return result,
        }
    }
    unreachable!("the loop returns on its final attempt")
}

#[cfg(unix)]
#[test]
fn cli_version_reports_stderr_from_a_nonzero_exit() {
    // A stand-in for the broken-shebang case: prints to stderr, exits
    // non-zero. The captured tail must survive into the failure.
    let (dir, exe) = fake_cli(
        "fake-codex",
        "printf 'env: node: No such file or directory\\n' >&2\nexit 127\n",
    );

    let failure = cli_version_retry(&exe, std::time::Duration::from_secs(5))
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

    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(unix)]
#[test]
fn cli_version_accepts_a_healthy_cli_and_bounds_a_hung_one() {
    let (healthy_dir, healthy) = fake_cli("healthy-cli", "printf 'codex-cli 0.9.1\\n'\n");
    assert_eq!(
        cli_version_retry(&healthy, std::time::Duration::from_secs(5)),
        Ok("codex-cli 0.9.1".to_string())
    );
    let _ = std::fs::remove_dir_all(&healthy_dir);

    // Mid first-run OAuth: prints a prompt, then hangs past the budget. A
    // loaded runner can take several seconds to schedule the freshly spawned
    // shell, so retry with a larger budget instead of racing its first printf.
    let (hung_dir, hung) = fake_cli(
        "hung-cli",
        "printf 'Sign in at https://x\\n'\nexec sleep 30\n",
    );
    let budget_cap = std::time::Duration::from_secs(16);
    let mut budget = std::time::Duration::from_secs(4);
    let failure = loop {
        let started = std::time::Instant::now();
        let failure = cli_version(&hung, budget)
            .expect_err("a CLI still running at the deadline is not a usable version");
        assert!(
            started.elapsed() < budget * 4,
            "the deadline, not the hung CLI, must bound the probe"
        );
        let captured_prompt = matches!(
            &failure,
            CliVersionFailure::TimedOut { tail, .. } if tail.contains("Sign in at")
        );
        if captured_prompt || budget >= budget_cap {
            break failure;
        }
        budget = (budget * 2).min(budget_cap);
    };
    let CliVersionFailure::TimedOut { seconds, tail } = &failure else {
        panic!("expected a timeout, got {failure:?}");
    };
    assert_eq!(*seconds, budget.as_secs());
    assert!(
        tail.contains("Sign in at"),
        "output printed before the kill must survive, got {tail:?}"
    );
    let _ = std::fs::remove_dir_all(&hung_dir);
}
