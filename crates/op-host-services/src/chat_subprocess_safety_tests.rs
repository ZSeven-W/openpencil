//! Tests for the CLI containment policy in `chat_subprocess_safety.rs`.
//!
//! A sibling file rather than an inline `mod tests`: the policy module
//! reached the 800-line cap, and the convention here is to move the
//! tests out first (pure code motion — no case was changed).

use super::*;

#[test]
fn grok_prompt_file_is_private_and_removed_with_workspace() {
    let source_dir = std::env::temp_dir().join(format!(
        "openpencil-cli-source-{}-{}",
        std::process::id(),
        TURN_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&source_dir).unwrap();
    let source = source_dir.join("reference.png");
    fs::write(&source, b"image").unwrap();
    let original = format!("inspect [attached image: {}]", source.display());

    let turn = IsolatedTurn::prepare(Some(CliName::GrokBuild), &original, &[source])
        .unwrap()
        .unwrap();
    let cwd = turn.cwd().to_path_buf();
    let prompt_path = turn.prompt_file().unwrap();
    let config_dir = turn.claude_config_dir().unwrap().to_path_buf();
    let settings_path = config_dir.join("settings.json");
    assert!(prompt_path.starts_with(&cwd));
    assert!(config_dir.starts_with(&cwd));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&settings_path).unwrap()).unwrap(),
        serde_json::json!({"permissions": {"defaultMode": "dontAsk"}})
    );
    let mut env = vec![(
        "CLAUDE_CONFIG_DIR".to_string(),
        "/host/claude-config".to_string(),
    )];
    append_isolated_env(&mut env, Some(&turn));
    assert_eq!(
        env,
        vec![(
            "CLAUDE_CONFIG_DIR".to_string(),
            config_dir.to_string_lossy().into_owned(),
        )]
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&config_dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(&settings_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    assert!(turn.prompt().contains("OPENPENCIL AUTOMATION SAFETY"));
    assert!(!turn.prompt().contains(&source_dir.to_string_lossy()[..]));
    assert!(turn.prompt().contains("attachment-0-reference.png"));
    drop(turn);
    assert!(!cwd.exists());
    assert!(!config_dir.exists());
    let _ = fs::remove_dir_all(source_dir);
}

#[test]
fn grok_generation_prompt_is_unguarded_and_still_private() {
    let turn = IsolatedTurn::prepare_generation(
        Some(CliName::GrokBuild),
        "return only I(...) JavaScript",
        &[],
    )
    .unwrap()
    .unwrap();
    assert_eq!(turn.prompt(), "return only I(...) JavaScript");
    assert_eq!(
        fs::read_to_string(turn.prompt_file().unwrap()).unwrap(),
        turn.prompt()
    );
    assert!(turn.claude_config_dir().is_some());
}

#[test]
fn non_agent_cli_does_not_get_an_isolated_turn() {
    assert!(IsolatedTurn::prepare(Some(CliName::Codex), "hi", &[])
        .unwrap()
        .is_none());
}

#[test]
fn stderr_errors_explain_auth_and_permission_failures() {
    assert!(
        friendly_stderr_error(Some(CliName::GrokBuild), "login required")
            .unwrap()
            .contains("grok login")
    );
    assert!(
        friendly_stderr_error(Some(CliName::Antigravity), "permission required")
            .unwrap()
            .contains("interactive permission")
    );
    assert!(friendly_stdout_error(
        Some(CliName::Antigravity),
        "Authentication required. Please visit the URL to log in:"
    )
    .unwrap()
    .contains("agy"));
    assert!(friendly_stdout_error(Some(CliName::GrokBuild), "Authentication required").is_none());
}

#[test]
fn auth_matching_ignores_authorship_but_keeps_the_authorization_family() {
    for real in [
        "authentication required",
        "you are not authenticated",
        "unauthorized",
        "authorization header missing",
        "oauth token expired",
        "authorised device not found",
    ] {
        assert!(mentions_auth(real), "should classify: {real}");
    }
    // `author…` in ordinary chatter must not read as an auth failure
    // — a false positive costs the reader the real diagnosis.
    for false_friend in [
        "file authored by another process",
        "authoring a new document failed",
        "authorship metadata missing",
        "not an authoritative answer",
    ] {
        assert!(
            !mentions_auth(false_friend),
            "should not classify: {false_friend}"
        );
    }
    // End to end: an authorship-flavoured crash keeps falling
    // through to the tail-carrying fallback.
    assert!(friendly_stderr_error(
        Some(CliName::Antigravity),
        "panic: authored node had no parent"
    )
    .is_none());
}

#[test]
fn guarded_child_env_excludes_host_token_and_unrelated_secrets() {
    let vars = vec![
        ("HOME".to_string(), "/tmp/home".to_string()),
        ("HTTPS_PROXY".to_string(), "http://proxy".to_string()),
        ("OPENPENCIL_MCP_TOKEN".to_string(), "shutdown".to_string()),
        ("DATABASE_URL".to_string(), "secret".to_string()),
        ("XAI_API_KEY".to_string(), "xai".to_string()),
        ("GOOGLE_API_KEY".to_string(), "google".to_string()),
    ];

    let grok = filtered_env(CliName::GrokBuild, vars.clone());
    assert!(grok.iter().any(|(key, _)| key == "HOME"));
    assert!(grok.iter().any(|(key, _)| key == "HTTPS_PROXY"));
    assert!(grok.iter().any(|(key, _)| key == "XAI_API_KEY"));
    assert!(!grok.iter().any(|(key, _)| key == "GOOGLE_API_KEY"));
    assert!(!grok.iter().any(|(key, _)| key == "OPENPENCIL_MCP_TOKEN"));
    assert!(!grok.iter().any(|(key, _)| key == "DATABASE_URL"));

    let antigravity = filtered_env(CliName::Antigravity, vars);
    assert!(antigravity.iter().any(|(key, _)| key == "GOOGLE_API_KEY"));
    assert!(!antigravity.iter().any(|(key, _)| key == "XAI_API_KEY"));
    assert!(!antigravity
        .iter()
        .any(|(key, _)| key == "OPENPENCIL_MCP_TOKEN"));
}

#[test]
fn opencode_probe_env_keeps_config_but_not_unrelated_provider_keys() {
    let vars = vec![
        ("HOME".to_string(), "/tmp/home".to_string()),
        ("PATH".to_string(), "/usr/bin".to_string()),
        ("XDG_CONFIG_HOME".to_string(), "/tmp/config".to_string()),
        ("XDG_RUNTIME_DIR".to_string(), "/tmp/runtime".to_string()),
        (
            "OPENCODE_CONFIG".to_string(),
            "/tmp/opencode.json".to_string(),
        ),
        ("OPENAI_API_KEY".to_string(), "unrelated".to_string()),
        ("ANTHROPIC_API_KEY".to_string(), "unrelated".to_string()),
        ("DATABASE_URL".to_string(), "unrelated".to_string()),
    ];

    let opencode = filtered_env(CliName::OpenCode, vars);
    let keys: Vec<_> = opencode.iter().map(|(key, _)| key.as_str()).collect();
    assert!(keys.contains(&"HOME"));
    assert!(keys.contains(&"PATH"));
    assert!(keys.contains(&"XDG_CONFIG_HOME"));
    assert!(keys.contains(&"XDG_RUNTIME_DIR"));
    assert!(keys.contains(&"OPENCODE_CONFIG"));
    assert!(!keys.contains(&"OPENAI_API_KEY"));
    assert!(!keys.contains(&"ANTHROPIC_API_KEY"));
    assert!(!keys.contains(&"DATABASE_URL"));
}

#[test]
fn antigravity_keeps_linux_keyring_session_but_grok_does_not() {
    for key in ["DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR"] {
        assert!(allowed_env(CliName::Antigravity, key), "{key}");
        assert!(!allowed_env(CliName::GrokBuild, key), "{key}");
    }
}
