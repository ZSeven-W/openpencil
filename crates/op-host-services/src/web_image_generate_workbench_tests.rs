use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::time::Duration;

use op_editor_core::agent_settings::{ImageGenProfile, ImageGenProvider, ImageTestStatus};

use super::{generate_workbench, test_workbench, ImageGenerateError, WorkbenchPolling};

/// One canned reply, served in order by [`mock_workbench`].
#[derive(Clone)]
struct Reply {
    status: &'static str,
    body: &'static str,
}

impl Reply {
    const fn ok(body: &'static str) -> Self {
        Self {
            status: "200 OK",
            body,
        }
    }

    /// Close the connection without a response.
    const DROP: Self = Self {
        status: "",
        body: "",
    };

    const fn status(status: &'static str) -> Self {
        Self { status, body: "{}" }
    }
}

/// A mock Workbench that serves `replies` in order, then stops. Returns
/// its base URL plus every raw request head it saw — same shape as the
/// `hub_auth_client_tests` mock, so the Idempotency-Key contract is
/// assertable at the HTTP layer.
fn mock_workbench(replies: Vec<Reply>) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind mock workbench");
    let port = listener.local_addr().expect("addr").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for reply in replies {
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
            let mut body = vec![0u8; length];
            if length > 0 {
                let _ = stream.read_exact(&mut body);
            }
            let _ = tx.send(format!("{head}{}", String::from_utf8_lossy(&body)));
            // `Reply::DROP` hangs up without answering — a flaky link.
            if reply.status.is_empty() {
                continue;
            }
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
                 Connection: close\r\n\r\n{}",
                reply.status,
                reply.body.len(),
                reply.body
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        }
    });
    (format!("http://127.0.0.1:{port}"), rx)
}

fn workbench_profile(base_url: &str) -> ImageGenProfile {
    ImageGenProfile {
        id: "igp-workbench".into(),
        name: "Workbench".into(),
        provider: ImageGenProvider::Workbench,
        api_key: "wb-test-key".into(),
        model: "Qwen-Image-2.1".into(),
        base_url: Some(base_url.to_string()),
        test_status: ImageTestStatus::Idle,
    }
}

fn quick_polling() -> WorkbenchPolling {
    WorkbenchPolling {
        interval: Duration::from_millis(5),
        timeout: Duration::from_secs(2),
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client")
}

/// The shape a live workbench actually answers with: both the submit and
/// the poll reply wrap the job in `{"job": {…}}`. The flat-shape test
/// below passed while every real submit failed with a missing id.
#[tokio::test]
async fn job_wrapped_replies_from_a_live_workbench_are_read() {
    let (base, _rx) = mock_workbench(vec![
        Reply::ok(r#"{"job":{"id":"job-9","status":"queued"}}"#),
        Reply::ok(r#"{"job":{"id":"job-9","status":"running"}}"#),
        Reply::ok(
            r#"{"job":{"id":"job-9","status":"succeeded","artifacts":[{"id":"a1","name":"out.png","kind":"image","size":9,"url":"/media/tok-9"}]}}"#,
        ),
        Reply::ok("fake-png"),
    ]);
    let profile = workbench_profile(&base);
    let url = generate_workbench(
        &client(),
        "a latte",
        &profile,
        Some(1024.0),
        Some(768.0),
        quick_polling(),
    )
    .await
    .expect("a job-wrapped live reply must be read");
    assert_eq!(url, "data:image/png;base64,ZmFrZS1wbmc=");
}

#[tokio::test]
async fn submit_poll_succeed_returns_the_absolute_artifact_url() {
    let (base, rx) = mock_workbench(vec![
        Reply::ok(r#"{"id":"job-7"}"#),
        Reply::ok(r#"{"id":"job-7","status":"queued"}"#),
        Reply::ok(r#"{"id":"job-7","status":"running"}"#),
        Reply::ok(
            r#"{"id":"job-7","status":"succeeded","artifacts":[{"id":"a1","name":"out.png","kind":"image","size":9,"url":"/media/tok-123"}]}"#,
        ),
        Reply::ok("fake-png"),
    ]);
    let profile = workbench_profile(&base);

    let url = generate_workbench(
        &client(),
        "a cat",
        &profile,
        Some(1024.0),
        Some(768.0),
        quick_polling(),
    )
    .await
    .expect("generation succeeds");

    assert_eq!(url, "data:image/png;base64,ZmFrZS1wbmc=");
    let requests: Vec<String> = rx.try_iter().collect();
    assert_eq!(
        requests.len(),
        5,
        "submit + three polls + the authenticated download"
    );
    let download = &requests[4];
    assert!(download.contains("GET /media/tok-123"));
    assert!(
        download.contains("authorization: Bearer wb-test-key")
            || download.contains("Authorization: Bearer wb-test-key"),
        "/media answers 401 without the key"
    );
    let submit = &requests[0];
    assert!(submit.contains("POST /workbench-api/qwen-image"));
    assert!(
        submit.contains("authorization: Bearer wb-test-key")
            || submit.contains("Authorization: Bearer wb-test-key")
    );
    let submit_body = submit.split("\r\n\r\n").nth(1).unwrap_or_default();
    let parsed: serde_json::Value = serde_json::from_str(submit_body).expect("submit body json");
    assert_eq!(parsed["mode"], "t2i");
    assert_eq!(parsed["prompt"], "a cat");
    assert_eq!(parsed["width"], 1024);
    assert_eq!(parsed["height"], 768);
    assert_eq!(parsed["outputSizing"], "explicit");
    assert_eq!(parsed["transparent"], false);
    assert_eq!(parsed["model"], "Qwen-Image-2.1");
    assert!(requests[1].contains("GET /workbench-api/qwen-image/job-7"));
}

#[tokio::test]
async fn the_idempotency_key_header_is_present_and_unique_per_submission() {
    let (base, rx) = mock_workbench(vec![
        Reply::ok(r#"{"id":"job-1"}"#),
        Reply::ok(r#"{"id":"job-1","status":"failed","error":"boom"}"#),
        Reply::ok(r#"{"id":"job-2"}"#),
        Reply::ok(r#"{"id":"job-2","status":"failed","error":"boom"}"#),
    ]);
    let profile = workbench_profile(&base);

    let first = generate_workbench(&client(), "a cat", &profile, None, None, quick_polling()).await;
    let second =
        generate_workbench(&client(), "a cat", &profile, None, None, quick_polling()).await;
    assert!(first.is_err() && second.is_err());

    let requests: Vec<String> = rx.try_iter().collect();
    let keys: Vec<&str> = requests
        .iter()
        .filter(|request| request.contains("POST /workbench-api/qwen-image"))
        .map(|request| {
            request
                .lines()
                .find(|line| line.to_ascii_lowercase().starts_with("idempotency-key:"))
                .and_then(|line| line.split_once(':'))
                .map(|(_, value)| value.trim())
                .unwrap_or("")
        })
        .collect();
    assert_eq!(keys.len(), 2, "every submission carries the header");
    assert!(!keys[0].is_empty() && !keys[1].is_empty());
    assert_ne!(keys[0], keys[1], "each submission must mint a fresh key");
}

#[tokio::test]
async fn qwen_mode_active_conflict_maps_to_busy() {
    for reply in [
        Reply::ok(r#"{"error":"qwen_mode_active"}"#),
        Reply::status("409 Conflict"),
    ] {
        let (base, _rx) = mock_workbench(vec![reply]);
        let profile = workbench_profile(&base);
        let error = generate_workbench(&client(), "a cat", &profile, None, None, quick_polling())
            .await
            .expect_err("busy machine must fail");
        assert!(
            matches!(error, ImageGenerateError::Busy { .. }),
            "expected Busy, got {error}"
        );
        assert!(error.is_degradable());
    }
}

#[tokio::test]
async fn a_failed_job_maps_to_upstream_with_its_error() {
    let (base, _rx) = mock_workbench(vec![
        Reply::ok(r#"{"id":"job-9"}"#),
        Reply::ok(r#"{"id":"job-9","status":"failed","error":"cuda oom"}"#),
    ]);
    let profile = workbench_profile(&base);
    let error = generate_workbench(&client(), "a cat", &profile, None, None, quick_polling())
        .await
        .expect_err("failed job");
    match error {
        ImageGenerateError::Upstream { ref message, .. } => assert_eq!(message, "cuda oom"),
        other => panic!("expected Upstream, got {other}"),
    }
    assert!(error.is_degradable());
}

#[tokio::test]
async fn a_poll_that_never_finishes_times_out() {
    // Far more "running" replies than the tiny budget can consume, so
    // the loop ends on the wall-clock check — never on a dead socket.
    let running = Reply::ok(r#"{"id":"job-slow","status":"running"}"#);
    let (base, _rx) = mock_workbench(
        std::iter::once(Reply::ok(r#"{"id":"job-slow"}"#))
            .chain(std::iter::repeat_n(running, 64))
            .collect(),
    );
    let profile = workbench_profile(&base);
    let polling = WorkbenchPolling {
        interval: Duration::from_millis(5),
        timeout: Duration::from_millis(60),
    };
    let error = generate_workbench(&client(), "a cat", &profile, None, None, polling)
        .await
        .expect_err("must hit the wall-clock budget");
    assert!(matches!(error, ImageGenerateError::Timeout { .. }));
    assert!(error.is_degradable());
}

#[tokio::test]
async fn a_profile_without_base_url_is_a_config_fault_not_a_degradable_one() {
    let profile = workbench_profile("");
    let error = generate_workbench(&client(), "a cat", &profile, None, None, quick_polling())
        .await
        .expect_err("no endpoint");
    assert!(!error.is_degradable());
}

#[tokio::test]
async fn the_status_probe_maps_ready_online_error_onto_the_test_status() {
    for (body, expected) in [
        (
            r#"{"model":"Qwen-Image-2.1","online":true,"ready":true,"error":null}"#,
            ImageTestStatus::Valid,
        ),
        (
            r#"{"model":"Qwen-Image-2.1","online":true,"ready":false,"error":null}"#,
            ImageTestStatus::Invalid,
        ),
        (
            r#"{"model":"Qwen-Image-2.1","online":false,"ready":false,"error":"warmup"}"#,
            ImageTestStatus::Invalid,
        ),
    ] {
        let (base, rx) = mock_workbench(vec![Reply::ok(body)]);
        let profile = workbench_profile(&base);
        assert_eq!(test_workbench(&client(), &profile).await, expected);
        let requests: Vec<String> = rx.try_iter().collect();
        assert!(
            requests[0].contains("GET /workbench-api/qwen-image/status"),
            "the probe must hit the status endpoint"
        );
    }
}

#[test]
fn private_and_tailnet_hosts_bypass_the_inherited_proxy() {
    for url in [
        "http://100.64.12.34:8787",
        "http://192.168.1.20:8787/",
        "http://10.0.0.5",
        "http://127.0.0.1:8787",
        "http://localhost:8787",
        "https://studio.example.ts.net/",
        "http://studio.local",
    ] {
        assert!(
            super::bypasses_proxy(url),
            "{url} should be dialled directly"
        );
    }
    for url in ["https://api.openai.com", "http://8.8.8.8", "not a url", ""] {
        assert!(
            !super::bypasses_proxy(url),
            "{url} keeps the configured proxy"
        );
    }
}

#[test]
fn slot_boxes_map_onto_resolutions_the_workbench_accepts() {
    for (w, h) in [
        (Some(1440.0), Some(120.0)),
        (Some(180.0), Some(240.0)),
        (Some(375.0), Some(812.0)),
        (Some(1024.0), Some(768.0)),
        (None, None),
        (Some(0.0), Some(50.0)),
    ] {
        let (gw, gh) = super::workbench_size(w, h);
        for edge in [gw, gh] {
            assert_eq!(edge % 32, 0, "{w:?}x{h:?} -> {gw}x{gh}");
            assert!((256..=2048).contains(&edge), "{w:?}x{h:?} -> {gw}x{gh}");
        }
    }
    assert_eq!(
        super::workbench_size(Some(1024.0), Some(768.0)),
        (1024, 768)
    );
    assert_eq!(super::workbench_size(None, None), (1024, 1024));
    // Portrait keeps its orientation.
    let (pw, ph) = super::workbench_size(Some(375.0), Some(812.0));
    assert!(ph > pw);
}

#[tokio::test]
async fn a_rate_limited_submit_waits_and_resubmits() {
    // Measured on the live machine (0923): an enrichment pass fanning out
    // eleven slots got `429 {"error":"rate_limited"}` and every throttled
    // slot fell straight back to stock search.
    let (base, rx) = mock_workbench(vec![
        Reply {
            status: "429 Too Many Requests",
            body: r#"{"error":"rate_limited"}"#,
        },
        Reply::ok(r#"{"job":{"id":"job-3","status":"queued"}}"#),
        Reply::ok(
            r#"{"job":{"id":"job-3","status":"succeeded","artifacts":[{"id":"a1","name":"out.png","kind":"image","size":9,"url":"/media/tok-3"}]}}"#,
        ),
        Reply::ok("fake-png"),
    ]);
    let profile = workbench_profile(&base);
    let url = generate_workbench(
        &client(),
        "a bowl of noodles",
        &profile,
        Some(512.0),
        Some(512.0),
        quick_polling(),
    )
    .await
    .expect("a throttled submit must be retried, not surfaced");
    assert_eq!(url, "data:image/png;base64,ZmFrZS1wbmc=");
    let heads: Vec<String> = rx.try_iter().collect();
    let keys: Vec<&str> = heads
        .iter()
        .filter(|head| head.starts_with("POST"))
        .filter_map(|head| {
            head.lines()
                .find(|line| line.to_ascii_lowercase().starts_with("idempotency-key:"))
        })
        .collect();
    assert_eq!(keys.len(), 2, "one resubmission: {heads:?}");
    assert_ne!(keys[0], keys[1], "each submission carries a fresh key");
}

#[tokio::test]
async fn a_machine_that_stays_rate_limited_degrades_as_busy_within_budget() {
    let throttled = Reply {
        status: "429 Too Many Requests",
        body: r#"{"error":"rate_limited"}"#,
    };
    let (base, _rx) = mock_workbench(vec![throttled; 64]);
    let profile = workbench_profile(&base);
    let error = generate_workbench(
        &client(),
        "a bowl of noodles",
        &profile,
        Some(512.0),
        Some(512.0),
        quick_polling(),
    )
    .await
    .expect_err("a machine that never accepts must give up");
    assert!(
        matches!(error, ImageGenerateError::Busy { .. }),
        "expected Busy, got {error}"
    );
    assert!(error.is_degradable());
}

#[tokio::test]
async fn a_rate_limited_poll_keeps_polling_the_running_job() {
    let (base, _rx) = mock_workbench(vec![
        Reply::ok(r#"{"job":{"id":"job-4","status":"queued"}}"#),
        Reply {
            status: "429 Too Many Requests",
            body: r#"{"error":"rate_limited"}"#,
        },
        Reply::ok(
            r#"{"job":{"id":"job-4","status":"succeeded","artifacts":[{"id":"a1","name":"out.png","kind":"image","size":9,"url":"/media/tok-4"}]}}"#,
        ),
        Reply::ok("fake-png"),
    ]);
    let profile = workbench_profile(&base);
    let url = generate_workbench(
        &client(),
        "a bowl of noodles",
        &profile,
        Some(512.0),
        Some(512.0),
        quick_polling(),
    )
    .await
    .expect("a throttled poll is not a failed job");
    assert_eq!(url, "data:image/png;base64,ZmFrZS1wbmc=");
}

#[tokio::test]
async fn a_dropped_poll_and_a_dropped_download_do_not_lose_the_job() {
    // Measured on the live machine (0923): a flaky tailnet dropped polls and
    // artifact fetches, and six rendered jobs fell back to stock search.
    let (base, _rx) = mock_workbench(vec![
        Reply::ok(r#"{"job":{"id":"job-5","status":"queued"}}"#),
        Reply::DROP,
        Reply::ok(
            r#"{"job":{"id":"job-5","status":"succeeded","artifacts":[{"id":"a1","name":"out.png","kind":"image","size":9,"url":"/media/tok-5"}]}}"#,
        ),
        Reply::DROP,
        Reply::ok("fake-png"),
    ]);
    let profile = workbench_profile(&base);
    let url = generate_workbench(
        &client(),
        "a bowl of noodles",
        &profile,
        Some(512.0),
        Some(512.0),
        quick_polling(),
    )
    .await
    .expect("transient drops on an accepted job must be retried");
    assert_eq!(url, "data:image/png;base64,ZmFrZS1wbmc=");
}
