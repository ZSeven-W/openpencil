//! Incremental paint-scene patches for live canvas geometry gestures.
//!
//! Drag history advances the document revision at press time. Later cursor
//! frames mutate canonical geometry without another revision bump, so the
//! revision-keyed scene cache cannot rebuild those frames. Simple leaf edits
//! are patched in place; layout-dependent edits fall back to an invalidated
//! full scene rebuild.

use super::WidgetHostNative;
use op_editor_core::{NodeId, PenNodeExt};
use op_editor_ui::layout_scene::SceneNode;
use op_editor_ui::Rect;

impl WidgetHostNative {
    /// Prepare the cheap bounds-patch path before the canonical write.
    pub(in crate::widget_host) fn prepare_live_bounds_update(&mut self) -> bool {
        let id = self.editor_state.selection.anchor.clone();
        let patchable = self.bounds_patch_is_safe(&id);
        if patchable {
            self.refresh_layout_scene();
        }
        patchable
    }

    /// Synchronize one live bounds write with the paint scene. The caller must
    /// call [`Self::prepare_live_bounds_update`] before mutating the document.
    pub(in crate::widget_host) fn finish_live_bounds_update(
        &mut self,
        bounds: Rect,
        patchable: bool,
    ) {
        let id = self.editor_state.selection.anchor.clone();
        if patchable && patch_scene_bounds(&mut self.layout_scene, &id, bounds) {
            self.finish_incremental_scene_patch();
        } else {
            self.invalidate_live_scene_for_rebuild();
        }
    }

    /// Rotation does not participate in layout, so every resolved scene-node
    /// kind can take the cheap path.
    pub(in crate::widget_host) fn finish_live_rotation_update(&mut self, rotation: f32) {
        let id = self.editor_state.selection.anchor.clone();
        if patch_scene_rotation(&mut self.layout_scene, &id, rotation) {
            self.finish_incremental_scene_patch();
        } else {
            self.invalidate_live_scene_for_rebuild();
        }
    }

    /// Reconcile an incrementally patched gesture against the canonical tree.
    pub(in crate::widget_host) fn invalidate_live_scene_for_rebuild(&mut self) {
        self.scene_cache.invalidate();
        self.mark_dirty();
    }

    fn finish_incremental_scene_patch(&mut self) {
        self.scene_cache.invalidate();
        self.editor_state_dirty = false;
        self.layout_transition = None;
        self.drop_pan_cache();
    }

    fn bounds_patch_is_safe(&self, id: &NodeId) -> bool {
        let Some((parent, _)) =
            op_editor_core::walkers::find_parent_and_index(self.editor_state.active_children(), id)
        else {
            return false;
        };
        if parent.is_some() {
            return false;
        }
        let Some(node) =
            op_editor_core::walkers::find_node(self.editor_state.active_children(), id)
        else {
            return false;
        };
        node.children().is_none_or(|children| children.is_empty())
            && matches!(
                node,
                jian_ops_schema::node::PenNode::Rectangle(_)
                    | jian_ops_schema::node::PenNode::Ellipse(_)
                    | jian_ops_schema::node::PenNode::Polygon(_)
                    | jian_ops_schema::node::PenNode::Line(_)
                    | jian_ops_schema::node::PenNode::Frame(_)
            )
    }
}

fn patch_scene_bounds(
    scene: &mut op_editor_ui::layout_scene::LayoutScene,
    id: &NodeId,
    bounds: Rect,
) -> bool {
    let Some(page) = scene.pages.get_mut(scene.active_page_index) else {
        return false;
    };
    patch_node_bounds(&mut page.children, id.as_str(), bounds)
}

fn patch_node_bounds(nodes: &mut [SceneNode], id: &str, bounds: Rect) -> bool {
    for node in nodes {
        if node.id == id {
            node.bounds = bounds;
            node.aggregate_bounds_cache =
                SceneNode::compute_aggregate_bounds(bounds, &node.children);
            return true;
        }
        if patch_node_bounds(&mut node.children, id, bounds) {
            node.aggregate_bounds_cache =
                SceneNode::compute_aggregate_bounds(node.bounds, &node.children);
            return true;
        }
    }
    false
}

fn patch_scene_rotation(
    scene: &mut op_editor_ui::layout_scene::LayoutScene,
    id: &NodeId,
    rotation: f32,
) -> bool {
    let Some(page) = scene.pages.get_mut(scene.active_page_index) else {
        return false;
    };
    patch_node_rotation(&mut page.children, id.as_str(), rotation)
}

fn patch_node_rotation(nodes: &mut [SceneNode], id: &str, rotation: f32) -> bool {
    for node in nodes {
        if node.id == id {
            node.rotation = rotation;
            return true;
        }
        if patch_node_rotation(&mut node.children, id, rotation) {
            return true;
        }
    }
    false
}
