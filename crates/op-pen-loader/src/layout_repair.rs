//! Post-process jian/taffy layout rects for generated fit-content stacks.
//!
//! Taffy 0.5 can under-report the own height of nested auto/flex
//! containers whose text children later resolve taller than the
//! container's cross-axis contribution. The child text rects are
//! correct, but following siblings start from the too-small parent
//! bottom. Before converting to paint payloads, reconcile those
//! container rects and reflow start-aligned in-flow siblings.

use std::collections::BTreeMap;

use jian_ops_schema::node::base::NumberOrExpression;
use jian_ops_schema::node::container::{ContainerProps, JustifyContent, LayoutMode, Padding};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};

pub(crate) fn repair_fit_content_layout(root: &PenNode, rects: &mut BTreeMap<String, [f32; 4]>) {
    repair_node(root, rects);
}

fn repair_node(node: &PenNode, rects: &mut BTreeMap<String, [f32; 4]>) {
    if let Some(children) = children(node) {
        for child in children {
            repair_node(child, rects);
        }
    }

    let Some(props) = container_props(node) else {
        return;
    };
    let Some(kids) = children(node) else {
        return;
    };
    if kids.is_empty() {
        return;
    }

    match props.layout.as_ref() {
        Some(LayoutMode::Vertical) if start_justified(props) => {
            repair_vertical_container(node, props, kids, rects);
        }
        Some(LayoutMode::Horizontal) if start_justified(props) => {
            repair_horizontal_container(node, props, kids, rects);
        }
        _ => repair_container_to_child_bounds(node, props, kids, rects),
    }
}

fn repair_vertical_container(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
) {
    let Some(parent) = rect(node, rects) else {
        return;
    };
    let padding = padding_sides(props.padding.as_ref());
    let gap = gap_value(props);
    let mut cursor = parent.y + padding.top;
    let mut bottom = cursor;

    for child in kids.iter().filter(|child| in_flow(child)) {
        let Some(child_rect) = rect(child, rects) else {
            continue;
        };
        let dy = cursor - child_rect.y;
        if dy.abs() > 0.5 {
            shift_subtree(child, rects, 0.0, dy);
        }
        if let Some(updated) = rect(child, rects) {
            bottom = updated.y + updated.h;
            cursor = bottom + gap;
        }
    }

    let desired_h = bottom - parent.y + padding.bottom;
    if height_can_follow_content(props) && desired_h > parent.h + 0.5 {
        set_height(node, rects, desired_h);
    }
    repair_container_to_child_bounds(node, props, kids, rects);
}

fn repair_horizontal_container(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
) {
    let Some(parent) = rect(node, rects) else {
        return;
    };
    let padding = padding_sides(props.padding.as_ref());
    let gap = gap_value(props);
    let mut cursor = parent.x + padding.left;
    let mut right = cursor;
    let mut bottom = parent.y + padding.top;

    for child in kids.iter().filter(|child| in_flow(child)) {
        let Some(child_rect) = rect(child, rects) else {
            continue;
        };
        let dx = cursor - child_rect.x;
        if dx.abs() > 0.5 {
            shift_subtree(child, rects, dx, 0.0);
        }
        if let Some(updated) = rect(child, rects) {
            right = updated.x + updated.w;
            bottom = bottom.max(updated.y + updated.h);
            cursor = right + gap;
        }
    }

    let desired_w = right - parent.x + padding.right;
    if width_can_follow_content(props) && desired_w > parent.w + 0.5 {
        set_width(node, rects, desired_w);
    }
    let desired_h = bottom - parent.y + padding.bottom;
    if height_can_follow_content(props) && desired_h > parent.h + 0.5 {
        set_height(node, rects, desired_h);
    }
    repair_container_to_child_bounds(node, props, kids, rects);
}

fn repair_container_to_child_bounds(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
) {
    if !height_can_follow_content(props) && !width_can_follow_content(props) {
        return;
    }
    let Some(parent) = rect(node, rects) else {
        return;
    };
    let padding = padding_sides(props.padding.as_ref());
    let mut max_right = parent.x + padding.left;
    let mut max_bottom = parent.y + padding.top;
    for child in kids.iter().filter(|child| in_flow(child)) {
        if let Some(child_rect) = rect(child, rects) {
            max_right = max_right.max(child_rect.x + child_rect.w);
            max_bottom = max_bottom.max(child_rect.y + child_rect.h);
        }
    }
    if width_can_follow_content(props) {
        let desired_w = max_right - parent.x + padding.right;
        if desired_w > parent.w + 0.5 {
            set_width(node, rects, desired_w);
        }
    }
    if height_can_follow_content(props) {
        let desired_h = max_bottom - parent.y + padding.bottom;
        if desired_h > parent.h + 0.5 {
            set_height(node, rects, desired_h);
        }
    }
}

fn shift_subtree(node: &PenNode, rects: &mut BTreeMap<String, [f32; 4]>, dx: f32, dy: f32) {
    if let Some(r) = rects.get_mut(node_id(node)) {
        r[0] += dx;
        r[1] += dy;
    }
    if let Some(kids) = children(node) {
        for child in kids {
            shift_subtree(child, rects, dx, dy);
        }
    }
}

#[derive(Clone, Copy)]
struct Rect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

#[derive(Clone, Copy)]
struct Sides {
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
}

fn rect(node: &PenNode, rects: &BTreeMap<String, [f32; 4]>) -> Option<Rect> {
    rects.get(node_id(node)).map(|r| Rect {
        x: r[0],
        y: r[1],
        w: r[2],
        h: r[3],
    })
}

fn set_width(node: &PenNode, rects: &mut BTreeMap<String, [f32; 4]>, width: f32) {
    if let Some(r) = rects.get_mut(node_id(node)) {
        r[2] = width;
    }
}

fn set_height(node: &PenNode, rects: &mut BTreeMap<String, [f32; 4]>, height: f32) {
    if let Some(r) = rects.get_mut(node_id(node)) {
        r[3] = height;
    }
}

fn start_justified(props: &ContainerProps) -> bool {
    matches!(props.justify_content, None | Some(JustifyContent::Start))
}

fn width_can_follow_content(props: &ContainerProps) -> bool {
    can_follow_content(props.width.as_ref())
}

fn height_can_follow_content(props: &ContainerProps) -> bool {
    can_follow_content(props.height.as_ref())
}

fn can_follow_content(sizing: Option<&SizingBehavior>) -> bool {
    matches!(
        sizing,
        None | Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    )
}

fn gap_value(props: &ContainerProps) -> f32 {
    match props.gap.as_ref() {
        Some(NumberOrExpression::Number(v)) => *v as f32,
        _ => 0.0,
    }
}

fn padding_sides(padding: Option<&Padding>) -> Sides {
    match padding {
        Some(Padding::Uniform(v)) => {
            let v = *v as f32;
            Sides {
                top: v,
                right: v,
                bottom: v,
                left: v,
            }
        }
        Some(Padding::XY([v, h])) => Sides {
            top: *v as f32,
            right: *h as f32,
            bottom: *v as f32,
            left: *h as f32,
        },
        Some(Padding::LtrB([t, r, b, l])) => Sides {
            top: *t as f32,
            right: *r as f32,
            bottom: *b as f32,
            left: *l as f32,
        },
        _ => Sides {
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
            left: 0.0,
        },
    }
}

fn in_flow(node: &PenNode) -> bool {
    let Some(base) = base(node) else {
        return true;
    };
    base.x.is_none() && base.y.is_none()
}

fn node_id(node: &PenNode) -> &str {
    base(node).map(|b| b.id.as_str()).unwrap_or("")
}

fn base(node: &PenNode) -> Option<&jian_ops_schema::node::base::PenNodeBase> {
    Some(match node {
        PenNode::Frame(n) => &n.base,
        PenNode::Group(n) => &n.base,
        PenNode::Rectangle(n) => &n.base,
        PenNode::Text(n) => &n.base,
        PenNode::TextInput(n) => &n.base,
        PenNode::TextArea(n) => &n.base,
        PenNode::Select(n) => &n.base,
        PenNode::Switch(n) => &n.base,
        PenNode::Checkbox(n) => &n.base,
        PenNode::Slider(n) => &n.base,
        PenNode::RadioGroup(n) => &n.base,
        PenNode::NumberInput(n) => &n.base,
        PenNode::Progress(n) => &n.base,
        PenNode::Tabs(n) => &n.base,
        PenNode::IconFont(n) => &n.base,
        PenNode::Image(n) => &n.base,
        PenNode::Ellipse(n) => &n.base,
        PenNode::Line(n) => &n.base,
        PenNode::Path(n) => &n.base,
        PenNode::Polygon(n) => &n.base,
        PenNode::Ref(n) => &n.base,
    })
}

fn container_props(node: &PenNode) -> Option<&ContainerProps> {
    match node {
        PenNode::Frame(n) => Some(&n.container),
        PenNode::Group(n) => Some(&n.container),
        PenNode::Rectangle(n) => Some(&n.container),
        _ => None,
    }
}

fn children(node: &PenNode) -> Option<&[PenNode]> {
    match node {
        PenNode::Frame(n) => n.children.as_deref(),
        PenNode::Group(n) => n.children.as_deref(),
        PenNode::Rectangle(n) => n.children.as_deref(),
        PenNode::Tabs(n) => n.children.as_deref(),
        _ => None,
    }
}
