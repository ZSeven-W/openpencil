use super::*;
use base64::Engine as _;
use op_ai::chat_provider::ChatAttachment;
use op_image_enrich::net::{ImageRelevanceJudge, RelevanceVerdict};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Instant;

const QUERY: &str = "glass bottle closeup";
const BRIEF: &str = "macro photograph of a glass bottle, warm daylight";

fn image_bytes(jpeg: bool) -> Vec<u8> {
    let mut surface = skia_safe::surfaces::raster_n32_premul((3, 2)).unwrap();
    surface.canvas().clear(skia_safe::Color::BLUE);
    surface
        .image_snapshot()
        .encode(
            None,
            if jpeg {
                skia_safe::EncodedImageFormat::JPEG
            } else {
                skia_safe::EncodedImageFormat::PNG
            },
            80,
        )
        .unwrap()
        .as_bytes()
        .to_vec()
}

fn test_provider(kind: BuiltinAgentKind, base_url: String) -> ConfiguredBuiltinProvider {
    let model = match kind {
        BuiltinAgentKind::OpenAiCompat => "glm-5.3-flash",
        BuiltinAgentKind::Anthropic => "claude-vision-test",
    };
    let config = BuiltinAgentConfig {
        id: "native-vision-test".into(),
        preset: op_editor_core::BuiltinAgentPresetKey::Custom,
        display_name: "Native vision test".into(),
        kind,
        api_key: "sk-test".into(),
        models: vec![model.into()],
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

fn read_body(stream: &mut TcpStream) -> Value {
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
        .expect("JSON body content length");
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).expect("complete JSON request body")
}

fn capture_server(
    kind: BuiltinAgentKind,
    reply: String,
) -> (String, std::thread::JoinHandle<Value>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let payload = match kind {
        BuiltinAgentKind::OpenAiCompat => format!(
            "data: {}\n\ndata: [DONE]\n\n",
            json!({"choices": [{"delta": {"content": reply}}]})
        ),
        BuiltinAgentKind::Anthropic => format!(
            "data: {}\n\ndata: {}\n\n",
            json!({"type": "content_block_delta", "delta": {"type": "text_delta", "text": reply}}),
            json!({"type": "message_delta", "delta": {"stop_reason": "end_turn"}})
        ),
    };
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
        let body = read_body(&mut stream);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            payload.len(), payload
        );
        stream.write_all(response.as_bytes()).unwrap();
        body
    });
    (format!("http://{address}/v1"), server)
}

fn assert_native_image(body: &Value, kind: BuiltinAgentKind, media_type: &str, bytes: &[u8]) {
    let user = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|message| message["role"] == "user")
        .unwrap();
    let blocks = user["content"]
        .as_array()
        .expect("native multimodal content");
    assert_eq!(blocks.len(), 2, "one text block and one image block");
    let text = blocks.iter().find(|block| block["type"] == "text").unwrap()["text"]
        .as_str()
        .unwrap();
    assert!(text.contains(QUERY) && text.contains(BRIEF));
    assert!(!text.contains("[attached") && !text.contains("openpencil-chat-"));
    assert!(!text.contains(std::env::temp_dir().to_string_lossy().as_ref()));
    let original: Value =
        serde_json::from_str(text.split_once("Original image brief: ").unwrap().1).unwrap();
    assert_eq!(original["original_query"], QUERY);
    assert_eq!(original["image_prompt"], BRIEF);
    let encoded = match kind {
        BuiltinAgentKind::OpenAiCompat => {
            assert_eq!(body["model"], "glm-5.3-flash");
            assert_eq!(body["thinking"]["type"], "disabled");
            assert!(body.get("reasoning_effort").is_none());
            let image = blocks
                .iter()
                .find(|block| block["type"] == "image_url")
                .unwrap();
            image["image_url"]["url"]
                .as_str()
                .unwrap()
                .strip_prefix(&format!("data:{media_type};base64,"))
                .expect("image data URL carries actual MIME type")
        }
        BuiltinAgentKind::Anthropic => {
            assert_eq!(body["model"], "claude-vision-test");
            assert!(body.get("thinking").is_none());
            let image = blocks
                .iter()
                .find(|block| block["type"] == "image")
                .unwrap();
            assert_eq!(image["source"]["type"], "base64");
            assert_eq!(image["source"]["media_type"], media_type);
            image["source"]["data"].as_str().unwrap()
        }
    };
    assert_eq!(
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap(),
        bytes
    );
    assert_eq!(body["max_tokens"], 1000);
    assert_eq!(body["stream"], true);
}

#[test]
fn joined_vision_judge_sends_real_jpeg_and_png_bytes_on_both_native_wires() {
    let _transport = crate::image_relevance_judge::tests::transport_lock();
    for kind in [BuiltinAgentKind::OpenAiCompat, BuiltinAgentKind::Anthropic] {
        for jpeg in [true, false] {
            let (verdict, expected) = if jpeg {
                ("on", RelevanceVerdict::On)
            } else {
                ("weak", RelevanceVerdict::Weak)
            };
            let reply = crate::image_relevance_judge::tests::valid_reply(verdict, "visual match");
            let (base_url, server) = capture_server(kind, reply);
            let provider = test_provider(kind, base_url);
            let judge = crate::image_relevance_judge::ChatVisionJudge::new(Arc::new(provider));
            assert!(judge.supports_deadline_judging());
            let bytes = image_bytes(jpeg);
            let candidates = judge.judge_before(
                QUERY,
                BRIEF,
                std::slice::from_ref(&bytes),
                Instant::now() + Duration::from_secs(3),
            );
            let body = server.join().expect("capture server exits");
            assert_eq!(candidates.len(), 1);
            assert_eq!(candidates[0].verdict, expected, "{candidates:?}");
            assert_native_image(
                &body,
                kind,
                if jpeg { "image/jpeg" } else { "image/png" },
                &bytes,
            );
        }
    }
}

#[test]
fn legacy_send_keeps_attachment_paths_in_the_text_wire() {
    for kind in [BuiltinAgentKind::OpenAiCompat, BuiltinAgentKind::Anthropic] {
        let (base_url, server) = capture_server(kind, "legacy-ok".into());
        let provider = test_provider(kind, base_url);
        let deltas: Vec<_> = provider
            .send(ChatRequest {
                system_prompt: "Use the existing attachment workflow.".into(),
                user_message: "inspect the legacy attachment".into(),
                max_output_tokens: 73,
                attachments: vec![ChatAttachment {
                    name: "legacy.png".into(),
                    media_type: "image/png".into(),
                    data: image_bytes(false),
                }],
                ..Default::default()
            })
            .collect();
        assert!(deltas
            .iter()
            .any(|delta| matches!(delta, ChatDelta::TextDelta(text) if text == "legacy-ok")));
        let body = server.join().unwrap();
        let messages = body["messages"].as_array().unwrap();
        let text = messages.last().unwrap()["content"]
            .as_str()
            .expect("legacy text wire");
        assert!(text.contains("inspect the legacy attachment"));
        assert!(text.contains("[attached image:") && text.contains("openpencil-chat-"));
        assert_eq!(body["max_tokens"], 73);
    }
}

#[test]
fn joined_non_image_attachment_fails_without_sending_a_text_only_http_request() {
    for kind in [BuiltinAgentKind::OpenAiCompat, BuiltinAgentKind::Anthropic] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let provider = test_provider(
            kind,
            format!("http://{}/v1", listener.local_addr().unwrap()),
        );
        let deltas: Vec<_> = provider
            .send_cancellable_joined(
                ChatRequest {
                    system_prompt: "Inspect attached evidence.".into(),
                    user_message: "do not judge a missing image".into(),
                    attachments: vec![ChatAttachment {
                        name: "notes.txt".into(),
                        media_type: "text/plain".into(),
                        data: b"not an image".to_vec(),
                    }],
                    ..Default::default()
                },
                Arc::new(AtomicBool::new(false)),
            )
            .unwrap()
            .collect();
        assert!(
            deltas
                .iter()
                .any(|delta| matches!(delta, ChatDelta::Error(_))),
            "{deltas:?}"
        );
        assert!(!deltas
            .iter()
            .any(|delta| matches!(delta, ChatDelta::TextDelta(_))));
        assert!(
            matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock)
        );
    }
}
