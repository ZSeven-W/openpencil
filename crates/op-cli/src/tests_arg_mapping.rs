//! The rest of the TS-parity argument-mapping table: pages, layout, vars,
//! themes, read-nodes, theme presets, codegen, and the generic flag parser.
//!
//! Split out of `tests.rs` (pure code motion) so both files stay under the
//! repo's 800-line cap.

use super::*;

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
fn parse_args_maps_themes_to_ts_variables_read_tool() {
    let p = parse_args(&["themes".to_string()]).expect("parse themes");
    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "get_variables".to_string(),
            args: vec![],
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
                ("nodeIds".to_string(), r#"["n10","n11"]"#.to_string()),
                ("depth".to_string(), "0".to_string()),
                ("pageId".to_string(), "p1".to_string()),
                ("includeVariables".to_string(), "true".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_keeps_each_read_nodes_positional_id() {
    let p = parse_args(&[
        "read-nodes".to_string(),
        "n10".to_string(),
        "n11".to_string(),
    ])
    .expect("parse");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "read_nodes".to_string(),
            args: vec![("nodeIds".to_string(), r#"["n10","n11"]"#.to_string())],
        }
    );
}

#[test]
fn parse_args_splits_comma_joined_read_nodes_ids_into_json_array() {
    let p = parse_args(&[
        "read-nodes".to_string(),
        "n10,n11".to_string(),
        "n12".to_string(),
        "".to_string(),
    ])
    .expect("parse");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "read_nodes".to_string(),
            args: vec![("nodeIds".to_string(), r#"["n10","n11","n12"]"#.to_string())],
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
            let design_path = resolve_file_path_arg("/tmp/design.op");
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&args_json).expect("args json"),
                serde_json::json!({
                    "filePath": design_path,
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
            args: vec![(
                "data".to_string(),
                r##"{"type":"rectangle","name":"Card","x":12,"y":24,"width":200,"height":100,"fill":[{"type":"solid","color":"#ffffff"}]}"##
                    .to_string(),
            )],
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
