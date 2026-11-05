//! Figma auto-layout child positioning.
//!
//! Figma omits `stackPositioning` for the default `AUTO` value and
//! writes `ABSOLUTE` only for children that must stay out of their
//! parent's auto-layout flow. Jian uses authored `x` / `y` as its
//! absolute-position signal, so OpenPencil layout mode must clear
//! those coordinates from ordinary flow children while retaining
//! them (plus constraints) for absolute children.

use crate::common::FigLayoutMode;
use crate::kiwi::FigValue;
use jian_ops_schema::constraints::{Constraints, HConstraint, VConstraint};
use jian_ops_schema::node::{PenNode, PenNodeBase};

fn is_auto_layout(mode: Option<&str>) -> bool {
    matches!(mode, Some("HORIZONTAL" | "VERTICAL"))
}

fn is_absolute_child(figma: &FigValue, parent_stack_mode: Option<&str>) -> bool {
    is_auto_layout(parent_stack_mode) && figma.get_str("stackPositioning") == Some("ABSOLUTE")
}

/// Absolute children do not inherit their auto-layout parent's grow
/// or stretch sizing. Passing no parent mode preserves their own
/// Hug/Fit Content sizing while disabling only parent-axis rules.
pub(crate) fn sizing_parent_stack_mode<'a>(
    figma: &FigValue,
    parent_stack_mode: Option<&'a str>,
    layout_mode: FigLayoutMode,
) -> Option<&'a str> {
    match layout_mode {
        // Preserve promises fixed authored geometry; never expose a
        // parent layout mode to grow/stretch sizing resolution.
        FigLayoutMode::Preserve => None,
        FigLayoutMode::OpenPencil if is_absolute_child(figma, parent_stack_mode) => None,
        FigLayoutMode::OpenPencil => parent_stack_mode,
    }
}

/// Apply the positioning contract after a node has been converted.
///
/// Preserve mode keeps every authored coordinate because its canvas
/// path bypasses flex layout. It still records constraints for an
/// explicit absolute child so that intent survives a later switch to
/// layout-resolved editing. OpenPencil mode clears coordinates from
/// default/AUTO children so they participate in flex, and retains
/// coordinates plus constraints only for ABSOLUTE children.
pub(crate) fn apply_layout_positioning(
    node: &mut PenNode,
    figma: &FigValue,
    parent_stack_mode: Option<&str>,
    layout_mode: FigLayoutMode,
) {
    if !is_auto_layout(parent_stack_mode) {
        return;
    }

    let absolute = is_absolute_child(figma, parent_stack_mode);
    let base = base_mut(node);
    if layout_mode == FigLayoutMode::OpenPencil && !absolute {
        base.x = None;
        base.y = None;
        base.constraints = None;
    } else if absolute {
        base.constraints = Some(map_figma_constraints(figma));
    }
}

/// Auto-layout flow order is ascending while the tree builder stores
/// descending paint order (topmost first). OpenPencil mode clears authored
/// coordinates and feeds children to flex layout, so it must emit flow order.
/// Preserve mode keeps authored coordinates and bypasses flex layout; retaining
/// topmost-first order is therefore required for canonical canvas z-order.
///
/// The `.op` children array is BOTH flow order and paint order (first =
/// topmost), so only the flow children are reversed: an ABSOLUTE child (the
/// only kind that keeps `constraints` in OpenPencil mode) stays in its
/// topmost-first slot. Reversing everything sank a mask or action sheet that
/// was topmost in Figma beneath the content it overlays (issue #276).
pub(crate) fn order_children(
    children: Vec<PenNode>,
    has_auto_layout: bool,
    layout_mode: FigLayoutMode,
) -> Vec<PenNode> {
    if layout_mode != FigLayoutMode::OpenPencil || !has_auto_layout || children.len() < 2 {
        return children;
    }
    let mut children = children;
    let absolute: Vec<bool> = children
        .iter_mut()
        .map(|child| base_mut(child).constraints.is_some())
        .collect();
    let mut slots: Vec<Option<PenNode>> = children.into_iter().map(Some).collect();
    // Flow children, topmost-first → reversed into flow order.
    let mut flow: Vec<PenNode> = slots
        .iter_mut()
        .zip(&absolute)
        .filter(|(_, abs)| !**abs)
        .filter_map(|(slot, _)| slot.take())
        .collect();
    flow.reverse();
    let mut flow = flow.into_iter();
    slots
        .into_iter()
        .zip(absolute)
        .filter_map(|(slot, abs)| if abs { slot } else { flow.next() })
        .collect()
}

fn map_figma_constraints(figma: &FigValue) -> Constraints {
    Constraints {
        h: match figma.get_str("horizontalConstraint") {
            Some("CENTER") => HConstraint::Center,
            Some("MAX" | "FIXED_MAX") => HConstraint::Right,
            Some("STRETCH") => HConstraint::LeftRight,
            Some("SCALE") => HConstraint::Scale,
            _ => HConstraint::Left,
        },
        v: match figma.get_str("verticalConstraint") {
            Some("CENTER") => VConstraint::Center,
            Some("MAX" | "FIXED_MAX") => VConstraint::Bottom,
            Some("STRETCH") => VConstraint::TopBottom,
            Some("SCALE") => VConstraint::Scale,
            _ => VConstraint::Top,
        },
    }
}

fn base_mut(node: &mut PenNode) -> &mut PenNodeBase {
    match node {
        PenNode::Frame(node) => &mut node.base,
        PenNode::Group(node) => &mut node.base,
        PenNode::Rectangle(node) => &mut node.base,
        PenNode::Ellipse(node) => &mut node.base,
        PenNode::Line(node) => &mut node.base,
        PenNode::Polygon(node) => &mut node.base,
        PenNode::Path(node) => &mut node.base,
        PenNode::Text(node) => &mut node.base,
        PenNode::TextInput(node) => &mut node.base,
        PenNode::TextArea(node) => &mut node.base,
        PenNode::Select(node) => &mut node.base,
        PenNode::Switch(node) => &mut node.base,
        PenNode::Checkbox(node) => &mut node.base,
        PenNode::Slider(node) => &mut node.base,
        PenNode::RadioGroup(node) => &mut node.base,
        PenNode::NumberInput(node) => &mut node.base,
        PenNode::Progress(node) => &mut node.base,
        PenNode::Tabs(node) => &mut node.base,
        PenNode::Image(node) => &mut node.base,
        PenNode::IconFont(node) => &mut node.base,
        PenNode::Ref(node) => &mut node.base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obj(fields: Vec<(&str, FigValue)>) -> FigValue {
        FigValue::Object(
            fields
                .into_iter()
                .map(|(key, value)| (key.into(), value))
                .collect(),
        )
    }

    #[test]
    fn maps_figma_constraint_variants() {
        let cases = [
            (None, HConstraint::Left, VConstraint::Top),
            (Some("MIN"), HConstraint::Left, VConstraint::Top),
            (Some("FIXED_MIN"), HConstraint::Left, VConstraint::Top),
            (Some("CENTER"), HConstraint::Center, VConstraint::Center),
            (Some("MAX"), HConstraint::Right, VConstraint::Bottom),
            (Some("FIXED_MAX"), HConstraint::Right, VConstraint::Bottom),
            (
                Some("STRETCH"),
                HConstraint::LeftRight,
                VConstraint::TopBottom,
            ),
            (Some("SCALE"), HConstraint::Scale, VConstraint::Scale),
        ];

        for (raw, expected_h, expected_v) in cases {
            let mut fields = Vec::new();
            if let Some(raw) = raw {
                fields.push(("horizontalConstraint", FigValue::Str(raw.into())));
                fields.push(("verticalConstraint", FigValue::Str(raw.into())));
            }
            let constraints = map_figma_constraints(&obj(fields));
            assert_eq!(constraints.h, expected_h, "horizontal {raw:?}");
            assert_eq!(constraints.v, expected_v, "vertical {raw:?}");
        }
    }

    #[test]
    fn preserve_never_exposes_parent_grow_or_stretch_mode() {
        let flow = obj(Vec::new());
        let absolute = obj(vec![("stackPositioning", FigValue::Str("ABSOLUTE".into()))]);
        assert_eq!(
            sizing_parent_stack_mode(&flow, Some("VERTICAL"), FigLayoutMode::Preserve),
            None
        );
        assert_eq!(
            sizing_parent_stack_mode(&absolute, Some("VERTICAL"), FigLayoutMode::Preserve),
            None
        );
        assert_eq!(
            sizing_parent_stack_mode(&flow, Some("VERTICAL"), FigLayoutMode::OpenPencil),
            Some("VERTICAL")
        );
        assert_eq!(
            sizing_parent_stack_mode(&absolute, Some("VERTICAL"), FigLayoutMode::OpenPencil),
            None
        );
    }
}
