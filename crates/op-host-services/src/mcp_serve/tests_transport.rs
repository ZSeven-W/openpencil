//! File-path argument + HTTP-transport tests for the stdio/HTTP MCP server.
//! Split out of `mcp_serve/tests.rs` at the 800-line cap; nested under that
//! module so `use super::*` still reaches its helpers and `mcp_serve`'s own
//! items.

use super::*;

fn temp_doc_paths(test_name: &str) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(format!(
        "openpencil-mcp-filepath-{test_name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let primary = dir.join("primary.op");
    let alternate = dir.join("alternate.op");
    (dir, primary, alternate)
}

fn write_named_doc(path: &std::path::Path, node_id: &str, name: &str) {
    std::fs::write(
        path,
        format!(
            r##"{{
  "version": "1.0.0",
  "children": [
    {{
      "id": "{node_id}",
      "type": "rectangle",
      "name": "{name}",
      "x": 0,
      "y": 0,
      "width": 100,
      "height": 60,
      "fill": [{{ "type": "solid", "color": "#FFFFFF" }}]
    }}
  ]
}}"##
        ),
    )
    .expect("write doc");
}

#[test]
fn load_editor_state_accepts_ts_future_version_files() {
    let (dir, primary_path, _) = temp_doc_paths("future-version-load");
    std::fs::write(
        &primary_path,
        r##"{
  "version": "2.8",
  "children": [
    {
      "id": "future-node",
      "type": "rectangle",
      "name": "Future Version",
      "x": 0,
      "y": 0,
      "width": 100,
      "height": 60,
      "fill": [{ "type": "solid", "color": "#FFFFFF" }]
    }
  ]
}"##,
    )
    .expect("write future-version doc");

    let state = load_editor_state(&primary_path).expect("future TS .op should load");

    assert_eq!(state.active_children()[0].base().id, "future-node");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn file_backed_loader_restores_embedded_authored_geometry_mode() {
    let (dir, primary_path, _) = temp_doc_paths("preserve-geometry-load");
    std::fs::write(
        &primary_path,
        r#"{
          "version":"1.0.0",
          "children":[],
          "editorMeta":{"preserveAuthoredGeometry":true}
        }"#,
    )
    .expect("write preserve metadata doc");

    let state = load_editor_state(&primary_path).expect("file-backed state loads");

    assert!(state.editor_ui.preserve_authored_geometry);
    let _ = std::fs::remove_dir_all(dir);
}

fn assert_response_file_path_matches(response: &str, expected: &std::path::Path) {
    let tool_text = crate::mcp_serve::tool_text(response);
    let result: serde_json::Value = serde_json::from_str(&tool_text).expect("tool result JSON");
    let actual = result["filePath"]
        .as_str()
        .expect("open_document response filePath");
    let actual = std::path::PathBuf::from(actual)
        .canonicalize()
        .expect("canonicalize actual response filePath");
    let expected = expected
        .canonicalize()
        .expect("canonicalize expected response filePath");
    assert_eq!(actual, expected, "{response}");
}

#[test]
fn process_message_reads_document_from_ts_file_path_arg() {
    let (dir, primary_path, alternate_path) = temp_doc_paths("read");
    write_named_doc(&primary_path, "n1", "Primary");
    write_named_doc(&alternate_path, "n2", "Alternate");
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let file_path_json =
        serde_json::to_string(&alternate_path.to_string_lossy()).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{{"name":"batch_get","arguments":{{"filePath":{file_path_json},"readDepth":1}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert!(response.contains("Alternate"), "{response}");
    assert!(!response.contains("Primary"), "{response}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn process_message_open_document_reports_ts_file_path_target() {
    let (dir, primary_path, alternate_path) = temp_doc_paths("open");
    write_named_doc(&primary_path, "n1", "Primary");
    write_named_doc(&alternate_path, "n2", "Alternate");
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let alternate = alternate_path.to_string_lossy().to_string();
    let file_path_json = serde_json::to_string(&alternate).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{{"name":"open_document","arguments":{{"filePath":{file_path_json}}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert_response_file_path_matches(&response, &alternate_path);
    assert!(!response.contains("warning"), "{response}");
    assert!(!response.contains("does not reopen files"), "{response}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn process_message_open_document_creates_missing_ts_file_path_target() {
    let (dir, primary_path, alternate_path) = temp_doc_paths("open-create");
    write_named_doc(&primary_path, "n1", "Primary");
    assert!(!alternate_path.exists());
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let alternate = alternate_path.to_string_lossy().to_string();
    let file_path_json = serde_json::to_string(&alternate).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":24,"method":"tools/call","params":{{"name":"open_document","arguments":{{"filePath":{file_path_json}}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert!(
        alternate_path.exists(),
        "open_document should create the target .op file"
    );
    assert_response_file_path_matches(&response, &alternate_path);
    let created = std::fs::read_to_string(&alternate_path).expect("created document");
    assert!(created.contains(r#""version""#), "{created}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn process_message_writes_document_to_ts_file_path_arg() {
    let (dir, primary_path, alternate_path) = temp_doc_paths("write");
    write_named_doc(&primary_path, "n1", "Primary");
    write_named_doc(&alternate_path, "n2", "Alternate");
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let file_path_json =
        serde_json::to_string(&alternate_path.to_string_lossy()).expect("path json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{{"name":"add_page","arguments":{{"filePath":{file_path_json},"name":"FromFilePath"}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert!(
        crate::mcp_serve::tool_text(&response).contains(r#""wrote":"true""#),
        "{response}"
    );
    let primary_text = std::fs::read_to_string(&primary_path).expect("primary doc");
    let alternate_text = std::fs::read_to_string(&alternate_path).expect("alternate doc");
    assert!(!primary_text.contains("FromFilePath"), "{primary_text}");
    assert!(alternate_text.contains("FromFilePath"), "{alternate_text}");
    assert!(
        alternate_text.contains("editorMeta"),
        "MCP writes use the canonical desktop serializer"
    );
    let reloaded = crate::doc_io::load_editor_state(&alternate_path, op_editor_core::Locale::EnUs)
        .expect("MCP output reloads through the product loader");
    assert!(reloaded
        .doc
        .pages
        .as_ref()
        .is_some_and(|pages| pages.iter().any(|page| page.name == "FromFilePath")));
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn process_message_save_document_can_copy_ts_file_path_source_to_target() {
    let (dir, primary_path, source_path) = temp_doc_paths("save-source");
    let target_path = dir.join("saved-target.op");
    write_named_doc(&primary_path, "n1", "Primary");
    write_named_doc(&source_path, "n2", "Source");
    let mut state = load_editor_state(&primary_path).expect("primary state");
    let target_json = serde_json::to_string(&target_path.to_string_lossy()).expect("target json");
    let source_json = serde_json::to_string(&source_path.to_string_lossy()).expect("source json");
    let line = format!(
        r#"{{"jsonrpc":"2.0","id":25,"method":"tools/call","params":{{"name":"save_document","arguments":{{"filePath":{target_json},"sourceFilePath":{source_json}}}}}}}"#
    );

    let response = process_message(&mut state, &primary_path, &line)
        .expect("dispatch")
        .expect("response");

    assert!(
        crate::mcp_serve::tool_text(&response).contains(r#""ok":"true""#),
        "{response}"
    );
    let target_text = std::fs::read_to_string(&target_path).expect("target doc");
    assert!(target_text.contains("Source"), "{target_text}");
    assert!(!target_text.contains("Primary"), "{target_text}");
    let _ = std::fs::remove_dir_all(dir);
}

/// In-memory `Read + Write` stand-in for a `TcpStream` so the HTTP
/// transport can be exercised without a real socket.
struct MockStream {
    input: std::io::Cursor<Vec<u8>>,
    output: Vec<u8>,
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

#[test]
fn http_request_body_reads_exactly_content_length() {
    // Trailing bytes past Content-Length must NOT leak into the body.
    let body = r#"{"method":"ping"}"#;
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}TRAILING-IGNORED",
        body.len()
    );
    let mut cur = std::io::Cursor::new(request.into_bytes());
    assert_eq!(read_http_request_body(&mut cur).unwrap(), body);
}

#[test]
fn http_request_accepts_bodies_larger_than_the_old_8_mib_cap() {
    // A realistic whole-document live sync (`/api/mcp/document`) carrying
    // embedded base64 images runs to tens of MiB; the old 8 MiB cap rejected
    // such documents. A ~9 MiB body must now be accepted.
    let body = "x".repeat(9 * 1024 * 1024);
    let request = format!(
        "POST /api/mcp/document HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let mut cur = std::io::Cursor::new(request.into_bytes());
    let req = read_http_request(&mut cur).expect("9 MiB body must be accepted");
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/api/mcp/document");
    assert_eq!(req.body.len(), 9 * 1024 * 1024);
}

#[test]
fn document_sync_route_matches_only_post_to_the_ts_rest_path() {
    assert!(is_document_sync_route("POST", "/api/mcp/document"));
    assert!(!is_document_sync_route("GET", "/api/mcp/document"));
    assert!(!is_document_sync_route("POST", "/mcp"));
    assert!(!is_document_sync_route("POST", "/api/mcp/selection"));
}

#[test]
fn parse_document_sync_body_mirrors_ts_validation() {
    // Missing `document` key.
    assert_eq!(
        parse_document_sync_body(r#"{"sourceClientId":"x"}"#)
            .unwrap_err()
            .to_string(),
        "Missing document in request body"
    );
    // Non-object document / missing version / no children|pages array.
    assert_eq!(
        parse_document_sync_body(r#"{"document":42}"#)
            .unwrap_err()
            .to_string(),
        "Invalid document format"
    );
    assert_eq!(
        parse_document_sync_body(r#"{"document":{"children":[]}}"#)
            .unwrap_err()
            .to_string(),
        "Invalid document format"
    );
    assert_eq!(
        parse_document_sync_body(r#"{"document":{"version":"1.0"}}"#)
            .unwrap_err()
            .to_string(),
        "Invalid document format"
    );
    // Malformed JSON.
    assert_eq!(
        parse_document_sync_body("not json")
            .unwrap_err()
            .to_string(),
        "Invalid document format"
    );
    // Valid: version + children array.
    let inner = parse_document_sync_body(r#"{"document":{"version":"1.0","children":[]}}"#)
        .expect("children form valid");
    assert!(inner.contains(r#""version":"1.0""#));
    assert!(inner.contains(r#""children":[]"#));
    // Valid: version + pages array.
    assert!(parse_document_sync_body(
        r#"{"document":{"version":"1.0","pages":[{"id":"p1","name":"P","children":[]}]}}"#
    )
    .is_ok());

    let request = parse_document_sync_request(
        r#"{"document":{"version":"1.0","children":[]},"baseVersion":7,"activePageIndex":3,"preserveAuthoredGeometry":true,"metadataOnly":true}"#,
    )
    .expect("metadata-aware request");
    assert_eq!(request.base_version, Some(7));
    assert_eq!(request.active_page_index, Some(3));
    assert_eq!(request.preserve_authored_geometry, Some(true));
    assert!(request.metadata_only);
}

#[test]
fn document_sync_document_json_borrows_the_request_slice() {
    let body = r#"{ "document" : { "version": "1.0", "children": [ ] }, "baseVersion": 9 }"#;
    let expected = r#"{ "version": "1.0", "children": [ ] }"#;

    let request = parse_document_sync_request(body).expect("borrowed document request");

    assert_eq!(request.document_json, expected);
    assert_eq!(request.base_version, Some(9));
    let body_range = body.as_ptr() as usize..body.as_ptr() as usize + body.len();
    assert!(body_range.contains(&(request.document_json.as_ptr() as usize)));
}

#[test]
fn document_sync_metadata_overrides_nested_fields_independently() {
    let embedded = op_pen_loader::EditorMeta {
        active_page_index: 5,
        preserve_authored_geometry: true,
        scenario: None,
        pinned_style_guide: None,
    };

    let page_override = parse_document_sync_request(
        r#"{"document":{"version":"1.0","children":[]},"activePageIndex":2}"#,
    )
    .expect("active-page override");
    assert_eq!(
        page_override.resolved_editor_meta(Some(embedded.clone())),
        op_pen_loader::EditorMeta {
            active_page_index: 2,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
        }
    );

    let geometry_override = parse_document_sync_request(
        r#"{"document":{"version":"1.0","children":[]},"preserveAuthoredGeometry":false}"#,
    )
    .expect("geometry override");
    assert_eq!(
        geometry_override.resolved_editor_meta(Some(embedded.clone())),
        op_pen_loader::EditorMeta {
            active_page_index: 5,
            preserve_authored_geometry: false,
            scenario: None,
            pinned_style_guide: None,
        }
    );

    let legacy = parse_document_sync_request(r#"{"document":{"version":"1.0","children":[]}}"#)
        .expect("legacy wrapper");
    assert_eq!(
        legacy.resolved_editor_meta(Some(embedded.clone())),
        embedded
    );
    assert_eq!(legacy.resolved_editor_meta(None), Default::default());

    let nested = parse_document_sync_request(
        r#"{"document":{"version":"1.0","children":[],"editorMeta":{"activePageIndex":5,"preserveAuthoredGeometry":true}},"activePageIndex":2}"#,
    )
    .expect("nested metadata request");
    assert_eq!(nested.embedded_editor_meta, Some(embedded));
    assert_eq!(
        nested.resolved_editor_meta(nested.embedded_editor_meta.clone()),
        op_pen_loader::EditorMeta {
            active_page_index: 2,
            preserve_authored_geometry: true,
            scenario: None,
            pinned_style_guide: None,
        }
    );
}

#[test]
fn document_sync_ok_and_error_bodies_match_ts_shapes() {
    assert_eq!(document_sync_ok(7), r#"{"ok":true,"version":7}"#);
    let err = rest_error_body("Invalid document format");
    assert!(err.contains(r#""ok":false"#), "{err}");
    assert!(
        err.contains(r#""error":"Invalid document format""#),
        "{err}"
    );
}

#[test]
fn read_http_request_strips_query_string_from_path() {
    // A query string must not defeat exact-path routing (`/api/mcp/document`).
    let request =
        "GET /api/mcp/document?clientId=abc&v=2 HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n";
    let mut cur = std::io::Cursor::new(request.as_bytes().to_vec());
    let req = read_http_request(&mut cur).expect("request parses");
    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/api/mcp/document");
}

#[test]
fn read_http_request_does_not_panic_on_multibyte_header() {
    // A header whose bytes put a multibyte UTF-8 boundary near offset 15 must
    // not panic the Content-Length scan (the old `l[..15]` byte-slice would).
    let request = "POST /mcp HTTP/1.1\r\nX-Ünïcödé-Header: yes\r\nContent-Length: 0\r\n\r\n";
    let mut cur = std::io::Cursor::new(request.as_bytes().to_vec());
    let req = read_http_request(&mut cur).expect("multibyte header must not panic");
    assert_eq!(req.method, "POST");
    assert_eq!(req.body, "");
}

#[test]
fn http_request_still_rejects_an_over_cap_content_length() {
    // 300 MiB declared (> 256 MiB cap). The cap is checked from the header
    // BEFORE any body buffer is allocated, so this is cheap and must reject.
    let request = "POST /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: 314572800\r\n\r\n";
    let mut cur = std::io::Cursor::new(request.as_bytes().to_vec());
    let err = read_http_request(&mut cur).expect_err("over-cap body must be rejected");
    // Framing rejected before any handler ran — a client fault, not a socket
    // failure, so it must classify as `Protocol`.
    assert!(matches!(err, McpServeError::Protocol(_)), "{err:?}");
    assert!(err.to_string().contains("exceeds"), "{err}");
}

#[test]
fn credential_settings_request_rejects_over_256_kib_before_reading_body() {
    let request = concat!(
        "POST /api/settings/credentials HTTP/1.1\r\n",
        "Host: x\r\n",
        "Content-Length: 262145\r\n",
        "\r\n"
    );
    let mut cur = std::io::Cursor::new(request.as_bytes().to_vec());

    let err = read_http_request(&mut cur).expect_err("oversized credential body must be rejected");

    assert!(matches!(err, McpServeError::Protocol(_)), "{err:?}");
    assert!(
        err.to_string()
            .contains("credential settings body exceeds 256 KiB"),
        "{err}"
    );
}

#[test]
fn model_discovery_request_rejects_over_256_kib_before_reading_body() {
    let request = concat!(
        "POST /api/ai/models/discover HTTP/1.1\r\n",
        "Host: x\r\n",
        "Content-Length: 262145\r\n",
        "\r\n"
    );
    let mut cur = std::io::Cursor::new(request.as_bytes().to_vec());

    let err = read_http_request(&mut cur).expect_err("oversized discovery body must be rejected");

    assert!(matches!(err, McpServeError::Protocol(_)), "{err:?}");
    assert!(
        err.to_string()
            .contains("model discovery body exceeds 256 KiB"),
        "{err}"
    );
}

#[test]
fn design_md_evidence_requires_one_decimal_content_length() {
    for (headers, expected_status) in [
        ("Host: x\r\n", "411 Length Required"),
        ("Host: x\r\nContent-Length: nope\r\n", "400 Bad Request"),
        (
            "Host: x\r\nContent-Length: 2\r\nContent-Length: 2\r\n",
            "400 Bad Request",
        ),
        (
            "Host: x\r\nContent-Length: 999999999999999999999999999999999999\r\n",
            "400 Bad Request",
        ),
    ] {
        let request = format!("POST /api/generate/design-md HTTP/1.1\r\n{headers}\r\n{{}}");
        let mut cur = std::io::Cursor::new(request.into_bytes());
        let err = read_http_request(&mut cur).expect_err("invalid framing must reject");
        assert!(
            matches!(
                err,
                McpServeError::Framing { status, .. } if status == expected_status
            ),
            "{err:?}"
        );
    }
}

#[test]
fn design_md_evidence_rejects_over_256_kib_before_reading_body() {
    let request = concat!(
        "POST /api/generate/design-md HTTP/1.1\r\n",
        "Host: x\r\n",
        "Content-Length: 262145\r\n",
        "\r\n"
    );
    let mut cur = std::io::Cursor::new(request.as_bytes().to_vec());
    let err = read_http_request(&mut cur).expect_err("oversized evidence must reject");
    assert!(
        matches!(
            err,
            McpServeError::Framing {
                status: "413 Payload Too Large",
                ..
            }
        ),
        "{err:?}"
    );
    assert!(err.to_string().contains("256 KiB"));
}

#[test]
fn non_post_design_md_body_is_capped_before_it_is_read() {
    let job_path = "/api/generate/design-md/0123456789abcdef0123456789abcdef";
    for path in ["/api/generate/design-md", job_path] {
        for (length, expected_status) in [
            (262_145, "413 Payload Too Large"),
            (262_144, "400 Bad Request"),
        ] {
            let request =
                format!("DELETE {path} HTTP/1.1\r\nHost: x\r\nContent-Length: {length}\r\n\r\n");
            let mut cur = std::io::Cursor::new(request.into_bytes());
            let err = read_http_request(&mut cur).expect_err("body must reject before read");
            assert!(
                matches!(
                    err,
                    McpServeError::Framing { status, .. } if status == expected_status
                ),
                "{err:?}"
            );
        }
    }
}

#[test]
fn design_md_evidence_rejects_invalid_utf8_instead_of_replacing_it() {
    let head = b"POST /api/generate/design-md HTTP/1.1\r\nHost: x\r\nContent-Length: 1\r\n\r\n";
    let mut request = head.to_vec();
    request.push(0xff);
    let mut cur = std::io::Cursor::new(request);
    let err = read_http_request(&mut cur).expect_err("invalid UTF-8 must reject");
    assert!(
        matches!(
            err,
            McpServeError::Framing {
                status: "400 Bad Request",
                ..
            }
        ),
        "{err:?}"
    );
    assert!(err.to_string().contains("valid UTF-8"));
}

#[test]
fn design_md_evidence_rejects_ambiguous_content_type_headers() {
    let request = concat!(
        "POST /api/generate/design-md HTTP/1.1\r\n",
        "Host: x\r\n",
        "Content-Type: application/json\r\n",
        "Content-Type: text/plain\r\n",
        "Content-Length: 2\r\n",
        "\r\n{}"
    );
    let mut cur = std::io::Cursor::new(request.as_bytes().to_vec());
    let err = read_http_request(&mut cur).expect_err("duplicate Content-Type must reject");
    assert!(
        matches!(
            err,
            McpServeError::Framing {
                status: "400 Bad Request",
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn http_transport_serves_initialize() {
    let rpc = r#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{}}"#;
    let request = format!(
        "POST / HTTP/1.1\r\nHost: x\r\nContent-Length: {}\r\n\r\n{rpc}",
        rpc.len()
    );
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.into_bytes()),
        output: Vec::new(),
    };
    let mut state = EditorState::new();
    serve_http_connection(
        &mut stream,
        &mut state,
        std::path::Path::new("/tmp/unused.op"),
    )
    .expect("serve_http_connection");
    let resp = String::from_utf8(stream.output).unwrap();
    assert!(resp.starts_with("HTTP/1.1 200 OK"), "status line: {resp}");
    assert!(resp.contains("Content-Type: application/json"));
    assert!(resp.contains("mcp-session-id: openpencil"));
    assert!(resp.contains("Access-Control-Allow-Origin: *"));
    assert!(resp.contains("Cache-Control: no-store"));
    // The JSON-RPC initialize reply carries the protocol handshake +
    // the request id, proving the body round-tripped over HTTP.
    assert!(resp.contains(r#""protocolVersion""#), "body: {resp}");
    assert!(resp.contains(r#""id":7"#), "body: {resp}");
}

#[test]
fn http_transport_serves_options_preflight() {
    let request = "OPTIONS /mcp HTTP/1.1\r\nHost: x\r\nContent-Length: 0\r\n\r\n";
    let mut stream = MockStream {
        input: std::io::Cursor::new(request.as_bytes().to_vec()),
        output: Vec::new(),
    };
    let mut state = EditorState::new();
    serve_http_connection(
        &mut stream,
        &mut state,
        std::path::Path::new("/tmp/unused.op"),
    )
    .expect("serve_http_connection");
    let resp = String::from_utf8(stream.output).unwrap();
    assert!(resp.starts_with("HTTP/1.1 204 No Content"), "{resp}");
    assert!(resp.contains("Access-Control-Allow-Methods"));
}
