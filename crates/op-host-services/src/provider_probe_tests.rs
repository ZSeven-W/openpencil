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
        codex_connection_info_from_auth(Some(&auth)),
        "Connected via pro (dev@example.com)"
    );
}

#[test]
fn codex_auth_info_falls_back_to_auth_mode_then_generic() {
    // Email present but no plan claim → auth_mode label.
    let token = jwt_with_payload(r#"{"email":"dev@example.com"}"#);
    let auth = format!(r#"{{"auth_mode":"chatgpt","tokens":{{"id_token":"{token}"}}}}"#);
    assert_eq!(
        codex_connection_info_from_auth(Some(&auth)),
        "Connected via chatgpt (dev@example.com)"
    );
    // No tokens → bare auth_mode.
    assert_eq!(
        codex_connection_info_from_auth(Some(r#"{"auth_mode":"api-key"}"#)),
        "Connected via api-key"
    );
    // No auth.json at all.
    assert_eq!(
        codex_connection_info_from_auth(None),
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
        opencode_provider_summary(&models),
        "Connected (anthropic, openai, google +1)"
    );
    assert_eq!(
        opencode_provider_summary(&models[..3]),
        "Connected (anthropic, openai)"
    );
    assert_eq!(
        opencode_provider_summary(&[]),
        "Connected via OpenCode server"
    );
}

#[test]
fn connected_probe_outcome_rejects_empty_model_list() {
    let outcome = connected_probe_outcome(
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
        copilot_connection_info(Some(&full)),
        "Connected as @octocat (oauth)"
    );
    let message_only = CopilotAuth {
        login: None,
        auth_type: None,
        status_message: Some("Signed in via device flow".into()),
    };
    assert_eq!(
        copilot_connection_info(Some(&message_only)),
        "Signed in via device flow"
    );
    assert_eq!(copilot_connection_info(None), "Connected via GitHub");
}

#[test]
fn friendly_error_mappers_match_ts_tables() {
    assert!(friendly_claude_error("process exited with code 1").contains("claude login"));
    assert_eq!(
        friendly_claude_error("process exited with code 7"),
        "Unable to connect. Claude Code process exited unexpectedly."
    );
    assert!(friendly_copilot_error("spawn ENOENT").contains("not found"));
    assert!(friendly_copilot_error("not authenticated yet").contains("copilot login"));
    assert_eq!(
        friendly_copilot_error("Connection timed out"),
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
    assert_eq!(
        install_command(AgentProvider::GeminiCli),
        "npm install -g @anthropic-ai/gemini-cli"
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
    let outcome =
        ProbeOutcome::not_installed(AgentProvider::ClaudeCode, "Claude Code CLI not found");
    assert!(outcome.not_installed);
    assert!(!outcome.connected);
    assert_eq!(
        outcome.install_command.as_deref(),
        Some("npm install -g @anthropic-ai/claude-code")
    );
    assert_eq!(outcome.error.as_deref(), Some("Claude Code CLI not found"));
}
