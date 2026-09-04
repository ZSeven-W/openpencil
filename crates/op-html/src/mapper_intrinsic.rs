//! Conservative bottom-up sizing for wrappers around one imported image.
//!
//! Browsers give replaced elements an intrinsic box. The schema has no
//! generic intrinsic-size field, so synthetic wrappers (`inline-row`, boxed
//! links, `picture`) otherwise stay `FitContent` and become unmeasurable to
//! the importer's flex-wrap pass even when their sole image is numeric.

use jian_ops_schema::node::container::{ContainerProps, Padding};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

use super::node_access::node_base;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct IntrinsicAxes {
    pub width: Option<f64>,
    pub height: Option<f64>,
}

/// Numeric outer axes of a node chain that ends in exactly one image.
///
/// Dynamic axes stay unknown independently, and positioned/multi-child chains
/// are rejected because their child does not define the wrapper's flow box.
pub(crate) fn single_image_outer_size(nodes: &[PenNode]) -> IntrinsicAxes {
    single_image_chain_size(nodes).unwrap_or_default()
}

fn single_image_chain_size(nodes: &[PenNode]) -> Option<IntrinsicAxes> {
    let [node] = nodes else {
        return None;
    };
    single_node_image_size(node)
}

pub(crate) fn promote_single_image_size(container: &mut ContainerProps, children: &[PenNode]) {
    let child = single_image_outer_size(children);
    let Some((horizontal_padding, vertical_padding)) = padding_axes(container.padding.as_ref())
    else {
        return;
    };
    promote_axis(
        &mut container.width,
        child.width.map(|value| value + horizontal_padding),
        container.limits.min_width,
        container.limits.max_width,
    );
    promote_axis(
        &mut container.height,
        child.height.map(|value| value + vertical_padding),
        container.limits.min_height,
        container.limits.max_height,
    );
}

fn single_node_image_size(node: &PenNode) -> Option<IntrinsicAxes> {
    let base = node_base(node);
    if base.x.is_some() || base.y.is_some() {
        return None;
    }
    match node {
        PenNode::Image(image) => Some(IntrinsicAxes {
            width: numeric_axis(
                image.width.as_ref(),
                image.limits.min_width,
                image.limits.max_width,
            ),
            height: numeric_axis(
                image.height.as_ref(),
                image.limits.min_height,
                image.limits.max_height,
            ),
        }),
        PenNode::Frame(frame) => sized_container(
            &frame.container,
            frame.children.as_deref().unwrap_or_default(),
        ),
        PenNode::Group(group) => sized_container(
            &group.container,
            group.children.as_deref().unwrap_or_default(),
        ),
        PenNode::Rectangle(rectangle) => sized_container(
            &rectangle.container,
            rectangle.children.as_deref().unwrap_or_default(),
        ),
        _ => None,
    }
}

fn sized_container(container: &ContainerProps, children: &[PenNode]) -> Option<IntrinsicAxes> {
    let child = single_image_chain_size(children)?;
    let Some((horizontal_padding, vertical_padding)) = padding_axes(container.padding.as_ref())
    else {
        return Some(IntrinsicAxes::default());
    };
    Some(IntrinsicAxes {
        width: resolved_wrapper_axis(
            container.width.as_ref(),
            child.width.map(|value| value + horizontal_padding),
            container.limits.min_width,
            container.limits.max_width,
        ),
        height: resolved_wrapper_axis(
            container.height.as_ref(),
            child.height.map(|value| value + vertical_padding),
            container.limits.min_height,
            container.limits.max_height,
        ),
    })
}

fn resolved_wrapper_axis(
    axis: Option<&SizingBehavior>,
    child: Option<f64>,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> Option<f64> {
    match axis {
        Some(SizingBehavior::Number(value)) if value.is_finite() && *value >= 0.0 => {
            Some(clamp_axis(*value, minimum, maximum))
        }
        None | Some(SizingBehavior::Keyword(SizingKeyword::FitContent)) => {
            child.map(|value| clamp_axis(value, minimum, maximum))
        }
        _ => None,
    }
}

fn promote_axis(
    axis: &mut Option<SizingBehavior>,
    child: Option<f64>,
    minimum: Option<f64>,
    maximum: Option<f64>,
) {
    if !matches!(
        axis,
        None | Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    ) {
        return;
    }
    if let Some(value) = child {
        *axis = Some(SizingBehavior::Number(clamp_axis(value, minimum, maximum)));
    }
}

fn numeric_axis(
    axis: Option<&SizingBehavior>,
    minimum: Option<f64>,
    maximum: Option<f64>,
) -> Option<f64> {
    match axis {
        Some(SizingBehavior::Number(value)) if value.is_finite() && *value >= 0.0 => {
            Some(clamp_axis(*value, minimum, maximum))
        }
        _ => None,
    }
}

fn clamp_axis(value: f64, minimum: Option<f64>, maximum: Option<f64>) -> f64 {
    let mut value = value;
    if let Some(maximum) = maximum {
        value = value.min(maximum);
    }
    if let Some(minimum) = minimum {
        value = value.max(minimum);
    }
    value.max(0.0)
}

fn padding_axes(padding: Option<&Padding>) -> Option<(f64, f64)> {
    match padding {
        None => Some((0.0, 0.0)),
        Some(Padding::Uniform(value)) => Some((value * 2.0, value * 2.0)),
        Some(Padding::XY([vertical, horizontal])) => Some((horizontal * 2.0, vertical * 2.0)),
        Some(Padding::LtrB([top, right, bottom, left])) => Some((left + right, top + bottom)),
        Some(Padding::Expression(_)) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jian_ops_schema::node::base::PenNodeBase;
    use jian_ops_schema::node::{FrameNode, ImageNode, ImageSrc};
    use jian_ops_schema::sizing::SizeLimits;

    fn image(width: SizingBehavior, height: SizingBehavior) -> PenNode {
        PenNode::Image(ImageNode {
            base: PenNodeBase {
                id: "image".to_string(),
                ..Default::default()
            },
            src: ImageSrc::from("data:image/png;base64,AA=="),
            object_fit: None,
            width: Some(width),
            height: Some(height),
            limits: SizeLimits::default(),
            corner_radius: None,
            effects: None,
            exposure: None,
            contrast: None,
            saturation: None,
            temperature: None,
            tint: None,
            highlights: None,
            shadows: None,
            image_prompt: None,
            image_search_query: None,
            state: None,
            bindings: None,
            events: None,
            lifecycle: None,
            semantics: None,
            gestures: None,
            route: None,
        })
    }

    fn frame(container: ContainerProps, child: PenNode) -> PenNode {
        PenNode::Frame(FrameNode {
            base: PenNodeBase {
                id: "frame".to_string(),
                ..Default::default()
            },
            container,
            children: Some(vec![child]),
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
        })
    }

    #[test]
    fn one_image_chain_adds_padding_and_clamps_auto_axes() {
        let child = image(SizingBehavior::Number(100.0), SizingBehavior::Number(50.0));
        let wrapped = frame(
            ContainerProps {
                width: Some(SizingBehavior::Keyword(SizingKeyword::FitContent)),
                height: Some(SizingBehavior::Keyword(SizingKeyword::FitContent)),
                padding: Some(Padding::LtrB([2.0, 4.0, 6.0, 8.0])),
                limits: SizeLimits {
                    max_width: Some(110.0),
                    ..Default::default()
                },
                ..Default::default()
            },
            child,
        );
        assert_eq!(
            single_image_outer_size(&[wrapped]),
            IntrinsicAxes {
                width: Some(110.0),
                height: Some(58.0)
            }
        );
    }

    #[test]
    fn dynamic_or_positioned_chains_do_not_invent_axes() {
        let dynamic = frame(
            ContainerProps {
                width: Some(SizingBehavior::Keyword(SizingKeyword::FillContainer)),
                height: Some(SizingBehavior::Keyword(SizingKeyword::FitContent)),
                ..Default::default()
            },
            image(SizingBehavior::Number(100.0), SizingBehavior::Number(50.0)),
        );
        assert_eq!(
            single_image_outer_size(&[dynamic]),
            IntrinsicAxes {
                width: None,
                height: Some(50.0)
            }
        );

        let mut positioned = image(SizingBehavior::Number(100.0), SizingBehavior::Number(50.0));
        if let PenNode::Image(image) = &mut positioned {
            image.base.x = Some(1.0);
        }
        assert_eq!(
            single_image_outer_size(&[positioned]),
            IntrinsicAxes::default()
        );
    }
}
