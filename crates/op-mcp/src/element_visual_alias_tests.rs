//! TS chart and visual element alias parity checks.

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
fn visual_chart_aliases_emit_ts_builder_semantic_subtrees() {
    let ring = semantic_value(
        "add_activity_ring_v1",
        args([
            ("center_text", "72%"),
            ("size", "96"),
            ("thickness", "10"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(ring["role"], "activity-ring");
    assert_eq!(ring["cornerRadius"].as_f64(), Some(48.0));
    assert_eq!(ring["stroke"]["thickness"].as_f64(), Some(10.0));
    assert_eq!(ring["stroke"]["fill"][0]["color"], "#000000");
    assert_eq!(ring["children"][0]["content"], "72%");

    let dots = semantic_value(
        "add_carousel_dots_v1",
        args([("total", "4"), ("current", "2"), ("theme", "system")]),
    );
    assert_eq!(dots["role"], "carousel-dots");
    assert_eq!(dots["children"].as_array().expect("dots").len(), 4);
    assert_eq!(dots["children"][2]["role"], "dot-active");
    assert_eq!(dots["children"][2]["width"].as_f64(), Some(16.0));
    assert_eq!(
        dots["children"][2]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(dots["children"][0]["fill"][0]["color"], "$color-border");

    let swatch = semantic_value(
        "add_color_swatch_v1",
        args([
            ("color", "$color-accent"),
            ("label", "Accent"),
            ("size", "72"),
            ("theme", "dark"),
        ]),
    );
    assert_eq!(swatch["role"], "color-swatch");
    assert_eq!(swatch["children"][0]["role"], "color-swatch-square");
    assert_eq!(swatch["children"][0]["fill"][0]["color"], "$color-accent");
    assert_eq!(swatch["children"][1]["content"], "Accent");

    let bars = semantic_value(
        "add_chart_bars_v1",
        args([
            ("values", "[5,10,0]"),
            ("bar_width", "10"),
            ("gap", "4"),
            ("chart_height", "100"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(bars["role"], "chart-bars");
    assert_eq!(bars["height"].as_f64(), Some(100.0));
    assert_eq!(bars["children"][0]["height"].as_f64(), Some(50.0));
    assert_eq!(bars["children"][1]["height"].as_f64(), Some(100.0));
    assert_eq!(bars["children"][2]["height"].as_f64(), Some(2.0));
    assert_eq!(bars["children"][0]["fill"][0]["color"], "$color-chart-1");

    let pie = semantic_value(
        "add_chart_pie_v1",
        args([
            ("values", "[1,3]"),
            ("diameter", "120"),
            ("inner_radius_ratio", "0.4"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(pie["role"], "chart-pie");
    assert_eq!(pie["layout"], "none");
    assert_eq!(pie["children"].as_array().expect("slices").len(), 2);
    assert_eq!(pie["children"][0]["startAngle"].as_f64(), Some(-90.0));
    assert_eq!(pie["children"][0]["sweepAngle"].as_f64(), Some(90.0));
    assert_eq!(pie["children"][0]["innerRadius"].as_f64(), Some(0.4));
    assert_eq!(pie["children"][0]["fill"][0]["color"], "$color-chart-1");
    assert_eq!(pie["children"][1]["fill"][0]["color"], "$color-chart-2");

    let empty_chart = semantic_value(
        "add_empty_chart_v1",
        args([
            ("width", "300"),
            ("height", "180"),
            ("title", "No sales"),
            ("subtitle", "Sync a source"),
            ("icon", "pie-chart"),
            ("theme", "system"),
        ]),
    );
    assert_eq!(empty_chart["role"], "empty-chart");
    assert_eq!(empty_chart["cornerRadius"].as_f64(), Some(12.0));
    assert_eq!(empty_chart["gap"].as_f64(), Some(8.0));
    assert_eq!(empty_chart["padding"].as_f64(), Some(24.0));
    assert_eq!(empty_chart["fill"][0]["color"], "$color-surface-2");
    assert_eq!(empty_chart["stroke"]["fill"][0]["color"], "$color-border");
    assert_eq!(
        empty_chart["children"][0]["fill"][0]["color"],
        "$color-text-muted"
    );
    assert_eq!(
        empty_chart["children"][1]["fill"][0]["color"],
        "$color-text-primary"
    );
    assert_eq!(
        empty_chart["children"][2]["fill"][0]["color"],
        "$color-text-muted"
    );
}
