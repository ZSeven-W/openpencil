//! PenNode constructor helpers — assemble canonical
//! `jian_ops_schema` nodes from the meaningful converter fields,
//! filling the behaviour-trait slots (state / events / lifecycle …)
//! with `None`.

use crate::text_mapper::TextProps;
use jian_ops_schema::node::text::{FontStyleKind as TextFontStyle, FontWeight};
use jian_ops_schema::node::{
    ContainerProps, EllipseNode, FrameNode, GroupNode, LineNode, PathNode, PenNode, PenNodeBase,
    RectangleNode, RefNode, TextNode,
};
use jian_ops_schema::sizing::SizingBehavior;
use jian_ops_schema::style::{FontStyleKind as StyleFontStyle, PenEffect, PenFill, PenStroke};

pub fn frame_node(
    base: PenNodeBase,
    container: ContainerProps,
    children: Option<Vec<PenNode>>,
    reusable: Option<bool>,
) -> PenNode {
    PenNode::Frame(FrameNode {
        base,
        container,
        children,
        image_search_query: None,
        reusable,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

pub fn group_node(
    base: PenNodeBase,
    container: ContainerProps,
    children: Option<Vec<PenNode>>,
) -> PenNode {
    PenNode::Group(GroupNode {
        base,
        container,
        children,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

pub fn rectangle_node(base: PenNodeBase, container: ContainerProps) -> PenNode {
    PenNode::Rectangle(RectangleNode {
        base,
        container,
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// Ellipse — includes the optional arc fields.
#[allow(clippy::too_many_arguments)]
pub fn ellipse_node(
    base: PenNodeBase,
    width: SizingBehavior,
    height: SizingBehavior,
    fill: Option<Vec<PenFill>>,
    stroke: Option<PenStroke>,
    effects: Option<Vec<PenEffect>>,
    start_angle: Option<f64>,
    sweep_angle: Option<f64>,
    inner_radius: Option<f64>,
) -> PenNode {
    PenNode::Ellipse(EllipseNode {
        base,
        width: Some(width),
        height: Some(height),
        corner_radius: None,
        inner_radius,
        start_angle,
        sweep_angle,
        fill,
        stroke,
        effects,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

pub fn line_node(
    base: PenNodeBase,
    x2: f64,
    y2: f64,
    stroke: Option<PenStroke>,
    effects: Option<Vec<PenEffect>>,
) -> PenNode {
    PenNode::Line(LineNode {
        base,
        x2: Some(x2),
        y2: Some(y2),
        stroke,
        effects,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn path_node(
    base: PenNodeBase,
    d: Option<String>,
    icon_id: Option<String>,
    width: SizingBehavior,
    height: SizingBehavior,
    fill: Option<Vec<PenFill>>,
    stroke: Option<PenStroke>,
    effects: Option<Vec<PenEffect>>,
) -> PenNode {
    PenNode::Path(PathNode {
        base,
        icon_id,
        d,
        anchors: None,
        closed: None,
        width: Some(width),
        height: Some(height),
        fill,
        stroke,
        effects,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// Text — spreads the mapped [`TextProps`] onto a `TextNode`.
pub fn text_node(
    base: PenNodeBase,
    width: SizingBehavior,
    height: SizingBehavior,
    props: TextProps,
    fill: Option<Vec<PenFill>>,
    effects: Option<Vec<PenEffect>>,
) -> PenNode {
    let font_style = props.font_style.map(|s| match s {
        StyleFontStyle::Normal => TextFontStyle::Normal,
        StyleFontStyle::Italic => TextFontStyle::Italic,
    });
    PenNode::Text(TextNode {
        base,
        width: Some(width),
        height: Some(height),
        content: props.content,
        font_family: props.font_family,
        font_size: props.font_size,
        font_weight: props.font_weight.map(FontWeight::Number),
        font_style,
        letter_spacing: props.letter_spacing,
        line_height: props.line_height,
        text_align: props.text_align,
        text_align_vertical: props.text_align_vertical,
        text_growth: props.text_growth,
        underline: props.underline,
        strikethrough: props.strikethrough,
        fill,
        effects,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}

/// Component-instance reference.
pub fn ref_node(base: PenNodeBase, target: String) -> PenNode {
    PenNode::Ref(RefNode {
        base,
        target,
        descendants: None,
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    })
}
