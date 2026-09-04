use super::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::time::Duration;

fn read_http_request(stream: &mut TcpStream) -> String {
    let mut buf = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let n = stream.read(&mut chunk).expect("read request");
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        let Some(header_end) = buf.windows(4).position(|w| w == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_len = headers
            .lines()
            .find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        if buf.len() >= header_end + 4 + content_len {
            break;
        }
    }
    String::from_utf8_lossy(&buf).to_string()
}

fn http_request_body(request: &str) -> Value {
    let body_start = request
        .find("\r\n\r\n")
        .map(|index| index + 4)
        .expect("request body separator");
    serde_json::from_str(&request[body_start..]).expect("request body JSON")
}

fn capture_classic_openai_body(model: &str, thinking: ThinkingMode) -> Value {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind request capture server");
    let addr = listener.local_addr().expect("request capture address");
    let (request_tx, request_rx) = std_mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let request = read_http_request(&mut stream);
        request_tx.send(request).expect("capture request");
        let body = concat!(
            "data: {\"choices\":[{\"delta\":{\"content\":\"ok\"}}]}\n\n",
            "data: [DONE]\n\n"
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write SSE response");
    });

    let mut config = builtin_config(BuiltinAgentKind::OpenAiCompat, format!("http://{addr}/v1"));
    config.set_models([model]);
    let mut provider =
        ConfiguredBuiltinProvider::from_builtin_agent(&config).expect("ready capture provider");
    provider.max_retries = 0;
    provider.min_gap = Duration::ZERO;
    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "continue the design".into(),
            thinking,
            ..Default::default()
        })
        .collect();
    assert!(
        deltas
            .iter()
            .any(|delta| matches!(delta, ChatDelta::TextDelta(text) if text == "ok")),
        "capture response should complete: {deltas:?}"
    );
    server.join().expect("request capture server exits");
    http_request_body(&request_rx.recv().expect("captured request"))
}

fn builtin_config(kind: BuiltinAgentKind, base_url: impl Into<String>) -> BuiltinAgentConfig {
    BuiltinAgentConfig {
        id: "builtin-test".into(),
        preset: op_editor_core::BuiltinAgentPresetKey::Custom,
        display_name: "Test provider".into(),
        kind,
        api_key: "sk-test".into(),
        models: vec!["test-model".into()],
        base_url: base_url.into(),
        enabled: true,
    }
}

#[test]
fn explicit_saved_model_constructor_routes_one_member_only() {
    let mut config = builtin_config(BuiltinAgentKind::OpenAiCompat, "https://api.example.com/v1");
    config.set_models(["model-a", "model-b"]);

    let provider = ConfiguredBuiltinProvider::from_builtin_agent_with_model(&config, "model-b")
        .expect("saved model builds");

    assert_eq!(provider.model, "model-b");
    assert!(
        ConfiguredBuiltinProvider::from_builtin_agent_with_model(&config, "runtime-only").is_none()
    );
}

#[test]
fn provider_normalizes_base_urls_and_preserves_full_endpoints() {
    let lm_studio = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        "  http://localhost:1234/v1/  ",
    ))
    .expect("ready LM Studio provider");
    assert_eq!(lm_studio.base_url, "http://localhost:1234/v1");
    assert_eq!(
        lm_studio.endpoint("/chat/completions"),
        "http://localhost:1234/v1/chat/completions"
    );

    let full_endpoint = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        "http://localhost:1234/v1/chat/completions/",
    ))
    .expect("ready full-endpoint provider");
    assert_eq!(
        full_endpoint.endpoint("/chat/completions"),
        "http://localhost:1234/v1/chat/completions"
    );

    let anthropic = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::Anthropic,
        "https://api.anthropic.com/v1/",
    ))
    .expect("ready Anthropic provider");
    assert_eq!(
        anthropic.endpoint("/v1/messages"),
        "https://api.anthropic.com/v1/messages"
    );

    let defaulted = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        "  ",
    ))
    .expect("ready default provider");
    assert_eq!(
        defaulted.base_url,
        BuiltinAgentKind::OpenAiCompat.default_base_url()
    );
}

#[test]
fn invalid_ready_provider_remains_constructible_and_aborts_with_precise_error() {
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        "not a provider url",
    ))
    .expect("ready providers remain routable even when their URL is invalid");

    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "hello".into(),
            ..Default::default()
        })
        .collect();

    assert!(
        matches!(deltas.first(), Some(ChatDelta::Error(message)) if message.starts_with("Invalid provider endpoint:")),
        "expected precise endpoint error first, got {deltas:?}"
    );
    assert!(
        matches!(
            deltas.get(1),
            Some(ChatDelta::Done {
                stop_reason: StopReason::Aborted
            })
        ),
        "invalid endpoints must abort the selected provider turn, got {deltas:?}"
    );
    assert_eq!(deltas.len(), 2, "invalid endpoints must fail immediately");
}

#[test]
fn invalid_provider_with_canvas_tools_aborts_before_starting_the_agent_loop() {
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        "not a provider url",
    ))
    .expect("ready providers remain routable even when their URL is invalid");
    let (executor, _requests) = crate::chat_canvas_tools::chat_tool_channel();
    let provider = provider.with_canvas_tools(Vec::new(), Arc::new(executor));

    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "draw a card".into(),
            ..Default::default()
        })
        .collect();

    assert!(
        matches!(deltas.as_slice(), [ChatDelta::Error(message), ChatDelta::Done { stop_reason: StopReason::Aborted }] if message.starts_with("Invalid provider endpoint:")),
        "tool-capable turns must surface provider construction errors before entering the loop: {deltas:?}"
    );
}

#[test]
fn unsupported_provider_scheme_aborts_before_connecting() {
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        "ftp://example.com/v1",
    ))
    .expect("ready providers remain routable even when their scheme is invalid");

    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "hello".into(),
            ..Default::default()
        })
        .collect();
    assert!(
        matches!(deltas.as_slice(), [ChatDelta::Error(message), ChatDelta::Done { stop_reason: StopReason::Aborted }] if message.starts_with("Invalid provider endpoint:")),
        "unsupported schemes must surface as provider errors, got {deltas:?}"
    );
}

#[test]
fn provider_endpoint_rejects_queries_and_fragments() {
    for endpoint in [
        "https://example.com/v1?api-version=1",
        "https://example.com/v1#chat",
    ] {
        let error = normalize_provider_base_url(endpoint)
            .expect_err("query and fragment endpoints must be rejected")
            .to_string();
        assert!(
            error.starts_with("Invalid provider endpoint:"),
            "unexpected validation error for {endpoint}: {error}"
        );
    }
}

/// Drive one stalled-provider turn and report which BEHAVIOR happened —
/// specifically, how many TCP connections the client actually attempted —
/// rather than how long the whole call took.
///
/// The original version of this helper measured wall-clock `elapsed` and
/// asserted an upper bound (`< 1s`) to infer "no retry happened" (a real
/// retry sleeps a >=1s exponential backoff — see `backoff_delay`). Under a
/// loaded machine (several `cargo test` processes racing for CPU), the whole
/// call — including the CORRECT, no-retry path — can occasionally cross
/// that bound purely from scheduling delay, failing a test whose underlying
/// behavior was fine (confirmed: reproducibly green under
/// `--test-threads=1`, flaky only under full parallel load). `.collect()`
/// only returns once the client has synchronously finished every attempt it
/// will EVER make (a timeout aborts inline; a retry sleeps its backoff
/// inline before opening the next connection), so by the time it returns,
/// whether a second connection was attempted is already decided — counting
/// accepted connections is a discrete, load-independent stand-in for "did a
/// retry fire" that never depends on how fast the CPU got there.
fn collect_stalled_provider_turn(max_retries: u32) -> (Vec<ChatDelta>, Vec<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stalled server");
    listener
        .set_nonblocking(true)
        .expect("nonblocking stalled listener");
    let addr = listener.local_addr().expect("stalled server address");
    let accepted: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let accepted_for_server = Arc::clone(&accepted);
    let (stop_tx, stop_rx) = std_mpsc::channel::<()>();

    let server = std::thread::spawn(move || {
        // Every accepted socket is held here (never responded to) until the
        // thread exits — accept in a LOOP (not once) so a client retry (a
        // second connection attempt) is observable at all; the original
        // single-`accept()` design could not distinguish "no retry" from
        // "no second accept call existed to observe it with".
        let mut held = Vec::new();
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).ok();
                    let request = read_http_request(&mut stream);
                    accepted_for_server.lock().unwrap().push(request);
                    held.push(stream);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if stop_rx.try_recv().is_ok() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(_) => break,
            }
        }
    });

    let mut provider = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        format!("http://{addr}/v1/"),
    ))
    .expect("ready stalled provider");
    provider.http_client = Some(
        reqwest::Client::builder()
            .use_rustls_tls()
            .connect_timeout(Duration::from_millis(30))
            .timeout(Duration::from_millis(80))
            .build()
            .expect("short-timeout test client"),
    );
    provider.max_retries = max_retries;
    provider.min_gap = Duration::ZERO;

    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "hello".into(),
            ..Default::default()
        })
        .collect();

    // Pure TCP/poll-loop settle slop: lets the nonblocking accept loop
    // (2ms poll cadence) catch up to a connection the kernel may have
    // already queued. This is NOT a race against the retry decision — that
    // finished synchronously inside `.collect()` above — so, unlike the old
    // `elapsed < 1s` assertion, widening or narrowing this sleep can never
    // flip which outcome the test observes, only how promptly it observes
    // the already-final one.
    std::thread::sleep(Duration::from_millis(50));
    let _ = stop_tx.send(());
    server.join().expect("stalled server exits");
    let requests = accepted.lock().unwrap().clone();
    (deltas, requests)
}

#[test]
fn stalled_provider_request_times_out_and_aborts_without_retrying() {
    let (deltas, accepted) = collect_stalled_provider_turn(0);

    assert_eq!(
        accepted.len(),
        1,
        "zero retry budget must make exactly one connection attempt: {accepted:?}"
    );
    assert!(accepted[0].starts_with("POST /v1/chat/completions "));
    assert!(
        matches!(deltas.first(), Some(ChatDelta::Error(message)) if message.contains("timed out") && message.contains("/v1/chat/completions")),
        "expected timeout-context provider error, got {deltas:?}"
    );
    assert!(
        matches!(
            deltas.get(1),
            Some(ChatDelta::Done {
                stop_reason: StopReason::Aborted
            })
        ),
        "timeouts must abort the provider turn, got {deltas:?}"
    );
    assert_eq!(deltas.len(), 2);
}

#[test]
fn request_timeout_does_not_consume_available_retry_budget() {
    let (deltas, accepted) = collect_stalled_provider_turn(3);

    assert_eq!(
        accepted.len(),
        1,
        "a client-side timeout must abort immediately without spending ANY of the \
         3-retry budget — a retried (second) connection attempt would show up here: {accepted:?}"
    );
    assert!(
        matches!(deltas.as_slice(), [ChatDelta::Error(message), ChatDelta::Done { stop_reason: StopReason::Aborted }] if message.contains("timed out")),
        "timeout with retry budget must still abort immediately, got {deltas:?}"
    );
}

#[test]
fn provider_does_not_follow_redirects_or_echo_upstream_error_bodies() {
    let redirected = TcpListener::bind("127.0.0.1:0").expect("bind redirect target");
    redirected
        .set_nonblocking(true)
        .expect("nonblocking redirect target");
    let redirected_addr = redirected.local_addr().expect("redirect target address");
    let (followed_tx, followed_rx) = std_mpsc::channel();
    let redirected_server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            match redirected.accept() {
                Ok((mut stream, _)) => {
                    let _ = read_http_request(&mut stream);
                    let body = "data: {\"choices\":[{\"finish_reason\":\"stop\"}]}\n\n";
                    let response = format!(
                        "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                    followed_tx.send(true).unwrap();
                    return;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("redirect target accept failed: {error}"),
            }
        }
        followed_tx.send(false).unwrap();
    });

    let origin = TcpListener::bind("127.0.0.1:0").expect("bind redirect origin");
    let origin_addr = origin.local_addr().expect("redirect origin address");
    let origin_server = std::thread::spawn(move || {
        let (mut stream, _) = origin.accept().expect("accept origin request");
        let _ = read_http_request(&mut stream);
        let body = r#"{"error":"echoed credential sk-test"}"#;
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: http://{redirected_addr}/v1/chat/completions\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let mut provider = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        format!("http://{origin_addr}/v1"),
    ))
    .expect("ready redirecting provider");
    provider.max_retries = 0;
    provider.min_gap = Duration::ZERO;

    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "hello".into(),
            ..Default::default()
        })
        .collect();
    origin_server.join().expect("origin server exits");
    let followed = followed_rx.recv().expect("redirect result");
    redirected_server.join().expect("redirect target exits");

    assert!(!followed, "provider client must not follow redirects");
    let error = deltas
        .iter()
        .find_map(|delta| match delta {
            ChatDelta::Error(message) => Some(message.as_str()),
            _ => None,
        })
        .expect("redirect must surface a status-only error");
    assert!(error.contains("302"), "unexpected error: {error}");
    assert!(!error.contains("sk-test"), "credential leaked: {error}");
    assert!(!error.contains("echoed credential"), "body leaked: {error}");
}

#[test]
fn exhausted_rate_limit_error_names_http_429_for_the_non_retryable_classifier() {
    // Regression lock for a real bug: the prior wording "...(429)..." never
    // contained the literal substring `op_orchestrator::retry::is_non_retryable`
    // matches on ("http 429"), so a genuinely exhausted rate limit from this
    // builtin-http path was misclassified as retryable — burning two more
    // full orchestrator attempts before the failure ladder gave up. This test
    // does not import `is_non_retryable` (op-host-services -> op-orchestrator
    // is the wrong direction for a private fn anyway); it locks the actual
    // production error TEXT against the classifier's documented contract.
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind rate-limited server");
    let addr = listener.local_addr().expect("rate-limited server address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept rate-limited request");
        let _ = read_http_request(&mut stream);
        let body = r#"{"error":"rate limited"}"#;
        let response = format!(
            "HTTP/1.1 429 Too Many Requests\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(response.as_bytes()).unwrap();
    });

    let mut provider = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        format!("http://{addr}/v1/"),
    ))
    .expect("ready rate-limited provider");
    // Zero retry budget so the loop hits the exhausted branch on its first
    // attempt without sleeping through real backoff delays.
    provider.max_retries = 0;
    provider.min_gap = Duration::ZERO;

    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "hello".into(),
            ..Default::default()
        })
        .collect();
    server.join().expect("rate-limited server exits");

    let error = deltas
        .iter()
        .find_map(|delta| match delta {
            ChatDelta::Error(message) => Some(message.as_str()),
            _ => None,
        })
        .expect("exhausted 429 must surface a provider error");
    assert!(
        error.to_lowercase().contains("http 429"),
        "exhausted-429 message must contain the literal substring \"http 429\" \
         (case-insensitive) so is_non_retryable classifies it correctly: {error}"
    );
}

#[test]
fn parse_openai_sse_data_extracts_text_delta() {
    let data = r#"{"choices":[{"delta":{"content":"hello"}}]}"#;
    assert_eq!(
        parse_openai_sse_data(data),
        Some(ChatDelta::TextDelta("hello".into()))
    );
}

#[test]
fn classic_openai_mixed_reasoning_and_content_delta_preserves_content() {
    let data =
        r#"{"choices":[{"delta":{"reasoning_content":"plan","content":"batch_design(...)"}}]}"#;
    assert_eq!(
        parse_openai_sse_data(data),
        Some(ChatDelta::TextDelta("batch_design(...)".into()))
    );
}

#[test]
fn classic_openai_reasoning_only_delta_stays_visible_as_thinking() {
    let data = r#"{"choices":[{"delta":{"reasoning_content":"plan"}}]}"#;
    assert_eq!(
        parse_openai_sse_data(data),
        Some(ChatDelta::Thinking("plan".into()))
    );
}

#[test]
fn classic_kimi_k3_request_uses_only_low_reasoning_effort() {
    let body = capture_classic_openai_body("kimi-k3", ThinkingMode::Disabled);
    assert_eq!(body["reasoning_effort"], "low");
    assert!(
        body.get("thinking").is_none(),
        "K3 rejects `thinking` and cannot receive both controls: {body}"
    );
}

#[test]
fn parse_anthropic_sse_data_extracts_text_delta() {
    let data = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hello"}}"#;
    assert_eq!(
        parse_anthropic_sse_data(data),
        Some(ChatDelta::TextDelta("hello".into()))
    );
}

#[test]
fn openai_sse_error_does_not_reflect_upstream_message() {
    let sentinel = "SENTINEL_sk-browser-secret";
    let data = format!(r#"{{"error":{{"message":"provider echoed {sentinel}"}}}}"#);

    let delta = parse_openai_sse_data(&data).expect("error event parses");

    assert_eq!(
        delta,
        ChatDelta::Error("OpenAI-compatible provider reported a stream error".into())
    );
    assert!(!format!("{delta:?}").contains(sentinel));
}

#[test]
fn anthropic_sse_error_does_not_reflect_upstream_message() {
    let sentinel = "SENTINEL_anthropic-browser-secret";
    let data = format!(
        r#"{{"type":"error","error":{{"type":"invalid_request_error","message":"provider echoed {sentinel}"}}}}"#
    );

    let delta = parse_anthropic_sse_data(&data).expect("error event parses");

    assert_eq!(
        delta,
        ChatDelta::Error("Anthropic provider reported a stream error".into())
    );
    assert!(!format!("{delta:?}").contains(sentinel));
}

#[test]
fn thinking_field_gate_covers_every_reasoning_family() {
    // 这条测试此前断言 `!is_minimax_model("deepseek-v4-pro")`,把"DeepSeek 不下发
    // 关思考字段"当成了契约 —— 而那正是缺陷本身:它的 profile 明写
    // thinking_disabled,官方也支持同形字段,漏发的结果是 loop 每轮泄漏 reasoning、
    // `batch_design` 被截断。判定收敛到 op-orchestrator 后,这里只守传输层用的是
    // 那张共享表,家族覆盖由该表自己的测试守。
    use op_orchestrator::accepts_thinking_body_field;
    assert!(accepts_thinking_body_field("MiniMax-M3"));
    assert!(accepts_thinking_body_field("abab6.5s-chat"));
    assert!(accepts_thinking_body_field("glm-5.2"));
    assert!(accepts_thinking_body_field("deepseek-v4-pro"));
    // 未知字段会被拒的端点仍然不加。
    assert!(!accepts_thinking_body_field("qwen3-coder-plus"));
    assert!(!accepts_thinking_body_field("ark-code-latest"));
}

#[test]
fn is_retryable_status_flags_provider_rate_limit_and_overload() {
    assert!(is_retryable_status(reqwest::StatusCode::TOO_MANY_REQUESTS));
    assert!(is_retryable_status(
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    ));
    assert!(is_retryable_status(
        reqwest::StatusCode::from_u16(529).expect("status 529")
    ));

    assert!(!is_retryable_status(reqwest::StatusCode::OK));
    assert!(!is_retryable_status(reqwest::StatusCode::BAD_REQUEST));
    assert!(!is_retryable_status(reqwest::StatusCode::UNAUTHORIZED));
    assert!(!is_retryable_status(reqwest::StatusCode::NOT_FOUND));
    assert!(!is_retryable_status(
        reqwest::StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS
    ));
}

#[test]
fn parse_retry_after_accepts_integer_seconds_and_caps_large_values() {
    let mut headers = reqwest::header::HeaderMap::new();
    assert_eq!(parse_retry_after(&headers), None);

    headers.insert(
        reqwest::header::RETRY_AFTER,
        reqwest::header::HeaderValue::from_static("3"),
    );
    assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(3)));

    headers.insert(
        reqwest::header::RETRY_AFTER,
        reqwest::header::HeaderValue::from_static("0"),
    );
    assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(0)));

    headers.insert(
        reqwest::header::RETRY_AFTER,
        reqwest::header::HeaderValue::from_static("abc"),
    );
    assert_eq!(parse_retry_after(&headers), None);

    headers.insert(
        reqwest::header::RETRY_AFTER,
        reqwest::header::HeaderValue::from_static("3600"),
    );
    assert_eq!(parse_retry_after(&headers), Some(Duration::from_secs(30)));
}

#[test]
fn backoff_delay_exponentially_increases_and_caps() {
    assert_eq!(backoff_delay(0), Duration::from_secs(1));
    assert_eq!(backoff_delay(1), Duration::from_secs(2));
    assert_eq!(backoff_delay(2), Duration::from_secs(4));
    assert_eq!(backoff_delay(3), Duration::from_secs(8));
    assert_eq!(backoff_delay(99), Duration::from_secs(8));
}

#[test]
fn throttle_wait_respects_reserved_last_request_slot() {
    let now = std::time::Instant::now();
    let min_gap = Duration::from_millis(350);

    assert_eq!(throttle_wait(None, now, min_gap), Duration::ZERO);
    assert_eq!(throttle_wait(Some(now), now, min_gap), min_gap);
    assert_eq!(
        throttle_wait(Some(now - Duration::from_millis(400)), now, min_gap),
        Duration::ZERO
    );
}

#[test]
fn openai_sse_error_finishes_aborted() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local SSE server");
    let addr = listener.local_addr().expect("local addr");
    let (req_tx, req_rx) = std_mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set read timeout");
        let request = read_http_request(&mut stream);
        req_tx.send(request).expect("send request capture");

        let body = "data: {\"error\":{\"message\":\"bad key\"}}\n\n";
        let response = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write SSE response");
    });
    let provider = ConfiguredBuiltinProvider::from_builtin_agent(&BuiltinAgentConfig {
        id: "builtin-1".into(),
        preset: op_editor_core::BuiltinAgentPresetKey::Custom,
        display_name: "Mock OpenAI".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key: "sk-test".into(),
        models: vec!["gpt-test".into()],
        base_url: format!("http://{addr}"),
        enabled: true,
    })
    .expect("ready provider");

    let deltas: Vec<ChatDelta> = provider
        .send(ChatRequest {
            user_message: "hello".into(),
            max_output_tokens: 64,
            ..Default::default()
        })
        .collect();
    server.join().expect("server thread exits");
    let request = req_rx
        .recv()
        .expect("captured request")
        .to_ascii_lowercase();

    assert!(request.starts_with("post /chat/completions "));
    assert!(request.contains("authorization: bearer sk-test"));
    assert!(
        deltas
            .iter()
        .any(|d| matches!(d, ChatDelta::Error(message) if message == "OpenAI-compatible provider reported a stream error")),
        "expected upstream error delta, got {deltas:?}"
    );
    assert!(
        matches!(
            deltas.last(),
            Some(ChatDelta::Done {
                stop_reason: StopReason::Aborted
            })
        ),
        "SSE errors must terminate as aborted, got {deltas:?}"
    );
}
