//! A stray image node moves into the empty media slot that was waiting for it.
//!
//! Measured (test0711-1-glm, 2026-07-12): every deal card came out as
//!
//! ```text
//! Deal Card (vertical)
//! ├── image band (160, layout none, clip)
//! │   ├── DealImg   (fill x 160, EMPTY)      <- the slot, never filled
//! │   ├── "-35%" badge (absolute)
//! │   └── heart     (absolute)
//! ├── content (title / rating / price)
//! └── image "maldives overwater villa ocean" (fill x 300)   <- the photo,
//!                                                              parented to the
//!                                                              CARD, not the slot
//! ```
//!
//! so each card painted a white band with a lone badge on it and hung the photo
//! BELOW the price row. The model got the tree wrong, not the content: the slot
//! and the photo are both there, one level apart. The repair reunites them —
//! the photo moves into the empty slot and fills it; the layout it was authored
//! for then works as designed.
//!
//! Deliberately narrow (this is a contract, not a guess):
//! - The image must be a DIRECT child of the container that holds the slot.
//! - The slot must be an EMPTY frame whose own container is an image band —
//!   i.e. a frame sized like a media box, not a text row.
//! - Exactly one candidate slot, so there is never a choice to make.

use jian_ops_schema::node::PenNode;
use op_editor_core::{EditorCommand, NodeId, PenNodeExt};

use crate::types::DocSink;

/// A media slot is at least this tall — a 20px empty spacer is not a photo box.
const MEDIA_SLOT_MIN_H: f64 = 48.0;

pub(crate) fn adopt_stray_images_for_all_roots(sink: &mut dyn DocSink) {
    let moves: Vec<(NodeId, NodeId)> = {
        let mut out = Vec::new();
        for root in sink.state().active_children() {
            collect(root, &mut out);
        }
        out
    };
    for (image_id, slot_id) in moves {
        sink.apply(EditorCommand::MoveNode {
            node_id: image_id.clone(),
            target_parent: slot_id,
            page_id: None,
            index: None,
        });
        sink.apply(EditorCommand::PatchNodeData {
            node_id: image_id,
            patch_json: r#"{"width":"fill_container","height":"fill_container","x":null,"y":null}"#
                .to_string(),
            page_id: None,
        });
    }
}

fn collect(node: &PenNode, out: &mut Vec<(NodeId, NodeId)>) {
    if let Some(children) = node.children() {
        // A stray photo: an image node parented straight into a card, beside
        // (not inside) the band that holds its slot.
        let strays: Vec<&PenNode> = children
            .iter()
            .filter(|c| matches!(c, PenNode::Image(_)) && has_source(c))
            .collect();
        if strays.len() == 1 {
            let mut slots = Vec::new();
            for child in children {
                collect_empty_media_slots(child, &mut slots, 0);
            }
            if let [slot] = slots.as_slice() {
                out.push((
                    NodeId::new(strays[0].id_str().to_string()),
                    NodeId::new(slot.to_string()),
                ));
            }
        }
        for child in children {
            collect(child, out);
        }
    }
}

fn has_source(node: &PenNode) -> bool {
    match node {
        PenNode::Image(image) => !image.src.is_empty(),
        _ => false,
    }
}

/// Empty frames, at most two levels down, that are shaped like a media box.
fn collect_empty_media_slots<'a>(node: &'a PenNode, out: &mut Vec<&'a str>, depth: usize) {
    if depth > 2 {
        return;
    }
    if let PenNode::Frame(_) = node {
        let empty = node.children().is_none_or(|c| c.is_empty());
        if empty
            && node
                .height_px()
                .is_some_and(|h| h >= MEDIA_SLOT_MIN_H)
        {
            out.push(node.id_str());
            return;
        }
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_empty_media_slots(child, out, depth + 1);
        }
    }
}

#[cfg(test)]
#[path = "stray_image_adopt_tests.rs"]
mod tests;
