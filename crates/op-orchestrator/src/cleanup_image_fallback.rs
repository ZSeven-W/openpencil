//! Cleanup-driver adapters for the pure image and map placeholder builders.

use crate::image_fallback_policy::image_fallback_policy;
use crate::map_placeholder::map_placeholder;
use crate::repair_summary::{CheckCategory, RepairCounter, RepairSummary};
use crate::types::DocSink;
use jian_ops_schema::node::PenNode;
use op_editor_core::{first_solid_fill_hex, walkers, EditorCommand, NodeId};

/// Apply map patches before image target collection and checkpoint the group.
pub(super) fn repair_map_placeholders(
    sink: &mut dyn DocSink,
    root_id: &str,
    summary: &mut RepairSummary,
    counter: &mut RepairCounter,
) {
    let Some(root) = walkers::find_node(
        sink.state().active_children(),
        &NodeId::new(root_id.to_string()),
    ) else {
        counter.checkpoint(summary, CheckCategory::Structure, "map-placeholder");
        return;
    };
    let rects = crate::image_fallback_policy::resolved_rects(sink.state());
    let theme = crate::role_defaults::detect_theme_from_fill(first_solid_fill_hex(root));
    let patches = map_placeholder(root, &rects, theme);
    for patch in patches {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(patch.node_id),
            patch_json: patch.patch_json,
            page_id: None,
        });
    }
    counter.checkpoint(summary, CheckCategory::Structure, "map-placeholder");
}

/// Apply the policy to every active-page root and close its checkpoint.
pub(super) fn repair_image_fallback_policy(
    sink: &mut dyn DocSink,
    summary: &mut RepairSummary,
    counter: &mut RepairCounter,
) {
    let rects = crate::image_fallback_policy::resolved_rects(sink.state());
    let roots: Vec<PenNode> = sink.state().active_children().to_vec();
    let patches: Vec<_> = roots
        .iter()
        .flat_map(|root| image_fallback_policy(root, &rects, false))
        .collect();
    for patch in patches {
        sink.apply(EditorCommand::PatchNodeData {
            node_id: NodeId::new(patch.node_id),
            patch_json: patch.patch_json,
            page_id: None,
        });
    }
    counter.checkpoint(summary, CheckCategory::Structure, "image-fallback-policy");
}

#[cfg(test)]
#[path = "cleanup_image_fallback_tests.rs"]
mod tests;
