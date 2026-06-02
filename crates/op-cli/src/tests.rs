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
fn parse_args_maps_start_to_rust_mcp_server() {
    let p = parse_args(&[
        "--port".to_string(),
        "3200".to_string(),
        "start".to_string(),
        "--file".to_string(),
        "/tmp/session.op".to_string(),
    ])
    .expect("parse start");
    assert_eq!(p.port, 3200);
    assert_eq!(
        p.command,
        Command::StartMcp {
            document_path: Some("/tmp/session.op".to_string()),
        }
    );
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
        "{\n  \"version\": \"0.8.0\",\n  \"name\": \"OpenPencil CLI Session\",\n  \"children\": []\n}\n"
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
fn install_opencode_updates_plugin_array_and_preserves_other_plugins() {
    let home = std::env::temp_dir().join(format!("op-cli-skill-opencode-{}", std::process::id()));
    let config = home.join(".config/opencode/opencode.json");
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(config.parent().unwrap()).expect("config dir");
    std::fs::write(&config, r#"{"plugin":["other@file"]}"#).expect("seed config");

    skill_install_cli::install_target_at_home("opencode", &home).expect("install opencode");
    skill_install_cli::install_target_at_home("opencode", &home).expect("install is idempotent");

    let installed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config).expect("read config"))
            .expect("json config");
    let plugins = installed["plugin"].as_array().expect("plugin array");
    assert_eq!(plugins.len(), 2);
    assert!(plugins.iter().any(|p| p == "other@file"));
    assert!(plugins
        .iter()
        .any(|p| p.as_str().is_some_and(|s| s.contains("openpencil-skill"))));

    skill_install_cli::uninstall_target_at_home("opencode", &home).expect("uninstall opencode");

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
    assert_eq!(
        with_path.command,
        Command::ToolCall {
            tool: "open_document".to_string(),
            args: vec![("filePath".to_string(), "/tmp/design.op".to_string())],
        }
    );
}

#[test]
fn parse_args_maps_ts_save_to_save_document_tool() {
    let p = parse_args(&["save".to_string(), "/tmp/copy.op".to_string()]).expect("parse save");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "save_document".to_string(),
            args: vec![("filePath".to_string(), "/tmp/copy.op".to_string())],
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

#[test]
fn parse_args_maps_page_duplicate_name_to_ts_page_id_shape() {
    let args = vec![
        "page".to_string(),
        "duplicate".to_string(),
        "n2".to_string(),
        "--name".to_string(),
        "Checkout copy".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "duplicate_page".to_string(),
            args: vec![
                ("pageId".to_string(), "n2".to_string()),
                ("name".to_string(), "Checkout copy".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_layout_flags_to_ts_snapshot_shape() {
    let args = vec![
        "layout".to_string(),
        "--parent".to_string(),
        "n10".to_string(),
        "--depth".to_string(),
        "2".to_string(),
        "--page".to_string(),
        "p1".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "snapshot_layout".to_string(),
            args: vec![
                ("parentId".to_string(), "n10".to_string()),
                ("maxDepth".to_string(), "2".to_string()),
                ("pageId".to_string(), "p1".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_find_space_to_ts_tool_shape() {
    let args = vec![
        "find-space".to_string(),
        "--direction".to_string(),
        "bottom".to_string(),
        "--width".to_string(),
        "320".to_string(),
        "--height".to_string(),
        "240".to_string(),
        "--page".to_string(),
        "p1".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "find_empty_space".to_string(),
            args: vec![
                ("direction".to_string(), "bottom".to_string()),
                ("width".to_string(), "320".to_string()),
                ("height".to_string(), "240".to_string()),
                ("pageId".to_string(), "p1".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_vars_set_to_ts_bulk_tool_shape() {
    let args = vec![
        "vars:set".to_string(),
        r##"{"brand":{"type":"color","value":"#ff0000"}}"##.to_string(),
        "--replace".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "set_variables".to_string(),
            args: vec![
                (
                    "variables".to_string(),
                    r##"{"brand":{"type":"color","value":"#ff0000"}}"##.to_string(),
                ),
                ("replace".to_string(), "true".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_themes_set_to_ts_bulk_tool_shape() {
    let args = vec![
        "themes:set".to_string(),
        r#"{"Mode":["Light","Dark"]}"#.to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "set_themes".to_string(),
            args: vec![(
                "themes".to_string(),
                r#"{"Mode":["Light","Dark"]}"#.to_string(),
            )],
        }
    );
}

#[test]
fn parse_args_maps_read_nodes_to_ts_tool_shape() {
    let args = vec![
        "read-nodes".to_string(),
        "n10,n11".to_string(),
        "--depth".to_string(),
        "0".to_string(),
        "--vars".to_string(),
        "--page".to_string(),
        "p1".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "read_nodes".to_string(),
            args: vec![
                ("nodeIds".to_string(), "n10,n11".to_string()),
                ("depth".to_string(), "0".to_string()),
                ("pageId".to_string(), "p1".to_string()),
                ("includeVariables".to_string(), "true".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_theme_preset_commands_to_ts_tool_shape() {
    let save = parse_args(&[
        "theme:save".to_string(),
        "/tmp/acme.optheme".to_string(),
        "--name".to_string(),
        "Acme".to_string(),
    ])
    .expect("parse save");
    assert_eq!(
        save.command,
        Command::ToolCall {
            tool: "save_theme_preset".to_string(),
            args: vec![
                ("presetPath".to_string(), "/tmp/acme.optheme".to_string()),
                ("name".to_string(), "Acme".to_string()),
            ],
        }
    );

    let load = parse_args(&["theme:load".to_string(), "/tmp/acme.optheme".to_string()])
        .expect("parse load");
    assert_eq!(
        load.command,
        Command::ToolCall {
            tool: "load_theme_preset".to_string(),
            args: vec![("presetPath".to_string(), "/tmp/acme.optheme".to_string())],
        }
    );

    let list = parse_args(&["theme:list".to_string(), "/tmp".to_string()]).expect("parse list");
    assert_eq!(
        list.command,
        Command::ToolCall {
            tool: "list_theme_presets".to_string(),
            args: vec![("directory".to_string(), "/tmp".to_string())],
        }
    );
}

#[test]
fn parse_args_maps_codegen_plan_to_structured_mcp_args() {
    let args = vec![
        "codegen:plan".to_string(),
        r#"{"chunks":[],"sharedStyles":[],"rootLayout":{"nodeId":"n1"}}"#.to_string(),
        "--file".to_string(),
        "/tmp/design.op".to_string(),
        "--page".to_string(),
        "page-1".to_string(),
    ];
    let p = parse_args(&args).expect("parse codegen plan");
    match p.command {
        Command::ToolCallJson { tool, args_json } => {
            assert_eq!(tool, "codegen_plan");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&args_json).expect("args json"),
                serde_json::json!({
                    "filePath": "/tmp/design.op",
                    "pageId": "page-1",
                    "plan": {
                        "chunks": [],
                        "sharedStyles": [],
                        "rootLayout": { "nodeId": "n1" },
                    },
                })
            );
        }
        other => panic!("expected structured codegen plan call, got {other:?}"),
    }
}

#[test]
fn parse_args_maps_codegen_submit_to_structured_mcp_args() {
    let args = vec![
        "codegen:submit".to_string(),
        "plan-1".to_string(),
        r#"{"chunkId":"hero","code":"export const Hero = () => null;","contract":{"provides":[],"requires":[]}}"#.to_string(),
    ];
    let p = parse_args(&args).expect("parse codegen submit");
    match p.command {
        Command::ToolCallJson { tool, args_json } => {
            assert_eq!(tool, "codegen_submit_chunk");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&args_json).expect("args json"),
                serde_json::json!({
                    "planId": "plan-1",
                    "result": {
                        "chunkId": "hero",
                        "code": "export const Hero = () => null;",
                        "contract": { "provides": [], "requires": [] },
                    },
                })
            );
        }
        other => panic!("expected structured codegen submit call, got {other:?}"),
    }
}

#[test]
fn parse_args_maps_codegen_assemble_and_clean_to_mcp_tools() {
    let assemble =
        parse_args(&["codegen:assemble".to_string(), "plan-1".to_string()]).expect("assemble");
    assert_eq!(
        assemble.command,
        Command::ToolCall {
            tool: "codegen_assemble".to_string(),
            args: vec![
                ("planId".to_string(), "plan-1".to_string()),
                ("framework".to_string(), "react".to_string()),
            ],
        }
    );

    let clean = parse_args(&["codegen:clean".to_string(), "plan-1".to_string()]).expect("clean");
    assert_eq!(
        clean.command,
        Command::ToolCall {
            tool: "codegen_clean".to_string(),
            args: vec![("planId".to_string(), "plan-1".to_string())],
        }
    );
}

#[test]
fn parse_args_maps_import_figma_to_direct_converter() {
    let p = parse_args(&[
        "import:figma".to_string(),
        "/tmp/source.fig".to_string(),
        "--out".to_string(),
        "/tmp/converted.op".to_string(),
    ])
    .expect("parse import:figma");
    assert_eq!(
        p.command,
        Command::ImportFigma {
            fig_path: "/tmp/source.fig".to_string(),
            out_path: "/tmp/converted.op".to_string(),
        }
    );
}

#[test]
fn figma_default_out_path_matches_ts_suffix_replacement() {
    assert_eq!(figma_default_out_path("checkout.fig"), "checkout.op");
    assert_eq!(
        figma_default_out_path("/tmp/checkout.fig"),
        "/tmp/checkout.op"
    );
    assert_eq!(
        figma_default_out_path("/tmp/checkout.FIG"),
        "/tmp/checkout.FIG"
    );
}

#[test]
fn parse_args_maps_ts_insert_json_alias_to_rust_tool() {
    let args = vec![
        "insert".to_string(),
        r##"{"type":"rectangle","name":"Card","x":12,"y":24,"width":200,"height":100,"fill":[{"type":"solid","color":"#ffffff"}]}"##.to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "insert_node".to_string(),
            args: vec![
                ("kind".to_string(), "rect".to_string()),
                ("name".to_string(), "Card".to_string()),
                ("x".to_string(), "12".to_string()),
                ("y".to_string(), "24".to_string()),
                ("width".to_string(), "200".to_string()),
                ("height".to_string(), "100".to_string()),
                ("fill_hex".to_string(), "#ffffff".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_defaults_port_and_reads_tool_call() {
    let args = vec![
        "insert_node".to_string(),
        "kind=rect".to_string(),
        "x=10".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(p.port, DEFAULT_PORT);
    match p.command {
        Command::ToolCall { tool, args } => {
            assert_eq!(tool, "insert_node");
            assert_eq!(args.len(), 2);
            assert_eq!(args[0], ("kind".to_string(), "rect".to_string()));
        }
        _ => panic!("expected ToolCall"),
    }
}

#[test]
fn parse_args_reads_explicit_port_anywhere() {
    let args = vec![
        "--port".to_string(),
        "9001".to_string(),
        "tools".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(p.port, 9001);
    assert!(matches!(p.command, Command::ToolsList));
}

#[test]
fn parse_args_reads_flag_arguments_for_generic_tools() {
    let args = vec![
        "get_node".to_string(),
        "--node_id".to_string(),
        "n1".to_string(),
    ];
    let p = parse_args(&args).expect("parse");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "get_node".to_string(),
            args: vec![("node_id".to_string(), "n1".to_string())],
        }
    );
}

#[test]
fn parse_args_rejects_non_kv_argument() {
    let args = vec!["insert_node".to_string(), "bogus".to_string()];
    assert!(parse_args(&args).is_err());
}

#[test]
fn parse_args_rejects_bad_port() {
    let args = vec![
        "--port".to_string(),
        "notnum".to_string(),
        "tools".to_string(),
    ];
    assert!(parse_args(&args).is_err());
    let missing = vec!["--port".to_string()];
    assert!(parse_args(&missing).is_err());
}

#[test]
fn parse_args_help_version_and_empty() {
    assert!(matches!(
        parse_args(&["help".to_string()]).unwrap().command,
        Command::Help
    ));
    assert!(matches!(parse_args(&[]).unwrap().command, Command::Help));
    assert!(matches!(
        parse_args(&["version".to_string()]).unwrap().command,
        Command::Version
    ));
}
