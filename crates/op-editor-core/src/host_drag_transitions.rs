//! Canvas press/drag state transitions shared by the native and web
//! widget hosts (their `canvas_select_drag.rs` / `node_drag.rs` twins
//! used to carry these as copy-pasted inline blocks).
//!
//! Everything here touches only `EditorState`; the hosts keep the
//! platform glue (`mark_dirty`, scene caches, layer-panel scrolling,
//! hover-probe caches, drag-state structs) in their thin wrappers.

use crate::id_allocator::{IdAllocError, IdAllocator, SequentialIdAllocator};
use crate::selection_resolve::{resolve_canvas_depth_targets, CanvasDepthTargets};
use crate::state::EditorState;
use crate::NodeId;

/// Canvas double-click window, in milliseconds.
const CANVAS_DOUBLE_CLICK_MS: u64 = 400;

/// Everything a canvas press needs to route itself, resolved once from
/// the rendered root-to-deepest hit path.
pub struct CanvasPressResolve {
    /// Depth-resolved targets for the current entered-container scope.
    pub targets: CanvasDepthTargets,
    /// Node the press actually selects — normally `targets.primary`,
    /// but the deepest node when it is already the crop selection.
    pub primary: NodeId,
    /// Second press inside the double-click window over the same
    /// deepest geometry.
    pub is_double: bool,
    /// The deepest hit is the current single crop-editable selection, so
    /// this press must preserve it (a second press then activates crop
    /// editing) instead of drilling one level.
    pub selected_crop_is_deepest: bool,
}

/// Resolve a canvas press over `hit_path` and refresh the double-click
/// tracker. `None` when the path carries no usable target.
///
/// Shift and an existing multi-selection deliberately disable
/// drill-down so a set-edit gesture cannot unexpectedly enter a
/// container.
pub fn resolve_canvas_press(
    state: &mut EditorState,
    hit_path: &[NodeId],
    now_ms: u64,
    shift_held: bool,
) -> Option<CanvasPressResolve> {
    let deepest = hit_path.last()?.clone();
    let targets =
        resolve_canvas_depth_targets(hit_path, state.editor_ui.entered_container.as_ref())?;
    let is_double = matches!(
        &state.editor_ui.last_canvas_click,
        Some((prev, t)) if *prev == deepest
            && now_ms.saturating_sub(*t) < CANVAS_DOUBLE_CLICK_MS
    ) && !shift_held
        && state.selection_count() <= 1;
    state.editor_ui.last_canvas_click = if shift_held || is_double {
        None
    } else {
        Some((deepest.clone(), now_ms))
    };
    // A leaf selected directly from the Layer panel can sit below the
    // canvas depth resolver's primary target. Preserve that exact crop
    // selection on the first press so the second press can activate crop
    // editing. A child hit does not qualify: it must retain ordinary
    // one-level drill semantics.
    let selected_crop_is_deepest = deepest == state.selection.anchor
        && state.selection_count() == 1
        && state.can_edit_selected_image_crop();
    let primary = if selected_crop_is_deepest {
        deepest
    } else {
        targets.primary.clone()
    };
    Some(CanvasPressResolve {
        targets,
        primary,
        is_double,
        selected_crop_is_deepest,
    })
}

/// Double-click drill: select the direct child under the pointer and
/// enter the primary as the sibling scope. The stationary-pointer hover
/// is rebased immediately so the outline follows without a mouse move.
pub fn enter_child_scope(state: &mut EditorState, primary: NodeId, secondary: NodeId) {
    state.set_single_selection(secondary.clone());
    state.editor_ui.entered_container = Some(primary);
    state.editor_ui.canvas_hover_node = Some(secondary);
}

/// Apply the press selection at the resolved level and exit the entered
/// container when the press landed outside it.
///
/// A plain click selects the solid-outline primary, so clicking a
/// sibling inside an entered scope moves selection at that level
/// instead of dragging an arbitrary deepest leaf. Shift toggles set
/// membership.
///
/// Returns whether this press should begin a node drag — a shift press
/// that removed the node from the set must not drag it.
pub fn apply_canvas_press_selection(
    state: &mut EditorState,
    target: NodeId,
    shift_held: bool,
    hit_path: &[NodeId],
) -> bool {
    let should_start_drag = if shift_held {
        let was_in_set = state.is_selected(&target);
        state.toggle_selection(target);
        !was_in_set
    } else {
        let already_in_set = state.is_selected(&target);
        if !already_in_set || state.selection_count() == 1 {
            state.set_single_selection(target);
        }
        true
    };
    if state
        .editor_ui
        .entered_container
        .as_ref()
        .is_some_and(|entered| !hit_path.contains(entered))
    {
        state.editor_ui.entered_container = None;
    }
    should_start_drag
}

/// Outcome of the first cursor move that promotes a press into a drag.
pub struct NodeDragActivation {
    /// Selection ids the Option/Alt gesture cloned away from — the drop
    /// policy must not treat them as containers.
    pub option_drag_source_ids: Vec<NodeId>,
    /// The clone (and any flex reorder) mutated the tree, so the caller
    /// invalidates its scene cache and repaints.
    pub duplicated: bool,
}

/// Promote a press into a drag: the gesture can no longer be the first
/// half of a double-click drill, and an Option/Alt drag clones the
/// selection in place before travelling.
pub fn activate_node_drag(
    state: &mut EditorState,
    next_node_id: &mut u64,
    alt_held: bool,
    total_dx: f64,
    total_dy: f64,
) -> NodeDragActivation {
    let Ok(mut allocator) = SequentialIdAllocator::for_document(&state.doc, *next_node_id) else {
        state.editor_ui.last_canvas_click = None;
        return NodeDragActivation {
            option_drag_source_ids: Vec::new(),
            duplicated: false,
        };
    };
    match activate_node_drag_with_allocator(state, &mut allocator, alt_held, total_dx, total_dy) {
        Ok(activation) => {
            if activation.duplicated {
                *next_node_id = allocator.next_counter();
            }
            activation
        }
        Err(_) => {
            // Preserve the legacy infallible gesture behavior.
            state.editor_ui.last_canvas_click = None;
            NodeDragActivation {
                option_drag_source_ids: Vec::new(),
                duplicated: false,
            }
        }
    }
}

/// Allocator-aware form of [`activate_node_drag`].
///
/// An allocation failure leaves the editor document, selection, and
/// double-click tracker unchanged.
pub fn activate_node_drag_with_allocator(
    state: &mut EditorState,
    allocator: &mut dyn IdAllocator,
    alt_held: bool,
    total_dx: f64,
    total_dy: f64,
) -> Result<NodeDragActivation, IdAllocError> {
    // Once the gesture becomes a drag it cannot be the first half of a
    // later double-click drill.
    let previous_canvas_click = state.editor_ui.last_canvas_click.take();
    let option_source_ids: Vec<NodeId> = state.selection.set.to_vec();
    let duplicated = if alt_held && !option_source_ids.is_empty() {
        match state.duplicate_selected_with_allocator(allocator, 0.0) {
            Ok(result) => result.is_some(),
            Err(error) => {
                state.editor_ui.last_canvas_click = previous_canvas_click;
                return Err(error);
            }
        }
    } else {
        false
    };
    if duplicated {
        let _ = state.move_selected_in_layout_direction(total_dx, total_dy);
        Ok(NodeDragActivation {
            option_drag_source_ids: option_source_ids,
            duplicated: true,
        })
    } else {
        Ok(NodeDragActivation {
            option_drag_source_ids: Vec::new(),
            duplicated: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id_allocator::{NamespacedIdAllocator, PeerNamespace};
    use crate::test_support::{rect, state_with};

    #[test]
    fn allocator_aware_alt_drag_uses_session_namespace() {
        let mut state = state_with(vec![rect("n1", "Rectangle", 1.0, 2.0, 10.0, 10.0)]);
        state.set_single_selection(NodeId::new("n1"));
        state.editor_ui.last_canvas_click = Some((NodeId::new("n1"), 10));
        let namespace = PeerNamespace::try_from("drag-peer").unwrap();
        let mut allocator = NamespacedIdAllocator::new(namespace, 0);

        let activation =
            activate_node_drag_with_allocator(&mut state, &mut allocator, true, 3.0, 4.0).unwrap();

        assert!(activation.duplicated);
        assert_eq!(activation.option_drag_source_ids, vec![NodeId::new("n1")]);
        assert_eq!(state.selection.set.len(), 1);
        assert!(state.selection.anchor.as_str().starts_with("c_drag-peer_"));
        assert!(state.editor_ui.last_canvas_click.is_none());
    }

    #[test]
    fn allocator_failure_is_editor_state_atomic() {
        let mut state = state_with(vec![rect("n1", "Rectangle", 1.0, 2.0, 10.0, 10.0)]);
        state.set_single_selection(NodeId::new("n1"));
        state.editor_ui.last_canvas_click = Some((NodeId::new("n1"), 10));
        let before_doc = state.doc.clone();
        let before_selection = state.selection.clone();
        let before_click = state.editor_ui.last_canvas_click.clone();
        let namespace = PeerNamespace::try_from("drag-peer").unwrap();
        let mut allocator = NamespacedIdAllocator::new(namespace, u64::MAX);

        let result = activate_node_drag_with_allocator(&mut state, &mut allocator, true, 3.0, 4.0);

        assert_eq!(result.err(), Some(IdAllocError::CounterExhausted));
        assert_eq!(state.doc, before_doc);
        assert_eq!(state.selection, before_selection);
        assert_eq!(state.editor_ui.last_canvas_click, before_click);
    }
}
