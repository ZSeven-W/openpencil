//! Redraw-scheduler tests for the desktop event loop. Split out of
//! `main.rs` to keep that file under the 800-line cap.

use super::*;
use op_host_services::mcp_serve::tool_text;

#[test]
fn cursor_only_redraw_without_visible_state_change_skips_present() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;
    app.pending_cursor_move = Some((1200.0, 20.0));

    assert!(!app.prepare_redraw());
    assert!(!app.redraw_pending);
    assert!(app.pending_cursor_move.is_none());
}

#[test]
fn consumed_press_dirties_existing_cursor_redraw_without_second_request() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;

    assert!(!app.request_redraw(true));
    assert!(app.prepare_redraw());
}

#[test]
fn cursor_redraw_still_paints_when_layer_hover_changes() {
    let mut app = DesktopApp::new(None);
    app.redraw_pending = true;
    app.pending_cursor_move = Some((
        20.0,
        op_editor_ui::widgets::TOP_BAR_HEIGHT + 8.0 + 28.0 + 16.0,
    ));

    assert!(app.prepare_redraw());
}

#[test]
fn variable_row_input_keeps_resume_time_redraws_active() {
    // Serialize against reveal-streaming design-turn tests and start from
    // a quiescent registry so only the caret blink drives the deadline.
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();
    let mut app = DesktopApp::new(None);
    app.host.set_now_ms(240);
    app.host.editor_state_mut().editor_ui.variable_row_focus =
        Some(op_editor_core::editor_ui_state::VariableRowFocus::Name(0));
    app.host
        .editor_state_mut()
        .editor_ui
        .variable_row_input
        .touch(240);

    assert!(app.resume_time_needs_redraw());
    assert_eq!(app.host.next_animation_deadline_ms(), Some(740));
}

#[test]
fn fresh_app_fits_blank_frame_like_ts_canvas_init() {
    let app = DesktopApp::new(None);
    let v = app.host.editor_state().viewport;

    // Golden fit values track `property_panel_width` (the right rail is
    // shown on the fresh app, so the canvas region = 1440 − panel). At
    // the TS-matching `w-64` (256 px) panel the blank frame fits at 0.68.
    assert!((v.zoom - 0.68).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 158.0).abs() < 1e-2, "pan_y {}", v.pan_y);
}

#[test]
fn fresh_app_refits_blank_frame_to_actual_window_size_once() {
    let mut app = DesktopApp::new(None);
    app.viewport_width = 1000.0;
    app.viewport_height = 700.0;

    assert!(app.fit_initial_blank_frame_to_actual_viewport());
    let v = app.host.editor_state().viewport;
    assert!((v.zoom - 0.31333333).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - 64.0).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - 204.66666).abs() < 1e-2, "pan_y {}", v.pan_y);

    app.viewport_width = 1200.0;
    app.viewport_height = 800.0;
    assert!(!app.fit_initial_blank_frame_to_actual_viewport());
    let unchanged = app.host.editor_state().viewport;
    assert_eq!(v, unchanged);
}

#[test]
fn design_md_auto_generate_does_not_fall_back_to_local_extraction() {
    use jian_ops_schema::variable::{
        VariableDefinition, VariableKind, VariableScalar, VariableValue,
    };
    use op_ai::chat_provider::{ChatDelta, EchoProvider, StopReason};
    use std::collections::BTreeMap;

    let mut app = DesktopApp::new(None);
    let mut variables = BTreeMap::new();
    variables.insert(
        "$color-brand".to_string(),
        VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Scalar(VariableScalar::Str("#2563eb".to_string())),
        },
    );
    {
        let state = app.host.editor_state_mut();
        state.doc.name = Some("Generated Brief".to_string());
        state.doc.variables = Some(variables);
        state.doc.design_md = Some(op_editor_core::parse_design_md(
            "# Design System: Existing\n\n## Visual Theme\nOld brief",
        ));
        state.editor_ui.design_md_request = Some(op_editor_core::DesignMdRequest::AutoGenerate);
    }
    app.set_design_md_test_provider(Box::new(EchoProvider {
        script: vec![
            ChatDelta::TextDelta(
                "# Design System: LLM Brief\n\n\
                 ## 1. Visual Theme & Atmosphere\n\
                 Model-authored brief.\n\n\
                 ## 2. Color Palette & Roles\n\
                 **AI Orange** (#F97316) — Primary accent"
                    .into(),
            ),
            ChatDelta::Done {
                stop_reason: StopReason::EndTurn,
            },
        ],
    }));

    assert!(app.drain_design_md_action());
    assert!(app.host.editor_state().editor_ui.design_md_generating);

    let spec = app
        .host
        .editor_state()
        .doc
        .design_md
        .as_ref()
        .expect("existing design.md should remain until an LLM result lands");
    assert_eq!(spec.project_name.as_deref(), Some("Existing"));
    assert!(
        !spec.raw.contains("#2563EB"),
        "auto-generate must not masquerade as AI by using local extraction: {}",
        spec.raw
    );

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !app.poll_design_md_generation() {
        assert!(
            std::time::Instant::now() < deadline,
            "design.md generation worker did not finish"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let spec = app
        .host
        .editor_state()
        .doc
        .design_md
        .as_ref()
        .expect("LLM-generated design.md");
    assert_eq!(spec.project_name.as_deref(), Some("LLM Brief"));
    assert!(spec.raw.contains("#F97316"));
    assert!(!spec.raw.contains("#2563EB"));
    assert!(!app.host.editor_state().editor_ui.design_md_generating);

    assert!(app.host.editor_state_mut().undo());
    let restored = app
        .host
        .editor_state()
        .doc
        .design_md
        .as_ref()
        .expect("previous design.md restored");
    assert_eq!(restored.project_name.as_deref(), Some("Existing"));
}

#[test]
fn live_mcp_http_server_applies_write_requests_to_editor_state() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use op_editor_core::PenNodeExt;

    fn start_live_server() -> (op_host_services::mcp_live::McpLiveServer, u16) {
        // `bind(0)` to grab an ephemeral port, then re-`start` on that port,
        // has a TOCTOU window where the OS can reassign the port between the
        // probe-listener drop and the server bind — so a single attempt
        // occasionally fails. Retry with a fresh port each time to remove the
        // flake (the failure that masqueraded as a test regression).
        for _ in 0..20 {
            let port = {
                let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
                listener.local_addr().expect("local addr").port()
            };
            if let Ok(server) = op_host_services::mcp_live::McpLiveServer::start(port) {
                return (server, port);
            }
        }
        panic!("could not start MCP server on an unused port after 20 attempts");
    }

    fn post_json(port: u16, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect MCP server");
        let req = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).expect("write request");
        let mut out = String::new();
        stream.read_to_string(&mut out).expect("read response");
        out
    }

    let (mut server, port) = start_live_server();
    let mut state = op_editor_core::EditorState::new();
    let body = r##"{"jsonrpc":"2.0","id":1,"method":"insert_node","params":{"kind":"rect","name":"From MCP","x":"10","y":"20","width":"100","height":"50","fill_hex":"#00ff00"}}"##;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(post_json(port, body));
    });

    let started = Instant::now();
    let response = loop {
        server.pump(&mut state);
        if let Ok(response) = rx.try_recv() {
            break response;
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "MCP request timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    };

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(
        tool_text(&response).contains(r#""wrote":"true""#),
        "{response}"
    );
    assert!(
        state
            .active_children()
            .iter()
            .any(|node| node.base().name.as_deref() == Some("From MCP")),
        "MCP write should mutate the live editor state"
    );
}

#[test]
fn live_mcp_http_server_waits_for_split_http_request() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn start_live_server() -> (op_host_services::mcp_live::McpLiveServer, u16) {
        for _ in 0..20 {
            let port = {
                let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
                listener.local_addr().expect("local addr").port()
            };
            if let Ok(server) = op_host_services::mcp_live::McpLiveServer::start(port) {
                return (server, port);
            }
        }
        panic!("could not start MCP server on an unused port after 20 attempts");
    }

    fn post_ping_in_chunks(port: u16) -> String {
        let body = r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#;
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect MCP server");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set read timeout");
        let head = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).expect("write head");
        stream.flush().expect("flush head");
        std::thread::sleep(Duration::from_millis(50));
        stream
            .write_all(format!("\r\n{body}").as_bytes())
            .expect("write body");
        let mut out = String::new();
        stream.read_to_string(&mut out).expect("read response");
        out
    }

    let (_server, port) = start_live_server();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(post_ping_in_chunks(port));
    });

    let started = Instant::now();
    let response = loop {
        if let Ok(response) = rx.try_recv() {
            break response;
        }
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "split HTTP request timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    };

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains(r#""mode":"live""#), "{response}");
}

#[test]
fn live_mcp_http_server_routes_file_path_requests_to_target_file() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn start_live_server() -> (op_host_services::mcp_live::McpLiveServer, u16) {
        // `bind(0)` to grab an ephemeral port, then re-`start` on that port,
        // has a TOCTOU window where the OS can reassign the port between the
        // probe-listener drop and the server bind — so a single attempt
        // occasionally fails. Retry with a fresh port each time to remove the
        // flake (the failure that masqueraded as a test regression).
        for _ in 0..20 {
            let port = {
                let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
                listener.local_addr().expect("local addr").port()
            };
            if let Ok(server) = op_host_services::mcp_live::McpLiveServer::start(port) {
                return (server, port);
            }
        }
        panic!("could not start MCP server on an unused port after 20 attempts");
    }

    fn post_json(port: u16, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect MCP server");
        let req = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).expect("write request");
        let mut out = String::new();
        stream.read_to_string(&mut out).expect("read response");
        out
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

    let (mut server, port) = start_live_server();
    let mut state = op_editor_core::EditorState::new();
    let dir = std::env::temp_dir().join(format!(
        "openpencil-live-mcp-filepath-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let alternate_path = dir.join("alternate.op");
    write_named_doc(&alternate_path, "n2", "Alternate");
    let file_path_json =
        serde_json::to_string(&alternate_path.to_string_lossy()).expect("path json");
    let body = format!(
        r#"{{"jsonrpc":"2.0","id":31,"method":"tools/call","params":{{"name":"batch_get","arguments":{{"filePath":{file_path_json},"readDepth":1}}}}}}"#
    );
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(post_json(port, &body));
    });

    let started = Instant::now();
    let response = loop {
        server.pump(&mut state);
        if let Ok(response) = rx.try_recv() {
            break response;
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "MCP request timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    };

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains("Alternate"), "{response}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn live_mcp_http_server_replaces_document_via_rest_document_sync() {
    use op_editor_core::PenNodeExt;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    fn start_live_server() -> (op_host_services::mcp_live::McpLiveServer, u16) {
        for _ in 0..20 {
            let port = {
                let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
                listener.local_addr().expect("local addr").port()
            };
            if let Ok(server) = op_host_services::mcp_live::McpLiveServer::start(port) {
                return (server, port);
            }
        }
        panic!("could not start MCP server on an unused port after 20 attempts");
    }

    fn post(port: u16, path: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect MCP server");
        let req = format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).expect("write request");
        let mut out = String::new();
        stream.read_to_string(&mut out).expect("read response");
        out
    }

    let (mut server, port) = start_live_server();
    let mut state = op_editor_core::EditorState::new();
    // Preserved chrome state a whole-document sync must NOT wipe.
    state.editor_ui.sidebar_open = false;

    // A TS whole-doc-sync client (`setSyncDocument`) POSTs `{document}` to
    // `/api/mcp/document` — the same REST shape `document.post.ts` serves.
    let body = r##"{"document":{"version":"1.0.0","children":[{"id":"n9","type":"rectangle","name":"Synced Rect","x":5,"y":6,"width":80,"height":40,"fill":[{"type":"solid","color":"#123456"}]}]},"sourceClientId":"ts-app"}"##;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(post(port, "/api/mcp/document", body));
    });

    let started = Instant::now();
    let response = loop {
        server.pump(&mut state);
        if let Ok(response) = rx.try_recv() {
            break response;
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "document sync timed out"
        );
        std::thread::sleep(Duration::from_millis(10));
    };

    assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
    assert!(response.contains(r#""ok":true"#), "{response}");
    assert!(response.contains(r#""version":"#), "{response}");
    // The live document was replaced with the synced tree...
    assert!(
        state
            .active_children()
            .iter()
            .any(|node| node.base().name.as_deref() == Some("Synced Rect")),
        "REST document sync should replace the live editor document"
    );
    // ...while preserved editor chrome survived the sync.
    assert!(
        !state.editor_ui.sidebar_open,
        "document sync must not reset editor_ui"
    );
}

#[test]
fn startup_mcp_bootstrap_starts_live_server_for_enabled_cli() {
    // Env-free CLI-integration home (bootstrap's detect/write target it,
    // never the real home / `CODEX_HOME`).
    let home = std::env::temp_dir().join(format!(
        "openpencil-mcp-bootstrap-start-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&home);
    let mut app = DesktopApp::new(None);
    app.mcp_integrations_home = Some(home.clone());
    let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
    settings.mcp_server.port = 0;
    settings.mcp_server.running = false;
    let codex_idx = op_editor_core::agent_settings::McpCli::ALL
        .iter()
        .position(|cli| *cli == op_editor_core::agent_settings::McpCli::Codex)
        .expect("Codex CLI index");
    settings.mcp_cli_enabled[codex_idx] = true;

    assert!(app.bootstrap_mcp_runtime_from_settings());

    assert!(app.mcp_server_active());
    assert!(
        app.host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_server
            .running
    );
    assert_ne!(
        app.mcp_server.as_ref().expect("server").port(),
        0,
        "ephemeral port should be reported after binding"
    );
    assert_eq!(
        app.host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_server
            .port,
        app.mcp_server.as_ref().expect("server").port(),
        "settings should reflect the bound port so the server is not restarted on every reconcile"
    );
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn startup_mcp_bootstrap_updates_cli_config_after_port_fallback() {
    use std::net::TcpListener;

    let busy = TcpListener::bind(("127.0.0.1", 0)).expect("bind busy port");
    let busy_port = busy.local_addr().expect("busy port addr").port();
    // Redirect CLI-config detection + writes to a temp home via the override
    // — no process-global `CODEX_HOME`/`HOME` mutation, so this test never
    // races another test's env access.
    let home = std::env::temp_dir().join(format!(
        "openpencil-mcp-bootstrap-home-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&home);

    let mut app = DesktopApp::new(None);
    app.mcp_integrations_home = Some(home.clone());
    let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
    settings.mcp_server.port = busy_port;
    settings.mcp_server.running = false;
    let codex_idx = op_editor_core::agent_settings::McpCli::ALL
        .iter()
        .position(|cli| *cli == op_editor_core::agent_settings::McpCli::Codex)
        .expect("Codex CLI index");
    settings.mcp_cli_enabled[codex_idx] = true;

    assert!(app.bootstrap_mcp_runtime_from_settings());

    let bound_port = app.mcp_server.as_ref().expect("server").port();
    assert_ne!(bound_port, busy_port);
    let codex_config = std::fs::read_to_string(home.join(".codex").join("config.toml"))
        .expect("Codex config should be written");
    assert!(
        codex_config.contains(&format!("http://127.0.0.1:{bound_port}/mcp")),
        "{codex_config}"
    );
    assert!(
        !codex_config.contains(&format!("http://127.0.0.1:{busy_port}/mcp")),
        "{codex_config}"
    );

    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn manual_mcp_start_falls_back_to_available_port_when_requested_port_is_busy() {
    use std::net::TcpListener;

    let busy = TcpListener::bind(("127.0.0.1", 0)).expect("bind busy port");
    let busy_port = busy.local_addr().expect("busy port addr").port();
    let mut app = DesktopApp::new(None);
    let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
    settings.mcp_server.port = busy_port;
    settings.mcp_server.running = true;

    assert!(app.reconcile_mcp_server_from_settings());

    let settings = &app.host.editor_state().editor_ui.agent_settings;
    let server = app.mcp_server.as_ref().expect("server should start");
    assert!(settings.mcp_server.running);
    assert_ne!(server.port(), busy_port);
    assert_eq!(settings.mcp_server.port, server.port());
}

#[test]
fn forced_live_mcp_starts_without_persisting_settings() {
    let mut app = DesktopApp::new(None);
    {
        let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
        settings.mcp_server.running = false;
        settings.mcp_server.port = 3100;
    }
    // `op start --live-mcp` forces the server on (ephemeral port 0 here to
    // avoid clashing with a real 3100 server during tests).
    app.force_live_mcp_port = Some(0);

    assert!(app.reconcile_mcp_server_from_settings());
    assert!(app.mcp_server_active());
    let bound = app.mcp_server.as_ref().expect("server").port();

    // A forced launch must NOT mutate / persist the user's settings.
    let settings = &app.host.editor_state().editor_ui.agent_settings;
    assert!(
        !settings.mcp_server.running,
        "forced launch must not flip persisted running=true"
    );
    assert_eq!(
        settings.mcp_server.port, 3100,
        "forced launch must not persist the bound port"
    );
    // The runtime force port tracks the bound port so the next reconcile is
    // a no-op (no restart loop).
    assert_eq!(app.force_live_mcp_port, Some(bound));
    assert!(!app.reconcile_mcp_server_from_settings());
}

#[test]
fn disabling_mcp_server_reports_change_so_caller_clears_port_file() {
    let mut app = DesktopApp::new(None);
    app.force_live_mcp_port = Some(0);
    assert!(app.reconcile_mcp_server_from_settings());
    assert!(app.mcp_server_active());
    // Drop the force flag and disable in settings → reconcile must stop the
    // server AND return true so the caller removes the discovery file.
    app.force_live_mcp_port = None;
    app.host
        .editor_state_mut()
        .editor_ui
        .agent_settings
        .mcp_server
        .running = false;
    assert!(app.reconcile_mcp_server_from_settings());
    assert!(!app.mcp_server_active());
}

#[test]
fn parse_live_mcp_port_accepts_all_three_forms() {
    let v = |xs: &[&str]| parse_live_mcp_port(xs.iter().map(|s| s.to_string()));
    // `--live-mcp <port>`
    assert_eq!(v(&["--live-mcp", "3100"]), Some(3100));
    // `--live-mcp=<port>`
    assert_eq!(v(&["--live-mcp=4321"]), Some(4321));
    // bare `--live-mcp` (with a following non-port arg) → default port
    assert_eq!(v(&["--live-mcp", "/tmp/a.op"]), Some(DEFAULT_LIVE_MCP_PORT));
    assert_eq!(v(&["--live-mcp"]), Some(DEFAULT_LIVE_MCP_PORT));
    // absent → None (normal GUI launch keeps settings-gated MCP behavior)
    assert_eq!(v(&["/tmp/a.op"]), None);
    assert_eq!(v(&[]), None);
}

#[test]
fn default_live_mcp_port_matches_ts_and_cli_default() {
    // CLI default + TS pen-mcp default are both 3100; they must agree so
    // `op start` and `op <tool>` find each other out of the box.
    assert_eq!(DEFAULT_LIVE_MCP_PORT, 3100);
}
