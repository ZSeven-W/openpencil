use super::*;
use op_ai::chat_provider::{ChatRequest, EffortLevel, ThinkingMode};

fn fixture(protocol: &str) -> Vec<u8> {
    format!(r#"{{"object":"list","data":[{{"id":"model-one","display_name":"Model One","object":"model","created":0,"owned_by":"llm-gateway","protocol":"{protocol}","capabilities":["text","streaming","tools"]}}]}}"#).into_bytes()
}

#[test]
fn gateway_protocols_are_explicit_and_session_routes_never_reuse_ids() {
    for (wire, expected) in [
        ("openai_chat_completions", HostedProtocol::OpenAiCompatible),
        ("anthropic_messages", HostedProtocol::AnthropicMessages),
        ("openai_responses", HostedProtocol::OpenAiResponses),
    ] {
        let first = catalog::parse(&fixture(wire), "session-one").unwrap();
        let second = catalog::parse(&fixture(wire), "session-two").unwrap();
        assert_eq!(first[0].protocol(), expected);
        assert_ne!(first[0].route_id(), second[0].route_id());
        assert_eq!(first[0].model_id(), "model-one");
        assert!(!first[0].capabilities().tools);
    }
    assert_eq!(
        catalog::parse(&fixture("responses-as-chat"), "one").unwrap_err(),
        HostedRuntimeError::UnsupportedProtocol
    );
}

#[test]
fn catalog_rejects_duplicates_unknown_fields_and_oversize() {
    let valid = String::from_utf8(fixture("openai_chat_completions")).unwrap();
    assert!(catalog::parse(
        valid
            .replace(
                "\"object\":\"list\"",
                "\"object\":\"list\",\"object\":\"list\""
            )
            .as_bytes(),
        "one"
    )
    .is_err());
    assert!(catalog::parse(
        valid
            .replace(
                "\"id\":\"model-one\"",
                "\"id\":\"model-one\",\"id\":\"model-two\""
            )
            .as_bytes(),
        "one"
    )
    .is_err());
    assert!(catalog::parse(
        valid.replace("\"owned_by\"", "\"api_key\"").as_bytes(),
        "one"
    )
    .is_err());
    assert_eq!(
        catalog::parse(&vec![b' '; catalog::MAX_CATALOG_BYTES + 1], "one").unwrap_err(),
        HostedRuntimeError::CatalogTooLarge
    );
    let mut value: serde_json::Value = serde_json::from_str(&valid).unwrap();
    let model = value["data"][0].clone();
    value["data"].as_array_mut().unwrap().push(model);
    assert!(catalog::parse(value.to_string().as_bytes(), "one").is_err());
}

fn request() -> ChatRequest {
    ChatRequest {
        user_message: "Hello".into(),
        system_prompt: "System".into(),
        history: Vec::new(),
        attachments: Vec::new(),
        model: Some("attacker-selected-model".into()),
        max_output_tokens: 128,
        thinking: ThinkingMode::Disabled,
        effort: EffortLevel::Low,
    }
}

#[test]
fn paid_request_uses_authoritative_model_and_exact_protocol() {
    for (wire, path) in [
        ("openai_chat_completions", "/chat/completions"),
        ("anthropic_messages", "/messages"),
    ] {
        let routes = catalog::parse(&fixture(wire), "one").unwrap();
        let (actual, body) = provider::request_body(&routes[0], request()).unwrap();
        assert_eq!(actual, path);
        assert_eq!(body["model"], "model-one");
        assert!(!body.to_string().contains("attacker-selected"));
        assert_eq!(body.get("system").is_some(), wire == "anthropic_messages");
    }
    let routes = catalog::parse(&fixture("openai_responses"), "one").unwrap();
    assert_eq!(
        provider::request_body(&routes[0], request()).unwrap_err(),
        HostedRuntimeError::UnsupportedProtocol
    );
}

#[test]
fn closed_errors_never_render_upstream_payloads() {
    for error in [
        HostedRuntimeError::AccessRejected,
        HostedRuntimeError::Network,
        HostedRuntimeError::GatewayRejected,
        HostedRuntimeError::SessionChanged,
    ] {
        assert!(!error.to_string().contains("https://"));
        assert!(!error.to_string().contains("opllm_"));
    }
}

#[test]
fn hosted_wire_has_one_authority_and_preserves_conversation() {
    let good = r#"{"hosted_route_id":"hosted-123","user":"next","history":[{"role":"user","content":"first"},{"role":"assistant","content":"reply"}]}"#;
    let request = crate::ai_proxy::parse_ai_stream_body(good).unwrap();
    assert!(crate::web_chat_standard::parse_standard_turn_body(good).is_none());
    assert_eq!(request.history.len(), 2);
    assert_eq!(request.hosted_route_id.as_deref(), Some("hosted-123"));
    for extra in [
        r#""model":"other""#,
        r#""provider":"codex""#,
        r#""credential":null"#,
        r#""builtinProviderId":"x""#,
        r#""endpoint":"https://evil.example""#,
        r#""hosted_route_id":"second""#,
    ] {
        let body = format!("{},{}{}", good.trim_end_matches('}'), extra, "}");
        assert!(
            crate::ai_proxy::parse_ai_stream_body(&body).is_none(),
            "{extra}"
        );
    }
}

#[test]
fn resetting_identity_clears_catalog_and_changes_route_namespace() {
    let session = Session {
        device_id: "device".into(),
        auth_epoch: 1,
    };
    let mut state = State::default();
    reset(&mut state, Some(session));
    state.routes = catalog::parse(&fixture("openai_chat_completions"), &state.namespace).unwrap();
    state.subject = Some("subject-one".into());
    let old_epoch = state.epoch;
    let old_namespace = state.namespace.clone();
    reset(
        &mut state,
        Some(Session {
            device_id: "device".into(),
            auth_epoch: 2,
        }),
    );
    assert_ne!(state.epoch, old_epoch);
    assert_ne!(state.namespace, old_namespace);
    assert!(state.subject.is_none());
    assert!(state.routes.is_empty());
}

struct PendingAccess {
    begins: std::sync::atomic::AtomicUsize,
    polls: std::sync::atomic::AtomicUsize,
    cancels: std::sync::atomic::AtomicUsize,
}

impl PendingAccess {
    fn new() -> Self {
        Self {
            begins: 0.into(),
            polls: 0.into(),
            cancels: 0.into(),
        }
    }
}

impl op_auth_bridge::HostedAccessProvider for PendingAccess {
    fn available(&self) -> bool {
        true
    }
    fn session_epoch(&self) -> u64 {
        1
    }
    fn begin(&self) -> Result<HostedAccessRequestId, op_auth_bridge::HostedAccessError> {
        self.begins
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(HostedAccessRequestId::new(1).unwrap())
    }
    fn poll(
        &self,
        _: HostedAccessRequestId,
    ) -> Result<HostedAccessPoll, op_auth_bridge::HostedAccessError> {
        self.polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(HostedAccessPoll::Pending)
    }
    fn cancel(&self, _: HostedAccessRequestId) {
        self.cancels
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

#[tokio::test]
async fn account_switch_cancels_waiting_private_access_handle() {
    let provider = PendingAccess::new();
    let result = access_with(&provider, || {
        if provider.polls.load(std::sync::atomic::Ordering::SeqCst) != 0 {
            Err(HostedRuntimeError::SessionChanged)
        } else {
            Ok(())
        }
    })
    .await;
    assert_eq!(result.unwrap_err(), HostedRuntimeError::SessionChanged);
    assert_eq!(provider.begins.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        provider.cancels.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[tokio::test]
async fn cancellation_drops_and_cancels_private_access_handle() {
    let provider = PendingAccess::new();
    let result =
        tokio::time::timeout(Duration::from_millis(10), access_with(&provider, || Ok(()))).await;
    assert!(result.is_err());
    assert_eq!(
        provider.cancels.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[test]
fn missing_identity_fails_before_private_access_or_network() {
    let provider = PendingAccess::new();
    let result = crate::chat_runtime::block_on_anywhere(access_with(&provider, || {
        Err(HostedRuntimeError::NotSignedIn)
    }));
    assert_eq!(result.unwrap_err(), HostedRuntimeError::NotSignedIn);
    assert_eq!(provider.begins.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn late_catalog_after_logout_and_changed_subject_are_rejected() {
    let session = Session {
        device_id: "device".into(),
        auth_epoch: 1,
    };
    let mut state = State::default();
    reset(&mut state, Some(session.clone()));
    let epoch = state.epoch;
    let routes = catalog::parse(&fixture("openai_chat_completions"), &state.namespace).unwrap();
    reset(&mut state, None);
    assert!(!accept_catalog(
        &mut state,
        &session,
        epoch,
        "subject-one".into(),
        routes.clone()
    ));
    assert!(state.routes.is_empty());
    reset(&mut state, Some(session.clone()));
    let epoch = state.epoch;
    state.subject = Some("subject-one".into());
    assert!(!accept_catalog(
        &mut state,
        &session,
        epoch,
        "subject-two".into(),
        routes
    ));
    assert!(state.routes.is_empty());
    assert!(state.session.is_none());
}

struct ReadyAfterSwitch(std::sync::atomic::AtomicBool);
impl op_auth_bridge::HostedAccessProvider for ReadyAfterSwitch {
    fn available(&self) -> bool {
        true
    }
    fn session_epoch(&self) -> u64 {
        1
    }
    fn begin(&self) -> Result<HostedAccessRequestId, op_auth_bridge::HostedAccessError> {
        Ok(HostedAccessRequestId::new(1).unwrap())
    }
    fn poll(
        &self,
        _: HostedAccessRequestId,
    ) -> Result<HostedAccessPoll, op_auth_bridge::HostedAccessError> {
        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
        let now = now_unix();
        let bytes = serde_json::json!({"version":1,"token":format!("opllm_{}", "A".repeat(43)),
            "token_id":"0123456789abcdef0123456789abcdef", "token_type":"Bearer",
            "issuer":"https://sso.example.test", "subject":"01234567-89ab-cdef-0123-456789abcdef",
            "audience":"llm-gateway", "scopes":["llm:models","llm:invoke"],
            "issued_at_unix":now, "expires_at_unix":now+300,
            "gateway_base_url":"https://gateway.example.test/v1"})
        .to_string()
        .into_bytes();
        Ok(HostedAccessPoll::Ready(HostedAccess::from_provider_json(
            bytes,
        )?))
    }
    fn cancel(&self, _: HostedAccessRequestId) {}
}

#[tokio::test]
async fn ready_access_arriving_after_account_switch_is_not_accepted() {
    let provider = ReadyAfterSwitch(false.into());
    let result = access_with(&provider, || {
        if provider.0.load(std::sync::atomic::Ordering::SeqCst) {
            Err(HostedRuntimeError::SessionChanged)
        } else {
            Ok(())
        }
    })
    .await;
    assert_eq!(result.unwrap_err(), HostedRuntimeError::SessionChanged);
}

#[test]
fn already_cancelled_send_never_starts_private_access_or_network() {
    use op_ai::chat_provider::{ChatDelta, ChatProvider, StopReason};
    let provider = provider::HostedProvider {
        session: Session {
            device_id: "fixture-device".into(),
            auth_epoch: 1,
        },
        epoch: 1,
        subject: "fixture-subject".into(),
        route: catalog::parse(&fixture("openai_chat_completions"), "test")
            .unwrap()
            .remove(0),
    };
    let result: Vec<_> = provider
        .send_cancellable(request(), std::sync::Arc::new(true.into()))
        .collect();
    assert!(matches!(
        &result[..],
        [ChatDelta::Done {
            stop_reason: StopReason::Aborted
        }]
    ));
}

#[test]
fn proxy_discards_buffered_hosted_deltas_after_session_invalidation() {
    struct StaleProvider;
    impl ChatProvider for StaleProvider {
        fn provider_label(&self) -> &str {
            "stale-hosted"
        }
        fn hosted_session_epoch(&self) -> Option<u64> {
            Some(0)
        }
        fn send(
            &self,
            _: ChatRequest,
        ) -> Box<dyn Iterator<Item = op_ai::chat_provider::ChatDelta> + Send> {
            Box::new(
                [op_ai::chat_provider::ChatDelta::TextDelta(
                    "old-account-output".into(),
                )]
                .into_iter(),
            )
        }
    }
    let request =
        crate::ai_proxy::parse_ai_stream_body(r#"{"hosted_route_id":"route","user":"hello"}"#)
            .unwrap();
    let mut output = Vec::new();
    crate::ai_proxy::stream_ai_response(&mut output, request, &StaleProvider, None).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(!output.contains("old-account-output"));
    assert!(output.contains("Hosted session changed"));
}
