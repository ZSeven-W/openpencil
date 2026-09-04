use super::*;

#[test]
fn json_escape_handles_quotes_backslash_control() {
    assert_eq!(json_escape(r#"a"b\c"#), r#"a\"b\\c"#);
    assert_eq!(json_escape("line\nbreak"), "line\\nbreak");
    assert_eq!(json_escape("tab\there"), "tab\\there");
    assert_eq!(json_escape("\u{0001}"), "\\u0001");
}

#[test]
fn args_to_json_builds_string_valued_object() {
    assert_eq!(args_to_json(&[]), "{}");
    let pairs = vec![
        ("kind".to_string(), "rect".to_string()),
        ("x".to_string(), "10".to_string()),
    ];
    assert_eq!(args_to_json(&pairs), r#"{"kind":"rect","x":"10"}"#);
}

#[test]
fn args_to_json_escapes_values() {
    let pairs = vec![("name".to_string(), r#"a"b"#.to_string())];
    assert_eq!(args_to_json(&pairs), r#"{"name":"a\"b"}"#);
}

#[test]
fn tool_call_body_wraps_name_and_arguments() {
    let body = tool_call_body("insert_node", r#"{"kind":"rect"}"#);
    assert!(body.contains(r#""method":"tools/call""#));
    assert!(body.contains(r#""name":"insert_node""#));
    assert!(body.contains(r#""arguments":{"kind":"rect"}"#));
}

#[test]
fn tools_list_body_is_a_tools_list_request() {
    assert!(tools_list_body().contains(r#""method":"tools/list""#));
}

#[test]
fn default_port_matches_ts_mcp_http_port() {
    assert_eq!(DEFAULT_PORT, 3100);
    assert!(USAGE.contains("127.0.0.1:3100/mcp"));
}

#[test]
fn parse_args_maps_ts_status_to_local_status_probe() {
    let p = parse_args(&["status".to_string()]).expect("parse status");
    assert_eq!(p.command, Command::Status);
}

#[test]
fn parse_args_without_port_is_not_explicit_so_discovery_runs() {
    // No `--port` ⇒ server-bound commands resolve the live editor's port
    // from ~/.openpencil/.op-mcp-port instead of pinning the default.
    let p = parse_args(&["status".to_string()]).expect("parse status");
    assert_eq!(p.port, DEFAULT_PORT);
    assert!(!p.port_explicit);
}

#[test]
fn parse_args_maps_stop_to_rust_mcp_stop() {
    let p = parse_args(&["stop".to_string()]).expect("parse stop");
    assert_eq!(p.command, Command::StopMcp);
}

#[test]
fn parse_args_maps_install_and_uninstall_targets() {
    let install = parse_args(&[
        "install".to_string(),
        "--target".to_string(),
        "codex".to_string(),
    ])
    .expect("parse install");
    assert_eq!(
        install.command,
        Command::InstallSkill {
            target: Some("codex".to_string()),
        }
    );

    let uninstall = parse_args(&[
        "uninstall".to_string(),
        "--target".to_string(),
        "opencode".to_string(),
    ])
    .expect("parse uninstall");
    assert_eq!(
        uninstall.command,
        Command::UninstallSkill {
            target: Some("opencode".to_string()),
        }
    );
}

#[test]
fn start_document_file_is_minimal_op_when_missing() {
    let dir = std::env::temp_dir().join(format!("op-cli-start-doc-{}", std::process::id()));
    let path = dir.join("nested").join("session.op");
    let _ = std::fs::remove_dir_all(&dir);

    app_control_cli::ensure_document_file(&path).expect("ensure document file");

    assert_eq!(
        std::fs::read_to_string(&path).expect("read document"),
        format!(
            "{{\n  \"version\": \"{}\",\n  \"name\": \"OpenPencil CLI Session\",\n  \"children\": []\n}}\n",
            env!("CARGO_PKG_VERSION")
        )
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn install_codex_writes_bundle_and_uninstall_removes_it() {
    let home = std::env::temp_dir().join(format!("op-cli-skill-codex-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).expect("temp home");

    skill_install_cli::install_target_at_home("codex", &home).expect("install codex");

    assert!(home
        .join(".codex/openpencil-skill/skills/openpencil-design/SKILL.md")
        .is_file());
    assert!(home.join(".agents/skills/openpencil-skill").exists());

    skill_install_cli::uninstall_target_at_home("codex", &home).expect("uninstall codex");

    assert!(!home.join(".codex/openpencil-skill").exists());
    assert!(!home.join(".agents/skills/openpencil-skill").exists());
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn install_opencode_writes_scanned_skills_dir_and_prunes_legacy_plugin_entry() {
    let home = std::env::temp_dir().join(format!("op-cli-skill-opencode-{}", std::process::id()));
    let config = home.join(".config/opencode/opencode.json");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(config.parent().unwrap()).expect("config dir");
    // Seed a config carrying the legacy no-op plugin entry plus a foreign one.
    std::fs::write(
        &config,
        r#"{"plugin":["other@file","openpencil-skill@git+https://github.com/zseven-w/openpencil-skill.git"]}"#,
    )
    .expect("seed config");

    // A stale plain file squatting on the discovery entry must be replaced,
    // not silently kept (it would make install report success while opencode
    // discovers nothing).
    let link = home.join(".config/opencode/skills/openpencil-skill");
    std::fs::create_dir_all(link.parent().unwrap()).expect("skills dir");
    std::fs::write(&link, "stale").expect("seed stale entry");

    skill_install_cli::install_target_at_home("opencode", &home).expect("install opencode");
    skill_install_cli::install_target_at_home("opencode", &home).expect("install is idempotent");

    // The skill lands where opencode's scanner looks:
    // <config-dir>/skills/**/SKILL.md (symlink into the bundle copy).
    assert!(
        link.join("openpencil-design/SKILL.md").exists(),
        "SKILL.md must be reachable under the scanned skills dir"
    );
    assert!(home
        .join(".config/opencode/openpencil-skill/skills/openpencil-design/SKILL.md")
        .exists());

    // The legacy plugin entry is pruned; foreign entries survive.
    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config).expect("read config"))
            .expect("json config");
    assert_eq!(installed["plugin"], serde_json::json!(["other@file"]));

    skill_install_cli::uninstall_target_at_home("opencode", &home).expect("uninstall opencode");
    assert!(std::fs::symlink_metadata(&link).is_err());
    assert!(!home.join(".config/opencode/openpencil-skill").exists());
    let uninstalled: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config).expect("read config"))
            .expect("json config");
    assert_eq!(uninstalled["plugin"], serde_json::json!(["other@file"]));
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn status_json_matches_ts_running_shape_without_requiring_server() {
    assert_eq!(
        status_json_from_running(DEFAULT_PORT, false),
        r#"{"running":false}"#
    );
    assert_eq!(
        status_json_from_running(3101, true),
        r#"{"running":true,"port":3101,"url":"http://127.0.0.1:3101"}"#
    );
}

#[test]
fn http_request_targets_mcp_endpoint() {
    let request = http_request("{}");
    assert!(request.starts_with("POST /mcp HTTP/1.1\r\n"));
}

#[test]
fn parse_args_maps_ts_get_id_alias_to_rust_tool() {
    let args = vec!["get".to_string(), "--id".to_string(), "n42".to_string()];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "batch_get".to_string(),
            args: vec![("nodeIds".to_string(), r#"["n42"]"#.to_string())],
        }
    );
}

#[test]
fn parse_args_maps_ts_get_parent_name_to_batch_get() {
    let args = vec![
        "get".to_string(),
        "--parent".to_string(),
        "n10".to_string(),
        "--name".to_string(),
        "Button".to_string(),
        "--depth".to_string(),
        "0".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "batch_get".to_string(),
            args: vec![
                ("patterns".to_string(), r#"[{"name":"Button"}]"#.to_string(),),
                ("parentId".to_string(), "n10".to_string()),
                ("readDepth".to_string(), "0".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_ts_get_type_id_depth_page_to_batch_get() {
    let args = vec![
        "get".to_string(),
        "--id".to_string(),
        "n11".to_string(),
        "--type".to_string(),
        "text".to_string(),
        "--depth".to_string(),
        "2".to_string(),
        "--page".to_string(),
        "p1".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "batch_get".to_string(),
            args: vec![
                ("nodeIds".to_string(), r#"["n11"]"#.to_string()),
                ("patterns".to_string(), r#"[{"type":"text"}]"#.to_string(),),
                ("readDepth".to_string(), "2".to_string()),
                ("pageId".to_string(), "p1".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_ts_open_to_open_document() {
    let p = parse_args(&["open".to_string()]).expect("parse open");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "open_document".to_string(),
            args: vec![],
        }
    );

    let with_path =
        parse_args(&["open".to_string(), "/tmp/design.op".to_string()]).expect("parse open path");
    let design_path = resolve_file_path_arg("/tmp/design.op");
    assert_eq!(
        with_path.command,
        Command::ToolCall {
            tool: "open_document".to_string(),
            args: vec![("filePath".to_string(), design_path)],
        }
    );
}

#[test]
fn parse_args_maps_ts_save_to_save_document_tool() {
    let p = parse_args(&["save".to_string(), "/tmp/copy.op".to_string()]).expect("parse save");
    let copy_path = resolve_file_path_arg("/tmp/copy.op");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "save_document".to_string(),
            args: vec![("filePath".to_string(), copy_path)],
        }
    );
}

#[test]
fn parse_args_maps_ts_save_file_flag_to_source_document() {
    let p = parse_args(&[
        "save".to_string(),
        "/tmp/copy.op".to_string(),
        "--file".to_string(),
        "/tmp/source.op".to_string(),
    ])
    .expect("parse save --file");
    let copy_path = resolve_file_path_arg("/tmp/copy.op");
    let source_path = resolve_file_path_arg("/tmp/source.op");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "save_document".to_string(),
            args: vec![
                ("filePath".to_string(), copy_path),
                ("sourceFilePath".to_string(), source_path),
            ],
        }
    );
}

#[test]
fn parse_args_maps_ts_page_list_alias_to_rust_tool() {
    let args = vec!["page".to_string(), "list".to_string()];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "list_pages".to_string(),
            args: vec![],
        }
    );
}

#[test]
fn parse_args_maps_page_add_name_alias_to_rust_tool() {
    let args = vec![
        "page".to_string(),
        "add".to_string(),
        "--name".to_string(),
        "Checkout".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "add_page".to_string(),
            args: vec![("name".to_string(), "Checkout".to_string())],
        }
    );
}

#[test]
fn parse_args_maps_page_remove_to_ts_remove_page_alias() {
    let args = vec!["page".to_string(), "remove".to_string(), "n2".to_string()];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "remove_page".to_string(),
            args: vec![("pageId".to_string(), "n2".to_string())],
        }
    );
}

#[test]
fn parse_args_maps_page_rename_to_ts_page_id_shape() {
    let args = vec![
        "page".to_string(),
        "rename".to_string(),
        "n2".to_string(),
        "Checkout".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "rename_page".to_string(),
            args: vec![
                ("pageId".to_string(), "n2".to_string()),
                ("name".to_string(), "Checkout".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_page_reorder_to_ts_page_id_shape() {
    let args = vec![
        "page".to_string(),
        "reorder".to_string(),
        "n2".to_string(),
        "0".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "reorder_page".to_string(),
            args: vec![
                ("pageId".to_string(), "n2".to_string()),
                ("index".to_string(), "0".to_string()),
            ],
        }
    );
}
