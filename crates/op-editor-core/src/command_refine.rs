//! Deterministic cleanup used by `EditorCommand::RefineDesign`.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers;
use jian_ops_schema::node::PenNode;

/// One fix applied during `RefineDesign`, mirroring TS design-refine's
/// `RefineFix` (`{ nodeId, nodeName?, fix }`). Surfaced by the `design_refine`
/// MCP tool as its `fixes[]` result so the Rust + TS tool results share shape.
#[derive(Debug, Clone, PartialEq)]
pub struct RefineFix {
    pub node_id: String,
    pub node_name: Option<String>,
    pub fix: String,
}

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
        Some(!refine_subtree(root).is_empty())
    }
}

/// Apply the deterministic refine transforms to `root` in place and return the
/// fix report (one entry per change). SHARED by `cmd_refine_design` (the host
/// apply path) and the `design_refine` MCP tool (which simulates on a clone to
/// report `fixes[]`), so the reported fixes always match what apply does.
pub fn refine_subtree(root: &mut PenNode) -> Vec<RefineFix> {
    let mut fixes = Vec::new();
    sanitize_auto_layout_child_positions(root, &mut fixes);
    adjust_root_height_to_content(root, &mut fixes);
    fixes
}

fn sanitize_auto_layout_child_positions(node: &mut PenNode, fixes: &mut Vec<RefineFix>) {
    let parent_auto_layout = node.is_auto_layout_container();
    if let Some(children) = node.children_mut() {
        for child in children {
            if parent_auto_layout {
                let id = child.base().id.clone();
                let name = child.base().name.clone();
                let base = child.base_mut();
                let removed_x = base.x.take().is_some();
                let removed_y = base.y.take().is_some();
                if removed_x || removed_y {
                    fixes.push(RefineFix {
                        node_id: id,
                        node_name: name,
                        fix: "Removed absolute position from auto-layout child".into(),
                    });
                }
            }
            sanitize_auto_layout_child_positions(child, fixes);
        }
    }
}

fn adjust_root_height_to_content(root: &mut PenNode, fixes: &mut Vec<RefineFix>) {
    let total: f64 = match root.children() {
        Some(children) if !children.is_empty() => {
            children.iter().filter_map(PenNodeExt::height_px).sum()
        }
        _ => return,
    };
    if total <= 0.0 {
        return;
    }
    if root.height_px().is_some_and(|current| current >= total) {
        return;
    }
    let id = root.base().id.clone();
    let name = root.base().name.clone();
    root.set_height_px(total);
    fixes.push(RefineFix {
        node_id: id,
        node_name: name,
        fix: "Adjusted root height to fit content".into(),
    });
}
