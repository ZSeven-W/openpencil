//! Export / connect-route / recent-file / selection tests for the
//! web-canvas daemon. Split out of `web_canvas_server_tests.rs` at the
//! 800-line cap; nested under that module so `use super::*` still reaches
//! its helpers and the daemon's own items.

use super::conn_tests::serve;
use super::*;

#[test]
fn normal_full_content_request_exports_complete_body_and_preserves_server_document() {
    use base64::Engine as _;
    let mut server = fresh_state();
    let before = server.editor.doc.clone();
    let document: serde_json::Value = serde_json::from_str(include_str!(
        "../../op-pen-loader/fixtures/normal-scroll.op"
    ))
    .unwrap();
    for (normal, height) in [(false, 812), (true, 1212)] {
        let body = serde_json::json!({"document":document,"format":"png","scale":1,
            "fullContent":normal,"selectedNodeId":"phone"})
        .to_string();
        let response = handle_web_canvas_request("POST", "/api/export/raster", &body, &mut server);
        assert!(response.status.starts_with("200"), "{}", response.body);
        let response: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        let png = base64::engine::general_purpose::STANDARD
            .decode(response["dataBase64"].as_str().unwrap())
            .unwrap();
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), height);
    }
    assert_eq!(server.editor.doc, before);
    assert_eq!(server.version, 0);
}

#[test]
fn post_export_pdf_returns_base64_pdf_without_replacing_daemon_document() {
    use base64::Engine as _;
    use op_editor_core::PenNodeExt;

    let mut s = fresh_state();
    let before_names: Vec<_> = s
        .editor
        .active_children()
        .iter()
        .filter_map(|n| n.base().name.clone())
        .collect();
    let export_body = r##"{"document":{"version":"1.0.0","children":[{"id":"pdf-node","type":"rectangle","name":"PDF Rect","x":1,"y":2,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]}}"##;

    let r = handle_web_canvas_request("POST", "/api/export/pdf", export_body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["mime"], "application/pdf");
    assert_eq!(parsed["fileName"], "openpencil-export.pdf");
    let data = parsed["dataBase64"].as_str().expect("dataBase64 string");
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("base64 pdf");
    assert!(pdf.starts_with(b"%PDF-"), "missing PDF header");
    assert!(
        pdf.windows(b"%%EOF".len()).any(|w| w == b"%%EOF"),
        "missing PDF EOF"
    );

    assert_eq!(s.version, 0, "export must not mutate sync version");
    let after_names: Vec<_> = s
        .editor
        .active_children()
        .iter()
        .filter_map(|n| n.base().name.clone())
        .collect();
    assert_eq!(after_names, before_names);
}

#[test]
fn post_export_pdf_rejects_invalid_document_without_mutating_state() {
    let mut s = fresh_state();

    let r = handle_web_canvas_request("POST", "/api/export/pdf", r#"{"document":1}"#, &mut s);

    assert!(r.status.starts_with("400"), "{}", r.body);
    assert!(r.body.contains("export PDF"), "{}", r.body);
    assert_eq!(s.version, 0);
}

#[test]
fn post_export_pdf_uses_request_active_page_index() {
    use base64::Engine as _;

    let mut s = fresh_state();
    let export_body = r##"{"activePageIndex":1,"document":{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"Empty","children":[]},{"id":"p2","name":"Exported","children":[{"id":"pdf-page-two","type":"rectangle","name":"PDF Page Two","x":1,"y":2,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]}]}}"##;

    let r = handle_web_canvas_request("POST", "/api/export/pdf", export_body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    let data = parsed["dataBase64"].as_str().expect("dataBase64 string");
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("base64 pdf");
    assert!(pdf.starts_with(b"%PDF-"), "missing PDF header");
    assert_eq!(s.version, 0, "export must not mutate sync version");
}

#[test]
fn post_export_pdf_gives_a_deck_one_page_per_board() {
    use base64::Engine as _;

    let mut s = fresh_state();
    // `editorMeta.scenario` is what makes this a deck — the two boards are
    // otherwise ordinary frames, and their differing sizes prove each page
    // is its own board rather than one shared sheet.
    let export_body = r##"{"document":{"version":"1.0.0","editorMeta":{"scenario":"slides"},"children":[
        {"id":"s1","type":"frame","name":"Cover","x":0,"y":0,"width":320,"height":180,"fill":[{"type":"solid","color":"#204080"}]},
        {"id":"s2","type":"frame","name":"Agenda","x":400,"y":0,"width":640,"height":360,"fill":[{"type":"solid","color":"#204080"}]}
    ]}}"##;

    let r = handle_web_canvas_request("POST", "/api/export/pdf", export_body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(parsed["dataBase64"].as_str().expect("dataBase64 string"))
        .expect("base64 pdf");
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.contains("/MediaBox [0 0 320 180]") && text.contains("/MediaBox [0 0 640 360]"),
        "expected one un-inset page per board, got MediaBoxes: {:?}",
        text.match_indices("/MediaBox")
            .map(|(at, _)| text[at..(at + 40).min(text.len())].to_string())
            .collect::<Vec<_>>()
    );
    assert_eq!(s.version, 0, "export must not mutate sync version");
}

/// The slides rail's "Export selected slides" row posts a `boards` list.
/// The browser's SELECTION cannot survive the document round-trip, so this
/// field is the only record of it that reaches the daemon — dropping it
/// would silently ship the whole deck.
#[test]
fn post_export_pdf_honours_a_boards_filter() {
    use base64::Engine as _;

    let mut s = fresh_state();
    let export_body = r##"{"boards":["s1","s3"],"document":{"version":"1.0.0","editorMeta":{"scenario":"slides"},"children":[
        {"id":"s1","type":"frame","name":"Cover","x":0,"y":0,"width":320,"height":180,"fill":[{"type":"solid","color":"#204080"}]},
        {"id":"s2","type":"frame","name":"Agenda","x":400,"y":0,"width":640,"height":360,"fill":[{"type":"solid","color":"#204080"}]},
        {"id":"s3","type":"frame","name":"End","x":1200,"y":0,"width":200,"height":100,"fill":[{"type":"solid","color":"#204080"}]}
    ]}}"##;

    let r = handle_web_canvas_request("POST", "/api/export/pdf", export_body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(parsed["dataBase64"].as_str().expect("dataBase64 string"))
        .expect("base64 pdf");
    let text = String::from_utf8_lossy(&pdf);
    assert!(
        text.contains("/MediaBox [0 0 320 180]") && text.contains("/MediaBox [0 0 200 100]"),
        "the two named boards must be the pages"
    );
    assert!(
        !text.contains("/MediaBox [0 0 640 360]"),
        "the unnamed board must NOT be a page"
    );
    assert_eq!(s.version, 0, "export must not mutate sync version");
}

/// An empty `boards` array means "the caller narrowed this to nothing",
/// which must fail rather than collapse onto the missing-field reading and
/// ship every slide.
#[test]
fn post_export_pdf_with_an_empty_boards_filter_exports_nothing() {
    let mut s = fresh_state();
    let export_body = r##"{"boards":[],"document":{"version":"1.0.0","editorMeta":{"scenario":"slides"},"children":[
        {"id":"s1","type":"frame","name":"Cover","x":0,"y":0,"width":320,"height":180,"fill":[{"type":"solid","color":"#204080"}]}
    ]}}"##;

    let r = handle_web_canvas_request("POST", "/api/export/pdf", export_body, &mut s);

    assert!(
        !r.status.starts_with("200"),
        "expected a failure, got {} {}",
        r.status,
        r.body
    );
    assert!(
        r.body.contains("nothing to export"),
        "expected the exporter's own reason, got {}",
        r.body
    );
}

#[test]
fn post_export_raster_returns_base64_png_without_replacing_daemon_document() {
    use base64::Engine as _;
    use op_editor_core::PenNodeExt;

    let mut s = fresh_state();
    let before_names: Vec<_> = s
        .editor
        .active_children()
        .iter()
        .filter_map(|n| n.base().name.clone())
        .collect();
    let export_body = r##"{"format":"png","scale":1,"document":{"version":"1.0.0","children":[{"id":"png-node","type":"rectangle","name":"PNG Rect","x":1,"y":2,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]}}"##;

    let r = handle_web_canvas_request("POST", "/api/export/raster", export_body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["mime"], "image/png");
    assert_eq!(parsed["fileName"], "openpencil-export.png");
    let data = parsed["dataBase64"].as_str().expect("dataBase64 string");
    let png = base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("base64 png");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "missing PNG header");

    assert_eq!(s.version, 0, "export must not mutate sync version");
    let after_names: Vec<_> = s
        .editor
        .active_children()
        .iter()
        .filter_map(|n| n.base().name.clone())
        .collect();
    assert_eq!(after_names, before_names);
}

#[test]
fn post_export_raster_crops_to_selected_node() {
    use base64::Engine as _;

    let mut s = fresh_state();
    let export_body = r##"{"format":"png","scale":1,"selectedNodeId":"small","document":{"version":"1.0.0","children":[{"id":"small","type":"rectangle","name":"Small","x":0,"y":0,"width":10,"height":10,"fill":[{"type":"solid","color":"#123456"}]},{"id":"far","type":"rectangle","name":"Far","x":300,"y":0,"width":50,"height":50,"fill":[{"type":"solid","color":"#654321"}]}]}}"##;

    let r = handle_web_canvas_request("POST", "/api/export/raster", export_body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    let data = parsed["dataBase64"].as_str().expect("dataBase64 string");
    let png = base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("base64 png");

    assert_eq!(png_dimensions(&png), (10, 10));
    assert_eq!(s.version, 0, "export must not mutate sync version");
}

#[test]
fn post_export_raster_uses_request_active_page_index() {
    use base64::Engine as _;

    let mut s = fresh_state();
    let export_body = r##"{"format":"png","scale":1,"activePageIndex":1,"selectedNodeId":"page-two","document":{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"Empty","children":[]},{"id":"p2","name":"Exported","children":[{"id":"page-two","type":"rectangle","name":"Page Two","x":0,"y":0,"width":10,"height":10,"fill":[{"type":"solid","color":"#123456"}]}]}]}}"##;

    let r = handle_web_canvas_request("POST", "/api/export/raster", export_body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    let data = parsed["dataBase64"].as_str().expect("dataBase64 string");
    let png = base64::engine::general_purpose::STANDARD
        .decode(data)
        .expect("base64 png");
    assert_eq!(png_dimensions(&png), (10, 10));
    assert_eq!(s.version, 0, "export must not mutate sync version");
}

fn png_dimensions(bytes: &[u8]) -> (u32, u32) {
    assert!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "missing PNG header"
    );
    let width = u32::from_be_bytes(bytes[16..20].try_into().expect("png width"));
    let height = u32::from_be_bytes(bytes[20..24].try_into().expect("png height"));
    (width, height)
}

#[test]
fn web_cli_and_acp_connect_routes_are_unavailable_in_both_dispatchers() {
    for path in ["/api/agents/connect", "/api/acp/connect"] {
        let direct = handle_web_canvas_request("POST", path, "{}", &mut fresh_state());
        assert_eq!(direct.status, "404 Not Found", "path={path}");

        let response = serve("POST", path, "{}");
        assert!(
            response.contains("404 Not Found"),
            "path={path}, {response}"
        );
    }
}

#[test]
fn retired_gemini_cli_connect_aliases_are_rejected_before_probe() {
    for provider in ["gemini", "gemini-cli"] {
        let mut state = fresh_state();
        let body = serde_json::json!({ "provider": provider }).to_string();

        let reply = handle_provider_connect_request_with_probe(&body, &mut state, |_| {
            panic!("retired Gemini CLI request must not reach the probe")
        });

        assert_eq!(reply.status, "400 Bad Request", "provider={provider}");
        assert!(state
            .editor
            .editor_ui
            .agent_settings
            .pending_provider_connect
            .is_none());
    }
}

#[test]
fn post_open_recent_loads_recent_path_and_bumps_version() {
    use op_editor_core::editor_ui_state::RecentFile;
    use op_editor_core::PenNodeExt;

    let path = write_temp_op(
        "recent-ok",
        r##"{"version":"1.0.0","children":[{"id":"recent-node","type":"rectangle","name":"Opened Recent","x":3,"y":4,"width":20,"height":10}]}"##,
    );
    let mut s = fresh_state();
    s.editor.editor_ui.recent_files = vec![RecentFile {
        path: path.to_string_lossy().into_owned(),
        modified_at: 1,
    }];

    let body = serde_json::json!({ "path": path.to_string_lossy() }).to_string();
    let r = handle_web_canvas_request("POST", "/api/file/open-recent", &body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    assert!(r.body.contains(r#""ok":true"#), "{}", r.body);
    assert_eq!(s.version, 1);
    assert!(s
        .editor
        .active_children()
        .iter()
        .any(|n| n.base().name.as_deref() == Some("Opened Recent")));
    assert_eq!(
        s.editor.editor_ui.recent_files[0].path,
        path.to_string_lossy()
    );
    assert_eq!(
        s.editor.editor_ui.file_name_display.as_deref(),
        Some(path.file_name().unwrap().to_str().unwrap())
    );
}

#[test]
fn post_open_recent_prunes_stale_recent_path_without_replacing_doc() {
    use op_editor_core::editor_ui_state::RecentFile;
    use op_editor_core::PenNodeExt;

    let missing = std::env::temp_dir().join(format!(
        "openpencil-web-canvas-missing-{}.op",
        std::process::id()
    ));
    let mut s = fresh_state();
    s.editor.editor_ui.recent_files = vec![RecentFile {
        path: missing.to_string_lossy().into_owned(),
        modified_at: 1,
    }];
    let before_names: Vec<_> = s
        .editor
        .active_children()
        .iter()
        .filter_map(|n| n.base().name.clone())
        .collect();

    let body = serde_json::json!({ "path": missing.to_string_lossy() }).to_string();
    let r = handle_web_canvas_request("POST", "/api/file/open-recent", &body, &mut s);

    assert!(r.status.starts_with("400"), "{}", r.body);
    assert!(r.body.contains(r#""pruned":true"#), "{}", r.body);
    assert_eq!(s.version, 0);
    assert!(s.editor.editor_ui.recent_files.is_empty());
    let after_names: Vec<_> = s
        .editor
        .active_children()
        .iter()
        .filter_map(|n| n.base().name.clone())
        .collect();
    assert_eq!(after_names, before_names);
}

#[test]
fn get_version_is_a_cheap_change_probe() {
    let mut s = fresh_state();
    let r = handle_web_canvas_request("GET", "/api/mcp/version", "", &mut s);
    assert!(r.status.starts_with("200"));
    // `collabSeq` rides along so one poll covers both document and
    // collaboration changes; `version` keeps its exact spelling and position.
    assert_eq!(r.body, r#"{"version":0,"collabSeq":0}"#);
    // A document mutation bumps the probed version.
    let _ = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut s);
    let r2 = handle_web_canvas_request("GET", "/api/mcp/version", "", &mut s);
    assert_eq!(r2.body, r#"{"version":1,"collabSeq":0}"#);
}

#[test]
fn selection_post_then_get_round_trips_ts_shape() {
    let mut s = fresh_state();
    // Initial GET: the TS `getSyncSelection()` empty shape.
    let r = handle_web_canvas_request("GET", "/api/mcp/selection", "", &mut s);
    assert!(r.status.starts_with("200"));
    let v: serde_json::Value = serde_json::from_str(&r.body).expect("json");
    assert_eq!(v["selectedIds"], serde_json::json!([]));
    assert_eq!(v["activePageId"], serde_json::Value::Null);
    // Renderer push (TS selection.post.ts body shape).
    let post = handle_web_canvas_request(
        "POST",
        "/api/mcp/selection",
        r#"{"selectedIds":["n1","n2"],"activePageId":null,"sourceClientId":"renderer:1"}"#,
        &mut s,
    );
    assert!(post.status.starts_with("200"), "{}", post.body);
    assert!(post.body.contains(r#""ok":true"#));
    // Selection is NOT a document mutation — version must not bump.
    assert_eq!(s.version, 0);
    // GET reflects the push; the live editor selection agrees.
    let r2 = handle_web_canvas_request("GET", "/api/mcp/selection", "", &mut s);
    let v2: serde_json::Value = serde_json::from_str(&r2.body).expect("json");
    assert_eq!(v2["selectedIds"], serde_json::json!(["n1", "n2"]));
    assert_eq!(s.editor.selection.set.len(), 2);
    assert_eq!(s.editor.selection.anchor.as_str(), "n2");
}

#[test]
fn selection_post_rejects_missing_ids_with_ts_error_text() {
    let mut s = fresh_state();
    for bad in [
        r#"{"activePageId":"p1"}"#,
        r#"{"selectedIds":"n1"}"#,
        "nope",
    ] {
        let r = handle_web_canvas_request("POST", "/api/mcp/selection", bad, &mut s);
        assert!(r.status.starts_with("400"), "{bad} → {}", r.status);
        assert!(r.body.contains("Missing selectedIds array"), "{}", r.body);
    }
}

#[test]
fn selection_post_switches_the_active_page_when_the_id_resolves() {
    let mut s = fresh_state();
    let paged = r##"{"document":{"version":"1.0.0","children":[],"pages":[
        {"id":"p1","name":"One","children":[]},
        {"id":"p2","name":"Two","children":[]}
    ]}}"##;
    let r = handle_web_canvas_request("POST", "/api/mcp/document", paged, &mut s);
    assert!(r.status.starts_with("200"), "{}", r.body);
    let post = handle_web_canvas_request(
        "POST",
        "/api/mcp/selection",
        r#"{"selectedIds":[],"activePageId":"p2"}"#,
        &mut s,
    );
    assert!(post.status.starts_with("200"));
    assert_eq!(s.editor.ui.active_page_index, 1);
    let get = handle_web_canvas_request("GET", "/api/mcp/selection", "", &mut s);
    assert!(get.body.contains(r#""activePageId":"p2""#), "{}", get.body);
    // An unknown page id is ignored (documented divergence from TS, which
    // stores the raw string): the active page stays put.
    let _ = handle_web_canvas_request(
        "POST",
        "/api/mcp/selection",
        r#"{"selectedIds":[],"activePageId":"ghost"}"#,
        &mut s,
    );
    assert_eq!(s.editor.ui.active_page_index, 1);
}

#[test]
fn selection_push_is_visible_to_the_mcp_get_selection_tool() {
    // The point of the selection sync: an external MCP client asking
    // `get_selection` over `/mcp` must see what the browser pushed.
    let mut s = fresh_state();
    let seeded = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut s);
    assert!(seeded.status.starts_with("200"), "{}", seeded.body);
    let post = handle_web_canvas_request(
        "POST",
        "/api/mcp/selection",
        r#"{"selectedIds":["n9"]}"#,
        &mut s,
    );
    assert!(post.status.starts_with("200"));
    // Dispatch get_selection through the same applier path serve_one uses.
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_selection","arguments":{}}}"#;
    let response =
        crate::mcp_serve::process_message_with_applier(&mut s.editor, msg, |_, editor, cmd| {
            editor.apply(cmd.clone())
        })
        .expect("dispatch")
        .unwrap_or_default();
    assert!(response.contains("n9"), "{response}");
}

// --- serve_one routing (socket-level, via a mock stream) ---
