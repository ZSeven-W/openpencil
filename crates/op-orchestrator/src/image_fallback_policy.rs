//! Orchestrator adapters for the shared image fallback policy.

use std::collections::HashMap;

use jian_ops_schema::node::PenNode;
use jian_scene::layout_scene::SceneNode;
use op_editor_core::{EditorCommand, EditorState, NodeId};

pub use op_image_enrich::{
    icon_name_for_query, ImageFallbackBranch, ImageFallbackPatch, SEARCH_FAILED_PLACEHOLDER_SRC,
};

/// The resolved absolute rectangle used by geometry-aware cleanup callers.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ResolvedRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

pub fn resolved_rects(state: &EditorState) -> HashMap<String, ResolvedRect> {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(state);
    let mut rects = HashMap::new();
    if let Some(page) = scene.active_page() {
        collect_rects(&page.children, &mut rects);
    }
    rects
}

fn collect_rects(nodes: &[SceneNode], rects: &mut HashMap<String, ResolvedRect>) {
    for node in nodes {
        let bounds = node.aggregate_bounds();
        rects.insert(
            node.id.clone(),
            ResolvedRect {
                x: f64::from(bounds.origin.x),
                y: f64::from(bounds.origin.y),
                width: f64::from(bounds.size.x),
                height: f64::from(bounds.size.y),
            },
        );
        collect_rects(&node.children, rects);
    }
}

pub fn image_fallback_policy(
    root: &PenNode,
    rects: &HashMap<String, ResolvedRect>,
    after_enrich: bool,
) -> Vec<ImageFallbackPatch> {
    let widths = rects
        .iter()
        .map(|(id, rect)| (id.clone(), rect.width))
        .collect();
    op_image_enrich::image_fallback_policy_with_widths(root, &widths, after_enrich)
}

pub fn apply_image_fallback_policy_to_state(state: &mut EditorState, after_enrich: bool) -> usize {
    let original_page = state.ui.active_page_index;
    let mut applied = 0;
    for page_index in 0..state.page_count() {
        if !state.set_active_page(page_index) && state.ui.active_page_index != page_index {
            continue;
        }
        let rects = resolved_rects(state);
        let roots: Vec<PenNode> = state.active_children().to_vec();
        let patches: Vec<_> = roots
            .iter()
            .flat_map(|root| image_fallback_policy(root, &rects, after_enrich))
            .collect();
        for patch in patches {
            if state.apply(EditorCommand::PatchNodeData {
                node_id: NodeId::new(patch.node_id),
                patch_json: patch.patch_json,
                page_id: None,
            }) {
                applied += 1;
            }
        }
    }
    let _ = state.set_active_page(original_page);
    applied
}
