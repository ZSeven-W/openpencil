//! Route fall-through, shutdown authentication, and `serve-web` argv parsing.
//!
//! Split out of `web_canvas_server_conn_tests.rs` at the repo's 800-line cap
//! (same reason `web_canvas_server_sse_tests.rs` was carved off earlier);
//! nested under it so `use super::*` still reaches the mock stream and the
//! `serve` helpers. Pure code motion.

use super::*;

#[test]
fn serve_one_unimplemented_api_route_is_404_not_jsonrpc() {
    // An `/api/mcp/*` route this daemon doesn't implement must 404, not
    // fall through to JSON-RPC dispatch.
    let r = serve("POST", "/api/mcp/not-a-route", "");
    assert!(r.contains("404 Not Found"), "{r}");
}

#[test]
fn serve_one_get_root_serves_html_not_jsonrpc() {
    // `GET /` is the static host-page route now — text/html either way
    // (200 host page with a bundle, 404 build-help page without one) and
    // never the old 405 from the JSON-RPC path guard.
    let r = serve("GET", "/", "");
    assert!(r.contains("Content-Type: text/html"), "{r}");
    assert!(!r.contains("405"), "{r}");
    // `POST /` keeps dispatching JSON-RPC (web_static ignores non-GET).
    let post = serve(
        "POST",
        "/",
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    );
    assert!(post.contains("200 OK"), "{post}");
    assert!(post.contains(r#""tools""#), "{post}");
}

#[test]
fn serve_one_token_authed_shutdown_signals_caller() {
    std::env::set_var("OPENPENCIL_MCP_TOKEN", "serve-web-shutdown-test");
    let state = Mutex::new(fresh_state());
    let hub = SseHub::default();
    // Wrong token → NOT a shutdown (falls through to JSON-RPC dispatch).
    let bad =
        r#"{"jsonrpc":"2.0","id":1,"method":"openpencil/shutdown","params":{"token":"nope"}}"#;
    let mut stream = mock_stream(&format!(
        "POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{bad}",
        bad.len()
    ));
    let wants_shutdown = serve_one(&mut stream, &state, &hub).expect("serve_one");
    assert!(!wants_shutdown, "a mismatched token must not shut down");
    // Matching token → ack + shutdown signal for the accept loop.
    let good = r#"{"jsonrpc":"2.0","id":2,"method":"openpencil/shutdown","params":{"token":"serve-web-shutdown-test"}}"#;
    let mut stream = mock_stream(&format!(
        "POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{good}",
        good.len()
    ));
    let wants_shutdown = serve_one(&mut stream, &state, &hub).expect("serve_one");
    assert!(wants_shutdown);
    let out = String::from_utf8_lossy(&stream.output);
    assert!(out.contains(r#""shuttingDown":true"#), "{out}");
}

#[test]
fn managed_handshake_token_is_lifecycle_only_and_authenticates_shutdown_body() {
    let lifecycle_token = "managed-lifecycle-test";
    let mut managed = fresh_state();
    managed.mode = ServeMode::Managed;
    managed.managed_token = Some(lifecycle_token.into());
    let state = Mutex::new(managed);
    let hub = SseHub::default();

    let body = format!(
        r#"{{"jsonrpc":"2.0","id":7,"method":"openpencil/shutdown","params":{{"token":"{lifecycle_token}"}}}}"#
    );
    let mut stream = mock_stream(&format!(
        "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:3100\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    ));

    let wants_shutdown =
        serve_one_in_mode(&mut stream, &state, &hub, ServeMode::Managed).expect("serve_one");

    assert!(wants_shutdown);
    let response = String::from_utf8_lossy(&stream.output);
    assert!(response.contains(r#""shuttingDown":true"#), "{response}");
}

#[test]
fn parse_serve_web_args_accepts_port_doc_and_host() {
    let parse = |args: &[&str]| parse_serve_web_args(args.iter().map(|s| s.to_string()));
    // Port only → loopback, empty document.
    let o = parse(&["3100"]).expect("port only");
    assert_eq!((o.port, o.managed), (3100, false));
    assert_eq!(o.path, None);
    assert_eq!(o.host, "127.0.0.1");
    assert!(o.allow_origins.is_empty());
    // Port + doc.
    let o = parse(&["3100", "/tmp/d.op"]).expect("port+doc");
    assert_eq!(o.port, 3100);
    assert_eq!(o.path.as_deref(), Some(std::path::Path::new("/tmp/d.op")));
    assert_eq!(o.host, "127.0.0.1");
    // `--host` in both spellings, before or after the doc.
    let o = parse(&["3100", "--host", "0.0.0.0", "/tmp/d.op"]).expect("host then doc");
    assert_eq!(o.port, 3100);
    assert_eq!(o.path.as_deref(), Some(std::path::Path::new("/tmp/d.op")));
    assert_eq!(o.host, "0.0.0.0");
    let o = parse(&["3100", "/tmp/d.op", "--host=0.0.0.0"]).expect("doc then host=");
    assert_eq!(o.port, 3100);
    assert_eq!(o.path.as_deref(), Some(std::path::Path::new("/tmp/d.op")));
    assert_eq!(o.host, "0.0.0.0");
    // Malformed shapes are rejected with a message, not silently dropped.
    assert!(parse(&[]).is_err(), "missing port");
    assert!(parse(&["nope"]).is_err(), "non-numeric port");
    assert!(parse(&["3100", "--host"]).is_err(), "--host without value");
    assert!(parse(&["3100", "a.op", "b.op"]).is_err(), "two docs");
}

#[test]
fn parse_serve_web_args_legacy_positional_unchanged() {
    let o = parse_serve_web_args(vec!["3100".into(), "doc.op".into()].into_iter()).unwrap();
    assert_eq!((o.port, o.managed), (3100, false));
    assert_eq!(o.path.as_deref(), Some(std::path::Path::new("doc.op")));
    assert_eq!(o.host, "127.0.0.1");
}

#[test]
fn parse_serve_web_args_managed_flag_form() {
    let o = parse_serve_web_args(
        vec![
            "--managed".into(),
            "--port".into(),
            "0".into(),
            "--file".into(),
            "a.op".into(),
            "--allow-origin".into(),
            "vscode-webview://x".into(),
            "--allow-origin".into(),
            "vscode-webview://y".into(),
        ]
        .into_iter(),
    )
    .unwrap();
    assert!(o.managed);
    assert_eq!(o.port, 0);
    assert_eq!(o.path.as_deref(), Some(std::path::Path::new("a.op")));
    assert_eq!(o.allow_origins.len(), 2);
}

#[test]
fn parse_serve_web_args_managed_accepts_only_loopback_hosts() {
    for host in ["127.0.0.1", "localhost", "::1"] {
        let o = parse_serve_web_args(
            vec![
                "--managed".into(),
                "--port".into(),
                "0".into(),
                "--host".into(),
                host.into(),
            ]
            .into_iter(),
        )
        .unwrap_or_else(|error| panic!("managed host {host:?} was rejected: {error}"));
        assert_eq!(o.host, host);
    }

    for host in ["0.0.0.0", "192.168.1.40", "example.com"] {
        let error = parse_serve_web_args(
            vec![
                "--managed".into(),
                "--port".into(),
                "0".into(),
                "--host".into(),
                host.into(),
            ]
            .into_iter(),
        )
        .err()
        .unwrap_or_else(|| panic!("managed host {host:?} must be rejected"));
        assert!(error.to_string().contains("loopback --host"), "{error}");
    }
}

#[test]
fn parse_serve_web_args_local_and_online_keep_non_loopback_opt_in() {
    let local =
        parse_serve_web_args(vec!["3100".into(), "--host".into(), "0.0.0.0".into()].into_iter())
            .expect("local LAN bind");
    assert!(!local.managed);
    assert!(!local.online);
    assert_eq!(local.host, "0.0.0.0");

    let online = parse_serve_web_args(
        vec![
            "--online".into(),
            "--port".into(),
            "3100".into(),
            "--host".into(),
            "0.0.0.0".into(),
        ]
        .into_iter(),
    )
    .expect("online Docker bind");
    assert!(!online.managed);
    assert!(online.online);
    assert_eq!(online.host, "0.0.0.0");
}

#[test]
fn handshake_line_is_single_line_json() {
    let line = handshake_json(41234, "aabbccdd00112233aabbccdd00112233");
    assert!(!line.contains('\n'));
    assert!(line.contains(r#""port":41234"#));
    assert!(line.contains(r#""token":"aabbccdd00112233aabbccdd00112233""#));
}

#[test]
fn indicators_endpoint_serves_parseable_relay_json() {
    let mut s = WebCanvasState::new(EditorState::starter(), 3100);

    let r = handle_web_canvas_request("GET", "/api/mcp/indicators", "", &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    let remote = op_editor_core::agent_indicators::parse_relay_json(&r.body)
        .expect("relay body parses back through the browser-side parser");
    // No design run in this test process — idle registry relays as such.
    assert!(!remote.run_active);
}

// --- serve_one layer: managed tokenless requests + Origin enforcement ---

#[test]
fn managed_request_origin_gate_is_tokenless_and_exact() {
    let allow = vec!["vscode-webview://abc".to_string()];
    assert!(managed_request_origin_allowed(&allow, None, None));
    assert!(managed_request_origin_allowed(
        &allow,
        Some("vscode-webview://abc"),
        Some("127.0.0.1:60615")
    ));
    assert!(!managed_request_origin_allowed(
        &allow,
        Some("vscode-webview://evil"),
        Some("127.0.0.1:60615")
    ));
    assert!(!managed_request_origin_allowed(
        &["*".into()],
        Some("*"),
        Some("127.0.0.1:60615")
    ));
}

#[test]
fn managed_request_origin_gate_accepts_only_the_exact_loopback_http_origin() {
    let allow = Vec::new();
    for (origin, host) in [
        ("http://127.0.0.1:60615", "127.0.0.1:60615"),
        ("http://localhost:60615", "localhost:60615"),
        ("http://[::1]:60615", "[::1]:60615"),
    ] {
        assert!(
            managed_request_origin_allowed(&allow, Some(origin), Some(host)),
            "{origin} must be accepted for its own loopback Host {host}"
        );
    }

    for (origin, host) in [
        ("http://127.0.0.1:60616", "127.0.0.1:60615"),
        ("http://localhost:60615", "127.0.0.1:60615"),
        ("https://127.0.0.1:60615", "127.0.0.1:60615"),
        ("http://canvas.example:60615", "canvas.example:60615"),
        ("http://127.0.0.1:60615", "canvas.example:60615"),
    ] {
        assert!(
            !managed_request_origin_allowed(&allow, Some(origin), Some(host)),
            "{origin} must be rejected for Host {host}"
        );
    }
    assert!(!managed_request_origin_allowed(
        &allow,
        Some("http://127.0.0.1:60615"),
        None
    ));
}

#[test]
fn managed_daemon_serves_health_without_request_credentials() {
    let request =
        "GET /api/mcp/server HTTP/1.1\r\nHost: 127.0.0.1:3100\r\nContent-Length: 0\r\n\r\n";
    let mut stream = mock_stream(request);
    let mut managed = fresh_state();
    managed.mode = ServeMode::Managed;
    managed.managed_token = Some("lifecycle-only".into());
    managed.allow_origins = vec!["vscode-webview://abc".into()];
    let state = Mutex::new(managed);

    serve_one_in_mode(&mut stream, &state, &SseHub::default(), ServeMode::Managed)
        .expect("serve_one");

    let response = String::from_utf8_lossy(&stream.output);
    assert!(response.contains("200 OK"), "{response}");
    assert!(response.contains(r#""running":true"#), "{response}");
    assert!(!response.contains("401 Unauthorized"), "{response}");
}

#[test]
fn managed_daemon_rejects_a_browser_origin_outside_its_allowlist() {
    let request = "GET /api/mcp/version HTTP/1.1\r\nHost: 127.0.0.1:3100\r\nOrigin: vscode-webview://evil\r\nContent-Length: 0\r\n\r\n";
    let mut stream = mock_stream(request);
    let mut managed = fresh_state();
    managed.mode = ServeMode::Managed;
    managed.managed_token = Some("lifecycle-only".into());
    managed.allow_origins = vec!["vscode-webview://abc".into()];
    let state = Mutex::new(managed);

    serve_one_in_mode(&mut stream, &state, &SseHub::default(), ServeMode::Managed)
        .expect("serve_one");

    let response = String::from_utf8_lossy(&stream.output);
    assert!(response.contains("403 Forbidden"), "{response}");
    assert!(
        response.contains("request origin is not allowed"),
        "{response}"
    );
}

#[test]
fn managed_daemon_serves_pkg_to_its_own_loopback_origin() {
    let request = "GET /pkg/op_host_web.js HTTP/1.1\r\nHost: 127.0.0.1:60615\r\nOrigin: http://127.0.0.1:60615\r\nContent-Length: 0\r\n\r\n";
    let mut stream = mock_stream(request);
    let mut managed = fresh_state();
    managed.mode = ServeMode::Managed;
    managed.managed_token = Some("lifecycle-only".into());
    // The supervisor is served on a different port. Its allowlist entry must
    // not prevent the daemon iframe from loading its own module graph.
    managed.allow_origins = vec!["http://127.0.0.1:57401".into()];
    let state = Mutex::new(managed);

    serve_one_in_mode(&mut stream, &state, &SseHub::default(), ServeMode::Managed)
        .expect("serve_one");

    let response = String::from_utf8_lossy(&stream.output);
    assert!(!response.contains("403 Forbidden"), "{response}");
    assert!(
        response.contains("Access-Control-Allow-Origin: http://127.0.0.1:60615"),
        "{response}"
    );
}

#[test]
fn managed_daemon_rejects_pkg_from_a_hostile_or_wrong_port_origin() {
    for origin in ["https://evil.example", "http://127.0.0.1:60616"] {
        let request = format!(
            "GET /pkg/op_host_web.js HTTP/1.1\r\nHost: 127.0.0.1:60615\r\nOrigin: {origin}\r\nContent-Length: 0\r\n\r\n"
        );
        let mut stream = mock_stream(&request);
        let mut managed = fresh_state();
        managed.mode = ServeMode::Managed;
        managed.managed_token = Some("lifecycle-only".into());
        managed.allow_origins = vec!["http://127.0.0.1:57401".into()];
        let state = Mutex::new(managed);

        serve_one_in_mode(&mut stream, &state, &SseHub::default(), ServeMode::Managed)
            .expect("serve_one");

        let response = String::from_utf8_lossy(&stream.output);
        assert!(response.contains("403 Forbidden"), "{response}");
        assert!(
            !response.contains("Access-Control-Allow-Origin"),
            "a rejected origin must not receive a CORS echo: {response}"
        );
    }
}

#[test]
fn cors_echoes_only_a_managed_allowed_origin() {
    let allow = vec!["vscode-webview://abc".to_string()];
    assert_eq!(
        cors_origin_for(
            &allow,
            Some("vscode-webview://abc"),
            Some("127.0.0.1:60615")
        )
        .as_deref(),
        Some("vscode-webview://abc")
    );
    assert_eq!(
        cors_origin_for(
            &allow,
            Some("http://127.0.0.1:60615"),
            Some("127.0.0.1:60615")
        )
        .as_deref(),
        Some("http://127.0.0.1:60615")
    );
    assert_eq!(
        cors_origin_for(
            &allow,
            Some("http://127.0.0.1:60616"),
            Some("127.0.0.1:60615")
        ),
        None
    );
    assert_eq!(
        cors_origin_for(&allow, Some("http://evil.local"), Some("evil.local")),
        None
    );
    assert_eq!(
        cors_origin_for(&["*".into()], Some("*"), Some("127.0.0.1:60615")),
        None
    );
    assert_eq!(cors_origin_for(&allow, None, None), None);
}

#[path = "web_canvas_server_sse_tests.rs"]
mod sse_tests;
