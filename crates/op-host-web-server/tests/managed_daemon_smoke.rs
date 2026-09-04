//! End-to-end smoke for the managed serve-web contract.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Daemon {
    child: Child,
    port: u16,
    token: String,
}

const ALLOWED_ORIGIN: &str = "vscode-webview://smoke-test";

fn spawn_managed(file: Option<&str>) -> Daemon {
    let mut args = vec![
        "--serve-web",
        "--managed",
        "--port",
        "0",
        "--allow-origin",
        ALLOWED_ORIGIN,
    ];
    if let Some(f) = file {
        args.extend(["--file", f]);
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_op-host-web-server"))
        .args(&args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn op-host-web-server");
    let stdout = child.stdout.take().expect("stdout piped");
    let mut line = String::new();
    BufReader::new(stdout)
        .read_line(&mut line)
        .expect("handshake line");
    let v: serde_json::Value = serde_json::from_str(&line).expect("handshake json");
    Daemon {
        child,
        port: v["port"].as_u64().expect("port") as u16,
        token: v["token"].as_str().expect("token").to_string(),
    }
}

/// Minimal HTTP/1.1 exchange over a raw TcpStream (no client dep).
fn http(
    port: u16,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<&str>,
) -> (u16, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let body = body.unwrap_or("");
    let token_header = token
        .map(|t| format!("X-OpenPencil-Token: {t}\r\n"))
        .unwrap_or_default();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{token_header}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    let mut raw = String::new();
    s.read_to_string(&mut raw).expect("read");
    let status: u16 = raw
        .split_whitespace()
        .nth(1)
        .expect("status")
        .parse()
        .expect("status u16");
    let payload = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, payload)
}

fn wait_exit(child: &mut Child, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if child.try_wait().expect("try_wait").is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn read_version(port: u16, token: &str) -> u64 {
    let (code, body) = http(port, "GET", "/api/mcp/version", Some(token), None);
    assert_eq!(code, 200);
    let v: serde_json::Value = serde_json::from_str(&body).expect("version json");
    v["version"].as_u64().expect("version")
}

#[test]
fn managed_contract_end_to_end() {
    // --file load coverage: a canonical corpus fixture that MUST exist —
    // spawn/read failures are test failures, never skips.
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vendor/jian/crates/jian-ops-schema/tests/corpus/nested-frame.op"
    );
    let fixture_text = std::fs::read_to_string(fixture).expect("fixture exists");
    // Probe a stable field to prove the daemon loaded the file. The corpus
    // root frame has no "name", so probe its "id" instead.
    let fx: serde_json::Value = serde_json::from_str(&fixture_text).expect("fixture json");
    let probe_id = fx["children"][0]["id"].as_str().expect("fixture root id");
    let probe = format!(r#""id":"{probe_id}""#);
    let mut d = spawn_managed(Some(fixture));

    // 1. managed traffic is TOKENLESS since 2abc6b4ca: the supervisor already
    //    owns this process through the stdin lease (asserted in step 8), so
    //    ordinary requests carry no per-request token and a stale one is
    //    ignored rather than refused. The handshake token survives for the
    //    lifecycle request alone.
    //
    //    What REPLACED the token as the browser-facing defence is the Origin
    //    allowlist, so it is asserted here at the process level too — not
    //    only in op-host-services' `serve_one` unit tests. It is now the only
    //    thing between a web page and this daemon, which makes it exactly the
    //    kind of boundary a smoke test should cross for real.
    assert_eq!(http(d.port, "GET", "/api/mcp/version", None, None).0, 200);
    // 202 today (the JSON-RPC POST is accepted for dispatch); asserted as
    // "accepted" rather than one exact code, because the property under test
    // is that a stale token is IGNORED, not which success code dispatch picks.
    let stale = http(d.port, "POST", "/mcp", Some("wrong"), Some("{}")).0;
    assert!(
        matches!(stale, 200 | 202),
        "a stale token must be ignored, not rejected — got {stale}"
    );
    let (origin_code, _, _) = http_with_origin(
        d.port,
        "GET",
        "/api/mcp/version",
        None,
        Some("https://not-allowlisted.example"),
        None,
    );
    assert_eq!(
        origin_code, 403,
        "a browser Origin outside the allowlist must still be refused"
    );

    // 2. authenticated /mcp succeeds (not only rejection coverage)
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let (code, mcp_body) = http(d.port, "POST", "/mcp", Some(&d.token), Some(init));
    assert_eq!(code, 200);
    assert!(
        mcp_body.contains(r#""result""#),
        "mcp initialize must answer: {mcp_body}"
    );

    // 3. static shell tokenless: property under test is "not 401" —
    //    a bundle-less CI daemon may serve the help page, still not a 401.
    assert_ne!(http(d.port, "GET", "/", None, None).0, 401);

    // 4. --file content actually loaded
    let (code, doc_body) = http(d.port, "GET", "/api/mcp/document", Some(&d.token), None);
    assert_eq!(code, 200);
    assert!(
        doc_body.contains(&probe),
        "fixture content must be served: want {probe}"
    );

    // 5. sync-reset idempotence
    let (_, first) = http(
        d.port,
        "POST",
        "/api/mcp/sync-reset",
        Some(&d.token),
        Some("{}"),
    );
    assert!(
        !first.contains(r#""skipped":true"#),
        "first reset must run: {first}"
    );
    let (_, second) = http(
        d.port,
        "POST",
        "/api/mcp/sync-reset",
        Some(&d.token),
        Some("{}"),
    );
    assert!(
        second.contains(r#""skipped":true"#),
        "second reset must skip: {second}"
    );

    // 6. baseVersion conflict — version RE-READ after the reset bumped it
    let current = read_version(d.port, &d.token);
    let stale = format!(
        r#"{{"document":{{"version":"1.0.0","children":[]}},"sourceClientId":"smoke","baseVersion":{}}}"#,
        current + 999
    );
    let (code, cbody) = http(
        d.port,
        "POST",
        "/api/mcp/document",
        Some(&d.token),
        Some(&stale),
    );
    assert_eq!(code, 409, "stale baseVersion must conflict: {cbody}");
    assert!(cbody.contains("version-conflict"));

    // 7. matching baseVersion applies — re-read again (nothing wrote since,
    //    but the pattern must not depend on that)
    let current = read_version(d.port, &d.token);
    let fresh = format!(
        r#"{{"document":{{"version":"1.0.0","children":[]}},"sourceClientId":"smoke","baseVersion":{current}}}"#
    );
    assert_eq!(
        http(
            d.port,
            "POST",
            "/api/mcp/document",
            Some(&d.token),
            Some(&fresh)
        )
        .0,
        200
    );

    // 8. parent-death lease: dropping stdin must exit the daemon
    drop(d.child.stdin.take());
    assert!(
        wait_exit(&mut d.child, Duration::from_secs(5)),
        "daemon must exit on stdin EOF"
    );
}

/// Raw exchange that also returns the response head (for header assertions).
fn http_with_origin(
    port: u16,
    method: &str,
    path: &str,
    token: Option<&str>,
    origin: Option<&str>,
    body: Option<&str>,
) -> (u16, String, String) {
    let mut s = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    let body = body.unwrap_or("");
    let token_h = token
        .map(|t| format!("X-OpenPencil-Token: {t}\r\n"))
        .unwrap_or_default();
    let origin_h = origin
        .map(|o| format!("Origin: {o}\r\n"))
        .unwrap_or_default();
    let req = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n{token_h}{origin_h}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(req.as_bytes()).expect("write");
    let mut raw = String::new();
    s.read_to_string(&mut raw).expect("read");
    let status: u16 = raw
        .split_whitespace()
        .nth(1)
        .expect("status")
        .parse()
        .expect("status u16");
    let mut parts = raw.splitn(2, "\r\n\r\n");
    let head = parts.next().unwrap_or("").to_string();
    let payload = parts.next().unwrap_or("").to_string();
    (status, head, payload)
}

#[test]
fn cors_allowlist_and_preflight_contract() {
    let mut d = spawn_managed(None);

    // allowlisted Origin is echoed back on an authed API response
    let (code, head, _) = http_with_origin(
        d.port,
        "GET",
        "/api/mcp/version",
        Some(&d.token),
        Some(ALLOWED_ORIGIN),
        None,
    );
    assert_eq!(code, 200);
    assert!(
        head.contains(&format!("Access-Control-Allow-Origin: {ALLOWED_ORIGIN}")),
        "allowlisted origin must be echoed: {head}"
    );

    // non-allowlisted Origin gets NO allow-origin header (and no wildcard)
    let (_, head, _) = http_with_origin(
        d.port,
        "GET",
        "/api/mcp/version",
        Some(&d.token),
        Some("http://evil.local"),
        None,
    );
    assert!(
        !head.contains("Access-Control-Allow-Origin"),
        "must not echo evil origin: {head}"
    );

    // OPTIONS preflight is tokenless-exempt and advertises the token header
    let (code, head, _) = http_with_origin(
        d.port,
        "OPTIONS",
        "/api/mcp/document",
        None,
        Some(ALLOWED_ORIGIN),
        None,
    );
    assert_ne!(code, 401, "preflight must not require the token");
    assert!(
        head.to_ascii_lowercase().contains("x-openpencil-token"),
        "preflight must allow the token header: {head}"
    );

    drop(d.child.stdin.take());
    assert!(wait_exit(&mut d.child, Duration::from_secs(5)));
}
