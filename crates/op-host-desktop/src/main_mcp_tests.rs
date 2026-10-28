//! Live-MCP HTTP server + startup-bootstrap tests, split from
//! `main_tests.rs` to keep both files under the repo's 800-line cap.

use super::*;
use op_host_services::mcp_serve::tool_text;

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

    fn post_json(port: u16, token: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect MCP server");
        let req = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nX-OpenPencil-Token: {token}\r\nContent-Length: {}\r\n\r\n{}",
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
    let token = server.token().to_owned();
    std::thread::spawn(move || {
        let _ = tx.send(post_json(port, &token, body));
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

    fn post_json(port: u16, token: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect MCP server");
        let req = format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1\r\nX-OpenPencil-Token: {token}\r\nContent-Length: {}\r\n\r\n{}",
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
    let token = server.token().to_owned();
    std::thread::spawn(move || {
        let _ = tx.send(post_json(port, &token, &body));
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

    fn post(port: u16, token: &str, path: &str, body: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect MCP server");
        let req = format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-OpenPencil-Token: {token}\r\nContent-Length: {}\r\n\r\n{}",
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
    let token = server.token().to_owned();
    std::thread::spawn(move || {
        let _ = tx.send(post(port, &token, "/api/mcp/document", body));
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
    mcp_integrations::set_cli_enabled_at_home(
        op_editor_core::agent_settings::McpCli::Codex,
        true,
        0,
        &home,
    )
    .expect("seed enabled Codex integration");
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
fn startup_mcp_bootstrap_migrates_dsh_home_to_profile_with_bound_lean_endpoint() {
    crate::test_config_root::guard_user_config();
    let home = std::env::temp_dir().join(format!(
        "openpencil-mcp-bootstrap-dsh-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&home);
    let legacy = mcp_integrations::set_cli_enabled_at_home(
        op_editor_core::agent_settings::McpCli::Dsh,
        true,
        4500,
        &home,
    )
    .expect("seed legacy home integration before a profile exists");
    let legacy_text = std::fs::read_to_string(&legacy).expect("legacy patch");
    std::fs::write(&legacy, format!("# keep this user comment\n{legacy_text}"))
        .expect("seed user comment");
    let profile = home.join(".dsh/profiles/desktop");
    std::fs::create_dir_all(&profile).expect("create DSH desktop profile");
    std::fs::write(profile.join("package.json"), "{}").expect("profile manifest");
    std::fs::write(profile.join("cordis.patch.yml"), "[]\n").expect("profile patch");

    let mut app = DesktopApp::new(None);
    app.mcp_integrations_home = Some(home.clone());
    let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
    settings.mcp_server.port = 0;
    settings.mcp_server.running = false;
    settings.mcp_lean_profile = true;

    assert!(app.bootstrap_mcp_runtime_from_settings());
    let port = app.mcp_server.as_ref().expect("live MCP server").port();
    assert_ne!(port, 0);
    let dsh_index = op_editor_core::agent_settings::McpCli::ALL
        .iter()
        .position(|cli| *cli == op_editor_core::agent_settings::McpCli::Dsh)
        .expect("DSH index");
    assert!(
        app.host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_cli_enabled[dsh_index]
    );
    let configured =
        std::fs::read_to_string(profile.join("cordis.patch.yml")).expect("migrated profile patch");
    assert!(
        configured.contains(&mcp_integrations::endpoint_url(port, true)),
        "{configured}"
    );
    assert!(!configured.contains("127.0.0.1:4500/"), "{configured}");
    assert_eq!(configured.matches("id: mcp-openpencil").count(), 1);
    assert_eq!(
        std::fs::read_to_string(&legacy).expect("valid empty home layer"),
        "# keep this user comment\n[]\n"
    );
    drop(app);
    let _ = std::fs::remove_dir_all(home);
}

#[test]
fn startup_mcp_bootstrap_keeps_detected_dsh_enabled_when_refresh_fails() {
    crate::test_config_root::guard_user_config();
    let home = std::env::temp_dir().join(format!(
        "openpencil-mcp-bootstrap-dsh-refresh-error-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&home);
    let profiles = home.join(".dsh/profiles");
    let desktop = profiles.join("desktop");
    std::fs::create_dir_all(&desktop).expect("create desktop profile");
    std::fs::write(desktop.join("package.json"), "{}").expect("desktop manifest");
    let configured = mcp_integrations::set_cli_enabled_at_home(
        op_editor_core::agent_settings::McpCli::Dsh,
        true,
        4500,
        &home,
    )
    .expect("seed valid integration");
    let original = std::fs::read(&configured).expect("existing integration");
    let broken = profiles.join("unreadable");
    std::fs::create_dir_all(&broken).expect("create unrelated profile");
    std::fs::write(broken.join("package.json"), [0xff, 0xfe]).expect("unreadable UTF-8 manifest");

    let mut app = DesktopApp::new(None);
    app.mcp_integrations_home = Some(home.clone());
    let settings = &mut app.host.editor_state_mut().editor_ui.agent_settings;
    settings.mcp_server.port = 0;
    settings.mcp_server.running = false;
    assert!(app.bootstrap_mcp_runtime_from_settings());

    let dsh_index = op_editor_core::agent_settings::McpCli::ALL
        .iter()
        .position(|cli| *cli == op_editor_core::agent_settings::McpCli::Dsh)
        .expect("DSH index");
    assert!(
        app.host
            .editor_state()
            .editor_ui
            .agent_settings
            .mcp_cli_enabled[dsh_index]
    );
    assert!(app.mcp_server_active());
    assert_eq!(
        std::fs::read(&configured).expect("untouched integration"),
        original
    );
    assert!(!home.join(".dsh/cordis.patch.yml").exists());
    drop(app);
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
    mcp_integrations::set_cli_enabled_at_home(
        op_editor_core::agent_settings::McpCli::Codex,
        true,
        busy_port,
        &home,
    )
    .expect("seed enabled Codex integration");

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
