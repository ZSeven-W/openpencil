//! Tests for the web-canvas daemon (`web_canvas_server.rs`) — sibling
//! file per the 800-line-cap convention (like `chat_session_tests.rs`).

use super::*;

fn fresh_state() -> WebCanvasState {
    WebCanvasState::new(EditorState::new(), 3100)
}

fn fresh_server_persistence_state() -> WebCanvasState {
    WebCanvasState::new_with_policy(
        EditorState::new(),
        3100,
        crate::web_credential_policy::WebCredentialPersistence::Server,
    )
}

// A minimal canonical document body in the TS `setSyncDocument` shape.
const SYNC_BODY: &str = r##"{"document":{"version":"1.0.0","children":[{"id":"n9","type":"rectangle","name":"Synced Rect","x":1,"y":2,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]},"sourceClientId":"web"}"##;

const CREDENTIAL_BODY: &str = r#"{
  "version":2,
  "builtin_agents":[{
    "id":"builtin-web-1","preset":"custom","display_name":"Private Model",
    "kind":"openai-compat","api_key":"sk-browser-only","model":"private-model",
    "base_url":"https://api.openai.com/v1","enabled":true
  }],
  "image_gen_profiles":[{
    "id":"igp-web-1","name":"Image","provider":"openai",
    "api_key":"image-browser-only","model":"gpt-image-1","base_url":null
  }],
  "active_image_gen_profile_id":"igp-web-1",
  "openverse_oauth":{"client_id":"client","client_secret":"openverse-browser-only"}
}"#;

fn write_temp_op(name: &str, body: &str) -> PathBuf {
    let path = std::env::temp_dir().join(temp_op_file_name(
        name,
        std::thread::current().name().unwrap_or("test"),
    ));
    std::fs::write(&path, body).expect("write temp op");
    path
}

fn temp_op_file_name(name: &str, thread_name: &str) -> String {
    format!(
        "openpencil-web-canvas-{}-{}-{}.op",
        sanitize_temp_file_component(name),
        std::process::id(),
        sanitize_temp_file_component(thread_name)
    )
}

fn sanitize_temp_file_component(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' => ch,
            _ => '-',
        })
        .collect();
    let trimmed = sanitized.trim_matches(['-', '.']);
    if trimmed.is_empty() {
        "test".to_string()
    } else {
        trimmed.to_string()
    }
}

#[test]
fn temp_op_file_name_sanitizes_windows_reserved_path_chars() {
    let file_name = temp_op_file_name(r#"recent:ok"#, r#"web_canvas_server::tests\case?*"#);

    for reserved in ['<', '>', ':', '"', '/', '\\', '|', '?', '*'] {
        assert!(
            !file_name.contains(reserved),
            "{file_name} contains reserved path character {reserved:?}"
        );
    }
    assert!(file_name.ends_with(".op"));
}

#[test]
fn server_health_matches_ts_running_port_shape() {
    let r = handle_web_canvas_request("GET", "/api/mcp/server", "", &mut fresh_state());
    assert!(r.status.starts_with("200"));
    // TS `server.get.ts` parity: clients test `running` + `port`.
    assert!(r.body.contains(r#""running":true"#));
    assert!(r.body.contains(r#""port":3100"#));
    assert!(r.body.contains(r#""localIp":"#));
    assert!(r.body.contains(r#""server":"openpencil-mcp""#));
}

#[test]
fn credential_policy_endpoint_reports_only_the_boolean() {
    let mut state = WebCanvasState::new_with_policy(
        EditorState::new(),
        3100,
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
    );
    let reply = handle_web_canvas_request("GET", "/api/settings/credential-policy", "", &mut state);

    assert_eq!(reply.status, "200 OK");
    assert_eq!(reply.body, r#"{"serverPersistence":false}"#);
    assert!(!reply.body.contains("api_key"));
    assert!(!reply.body.contains("secret"));
}

#[test]
fn browser_only_route_rejects_credentials_without_mutating_state() {
    let mut state = fresh_state();
    let before = crate::settings_io::fingerprint(&state.editor);

    let reply = handle_web_canvas_request(
        "POST",
        "/api/settings/credentials",
        CREDENTIAL_BODY,
        &mut state,
    );

    assert_eq!(reply.status, "403 Forbidden");
    assert_eq!(before, crate::settings_io::fingerprint(&state.editor));
    assert!(!reply.body.contains("sk-browser-only"));
}

#[test]
fn server_policy_merges_credentials_without_echoing_them() {
    let mut state = fresh_server_persistence_state();

    let reply = handle_web_canvas_request(
        "POST",
        "/api/settings/credentials",
        CREDENTIAL_BODY,
        &mut state,
    );

    assert_eq!(reply.status, "200 OK");
    assert_eq!(reply.body, r#"{"ok":true}"#);
    let agent = state
        .editor
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .find(|agent| agent.id.ends_with(":builtin:builtin-web-1"))
        .expect("merged browser agent");
    assert_eq!(agent.api_key, "sk-browser-only");
    assert!(!reply.body.contains("browser-only"));
}

#[test]
fn browser_only_restart_does_not_load_previously_persisted_browser_credentials() {
    let mut persisted = EditorState::new();
    crate::web_credentials::apply_json(&mut persisted, CREDENTIAL_BODY)
        .expect("private deployment snapshot");
    persisted.editor_ui.agent_settings.add_builtin_agent_config(
        "Operator",
        "operator-key",
        "operator-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://operator.example/v1",
    );

    let state = WebCanvasState::new_with_policy(
        persisted,
        3100,
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
    );
    let settings = &state.editor.editor_ui.agent_settings;

    assert!(settings
        .builtin_agents
        .iter()
        .all(|agent| !agent.id.starts_with("web-credential:")));
    assert!(settings
        .builtin_agents
        .iter()
        .any(|agent| agent.has_model("operator-model")));
    assert!(settings.image_gen_profiles.is_empty());
    assert!(settings.openverse_client_secret.is_empty());
    assert_eq!(settings.openverse_credential_owner, None);
}

#[test]
fn browser_only_startup_removes_browser_credentials_and_saves_once() {
    let mut editor = EditorState::new();
    crate::web_credentials::apply_json(&mut editor, CREDENTIAL_BODY)
        .expect("private deployment snapshot");
    editor.editor_ui.agent_settings.add_builtin_agent_config(
        "Operator",
        "operator-key",
        "operator-model",
        op_editor_core::BuiltinAgentKind::OpenAiCompat,
        "https://operator.example/v1",
    );
    let mut saves = 0;

    enforce_credential_persistence_policy(
        &mut editor,
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
        |saved| {
            saves += 1;
            let settings = &saved.editor_ui.agent_settings;
            assert!(settings
                .builtin_agents
                .iter()
                .all(|agent| !agent.id.starts_with("web-credential:")));
            assert!(settings
                .builtin_agents
                .iter()
                .any(|agent| agent.has_model("operator-model")));
            assert!(settings.image_gen_profiles.is_empty());
            assert!(settings.openverse_client_secret.is_empty());
            Ok(())
        },
    )
    .expect("browser credentials are scrubbed and saved");

    assert_eq!(saves, 1);
}

#[test]
fn browser_only_startup_propagates_credential_scrub_save_failure() {
    let mut editor = EditorState::new();
    crate::web_credentials::apply_json(&mut editor, CREDENTIAL_BODY)
        .expect("private deployment snapshot");

    let error = enforce_credential_persistence_policy(
        &mut editor,
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
        |_| Err(crate::settings_io::SettingsIoError::PathUnavailable),
    )
    .expect_err("startup must fail when scrubbed settings cannot be saved")
    .to_string();

    assert_eq!(
        error,
        "failed to remove browser-owned credentials while server persistence is disabled"
    );
    assert!(editor
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .all(|agent| !agent.id.starts_with("web-credential:")));
}

#[test]
fn server_persistence_startup_keeps_browser_credentials_without_saving() {
    let mut editor = EditorState::new();
    crate::web_credentials::apply_json(&mut editor, CREDENTIAL_BODY)
        .expect("private deployment snapshot");

    enforce_credential_persistence_policy(
        &mut editor,
        crate::web_credential_policy::WebCredentialPersistence::Server,
        |_| panic!("server persistence must not rewrite settings at startup"),
    )
    .expect("server persistence keeps stored browser credentials");

    assert!(editor
        .editor_ui
        .agent_settings
        .builtin_agents
        .iter()
        .any(|agent| agent.id.starts_with("web-credential:")));
}

#[test]
fn credential_persistence_failure_rolls_back_and_returns_500_without_echoing_secrets() {
    let mut state = fresh_server_persistence_state();
    let settings_before = crate::settings_io::fingerprint(&state.editor);
    let agent_settings_before = state.editor.editor_ui.agent_settings.clone();
    let document_children = state.editor.doc.children.as_ptr();
    let reply = handle_web_canvas_request(
        "POST",
        "/api/settings/credentials",
        CREDENTIAL_BODY,
        &mut state,
    );
    assert_eq!(reply.status, "200 OK");

    let reply = persist_api_settings(
        "POST",
        "/api/settings/credentials",
        &mut state,
        settings_before.clone(),
        Some(agent_settings_before),
        reply,
        |_| Err(crate::settings_io::SettingsIoError::PathUnavailable),
    );

    assert_eq!(reply.status, "500 Internal Server Error");
    assert_eq!(
        settings_before,
        crate::settings_io::fingerprint(&state.editor)
    );
    assert_eq!(state.editor.doc.children.as_ptr(), document_children);
    assert!(!reply.body.contains("sk-browser-only"));
    assert!(!reply.body.contains("simulated disk failure"));
}

#[test]
fn cross_origin_browser_cannot_write_server_credentials() {
    let state = Mutex::new(fresh_server_persistence_state());
    let before = {
        let guard = state.lock().unwrap();
        crate::settings_io::fingerprint(&guard.editor)
    };
    let request = format!(
        "POST /api/settings/credentials HTTP/1.1\r\nHost: 127.0.0.1:3100\r\nOrigin: https://evil.example\r\nContent-Length: {}\r\n\r\n{}",
        CREDENTIAL_BODY.len(),
        CREDENTIAL_BODY
    );
    let request_len = request.len();
    let mut stream = std::io::Cursor::new(request.into_bytes());

    serve_one(&mut stream, &state, &SseHub::default()).expect("request handled");

    let response = String::from_utf8_lossy(&stream.get_ref()[request_len..]);
    assert!(response.contains("403 Forbidden"));
    assert!(response.contains("cross-origin"));
    assert!(!response.contains("sk-browser-only"));
    let guard = state.lock().unwrap();
    assert_eq!(before, crate::settings_io::fingerprint(&guard.editor));
}

#[test]
fn cross_origin_browser_cannot_discover_models_with_a_credential() {
    let state = Mutex::new(fresh_state());
    let body = r#"{"id":"builtin-1","generation":1,"credential":{
        "id":"builtin-1","preset":"openai","display_name":"Private",
        "kind":"openai-compat","api_key":"sk-must-not-leak",
        "model":"fallback","base_url":"https://api.openai.com/v1",
        "enabled":true}}"#;
    let request = format!(
        "POST /api/ai/models/discover HTTP/1.1\r\nHost: 127.0.0.1:3100\r\nOrigin: https://evil.example\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body,
    );
    let request_len = request.len();
    let mut stream = std::io::Cursor::new(request.into_bytes());

    serve_one(&mut stream, &state, &SseHub::default()).expect("request handled");

    let response = String::from_utf8_lossy(&stream.get_ref()[request_len..]);
    assert!(response.contains("403 Forbidden"), "{response}");
    assert!(response.contains("cross-origin"), "{response}");
    assert!(!response.contains("sk-must-not-leak"), "{response}");
}

#[test]
fn cross_origin_browser_cannot_read_account_avatar() {
    let state = Mutex::new(fresh_state());
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: 127.0.0.1:3100\r\nOrigin: https://evil.example\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{{}}",
        op_editor_core::auth_routes::AVATAR
    );
    let request_len = request.len();
    let mut stream = std::io::Cursor::new(request.into_bytes());

    serve_one(&mut stream, &state, &SseHub::default()).expect("request handled");

    let response = String::from_utf8_lossy(&stream.get_ref()[request_len..]);
    assert!(response.contains("403 Forbidden"));
    assert!(response.contains("cross-origin"));
}

#[test]
fn account_avatar_proxy_rejects_subresource_gets_without_fetching() {
    let state = Mutex::new(fresh_state());
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1:3100\r\nContent-Length: 0\r\n\r\n",
        op_editor_core::auth_routes::AVATAR
    );
    let request_len = request.len();
    let mut stream = std::io::Cursor::new(request.into_bytes());

    serve_one(&mut stream, &state, &SseHub::default()).expect("request handled");

    let response = String::from_utf8_lossy(&stream.get_ref()[request_len..]);
    assert!(response.contains("404 Not Found"));
    assert!(!response.contains("\"encoded\""));
}

#[test]
fn credential_origin_check_allows_default_loopback_and_non_browser_clients() {
    for headers in [
        "Host: 127.0.0.1:3100\r\nOrigin: http://127.0.0.1:3100\r\n",
        "Host: localhost:3100\r\nOrigin: http://localhost:3100\r\n",
        "Host: [::1]:3100\r\nOrigin: http://[::1]:3100\r\n",
        "Host: private.example:8443\r\n",
    ] {
        let request = format!(
            "POST /api/settings/credentials HTTP/1.1\r\n{headers}Content-Length: 0\r\n\r\n"
        );
        let mut stream = std::io::Cursor::new(request.into_bytes());
        let request = crate::mcp_serve::read_http_request(&mut stream).unwrap();
        assert!(
            credential_request_origin_allowed_with_config(&request, None),
            "headers={headers:?}"
        );
    }
}

#[test]
fn credential_origin_check_allows_an_explicitly_configured_public_origin() {
    let request = "POST /api/settings/credentials HTTP/1.1\r\nHost: demo.example:8443\r\nOrigin: https://demo.example:8443\r\nContent-Length: 0\r\n\r\n";
    let mut stream = std::io::Cursor::new(request.as_bytes());
    let request = crate::mcp_serve::read_http_request(&mut stream).unwrap();

    assert!(credential_request_origin_allowed_with_config(
        &request,
        Some("https://other.example, https://demo.example:8443"),
    ));
}

#[test]
fn credential_origin_check_rejects_an_unconfigured_public_same_host_origin() {
    let request = "POST /api/settings/credentials HTTP/1.1\r\nHost: evil.example\r\nOrigin: https://evil.example\r\nContent-Length: 0\r\n\r\n";
    let mut stream = std::io::Cursor::new(request.as_bytes());
    let request = crate::mcp_serve::read_http_request(&mut stream).unwrap();

    assert!(!credential_request_origin_allowed_with_config(
        &request, None,
    ));
}

#[test]
fn credential_origin_check_rejects_null_malformed_and_cross_authority_origins() {
    for origin in [
        "null",
        "://bad",
        "https://evil.example",
        "file://private.example",
    ] {
        let request = format!(
            "POST /api/settings/credentials HTTP/1.1\r\nHost: private.example\r\nOrigin: {origin}\r\nContent-Length: 0\r\n\r\n"
        );
        let mut stream = std::io::Cursor::new(request.into_bytes());
        let request = crate::mcp_serve::read_http_request(&mut stream).unwrap();
        assert!(
            !credential_request_origin_allowed_with_config(&request, None),
            "origin={origin}"
        );
    }
}

#[test]
fn sensitive_browser_posts_include_credentials_and_ai_routes() {
    for path in ["/api/settings/credentials", "/api/ai/stream"] {
        let request = crate::mcp_serve::HttpRequest {
            method: "POST".into(),
            path: path.into(),
            body: String::new(),
            host: None,
            origin: None,
            token: None,
            content_type: None,
            authorization: None,
            cookie: None,
            query: None,
        };
        assert!(is_sensitive_browser_post(&request), "path={path}");
    }
}

#[test]
fn post_mcp_server_start_stop_updates_daemon_agent_settings() {
    let mut s = fresh_state();
    assert!(!s.editor.editor_ui.agent_settings.mcp_server.running);

    let start = handle_web_canvas_request(
        "POST",
        "/api/mcp/server",
        r#"{"action":"start","port":3201}"#,
        &mut s,
    );

    assert!(start.status.starts_with("200"), "{}", start.body);
    assert!(start.body.contains(r#""ok":true"#), "{}", start.body);
    assert!(s.editor.editor_ui.agent_settings.mcp_server.running);
    assert_eq!(s.editor.editor_ui.agent_settings.mcp_server.port, 3201);
    assert_eq!(s.version, 0, "MCP settings are not document mutations");

    let stop = handle_web_canvas_request(
        "POST",
        "/api/mcp/server",
        r#"{"action":"stop","port":3201}"#,
        &mut s,
    );

    assert!(stop.status.starts_with("200"), "{}", stop.body);
    assert!(!s.editor.editor_ui.agent_settings.mcp_server.running);
    assert_eq!(s.editor.editor_ui.agent_settings.mcp_server.port, 3201);
    assert_eq!(s.version, 0, "MCP settings are not document mutations");
}

#[test]
fn post_mcp_server_rejects_invalid_body_without_changing_settings() {
    let mut s = fresh_state();
    s.editor.editor_ui.agent_settings.mcp_server.running = true;
    s.editor.editor_ui.agent_settings.mcp_server.port = 4321;

    let r = handle_web_canvas_request("POST", "/api/mcp/server", r#"{"action":"restart"}"#, &mut s);

    assert!(r.status.starts_with("400"), "{}", r.body);
    assert!(r.body.contains("Invalid MCP server action"), "{}", r.body);
    assert!(s.editor.editor_ui.agent_settings.mcp_server.running);
    assert_eq!(s.editor.editor_ui.agent_settings.mcp_server.port, 4321);
}

#[test]
fn get_document_returns_doc_and_version() {
    let mut state = fresh_state();
    state.editor.editor_ui.preserve_authored_geometry = true;
    let r = handle_web_canvas_request("GET", "/api/mcp/document", "", &mut state);
    assert!(r.status.starts_with("200"));
    assert!(r.body.contains(r#""document":"#));
    assert!(r.body.contains(r#""version":0"#));
    assert!(r.body.contains(r#""activePageIndex":0"#));
    assert!(r.body.contains(r#""preserveAuthoredGeometry":true"#));
}

#[test]
fn no_path_startup_uses_the_same_starter_document_as_the_web_shell() {
    let editor = startup_editor_for_web_canvas(None).expect("startup editor");
    assert_eq!(editor.doc, EditorState::starter().doc);
}

#[test]
fn every_web_persistence_policy_propagates_strict_settings_load_failures() {
    for policy in [
        crate::web_credential_policy::WebCredentialPersistence::BrowserOnly,
        crate::web_credential_policy::WebCredentialPersistence::Server,
    ] {
        let checked_calls = std::cell::Cell::new(0);
        // The stub now returns the loader's own typed refusal instead of a
        // fabricated sentence; the assertion still checks that the start-up
        // path propagates it verbatim rather than re-wording it.
        let result = startup_editor_for_web_canvas_with_loader(None, policy, |_| {
            checked_calls.set(checked_calls.get() + 1);
            Err(crate::settings_io::SettingsIoError::Lossy)
        });

        let error = result
            .expect_err("all web policies must fail closed on invalid settings")
            .to_string();
        assert_eq!(
            error,
            crate::settings_io::SettingsIoError::Lossy.to_string()
        );
        assert_eq!(checked_calls.get(), 1, "policy={policy:?}");
    }
}

#[test]
fn post_document_replaces_doc_and_bumps_version() {
    use op_editor_core::PenNodeExt;
    let mut s = fresh_state();
    let r = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut s);
    assert!(r.status.starts_with("200"), "{}", r.body);
    assert!(r.body.contains(r#""ok":true"#));
    assert!(r.body.contains(r#""version":1"#));
    // The in-memory document was replaced with the synced tree.
    assert!(s
        .editor
        .active_children()
        .iter()
        .any(|n| n.base().name.as_deref() == Some("Synced Rect")));
    // A second sync bumps the version again (monotonic).
    let r2 = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut s);
    assert!(r2.body.contains(r#""version":2"#));
}

#[test]
fn post_file_save_requires_a_known_daemon_path() {
    let mut s = fresh_state();

    let r = handle_web_canvas_request("POST", "/api/file/save", SYNC_BODY, &mut s);

    assert!(r.status.starts_with("400"), "{}", r.body);
    assert!(r.body.contains("No file path"), "{}", r.body);
    assert_eq!(s.version, 0);
}

#[test]
fn post_file_save_writes_current_path_and_embedded_editor_meta() {
    use op_editor_core::PenNodeExt;

    let path = write_temp_op("save-target", r#"{"version":"1.0.0","children":[]}"#);
    let mut s = WebCanvasState::new_with_path(EditorState::new(), 3100, Some(path.clone()));
    let body = r##"{"document":{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[{"id":"saved-node","type":"rectangle","name":"Saved Rect","x":1,"y":2,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]}],"editorMeta":{"activePageIndex":1,"preserveAuthoredGeometry":true}},"activePageIndex":1}"##;

    let r = handle_web_canvas_request("POST", "/api/file/save", body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    assert!(r.body.contains(r#""ok":true"#), "{}", r.body);
    assert_eq!(s.version, 1);
    assert_eq!(s.editor.ui.active_page_index, 1);
    assert!(s.editor.editor_ui.preserve_authored_geometry);
    assert_eq!(s.editor.active_children()[0].base().id, "saved-node");
    let saved = std::fs::read_to_string(&path).expect("saved file");
    assert!(saved.contains("saved-node"), "{saved}");
    let saved_json: serde_json::Value = serde_json::from_str(&saved).expect("saved json");
    assert_eq!(saved_json["editorMeta"]["activePageIndex"], 1);
    assert_eq!(saved_json["editorMeta"]["preserveAuthoredGeometry"], true);
    let mut sidecar = path.clone();
    sidecar.set_extension("op.opmeta");
    assert!(
        !sidecar.exists(),
        "new saves should keep active page metadata inside the .op file"
    );
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(sidecar);
}

#[test]
fn post_file_save_keeps_document_wrapper_validation_errors() {
    let path = write_temp_op("save-validation", r#"{"version":"1.0.0","children":[]}"#);
    let mut state = WebCanvasState::new_with_path(EditorState::new(), 3100, Some(path.clone()));

    let missing = handle_web_canvas_request("POST", "/api/file/save", "{}", &mut state);
    assert!(missing.status.starts_with("400"), "{}", missing.body);
    assert!(missing.body.contains("save failed: missing document"));

    let scalar =
        handle_web_canvas_request("POST", "/api/file/save", r#"{"document":42}"#, &mut state);
    assert!(scalar.status.starts_with("400"), "{}", scalar.body);
    assert!(scalar
        .body
        .contains("save failed: document must be an object"));
    assert_eq!(state.version, 0);

    let _ = std::fs::remove_file(path);
}

#[test]
fn sync_reset_reloads_current_path_when_daemon_has_backing_file() {
    use op_editor_core::PenNodeExt;

    let path = write_temp_op(
        "reset-backed",
        r#"{"version":"1.0.0","children":[{"id":"from-disk","type":"rectangle","name":"Disk Rect","x":1,"y":2,"width":80,"height":40}]}"#,
    );
    let mut s = WebCanvasState::new_with_path(EditorState::starter(), 3100, Some(path.clone()));
    s.editor.editor_ui.account_ui_available = true;
    s.editor.editor_ui.account = op_editor_core::AccountState::signed_in_profile(
        "Fini".to_string(),
        Some("fini".to_string()),
    );
    let _ = s.replace_document(
        op_pen_loader::load_canonical(
            r#"{"version":"1.0.0","children":[{"id":"transient","type":"rectangle","name":"Transient","x":1,"y":2,"width":80,"height":40}]}"#,
        )
        .expect("transient doc")
        .value,
    );

    let r = handle_web_canvas_request("POST", "/api/mcp/sync-reset", "", &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    assert_eq!(s.version, 2);
    assert_eq!(s.editor.active_children()[0].base().id, "from-disk");
    assert_eq!(
        s.editor.editor_ui.file_name_display.as_deref(),
        Some(path.file_name().unwrap().to_str().unwrap())
    );
    assert_eq!(s.current_path.as_deref(), Some(path.as_path()));
    assert!(s.editor.editor_ui.account_ui_available);
    assert_eq!(
        s.editor.editor_ui.account,
        op_editor_core::AccountState::signed_in_profile(
            "Fini".to_string(),
            Some("fini".to_string()),
        )
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn sync_reset_preserves_signed_out_account_entry_capability() {
    let path = write_temp_op("reset-auth-gate", r#"{"version":"1.0.0","children":[]}"#);
    let mut state = WebCanvasState::new_with_path(EditorState::starter(), 3100, Some(path.clone()));
    state.editor.editor_ui.account_ui_available = true;
    state.editor.editor_ui.account = op_editor_core::AccountState::Anonymous;

    let reset = state.reset_document_guarded().expect("reset succeeds");

    assert!(!reset.skipped);
    assert!(state.editor.editor_ui.account_ui_available);
    assert_eq!(
        state.editor.editor_ui.account,
        op_editor_core::AccountState::Anonymous
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn post_document_rejects_invalid_body_with_400() {
    let mut s = fresh_state();
    let r = handle_web_canvas_request("POST", "/api/mcp/document", r#"{"nope":1}"#, &mut s);
    assert!(r.status.starts_with("400"));
    assert!(r.body.contains("Missing document in request body"));
    // A rejected sync must not bump the version.
    assert_eq!(s.version, 0);
}

#[test]
fn unknown_route_404s() {
    let r = handle_web_canvas_request("DELETE", "/whatever", "", &mut fresh_state());
    assert!(r.status.starts_with("404"));
}

#[test]
fn get_ai_models_returns_json_array() {
    let mut state = fresh_state();
    state
        .editor
        .editor_ui
        .agent_settings
        .add_builtin_agent_with_defaults("Built-in", "sk-test", "built-in-model");
    state.editor.chat.discovered_models = vec![op_editor_core::ModelEntry::new(
        op_editor_core::AgentProvider::ClaudeCode,
        "cli-model",
        "CLI model",
    )];
    state
        .editor
        .editor_ui
        .agent_settings
        .apply_provider_connect_outcome(
            op_editor_core::AgentProvider::ClaudeCode,
            op_editor_core::ProviderConnectOutcome {
                connected: true,
                info: Some("Connected via CLI".into()),
                ..Default::default()
            },
        );
    state.editor.rebuild_chat_models();

    let r = handle_web_canvas_request("GET", "/api/ai/models", "", &mut state);
    assert!(r.status.starts_with("200"));
    let models =
        serde_json::from_str::<Vec<serde_json::Value>>(&r.body).expect("models body is valid JSON");
    assert_eq!(models.len(), 1);
    assert_eq!(models[0]["displayName"], "built-in-model");
    assert_eq!(models[0]["providerDisplayName"], "Built-in");
    assert!(models[0]["builtinProviderId"].as_str().is_some());
}

#[path = "web_canvas_server_export_tests.rs"]
mod export_tests;

#[path = "web_canvas_server_conn_tests.rs"]
mod conn_tests;
