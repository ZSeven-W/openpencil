//! TS atom element alias parity checks.

use std::collections::BTreeMap;

use op_editor_core::{EditorCommand, EditorState};
use serde_json::Value;

use super::element_tools::{insert_kit_component_tools, InsertKitComponent};
use super::{McpTool, ToolOutcome};

fn alias_tool(name: &str) -> InsertKitComponent {
    insert_kit_component_tools(&EditorState::new())
        .into_iter()
        .find(|tool| tool.name() == name)
        .unwrap_or_else(|| panic!("missing element alias {name}"))
}

fn semantic_value(tool: &str, args: BTreeMap<String, String>) -> Value {
    match alias_tool(tool).call(&args) {
        ToolOutcome::OkWithCommand(_, EditorCommand::InsertSubtree { nodes, .. }) => {
            serde_json::to_value(&nodes[0]).expect("semantic node json")
        }
        other => panic!("expected semantic {tool} InsertSubtree, got {other:?}"),
    }
}

fn args<const N: usize>(pairs: [(&str, &str); N]) -> BTreeMap<String, String> {
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

#[test]
fn atom_aliases_emit_ts_builder_semantic_subtrees() {
    let divider = semantic_value(
        "add_divider_v1",
        args([
            ("orientation", "vertical"),
            ("thickness", "2"),
            ("theme", "dark"),
        ]),
    );
    assert_eq!(divider["type"], "rectangle");
    assert_eq!(divider["role"], "divider");
    assert_eq!(divider["width"].as_f64(), Some(2.0));
    assert_eq!(divider["height"], "fill_container");

    let avatar = semantic_value(
        "add_avatar_v1",
        args([("initial", "JD"), ("size", "80"), ("theme", "system")]),
    );
    assert_eq!(avatar["role"], "avatar");
    assert_eq!(avatar["width"].as_f64(), Some(80.0));
    assert_eq!(avatar["cornerRadius"].as_f64(), Some(40.0));
    assert_eq!(avatar["children"][0]["content"], "JD");
    assert_eq!(avatar["children"][0]["fontSize"].as_f64(), Some(32.0));

    let icon_button = semantic_value(
        "add_icon_button_v1",
        args([
            ("icon", "x"),
            ("size", "52"),
            ("icon_size", "20"),
            ("theme", "dark"),
        ]),
    );
    assert_eq!(icon_button["role"], "icon-button");
    assert_eq!(icon_button["width"].as_f64(), Some(52.0));
    assert_eq!(icon_button["cornerRadius"].as_f64(), Some(8.0));
    assert_eq!(icon_button["children"][0]["iconFontName"], "x");
    assert_eq!(icon_button["children"][0]["width"].as_f64(), Some(20.0));

    let icon_label = semantic_value(
        "add_icon_label_v1",
        args([
            ("icon", "info"),
            ("label", "Learn more"),
            ("gap", "12"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(icon_label["role"], "icon-label");
    assert_eq!(icon_label["width"], "fit_content");
    assert_eq!(icon_label["gap"].as_f64(), Some(12.0));
    assert_eq!(icon_label["children"][0]["iconFontName"], "info");
    assert_eq!(icon_label["children"][1]["content"], "Learn more");

    let skeleton = semantic_value(
        "add_skeleton_v1",
        args([
            ("rows", "3"),
            ("row_height", "10"),
            ("row_gap", "4"),
            ("last_row_short", "true"),
            ("theme", "dark"),
        ]),
    );
    assert_eq!(skeleton["role"], "skeleton");
    assert_eq!(skeleton["gap"].as_f64(), Some(4.0));
    assert_eq!(skeleton["children"].as_array().expect("rows").len(), 3);
    assert_eq!(skeleton["children"][0]["height"].as_f64(), Some(10.0));
    assert_eq!(skeleton["children"][0]["fill"][0]["color"], "#334155");
    assert_eq!(skeleton["children"][2]["width"].as_f64(), Some(220.0));

    let spinner = semantic_value(
        "add_spinner_v1",
        args([("size", "40"), ("thickness", "4"), ("theme", "system")]),
    );
    assert_eq!(spinner["role"], "spinner");
    assert_eq!(spinner["layout"], "none");
    assert_eq!(spinner["children"][0]["role"], "spinner-track");
    assert_eq!(
        spinner["children"][0]["stroke"]["fill"][0]["color"],
        "#E2E8F0"
    );
    assert_eq!(spinner["children"][1]["role"], "spinner-arc");
    assert_eq!(spinner["children"][1]["sweepAngle"].as_f64(), Some(270.0));
    assert_eq!(
        spinner["children"][1]["stroke"]["fill"][0]["color"],
        "#2563EB"
    );

    let tooltip = semantic_value(
        "add_tooltip_v1",
        args([("text", "Help"), ("position", "bottom"), ("theme", "dark")]),
    );
    assert_eq!(tooltip["role"], "tooltip-bottom");
    assert_eq!(tooltip["padding"], serde_json::json!([6.0, 10.0]));
    assert_eq!(tooltip["fill"][0]["color"], "#111827");
    assert_eq!(tooltip["children"][0]["role"], "tooltip-text");
    assert_eq!(tooltip["children"][0]["content"], "Help");
    assert_eq!(tooltip["children"][0]["fill"][0]["color"], "#FFFFFF");
}
