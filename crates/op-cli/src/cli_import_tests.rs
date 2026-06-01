use super::*;

#[test]
fn parse_args_maps_import_svg_parent_flag_to_ts_parent_arg() {
    let path = "/tmp/icon.svg";

    let p = parse_args(&[
        "import:svg".to_string(),
        path.to_string(),
        "--parent".to_string(),
        "n10".to_string(),
    ])
    .expect("parse import svg");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "import_svg".to_string(),
            args: vec![
                ("svgPath".to_string(), path.to_string()),
                ("parent".to_string(), "n10".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_import_svg_page_flag_to_ts_page_arg() {
    let path = "/tmp/icon.svg";

    let p = parse_args(&[
        "import:svg".to_string(),
        path.to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse import svg");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "import_svg".to_string(),
            args: vec![
                ("svgPath".to_string(), path.to_string()),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}
