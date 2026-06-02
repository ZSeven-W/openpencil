use super::*;

#[test]
fn parse_args_maps_selection_depth_and_file_to_ts_shape() {
    let p = parse_args(&[
        "selection".to_string(),
        "--depth".to_string(),
        "0".to_string(),
        "--file".to_string(),
        "screen.op".to_string(),
    ])
    .expect("parse selection");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "get_selection".to_string(),
            args: vec![
                ("readDepth".to_string(), "0".to_string()),
                ("filePath".to_string(), resolve_file_path_arg("screen.op")),
            ],
        }
    );
}
