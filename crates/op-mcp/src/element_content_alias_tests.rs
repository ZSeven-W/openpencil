//! TS content/card element alias parity checks.

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
fn content_card_aliases_emit_ts_builder_semantic_subtrees() {
    let cards = semantic_value(
        "add_card_row_v1",
        args([
            (
                "items",
                r#"[{"title":"Revenue","subtitle":"Up 12%","icon":"trending-up"},{"title":"Users","subtitle":"1.2k"}]"#,
            ),
            ("card_width", "160"),
            ("gap", "16"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(cards["role"], "scroll-row-wrapper");
    assert_eq!(cards["clipContent"], true);
    assert_eq!(cards["children"][0]["role"], "scroll-row");
    assert_eq!(cards["children"][0]["gap"].as_f64(), Some(16.0));
    assert_eq!(
        cards["children"][0]["padding"],
        serde_json::json!([0.0, 20.0])
    );
    assert_eq!(cards["children"][0]["children"][0]["role"], "card");
    assert_eq!(
        cards["children"][0]["children"][0]["width"].as_f64(),
        Some(160.0)
    );
    assert_eq!(
        cards["children"][0]["children"][0]["fill"][0]["color"],
        "$color-surface"
    );
    assert_eq!(
        cards["children"][0]["children"][0]["children"][1]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(
        cards["children"][0]["children"][0]["children"][2]["fill"][0]["color"],
        "$color-text-muted"
    );

    let comment = semantic_value(
        "add_comment_v1",
        args([
            ("author", "Mina"),
            ("timestamp", "2h"),
            ("body", "Looks good"),
            ("avatar_initial", "mi"),
            ("avatar_size", "44"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(comment["role"], "comment");
    assert_eq!(comment["children"][0]["role"], "comment-avatar");
    assert_eq!(comment["children"][0]["width"].as_f64(), Some(44.0));
    assert_eq!(
        comment["children"][0]["fill"][0]["color"],
        "$color-surface-2"
    );
    assert_eq!(comment["children"][0]["children"][0]["content"], "MI");
    assert_eq!(
        comment["children"][0]["children"][0]["fill"][0]["color"],
        "$color-text-body"
    );
    assert_eq!(
        comment["children"][1]["children"][0]["children"][1]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(
        comment["children"][1]["children"][1]["role"],
        "comment-body"
    );

    let chat = semantic_value(
        "add_chat_bubble_v1",
        args([
            ("message", "Need help?"),
            ("side", "left"),
            ("author", "Support"),
            ("timestamp", "now"),
            ("max_width", "220"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(chat["role"], "chat-bubble-left");
    assert_eq!(chat["alignItems"], "start");
    assert_eq!(chat["children"][0]["role"], "chat-bubble-author");
    assert_eq!(chat["children"][0]["fill"][0]["color"], "$color-text-muted");
    assert_eq!(chat["children"][1]["role"], "chat-bubble-surface");
    assert_eq!(chat["children"][1]["width"].as_f64(), Some(220.0));
    assert_eq!(chat["children"][1]["fill"][0]["color"], "$color-surface-2");
    assert_eq!(
        chat["children"][1]["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );

    let code = semantic_value(
        "add_code_block_v1",
        args([
            ("code", "fn main() {\n    println!(\"hi\");\n}"),
            ("language", "rust"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(code["role"], "code-block");
    assert_eq!(code["name"], "Code Block (rust)");
    assert_eq!(code["fill"][0]["color"], "$color-surface-2");
    assert_eq!(code["children"][0]["role"], "code");
    assert_eq!(code["children"][0]["textGrowth"], "fixed-width");

    let faq = semantic_value(
        "add_faq_item_v1",
        args([
            ("question", "Can I export?"),
            ("answer", "Yes, use the export menu."),
            ("expanded", "true"),
            ("show_divider", "true"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(faq["role"], "faq-item");
    assert_eq!(
        faq["padding"].as_array().map(|padding| padding.len()),
        Some(2)
    );
    assert_eq!(faq["padding"][0].as_f64(), Some(16.0));
    assert_eq!(faq["padding"][1].as_f64(), Some(0.0));
    assert_eq!(faq["children"][0]["role"], "faq-header");
    assert_eq!(
        faq["children"][0]["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(
        faq["children"][0]["children"][1]["role"],
        "faq-chevron-open"
    );
    assert_eq!(
        faq["children"][0]["children"][1]["iconFontName"],
        "chevron-down"
    );
    assert_eq!(faq["children"][1]["role"], "faq-answer");
    assert_eq!(faq["children"][1]["fill"][0]["color"], "$color-text-muted");
    assert_eq!(faq["children"][2]["role"], "faq-divider");
    assert_eq!(faq["children"][2]["fill"][0]["color"], "$color-border");

    let event = semantic_value(
        "add_event_card_v1",
        args([
            ("month", "OCT"),
            ("day", "15"),
            ("title", "Design review"),
            ("time", "2:00 PM"),
            ("location", "Room B"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(event["role"], "event-card");
    assert_eq!(event["fill"][0]["color"], "$color-surface");
    assert_eq!(event["stroke"]["fill"][0]["color"], "$color-border");
    assert_eq!(event["children"][0]["role"], "event-card-date");
    assert_eq!(event["children"][0]["fill"][0]["color"], "$color-surface-2");
    assert_eq!(
        event["children"][0]["children"][1]["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(event["children"][1]["role"], "event-card-text");
    assert_eq!(
        event["children"][1]["children"][0]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(
        event["children"][1]["children"][1]["children"][0]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(
        event["children"][1]["children"][2]["children"][1]["content"],
        "Room B"
    );
}
