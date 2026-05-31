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
