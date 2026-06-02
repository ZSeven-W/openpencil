//! Deterministic cleanup used by `EditorCommand::RefineDesign`.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::node::PenNode;

impl EditorState {
    /// Refine a generated design root. Returns `None` when the root is
    /// invalid/missing and `Some(changed)` when the command is accepted.
    pub(crate) fn cmd_refine_design(
        &mut self,
        root_id: &NodeId,
        _canvas_width: Option<i32>,
    ) -> Option<bool> {
        if !root_id.is_real() {
            return None;
        }
        let root = walkers::find_node_mut(self.active_children_mut(), root_id)?;
        let mut changed = sanitize_auto_layout_child_positions(root);
        changed |= adjust_root_height_to_content(root);
        Some(changed)
    }
}

fn sanitize_auto_layout_child_positions(node: &mut PenNode) -> bool {
    let parent_auto_layout = node.is_auto_layout_container();
    let mut changed = false;
    if let Some(children) = node.children_mut() {
        for child in children {
            if parent_auto_layout {
                let base = child.base_mut();
                if base.x.take().is_some() {
                    changed = true;
                }
                if base.y.take().is_some() {
                    changed = true;
                }
            }
            changed |= sanitize_auto_layout_child_positions(child);
        }
    }
    changed
}

fn adjust_root_height_to_content(root: &mut PenNode) -> bool {
    let Some(children) = root.children() else {
        return false;
    };
    if children.is_empty() {
        return false;
    }
    let total: f64 = children.iter().filter_map(PenNodeExt::height_px).sum();
    if total <= 0.0 {
        return false;
    }
    if root.height_px().is_some_and(|current| current >= total) {
        return false;
    }
    root.set_height_px(total);
    true
}
