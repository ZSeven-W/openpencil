//! TS-compatible style operation tool tests.

use std::collections::BTreeMap;

use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{CornerRadius, Padding};
use jian_ops_schema::node::{FontWeight, PenNode};
use jian_ops_schema::style::{PenFill, PenStroke, SolidFillBody, StrokeThickness};
use serde_json::Value;

use super::test_fixtures::{frame, rect, state_with, text};
use super::{
    replace_all_matching_properties_snapshot, search_all_unique_properties_snapshot, EditorCommand,
    McpTool, ToolOutcome,
};

fn solid_fill(hex: &str) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: hex.to_string(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

fn style_ops_state() -> op_editor_core::EditorState {
    let mut bg = rect("n2", "Card background", 0.0, 0.0, 240.0, 120.0);
    if let PenNode::Rectangle(r) = &mut bg {
        r.container.fill = Some(vec![solid_fill("#ffffff")]);
        r.container.stroke = Some(PenStroke {
            thickness: StrokeThickness::Uniform(2.0),
            align: None,
            join: None,
            cap: None,
            dash_pattern: None,
            dash_offset: None,
            fill: Some(vec![solid_fill("#222222")]),
        });
        r.container.corner_radius = Some(CornerRadius::Uniform(8.0));
        r.container.padding = Some(Padding::LtrB([4.0, 8.0, 4.0, 8.0]));
        r.container.gap = Some(NumberOrExpression::Number(12.0));
    }

    let mut title = text("n3", "Title", 16.0, 16.0, 160.0, 24.0, "Hello");
    if let PenNode::Text(t) = &mut title {
        t.fill = Some(vec![solid_fill("#111111")]);
        t.font_family = Some("Inter".into());
        t.font_size = Some(16.0);
        t.font_weight = Some(FontWeight::Number(700));
    }

    let root = frame("n1", "Card", 0.0, 0.0, 260.0, 160.0, vec![bg, title]);
    state_with(vec![root])
}

#[test]
fn search_all_unique_properties_recurses_and_splits_text_fill_from_shape_fill() {
    let tool = search_all_unique_properties_snapshot(&style_ops_state());
    let mut args = BTreeMap::new();
    args.insert("parents".into(), r#"["n1"]"#.into());
    args.insert(
        "properties".into(),
        r#"["fillColor","textColor","strokeColor","strokeThickness","cornerRadius","padding","gap","fontSize","fontFamily","fontWeight"]"#.into(),
    );

    let props = match tool.call(&args) {
        ToolOutcome::Ok(out) => {
            let raw = out.get("properties").expect("properties json");
            serde_json::from_str::<Value>(raw).expect("valid properties json")
        }
        other => panic!("expected search result, got {other:?}"),
    };

    assert_eq!(props["fillColor"], serde_json::json!(["#ffffff"]));
    assert_eq!(props["textColor"], serde_json::json!(["#111111"]));
    assert_eq!(props["strokeColor"], serde_json::json!(["#222222"]));
    assert_eq!(props["strokeThickness"][0].as_f64(), Some(2.0), "{props}");
    assert_eq!(props["cornerRadius"], serde_json::json!([8.0]));
    assert_eq!(props["padding"], serde_json::json!([[4.0, 8.0, 4.0, 8.0]]));
    assert_eq!(props["gap"], serde_json::json!([12.0]));
    assert_eq!(props["fontSize"], serde_json::json!([16.0]));
    assert_eq!(props["fontFamily"], serde_json::json!(["Inter"]));
    assert_eq!(props["fontWeight"], serde_json::json!([700]));
}

#[test]
fn replace_all_matching_properties_returns_bulk_command_and_replaced_count() {
    let mut args = BTreeMap::new();
    args.insert("parents".into(), r#"["n1"]"#.into());
    args.insert(
        "properties".into(),
        r##"{
          "fillColor":[{"from":"#ffffff","to":"#f8f8f8"}],
          "textColor":[{"from":"#111111","to":"#202020"}],
          "gap":[{"from":12,"to":16}]
        }"##
        .into(),
    );

    match replace_all_matching_properties_snapshot(&style_ops_state()).call(&args) {
        ToolOutcome::OkWithCommand(out, EditorCommand::ReplaceAllMatchingProperties { .. }) => {
            assert_eq!(out.get("replacedCount").map(String::as_str), Some("3"));
        }
        other => panic!("expected bulk style replacement command, got {other:?}"),
    }
}
