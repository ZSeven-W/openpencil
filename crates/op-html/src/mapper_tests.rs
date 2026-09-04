use super::*;
use crate::css::cascade::parse_stylesheet;
use crate::dom::DomNode;
use jian_ops_schema::node::container::{
    AlignItems, CornerRadius, JustifyContent, LayoutMode, Padding,
};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::{PenEffect, PenFill, StrokeThickness};

fn map_one(html_element: crate::dom::DomElement, css: &str) -> Option<PenNode> {
    let (rules, _) = parse_stylesheet(css, 1000);
    let options = crate::HtmlImportOptions::default();
    let mut context = MapCtx {
        opts: &options,
        rules: &rules,
        warnings: Vec::new(),
        warned: Default::default(),
        next_id: 0,
        node_count: 0,
        containing_width: options.viewport_width,
        containing_height: options.viewport_height(),
        containing_width_is_definite: true,
        positioned_width: options.viewport_width,
        positioned_height: options.viewport_height(),
        auto_margin_handled_by_parent: false,
        pending_base_outcome: Default::default(),
    };
    map_element(&mut context, &[&html_element], None)
}

#[test]
fn flex_row_maps_to_horizontal_frame() {
    let element = crate::dom::DomElement {
        tag: "div".into(),
        attrs: vec![(
            "style".into(),
            "display:flex;gap:12px;justify-content:space-between;align-items:center;padding:16px"
                .into(),
        )],
        children: Vec::new(),
    };
    let Some(PenNode::Frame(frame)) = map_one(element, "") else {
        panic!("expected frame")
    };
    assert_eq!(frame.container.layout, Some(LayoutMode::Horizontal));
    assert!(matches!(
        frame.container.gap,
        Some(jian_ops_schema::node::base::NumberOrExpression::Number(value)) if value == 12.0
    ));
    assert_eq!(
        frame.container.justify_content,
        Some(JustifyContent::SpaceBetween)
    );
    assert_eq!(frame.container.align_items, Some(AlignItems::Center));
    assert!(matches!(
        frame.container.padding,
        Some(Padding::Uniform(value)) if value == 16.0
    ));
}

#[test]
fn visual_styles_map_to_fill_stroke_effects() {
    let element = crate::dom::DomElement {
        tag: "div".into(),
        attrs: vec![(
            "style".into(),
            "background-color:#102030;border:2px solid #ff0000;border-radius:8px;\
             box-shadow:0 4px 8px rgba(0,0,0,0.25);overflow:hidden"
                .into(),
        )],
        children: Vec::new(),
    };
    let Some(PenNode::Frame(frame)) = map_one(element, "") else {
        panic!()
    };
    let fills = frame.container.fill.as_ref().unwrap();
    assert!(matches!(&fills[0], PenFill::Solid(solid) if solid.color == "#102030"));
    let stroke = frame.container.stroke.as_ref().unwrap();
    assert!(matches!(
        stroke.thickness,
        StrokeThickness::Uniform(width) if width == 2.0
    ));
    assert!(matches!(
        frame.container.corner_radius,
        Some(CornerRadius::Uniform(radius)) if radius == 8.0
    ));
    let effects = frame.container.effects.as_ref().unwrap();
    assert!(matches!(&effects[0], PenEffect::Shadow(shadow)
        if shadow.offset_y == 4.0 && shadow.blur == 8.0 && shadow.color == "#00000040"));
    assert_eq!(frame.container.clip_content, Some(true));
}

/// `hidden` / `clip` really do hide their overflow, so the clip is exact and
/// silent. `auto` / `scroll` lose content a browser lets the reader reach.
#[test]
fn scroll_containers_report_the_content_the_clip_hides() {
    for (overflow, expected) in [("auto", true), ("scroll", true), ("overlay", true)] {
        let result = crate::import_html(
            &format!("<div style='width:100px;height:50px;overflow:{overflow}'>copy</div>"),
            &crate::HtmlImportOptions::default(),
        );
        assert_eq!(
            result
                .warnings
                .iter()
                .any(|warning| warning.contains("overflow: auto / scroll")),
            expected,
            "{overflow}: {:?}",
            result.warnings
        );
    }

    for overflow in ["hidden", "clip", "visible"] {
        let result = crate::import_html(
            &format!("<div style='width:100px;height:50px;overflow:{overflow}'>copy</div>"),
            &crate::HtmlImportOptions::default(),
        );
        assert!(
            !result
                .warnings
                .iter()
                .any(|warning| warning.contains("overflow: auto / scroll")),
            "{overflow}: {:?}",
            result.warnings
        );
    }
}

#[test]
fn linear_gradient_fill() {
    let element = crate::dom::DomElement {
        tag: "div".into(),
        attrs: vec![(
            "style".into(),
            "background:linear-gradient(90deg,#000000,#ffffff)".into(),
        )],
        children: Vec::new(),
    };
    let Some(PenNode::Frame(frame)) = map_one(element, "") else {
        panic!()
    };
    let fills = frame.container.fill.as_ref().unwrap();
    let PenFill::LinearGradient(gradient) = &fills[0] else {
        panic!("expected gradient")
    };
    assert_eq!(gradient.angle, Some(90.0));
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(gradient.stops[1].color, "#ffffff");
    assert_eq!(gradient.stops[1].offset, 1.0);
}

#[test]
fn display_none_is_skipped() {
    let element = crate::dom::DomElement {
        tag: "div".into(),
        attrs: vec![("style".into(), "display:none".into())],
        children: Vec::new(),
    };
    assert!(map_one(element, "").is_none());
}

#[test]
fn gap_mode_from_margins() {
    let make_style = |margin_top: &str, margin_bottom: &str| {
        let mut style = ComputedStyle {
            props: Default::default(),
            font_size: 16.0,
        };
        style.props.insert("margin-top".into(), margin_top.into());
        style
            .props
            .insert("margin-bottom".into(), margin_bottom.into());
        style
    };
    let first = make_style("0", "8px");
    let second = make_style("8px", "8px");
    let third = make_style("8px", "16px");
    let fourth = make_style("8px", "0");
    let styles = vec![&first, &second, &third, &fourth];
    let (gap, deviated) = infer_gap_from_margins(&styles, 16.0);
    assert_eq!(gap, Some(16.0));
    assert!(deviated);
}

#[test]
fn definite_column_child_fills_but_row_flex_item_shrink_wraps() {
    let parent = crate::dom::DomElement {
        tag: "div".into(),
        attrs: vec![(
            "style".into(),
            "display:flex;flex-direction:column;width:600px".into(),
        )],
        children: vec![crate::dom::DomNode::Element(crate::dom::DomElement {
            tag: "div".into(),
            attrs: vec![],
            children: Vec::new(),
        })],
    };
    let Some(PenNode::Frame(frame)) = map_one(parent, "") else {
        panic!()
    };
    let PenNode::Frame(child) = &frame.children.as_ref().unwrap()[0] else {
        panic!()
    };
    use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
    assert_eq!(
        child.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    );
    assert_eq!(
        child.container.height,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    );

    let row = crate::dom::DomElement {
        tag: "div".into(),
        attrs: vec![("style".into(), "display:flex;width:600px".into())],
        children: vec![crate::dom::DomNode::Element(crate::dom::DomElement {
            tag: "div".into(),
            attrs: vec![],
            children: Vec::new(),
        })],
    };
    let Some(PenNode::Frame(row)) = map_one(row, "") else {
        panic!()
    };
    let PenNode::Frame(item) = &row.children.as_ref().unwrap()[0] else {
        panic!()
    };
    assert_eq!(
        item.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    );
}

#[test]
fn absolute_positioning_lands_on_base_xy() {
    let element = crate::dom::DomElement {
        tag: "div".into(),
        attrs: vec![(
            "style".into(),
            "position:absolute;left:24px;top:48px".into(),
        )],
        children: Vec::new(),
    };
    let Some(PenNode::Frame(frame)) = map_one(element, "") else {
        panic!()
    };
    assert_eq!(frame.base.x, Some(24.0));
    assert_eq!(frame.base.y, Some(48.0));
}

#[test]
fn right_offset_and_inline_flex_use_parent_geometry() {
    let parent = crate::dom::DomElement {
        tag: "div".into(),
        attrs: vec![(
            "style".into(),
            "position:relative;width:38px;height:38px".into(),
        )],
        children: vec![DomNode::Element(crate::dom::DomElement {
            tag: "small".into(),
            attrs: vec![(
                "style".into(),
                "position:absolute;right:-4px;top:-5px;width:17px;height:17px".into(),
            )],
            children: vec![DomNode::Text("2".into())],
        })],
    };
    let Some(PenNode::Frame(frame)) = map_one(parent, "") else {
        panic!()
    };
    let PenNode::Frame(badge) = &frame.children.as_ref().unwrap()[0] else {
        panic!()
    };
    assert_eq!((badge.base.x, badge.base.y), (Some(25.0), Some(-5.0)));

    let child = crate::dom::DomElement {
        tag: "button".into(),
        attrs: vec![("style".into(), "display:inline-flex".into())],
        children: vec![DomNode::Text("Go".into())],
    };
    let parent = crate::dom::DomElement {
        tag: "div".into(),
        attrs: Vec::new(),
        children: vec![DomNode::Element(child)],
    };
    let Some(PenNode::Frame(frame)) = map_one(parent, "") else {
        panic!()
    };
    let PenNode::Frame(inline_row) = &frame.children.as_ref().unwrap()[0] else {
        panic!()
    };
    let PenNode::Frame(button) = &inline_row.children.as_ref().unwrap()[0] else {
        panic!()
    };
    assert_eq!(
        button.container.width,
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    );
}

#[test]
fn implicit_single_column_grid_does_not_warn() {
    let (rules, _) = crate::css::cascade::parse_stylesheet("", 1000);
    let options = crate::HtmlImportOptions::default();
    let mut context = MapCtx {
        opts: &options,
        rules: &rules,
        warnings: Vec::new(),
        warned: Default::default(),
        next_id: 0,
        node_count: 0,
        containing_width: options.viewport_width,
        containing_height: options.viewport_height(),
        containing_width_is_definite: true,
        positioned_width: options.viewport_width,
        positioned_height: options.viewport_height(),
        auto_margin_handled_by_parent: false,
        pending_base_outcome: Default::default(),
    };
    for _ in 0..2 {
        let element = crate::dom::DomElement {
            tag: "div".into(),
            attrs: vec![("style".into(), "display:grid".into())],
            children: Vec::new(),
        };
        map_element(&mut context, &[&element], None);
    }
    let warnings = crate::render_warnings(&context.warnings);
    assert!(warnings.iter().all(|warning| !warning.contains("grid")));
}
