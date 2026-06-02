//! Redraw-scheduler tests for the desktop event loop. Split out of
//! `main.rs` to keep that file under the 800-line cap.

use super::*;

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
fn live_mcp_http_server_applies_write_requests_to_editor_state() {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use op_editor_core::PenNodeExt;

    fn unused_port() -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
        listener.local_addr().expect("local addr").port()
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

    let port = unused_port();
    let mut server = mcp_live::McpLiveServer::start(port).expect("start MCP server");
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
    assert!(response.contains(r#""wrote":"true""#), "{response}");
    assert!(
        state
            .active_children()
            .iter()
            .any(|node| node.base().name.as_deref() == Some("From MCP")),
        "MCP write should mutate the live editor state"
    );
}

#[test]
fn startup_mcp_bootstrap_starts_live_server_for_enabled_cli() {
    let mut app = DesktopApp::new(None);
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
}
