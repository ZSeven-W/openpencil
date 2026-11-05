use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider, ImageTestStatus};

use super::{generate_openai, images_endpoint, request_body, test_openai_compatible};

const ARK_BASE: &str = "https://ark.cn-beijing.volces.com/api/v3";

fn profile(model: &str, base_url: Option<&str>) -> ImageGenProfile {
    ImageGenProfile {
        id: "p".into(),
        name: "p".into(),
        provider: ImageGenProvider::Custom,
        api_key: "test-key".into(),
        model: model.into(),
        base_url: base_url.map(str::to_string),
        test_status: ImageTestStatus::Idle,
    }
}

/// One-shot mock: answers a single request with `status` + `body`, and
/// reports the raw request (head + body) it received.
fn mock_once(status: &'static str, body: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind mock");
    let port = listener.local_addr().expect("addr").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let Ok((mut stream, _)) = listener.accept() else {
            return;
        };
        let mut seen = Vec::new();
        let mut byte = [0u8; 1];
        while stream.read(&mut byte).map(|n| n == 1).unwrap_or(false) {
            seen.push(byte[0]);
            if seen.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let head = String::from_utf8_lossy(&seen).into_owned();
        let length = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.trim().split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        let mut request_body = vec![0u8; length];
        let _ = stream.read_exact(&mut request_body);
        let _ = tx.send(format!("{head}{}", String::from_utf8_lossy(&request_body)));
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len(),
        );
        let _ = stream.write_all(response.as_bytes());
    });
    (format!("http://127.0.0.1:{port}"), rx)
}

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    crate::chat_runtime::block_on_anywhere(future)
}

#[test]
fn endpoint_keeps_a_versioned_base_and_adds_v1_to_a_bare_host() {
    let cases = [
        (None, "https://api.openai.com/v1/images/generations"),
        (Some(""), "https://api.openai.com/v1/images/generations"),
        (
            Some("https://api.openai.com"),
            "https://api.openai.com/v1/images/generations",
        ),
        (
            Some("https://api.openai.com/v1"),
            "https://api.openai.com/v1/images/generations",
        ),
        (
            Some("https://ark.cn-beijing.volces.com/api/v3/"),
            "https://ark.cn-beijing.volces.com/api/v3/images/generations",
        ),
        (
            Some("http://localhost:8080/v1beta"),
            "http://localhost:8080/v1beta/images/generations",
        ),
        (
            Some("https://gw.example.com/openai"),
            "https://gw.example.com/openai/v1/images/generations",
        ),
        // A host that merely starts with `v1` is not a version segment.
        (
            Some("https://v1.example.com"),
            "https://v1.example.com/v1/images/generations",
        ),
        (
            Some("https://gw.example.com/api/v3/images/generations"),
            "https://gw.example.com/api/v3/images/generations",
        ),
    ];
    for (base, expected) in cases {
        assert_eq!(images_endpoint(base), expected, "base {base:?}");
    }
}

#[test]
fn seedream_gets_an_aspect_matched_2k_size_without_watermark() {
    let seedream = profile("doubao-seedream-5-0-flash-260915", Some(ARK_BASE));
    let body = request_body("cat", &seedream, Some(1600.0), Some(900.0));
    assert_eq!(body["size"], "2736x1536");
    assert_eq!(body["watermark"], false);
    assert_eq!(body["model"], "doubao-seedream-5-0-flash-260915");

    let square = request_body("cat", &seedream, None, None);
    assert_eq!(square["size"], "2048x2048");

    // Extreme slots clamp at 4:1 so the long side stays within 4096.
    let banner = request_body("cat", &seedream, Some(3000.0), Some(100.0));
    assert_eq!(banner["size"], "4096x1024");

    // An Ark inference-endpoint id names no model; the host identifies it.
    let endpoint_id = profile("ep-20261030-abcde", Some(ARK_BASE));
    assert_eq!(
        request_body("cat", &endpoint_id, None, None)["size"],
        "2048x2048"
    );
}

#[test]
fn seedream_3_keeps_the_dalle_sizes_and_plain_openai_is_unchanged() {
    let v3 = profile("doubao-seedream-3-0-t2i-250415", Some(ARK_BASE));
    let body = request_body("cat", &v3, Some(1600.0), Some(900.0));
    assert_eq!(body["size"], "1792x1024");
    assert_eq!(body["watermark"], false);

    let openai = profile("gpt-image-1", None);
    let body = request_body("cat", &openai, Some(900.0), Some(1600.0));
    assert_eq!(body["size"], "1024x1792");
    assert!(body.get("watermark").is_none());
    assert_eq!(body["response_format"], "url");
}

#[test]
fn generation_posts_to_the_versioned_base_path() {
    let (base, rx) = mock_once(
        "200 OK",
        r#"{"data":[{"url":"https://cdn.example/a.jpeg"}]}"#,
    );
    let p = profile("doubao-seedream-5-0", Some(&format!("{base}/api/v3")));
    let client = reqwest::Client::new();
    let url = block_on(generate_openai(&client, "a cat", &p, None, None)).expect("generated");
    assert_eq!(url, "https://cdn.example/a.jpeg");
    let request = rx.recv().expect("request seen");
    assert!(
        request.starts_with("POST /api/v3/images/generations HTTP/1.1"),
        "{request}"
    );
    assert!(request.contains("\"watermark\":false"), "{request}");
}

#[test]
fn test_probe_accepts_a_prompt_rejection_and_refuses_key_path_or_model_faults() {
    let cases = [
        (
            "400 Bad Request",
            r#"{"error":{"code":"InvalidParameter","message":"the parameter `prompt` must not be empty"}}"#,
            ImageTestStatus::Valid,
        ),
        (
            "401 Unauthorized",
            r#"{"error":{"message":"the API key is missing or invalid"}}"#,
            ImageTestStatus::Invalid,
        ),
        ("404 Not Found", r#"{}"#, ImageTestStatus::Invalid),
        (
            "400 Bad Request",
            r#"{"error":{"message":"The model `nope` does not exist"}}"#,
            ImageTestStatus::Invalid,
        ),
    ];
    let client = reqwest::Client::new();
    for (status, body, expected) in cases {
        let (base, rx) = mock_once(status, body);
        let p = profile("doubao-seedream-5-0", Some(&format!("{base}/api/v3")));
        assert_eq!(
            block_on(test_openai_compatible(&client, &p)),
            expected,
            "{status} {body}"
        );
        let request = rx.recv().expect("request seen");
        assert!(
            request.starts_with("POST /api/v3/images/generations"),
            "{request}"
        );
        assert!(request.contains("\"prompt\":\"\""), "{request}");
    }
}
