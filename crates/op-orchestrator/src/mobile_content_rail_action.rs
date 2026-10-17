//! Preserve full-width persistent actions whose inner shell already owns a rail.
use super::*;
use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::style::StrokeThickness;

pub(super) fn is_gutter_owned_trailing_action(
    node: &PenNode,
    index: usize,
    section_count: usize,
    root_width: f64,
) -> bool {
    if index + 1 != section_count || !crate::mobile_action_bar::is_action_bar(node) {
        return false;
    }
    let Some(props) = container_props(node) else {
        return false;
    };
    if !has_zero_corner_radius(props.corner_radius.as_ref())
        || props
            .effects
            .as_ref()
            .is_some_and(|effects| !effects.is_empty())
    {
        return false;
    }
    let Some(stroke) = props.stroke.as_ref() else {
        return false;
    };
    let [top, right, bottom, left] = match &stroke.thickness {
        StrokeThickness::Uniform(_) => return false,
        StrokeThickness::PerSide(sides) => *sides,
        StrokeThickness::Sided(sides) => [
            sides.top.unwrap_or(0.0),
            sides.right.unwrap_or(0.0),
            sides.bottom.unwrap_or(0.0),
            sides.left.unwrap_or(0.0),
        ],
    };
    // A thin horizontal separator is chrome, not a four-sided card border.
    right == 0.0
        && left == 0.0
        && (top > 0.0 || bottom > 0.0)
        && (0.0..=2.0).contains(&top)
        && (0.0..=2.0).contains(&bottom)
        && owns_structural_action_gutter(node, root_width, 0)
}

fn has_zero_corner_radius(radius: Option<&CornerRadius>) -> bool {
    match radius {
        None => true,
        Some(CornerRadius::Uniform(radius)) => *radius == 0.0,
        Some(CornerRadius::PerCorner(radii)) => radii.iter().all(|radius| *radius == 0.0),
    }
}

fn owns_structural_action_gutter(node: &PenNode, root_width: f64, depth: usize) -> bool {
    let Some(props) = container_props(node) else {
        return false;
    };
    if depth > 4
        // The shared transparency helper only reads uniform numeric radii;
        // a nested rounded/clipped card cannot own this chrome's page gutter.
        || (depth > 0 && !has_zero_corner_radius(props.corner_radius.as_ref()))
        || props.layout != Some(LayoutMode::Vertical)
        || has_authored_position(node)
        || has_expression_padding(node)
        || !matches!(
            props.height,
            None | Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
        )
        || !matches!(
            props.width,
            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
        ) && !matches!(props.width, Some(SizingBehavior::Number(width)) if width >= root_width - 1.0)
    {
        return false;
    }
    if let Some((left, right)) = horizontal_padding(node).filter(|pair| nonzero_pair(*pair)) {
        return (MIN_CONTENT_RAIL..=MAX_CONTENT_RAIL).contains(&left)
            && (left - right).abs() <= 0.5;
    }
    // Follow only one transparent full-width structural owner. A fixed-height
    // button's own label padding is not a page gutter, nor is a card's padding.
    node.children().is_some_and(|children| {
        children.len() == 1
            && is_transparent_surface(&children[0])
            && owns_structural_action_gutter(&children[0], root_width, depth + 1)
    })
}
