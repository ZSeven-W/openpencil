use super::*;

fn test_dir(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "openpencil-{label}-{}-{}",
        std::process::id(),
        TURN_SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn seed_antigravity_mcp(home: &Path, value: serde_json::Value) {
    let path = home.join(".gemini/config/mcp_config.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    let record_path = home.join(".openpencil/.op-mcp-port");
    fs::create_dir_all(record_path.parent().unwrap()).unwrap();
    fs::write(
        record_path,
        serde_json::to_vec(&serde_json::json!({
            "port": 3100,
            "writerPid": std::process::id(),
            "transport": "json-rpc",
            "token": "must-not-copy"
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn agent_turn_uses_private_home_and_minimal_mcp_policy() {
    let host_home = test_dir("agy-host-home");
    seed_antigravity_mcp(
        &host_home,
        serde_json::json!({
            "mcpServers": {
                "openpencil": {
                    "serverUrl": "http://127.0.0.1:3100/mcp",
                    "headers": {"Authorization": "must-not-copy"}
                },
                "other": {"serverUrl": "https://example.com/mcp"}
            }
        }),
    );
    assert!(validate_live_mcp_record(&host_home, 3101).is_err());
    let turn = IsolatedTurn::prepare_with_host_home(
        Some(CliName::Antigravity),
        "design a screen",
        &[],
        Some(&host_home),
    )
    .unwrap()
    .unwrap();
    let private_home = turn.home_dir().unwrap().to_path_buf();
    assert!(private_home.starts_with(turn.cwd()));
    let mut cli_args = Vec::new();
    turn.append_cli_args(&mut cli_args);
    assert_eq!(
        cli_args,
        [
            format!(
                "--gemini_dir={}",
                private_home.join(".gemini").to_string_lossy()
            ),
            "--app_data_dir=antigravity-cli".to_string()
        ]
    );

    let mcp: serde_json::Value = serde_json::from_slice(
        &fs::read(private_home.join(".gemini/config/mcp_config.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        mcp,
        serde_json::json!({"mcpServers": {"openpencil": {
            "serverUrl": "http://127.0.0.1:3100/mcp"
        }}})
    );
    let settings: serde_json::Value = serde_json::from_slice(
        &fs::read(private_home.join(".gemini/antigravity-cli/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(settings["toolPermission"], "strict");
    assert_eq!(settings["allowNonWorkspaceAccess"], false);
    assert_eq!(settings["enableTerminalSandbox"], true);
    assert_eq!(
        settings["permissions"]["allow"],
        serde_json::json!([ANTIGRAVITY_MCP_PERMISSION])
    );
    assert_eq!(
        settings["permissions"]["deny"],
        serde_json::json!(ANTIGRAVITY_DENY_RULES)
    );

    let mut env = vec![
        ("HOME".into(), "/host/home".into()),
        ("PATH".into(), "/host/bin".into()),
        ("APPDATA".into(), "/host/appdata".into()),
        ("TMPDIR".into(), "/host/tmp".into()),
        ("GOOGLE_API_KEY".into(), "provider-auth".into()),
    ];
    append_isolated_env(&mut env, Some(&turn));
    assert!(env
        .iter()
        .any(|(key, value)| key == "HOME" && value == "/host/home"));
    assert!(env
        .iter()
        .any(|(key, value)| key == "APPDATA" && value == "/host/appdata"));
    assert!(!env
        .iter()
        .any(|(key, value)| key == "PATH" && value == "/host/bin"));
    assert!(env.iter().any(|(key, value)| {
        key == "TMPDIR" && value == &private_home.join("tmp").to_string_lossy()
    }));
    assert!(env
        .iter()
        .any(|(key, value)| key == "GOOGLE_API_KEY" && value == "provider-auth"));
    let cwd = turn.cwd().to_path_buf();
    drop(turn);
    assert!(!cwd.exists());
    let _ = fs::remove_dir_all(host_home);
}

#[test]
fn agent_turn_rejects_non_loopback_or_disabled_mcp() {
    for server in [
        serde_json::json!({"serverUrl": "https://example.com/mcp"}),
        serde_json::json!({"serverUrl": "http://127.0.0.1:3100/mcp", "disabled": true}),
    ] {
        let host_home = test_dir("agy-unsafe-home");
        seed_antigravity_mcp(
            &host_home,
            serde_json::json!({"mcpServers": {"openpencil": server}}),
        );
        let error = IsolatedTurn::prepare_with_host_home(
            Some(CliName::Antigravity),
            "unsafe config must fail",
            &[],
            Some(&host_home),
        )
        .err()
        .expect("unsafe config should fail");
        assert!(matches!(error.kind(), io::ErrorKind::PermissionDenied));
        let _ = fs::remove_dir_all(host_home);
    }
}

#[test]
fn generation_turn_uses_empty_mcp_policy_without_host_config() {
    let turn = IsolatedTurn::prepare_for(
        Some(CliName::Antigravity),
        "return only JavaScript",
        &[],
        TurnPurpose::Generation,
        None,
    )
    .unwrap()
    .unwrap();
    let home = turn.home_dir().unwrap();
    let mcp: serde_json::Value =
        serde_json::from_slice(&fs::read(home.join(".gemini/config/mcp_config.json")).unwrap())
            .unwrap();
    let settings: serde_json::Value = serde_json::from_slice(
        &fs::read(home.join(".gemini/antigravity-cli/settings.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(mcp, serde_json::json!({"mcpServers": {}}));
    assert_eq!(settings["permissions"]["allow"], serde_json::json!([]));
    assert_eq!(
        settings["permissions"]["deny"],
        serde_json::json!(ANTIGRAVITY_DENY_RULES)
    );
    assert_eq!(turn.prompt(), "return only JavaScript");
}
