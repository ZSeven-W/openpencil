use super::*;
use op_ai::chat_provider::ChatToolResult;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct NoopExecutor;

impl ChatToolExecutor for NoopExecutor {
    fn execute(&self, _name: &str, _args_json: &str) -> ChatToolResult {
        ChatToolResult {
            content: "{}".into(),
            is_error: false,
        }
    }
}

fn test_provider(base_url: String) -> ConfiguredBuiltinProvider {
    let config = BuiltinAgentConfig {
        id: "cancel-test".into(),
        preset: op_editor_core::BuiltinAgentPresetKey::Custom,
        display_name: "Cancel test".into(),
        kind: BuiltinAgentKind::OpenAiCompat,
        api_key: "sk-test".into(),
        models: vec!["test-model".into()],
        base_url,
        enabled: true,
    };
    let mut provider =
        ConfiguredBuiltinProvider::from_builtin_agent(&config).expect("ready test provider");
    provider.min_gap = Duration::ZERO;
    provider.max_retries = 0;
    provider
}

#[test]
fn configured_builtin_cancellable_send_aborts_a_silent_http_turn() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind silent provider");
    listener
        .set_nonblocking(true)
        .expect("nonblocking silent provider");
    let address = listener.local_addr().expect("silent provider address");
    let stop = Arc::new(AtomicBool::new(false));
    let server_stop = Arc::clone(&stop);
    let server = std::thread::spawn(move || {
        let mut held_connections = Vec::new();
        while !server_stop.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((stream, _)) => held_connections.push(stream),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(_) => break,
            }
        }
    });

    let provider = test_provider(format!("http://{address}/v1"));
    assert!(provider.supports_cancellable_send());
    assert!(provider.supports_evidence_only_send());
    let cancel = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancel);
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(40));
        signal.store(true, Ordering::Release);
    });

    let started = Instant::now();
    let deltas: Vec<_> = provider
        .send_cancellable(
            ChatRequest {
                user_message: "extract design system".into(),
                ..Default::default()
            },
            cancel,
        )
        .collect();
    assert!(
        deltas.is_empty(),
        "cancelled transport leaked deltas: {deltas:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "cancellation waited for the silent HTTP provider"
    );

    stop.store(true, Ordering::Release);
    server.join().expect("silent provider server exits");
}

#[test]
fn configured_builtin_evidence_capability_rejects_tool_wiring() {
    let mut provider = test_provider("http://127.0.0.1:9/v1".into());
    assert!(provider.supports_evidence_only_send());
    provider.executor = Some(Arc::new(NoopExecutor));
    assert!(!provider.supports_evidence_only_send());
    provider.executor = None;
    provider.tools.push(ChatToolDef {
        name: "write_file".into(),
        description: "must not reach evidence extraction".into(),
        level: "modify".into(),
        input_schema_json: "{}".into(),
    });
    assert!(!provider.supports_evidence_only_send());
}
