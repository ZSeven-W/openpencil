//! Clipping-row stroke padding repair — pad a `clipContent` horizontal row so
//! a stroked child's outline isn't cropped.

use super::*;

pub(super) fn pad_clipping_horizontal_row_for_stroke(sink: &mut dyn DocSink, root_id: &str) {
    let repairs: Vec<ClipRowStrokePaddingRepair> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let mut repairs = Vec::new();
        collect_clip_row_stroke_padding_repairs(root, &mut repairs);
        repairs
    };

    for repair in repairs {
        sink.apply(EditorCommand::SetNodeLayoutProp {
            node_id: repair.node_id,
            property: "padding".to_string(),
            value: LayoutPropValue::NumberArray(vec![
                repair.padding[0],
                repair.padding[1],
                repair.padding[2],
                repair.padding[3],
            ]),
        });
    }
}

#[derive(Debug, Clone)]
pub(super) struct ClipRowStrokePaddingRepair {
    node_id: NodeId,
    padding: [f64; 4],
}

pub(super) fn collect_clip_row_stroke_padding_repairs(
    node: &PenNode,
    repairs: &mut Vec<ClipRowStrokePaddingRepair>,
) {
    if let Some(repair) = clip_row_stroke_padding_repair(node) {
        repairs.push(repair);
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_clip_row_stroke_padding_repairs(child, repairs);
        }
    }
}

pub(super) fn clip_row_stroke_padding_repair(node: &PenNode) -> Option<ClipRowStrokePaddingRepair> {
    let floors = clip_row_stroke_padding_floors(node)?;
    let props = frame_container_props(node)?;
    let mut padding = props
        .padding
        .as_ref()
        .map(padding_sides)
        .unwrap_or([0.0, 0.0, 0.0, 0.0]);
    let before = padding;
    for (side, floor) in padding.iter_mut().zip(floors) {
        if let Some(floor) = floor {
            *side = side.max(floor);
        }
    }
    // In particular, a fill-width row deliberately leaves the trailing edge
    // unprotected. Compare the actual target instead of requiring all four
    // sides to meet the stroke width; otherwise a zero trailing inset emits
    // the same SetNodeLayoutProp command on every finalize.
    if padding == before {
        return None;
    }
    Some(ClipRowStrokePaddingRepair {
        node_id: NodeId::new(node.id_str().to_string()),
        padding,
    })
}

/// Minimum padding that must survive later cleanup passes for a clipped
/// horizontal row with stroked children. `None` means the edge is deliberately
/// unconstrained: a fill-width rail may keep its trailing edge flush so the
/// next item remains visibly cropped.
pub(super) fn clip_row_stroke_padding_floors(node: &PenNode) -> Option<[Option<f64>; 4]> {
    let props = frame_container_props(node)?;
    if props.layout.as_ref() != Some(&LayoutMode::Horizontal) || props.clip_content != Some(true) {
        return None;
    }
    let stroke_padding = node
        .children()?
        .iter()
        .filter_map(node_stroke_width)
        .max_by(f64::total_cmp)?
        .ceil();
    // A fit-content row hugs its children and therefore needs room on every
    // edge. A fill-width rail preserves the intentional trailing crop.
    let trailing = matches!(
        props.width.as_ref(),
        Some(SizingBehavior::Keyword(SizingKeyword::FitContent))
    )
    .then_some(stroke_padding);
    Some([
        Some(stroke_padding),
        trailing,
        Some(stroke_padding),
        Some(stroke_padding),
    ])
}
