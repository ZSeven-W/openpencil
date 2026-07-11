//! Avatar / row-thumbnail slot contract repair.
//!
//! An avatar is a fixed SQUARE circle: `width == height`, `cornerRadius` ≥
//! half the size, `clipContent: true`, with its image child filling both
//! axes. Models routinely violate the shape half of that contract (measured:
//! GLM-5.2 test0711-2.op built an 88×44 pill holding a 44×44 image — on
//! canvas it reads as an empty grey circle NEXT TO a square photo). The
//! shape is a CONTRACT (a round avatar slot is round), so it is repaired
//! deterministically; what picture belongs inside stays the model's call.
//!
//! The same class covers ROW THUMBNAILS: a small media slot in a horizontal
//! row (mini-player art, track/list covers) given `width: fill_container`
//! steals the whole row's flex and stretches the artwork into a banner
//! (measured: a 44px MiniPlayer "MPArt" slot spanned half the player,
//! test0711-22). Row thumbs are squared to their height; only the
//! unbounded `fill_container` case is touched — a deliberately wide
//! NUMERIC thumb stays the designer's call.

use crate::types::DocSink;
use jian_ops_schema::node::container::CornerRadius;
use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

/// A pill-radius frame taller than this is a hero/banner, not an avatar.
const MAX_AVATAR_SIDE: f64 = 72.0;

/// A media slot holding `[empty stub frame, image]` as SIBLINGS — the model
/// laid a placeholder frame first, then put the real image NEXT TO it
/// instead of inside, so the photo renders beside an empty box (measured:
/// "Midnight Drive" cover, test0711-22 00:5x). The stub (FIRST child, no
/// children of its own) is dropped and the image takes the slot. A scrim
/// overlay is the reverse order ([image, frame]) and is never touched.
pub(crate) fn remove_empty_twin_stubs_beside_images_for_all_roots(sink: &mut dyn DocSink) {
    let repairs: Vec<(NodeId, NodeId)> = {
        let mut out = Vec::new();
        fn walk(node: &PenNode, out: &mut Vec<(NodeId, NodeId)>) {
            if let Some(children) = node.children() {
                if let [stub, image] = children.as_slice() {
                    let stub_is_empty_frame = matches!(stub, PenNode::Frame(_))
                        && stub.children().is_none_or(|c| c.is_empty());
                    if stub_is_empty_frame && matches!(image, PenNode::Image(_)) {
                        out.push((
                            NodeId::new(stub.id_str().to_string()),
                            NodeId::new(image.id_str().to_string()),
                        ));
                    }
                }
                for child in children {
                    walk(child, out);
                }
            }
        }
        for root in sink.state().active_children() {
            walk(root, &mut out);
        }
        out
    };
    for (stub_id, image_id) in repairs {
        sink.apply(EditorCommand::DeleteNode {
            node_id: stub_id,
            page_id: None,
        });
        sink.apply(EditorCommand::PatchNodeData {
            node_id: image_id,
            patch_json: r#"{"width":"fill_container","height":"fill_container"}"#.to_string(),
            page_id: None,
        });
    }
}

pub(crate) fn repair_avatar_slots_for_all_roots(sink: &mut dyn DocSink) {
    let repairs: Vec<(NodeId, String)> = {
        let mut out = Vec::new();
        for root in sink.state().active_children() {
            collect(root, &mut out);
        }
        out
    };
    for (node_id, patch_json) in repairs {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: None,
        });
    }
}

/// Avatar-query vocabulary — WE teach the model to bind these words into
/// avatar image search queries, so an image named with them is an avatar by
/// our own contract.
const AVATAR_NAME_WORDS: [&str; 4] = ["face", "headshot", "avatar", "portrait"];

fn is_avatar_named_image(node: &PenNode) -> bool {
    let PenNode::Image(image) = node else {
        return false;
    };
    image.base.name.as_deref().is_some_and(|name| {
        let lowered = name.to_ascii_lowercase();
        AVATAR_NAME_WORDS.iter().any(|word| lowered.contains(word))
    })
}

fn collect(node: &PenNode, out: &mut Vec<(NodeId, String)>) {
    let in_horizontal_row =
        node_layout_is_horizontal(node) && node.children().is_some_and(|c| c.len() >= 2);
    for child in node.children().into_iter().flatten() {
        if let Some((slot_patch, image_patch)) = avatar_slot_repair(child, in_horizontal_row) {
            out.push((NodeId::new(child.id_str().to_string()), slot_patch));
            if let Some((image_id, patch)) = image_patch {
                out.push((NodeId::new(image_id), patch));
            }
        }
        collect(child, out);
    }
}

/// Returns `(slot patch, optional image-child patch)` when `node` is a
/// mis-shaped avatar slot (small pill-radius frame whose only child is an
/// image but isn't a clipping square) or, inside a horizontal row, a row
/// thumbnail whose `fill_container` width steals the row.
fn avatar_slot_repair(
    node: &PenNode,
    in_horizontal_row: bool,
) -> Option<(String, Option<(String, String)>)> {
    let PenNode::Frame(frame) = node else {
        return None;
    };
    let children = node.children()?;
    let [only_child] = children.as_slice() else {
        return None;
    };
    if !matches!(only_child, PenNode::Image(_)) {
        return None;
    }
    // Avatar-named image branch: the slot ITSELF may carry no numeric
    // height at all (measured: an "AvatarImg" slot authored fill×fill
    // holding a fill×300 headshot resolved as a 42×300 strip down the
    // screen, test0711-22 00:25) — the image's avatar-query NAME is the
    // contract signal, and the whole slot is normalized to a 44px circle.
    if is_avatar_named_image(only_child) {
        let side = node
            .height_px()
            .filter(|h| *h > 0.0 && *h <= MAX_AVATAR_SIDE)
            .unwrap_or(44.0);
        let oversized_image = only_child.width_px().is_some_and(|w| w > MAX_AVATAR_SIDE)
            || only_child.height_px().is_some_and(|h| h > MAX_AVATAR_SIDE);
        let slot_not_square = node.width_px() != Some(side) || node.height_px() != Some(side);
        if oversized_image || slot_not_square {
            let slot_patch = format!(
                r#"{{"width":{side},"height":{side},"clipContent":true,"cornerRadius":{radius}}}"#,
                side = side.round(),
                radius = (side / 2.0).round()
            );
            let image_patch = Some((
                only_child.id_str().to_string(),
                r#"{"width":"fill_container","height":"fill_container"}"#.to_string(),
            ));
            return Some((slot_patch, image_patch));
        }
    }
    let height = node.height_px()?;
    if height <= 0.0 || height > MAX_AVATAR_SIDE {
        return None;
    }
    let radius = match frame.container.corner_radius.as_ref() {
        Some(CornerRadius::Uniform(r)) => *r,
        Some(CornerRadius::PerCorner(corners)) => corners.iter().copied().fold(f64::MAX, f64::min),
        None => 0.0,
    };
    let width = node.width_px();
    let pill = radius >= height / 2.0 - 1.0;
    // Row-thumb branch: only the unbounded flex-steal shape (width
    // fill_container) qualifies — a deliberately wide numeric thumb is a
    // design decision and stays untouched.
    let row_thumb = in_horizontal_row
        && width_is_fill_container(&frame.container.width)
        && (radius > 0.0 || frame.container.clip_content == Some(true));
    if !pill && !row_thumb {
        return None;
    }
    let square = width == Some(height);
    let clips = frame.container.clip_content == Some(true);
    let image_fills = only_child.width_px().is_none() && only_child.height_px().is_none();
    if square && clips && image_fills {
        return None;
    }
    let slot_patch = format!(
        r#"{{"width":{h},"height":{h},"clipContent":true}}"#,
        h = height.round()
    );
    let image_patch = (!image_fills).then(|| {
        (
            only_child.id_str().to_string(),
            r#"{"width":"fill_container","height":"fill_container"}"#.to_string(),
        )
    });
    Some((slot_patch, image_patch))
}

fn node_layout_is_horizontal(node: &PenNode) -> bool {
    use jian_ops_schema::node::container::LayoutMode;
    let layout = match node {
        PenNode::Frame(n) => n.container.layout.as_ref(),
        PenNode::Group(n) => n.container.layout.as_ref(),
        PenNode::Rectangle(n) => n.container.layout.as_ref(),
        _ => None,
    };
    matches!(layout, Some(LayoutMode::Horizontal))
}

fn width_is_fill_container(width: &Option<jian_ops_schema::sizing::SizingBehavior>) -> bool {
    use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
    matches!(
        width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    )
}
