use super::*;

#[test]
fn parse_args_maps_import_svg_parent_flag_to_ts_parent_arg() {
    let dir = std::env::temp_dir().join(format!("op-cli-import-svg-{}", std::process::id()));
    let path = dir.join("icon.svg");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    std::fs::write(&path, r#"<svg><rect width="10" height="10"/></svg>"#).expect("write svg");

    let p = parse_args(&[
        "import:svg".to_string(),
        path.to_string_lossy().to_string(),
        "--parent".to_string(),
        "n10".to_string(),
    ])
    .expect("parse import svg");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "import_svg".to_string(),
            args: vec![
                (
                    "svg".to_string(),
                    r#"<svg><rect width="10" height="10"/></svg>"#.to_string(),
                ),
                ("parent".to_string(), "n10".to_string()),
            ],
        }
    );
    let _ = std::fs::remove_dir_all(&dir);
}
