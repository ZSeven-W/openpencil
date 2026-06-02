use super::*;

#[test]
fn parse_args_maps_insert_parent_and_page_flags_to_ts_args() {
    let p = parse_args(&[
        "insert".to_string(),
        r##"{"kind":"rect","name":"Child","x":1,"y":2,"width":3,"height":4,"fill":"#fff"}"##
            .to_string(),
        "--parent".to_string(),
        "n10".to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse insert");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "insert_node".to_string(),
            args: vec![
                ("kind".to_string(), "rect".to_string()),
                ("name".to_string(), "Child".to_string()),
                ("x".to_string(), "1".to_string()),
                ("y".to_string(), "2".to_string()),
                ("width".to_string(), "3".to_string()),
                ("height".to_string(), "4".to_string()),
                ("fill_hex".to_string(), "#fff".to_string()),
                ("parent".to_string(), "n10".to_string()),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}
