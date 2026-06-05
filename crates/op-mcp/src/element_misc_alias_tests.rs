//! TS inline/navigation misc element alias parity checks.

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
fn misc_navigation_aliases_emit_ts_builder_semantic_subtrees() {
    let legend = semantic_value(
        "add_legend_item_v1",
        args([
            ("label", "Revenue"),
            ("color", "#2563EB"),
            ("value", "$12k"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(legend["role"], "legend-item");
    assert_eq!(legend["children"][0]["role"], "legend-item-marker");
    assert_eq!(legend["children"][0]["fill"][0]["color"], "#2563EB");
    assert_eq!(
        legend["children"][1]["fill"][0]["color"],
        "$color-text-body"
    );
    assert_eq!(
        legend["children"][2]["fill"][0]["color"],
        "$color-text-primary"
    );

    let price = semantic_value(
        "add_price_v1",
        args([("amount", "29"), ("currency", "$"), ("period", "/mo")]),
    );
    assert_eq!(price["role"], "price");
    assert_eq!(price["alignItems"], "end");
    assert_eq!(price["children"][0]["role"], "price-currency");
    assert_eq!(price["children"][1]["role"], "price-amount");
    assert_eq!(price["children"][2]["role"], "price-period");

    let quote = semantic_value(
        "add_quote_block_v1",
        args([
            ("quote", "Design is intent made visible."),
            ("author", "Mina"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(quote["role"], "quote-block");
    assert_eq!(quote["fill"][0]["color"], "$color-surface");
    assert_eq!(quote["children"][0]["role"], "quote-text");
    assert_eq!(quote["children"][1]["content"], "\u{2014} Mina");

    let chips = semantic_value(
        "add_nav_chip_row_v1",
        args([
            (
                "items",
                r#"[{"label":"Home","icon":"home","active":true},{"label":"Settings"}]"#,
            ),
            ("chip_width", "80"),
            ("gap", "10"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(chips["role"], "scroll-row-wrapper");
    assert_eq!(chips["children"][0]["gap"].as_f64(), Some(10.0));
    assert_eq!(
        chips["children"][0]["children"][0]["role"],
        "nav-chip-active"
    );
    assert_eq!(
        chips["children"][0]["children"][0]["children"][0]["iconFontName"],
        "home"
    );
    assert_eq!(chips["children"][0]["children"][1]["role"], "nav-chip");

    let tag = semantic_value(
        "add_tag_v1",
        args([
            ("label", "Status: Active"),
            ("tone", "success"),
            ("removable", "true"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(tag["role"], "tag");
    assert_eq!(tag["fill"][0]["color"], "#DCFCE7");
    assert_eq!(tag["children"][0]["fill"][0]["color"], "#166534");
    assert_eq!(tag["children"][1]["role"], "tag-remove");

    let stepper = semantic_value(
        "add_stepper_v1",
        args([("total", "4"), ("current", "1"), ("theme", "system")]),
    );
    assert_eq!(stepper["role"], "stepper");
    assert_eq!(stepper["children"].as_array().expect("stepper").len(), 7);
    assert_eq!(stepper["children"][0]["role"], "step-active");
    assert_eq!(stepper["children"][2]["role"], "step-active");
    assert_eq!(stepper["children"][4]["role"], "step");
    assert_eq!(stepper["children"][4]["fill"][0]["color"], "$color-border");
    assert_eq!(
        stepper["children"][4]["children"][0]["fill"][0]["color"],
        "$color-text-muted"
    );

    let timeline = semantic_value(
        "add_timeline_v1",
        args([
            (
                "items",
                r#"[{"title":"Queued","subtitle":"Now","active":true},{"title":"Done","subtitle":"Later"}]"#,
            ),
            ("theme", "system"),
        ]),
    );
    assert_eq!(timeline["role"], "timeline");
    assert_eq!(timeline["children"][0]["role"], "timeline-item");
    assert_eq!(timeline["children"][0]["alignItems"], "start");
    assert_eq!(
        timeline["children"][0]["children"][0]["children"][1]["fill"][0]["color"],
        "$color-border"
    );
    assert_eq!(
        timeline["children"][1]["children"][1]["children"][1]["fill"][0]["color"],
        "$color-text-muted"
    );
}
