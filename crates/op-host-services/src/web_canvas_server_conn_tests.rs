//! Connection-level tests for the web-canvas daemon — `serve_one` routing,
//! content-type + origin gates, JSON-RPC dispatch and the SSE stream. Split
//! out of `web_canvas_server_tests.rs` at the 800-line cap; nested under
//! that module so `use super::*` still reaches its helpers.

use super::*;

struct MockStream {
    input: std::io::Cursor<Vec<u8>>,
    output: Vec<u8>,
}

#[cfg(feature = "mcp-debug-tools")]
struct EnvVarGuard {
    key: &'static str,
    previous: Option<std::ffi::OsString>,
}

#[cfg(feature = "mcp-debug-tools")]
impl EnvVarGuard {
    fn set(key: &'static str, value: &str) -> Self {
        let previous = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, previous }
    }
}

#[cfg(feature = "mcp-debug-tools")]
impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match &self.previous {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

impl std::io::Read for MockStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.input.read(buf)
    }
}

impl std::io::Write for MockStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Drive one request through `serve_one` and return the raw HTTP response.
pub(super) fn serve(method: &str, path: &str, body: &str) -> String {
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.into_bytes()),
        output: Vec::new(),
    };
    let state = Mutex::new(fresh_state());
    let hub = SseHub::default();
    serve_one(&mut stream, &state, &hub).expect("serve_one");
    String::from_utf8_lossy(&stream.output).into_owned()
}

fn mock_stream(request: &str) -> MockStream {
    MockStream {
        input: std::io::Cursor::new(request.as_bytes().to_vec()),
        output: Vec::new(),
    }
}

/// One SSE payload for the hub tests.
fn tick(version: u64, collab_seq: u64) -> SseTick {
    SseTick {
        version,
        collab_seq,
    }
}

#[test]
fn serve_one_post_document_broadcasts_new_version_to_sse() {
    let state = Mutex::new(fresh_state());
    let hub = SseHub::default();
    let sub = hub.subscribe();
    let request = format!(
        "POST /api/mcp/document HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{}",
        SYNC_BODY.len(),
        SYNC_BODY
    );
    let mut stream = mock_stream(&request);
    serve_one(&mut stream, &state, &hub).expect("serve_one");
    // The whole-doc sync bumped the version to 1 and broadcast it.
    assert_eq!(sub.pending().expect("published").version, 1);
}

#[test]
fn serve_one_routes_rest_health_and_document() {
    assert!(serve("GET", "/api/mcp/server", "").contains("200 OK"));
    assert!(serve("GET", "/api/mcp/document", "").contains("200 OK"));
    let post = serve("POST", "/api/mcp/document", SYNC_BODY);
    assert!(post.contains("200 OK"), "{post}");
    assert!(post.contains(r#""ok":true"#));
}

/// Drive one request with an explicit Content-Type header through `serve_one`.
fn serve_with_content_type(method: &str, path: &str, content_type: &str, body: &str) -> String {
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: x\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = mock_stream(&request);
    let state = Mutex::new(fresh_state());
    let hub = SseHub::default();
    serve_one(&mut stream, &state, &hub).expect("serve_one");
    String::from_utf8_lossy(&stream.output).into_owned()
}

#[test]
fn serve_one_standard_ai_route_is_sse_not_404() {
    let r = serve_with_content_type("POST", "/api/ai/standard", "application/json", "not json");
    assert!(r.contains("text/event-stream"), "{r}");
    assert!(r.contains("invalid request body"), "{r}");
    assert!(!r.contains("404 Not Found"), "{r}");
}

#[test]
fn serve_one_builtin_model_discovery_route_is_json_not_404() {
    let response =
        serve_with_content_type("POST", "/api/ai/models/discover", "application/json", "{}");
    assert!(response.contains("400 Bad Request"), "{response}");
    assert!(
        response.contains("invalid model discovery request"),
        "{response}"
    );
    assert!(!response.contains("404 Not Found"), "{response}");
}

#[test]
fn managed_daemon_requires_its_token_before_model_discovery() {
    let body = r#"{"id":"builtin-1","generation":1,"credential":{}}"#;
    let request = format!(
        "POST /api/ai/models/discover HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body,
    );
    let mut stream = mock_stream(&request);
    let mut managed = fresh_state();
    managed.managed_token = Some("managed-secret".into());
    let state = Mutex::new(managed);

    serve_one(&mut stream, &state, &SseHub::default()).expect("serve_one");

    let response = String::from_utf8_lossy(&stream.output);
    assert!(response.contains("401 Unauthorized"), "{response}");
    assert!(
        !response.contains("invalid model discovery request"),
        "{response}"
    );
}

#[test]
fn serve_one_browser_json_routes_reject_simple_request_content_types() {
    // Cross-origin "simple requests" (text/plain, form-encoded, or no
    // Content-Type at all) never trigger a CORS preflight, so a drive-by
    // page could fire them at a local daemon. The JSON routes that carry
    // credentials or dial provider endpoints must refuse them outright.
    for (method, path) in [
        ("POST", "/api/ai/standard"),
        ("POST", "/api/ai/stream"),
        ("POST", "/api/ai/models/discover"),
        ("POST", "/api/settings/credentials"),
    ] {
        for content_type in ["text/plain", "application/x-www-form-urlencoded"] {
            let r = serve_with_content_type(method, path, content_type, "{}");
            assert!(
                r.contains("415 Unsupported Media Type"),
                "{method} {path} with {content_type} must be refused: {r}"
            );
        }
        let r = serve(method, path, "{}");
        assert!(
            r.contains("415 Unsupported Media Type"),
            "{method} {path} without a content type must be refused: {r}"
        );
    }
}

#[test]
fn serve_one_json_content_type_with_charset_parameter_is_accepted() {
    let r = serve_with_content_type(
        "POST",
        "/api/ai/standard",
        "application/json; charset=utf-8",
        "not json",
    );
    assert!(r.contains("invalid request body"), "{r}");
    assert!(!r.contains("415"), "{r}");
}

#[test]
fn serve_one_query_string_does_not_break_rest_routing() {
    // The query string must be stripped before exact-path routing.
    assert!(serve("GET", "/api/mcp/server?v=2", "").contains("200 OK"));
}

#[test]
fn serve_one_post_mcp_dispatches_jsonrpc() {
    let r = serve(
        "POST",
        "/mcp",
        r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#,
    );
    assert!(r.contains("200 OK"), "{r}");
}

#[cfg(feature = "mcp-debug-tools")]
#[test]
fn serve_one_post_mcp_debug_screenshot_uses_web_canvas_renderer() {
    let _debug_gate = EnvVarGuard::set("OPENPENCIL_DEBUG_TOOLS", "1");
    let state = Mutex::new(fresh_state());
    let hub = SseHub::default();
    {
        let mut guard = state.lock().expect("state lock");
        let seeded = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut guard);
        assert!(seeded.status.starts_with("200"), "{}", seeded.body);
    }

    let body = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"debug_screenshot","arguments":{"target":"root","dpr":1}}}"#;
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut stream = mock_stream(&request);
    serve_one(&mut stream, &state, &hub).expect("serve_one");
    let out = String::from_utf8_lossy(&stream.output);
    assert!(out.contains("200 OK"), "{out}");
    assert!(out.contains(r#""type":"image""#), "{out}");
    assert!(out.contains(r#""mimeType":"image/png""#), "{out}");
    assert!(
        !out.contains("No live canvas available"),
        "web daemon must serve screenshots from its live document, got {out}"
    );
}

#[test]
fn serve_one_get_mcp_is_405_not_a_tool_call() {
    let r = serve("GET", "/mcp", "");
    assert!(r.contains("405 Method Not Allowed"), "{r}");
}

#[test]
fn serve_one_unknown_path_is_404() {
    let r = serve("GET", "/favicon.ico", "");
    assert!(r.contains("404 Not Found"), "{r}");
}

#[test]
fn sync_reset_clears_web_document_and_bumps_version() {
    use op_editor_core::PenNodeExt;

    let mut s = fresh_state();
    let posted = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut s);
    assert!(posted.status.starts_with("200"), "{}", posted.body);
    assert_eq!(s.version, 1);
    assert!(
        s.editor
            .active_children()
            .iter()
            .any(|node| node.base().name.as_deref() == Some("Synced Rect")),
        "fixture document should be present before reset"
    );

    let reset = handle_web_canvas_request("POST", "/api/mcp/sync-reset", "", &mut s);
    assert!(reset.status.starts_with("200"), "{}", reset.body);
    assert!(reset.body.contains(r#""ok":true"#), "{}", reset.body);
    assert!(reset.body.contains(r#""version":2"#), "{}", reset.body);
    assert_eq!(s.version, 2);
    assert_eq!(s.editor.doc, EditorState::starter().doc);
    assert!(
        !s.editor
            .active_children()
            .iter()
            .any(|node| node.base().name.as_deref() == Some("Synced Rect")),
        "sync reset should remove the previous web document"
    );
}

#[test]
fn second_sync_reset_is_skipped_and_mutates_nothing() {
    let mut state = fresh_state();
    let first = state.reset_document_guarded().unwrap();
    assert!(!first.skipped);
    // capture full post-reset identity: a skipped reset must not move EITHER
    let v_before = state.document_version_for_test();
    let doc_before = serde_json::to_string(&state.editor.doc).unwrap();
    let second = state.reset_document_guarded().unwrap();
    assert!(second.skipped);
    assert_eq!(state.document_version_for_test(), v_before);
    assert_eq!(
        serde_json::to_string(&state.editor.doc).unwrap(),
        doc_before
    );
}

// A second valid body whose content DIFFERS from SYNC_BODY — rejected writes
// are asserted against it so "replace the doc but suppress the version bump"
// style bugs cannot pass.
const SYNC_BODY_ALT: &str = r##"{"document":{"version":"1.0.0","children":[{"id":"n77","type":"ellipse","name":"Rejected Ellipse","x":9,"y":9,"width":30,"height":30,"fill":[{"type":"solid","color":"#abcdef"}]}]},"sourceClientId":"web"}"##;

fn doc_fingerprint(state: &WebCanvasState) -> (u64, String) {
    (
        state.document_version_for_test(),
        serde_json::to_string(&state.editor.doc).unwrap(),
    )
}

#[test]
fn failed_reset_errors_and_mutates_nothing() {
    let mut state = fresh_state();
    state.current_path = Some(std::path::PathBuf::from("/nonexistent/x.op"));
    let before = doc_fingerprint(&state);
    // the reset MUST fail here — a silent success is itself a bug:
    assert!(state.reset_document_guarded().is_err());
    assert!(!state.reset_consumed); // retryable
    assert_eq!(doc_fingerprint(&state), before); // version AND bytes untouched
}

#[test]
fn document_post_with_stale_base_version_conflicts() {
    let mut state = fresh_state();
    let v0 = state.document_version_for_test();
    let ok = state.apply_document_push(SYNC_BODY, Some(v0)).unwrap();
    assert!(ok.applied);
    // capture BEFORE the stale attempt: a rejected write must change nothing
    let before = doc_fingerprint(&state);
    // stale write carries DIFFERENT bytes — proves the document didn't move
    let stale = state.apply_document_push(SYNC_BODY_ALT, Some(v0)).unwrap();
    assert!(!stale.applied);
    assert_eq!(doc_fingerprint(&state), before); // no bump, no content swap
    assert_eq!(stale.current_version, before.0);
}

#[test]
fn document_post_without_base_version_keeps_legacy_behavior() {
    let mut state = fresh_state();
    assert!(state.apply_document_push(SYNC_BODY, None).unwrap().applied);
    assert!(state.apply_document_push(SYNC_BODY, None).unwrap().applied);
}

#[test]
fn document_post_restores_embedded_authored_geometry_mode() {
    let mut state = fresh_state();
    let body = r#"{
      "document":{
        "version":"1.0.0",
        "children":[],
        "editorMeta":{
          "activePageIndex":0,
          "preserveAuthoredGeometry":true
        }
      }
    }"#;

    assert!(state.apply_document_push(body, None).unwrap().applied);
    assert!(state.editor.editor_ui.preserve_authored_geometry);
}

#[test]
fn document_post_restores_top_level_authored_geometry_mode() {
    let mut state = fresh_state();
    let body = r#"{
      "document":{"version":"1.0.0","children":[]},
      "preserveAuthoredGeometry":true
    }"#;

    assert!(state.apply_document_push(body, None).unwrap().applied);
    assert!(state.editor.editor_ui.preserve_authored_geometry);
}

#[test]
fn document_post_merges_top_level_editor_meta_fields_independently() {
    let nested = r#"{
      "document":{
        "version":"1.0.0",
        "children":[],
        "pages":[
          {"id":"p1","name":"One","children":[]},
          {"id":"p2","name":"Two","children":[]}
        ],
        "editorMeta":{"activePageIndex":1,"preserveAuthoredGeometry":true}
      },
      "preserveAuthoredGeometry":false
    }"#;
    let mut state = fresh_state();
    assert!(state.apply_document_push(nested, None).unwrap().applied);
    assert_eq!(state.editor.ui.active_page_index, 1);
    assert!(!state.editor.editor_ui.preserve_authored_geometry);

    let active_override = r#"{
      "document":{
        "version":"1.0.0",
        "children":[],
        "pages":[
          {"id":"p1","name":"One","children":[]},
          {"id":"p2","name":"Two","children":[]}
        ],
        "editorMeta":{"activePageIndex":1,"preserveAuthoredGeometry":true}
      },
      "activePageIndex":0
    }"#;
    assert!(
        state
            .apply_document_push(active_override, None)
            .unwrap()
            .applied
    );
    assert_eq!(state.editor.ui.active_page_index, 0);
    assert!(state.editor.editor_ui.preserve_authored_geometry);
}

#[test]
fn legacy_document_post_defaults_missing_editor_meta() {
    let mut state = fresh_state();
    state.editor.ui.active_page_index = 7;
    state.editor.editor_ui.preserve_authored_geometry = true;

    assert!(state.apply_document_push(SYNC_BODY, None).unwrap().applied);

    assert_eq!(state.editor.ui.active_page_index, 0);
    assert!(!state.editor.editor_ui.preserve_authored_geometry);
}

#[test]
fn metadata_only_post_updates_editor_meta_without_replacing_or_bumping_version() {
    let mut state = fresh_state();
    let two_pages = r#"{
      "document":{
        "version":"1.0.0",
        "children":[],
        "pages":[
          {"id":"p1","name":"One","children":[]},
          {"id":"p2","name":"Two","children":[]}
        ]
      }
    }"#;
    assert!(state.apply_document_push(two_pages, None).unwrap().applied);
    let before = doc_fingerprint(&state);
    let generation = state.editor.document_generation();
    let revision = state.editor.document_revision();

    let metadata_only = r##"{
      "document":{
        "version":"1.0.0",
        "children":[{"id":"ignored","type":"rectangle","x":0,"y":0,"width":10,"height":10}]
      },
      "activePageIndex":1,
      "preserveAuthoredGeometry":true,
      "metadataOnly":true
    }"##;
    let outcome = state
        .apply_document_push(metadata_only, None)
        .expect("metadata-only push");

    assert!(outcome.applied);
    assert_eq!(outcome.current_version, before.0);
    assert_eq!(doc_fingerprint(&state), before);
    assert_eq!(state.editor.document_generation(), generation);
    assert_eq!(state.editor.document_revision(), revision);
    assert_eq!(state.editor.ui.active_page_index, 1);
    assert!(state.editor.editor_ui.preserve_authored_geometry);
}

#[test]
fn malformed_document_push_stays_an_error() {
    let mut state = fresh_state();
    assert!(state.apply_document_push("{not json", None).is_err()); // still HTTP 400
}

#[test]
fn base_version_is_extracted_from_the_request_body() {
    let mut state = fresh_state();
    // No override: the stale baseVersion inside the body itself must conflict.
    let stale_in_body = SYNC_BODY.replacen(
        r#""sourceClientId":"web""#,
        r#""sourceClientId":"web","baseVersion":9999"#,
        1,
    );
    let out = state.apply_document_push(&stale_in_body, None).unwrap();
    assert!(!out.applied);
}

#[test]
fn sync_reset_route_reply_is_skipped_true_on_second_call() {
    let mut s = fresh_state();
    let first = handle_web_canvas_request("POST", "/api/mcp/sync-reset", "", &mut s);
    assert!(first.status.starts_with("200"), "{}", first.body);
    assert!(!first.body.contains(r#""skipped""#), "{}", first.body);

    let second = handle_web_canvas_request("POST", "/api/mcp/sync-reset", "", &mut s);
    assert!(second.status.starts_with("200"), "{}", second.body);
    assert!(second.body.contains(r#""skipped":true"#), "{}", second.body);
    assert!(second.body.contains(r#""version":1"#), "{}", second.body);
    assert_eq!(s.version, 1, "a skipped reset must not bump the version");
}

#[test]
fn document_post_route_409s_on_stale_base_version_without_mutating() {
    let mut s = fresh_state();
    let body_with_base_version = SYNC_BODY.replacen(
        r#""sourceClientId":"web""#,
        r#""sourceClientId":"web","baseVersion":9999"#,
        1,
    );
    let r = handle_web_canvas_request("POST", "/api/mcp/document", &body_with_base_version, &mut s);
    assert!(r.status.starts_with("409"), "{}", r.body);
    assert!(
        r.body.contains(r#""error":"version-conflict""#),
        "{}",
        r.body
    );
    assert!(r.body.contains(r#""version":0"#), "{}", r.body);
    assert_eq!(s.version, 0);
}

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

// --- serve_one layer: managed token auth + CORS allowlist enforcement ---

#[test]
fn managed_auth_gates_by_method_and_path() {
    let auth = RequestAuth {
        managed: true,
        token: "tok123".into(),
    };
    assert!(auth.allows("GET", "/", None)); // static shell
    assert!(auth.allows("GET", "/index.html", None));
    assert!(auth.allows("GET", "/pkg/op_host_web.js", None));
    assert!(auth.allows("GET", "/canvaskit/canvaskit.wasm", None)); // editor can't boot without
    assert!(auth.allows("GET", "/assets/iconify-catalog-brands.json", None));
    assert!(auth.allows("GET", "/smoke/step-1b.html", None));
    assert!(auth.allows("OPTIONS", "/api/mcp/document", None)); // preflight
    assert!(!auth.allows("POST", "/", None)); // JSON-RPC alias: privileged
    assert!(!auth.allows("GET", "/api/mcp/events", None)); // SSE: privileged
    assert!(!auth.allows("POST", "/mcp", Some("wrong")));
    assert!(auth.allows("POST", "/mcp", Some("tok123")));
}

#[test]
fn unmanaged_mode_keeps_open_behavior() {
    let auth = RequestAuth {
        managed: false,
        token: String::new(),
    };
    assert!(auth.allows("POST", "/api/mcp/document", None));
}

#[test]
fn cors_echoes_only_allowlisted_origin() {
    let allow = vec!["vscode-webview://abc".to_string()];
    assert_eq!(
        cors_origin_for(&allow, Some("vscode-webview://abc")).as_deref(),
        Some("vscode-webview://abc")
    );
    assert_eq!(cors_origin_for(&allow, Some("http://evil.local")), None);
    assert_eq!(cors_origin_for(&allow, None), None);
}

#[path = "web_canvas_server_sse_tests.rs"]
mod sse_tests;
