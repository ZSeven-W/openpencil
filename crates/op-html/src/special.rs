use crate::import_warning::ImportWarning;
use jian_ops_schema::node::base::{BoolOrExpression, NumberOrExpression, PenNodeBase};
use jian_ops_schema::node::container::{ContainerProps, CornerRadius};
use jian_ops_schema::node::{
    CheckboxNode, FrameNode, PenNode, ProgressNode, RadioGroupNode, RectangleNode, SelectNode,
    SelectOption, SliderNode, TextAreaNode, TextInputNode,
};
use jian_ops_schema::sizing::{SizeLimits, SizingBehavior, SizingKeyword};
use jian_ops_schema::style::{PenEffect, PenFill, PenStroke, SolidFillBody};

use crate::css::cascade::ComputedStyle;
use crate::dom::{DomElement, DomNode};
use crate::mapper::{container_props_from, MapCtx};

#[path = "special_image.rs"]
mod image;
#[cfg(test)]
pub(crate) use image::apply_intrinsic_axes;
use image::{map_image, map_svg};

/// `path` carries the ancestor chain because a replaced element can depend on
/// its parent: an `<img>` inside `<picture>` selects its source from the
/// sibling `<source>` list.
pub fn map_special(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
) -> Option<Option<PenNode>> {
    let element = *path.last()?;
    let node = match element.tag.as_str() {
        "img" => map_image(context, path, element, style),
        "svg" => map_svg(context, element, style),
        "input" => map_input(context, element, style),
        "textarea" => map_text_area(context, element, style),
        "select" => map_select(context, element, style),
        "progress" => map_progress(context, element, style),
        "hr" => map_horizontal_rule(context, style),
        "iframe" | "video" | "canvas" => map_placeholder(context, element, style),
        _ => return None,
    };
    Some(Some(node))
}

pub(crate) fn is_special_leaf_tag(tag: &str) -> bool {
    matches!(
        tag,
        "img"
            | "svg"
            | "input"
            | "textarea"
            | "select"
            | "progress"
            | "hr"
            | "iframe"
            | "video"
            | "canvas"
    )
}

pub(crate) fn map_radio_group(
    context: &mut MapCtx<'_>,
    path: &[&DomElement],
    style: &ComputedStyle,
) -> Option<Option<PenNode>> {
    let current = *path.last()?;
    if current.tag != "input" || current.attr("type") != Some("radio") {
        return None;
    }
    let name = current.attr("name");
    let radios: Vec<&DomElement> = path
        .get(path.len().saturating_sub(2))
        .map(|parent| {
            parent
                .children
                .iter()
                .filter_map(|child| match child {
                    DomNode::Element(element)
                        if element.tag == "input"
                            && element.attr("type") == Some("radio")
                            && element.attr("name") == name =>
                    {
                        Some(element)
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_else(|| vec![current]);
    if radios
        .first()
        .is_some_and(|first| !std::ptr::eq(*first, current))
    {
        return Some(None);
    }
    let options = radios
        .iter()
        .map(|radio| {
            let value = radio.attr("value").unwrap_or("on").to_string();
            SelectOption {
                label: radio.attr("aria-label").unwrap_or(&value).to_string(),
                value,
            }
        })
        .collect();
    let value = radios
        .iter()
        .find(|radio| has_attr(radio, "checked"))
        .and_then(|radio| radio.attr("value"))
        .map(str::to_string);
    let visual = visual_props(context, style);
    let node = PenNode::RadioGroup(RadioGroupNode {
        base: base(context, "radio_group", style),
        width: visual.width,
        height: visual.height,
        limits: visual.limits,
        value,
        options: Some(options),
        fill: visual.fill,
        stroke: visual.stroke,
        effects: visual.effects,
        corner_radius: visual.corner_radius,
        ..Default::default()
    });
    Some(Some(finish(context, node)))
}

struct VisualProps {
    width: Option<SizingBehavior>,
    height: Option<SizingBehavior>,
    limits: SizeLimits,
    fill: Option<Vec<PenFill>>,
    stroke: Option<PenStroke>,
    effects: Option<Vec<PenEffect>>,
    corner_radius: Option<CornerRadius>,
}

fn visual_props(context: &mut MapCtx<'_>, style: &ComputedStyle) -> VisualProps {
    let container = container_props_from(style, context);
    VisualProps {
        width: container.width,
        height: container.height,
        limits: container.limits,
        fill: container.fill,
        stroke: container.stroke,
        effects: container.effects,
        corner_radius: container.corner_radius,
    }
}

/// Every replaced element ends here, so the in-flow outcome `apply_base_style`
/// produces is parked on the context for `map_element::finish_leaf` to drain.
fn base(context: &mut MapCtx<'_>, name: &str, style: &ComputedStyle) -> PenNodeBase {
    let mut base = PenNodeBase {
        id: context.generate_id(),
        name: Some(name.to_string()),
        ..Default::default()
    };
    crate::mapper::apply_base_style(&mut base, style, context);
    base
}

fn base_with_sizing(
    context: &mut MapCtx<'_>,
    name: &str,
    style: &ComputedStyle,
    width: &mut Option<SizingBehavior>,
    height: &mut Option<SizingBehavior>,
    limits: SizeLimits,
) -> PenNodeBase {
    let mut sizing = ContainerProps {
        width: width.take(),
        height: height.take(),
        limits,
        ..Default::default()
    };
    let mut base = PenNodeBase {
        id: context.generate_id(),
        name: Some(name.to_string()),
        ..Default::default()
    };
    let outcome =
        crate::mapper::apply_base_style_with_box(&mut base, style, context, Some(&mut sizing));
    context.pending_base_outcome = outcome;
    *width = sizing.width;
    *height = sizing.height;
    base
}

fn finish(context: &mut MapCtx<'_>, node: PenNode) -> PenNode {
    context.node_count += 1;
    node
}

fn map_input(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    let input_type = element.attr("type").unwrap_or("text").to_ascii_lowercase();
    let visual = visual_props(context, style);
    let node = match input_type.as_str() {
        "text" | "email" | "password" | "search" | "url" | "tel" => {
            PenNode::TextInput(TextInputNode {
                base: base(context, "input", style),
                width: visual.width,
                height: visual.height,
                limits: visual.limits,
                placeholder: element.attr("placeholder").map(str::to_string),
                value: element.attr("value").map(str::to_string),
                fill: visual.fill,
                stroke: visual.stroke,
                effects: visual.effects,
                corner_radius: visual.corner_radius,
                ..Default::default()
            })
        }
        "checkbox" => PenNode::Checkbox(CheckboxNode {
            base: base(context, "checkbox", style),
            width: visual.width,
            height: visual.height,
            limits: visual.limits,
            checked: Some(BoolOrExpression::Bool(has_attr(element, "checked"))),
            label: None,
            fill: visual.fill,
            stroke: visual.stroke,
            effects: visual.effects,
            corner_radius: visual.corner_radius,
            ..Default::default()
        }),
        "range" => PenNode::Slider(SliderNode {
            base: base(context, "slider", style),
            width: visual.width,
            height: visual.height,
            limits: visual.limits,
            min: float_attr(element, "min"),
            max: float_attr(element, "max"),
            step: float_attr(element, "step"),
            value: float_attr(element, "value").map(NumberOrExpression::Number),
            fill: visual.fill,
            stroke: visual.stroke,
            effects: visual.effects,
            corner_radius: visual.corner_radius,
            ..Default::default()
        }),
        "radio" => PenNode::RadioGroup(RadioGroupNode {
            base: base(context, "radio_group", style),
            width: visual.width,
            height: visual.height,
            limits: visual.limits,
            value: has_attr(element, "checked")
                .then(|| element.attr("value").unwrap_or("on").to_string()),
            options: Some(vec![SelectOption {
                value: element.attr("value").unwrap_or("on").to_string(),
                label: element.attr("aria-label").unwrap_or("on").to_string(),
            }]),
            fill: visual.fill,
            stroke: visual.stroke,
            effects: visual.effects,
            corner_radius: visual.corner_radius,
            ..Default::default()
        }),
        _ => {
            context.warn_once(ImportWarning::InputTypeFallback);
            PenNode::TextInput(TextInputNode {
                base: base(context, "input", style),
                width: visual.width,
                height: visual.height,
                limits: visual.limits,
                placeholder: element.attr("placeholder").map(str::to_string),
                value: element.attr("value").map(str::to_string),
                fill: visual.fill,
                stroke: visual.stroke,
                effects: visual.effects,
                corner_radius: visual.corner_radius,
                ..Default::default()
            })
        }
    };
    finish(context, node)
}

fn map_text_area(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    let visual = visual_props(context, style);
    let node = PenNode::TextArea(TextAreaNode {
        base: base(context, "textarea", style),
        width: visual.width,
        height: visual.height,
        limits: visual.limits,
        placeholder: element.attr("placeholder").map(str::to_string),
        value: Some(element_text(element)),
        fill: visual.fill,
        stroke: visual.stroke,
        effects: visual.effects,
        corner_radius: visual.corner_radius,
        ..Default::default()
    });
    finish(context, node)
}

fn map_select(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    let visual = visual_props(context, style);
    let options: Vec<_> = element
        .children
        .iter()
        .filter_map(|child| match child {
            DomNode::Element(option) if option.tag == "option" => Some(option),
            _ => None,
        })
        .collect();
    let value = options
        .iter()
        .find(|option| has_attr(option, "selected"))
        .map(|option| {
            option
                .attr("value")
                .unwrap_or(&element_text(option))
                .to_string()
        });
    let options = options
        .into_iter()
        .map(|option| {
            let label = element_text(option);
            SelectOption {
                value: option.attr("value").unwrap_or(&label).to_string(),
                label,
            }
        })
        .collect();
    let node = PenNode::Select(SelectNode {
        base: base(context, "select", style),
        width: visual.width,
        height: visual.height,
        limits: visual.limits,
        placeholder: element.attr("placeholder").map(str::to_string),
        value,
        options: Some(options),
        fill: visual.fill,
        stroke: visual.stroke,
        effects: visual.effects,
        corner_radius: visual.corner_radius,
        ..Default::default()
    });
    finish(context, node)
}

fn map_progress(context: &mut MapCtx<'_>, element: &DomElement, style: &ComputedStyle) -> PenNode {
    let visual = visual_props(context, style);
    let node = PenNode::Progress(ProgressNode {
        base: base(context, "progress", style),
        width: visual.width,
        height: visual.height,
        limits: visual.limits,
        value: float_attr(element, "value").map(NumberOrExpression::Number),
        max: float_attr(element, "max"),
        fill: visual.fill,
        stroke: visual.stroke,
        effects: visual.effects,
        corner_radius: visual.corner_radius,
        ..Default::default()
    });
    finish(context, node)
}

fn map_horizontal_rule(context: &mut MapCtx<'_>, style: &ComputedStyle) -> PenNode {
    let mut container = container_props_from(style, context);
    container.width = container
        .width
        .or(Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)));
    container.height = container.height.or(Some(SizingBehavior::Number(1.0)));
    container.fill = container.fill.or_else(|| Some(vec![solid_fill("#e0e0e0")]));
    let node = PenNode::Rectangle(RectangleNode {
        base: base(context, "hr", style),
        container,
        children: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
    });
    finish(context, node)
}

fn map_placeholder(
    context: &mut MapCtx<'_>,
    element: &DomElement,
    style: &ComputedStyle,
) -> PenNode {
    context.warn_once(ImportWarning::ElementPlaceholder {
        tag: element.tag.clone(),
    });
    let mut container = container_props_from(style, context);
    container.width = container.width.or_else(|| numeric_attr(element, "width"));
    container.height = container.height.or_else(|| numeric_attr(element, "height"));
    if container.fill.is_none() {
        container.fill = Some(vec![solid_fill("#f0f0f0")]);
    }
    let node = PenNode::Frame(FrameNode {
        base: base(context, &element.tag, style),
        container,
        children: Some(Vec::new()),
        image_search_query: None,
        reusable: None,
        slot: None,
        state: None,
        bindings: None,
        events: None,
        lifecycle: None,
        semantics: None,
        gestures: None,
        route: None,
        screen: None,
        breakpoint: None,
    });
    finish(context, node)
}

fn numeric_attr(element: &DomElement, name: &str) -> Option<SizingBehavior> {
    float_attr(element, name).map(SizingBehavior::Number)
}

fn float_attr(element: &DomElement, name: &str) -> Option<f64> {
    element.attr(name)?.parse().ok()
}

fn has_attr(element: &DomElement, name: &str) -> bool {
    element.attrs.iter().any(|(attr, _)| attr == name)
}

fn element_text(element: &DomElement) -> String {
    element
        .children
        .iter()
        .map(|child| match child {
            DomNode::Text(text) => text.clone(),
            DomNode::Element(child) => element_text(child),
        })
        .collect()
}

fn solid_fill(color: &str) -> PenFill {
    PenFill::Solid(SolidFillBody {
        color: color.to_string(),
        explain: None,
        opacity: None,
        blend_mode: None,
    })
}

#[cfg(test)]
#[path = "special_tests.rs"]
mod tests;
