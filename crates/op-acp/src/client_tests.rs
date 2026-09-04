use super::*;
use crate::protocol::{METHOD_SESSION_CLOSE, METHOD_SESSION_DELETE};
use crate::transport::read_frame;

async fn mock_agent(read: impl AsyncRead + Unpin, mut write: impl AsyncWrite + Unpin) {
    let mut buf = BufReader::new(read);
    while let Ok(Some(frame)) = read_frame(&mut buf).await {
        let id = frame.get("id").cloned().unwrap_or(Value::Null);
        let method = frame.get("method").and_then(Value::as_str).unwrap_or("");
        let result = match method {
            "initialize" => serde_json::json!({
                "protocolVersion": 1,
                "agentCapabilities": { "mcpCapabilities": { "http": true } },
                "authMethods": [{ "id": "login", "name": "Agent login" }],
                "agentInfo": { "name": "Mock Agent", "version": "9.9" }
            }),
            "session/new" => serde_json::json!({
                "sessionId": "sess-1",
                "configOptions": [{
                    "id": "model",
                    "name": "Model",
                    "category": "model",
                    "type": "select",
                    "currentValue": "model-a",
                    "options": [
                        { "value": "model-a", "name": "Model A" },
                        { "value": "model-b", "name": "Model B" }
                    ]
                }]
            }),
            "session/set_config_option" => {
                assert_eq!(frame["params"]["sessionId"], "sess-1");
                assert_eq!(frame["params"]["configId"], "model");
                assert_eq!(frame["params"]["value"], "model-b");
                assert!(frame["params"].get("type").is_none());
                serde_json::json!({
                    "configOptions": [{
                        "id": "model",
                        "name": "Model",
                        "category": "model",
                        "type": "select",
                        "currentValue": "model-b",
                        "options": [{ "value": "model-b", "name": "Model B" }]
                    }]
                })
            }
            "session/prompt" => {
                let note = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "session/update",
                    "params": {
                        "sessionId": "sess-1",
                        "update": {
                            "sessionUpdate": "agent_message_chunk",
                            "content": { "type": "text", "text": "hi there" }
                        }
                    }
                });
                write_frame(&mut write, &note).await.unwrap();
                serde_json::json!({ "stopReason": "max_tokens" })
            }
            _ => break,
        };
        let response = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result });
        write_frame(&mut write, &response).await.unwrap();
    }
}

fn mock_connection() -> (
    AcpConnection,
    impl std::future::Future<Output = ()> + Send + 'static,
) {
    let (client_write, agent_read) = tokio::io::duplex(8192);
    let (agent_write, client_read) = tokio::io::duplex(8192);
    (
        AcpConnection::new(client_read, client_write, None),
        mock_agent(agent_read, agent_write),
    )
}

#[tokio::test]
async fn official_v1_handshake_session_config_and_stop_reason_are_retained() {
    let (mut conn, agent) = mock_connection();
    tokio::spawn(agent);
    let mut notes = conn.take_notifications().expect("notifications");

    conn.initialize("fallback").await.expect("initialize");
    assert_eq!(conn.protocol_version(), ProtocolVersion::V1);
    assert!(conn.agent_capabilities().mcp_capabilities.http);
    assert_eq!(conn.auth_methods().len(), 1);
    assert_eq!(conn.auth_methods()[0].id().to_string(), "login");
    assert_eq!(conn.agent_info().name, "Mock Agent");
    assert_eq!(conn.agent_info().version.as_deref(), Some("9.9"));

    let mut session = conn.new_session().await.expect("new_session");
    assert_eq!(session.session_id, "sess-1");
    assert_eq!(session.config_options.len(), 1);
    session.config_options = conn
        .set_session_config_option(
            &session.session_id,
            "model",
            SessionConfigOptionValue::value_id("model-b"),
        )
        .await
        .expect("set config");
    assert_eq!(session.config_options.len(), 1);

    let stop = conn
        .prompt(&session.session_id, "design a button")
        .await
        .expect("prompt");
    assert_eq!(stop, AcpStopReason::MaxTokens);
    assert_eq!(
        notes
            .recv()
            .await
            .expect("session/update")
            .session_id
            .as_deref(),
        Some("sess-1")
    );
}

#[tokio::test]
async fn unsupported_protocol_version_is_rejected() {
    let (client_write, agent_read) = tokio::io::duplex(2048);
    let (mut agent_write, client_read) = tokio::io::duplex(2048);
    tokio::spawn(async move {
        let frame = read_frame(&mut BufReader::new(agent_read))
            .await
            .unwrap()
            .unwrap();
        write_frame(
            &mut agent_write,
            &serde_json::json!({
                "jsonrpc": "2.0",
                "id": frame["id"],
                "result": { "protocolVersion": 2 }
            }),
        )
        .await
        .unwrap();
    });
    let mut conn = AcpConnection::new(client_read, client_write, None);
    let error = conn.initialize("fallback").await.unwrap_err();
    assert!(error.to_string().contains("unsupported protocol version 2"));
}

#[tokio::test]
async fn malformed_ndjson_fails_the_in_flight_handshake() {
    let (client_write, mut agent_read) = tokio::io::duplex(2048);
    let (mut agent_write, client_read) = tokio::io::duplex(2048);
    tokio::spawn(async move {
        let _request = read_frame(&mut BufReader::new(&mut agent_read))
            .await
            .unwrap()
            .unwrap();
        use tokio::io::AsyncWriteExt;
        agent_write.write_all(b"this is not json\n").await.unwrap();
    });
    let mut conn = AcpConnection::new(client_read, client_write, None);
    let error = conn.initialize("fallback").await.unwrap_err();
    assert!(error.to_string().contains("invalid ACP JSON frame"));
}

#[tokio::test]
async fn notification_overflow_fails_the_prompt_instead_of_reporting_success() {
    let (client_write, agent_read) = tokio::io::duplex(1024 * 1024);
    let (mut agent_write, client_read) = tokio::io::duplex(1024 * 1024);
    tokio::spawn(async move {
        let mut reader = BufReader::new(agent_read);
        for (expected, result) in [
            ("initialize", serde_json::json!({ "protocolVersion": 1 })),
            ("session/new", serde_json::json!({ "sessionId": "flood" })),
        ] {
            let request = read_frame(&mut reader).await.unwrap().unwrap();
            assert_eq!(request["method"], expected);
            write_frame(
                &mut agent_write,
                &serde_json::json!({
                    "jsonrpc": "2.0", "id": request["id"], "result": result
                }),
            )
            .await
            .unwrap();
        }
        let prompt = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(prompt["method"], "session/prompt");
        for index in 0..=NOTIFICATION_CAPACITY {
            write_frame(
                &mut agent_write,
                &serde_json::json!({
                    "jsonrpc": "2.0", "method": "session/update",
                    "params": {
                        "sessionId": "flood",
                        "update": {
                            "sessionUpdate": "agent_message_chunk",
                            "content": { "type": "text", "text": index.to_string() }
                        }
                    }
                }),
            )
            .await
            .unwrap();
        }
    });
    let mut conn = AcpConnection::new(client_read, client_write, None);
    conn.initialize("fallback").await.unwrap();
    let session = conn.new_session().await.unwrap();
    let _undrained_notifications = conn.take_notifications().unwrap();
    let error = conn.prompt(&session.session_id, "flood").await.unwrap_err();
    assert!(error.to_string().contains("queue overflow"), "{error}");
}

#[tokio::test]
async fn session_cancel_is_a_notification_without_an_id() {
    let (client_write, agent_read) = tokio::io::duplex(2048);
    let (_agent_write, client_read) = tokio::io::duplex(2048);
    let received = tokio::spawn(async move {
        read_frame(&mut BufReader::new(agent_read))
            .await
            .unwrap()
            .unwrap()
    });
    let conn = AcpConnection::new(client_read, client_write, None);
    conn.cancel_session("sess-cancel").await.expect("cancel");
    let frame = received.await.unwrap();
    assert_eq!(frame["method"], METHOD_SESSION_CANCEL);
    assert_eq!(frame["params"]["sessionId"], "sess-cancel");
    assert!(frame.get("id").is_none());
}

async fn mock_agent_checking_session_new(
    read: impl AsyncRead + Unpin,
    mut write: impl AsyncWrite + Unpin,
) {
    let mut buf = BufReader::new(read);
    while let Ok(Some(frame)) = read_frame(&mut buf).await {
        let id = frame["id"].clone();
        let result = match frame["method"].as_str().unwrap_or_default() {
            "initialize" => serde_json::json!({
                "protocolVersion": 1,
                "agentCapabilities": { "mcpCapabilities": { "http": true } }
            }),
            "session/new" => {
                let params = &frame["params"];
                let server = &params["mcpServers"][0];
                let ok = server["name"] == "openpencil"
                    && server["type"] == "http"
                    && server["url"] == "http://127.0.0.1:3100/mcp"
                    && server["headers"].as_array().is_some_and(Vec::is_empty)
                    && params["_meta"]["systemPrompt"] == "use the canvas tools"
                    && params["cwd"].as_str().is_some_and(|cwd| !cwd.is_empty());
                serde_json::json!({ "sessionId": if ok { "sess-mcp-ok" } else { "sess-bad" } })
            }
            _ => break,
        };
        write_frame(
            &mut write,
            &serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        )
        .await
        .unwrap();
    }
}

#[tokio::test]
async fn session_new_carries_mcp_servers_and_system_prompt_meta() {
    let (client_write, agent_read) = tokio::io::duplex(8192);
    let (agent_write, client_read) = tokio::io::duplex(8192);
    tokio::spawn(mock_agent_checking_session_new(agent_read, agent_write));
    let mut conn = AcpConnection::new(client_read, client_write, None);
    conn.initialize("fallback").await.unwrap();
    let session = conn
        .new_session_with(&NewSessionOptions {
            mcp_servers: vec![McpHttpServer {
                name: "openpencil".into(),
                url: "http://127.0.0.1:3100/mcp".into(),
            }],
            system_prompt_meta: Some("use the canvas tools".into()),
        })
        .await
        .unwrap();
    assert_eq!(session.session_id, "sess-mcp-ok");
}

#[tokio::test]
async fn http_mcp_requires_an_advertised_capability() {
    let (client_write, _agent_read) = tokio::io::duplex(2048);
    let (_agent_write, client_read) = tokio::io::duplex(2048);
    let conn = AcpConnection::new(client_read, client_write, None);
    let error = conn
        .new_session_with(&NewSessionOptions {
            mcp_servers: vec![McpHttpServer {
                name: "openpencil".into(),
                url: "http://127.0.0.1:3100/mcp".into(),
            }],
            system_prompt_meta: None,
        })
        .await
        .unwrap_err();
    assert!(error.to_string().contains("did not advertise"));
}

#[tokio::test]
async fn session_close_and_delete_require_capabilities_and_use_stable_v1_wire_methods() {
    let (client_write, agent_read) = tokio::io::duplex(4096);
    let (mut agent_write, client_read) = tokio::io::duplex(4096);
    let agent = tokio::spawn(async move {
        let mut read = BufReader::new(agent_read);
        let initialize = read_frame(&mut read).await.unwrap().unwrap();
        write_frame(
            &mut agent_write,
            &serde_json::json!({
                "jsonrpc": "2.0", "id": initialize["id"],
                "result": {
                    "protocolVersion": 1,
                    "agentCapabilities": {
                        "sessionCapabilities": { "close": {}, "delete": {} }
                    }
                }
            }),
        )
        .await
        .unwrap();
        let mut methods = Vec::new();
        for expected in [METHOD_SESSION_CLOSE, METHOD_SESSION_DELETE] {
            let request = read_frame(&mut read).await.unwrap().unwrap();
            assert_eq!(request["method"], expected);
            assert_eq!(request["params"]["sessionId"], "session-lifecycle");
            methods.push(request["method"].as_str().unwrap().to_string());
            write_frame(
                &mut agent_write,
                &serde_json::json!({
                    "jsonrpc": "2.0", "id": request["id"], "result": {}
                }),
            )
            .await
            .unwrap();
        }
        methods
    });
    let mut conn = AcpConnection::new(client_read, client_write, None);
    conn.initialize("fallback").await.unwrap();
    assert!(conn.supports_session_close());
    assert!(conn.supports_session_delete());
    assert!(conn
        .close_session_if_supported("session-lifecycle")
        .await
        .unwrap());
    assert!(conn
        .delete_session_if_supported("session-lifecycle")
        .await
        .unwrap());
    assert_eq!(
        agent.await.unwrap(),
        [METHOD_SESSION_CLOSE, METHOD_SESSION_DELETE]
    );
}

#[tokio::test]
async fn absent_session_lifecycle_capabilities_emit_no_wire_request() {
    let (client_write, mut agent_read) = tokio::io::duplex(1024);
    let (_agent_write, client_read) = tokio::io::duplex(1024);
    let conn = AcpConnection::new(client_read, client_write, None);
    assert!(!conn
        .close_session_if_supported("unsupported")
        .await
        .unwrap());
    assert!(!conn
        .delete_session_if_supported("unsupported")
        .await
        .unwrap());
    assert!(
        tokio::time::timeout(
            Duration::from_millis(50),
            read_frame(&mut BufReader::new(&mut agent_read))
        )
        .await
        .is_err(),
        "an unadvertised lifecycle method reached the wire"
    );
}

#[tokio::test]
async fn auth_required_retries_session_new_with_the_only_advertised_method() {
    let (client_write, agent_read) = tokio::io::duplex(4096);
    let (mut agent_write, client_read) = tokio::io::duplex(4096);
    let agent = tokio::spawn(async move {
        let mut read = BufReader::new(agent_read);
        for expected in ["initialize", "session/new", "authenticate", "session/new"] {
            let request = read_frame(&mut read).await.unwrap().unwrap();
            assert_eq!(request["method"], expected);
            let response = match expected {
                "initialize" => serde_json::json!({
                    "jsonrpc": "2.0", "id": request["id"],
                    "result": {
                        "protocolVersion": 1,
                        "authMethods": [{ "id": "browser-login", "name": "Browser login" }]
                    }
                }),
                "session/new" if request["id"] == 2 => serde_json::json!({
                    "jsonrpc": "2.0", "id": request["id"],
                    "error": { "code": -32000, "message": "authentication required" }
                }),
                "authenticate" => {
                    assert_eq!(request["params"]["methodId"], "browser-login");
                    serde_json::json!({
                        "jsonrpc": "2.0", "id": request["id"], "result": {}
                    })
                }
                "session/new" => serde_json::json!({
                    "jsonrpc": "2.0", "id": request["id"],
                    "result": { "sessionId": "authenticated-session" }
                }),
                _ => unreachable!(),
            };
            write_frame(&mut agent_write, &response).await.unwrap();
        }
    });
    let mut conn = AcpConnection::new(client_read, client_write, None);
    conn.initialize("fallback").await.unwrap();
    let session = conn.new_session().await.unwrap();
    assert_eq!(session.session_id, "authenticated-session");
    agent.await.unwrap();
}

#[tokio::test]
async fn auth_required_does_not_guess_between_multiple_methods() {
    let (client_write, agent_read) = tokio::io::duplex(4096);
    let (mut agent_write, client_read) = tokio::io::duplex(4096);
    let agent = tokio::spawn(async move {
        let mut read = BufReader::new(agent_read);
        let initialize = read_frame(&mut read).await.unwrap().unwrap();
        write_frame(
            &mut agent_write,
            &serde_json::json!({
                "jsonrpc": "2.0", "id": initialize["id"],
                "result": {
                    "protocolVersion": 1,
                    "authMethods": [
                        { "id": "login-a", "name": "Login A" },
                        { "id": "login-b", "name": "Login B" }
                    ]
                }
            }),
        )
        .await
        .unwrap();
        let session = read_frame(&mut read).await.unwrap().unwrap();
        write_frame(
            &mut agent_write,
            &serde_json::json!({
                "jsonrpc": "2.0", "id": session["id"],
                "error": { "code": -32000, "message": "authentication required" }
            }),
        )
        .await
        .unwrap();
        if let Ok(Ok(Some(frame))) =
            tokio::time::timeout(Duration::from_millis(100), read_frame(&mut read)).await
        {
            panic!("unexpected auth request: {frame}");
        }
    });
    let mut conn = AcpConnection::new(client_read, client_write, None);
    conn.initialize("fallback").await.unwrap();
    let error = conn.new_session().await.unwrap_err();
    assert!(error.to_string().contains("authentication method picker"));
    agent.await.unwrap();
}

#[tokio::test]
async fn in_flight_call_fails_fast_when_agent_exits() {
    let (client_write, agent_read) = tokio::io::duplex(1024);
    let (agent_write, client_read) = tokio::io::duplex(1024);
    drop(agent_read);
    drop(agent_write);
    let mut conn = AcpConnection::new(client_read, client_write, None);
    let started = std::time::Instant::now();
    assert!(matches!(
        conn.initialize("fallback").await,
        Err(AcpError::Closed)
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[cfg(unix)]
#[tokio::test]
async fn cancelling_shutdown_keeps_the_child_owned_for_drop_cleanup() {
    let mut command = Command::new("sh");
    command
        .args(["-c", "trap '' TERM; printf 'ready\\n'; sleep 30 & wait"])
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command.spawn().expect("spawn stubborn ACP child");
    let stdout = child.stdout.take().expect("piped child stdout");
    let mut stdout = BufReader::new(stdout);
    let mut ready = String::new();
    stdout.read_line(&mut ready).await.expect("read readiness");
    assert_eq!(ready, "ready\n");

    let (client_write, agent_read) = tokio::io::duplex(256);
    let mut connection = AcpConnection::new(stdout, client_write, Some(child));
    let mut shutdown = Box::pin(connection.shutdown());
    assert!(
        tokio::time::timeout(Duration::from_millis(50), &mut shutdown)
            .await
            .is_err(),
        "the stubborn child should keep graceful shutdown pending"
    );
    drop(shutdown);

    assert!(
        connection.child.is_some(),
        "a cancelled shutdown future must leave the child for Drop/disconnect"
    );
    connection.disconnect();
    assert!(connection.child.is_none());
    drop(agent_read);
}

#[test]
fn local_environment_allowlist_excludes_credentials() {
    for allowed in [
        "PATH",
        "Path",
        "HOME",
        "HTTPS_PROXY",
        "LC_MESSAGES",
        "APPDATA",
        "ComSpec",
        "SystemRoot",
    ] {
        assert!(local_env_allowed(allowed), "{allowed}");
    }
    for rejected in [
        "ANTHROPIC_API_KEY",
        "OPENAI_API_KEY",
        "AWS_SECRET_ACCESS_KEY",
        "GITHUB_TOKEN",
    ] {
        assert!(!local_env_allowed(rejected), "{rejected}");
    }
}

#[cfg(unix)]
#[test]
fn configured_path_resolves_the_exact_agent_executable() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("op-acp-resolve-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let executable = dir.join("mock-acp");
    std::fs::write(&executable, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
    let env = BTreeMap::from([("PATH".into(), dir.to_string_lossy().into_owned())]);
    assert_eq!(
        resolve_local_command("mock-acp", &env),
        executable.to_string_lossy()
    );
    std::fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
fn stub_agent(body: &str) -> (PathBuf, AcpAgentConfig) {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "op-acp-stub-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("agent.sh");
    std::fs::write(&path, body).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    (
        dir,
        AcpAgentConfig {
            id: "stub".into(),
            display_name: "Stub Agent".into(),
            connection_type: ConnectionType::Local,
            command: Some(path.to_string_lossy().into_owned()),
            args: Vec::new(),
            env: BTreeMap::new(),
            url: None,
            enabled: true,
        },
    )
}

#[cfg(unix)]
async fn connect_stub_retry(config: &AcpAgentConfig) -> Result<AcpConnection, AcpError> {
    for attempt in 0..3 {
        match connect_acp_agent(config).await {
            Err(error) if attempt < 2 && error.to_string().contains("Text file busy") => {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            result => return result,
        }
    }
    unreachable!()
}

#[cfg(unix)]
#[tokio::test]
async fn failed_handshake_quotes_redacted_bounded_stderr() {
    let (dir, config) = stub_agent(
        "#!/bin/sh\necho 'fatal: ANTHROPIC_API_KEY=sk-fake-000111 rejected' >&2\necho 'see https://agent.test/setup?token=fake-token' >&2\nexit 1\n",
    );
    let error = match connect_stub_retry(&config).await {
        Err(error) => error,
        Ok(_) => panic!("stub must not connect"),
    };
    std::fs::remove_dir_all(dir).unwrap();
    let text = error.to_string();
    assert!(text.contains("rejected"), "{text}");
    assert!(text.contains("agent.test/setup?<redacted>"), "{text}");
    assert!(!text.contains("sk-fake-000111"), "{text}");
    assert!(!text.contains("token=fake-token"), "{text}");
    assert!(text.chars().count() <= 96 + op_util::cli_output::TAIL_MAX_CHARS);
}

#[cfg(unix)]
#[tokio::test]
async fn stderr_capture_stays_bounded_under_a_flood() {
    let (dir, config) = stub_agent(
        "#!/bin/sh\nawk 'BEGIN{for(i=0;i<200000;i++) print \"agent chatter line \" i > \"/dev/stderr\"}'\nexit 1\n",
    );
    let error = match connect_stub_retry(&config).await {
        Err(error) => error,
        Ok(_) => panic!("stub must not connect"),
    };
    std::fs::remove_dir_all(dir).unwrap();
    let text = error.to_string();
    assert!(
        text.chars().count() <= 96 + op_util::cli_output::TAIL_MAX_CHARS,
        "error message was {} chars: {text}",
        text.chars().count()
    );
    assert!(text.contains("agent chatter line 199999"), "{text}");
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn broken_agents_stderr_survives_concurrent_connects() {
    let (dir, config) =
        stub_agent("#!/bin/sh\necho 'fatal: agent config rejected by upstream' >&2\nexit 1\n");
    let mut lost = 0usize;
    let mut total = 0usize;
    for _round in 0..6 {
        let mut pending = Vec::new();
        for _ in 0..16 {
            let config = config.clone();
            pending.push(tokio::spawn(async move {
                match connect_stub_retry(&config).await {
                    Err(error) => error.to_string(),
                    Ok(_) => "unexpectedly connected".to_string(),
                }
            }));
        }
        for handle in pending {
            let text = handle.await.expect("probe task");
            total += 1;
            if !text.contains("agent config rejected by upstream") {
                lost += 1;
            }
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
    assert_eq!(
        lost, 0,
        "{lost} of {total} connect failures lost the agent's stderr"
    );
}

#[cfg(feature = "remote")]
#[test]
fn remote_connector_uses_explicit_rustls_configuration() {
    assert!(matches!(
        remote_rustls_connector(),
        tokio_tungstenite::Connector::Rustls(_)
    ));
    let config = remote_websocket_config();
    assert_eq!(config.max_message_size, Some(MAX_INBOUND_FRAME_BYTES));
    assert_eq!(config.max_frame_size, Some(MAX_INBOUND_FRAME_BYTES));
}
