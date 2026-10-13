use super::*;
use op_ai::chat_provider::ChatToolResult;
use op_image_enrich::net::ImageRelevanceJudge;
use std::io::Read;
use std::net::TcpListener;
use std::time::Instant;

fn test_provider(base_url: String) -> ConfiguredBuiltinProvider {
    let config = BuiltinAgentConfig {
        id: "joined-cancel-test".into(),
        preset: op_editor_core::BuiltinAgentPresetKey::Custom,
        display_name: "Joined cancellation test".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key: "sk-test".into(),
        models: vec!["test-model".into()],
        base_url,
        enabled: true,
    };
    let mut provider = ConfiguredBuiltinProvider::from_builtin_agent(&config).unwrap();
    provider.http_client = Some(
        reqwest::Client::builder()
            .use_rustls_tls()
            .no_proxy()
            .build()
            .unwrap(),
    );
    provider.min_gap = Duration::ZERO;
    provider.max_retries = 0;
    provider
}

fn silent_server() -> (
    String,
    std::sync::mpsc::Receiver<()>,
    std::thread::JoinHandle<bool>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let (ready_tx, ready_rx) = std::sync::mpsc::channel();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "provider never connected");
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("accept failed: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut head = Vec::new();
        let mut byte = [0_u8];
        while !head.ends_with(b"\r\n\r\n") {
            assert!(head.len() < 64 * 1024);
            stream.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
        }
        let length = String::from_utf8_lossy(&head)
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().unwrap())
            })
            .expect("request content length");
        let mut body = vec![0; length];
        stream.read_exact(&mut body).unwrap();
        ready_tx.send(()).unwrap();
        // No response is sent: the task is waiting inside the HTTP transport.
        match stream.read(&mut byte) {
            Ok(0) => true,
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => true,
            other => panic!("cancelled HTTP connection remained open: {other:?}"),
        }
    });
    (format!("http://{address}/v1"), ready_rx, server)
}

#[test]
fn joined_cancel_and_drop_close_the_real_silent_http_connection() {
    for drop_iterator in [false, true] {
        let (base_url, request_ready, server) = silent_server();
        let provider = test_provider(base_url);
        assert!(provider.supports_joined_cancellable_send());
        let cancel = Arc::new(AtomicBool::new(false));
        let mut iter = provider
            .send_cancellable_joined(
                ChatRequest {
                    system_prompt: "Reply only with JSON.".into(),
                    user_message: "inspect this evidence".into(),
                    ..Default::default()
                },
                Arc::clone(&cancel),
            )
            .expect("builtin provider supports joined cancellation");
        request_ready
            .recv_timeout(Duration::from_secs(5))
            .expect("the real HTTP request reached the server");
        if drop_iterator {
            drop(iter);
        } else {
            cancel.store(true, Ordering::Release);
            assert!(iter.next().is_none());
        }
        assert!(server
            .join()
            .expect("silent server observed transport close"));
    }
}

#[test]
fn joined_capability_rejects_tools_and_executors_without_starting_a_turn() {
    struct NoopExecutor;
    impl ChatToolExecutor for NoopExecutor {
        fn execute(&self, _name: &str, _args_json: &str) -> ChatToolResult {
            panic!("unsupported joined turn must not execute tools")
        }
    }
    let mut provider = test_provider("http://127.0.0.1:9/v1".into());
    assert!(provider.supports_joined_cancellable_send());
    for executor in [false, true] {
        provider.tools.clear();
        provider.executor = None;
        if executor {
            provider.executor = Some(Arc::new(NoopExecutor));
        } else {
            provider.tools.push(ChatToolDef {
                name: "test_tool".into(),
                description: "tool capability test".into(),
                level: "read".into(),
                input_schema_json: "{}".into(),
            });
        }
        assert!(!provider.supports_joined_cancellable_send());
        assert!(provider
            .send_cancellable_joined(ChatRequest::default(), Arc::new(AtomicBool::new(false)))
            .is_none());
        assert!(provider.supports_cancellable_send());
    }
}

#[test]
fn joined_chat_vision_deadline_closes_the_real_silent_http_connection() {
    let _transport = crate::image_relevance_judge::tests::transport_lock();
    let (base_url, request_ready, server) = silent_server();
    let provider = test_provider(base_url);
    let judge = crate::image_relevance_judge::ChatVisionJudge::new(Arc::new(provider));
    assert!(judge.supports_deadline_judging());
    let started = Instant::now();
    let worker = std::thread::spawn(move || {
        judge.judge_before(
            "product photo",
            "a glass bottle",
            &[vec![0xff, 0xd8, 0xff, 0xe0, 1]],
            started + Duration::from_secs(1),
        )
    });
    request_ready
        .recv_timeout(Duration::from_secs(3))
        .expect("the image judge reached the actual HTTP transport");
    let candidates = worker.join().expect("deadline worker exits");
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.verdict == op_image_enrich::net::RelevanceVerdict::Off),
        "silent provider cannot produce an accepted candidate"
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(server.join().expect("deadline closed the HTTP connection"));
}
