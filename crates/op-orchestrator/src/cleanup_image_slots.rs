//! Materialize empty image-fill slots.
//!
//! Weak models often express a picture slot as a Frame/Rectangle carrying a
//! single `PenFill::Image` whose url is still empty, instead of the
//! `PenNode::Image` node the downstream image-enrichment pipeline (and the
//! property panel) recognises. This pass converts exactly that shape —
//! childless, exactly one image fill, placeholder url — into an in-place
//! `PenNode::Image` so the slot behaves like every other image slot from then
//! on. Everything else stays untouched: a container with real children is a
//! legitimate background-image design, a landed url is already filled, and a
//! multi-fill stack is not a slot.

use super::*;

use jian_ops_schema::style::PenFill;

/// Per-root cleanup pass: replace childless Frame/Rectangle nodes whose only
/// fill is an image fill with a still-empty url by a `PenNode::Image` in
/// place, keeping the node id and the geometry fields both types share.
pub(super) fn materialize_empty_image_fill_slots(sink: &mut dyn DocSink, root_id: &str) {
    let patches: Vec<(NodeId, String)> = {
        let Some(root) = find_root(sink.state(), root_id) else {
            return;
        };
        let mut patches = Vec::new();
        collect_materialize_patches(root, &mut patches);
        patches
    };
    for (node_id, patch_json) in patches {
        sink.apply(EditorCommand::PatchNodeData {
            node_id,
            patch_json,
            page_id: None,
        });
    }
}

fn collect_materialize_patches(node: &PenNode, patches: &mut Vec<(NodeId, String)>) {
    if let Some(patch) = materialize_patch(node) {
        patches.push((NodeId::new(node.id_str().to_string()), patch));
        // A validated slot is childless by definition — nothing below to scan.
        return;
    }
    if let Some(children) = node.children() {
        for child in children {
            collect_materialize_patches(child, patches);
        }
    }
}

/// Mirror of the desktop crate's `has_empty_image_fill` slot predicate
/// (`op-host-desktop/src/image_search_session/targets.rs`), plus the
/// childless requirement this pass needs. Kept local because op-orchestrator
/// must not depend on the desktop crate; keep the two in sync if the slot
/// shape ever grows.
fn is_materializable_empty_image_fill_slot(node: &PenNode) -> bool {
    if node.children().is_some_and(|children| !children.is_empty()) {
        return false;
    }
    let container = match node {
        PenNode::Frame(frame) => &frame.container,
        PenNode::Rectangle(rect) => &rect.container,
        _ => return false,
    };
    let Some([PenFill::Image(body)]) = container.fill.as_deref() else {
        return false;
    };
    body.url.trim().is_empty() || body.url.starts_with(SVG_PLACEHOLDER_SRC_PREFIX)
}

/// The `data:image/svg+xml;charset=utf-8,%3Csvg` placeholder prefix the
/// desktop detector treats as "no image has landed yet".
const SVG_PLACEHOLDER_SRC_PREFIX: &str = "data:image/svg+xml;charset=utf-8,%3Csvg";

/// The `PatchNodeData` payload that flips one validated slot into an image
/// node in place. Built from the node's own JSON: `type` becomes `image`,
/// `src` becomes empty, and the query/prompt bindings are cleared. The keys
/// the image schema shares (`id`/`name`/`role`/`width`/`height`/
/// `cornerRadius`/`opacity`/…) survive verbatim, so the geometry the author
/// gave the slot is the geometry the image node gets; the container-only keys
/// left over in the merged node (`fill`, `children`, `layout`, …) are not
/// image-schema fields and are dropped when the payload is parsed back.
fn materialize_patch(node: &PenNode) -> Option<String> {
    if !is_materializable_empty_image_fill_slot(node) {
        return None;
    }
    let mut value = serde_json::to_value(node).ok()?;
    let object = value.as_object_mut()?;
    object.insert(
        "type".to_string(),
        serde_json::Value::String("image".to_string()),
    );
    object.insert("src".to_string(), serde_json::Value::String(String::new()));
    object.insert("imageSearchQuery".to_string(), serde_json::Value::Null);
    object.insert("imagePrompt".to_string(), serde_json::Value::Null);
    serde_json::to_string(&value).ok()
}
