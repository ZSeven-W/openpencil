//! TS feedback/media element alias parity checks.

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
fn feedback_and_placeholder_aliases_emit_ts_builder_semantic_subtrees() {
    let alert = semantic_value(
        "add_alert_v1",
        args([
            ("message", "Saved"),
            ("icon", "check"),
            ("dismissible", "true"),
            ("theme", "dark"),
        ]),
    );
    assert_eq!(alert["role"], "alert");
    assert_eq!(alert["width"], "fill_container");
    assert_eq!(alert["padding"], serde_json::json!([12.0, 16.0]));
    assert_eq!(
        alert["children"].as_array().expect("alert children").len(),
        3
    );
    assert_eq!(alert["children"][0]["iconFontName"], "check");
    assert_eq!(alert["children"][1]["role"], "alert-message");
    assert_eq!(alert["children"][2]["role"], "alert-close");

    let callout = semantic_value(
        "add_callout_v1",
        args([
            ("body", "Check your setup"),
            ("title", "Heads up"),
            ("tone", "warning"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(callout["role"], "callout");
    assert_eq!(callout["fill"][0]["color"], "$color-warning-bg");
    assert_eq!(callout["children"][0]["role"], "callout-icon");
    assert_eq!(
        callout["children"][0]["fill"][0]["color"],
        "$color-warning-text"
    );
    assert_eq!(callout["children"][1]["role"], "callout-text");
    assert_eq!(
        callout["children"][1]["children"][0]["role"],
        "callout-title"
    );
    assert_eq!(
        callout["children"][1]["children"][1]["role"],
        "callout-body"
    );

    let toast = semantic_value(
        "add_toast_v1",
        args([("message", "Copied"), ("icon", "check"), ("theme", "dark")]),
    );
    assert_eq!(toast["role"], "toast");
    assert_eq!(toast["fill"][0]["color"], "#F1F5F9");
    assert_eq!(toast["children"][0]["fill"][0]["color"], "#0F172A");
    assert_eq!(toast["children"][1]["role"], "toast-message");
    assert_eq!(toast["children"][1]["fill"][0]["color"], "#0F172A");

    let empty_state = semantic_value(
        "add_empty_state_v1",
        args([
            ("title", "No items"),
            ("subtitle", "Create one to get started"),
            ("icon", "inbox"),
            ("cta_label", "Create"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(empty_state["role"], "empty-state");
    assert_eq!(empty_state["layout"], "vertical");
    assert_eq!(empty_state["padding"], serde_json::json!([48.0, 24.0]));
    assert_eq!(
        empty_state["children"]
            .as_array()
            .expect("empty-state children")
            .len(),
        4
    );
    assert_eq!(empty_state["children"][0]["role"], "empty-state-icon");
    assert_eq!(empty_state["children"][3]["role"], "button");

    let image = semantic_value(
        "add_image_placeholder_v1",
        args([
            ("width", "260"),
            ("height", "120"),
            ("label", "Hero image"),
            ("icon", "image-plus"),
            ("corner_radius", "10"),
            ("image_search_query", "modern office"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(image["role"], "image-placeholder");
    assert_eq!(image["fill"][0]["color"], "$color-bg-deep");
    assert_eq!(image["imageSearchQuery"], "modern office");
    assert_eq!(image["children"][0]["role"], "image-placeholder-icon");
    assert_eq!(
        image["children"][0]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(image["children"][1]["role"], "image-placeholder-label");

    let video = semantic_value(
        "add_video_placeholder_v1",
        args([
            ("width", "360"),
            ("height", "200"),
            ("label", "Coming soon"),
            ("corner_radius", "16"),
            ("theme", "dark"),
        ]),
    );
    assert_eq!(video["role"], "video-placeholder");
    assert_eq!(video["fill"][0]["color"], "#334155");
    assert_eq!(video["children"][0]["role"], "video-placeholder-icon");
    assert_eq!(video["children"][0]["iconFontName"], "play");
    assert_eq!(video["children"][0]["fill"][0]["color"], "#FFFFFF");
    assert_eq!(video["children"][1]["role"], "video-placeholder-label");
    assert_eq!(video["children"][1]["fill"][0]["color"], "#FFFFFFB3");
}
