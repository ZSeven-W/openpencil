//! TS input/form element alias parity checks.

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
fn input_form_aliases_emit_ts_builder_semantic_subtrees() {
    let chip = semantic_value(
        "add_chip_input_v1",
        args([
            ("label", "Recipients"),
            ("chips", r#"["Mina","Kai"]"#),
            ("placeholder", "Add email"),
            ("required", "true"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(chip["role"], "chip-input");
    assert_eq!(chip["children"][0]["role"], "chip-input-label");
    assert_eq!(chip["children"][0]["content"], "Recipients *");
    assert_eq!(chip["children"][1]["role"], "chip-input-field");
    assert_eq!(chip["children"][1]["fill"][0]["color"], "$color-surface");
    assert_eq!(
        chip["children"][1]["stroke"]["fill"][0]["color"],
        "$color-border"
    );
    assert_eq!(chip["children"][1]["children"][0]["role"], "chip");
    assert_eq!(
        chip["children"][1]["children"][0]["fill"][0]["color"],
        "$color-surface-2"
    );
    assert_eq!(
        chip["children"][1]["children"][0]["children"][1]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(
        chip["children"][1]["children"][2]["fill"][0]["color"],
        "$color-text-subtle"
    );

    let combo = semantic_value(
        "add_combobox_v1",
        args([
            ("label", "Country"),
            ("placeholder", "Search country"),
            ("value", "Uni"),
            (
                "options",
                r#"[{"label":"United States","highlighted":true},{"label":"United Kingdom"}]"#,
            ),
            ("theme", "system"),
        ]),
    );
    assert_eq!(combo["role"], "combobox");
    assert_eq!(
        combo["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(combo["children"][1]["role"], "combobox-input");
    assert_eq!(combo["children"][1]["fill"][0]["color"], "$color-surface");
    assert_eq!(
        combo["children"][1]["stroke"]["fill"][0]["color"],
        "$color-accent"
    );
    assert_eq!(
        combo["children"][1]["children"][1]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(combo["children"][2]["role"], "combobox-dropdown");
    assert_eq!(
        combo["children"][2]["children"][0]["fill"][0]["color"],
        "$color-surface-2"
    );

    let date = semantic_value(
        "add_date_picker_v1",
        args([
            ("label", "Due date"),
            ("value", "Jan 15, 2026"),
            ("required", "true"),
            ("clearable", "true"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(date["role"], "date-picker");
    assert_eq!(
        date["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(date["children"][1]["role"], "date-picker-input");
    assert_eq!(date["children"][1]["fill"][0]["color"], "$color-surface");
    assert_eq!(
        date["children"][1]["stroke"]["fill"][0]["color"],
        "$color-border"
    );
    assert_eq!(
        date["children"][1]["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(
        date["children"][1]["children"][1]["children"][0]["role"],
        "date-picker-clear"
    );

    let input = semantic_value(
        "add_input_with_action_v1",
        args([
            ("placeholder", "Ask anything"),
            ("value", "Ship it"),
            ("action_kind", "icon"),
            ("action_icon", "send"),
            ("leading_icon", "search"),
            ("width", "360"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(input["role"], "input-with-action");
    assert_eq!(input["width"].as_f64(), Some(360.0));
    assert_eq!(input["children"][0]["role"], "input-with-action-input");
    assert_eq!(input["children"][0]["fill"][0]["color"], "$color-surface");
    assert_eq!(
        input["children"][0]["stroke"]["fill"][0]["color"],
        "$color-border"
    );
    assert_eq!(
        input["children"][0]["children"][0]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(input["children"][1]["children"][0]["iconFontName"], "send");
    assert_eq!(input["children"][1]["fill"][0]["color"], "$color-accent");

    let otp = semantic_value(
        "add_otp_input_v1",
        args([
            ("length", "5"),
            ("digits", r#"["1","2"]"#),
            ("focused_index", "2"),
            ("slot_size", "40"),
            ("gap", "10"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(otp["role"], "otp-input");
    assert_eq!(otp["children"].as_array().map(Vec::len), Some(5));
    assert_eq!(otp["children"][0]["role"], "otp-slot-filled");
    assert_eq!(otp["children"][0]["fill"][0]["color"], "$color-surface");
    assert_eq!(
        otp["children"][0]["stroke"]["fill"][0]["color"],
        "$color-border-strong"
    );
    assert_eq!(
        otp["children"][0]["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(otp["children"][2]["role"], "otp-slot-focused");

    let textarea = semantic_value(
        "add_textarea_v1",
        args([
            ("label", "Notes"),
            ("placeholder", "Write details"),
            ("rows", "3"),
            ("required", "true"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(textarea["role"], "textarea");
    assert_eq!(textarea["children"][0]["content"], "Notes *");
    assert_eq!(textarea["children"][1]["role"], "textarea-input");
    assert_eq!(textarea["children"][1]["height"].as_f64(), Some(96.0));
    assert_eq!(
        textarea["children"][1]["children"][0]["content"],
        "Write details"
    );
}
