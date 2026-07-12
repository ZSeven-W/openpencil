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

fn builtin_config(kind: BuiltinAgentKind, base_url: impl Into<String>) -> BuiltinAgentConfig {
    BuiltinAgentConfig {
        id: "builtin-test".into(),
        preset: op_editor_core::BuiltinAgentPresetKey::Custom,
        display_name: "Test provider".into(),
        kind,
        api_key: "sk-test".into(),
        model: "test-model".into(),
        base_url: base_url.into(),
        enabled: true,
    }
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
            .expect_err("query and fragment endpoints must be rejected");
        assert!(
            error.starts_with("Invalid provider endpoint:"),
            "unexpected validation error for {endpoint}: {error}"
        );
    }
}

fn collect_stalled_provider_turn(max_retries: u32) -> (Vec<ChatDelta>, Duration, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind stalled server");
    let addr = listener.local_addr().expect("stalled server address");
    let (accepted_tx, accepted_rx) = std_mpsc::channel();
    let (release_tx, release_rx) = std_mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept stalled request");
        let request = read_http_request(&mut stream);
        accepted_tx.send(request).expect("record stalled request");
        let _ = release_rx.recv_timeout(Duration::from_secs(10));
        // Intentionally send no response; keep the accepted socket open until
        // the client-side request deadline has fired.
    });

    let mut provider = ConfiguredBuiltinProvider::from_builtin_agent(&builtin_config(
        BuiltinAgentKind::OpenAiCompat,
        format!("http://{addr}/v1/"),
    ))
    .expect("ready stalled provider");
    provider.http_client = Some(
        reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(50))
            .timeout(Duration::from_millis(150))
            .build()
            .expect("short-timeout test client"),
    );
    provider.max_retries = max_retries;
    provider.min_gap = Duration::ZERO;

    let started = std::time::Instant::now();
    let deltas: Vec<_> = provider
        .send(ChatRequest {
            user_message: "hello".into(),
            ..Default::default()
        })
        .collect();
    let elapsed = started.elapsed();
    let _ = release_tx.send(());
    server.join().expect("stalled server exits");
    let request = accepted_rx.recv().expect("stalled request captured");
    (deltas, elapsed, request)
}

#[test]
fn stalled_provider_request_times_out_and_aborts_without_retrying() {
    let (deltas, elapsed, request) = collect_stalled_provider_turn(0);

    assert!(request.starts_with("POST /v1/chat/completions "));
    assert!(
        elapsed < Duration::from_secs(1),
        "zero-retry short timeout took {elapsed:?}"
    );
    assert!(
        matches!(deltas.first(), Some(ChatDelta::Error(message)) if message.contains("timed out") && message.contains("/v1/chat/completions")),
        "expected timeout-context provider error, got {deltas:?} after {elapsed:?}"
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
    let (deltas, elapsed, _) = collect_stalled_provider_turn(3);

    assert!(
        elapsed < Duration::from_secs(1),
        "timeouts must abort before retry backoff, took {elapsed:?}"
    );
    assert!(
        matches!(deltas.as_slice(), [ChatDelta::Error(message), ChatDelta::Done { stop_reason: StopReason::Aborted }] if message.contains("timed out")),
        "timeout with retry budget must still abort immediately, got {deltas:?}"
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
fn parse_anthropic_sse_data_extracts_text_delta() {
    let data = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"hello"}}"#;
    assert_eq!(
        parse_anthropic_sse_data(data),
        Some(ChatDelta::TextDelta("hello".into()))
    );
}

#[test]
fn is_minimax_model_gates_thinking_field() {
    // 仅 MiniMax 模型加 `thinking:{type:disabled}`;别的 provider 不加。
    assert!(is_minimax_model("MiniMax-M3"));
    assert!(is_minimax_model("MiniMax-M2.7"));
    assert!(is_minimax_model("abab6.5s-chat"));
    assert!(!is_minimax_model("deepseek-v4-pro"));
    assert!(!is_minimax_model("qwen3-coder-plus"));
    assert!(!is_minimax_model("ark-code-latest"));
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
        model: "gpt-test".into(),
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
            .any(|d| matches!(d, ChatDelta::Error(message) if message == "bad key")),
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
