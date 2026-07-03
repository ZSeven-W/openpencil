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
use jian_ops_schema::node::container::{
    AlignItems, ContainerProps, JustifyContent, LayoutMode, Padding,
};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
use jian_ops_schema::style::StrokeThickness;

pub(crate) fn repair_fit_content_layout(root: &PenNode, rects: &mut BTreeMap<String, [f32; 4]>) {
    repair_node(root, rects, true);
}

fn repair_node(node: &PenNode, rects: &mut BTreeMap<String, [f32; 4]>, is_root: bool) {
    if let Some(children) = children(node) {
        for child in children {
            repair_node(child, rects, false);
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
        Some(LayoutMode::Vertical) => {
            repair_vertical_container(node, props, kids, rects, is_root);
        }
        Some(LayoutMode::Horizontal) if start_justified(props) => {
            repair_horizontal_container(node, props, kids, rects);
        }
        None | Some(LayoutMode::None)
            if is_frame(node)
                && should_infer_horizontal_layout(props, kids)
                && !explicit_absolute_intent(props, kids) =>
        {
            repair_inferred_horizontal_container(node, props, kids, rects);
        }
        _ => repair_container_to_child_bounds(node, props, kids, rects),
    }
    repair_split_metric_column_content(node, props, kids, rects);
}

fn repair_vertical_container(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
    is_root: bool,
) {
    let Some(parent) = rect(node, rects) else {
        return;
    };
    let layout_kids: Vec<&PenNode> = kids.iter().filter(|child| layout_child(child)).collect();
    if layout_kids.is_empty() {
        return;
    }
    let padding = padding_sides(props.padding.as_ref());
    let gap = gap_value(props);
    let avail_w = (parent.w - padding.left - padding.right).max(0.0);
    let avail_h = (parent.h - padding.top - padding.bottom).max(0.0);
    let rects_now: Vec<Rect> = layout_kids
        .iter()
        .filter_map(|child| rect(child, rects))
        .collect();
    if rects_now.len() != layout_kids.len() {
        return;
    }

    let total_main: f32 = rects_now.iter().map(|r| r.h).sum();
    let total_gap = gap * layout_kids.len().saturating_sub(1) as f32;
    let free = (avail_h - total_main - total_gap).max(0.0);
    let mut cursor = parent.y
        + padding.top
        + match props.justify_content {
            Some(JustifyContent::Center) => free / 2.0,
            Some(JustifyContent::End) => free,
            Some(JustifyContent::SpaceAround) => {
                (avail_h - total_main) / layout_kids.len() as f32 / 2.0
            }
            _ => 0.0,
        };
    let effective_gap = match props.justify_content {
        Some(JustifyContent::SpaceBetween) if layout_kids.len() > 1 => {
            (avail_h - total_main) / (layout_kids.len() - 1) as f32
        }
        Some(JustifyContent::SpaceAround) => (avail_h - total_main) / layout_kids.len() as f32,
        _ => gap,
    };
    let mut bottom = cursor;

    for (child, child_rect) in layout_kids.iter().zip(rects_now.iter()) {
        let cross_pos = match props.align_items {
            Some(AlignItems::Center) => ((avail_w - child_rect.w) / 2.0).clamp(0.0, avail_w),
            Some(AlignItems::End) => (avail_w - child_rect.w).max(0.0),
            _ => 0.0,
        };
        let target_x = parent.x + padding.left + cross_pos;
        let target_y = cursor;
        let dx = target_x - child_rect.x;
        let dy = target_y - child_rect.y;
        if dx.abs() > 0.5 || dy.abs() > 0.5 {
            shift_subtree(child, rects, dx, dy);
        }
        bottom = target_y + child_rect.h;
        cursor = bottom + effective_gap;
    }

    let desired_h = bottom - parent.y + padding.bottom;
    if height_can_expand_to_content_or_root(props, is_root) && desired_h > parent.h + 0.5 {
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
    let padding = inferred_horizontal_padding(node, props, kids);
    let gap = gap_value(props);
    let mut cursor = parent.x + padding.left;
    let mut right = cursor;
    let mut bottom = parent.y + padding.top;

    for child in kids.iter().filter(|child| layout_child(child)) {
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
    if width_can_expand_to_content(props) && desired_w > parent.w + 0.5 {
        set_width(node, rects, desired_w);
    }
    let desired_h = bottom - parent.y + padding.bottom;
    if height_can_expand_to_content(props) && desired_h > parent.h + 0.5 {
        set_height(node, rects, desired_h);
    }
    repair_container_to_child_bounds(node, props, kids, rects);
}

fn repair_inferred_horizontal_container(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
) {
    let Some(parent) = rect(node, rects) else {
        return;
    };
    let layout_kids: Vec<&PenNode> = kids.iter().filter(|child| layout_child(child)).collect();
    if layout_kids.is_empty() {
        return;
    }

    let padding = inferred_horizontal_padding(node, props, kids);
    let gap = gap_value(props);
    let avail_w = (parent.w - padding.left - padding.right).max(0.0);
    let avail_h = (parent.h - padding.top - padding.bottom).max(0.0);
    let rects_now: Vec<Rect> = layout_kids
        .iter()
        .filter_map(|child| rect(child, rects))
        .collect();
    if rects_now.len() != layout_kids.len() {
        return;
    }

    let center_legacy_badge = is_legacy_fixed_width_text_badge(node, props, kids);
    let total_main: f32 = rects_now.iter().map(|r| r.w).sum();
    let total_gap = gap * layout_kids.len().saturating_sub(1) as f32;
    let free = (avail_w - total_main - total_gap).max(0.0);
    let mut cursor = if center_legacy_badge {
        free / 2.0
    } else {
        match props.justify_content {
            Some(JustifyContent::Center) => free / 2.0,
            Some(JustifyContent::End) => free,
            Some(JustifyContent::SpaceAround) => {
                (avail_w - total_main) / layout_kids.len() as f32 / 2.0
            }
            _ => 0.0,
        }
    };
    let effective_gap = match props.justify_content {
        Some(JustifyContent::SpaceBetween) if layout_kids.len() > 1 => {
            (avail_w - total_main) / (layout_kids.len() - 1) as f32
        }
        Some(JustifyContent::SpaceAround) => (avail_w - total_main) / layout_kids.len() as f32,
        _ => gap,
    };

    for (index, (child, child_rect)) in layout_kids.iter().zip(rects_now.iter()).enumerate() {
        let cross_pos = if center_legacy_badge {
            ((avail_h - child_rect.h) / 2.0).clamp(0.0, avail_h)
        } else {
            match props.align_items {
                Some(AlignItems::Center) => ((avail_h - child_rect.h) / 2.0).clamp(0.0, avail_h),
                Some(AlignItems::End) => (avail_h - child_rect.h).max(0.0),
                _ => 0.0,
            }
        };
        let target_x = parent.x + padding.left + cursor;
        let target_y = parent.y + padding.top + cross_pos;
        let dx = target_x - child_rect.x;
        let dy = target_y - child_rect.y;
        if dx.abs() > 0.5 || dy.abs() > 0.5 {
            shift_subtree(child, rects, dx, dy);
        }
        if can_stretch_inferred_row_card(child) && avail_h > child_rect.h + 0.5 {
            set_height(child, rects, avail_h);
        }
        if index == layout_kids.len() - 1 {
            let max_w = (parent.x + parent.w - padding.right - target_x).max(0.0);
            if child_rect.w > max_w + 0.5 && can_shrink_inferred_row_tail(child) {
                set_width(child, rects, max_w);
            }
        }
        cursor += child_rect.w + effective_gap;
    }

    repair_container_to_child_bounds(node, props, kids, rects);
}

fn repair_container_to_child_bounds(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
) {
    if is_unbounded_group(node, props) {
        return;
    }
    if !height_can_expand_to_content(props) && !width_can_expand_to_content(props) {
        return;
    }
    let Some(parent) = rect(node, rects) else {
        return;
    };
    let padding = padding_sides(props.padding.as_ref());
    let mut max_right = parent.x + padding.left;
    let mut max_bottom = parent.y + padding.top;
    for child in kids.iter().filter(|child| layout_child(child)) {
        if let Some(child_rect) = rect(child, rects) {
            max_right = max_right.max(child_rect.x + child_rect.w);
            max_bottom = max_bottom.max(child_rect.y + child_rect.h);
        }
    }
    if width_can_expand_to_content(props) {
        let desired_w = max_right - parent.x + padding.right;
        if desired_w > parent.w + 0.5 {
            set_width(node, rects, desired_w);
        }
    }
    if height_can_expand_to_content(props) {
        let desired_h = max_bottom - parent.y + padding.bottom;
        if desired_h > parent.h + 0.5 {
            set_height(node, rects, desired_h);
        }
    }
}

fn repair_split_metric_column_content(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
) {
    if !is_frame(node) || props.layout.is_some() {
        return;
    }
    let layout_kids: Vec<&PenNode> = kids.iter().filter(|child| layout_child(child)).collect();
    if layout_kids.len() != 2 {
        return;
    }
    let [left, right] = [layout_kids[0], layout_kids[1]];
    let (Some(left_props), Some(right_props)) = (container_props(left), container_props(right))
    else {
        return;
    };
    if right_props.layout != Some(LayoutMode::Vertical)
        || padding_sides(right_props.padding.as_ref()).left > 0.5
        || right_only_stroke_width(left_props).unwrap_or(0.0) <= 0.0
    {
        return;
    }
    let Some(parent_rect) = rect(node, rects) else {
        return;
    };
    let (Some(left_rect), Some(right_rect)) = (rect(left, rects), rect(right, rects)) else {
        return;
    };
    if (left_rect.w - right_rect.w).abs() > 1.0
        || ((left_rect.x + left_rect.w) - right_rect.x).abs() > 1.0
        || (left_rect.x - parent_rect.x).abs() > 1.0
    {
        return;
    }
    let inset = padding_sides(left_props.padding.as_ref()).right;
    if inset <= 0.5 {
        return;
    }
    let Some(right_children) = children(right) else {
        return;
    };
    let min_child_x = right_children
        .iter()
        .filter(|child| layout_child(child))
        .filter_map(|child| rect(child, rects).map(|r| r.x))
        .fold(f32::INFINITY, f32::min);
    if !min_child_x.is_finite() {
        return;
    }
    let desired_x = right_rect.x + inset;
    if min_child_x >= desired_x - 0.5 {
        return;
    }
    let dx = desired_x - min_child_x;
    for child in right_children.iter().filter(|child| layout_child(child)) {
        shift_subtree(child, rects, dx, 0.0);
    }
}

fn right_only_stroke_width(props: &ContainerProps) -> Option<f32> {
    let thickness = &props.stroke.as_ref()?.thickness;
    let sides = match thickness {
        StrokeThickness::Uniform(_) => return None,
        StrokeThickness::PerSide([top, right, bottom, left]) => [*top, *right, *bottom, *left],
        StrokeThickness::Sided(sides) => [
            sides.top.unwrap_or(0.0),
            sides.right.unwrap_or(0.0),
            sides.bottom.unwrap_or(0.0),
            sides.left.unwrap_or(0.0),
        ],
    };
    let [top, right, bottom, left] = sides;
    (right > 0.0 && top <= 0.0 && bottom <= 0.0 && left <= 0.0).then_some(right)
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
    rects.get(node_id(node)).map(|r| {
        let (authored_w, authored_h) = authored_numeric_size(node);
        Rect {
            x: r[0],
            y: r[1],
            w: if r[2] > 0.0 {
                r[2]
            } else {
                authored_w.unwrap_or(r[2])
            },
            h: if r[3] > 0.0 {
                r[3]
            } else {
                authored_h.unwrap_or(r[3])
            },
        }
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

fn should_infer_horizontal_layout(props: &ContainerProps, kids: &[PenNode]) -> bool {
    props.padding.is_some()
        || props.gap.is_some()
        || props.justify_content.is_some()
        || props.align_items.is_some()
        || kids.iter().any(child_has_fill_container_axis)
}

/// An EXPLICITLY authored `layout: "none"` whose children carry real x/y is
/// absolute placement by spec — a badge dot pinned on a bell icon (x:28, y:8
/// over a 20px icon at 12,12). Models belt-and-suspender these buttons with
/// justifyContent+alignItems too, which used to trip the flow inference and
/// stomp taffy's absolute positions (measured: the badge rendered BESIDE the
/// bell). Legacy TS docs with an ABSENT layout field and stale x/y=0 children
/// still get the inference — their coordinates are flow debris, not intent.
fn explicit_absolute_intent(props: &ContainerProps, kids: &[PenNode]) -> bool {
    matches!(props.layout.as_ref(), Some(LayoutMode::None))
        && kids
            .iter()
            .any(|k| crate::adapter::node_base_xy(k).is_some_and(|(x, y)| x != 0.0 || y != 0.0))
}

fn inferred_horizontal_padding(node: &PenNode, props: &ContainerProps, kids: &[PenNode]) -> Sides {
    let mut padding = padding_sides(props.padding.as_ref());
    if legacy_zero_horizontal_inset_row(node, props, kids) {
        if let Some(Padding::XY([vertical, horizontal])) = props.padding.as_ref() {
            if *vertical > 0.0 && horizontal.abs() < f64::EPSILON {
                let inset = *vertical as f32;
                padding.left = inset;
                padding.right = inset;
            }
        }
    }
    padding
}

fn legacy_zero_horizontal_inset_row(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
) -> bool {
    if props.layout.is_some()
        || props.gap.is_none()
        || props.align_items != Some(AlignItems::Center)
        || !matches!(
            props.padding.as_ref(),
            Some(Padding::XY([vertical, horizontal]))
                if *vertical > 0.0 && horizontal.abs() < f64::EPSILON
        )
    {
        return false;
    }
    if base(node)
        .and_then(|base| base.name.as_deref())
        .map(|name| name.to_ascii_lowercase().starts_with("habit"))
        .unwrap_or(false)
    {
        return false;
    }
    let layout_kids: Vec<&PenNode> = kids.iter().filter(|child| layout_child(child)).collect();
    if layout_kids.len() != 2 {
        return false;
    }
    if layout_kids.iter().any(|child| {
        base(child)
            .and_then(|base| base.name.as_deref())
            .map(|name| name.to_ascii_lowercase().contains("habit"))
            .unwrap_or(false)
    }) {
        return false;
    }
    let (first_w, first_h) = authored_numeric_size(layout_kids[0]);
    if first_w.unwrap_or(f32::MAX) > 32.0 || first_h.unwrap_or(f32::MAX) > 32.0 {
        return false;
    }
    matches!(
        container_props(layout_kids[1]).and_then(|props| props.layout.as_ref()),
        Some(LayoutMode::Vertical)
    )
}

fn can_shrink_inferred_row_tail(node: &PenNode) -> bool {
    let Some(props) = container_props(node) else {
        return false;
    };
    props.clip_content != Some(true) && props.fill.is_none() && props.stroke.is_none()
}

fn can_stretch_inferred_row_card(node: &PenNode) -> bool {
    let PenNode::Frame(frame) = node else {
        return false;
    };
    let has_card_paint = frame.container.fill.is_some() || frame.container.stroke.is_some();
    if !has_card_paint {
        return false;
    }
    frame
        .base
        .name
        .as_deref()
        .map(|name| {
            let name = name.to_ascii_lowercase();
            name.contains("metric") || name.contains("card")
        })
        .unwrap_or(false)
}

fn is_legacy_fixed_width_text_badge(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
) -> bool {
    if props.padding.is_none()
        || props.justify_content.is_some()
        || props.align_items.is_some()
        || kids.len() != 1
        || !is_text_node(&kids[0])
    {
        return false;
    }
    if !matches!(props.width.as_ref(), Some(SizingBehavior::Number(_)))
        || !matches!(props.height.as_ref(), Some(SizingBehavior::Number(_)))
    {
        return false;
    }
    let Some(base) = base(node) else {
        return false;
    };
    let role_is_badge = base
        .role
        .as_deref()
        .map(|role| role.to_ascii_lowercase().contains("badge"))
        .unwrap_or(false);
    let name_is_badge = base
        .name
        .as_deref()
        .map(|name| name.to_ascii_lowercase().contains("badge"))
        .unwrap_or(false);
    role_is_badge || name_is_badge
}

fn is_text_node(node: &PenNode) -> bool {
    matches!(node, PenNode::Text(_))
}

fn height_can_follow_content(props: &ContainerProps) -> bool {
    can_follow_content(props.height.as_ref())
}

fn height_can_expand_to_content(props: &ContainerProps) -> bool {
    height_can_follow_content(props) || props.clip_content != Some(true)
}

fn height_can_expand_to_content_or_root(props: &ContainerProps, is_root: bool) -> bool {
    // A clipped, explicitly-sized frame must honour its declared height even at
    // the root: Pencil clips a fixed-height screen whose content overflows
    // rather than growing the frame to fit it. Without this guard the
    // `|| is_root` override grew a `height: 900, clip: true` screen to its
    // ~950px content height (off-by-50 vs Pencil's clipped baseline). The
    // non-root path already refuses via `height_can_expand_to_content`; mirror
    // it for the root instead of blanket-allowing expansion.
    if matches!(props.height.as_ref(), Some(SizingBehavior::Number(_)))
        && props.clip_content == Some(true)
    {
        return height_can_expand_to_content(props);
    }
    height_can_expand_to_content(props) || is_root
}

fn width_can_expand_to_content(props: &ContainerProps) -> bool {
    match props.width.as_ref() {
        None | Some(SizingBehavior::Keyword(SizingKeyword::FitContent)) => true,
        Some(SizingBehavior::Number(_)) => {
            props.clip_content != Some(true) && props.fill.is_none() && props.stroke.is_none()
        }
        _ => false,
    }
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

fn layout_child(node: &PenNode) -> bool {
    !matches!(base(node).and_then(|b| b.role.as_deref()), Some("overlay"))
}

fn is_frame(node: &PenNode) -> bool {
    matches!(node, PenNode::Frame(_))
}

fn is_unbounded_group(node: &PenNode, props: &ContainerProps) -> bool {
    matches!(node, PenNode::Group(_)) && props.width.is_none() && props.height.is_none()
}

fn child_has_fill_container_axis(node: &PenNode) -> bool {
    let (width, height) = node_sizing(node);
    matches!(
        width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    ) || matches!(
        height,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    )
}

fn authored_numeric_size(node: &PenNode) -> (Option<f32>, Option<f32>) {
    let numeric = |sizing: Option<&SizingBehavior>| match sizing {
        Some(SizingBehavior::Number(v)) => Some(*v as f32),
        _ => None,
    };
    let (width, height) = node_sizing(node);
    (numeric(width), numeric(height))
}

fn node_sizing(node: &PenNode) -> (Option<&SizingBehavior>, Option<&SizingBehavior>) {
    match node {
        PenNode::Frame(n) => (n.container.width.as_ref(), n.container.height.as_ref()),
        PenNode::Group(n) => (n.container.width.as_ref(), n.container.height.as_ref()),
        PenNode::Rectangle(n) => (n.container.width.as_ref(), n.container.height.as_ref()),
        PenNode::Ellipse(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::Text(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::TextInput(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::TextArea(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::Select(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::Switch(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::Checkbox(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::Slider(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::Image(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::IconFont(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::Polygon(n) => (n.width.as_ref(), n.height.as_ref()),
        PenNode::Path(n) => (n.width.as_ref(), n.height.as_ref()),
        _ => (None, None),
    }
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
