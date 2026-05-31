//! Tests for batch style-property replacement commands.

use crate::command::{EditorCommand, StylePropValue, StylePropertyReplacement};
use crate::test_support::{frame, rect, state_with, text};
use crate::walkers::find_node;
use crate::NodeId;
use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{CornerRadius, Padding};
use jian_ops_schema::node::{FontWeight, PenNode};
use jian_ops_schema::style::{PenFill, PenStroke, SolidFillBody, StrokeThickness};

fn id(raw: &str) -> NodeId {
    NodeId::new(raw)
}

fn solid_fill(hex: &str) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: hex.to_string(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

fn styled_state() -> crate::EditorState {
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

    state_with(vec![frame(
        "n1",
        "Card",
        0.0,
        0.0,
        260.0,
        160.0,
        vec![bg, title],
    )])
}

#[test]
fn replace_all_matching_properties_updates_descendants_in_one_command() {
    let mut state = styled_state();

    assert!(state.apply(EditorCommand::ReplaceAllMatchingProperties {
        page_id: None,
        parent_ids: vec![id("n1")],
        replacements: vec![
            StylePropertyReplacement {
                property: "fillColor".into(),
                from: StylePropValue::String("#ffffff".into()),
                to: StylePropValue::String("#f8f8f8".into()),
            },
            StylePropertyReplacement {
                property: "textColor".into(),
                from: StylePropValue::String("#111111".into()),
                to: StylePropValue::String("#202020".into()),
            },
            StylePropertyReplacement {
                property: "strokeThickness".into(),
                from: StylePropValue::Number(2.0),
                to: StylePropValue::Number(4.0),
            },
            StylePropertyReplacement {
                property: "cornerRadius".into(),
                from: StylePropValue::Number(8.0),
                to: StylePropValue::Number(10.0),
            },
            StylePropertyReplacement {
                property: "padding".into(),
                from: StylePropValue::NumberArray(vec![4.0, 8.0, 4.0, 8.0]),
                to: StylePropValue::Number(6.0),
            },
            StylePropertyReplacement {
                property: "gap".into(),
                from: StylePropValue::Number(12.0),
                to: StylePropValue::Number(16.0),
            },
            StylePropertyReplacement {
                property: "fontFamily".into(),
                from: StylePropValue::String("Inter".into()),
                to: StylePropValue::String("Geist".into()),
            },
            StylePropertyReplacement {
                property: "fontSize".into(),
                from: StylePropValue::Number(16.0),
                to: StylePropValue::Number(18.0),
            },
            StylePropertyReplacement {
                property: "fontWeight".into(),
                from: StylePropValue::Number(700.0),
                to: StylePropValue::String("bold".into()),
            },
        ],
    }));

    let bg = find_node(state.active_children(), &id("n2")).expect("background");
    let PenNode::Rectangle(bg) = bg else {
        panic!("expected rect");
    };
    assert!(
        bg.children.is_none(),
        "style replacement must not materialize empty child arrays"
    );
    assert!(matches!(
        &bg.container.fill.as_ref().unwrap()[0],
        PenFill::Solid(body) if body.color == "#f8f8f8"
    ));
    assert!(matches!(
        &bg.container.stroke.as_ref().unwrap().thickness,
        StrokeThickness::Uniform(4.0)
    ));
    assert_eq!(
        bg.container.corner_radius,
        Some(CornerRadius::Uniform(10.0))
    );
    assert_eq!(bg.container.padding, Some(Padding::Uniform(6.0)));
    assert_eq!(bg.container.gap, Some(NumberOrExpression::Number(16.0)));

    let title = find_node(state.active_children(), &id("n3")).expect("title");
    let PenNode::Text(title) = title else {
        panic!("expected text");
    };
    assert!(matches!(
        &title.fill.as_ref().unwrap()[0],
        PenFill::Solid(body) if body.color == "#202020"
    ));
    assert_eq!(title.font_family.as_deref(), Some("Geist"));
    assert_eq!(title.font_size, Some(18.0));
    assert_eq!(title.font_weight, Some(FontWeight::Keyword("bold".into())));
}
