//! Avatar / row-thumbnail slot contract repair.
//!
//! An explicitly named/role-tagged avatar is a fixed SQUARE circle:
//! `width == height`, `cornerRadius` ≥ half the size, `clipContent: true`,
//! with its image child filling both axes. Models routinely violate that
//! contract (measured:
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

fn is_explicit_avatar_slot(node: &PenNode) -> bool {
    node.base()
        .role
        .as_deref()
        .is_some_and(|role| matches!(role.to_ascii_lowercase().as_str(), "avatar" | "user-avatar"))
        || node.base().name.as_deref().is_some_and(|name| {
            name.chars()
                .filter(|character| character.is_ascii_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
                .contains("avatar")
        })
}

fn is_explicit_row_media_slot(node: &PenNode) -> bool {
    if node.base().role.as_deref() == Some("image-placeholder") {
        return true;
    }
    let Some(name) = node.base().name.as_deref() else {
        return false;
    };
    let compact = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    [
        "art",
        "artwork",
        "cover",
        "thumb",
        "thumbnail",
        "avatar",
        "img",
        "image",
        "photo",
        "media",
    ]
    .iter()
    .any(|suffix| compact.ends_with(suffix))
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
/// mis-shaped explicitly named avatar slot or, inside a horizontal row, an
/// explicitly named media thumbnail whose `fill_container` width steals the
/// row.
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
    let radius = match frame.container.corner_radius.as_ref() {
        Some(CornerRadius::Uniform(radius)) => *radius,
        Some(CornerRadius::PerCorner(corners)) => corners.iter().copied().fold(f64::MAX, f64::min),
        None => 0.0,
    };
    // Explicit-avatar branch: the slot ITSELF may carry no numeric
    // height at all (measured: an "AvatarImg" slot authored fill×fill
    // holding a fill×300 headshot resolved as a 42×300 strip down the
    // screen, test0711-22 00:25). The slot's authored Avatar name/role is the
    // contract signal; image words such as "portrait" are not sufficient
    // because a hero image can also be a portrait.
    if is_explicit_avatar_slot(node) {
        let side = match node.height_px() {
            Some(height) if height > 0.0 && height <= MAX_AVATAR_SIDE => height,
            Some(_) => return None,
            None => 44.0,
        };
        let clips = frame.container.clip_content == Some(true);
        let image_fills = image_fills_both_axes(only_child);
        let slot_not_square = node.width_px() != Some(side) || node.height_px() != Some(side);
        let slot_not_round = radius + f64::EPSILON < side / 2.0;
        if !clips || !image_fills || slot_not_square || slot_not_round {
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
    let width = node.width_px();
    // Row-thumb branch: only the unbounded flex-steal shape (width
    // fill_container) qualifies — a deliberately wide numeric thumb is a
    // design decision and stays untouched. A media name/role is required;
    // clipping geometry alone does not prove that a small surface is a thumb.
    let row_thumb = in_horizontal_row
        && sizing_is_fill_container(&frame.container.width)
        && is_explicit_row_media_slot(node)
        && (radius > 0.0 || frame.container.clip_content == Some(true));
    if !row_thumb {
        return None;
    }
    let square = width == Some(height);
    let clips = frame.container.clip_content == Some(true);
    let image_fills = image_fills_both_axes(only_child);
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

fn image_fills_both_axes(node: &PenNode) -> bool {
    matches!(node, PenNode::Image(image)
        if sizing_is_fill_container(&image.width) && sizing_is_fill_container(&image.height))
}

fn sizing_is_fill_container(width: &Option<jian_ops_schema::sizing::SizingBehavior>) -> bool {
    use jian_ops_schema::sizing::{SizingBehavior, SizingKeyword};
    matches!(
        width,
        Some(SizingBehavior::Keyword(SizingKeyword::FillContainer))
    )
}
