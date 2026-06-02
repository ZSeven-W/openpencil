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
                (
                    "data".to_string(),
                    r##"{"kind":"rect","name":"Child","x":1,"y":2,"width":3,"height":4,"fill":"#fff"}"##
                        .to_string(),
                ),
                ("parent".to_string(), "n10".to_string()),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_update_page_flag_to_ts_page_arg() {
    let p = parse_args(&[
        "update".to_string(),
        "n10".to_string(),
        r##"{"name":"Updated","x":5,"fill":[{"type":"solid","color":"#123456"}]}"##.to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse update");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "update_node".to_string(),
            args: vec![
                ("nodeId".to_string(), "n10".to_string()),
                (
                    "data".to_string(),
                    r##"{"name":"Updated","x":5,"fill":[{"type":"solid","color":"#123456"}]}"##
                        .to_string(),
                ),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_delete_page_flag_to_ts_page_arg() {
    let p = parse_args(&[
        "delete".to_string(),
        "n10".to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse delete");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "delete_node".to_string(),
            args: vec![
                ("nodeId".to_string(), "n10".to_string()),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_move_page_and_index_flags_to_ts_args() {
    let p = parse_args(&[
        "move".to_string(),
        "n10".to_string(),
        "--parent".to_string(),
        "n20".to_string(),
        "--index".to_string(),
        "2".to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse move");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "move_node".to_string(),
            args: vec![
                ("nodeId".to_string(), "n10".to_string()),
                ("parent".to_string(), "n20".to_string()),
                ("index".to_string(), "2".to_string()),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_copy_page_flag_to_ts_args() {
    let p = parse_args(&[
        "copy".to_string(),
        "n10".to_string(),
        "--parent".to_string(),
        "n20".to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse copy");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "copy_node".to_string(),
            args: vec![
                ("sourceId".to_string(), "n10".to_string()),
                ("parent".to_string(), "n20".to_string()),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}

#[test]
fn parse_args_maps_replace_page_flag_to_ts_page_arg() {
    let p = parse_args(&[
        "replace".to_string(),
        "n10".to_string(),
        r##"{"type":"rectangle","name":"Replacement","x":1,"y":2,"width":3,"height":4,"fill":"#fff"}"##
            .to_string(),
        "--page".to_string(),
        "page-2".to_string(),
    ])
    .expect("parse replace");

    assert_eq!(
        p.command,
        Command::ToolCall {
            tool: "replace_node".to_string(),
            args: vec![
                ("nodeId".to_string(), "n10".to_string()),
                (
                    "data".to_string(),
                    r##"{"type":"rectangle","name":"Replacement","x":1,"y":2,"width":3,"height":4,"fill":"#fff"}"##
                        .to_string(),
                ),
                ("pageId".to_string(), "page-2".to_string()),
            ],
        }
    );
}
