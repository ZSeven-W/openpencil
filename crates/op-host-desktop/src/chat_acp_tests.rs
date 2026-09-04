use super::*;
use op_acp::ConnectionType;
// Every consumer of these atomics sits behind `#[cfg(unix)]` (the local-socket
// ACP tests), so on Windows the import is dead and `-D warnings` rejects it.
#[cfg(unix)]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
#[cfg(unix)]
use std::sync::Arc;

fn config() -> AcpAgentConfig {
    AcpAgentConfig {
        id: "acp-1".into(),
        display_name: "Test Agent".into(),
        connection_type: ConnectionType::Local,
        command: Some("test-acp-agent".into()),
        args: vec![],
        env: std::collections::BTreeMap::new(),
        url: None,
        enabled: true,
    }
}

#[test]
fn provider_label_names_the_agent() {
    let provider = AcpProvider::new(config(), Some(3100));
    assert_eq!(provider.provider_label(), "ACP: Test Agent");
}

#[test]
fn provider_constructs_as_chat_provider_trait_object() {
    let _: Arc<dyn ChatProvider> = Arc::new(AcpProvider::new(config(), Some(3100)));
}

#[test]
fn send_without_live_mcp_refuses_with_ts_error() {
    let provider = AcpProvider::new(config(), None);
    let deltas: Vec<ChatDelta> = provider.send(ChatRequest::default()).collect();
    assert_eq!(deltas.len(), 2);
    match &deltas[0] {
        ChatDelta::Error(message) => {
            assert!(message.starts_with("MCP server is not running."));
            assert!(message.contains("Settings → MCP"));
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(matches!(
        deltas[1],
        ChatDelta::Done {
            stop_reason: StopReason::Aborted
        }
    ));
}

#[test]
fn session_options_advertise_openpencil_mcp_and_acp_prompt() {
    let options = session_options_for_port(4123);
    assert_eq!(options.mcp_servers.len(), 1);
    assert_eq!(options.mcp_servers[0].name, "openpencil");
    assert_eq!(options.mcp_servers[0].url, "http://127.0.0.1:4123/mcp");
    assert_eq!(
        options.system_prompt_meta.as_deref(),
        Some(ACP_SYSTEM_PROMPT)
    );
}

#[test]
fn remote_session_options_never_expose_the_desktop_loopback_mcp() {
    let options = session_options_for_connection(ConnectionType::Remote, Some(4123))
        .expect("remote sessions are allowed without local MCP");
    assert!(options.mcp_servers.is_empty());
    assert!(options.system_prompt_meta.is_none());
    assert!(session_options_for_connection(ConnectionType::Remote, None).is_some());
    assert!(session_options_for_connection(ConnectionType::Local, None).is_none());
}

#[test]
fn stable_v1_stop_reasons_reach_the_chat_terminal_delta() {
    assert_eq!(
        map_acp_stop_reason(AcpStopReason::EndTurn),
        StopReason::EndTurn
    );
    assert_eq!(
        map_acp_stop_reason(AcpStopReason::MaxTokens),
        StopReason::MaxTokens
    );
    for reason in [
        AcpStopReason::Cancelled,
        AcpStopReason::MaxTurnRequests,
        AcpStopReason::Refusal,
    ] {
        assert_eq!(map_acp_stop_reason(reason), StopReason::Aborted);
    }
}

#[test]
fn model_and_thought_preferences_only_use_advertised_choices() {
    let config_options = serde_json::from_value(serde_json::json!([
        {
            "id": "model", "name": "Model", "category": "model",
            "type": "select", "currentValue": "small",
            "options": [
                { "value": "small", "name": "Small" },
                { "value": "large", "name": "Large" }
            ]
        },
        {
            "id": "reasoning", "name": "Reasoning", "category": "thought_level",
            "type": "select", "currentValue": "low",
            "options": [{
                "group": "effort", "name": "Effort",
                "options": [
                    { "value": "low", "name": "Low" },
                    { "value": "xhigh", "name": "Max" }
                ]
            }]
        }
    ]))
    .expect("official stable-v1 config options");
    let session = AcpSession {
        session_id: "s1".into(),
        config_options,
    };
    assert_eq!(
        requested_config_value(&session, SessionConfigOptionCategory::Model, &["Large"]),
        Some(("model".into(), "large".into()))
    );
    assert_eq!(
        requested_config_value(
            &session,
            SessionConfigOptionCategory::ThoughtLevel,
            &["max", "xhigh"]
        ),
        Some(("reasoning".into(), "xhigh".into()))
    );
    assert!(requested_config_value(
        &session,
        SessionConfigOptionCategory::Model,
        &["not-advertised"]
    )
    .is_none());
}

#[test]
fn acp_system_prompt_matches_ts_text_anchors() {
    assert!(ACP_SYSTEM_PROMPT.starts_with(
        "You are an AI design assistant integrated inside the OpenPencil vector design tool."
    ));
    assert!(ACP_SYSTEM_PROMPT.contains("- DO use the `mcp__openpencil__*` tools"));
    assert!(ACP_SYSTEM_PROMPT.contains("2. **Build skeleton**: Call `design_skeleton`"));
    assert!(ACP_SYSTEM_PROMPT.contains("4. **Refine**: ALWAYS call `design_refine`"));
    assert!(ACP_SYSTEM_PROMPT.contains("Mobile top rhythm"));
    assert!(ACP_SYSTEM_PROMPT.contains("favorite/heart"));
    assert!(ACP_SYSTEM_PROMPT.contains("signature moment"));
    assert!(ACP_SYSTEM_PROMPT.ends_with("Every key has a value; every value has a key."));
}

#[cfg(unix)]
#[test]
fn successful_turn_closes_then_deletes_an_ephemeral_session() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "op-acp-close-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let cleanup_order = dir.join("cleanup-order");
    let agent = dir.join("agent.sh");
    std::fs::write(
        &agent,
        r#"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentCapabilities":{"mcpCapabilities":{"http":true},"sessionCapabilities":{"close":{},"delete":{}}}}}'
      ;;
    *'"method":"session/new"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"complete"}}'
      ;;
    *'"method":"session/prompt"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"stopReason":"end_turn"}}'
      ;;
    *'"method":"session/close"'*)
      printf '%s\n' close >> "$CLEANUP_ORDER"
      printf '%s\n' '{"jsonrpc":"2.0","id":4,"result":{}}'
      ;;
    *'"method":"session/delete"'*)
      printf '%s\n' delete >> "$CLEANUP_ORDER"
      printf '%s\n' '{"jsonrpc":"2.0","id":5,"result":{}}'
      exit 0
      ;;
  esac
done
"#,
    )
    .unwrap();
    std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut agent_config = config();
    agent_config.command = Some(agent.to_string_lossy().into_owned());
    agent_config.env.insert(
        "CLEANUP_ORDER".into(),
        cleanup_order.to_string_lossy().into_owned(),
    );
    let provider = AcpProvider::new(agent_config, Some(3100));
    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "finish normally".into(),
            ..Default::default()
        })
        .collect();
    assert!(
        matches!(
            deltas.last(),
            Some(ChatDelta::Done {
                stop_reason: StopReason::EndTurn
            })
        ),
        // This failed once on the linux-aarch64 CI leg (2026-08-28) and the
        // bare matches! left no evidence of WHAT the turn ended with. Print
        // the transcript so the next occurrence is attributable.
        "turn must end with Done/EndTurn, got: {deltas:?}"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !cleanup_order.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        std::fs::read_to_string(&cleanup_order).unwrap(),
        "close\ndelete\n",
        "successful turn did not clean up in stable-v1 resource order"
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn cancelling_a_silent_agent_sends_session_cancel_before_cleanup() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "op-acp-cancel-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let prompt_marker = dir.join("prompt-seen");
    let cancel_marker = dir.join("cancel-seen");
    let close_marker = dir.join("close-seen");
    let delete_marker = dir.join("delete-seen");
    let agent = dir.join("agent.sh");
    std::fs::write(
        &agent,
        r#"#!/bin/sh
while IFS= read -r line; do
  case "$line" in
    *'"method":"initialize"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentCapabilities":{"mcpCapabilities":{"http":true},"sessionCapabilities":{"close":{},"delete":{}}}}}'
      ;;
    *'"method":"session/new"'*)
      printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"silent"}}'
      ;;
    *'"method":"session/prompt"'*)
      : > "$PROMPT_MARKER"
      ;;
    *'"method":"session/cancel"'*)
      : > "$CANCEL_MARKER"
      printf '%s\n' '{"jsonrpc":"2.0","id":3,"result":{"stopReason":"cancelled"}}'
      ;;
    *'"method":"session/close"'*)
      : > "$CLOSE_MARKER"
      printf '%s\n' '{"jsonrpc":"2.0","id":4,"result":{}}'
      ;;
    *'"method":"session/delete"'*)
      : > "$DELETE_MARKER"
      printf '%s\n' '{"jsonrpc":"2.0","id":5,"result":{}}'
      exit 0
      ;;
  esac
done
"#,
    )
    .unwrap();
    std::fs::set_permissions(&agent, std::fs::Permissions::from_mode(0o755)).unwrap();

    let mut agent_config = config();
    agent_config.command = Some(agent.to_string_lossy().into_owned());
    agent_config.env.insert(
        "PROMPT_MARKER".into(),
        prompt_marker.to_string_lossy().into_owned(),
    );
    agent_config.env.insert(
        "CANCEL_MARKER".into(),
        cancel_marker.to_string_lossy().into_owned(),
    );
    agent_config.env.insert(
        "CLOSE_MARKER".into(),
        close_marker.to_string_lossy().into_owned(),
    );
    agent_config.env.insert(
        "DELETE_MARKER".into(),
        delete_marker.to_string_lossy().into_owned(),
    );
    let provider = AcpProvider::new(agent_config, Some(3100));
    assert!(provider.supports_cancellable_send());
    let cancel = Arc::new(AtomicBool::new(false));
    let mut iter = provider.send_cancellable(
        ChatRequest {
            user_message: "stay silent".into(),
            ..Default::default()
        },
        Arc::clone(&cancel),
    );

    // The first GUI-aware PATH resolution may run the bounded (8 s)
    // login-shell probe before spawning the absolute test agent.
    let deadline = Instant::now() + Duration::from_secs(12);
    while !prompt_marker.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    if !prompt_marker.is_file() {
        panic!(
            "agent never received session/prompt; first provider delta: {:?}",
            iter.next()
        );
    }
    cancel.store(true, Ordering::Relaxed);
    assert!(iter.next().is_none());

    let deadline = Instant::now() + Duration::from_secs(5);
    while !cancel_marker.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        cancel_marker.is_file(),
        "agent never received session/cancel"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !close_marker.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        close_marker.is_file(),
        "agent never received capability-gated session/close after cancellation"
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !delete_marker.is_file() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        delete_marker.is_file(),
        "agent never received capability-gated session/delete after cancellation"
    );
    std::thread::sleep(Duration::from_millis(100));
    std::fs::remove_dir_all(dir).unwrap();
}
