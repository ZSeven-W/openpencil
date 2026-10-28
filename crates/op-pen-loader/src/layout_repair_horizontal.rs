//! Explicit horizontal justification after child sizes have been repaired.

use super::*;
use op_editor_core::PenNodeExt;

/// Normalize only the layout clone, leaving authored coordinates and explicit
/// absolute-positioning contracts intact in the canonical document. Fill-width
/// rows need their flow children in Taffy before it measures wrapping content;
/// moving rectangles afterward cannot repair an incorrectly allocated width.
pub(crate) fn normalize_fill_row_coordinates(node: &mut PenNode) {
    let normalize = container_props(node).is_some_and(|props| {
        props.layout == Some(LayoutMode::Horizontal)
            && !start_justified(props)
            && children(node).is_some_and(|kids| {
                kids.iter().any(|child| {
                    layout_child(child)
                        && matches!(
                            node_sizing(child).0,
                            Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
                        )
                })
            })
    });
    if let Some(kids) = node.children_mut() {
        for child in kids {
            if normalize && layout_child(child) {
                let base = child.base_mut();
                base.x = None;
                base.y = None;
            }
            normalize_fill_row_coordinates(child);
        }
    }
}

/// AI-authored flow children often retain x/y coordinates. Taffy treats
/// those as absolute, so it cannot distribute the row's remaining space.
/// Reposition only the same in-flow children used by the other repair passes;
/// authored overlays and constraint-positioned nodes keep their geometry.
pub(super) fn repair_justified_horizontal_container(
    node: &PenNode,
    props: &ContainerProps,
    kids: &[PenNode],
    rects: &mut BTreeMap<String, [f32; 4]>,
) {
    let Some(mut parent) = rect(node, rects) else {
        return;
    };
    let layout_kids: Vec<_> = kids.iter().filter(|child| layout_child(child)).collect();
    if layout_kids.is_empty() {
        return;
    }
    let child_rects: Vec<_> = layout_kids
        .iter()
        .filter_map(|child| rect(child, rects))
        .collect();
    if child_rects.len() != layout_kids.len() {
        return;
    }

    let padding = padding_sides(props.padding.as_ref());
    let gap = gap_value(props);
    // A hug-height row whose children were absolute can initially measure
    // zero. Resolve its content height before centering/end-aligning children.
    let content_height =
        child_rects.iter().map(|child| child.h).fold(0.0, f32::max) + padding.top + padding.bottom;
    if height_can_expand_to_content(props) && content_height > parent.h + 0.5 {
        set_height(node, rects, content_height);
        parent.h = content_height;
    }
    let available_width = (parent.w - padding.left - padding.right).max(0.0);
    let available_height = (parent.h - padding.top - padding.bottom).max(0.0);
    let child_count = layout_kids.len();
    let total_width: f32 = child_rects.iter().map(|child| child.w).sum();
    // Distribution adds to the authored gap. Overflow falls back to that gap
    // rather than creating negative spacing and overlapping adjacent children.
    let free = (available_width - total_width - gap * (child_count - 1) as f32).max(0.0);
    let (leading, distributed_gap) = match props.justify_content {
        Some(JustifyContent::Center) => (free / 2.0, 0.0),
        Some(JustifyContent::End) => (free, 0.0),
        Some(JustifyContent::SpaceBetween) if child_count > 1 => {
            (0.0, free / (child_count - 1) as f32)
        }
        Some(JustifyContent::SpaceAround) => {
            let around = free / child_count as f32;
            (around / 2.0, around)
        }
        _ => (0.0, 0.0),
    };
    let mut cursor = parent.x + padding.left + leading;
    for (child, child_rect) in layout_kids.into_iter().zip(child_rects) {
        let cross = match props.align_items {
            Some(AlignItems::Center) => ((available_height - child_rect.h) / 2.0).max(0.0),
            Some(AlignItems::End) => (available_height - child_rect.h).max(0.0),
            _ => 0.0,
        };
        let dx = cursor - child_rect.x;
        let dy = parent.y + padding.top + cross - child_rect.y;
        if dx.abs() > 0.5 || dy.abs() > 0.5 {
            shift_subtree(child, rects, dx, dy);
        }
        cursor += child_rect.w + gap + distributed_gap;
    }
    repair_container_to_child_bounds(node, props, kids, rects);
}
