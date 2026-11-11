//! The timeline's rows, in the scope's tree order: each animating node's
//! lanes indented by depth, an expanded lane's property sub-tracks under
//! it, and the selected node even before it animates, so a first keyframe
//! has somewhere to go.

use crate::widgets::motion_timeline::{TimelineLane, TimelineRow};
use jian_ops_schema::node::PenNode;
use op_editor_core::motion_timeline_projection::TimelineStrip;
use op_editor_core::motion_timeline_scope::motion_node_label;
use op_editor_core::pen_node_ext::PenNodeExt;
use op_editor_core::EditorState;

/// Rows for `root`'s subtree. `strips` are the tab's (drag-previewed)
/// strips; a node's lanes keep their projection order.
pub(crate) fn rows(
    state: &EditorState,
    root: &PenNode,
    strips: &[TimelineStrip],
) -> Vec<TimelineRow> {
    let selected = state.selection.anchor.as_str();
    let timeline = &state.editor_ui.motion_timeline;
    let mut out = Vec::new();
    let mut stack = vec![(root, 0_usize)];
    while let Some((node, depth)) = stack.pop() {
        let id = node.id_str();
        let lanes: Vec<&TimelineStrip> =
            strips.iter().filter(|strip| strip.node_id == id).collect();
        let name = motion_node_label(node);
        if lanes.is_empty() && id == selected {
            out.push(TimelineRow {
                label: name.clone(),
                depth,
                lane: TimelineLane::Empty {
                    node_id: id.to_owned(),
                },
            });
        }
        for strip in &lanes {
            let properties = properties(node, strip.animation);
            let expanded = timeline.is_expanded(id, strip.animation) && !properties.is_empty();
            out.push(TimelineRow {
                label: if lanes.len() > 1 {
                    format!("{name} · {}", strip.animation + 1)
                } else {
                    name.clone()
                },
                depth,
                lane: TimelineLane::Strip {
                    strip: (*strip).clone(),
                    expanded,
                    expandable: !properties.is_empty(),
                },
            });
            if expanded {
                out.extend(
                    properties
                        .into_iter()
                        .map(|(property, keyframes)| TimelineRow {
                            label: property,
                            depth: depth + 1,
                            lane: TimelineLane::Property {
                                strip: (*strip).clone(),
                                keyframes,
                            },
                        }),
                );
            }
        }
        for child in node.children().into_iter().flatten().rev() {
            stack.push((child, depth + 1));
        }
    }
    out
}

/// The properties animation `index` keys, in first-keyed order, each with
/// the keyframes that hold it.
fn properties(node: &PenNode, index: usize) -> Vec<(String, Vec<usize>)> {
    let Some(animation) = node
        .motion_declarations()
        .1
        .and_then(|animations| animations.get(index))
    else {
        return Vec::new();
    };
    let mut out: Vec<(String, Vec<usize>)> = Vec::new();
    for (keyframe, frame) in animation.keyframes.iter().enumerate() {
        for property in frame.values.keys() {
            match out.iter_mut().find(|(name, _)| name == property) {
                Some((_, keyframes)) => keyframes.push(keyframe),
                None => out.push((property.clone(), vec![keyframe])),
            }
        }
    }
    out
}
