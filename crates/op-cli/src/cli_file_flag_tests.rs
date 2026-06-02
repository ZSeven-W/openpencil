use super::*;

#[test]
fn parse_args_resolves_relative_file_flag_before_sending_to_mcp_server() {
    let expected = std::env::current_dir()
        .expect("cwd")
        .join("screen.op")
        .display()
        .to_string();
    let parsed = parse_args(&[
        "get".to_string(),
        "--id".to_string(),
        "n10".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse get");

    assert_eq!(
        parsed.command,
        Command::ToolCall {
            tool: "batch_get".to_string(),
            args: vec![
                ("nodeIds".to_string(), r#"["n10"]"#.to_string()),
                ("filePath".to_string(), expected),
            ],
        }
    );
}

#[test]
fn parse_args_maps_file_flag_on_read_aliases_to_ts_file_path() {
    let get = parse_args(&[
        "get".to_string(),
        "--id".to_string(),
        "n10".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse get");
    assert_eq!(
        get.command,
        Command::ToolCall {
            tool: "batch_get".to_string(),
            args: vec![
                ("nodeIds".to_string(), r#"["n10"]"#.to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );

    let layout = parse_args(&[
        "layout".to_string(),
        "--parent".to_string(),
        "n10".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse layout");
    assert_eq!(
        layout.command,
        Command::ToolCall {
            tool: "snapshot_layout".to_string(),
            args: vec![
                ("parentId".to_string(), "n10".to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );

    let find_space = parse_args(&[
        "find-space".to_string(),
        "--width".to_string(),
        "320".to_string(),
        "--height".to_string(),
        "240".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse find-space");
    assert_eq!(
        find_space.command,
        Command::ToolCall {
            tool: "find_empty_space".to_string(),
            args: vec![
                ("direction".to_string(), "right".to_string()),
                ("width".to_string(), "320".to_string()),
                ("height".to_string(), "240".to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );
}

#[test]
fn parse_args_maps_file_flag_on_write_aliases_to_ts_file_path() {
    let insert = parse_args(&[
        "insert".to_string(),
        r#"{"type":"rectangle","name":"Card"}"#.to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse insert");
    assert_eq!(
        insert.command,
        Command::ToolCall {
            tool: "insert_node".to_string(),
            args: vec![
                (
                    "data".to_string(),
                    r#"{"type":"rectangle","name":"Card"}"#.to_string(),
                ),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );

    let update = parse_args(&[
        "update".to_string(),
        "n10".to_string(),
        r#"{"name":"Updated"}"#.to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse update");
    assert_eq!(
        update.command,
        Command::ToolCall {
            tool: "update_node".to_string(),
            args: vec![
                ("nodeId".to_string(), "n10".to_string()),
                ("data".to_string(), r#"{"name":"Updated"}"#.to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );

    let delete = parse_args(&[
        "delete".to_string(),
        "n10".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse delete");
    assert_eq!(
        delete.command,
        Command::ToolCall {
            tool: "delete_node".to_string(),
            args: vec![
                ("nodeId".to_string(), "n10".to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );
}

#[test]
fn parse_args_maps_file_flag_on_page_and_variable_aliases_to_ts_file_path() {
    let page_add = parse_args(&[
        "page".to_string(),
        "add".to_string(),
        "--name".to_string(),
        "Checkout".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse page add");
    assert_eq!(
        page_add.command,
        Command::ToolCall {
            tool: "add_page".to_string(),
            args: vec![
                ("name".to_string(), "Checkout".to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );

    let vars = parse_args(&[
        "vars".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse vars");
    assert_eq!(
        vars.command,
        Command::ToolCall {
            tool: "get_variables".to_string(),
            args: vec![("filePath".to_string(), resolve_file_path_arg("screen.op"))],
        }
    );

    let theme_save = parse_args(&[
        "theme:save".to_string(),
        "brand.optheme".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse theme save");
    assert_eq!(
        theme_save.command,
        Command::ToolCall {
            tool: "save_theme_preset".to_string(),
            args: vec![
                ("presetPath".to_string(), "brand.optheme".to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );
}

#[test]
fn parse_args_maps_file_flag_on_import_svg_to_ts_file_path() {
    let import = parse_args(&[
        "import:svg".to_string(),
        "icon.svg".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse import:svg");

    assert_eq!(
        import.command,
        Command::ToolCall {
            tool: "import_svg".to_string(),
            args: vec![
                ("svgPath".to_string(), "icon.svg".to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );
}

#[test]
fn parse_args_maps_file_flag_on_generic_tool_to_ts_file_path() {
    let generic = parse_args(&[
        "insert_node".to_string(),
        "kind=rect".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse generic tool");

    assert_eq!(
        generic.command,
        Command::ToolCall {
            tool: "insert_node".to_string(),
            args: vec![
                ("kind".to_string(), "rect".to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );
}
