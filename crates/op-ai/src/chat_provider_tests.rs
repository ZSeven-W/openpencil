use super::*;

/// An executor that overrides NOTHING but the mandatory `execute` — the
/// shape of every host that hasn't wired up the unresolved-blocker scan
/// (web, any future non-desktop host). Anchors the zero-impact
/// requirement: an unwired host must behave EXACTLY as it did before
/// `check_blockers` existed, purely by inheriting the trait default.
struct BareExecutor;
impl ChatToolExecutor for BareExecutor {
    fn execute(&self, _name: &str, _args_json: &str) -> ChatToolResult {
        ChatToolResult {
            content: "{}".into(),
            is_error: false,
        }
    }
}

#[test]
fn unwired_executor_check_blockers_default_is_empty_and_never_blocks() {
    let report = BareExecutor.check_blockers();
    assert_eq!(report, BlockerReport::default());
    assert!(
        !report.has_blockers(),
        "an executor that never overrides check_blockers must report no blockers, \
         so the loop's completion gate takes the exact same path it did before \
         this feature existed"
    );
}

#[test]
fn cli_name_backend_table_matches_architecture_memo() {
    // project_agent_runtime memory:
    //  Subprocess IPC = Claude Code / Copilot / Codex / Antigravity /
    //                   Grok Build / DeepSeek Harness
    //  HTTP server   = OpenCode
    assert!(matches!(
        CliName::ClaudeCode.backend(),
        ChatProviderKind::Subprocess(_)
    ));
    assert!(matches!(
        CliName::Copilot.backend(),
        ChatProviderKind::Subprocess(_)
    ));
    assert!(matches!(
        CliName::Antigravity.backend(),
        ChatProviderKind::Subprocess(_)
    ));
    assert!(matches!(
        CliName::GrokBuild.backend(),
        ChatProviderKind::Subprocess(_)
    ));
    // DeepSeek Harness has no ACP support and no HTTP server mode:
    // one subprocess, one prompt in, one answer out.
    assert!(matches!(
        CliName::Dsh.backend(),
        ChatProviderKind::Subprocess(_)
    ));
    assert!(matches!(
        CliName::Kimi.backend(),
        ChatProviderKind::Subprocess(CliName::Kimi)
    ));
    assert!(matches!(
        CliName::Codex.backend(),
        ChatProviderKind::Subprocess(_)
    ));
    assert!(matches!(
        CliName::OpenCode.backend(),
        ChatProviderKind::HttpServer(_)
    ));
}

#[test]
fn cli_default_binary_uses_expected_names() {
    assert_eq!(CliName::ClaudeCode.default_binary(), "claude");
    assert_eq!(CliName::Codex.default_binary(), "codex");
    assert_eq!(CliName::OpenCode.default_binary(), "opencode");
    assert_eq!(CliName::Antigravity.default_binary(), "agy");
    assert_eq!(CliName::GrokBuild.default_binary(), "grok");
    assert_eq!(CliName::Dsh.default_binary(), "dsh");
    assert_eq!(CliName::Kimi.default_binary(), "kimi");
}

#[test]
fn provider_config_new_seeds_binary_for_cli_kinds() {
    let cfg = ChatProviderConfig::new(ChatProviderKind::Subprocess(CliName::ClaudeCode));
    assert_eq!(cfg.binary, "claude");
    let cfg2 = ChatProviderConfig::new(ChatProviderKind::HttpServer(CliName::OpenCode));
    assert_eq!(cfg2.binary, "opencode");
    // BuiltIn / Acp leave binary empty — built-in needs no
    // spawn target; Acp's binary is user-supplied per-instance.
    let cfg3 = ChatProviderConfig::new(ChatProviderKind::BuiltIn);
    assert!(cfg3.binary.is_empty());
    let cfg4 = ChatProviderConfig::new(ChatProviderKind::Acp);
    assert!(cfg4.binary.is_empty());
}

#[test]
fn echo_provider_replays_script() {
    let p = EchoProvider {
        script: vec![
            ChatDelta::TextDelta("Hello".into()),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ],
    };
    let req = ChatRequest {
        system_prompt: String::new(),
        user_message: "hi".into(),
        max_output_tokens: 1024,
        ..Default::default()
    };
    let mut iter = p.send(req);
    match iter.next() {
        Some(ChatDelta::TextDelta(s)) => assert_eq!(s, "Hello"),
        _ => panic!(),
    }
    match iter.next() {
        Some(ChatDelta::Done { .. }) => {}
        _ => panic!(),
    }
}

#[test]
fn cli_label_is_human_readable() {
    assert_eq!(CliName::ClaudeCode.label(), "Claude Code");
    assert_eq!(CliName::OpenCode.label(), "OpenCode");
    assert_eq!(CliName::Antigravity.label(), "Antigravity");
    assert_eq!(CliName::GrokBuild.label(), "Grok Build");
    assert_eq!(CliName::Dsh.label(), "DeepSeek Harness");
    assert_eq!(CliName::Kimi.label(), "Kimi Code");
}

#[test]
fn chat_request_thinking_effort_defaults_and_wire_tokens() {
    // A defaulted request reasons adaptively at low effort —
    // matching TS `DEFAULT_THINKING_MODE` / `DEFAULT_THINKING_EFFORT`.
    let req = ChatRequest::default();
    assert_eq!(req.thinking, ThinkingMode::Adaptive);
    assert_eq!(req.effort, EffortLevel::Low);
    // Wire tokens match the TS chat-request vocabulary.
    assert_eq!(ThinkingMode::Adaptive.as_str(), "adaptive");
    assert_eq!(ThinkingMode::Disabled.as_str(), "disabled");
    assert_eq!(ThinkingMode::Enabled.as_str(), "enabled");
    assert_eq!(EffortLevel::Low.as_str(), "low");
    assert_eq!(EffortLevel::Medium.as_str(), "medium");
    assert_eq!(EffortLevel::High.as_str(), "high");
    assert_eq!(EffortLevel::Max.as_str(), "max");
}

#[test]
fn effort_budget_tokens_climbs_with_level() {
    // A provider with a numeric thinking knob scales its budget
    // with effort; the table is monotonically increasing.
    assert_eq!(EffortLevel::Low.budget_tokens(), 4096);
    assert_eq!(EffortLevel::Medium.budget_tokens(), 10_000);
    assert_eq!(EffortLevel::High.budget_tokens(), 24_000);
    assert_eq!(EffortLevel::Max.budget_tokens(), 32_000);
    assert!(EffortLevel::Low.budget_tokens() < EffortLevel::Medium.budget_tokens());
    assert!(EffortLevel::High.budget_tokens() < EffortLevel::Max.budget_tokens());
}

#[test]
fn chat_request_attachments_default_empty() {
    let req = ChatRequest::default();
    assert!(req.attachments.is_empty());
}

#[test]
fn chat_request_model_defaults_to_none() {
    // No model selected = the CLI keeps its own default; no flag
    // is ever emitted for the unset case.
    let req = ChatRequest::default();
    assert!(req.model.is_none());
    assert!(req.model_id().is_none());
}

#[test]
fn chat_request_model_id_trims_and_rejects_blank() {
    let mut req = ChatRequest {
        model: Some("  gpt-5.5  ".into()),
        ..Default::default()
    };
    assert_eq!(req.model_id(), Some("gpt-5.5"));
    // Blank / whitespace-only ids are treated as unset so a bare
    // `--model` flag can never reach a CLI.
    req.model = Some("   ".into());
    assert!(req.model_id().is_none());
    req.model = Some(String::new());
    assert!(req.model_id().is_none());
}

#[test]
fn chat_request_history_defaults_empty_and_roles_have_wire_tokens() {
    // A defaulted request is single-shot — no prior turns. The
    // role tokens match the TS chat wire vocabulary.
    let req = ChatRequest::default();
    assert!(req.history.is_empty());
    assert_eq!(ChatHistoryRole::User.as_str(), "user");
    assert_eq!(ChatHistoryRole::Assistant.as_str(), "assistant");
}

#[test]
fn chat_attachment_is_image_checks_media_type() {
    let png = ChatAttachment {
        name: "shot.png".into(),
        media_type: "image/png".into(),
        data: vec![1, 2, 3],
    };
    assert!(png.is_image());
    let txt = ChatAttachment {
        name: "notes.txt".into(),
        media_type: "text/plain".into(),
        data: vec![],
    };
    assert!(!txt.is_image());
}
