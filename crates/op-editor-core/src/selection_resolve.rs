//! Canvas hover and click depth resolution.
//!
//! A scene hit supplies a path ordered from the design root to the
//! deepest painted node under the pointer. The design root is an
//! implicit scope: a normal interaction targets its direct child,
//! while an entered container becomes the scope and targets its
//! direct child. The next path element is exposed separately for
//! double-click selection and dashed secondary hover feedback.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers::{descendant_contains, find_node};
use jian_ops_schema::node::PenNode;

/// The two node depths relevant to one pointer hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanvasDepthTargets {
    /// Direct child of the active scope, or the scope itself when the
    /// path has no deeper node.
    pub primary: NodeId,
    /// Direct child of `primary` under the pointer, when present.
    pub secondary_under_pointer: Option<NodeId>,
}

/// Resolve one root-to-deepest scene hit path without skipping levels.
///
/// With no entered container, the first path item is the implicit
/// design scope. When `entered` occurs in the path, it replaces that
/// scope. An entered container outside this hit path (for example a
/// sibling) has no effect on the result.
pub fn resolve_canvas_depth_targets(
    path: &[NodeId],
    entered: Option<&NodeId>,
) -> Option<CanvasDepthTargets> {
    let last_index = path.len().checked_sub(1)?;
    let scope_index = entered
        .and_then(|entered| path.iter().position(|id| id == entered))
        .unwrap_or(0);
    let primary_index = (scope_index + 1).min(last_index);

    Some(CanvasDepthTargets {
        primary: path[primary_index].clone(),
        secondary_under_pointer: path.get(primary_index + 1).cloned(),
    })
}

/// Compatibility outcome for the older id-only selection entry point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionResolution {
    /// Legacy no-op outcome retained for downstream API compatibility.
    /// Depth-based resolution no longer emits it.
    Keep,
    /// Select the resolved primary depth.
    Select(NodeId),
}

/// True when `node` is a strict descendant of `ancestor` (not the
/// same node).
pub fn is_strict_descendant(children: &[PenNode], ancestor: &NodeId, node: &NodeId) -> bool {
    if ancestor == node {
        return false;
    }
    let Some(anc) = find_node(children, ancestor) else {
        return false;
    };
    descendant_contains(anc, node)
}

/// Ancestor chain from the top-level node down to `target`
/// (inclusive): `[top, …, parent, target]`.
fn ancestor_path(children: &[PenNode], target: &NodeId) -> Option<Vec<NodeId>> {
    for child in children {
        if child.id_str() == target.as_str() {
            return Some(vec![target.clone()]);
        }
        if let Some(grand) = child.children() {
            if let Some(mut path) = ancestor_path(grand, target) {
                if let Some(id) = NodeId::new_opt(child.id_str()) {
                    path.insert(0, id);
                }
                return Some(path);
            }
        }
    }
    None
}

/// Resolve the primary selection target for callers that only have a
/// document-tree hit id rather than a scene hit path.
///
/// This compatibility entry point reconstructs the root-to-deepest
/// path and applies [`resolve_canvas_depth_targets`]. Node type,
/// image fills, and the current selection never skip a path level.
pub fn resolve_canvas_selection_target(
    children: &[PenNode],
    deepest: &NodeId,
    entered: Option<&NodeId>,
    _selection: &[NodeId],
) -> SelectionResolution {
    let Some(path) = ancestor_path(children, deepest) else {
        return SelectionResolution::Select(deepest.clone());
    };
    let target = resolve_canvas_depth_targets(&path, entered)
        .map(|targets| targets.primary)
        .unwrap_or_else(|| deepest.clone());
    SelectionResolution::Select(target)
}

impl EditorState {
    /// Exit the entered container when the selection no longer points
    /// inside it (the container itself still counts as inside). An
    /// empty selection also exits — clicking blank canvas leaves the
    /// container; Escape's own tier bypasses this by clearing the
    /// selection directly.
    pub fn sync_entered_container_with_selection(&mut self) {
        let Some(ec) = self.editor_ui.entered_container.clone() else {
            return;
        };
        let children = self.active_children();
        let inside = self
            .selection
            .set
            .iter()
            .any(|id| *id == ec || is_strict_descendant(children, &ec, id));
        if !inside {
            self.editor_ui.entered_container = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(values: &[&str]) -> Vec<NodeId> {
        values.iter().map(|value| NodeId::new(*value)).collect()
    }

    #[test]
    fn four_level_path_exposes_only_the_next_two_depths() {
        let path = ids(&["root", "level-1", "level-2", "level-3"]);

        assert_eq!(
            resolve_canvas_depth_targets(&path, None),
            Some(CanvasDepthTargets {
                primary: NodeId::new("level-1"),
                secondary_under_pointer: Some(NodeId::new("level-2")),
            })
        );
    }

    #[test]
    fn entered_scope_advances_exactly_one_level_without_skipping() {
        let path = ids(&["root", "level-1", "level-2", "level-3"]);
        let entered = NodeId::new("level-1");

        assert_eq!(
            resolve_canvas_depth_targets(&path, Some(&entered)),
            Some(CanvasDepthTargets {
                primary: NodeId::new("level-2"),
                secondary_under_pointer: Some(NodeId::new("level-3")),
            })
        );
    }

    #[test]
    fn deepest_entered_scope_resolves_to_itself() {
        let path = ids(&["root", "level-1", "level-2"]);
        let entered = NodeId::new("level-2");

        assert_eq!(
            resolve_canvas_depth_targets(&path, Some(&entered)),
            Some(CanvasDepthTargets {
                primary: NodeId::new("level-2"),
                secondary_under_pointer: None,
            })
        );
    }

    #[test]
    fn entered_sibling_does_not_change_the_implicit_root_scope() {
        let path = ids(&["root", "level-1", "level-2"]);
        let sibling = NodeId::new("other-branch");

        assert_eq!(
            resolve_canvas_depth_targets(&path, Some(&sibling)),
            Some(CanvasDepthTargets {
                primary: NodeId::new("level-1"),
                secondary_under_pointer: Some(NodeId::new("level-2")),
            })
        );
    }

    #[test]
    fn root_surface_resolves_to_root_without_a_secondary() {
        let path = ids(&["root"]);

        assert_eq!(
            resolve_canvas_depth_targets(&path, None),
            Some(CanvasDepthTargets {
                primary: NodeId::new("root"),
                secondary_under_pointer: None,
            })
        );
    }

    #[test]
    fn empty_path_has_no_depth_targets() {
        assert_eq!(resolve_canvas_depth_targets(&[], None), None);
    }
}
