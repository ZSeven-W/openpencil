//! Canvas click selection semantics: parent promotion + enter-group.
//!
//! Ports the TS behavior from `skia-interaction.ts:368-405` (click
//! promotion / click-child-of-selected) generalized to the audit's
//! agreed semantics: a click on a nested node promotes to its
//! outermost frame/group ancestor below the page root (unit
//! selection), UNLESS the user has "entered" that container via
//! double-click — then promotion stops at the entered container's
//! child. The entered container lives on
//! `EditorUiState::entered_container`; Escape and selecting outside
//! the container exit it.
//!
//! DELIBERATE DIVERGENCE from TS: promotion climbs transitively to
//! the highest contiguous frame/group ancestor below the page root
//! (Figma-style unit selection). TS's literal code promotes only one
//! level under a grandparent condition — the climb is the intended
//! UX per the parity audit. Shift+click toggles the RESOLVED target
//! (deselects an already-selected hit); TS's shift branch no-ops on
//! selected hits, which reads as a quirk rather than intent.

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::state::EditorState;
use crate::walkers::{descendant_contains, find_node};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::style::PenFill;

/// Outcome of resolving a canvas hit against the current selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionResolution {
    /// The hit is a child of an already-selected node — keep the
    /// selection unchanged (TS "click child of selected" rule); the
    /// press still drags the existing set.
    Keep,
    /// Select this node (the promoted ancestor, or the hit itself).
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

/// TS `hasImageVisual`: an Image node, or any node carrying an image
/// fill. Such nodes select directly — promotion would make images
/// inside frames impossible to grab.
pub fn node_has_image_visual(node: &PenNode) -> bool {
    if matches!(node, PenNode::Image(_)) {
        return true;
    }
    crate::fills::node_fills(node)
        .map(|fills| fills.iter().any(|f| matches!(f, PenFill::Image(_))))
        .unwrap_or(false)
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

/// Resolve the node a canvas press should select, given the deepest
/// hit, the current selection, and the entered container.
///
/// Rules (TS `skia-interaction.ts:368-405`, generalized):
/// 1. A hit that is already selected re-selects itself (the host
///    keeps multi-set membership on plain click).
/// 2. A hit that is a strict descendant of any selected node keeps
///    the selection unchanged — clicking a child of a selected
///    container drags the container.
/// 3. Image-visual hits select directly (TS `hasImageVisual` guard).
/// 4. Otherwise the hit promotes to its highest contiguous
///    frame/group ancestor below the page root. Climbing stops at
///    the entered container, so inside an entered container the
///    promotion lands on that container's child.
pub fn resolve_canvas_selection_target(
    children: &[PenNode],
    deepest: &NodeId,
    entered: Option<&NodeId>,
    selection: &[NodeId],
) -> SelectionResolution {
    if selection.iter().any(|s| s == deepest) {
        return SelectionResolution::Select(deepest.clone());
    }
    if selection
        .iter()
        .any(|sel| is_strict_descendant(children, sel, deepest))
    {
        return SelectionResolution::Keep;
    }
    // Clicking the entered container's own surface selects it as-is.
    if entered == Some(deepest) {
        return SelectionResolution::Select(deepest.clone());
    }
    if let Some(node) = find_node(children, deepest) {
        if node_has_image_visual(node) {
            return SelectionResolution::Select(deepest.clone());
        }
    }
    let Some(path) = ancestor_path(children, deepest) else {
        return SelectionResolution::Select(deepest.clone());
    };
    // Climb from the nearest ancestor towards the page root while the
    // ancestors stay frame/group containers; never climb up to (or
    // past) the entered container.
    let mut target = deepest.clone();
    for anc in path.iter().rev().skip(1) {
        if entered == Some(anc) {
            break;
        }
        let Some(anc_node) = find_node(children, anc) else {
            break;
        };
        if !matches!(anc_node, PenNode::Frame(_) | PenNode::Group(_)) {
            break;
        }
        target = anc.clone();
    }
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
