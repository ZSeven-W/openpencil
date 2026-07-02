//! Tests for the web-canvas daemon (`web_canvas_server.rs`) — sibling
//! file per the 800-line-cap convention (like `chat_session_tests.rs`).

use super::*;

fn fresh_state() -> WebCanvasState {
    WebCanvasState::new(EditorState::new(), 3100)
}

// A minimal canonical document body in the TS `setSyncDocument` shape.
const SYNC_BODY: &str = r##"{"document":{"version":"1.0.0","children":[{"id":"n9","type":"rectangle","name":"Synced Rect","x":1,"y":2,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]},"sourceClientId":"web"}"##;

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
    let r = handle_web_canvas_request("GET", "/api/mcp/document", "", &mut fresh_state());
    assert!(r.status.starts_with("200"));
    assert!(r.body.contains(r#""document":"#));
    assert!(r.body.contains(r#""version":0"#));
}

#[test]
fn no_path_startup_uses_the_same_starter_document_as_the_web_shell() {
    let editor = startup_editor_for_web_canvas(None).expect("startup editor");
    assert_eq!(editor.doc, EditorState::starter().doc);
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
fn post_file_save_writes_current_path_and_embedded_active_page_meta() {
    use op_editor_core::PenNodeExt;

    let path = write_temp_op("save-target", r#"{"version":"1.0.0","children":[]}"#);
    let mut s = WebCanvasState::new_with_path(EditorState::new(), 3100, Some(path.clone()));
    let body = r##"{"document":{"version":"1.0.0","children":[],"pages":[{"id":"p1","name":"One","children":[]},{"id":"p2","name":"Two","children":[{"id":"saved-node","type":"rectangle","name":"Saved Rect","x":1,"y":2,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]}]},"activePageIndex":1}"##;

    let r = handle_web_canvas_request("POST", "/api/file/save", body, &mut s);

    assert!(r.status.starts_with("200"), "{}", r.body);
    assert!(r.body.contains(r#""ok":true"#), "{}", r.body);
    assert_eq!(s.version, 1);
    assert_eq!(s.editor.ui.active_page_index, 1);
    assert_eq!(s.editor.active_children()[0].base().id, "saved-node");
    let saved = std::fs::read_to_string(&path).expect("saved file");
    assert!(saved.contains("saved-node"), "{saved}");
    let saved_json: serde_json::Value = serde_json::from_str(&saved).expect("saved json");
    assert_eq!(saved_json["editorMeta"]["activePageIndex"], 1);
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
fn sync_reset_reloads_current_path_when_daemon_has_backing_file() {
    use op_editor_core::PenNodeExt;

    let path = write_temp_op(
        "reset-backed",
        r#"{"version":"1.0.0","children":[{"id":"from-disk","type":"rectangle","name":"Disk Rect","x":1,"y":2,"width":80,"height":40}]}"#,
    );
    let mut s = WebCanvasState::new_with_path(EditorState::starter(), 3100, Some(path.clone()));
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
    // The AI proxy model list is served as a JSON array — empty
    // when nothing is configured, but always well-formed JSON the
    // web bundle can `JSON.parse`.
    let r = handle_web_canvas_request("GET", "/api/ai/models", "", &mut fresh_state());
    assert!(r.status.starts_with("200"));
    let parsed: serde_json::Value =
        serde_json::from_str(&r.body).expect("models body is valid JSON");
    assert!(
        parsed.is_array(),
        "models body must be a JSON array: {}",
        r.body
    );
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

    assert_eq!(png_dimensions(&png), (42, 42));
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
    assert_eq!(png_dimensions(&png), (42, 42));
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
fn provider_connect_probe_response_updates_agent_settings_and_models() {
    use op_ai::agent_settings_state::AgentProvider as ProbeProvider;
    use op_ai::chat_models::ModelEntry as ProbeModelEntry;
    use op_editor_core::agent_settings::ProviderConnectPhase;
    use op_editor_core::AgentProvider;

    let mut s = fresh_state();
    let body = serde_json::json!({ "provider": "codex" }).to_string();
    let r = handle_provider_connect_request_with_probe(&body, &mut s, |_| {
        crate::provider_probe::ProbeOutcome {
            connected: true,
            models: vec![ProbeModelEntry::new(
                ProbeProvider::CodexCli,
                "gpt-5.5",
                "GPT-5.5",
            )],
            connection_info: Some("Connected via Codex CLI".to_string()),
            version: Some("codex 1.2.3".to_string()),
            ..Default::default()
        }
    });

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["connected"], true);
    assert_eq!(parsed["models"][0]["value"], "gpt-5.5");

    let idx =
        op_editor_core::agent_settings::AgentSettings::provider_index(AgentProvider::CodexCli);
    let settings = &s.editor.editor_ui.agent_settings;
    assert!(settings.provider_verified_connected(AgentProvider::CodexCli));
    assert_eq!(
        settings.provider_connection[idx].phase,
        ProviderConnectPhase::Connected
    );
    assert!(s
        .editor
        .chat
        .discovered_models
        .iter()
        .any(|m| m.provider == AgentProvider::CodexCli && m.value == "gpt-5.5"));
    assert!(s
        .editor
        .chat
        .available_models
        .iter()
        .any(|m| m.provider == AgentProvider::CodexCli && m.value == "gpt-5.5"));
}

#[test]
fn provider_connect_probe_response_without_models_is_failure() {
    use op_editor_core::agent_settings::ProviderConnectPhase;
    use op_editor_core::AgentProvider;

    let mut s = fresh_state();
    s.editor
        .chat
        .discovered_models
        .push(op_editor_core::ModelEntry::new(
            AgentProvider::CodexCli,
            "stale-gpt",
            "Stale GPT",
        ));
    let body = serde_json::json!({ "provider": "codex" }).to_string();
    let r = handle_provider_connect_request_with_probe(&body, &mut s, |_| {
        crate::provider_probe::ProbeOutcome {
            connected: true,
            connection_info: Some("Connected via Codex CLI".to_string()),
            version: Some("codex 1.2.3".to_string()),
            ..Default::default()
        }
    });

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    assert_eq!(parsed["connected"], false);
    assert!(
        parsed["error"]
            .as_str()
            .is_some_and(|error| error.contains("No models found")),
        "{}",
        r.body
    );

    let settings = &s.editor.editor_ui.agent_settings;
    assert!(!settings.provider_verified_connected(AgentProvider::CodexCli));
    let idx =
        op_editor_core::agent_settings::AgentSettings::provider_index(AgentProvider::CodexCli);
    assert_eq!(
        settings.provider_connection[idx].phase,
        ProviderConnectPhase::Error
    );
    assert!(
        !s.editor
            .chat
            .available_models
            .iter()
            .any(|m| m.provider == AgentProvider::CodexCli),
        "stale Codex models must not become selectable after a failed connect"
    );
}

#[test]
fn acp_connect_probe_response_updates_agent_settings() {
    use op_editor_core::agent_settings::{AcpAgentConnectPhase, AcpConnectionType};
    use std::collections::BTreeMap;

    let mut s = fresh_state();
    s.editor.editor_ui.agent_settings.add_acp_agent_config(
        "Claude Code",
        AcpConnectionType::Local,
        "claude",
        Vec::new(),
        BTreeMap::new(),
        None,
        true,
    );
    let body = serde_json::json!({ "id": "acp-1" }).to_string();
    let r = handle_acp_agent_connect_request_with_probe(&body, &mut s, |_| {
        crate::acp_agent_probe_host::AcpAgentProbeOutcome {
            connected: true,
            info: Some("Claude Code 1.0".into()),
            error: None,
        }
    });

    assert!(r.status.starts_with("200"), "{}", r.body);
    let parsed: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
    assert_eq!(parsed["connected"], true);
    assert_eq!(parsed["connectionInfo"], "Claude Code 1.0");

    let settings = &s.editor.editor_ui.agent_settings;
    assert!(settings.acp_agents[0].connected);
    let conn = settings.acp_agent_connection_for("acp-1");
    assert_eq!(conn.phase, AcpAgentConnectPhase::Connected);
    assert_eq!(conn.info.as_deref(), Some("Claude Code 1.0"));
}

#[test]
fn acp_connect_failure_keeps_agent_disconnected() {
    use op_editor_core::agent_settings::{AcpAgentConnectPhase, AcpConnectionType};
    use std::collections::BTreeMap;

    let mut s = fresh_state();
    s.editor.editor_ui.agent_settings.add_acp_agent_config(
        "Claude Code",
        AcpConnectionType::Local,
        "claude",
        Vec::new(),
        BTreeMap::new(),
        None,
        true,
    );
    let body = serde_json::json!({ "id": "acp-1" }).to_string();
    let r = handle_acp_agent_connect_request_with_probe(&body, &mut s, |_| {
        crate::acp_agent_probe_host::AcpAgentProbeOutcome {
            connected: false,
            info: None,
            error: Some("failed to spawn ACP agent".into()),
        }
    });

    assert!(r.status.starts_with("200"), "{}", r.body);
    let settings = &s.editor.editor_ui.agent_settings;
    assert!(!settings.acp_agents[0].connected);
    let conn = settings.acp_agent_connection_for("acp-1");
    assert_eq!(conn.phase, AcpAgentConnectPhase::Error);
    assert_eq!(conn.error.as_deref(), Some("failed to spawn ACP agent"));
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
    assert_eq!(r.body, r#"{"version":0}"#);
    // A document mutation bumps the probed version.
    let _ = handle_web_canvas_request("POST", "/api/mcp/document", SYNC_BODY, &mut s);
    let r2 = handle_web_canvas_request("GET", "/api/mcp/version", "", &mut s);
    assert_eq!(r2.body, r#"{"version":1}"#);
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
        crate::mcp_serve::process_message_with_applier(&mut s.editor, msg, |editor, cmd| {
            editor.apply(cmd.clone())
        })
        .expect("dispatch")
        .unwrap_or_default();
    assert!(response.contains("n9"), "{response}");
}

// --- serve_one routing (socket-level, via a mock stream) ---

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
fn serve(method: &str, path: &str, body: &str) -> String {
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

#[test]
fn sse_hub_broadcasts_version_to_all_subscribers() {
    let hub = SseHub::default();
    let a = hub.subscribe();
    let b = hub.subscribe();
    hub.broadcast(5);
    assert_eq!(a.recv().unwrap(), 5);
    assert_eq!(b.recv().unwrap(), 5);
}

#[test]
fn sse_hub_prunes_disconnected_subscribers() {
    let hub = SseHub::default();
    let live = hub.subscribe();
    drop(hub.subscribe()); // a disconnected client (receiver dropped)
    assert_eq!(hub.subscriber_count(), 2);
    hub.broadcast(1); // prunes the dropped one
    assert_eq!(hub.subscriber_count(), 1);
    assert_eq!(live.recv().unwrap(), 1);
}

#[test]
fn write_sse_event_emits_data_frame() {
    let mut stream = mock_stream("");
    write_sse_event(&mut stream, 42).expect("write");
    assert_eq!(
        String::from_utf8_lossy(&stream.output),
        "data: {\"version\":42}\n\n"
    );
}

#[test]
fn serve_sse_emits_initial_then_each_version_until_hub_drops() {
    let (tx, rx) = mpsc::channel();
    tx.send(9).expect("send"); // one bump, then the sender drops → Disconnected
    drop(tx);
    let mut stream = mock_stream("");
    serve_sse(&mut stream, rx, 7).expect("serve_sse");
    let out = String::from_utf8_lossy(&stream.output);
    assert!(out.contains("text/event-stream"), "{out}");
    assert!(out.contains(r#"data: {"version":7}"#), "{out}"); // initial sync
    assert!(out.contains(r#"data: {"version":9}"#), "{out}"); // broadcast bump
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
    assert_eq!(sub.recv().unwrap(), 1);
}

#[test]
fn serve_one_routes_rest_health_and_document() {
    assert!(serve("GET", "/api/mcp/server", "").contains("200 OK"));
    assert!(serve("GET", "/api/mcp/document", "").contains("200 OK"));
    let post = serve("POST", "/api/mcp/document", SYNC_BODY);
    assert!(post.contains("200 OK"), "{post}");
    assert!(post.contains(r#""ok":true"#));
}

#[test]
fn serve_one_standard_ai_route_is_sse_not_404() {
    let r = serve("POST", "/api/ai/standard", "not json");
    assert!(r.contains("text/event-stream"), "{r}");
    assert!(r.contains("invalid request body"), "{r}");
    assert!(!r.contains("404 Not Found"), "{r}");
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
    assert_eq!(
        parse(&["3100"]).expect("port only"),
        (3100, None, "127.0.0.1".to_string())
    );
    // Port + doc.
    assert_eq!(
        parse(&["3100", "/tmp/d.op"]).expect("port+doc"),
        (
            3100,
            Some(PathBuf::from("/tmp/d.op")),
            "127.0.0.1".to_string()
        )
    );
    // `--host` in both spellings, before or after the doc.
    assert_eq!(
        parse(&["3100", "--host", "0.0.0.0", "/tmp/d.op"]).expect("host then doc"),
        (
            3100,
            Some(PathBuf::from("/tmp/d.op")),
            "0.0.0.0".to_string()
        )
    );
    assert_eq!(
        parse(&["3100", "/tmp/d.op", "--host=0.0.0.0"]).expect("doc then host="),
        (
            3100,
            Some(PathBuf::from("/tmp/d.op")),
            "0.0.0.0".to_string()
        )
    );
    // Malformed shapes are rejected with a message, not silently dropped.
    assert!(parse(&[]).is_err(), "missing port");
    assert!(parse(&["nope"]).is_err(), "non-numeric port");
    assert!(parse(&["3100", "--host"]).is_err(), "--host without value");
    assert!(parse(&["3100", "a.op", "b.op"]).is_err(), "two docs");
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
