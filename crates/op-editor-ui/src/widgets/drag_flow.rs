//! Canvas node-drag flow shared by the native and web widget hosts.
//!
//! The two hosts' `widget_host/canvas_select_drag.rs` and
//! `widget_host/node_drag.rs` twins used to carry this whole
//! read-plan / mutate / preview pipeline as a verbatim copy-paste run.
//! Everything below reads the layout scene and mutates `EditorState`
//! only, so it stays wasm32-clean; the hosts keep the platform tail
//! (`refresh_layout_scene`, `scene_cache.invalidate`, `mark_dirty`,
//! layout transitions, drag-state bookkeeping) in thin wrappers.

use jian_ops_schema::node::PenNode;
use op_editor_core::drag_mutators::{
    auto_layout_direction, parent_of, DragDropTarget, FlexDirection,
};
use op_editor_core::editor_ui_state::{CanvasDropIndicator, CanvasOverlayLine, CanvasOverlayRect};
use op_editor_core::PenNodeExt;
use op_editor_core::{EditorState, NodeId};

use crate::layout_scene::{LayoutScene, ScenePage};
use crate::{Point2D, Rect};

pub use super::drag_flow_index::{
    build_canvas_drop_index, plan_drag_commit_indexed, CanvasDropIndex,
};

/// Read-phase summary of one dragged node — collected before any
/// mutation so no document / scene borrow survives into the mutators.
pub struct DragCommitPlan {
    /// Where the node should land, or `None` when the drop is a no-op.
    pub target: Option<DragDropTarget>,
    /// Absolute bounds the node occupies at the drop point.
    pub dropped_bounds: Rect,
    /// Ghost / target / insertion overlay for the live drop preview.
    pub indicator: Option<CanvasDropIndicator>,
}

/// Result of one live-preview step on a single-selection drag.
pub struct LiveDragPreview {
    /// The preview reordered / reparented the tree.
    pub mutated: bool,
    /// Bounds the drag overlay should ghost at, when the node stayed
    /// inside its current auto-layout parent.
    pub overlay_bounds: Option<Rect>,
    /// Scene snapshot captured only when the preview is about to mutate
    /// the document. Hosts use it to animate sibling reflow.
    pub before_scene: Option<LayoutScene>,
}

struct ContainerDropCandidate {
    parent_id: NodeId,
    bounds: Rect,
    flex: Option<FlexDirection>,
    index: usize,
    insertion: Option<CanvasOverlayLine>,
}

/// Read phase — gather everything the commit needs as owned data.
///
/// `total_dx` / `total_dy` are the gesture's net doc-space travel since
/// the press; flex children never doc-translate during a drag, so that
/// delta is the only record of where the user dropped them.
pub fn plan_drag_commit(
    state: &EditorState,
    scene: &LayoutScene,
    id: &NodeId,
    total_dx: f64,
    total_dy: f64,
    excluded_ids: &[NodeId],
) -> Option<DragCommitPlan> {
    let children = state.active_children();
    let current_parent = parent_of(children, id);
    let current_parent_flex = current_parent
        .as_ref()
        .and_then(|parent_id| op_editor_core::walkers::find_node(children, parent_id))
        .and_then(auto_layout_direction);
    let page = scene.active_page()?;
    let node_scene = page.find(id.as_str())?;
    let mut nb = node_scene.aggregate_bounds();
    if current_parent_flex.is_some() {
        // Flex children never doc-translate during the drag — the
        // accumulated cursor delta is where the user dropped them.
        nb.origin.x += total_dx as f32;
        nb.origin.y += total_dy as f32;
    }
    let center = Point2D::new(nb.origin.x + nb.size.x / 2.0, nb.origin.y + nb.size.y / 2.0);
    let candidate = container_drop_candidate(state, page, id, center, nb, excluded_ids);
    let mut indicator = None;
    let target = if let Some(candidate) = candidate {
        let same_parent = current_parent.as_ref() == Some(&candidate.parent_id);
        if same_parent && candidate.flex.is_none() {
            None
        } else {
            indicator = Some(CanvasDropIndicator {
                ghost: overlay_rect(nb),
                target: Some(overlay_rect(candidate.bounds)),
                insertion: candidate.insertion,
            });
            Some(DragDropTarget::Container {
                parent_id: candidate.parent_id,
                parent_abs_x: candidate.bounds.origin.x as f64,
                parent_abs_y: candidate.bounds.origin.y as f64,
                index: candidate.index,
            })
        }
    } else if current_parent.is_some() {
        indicator = Some(CanvasDropIndicator {
            ghost: overlay_rect(nb),
            target: None,
            insertion: None,
        });
        Some(DragDropTarget::PageRoot { index: 0 })
    } else {
        None
    };
    Some(DragCommitPlan {
        target,
        dropped_bounds: nb,
        indicator,
    })
}

/// Mutation phase — apply the planned reparent / reorder.
pub fn apply_drag_commit(state: &mut EditorState, id: &NodeId, plan: DragCommitPlan) -> bool {
    let Some(target) = plan.target else {
        return false;
    };
    let bounds = plan.dropped_bounds;
    state.move_node_to_drop_target(
        id,
        target,
        bounds.origin.x as f64,
        bounds.origin.y as f64,
        bounds.size.x as f64,
        bounds.size.y as f64,
    )
}

/// Release commit for the whole selection: a node dropped into another
/// container reparents there; a node dropped outside every container
/// becomes a page root; a child dropped within an auto-layout parent
/// re-inserts at the midpoint-derived sibling index. Free-layout
/// children were already translated live.
///
/// Returns `true` when the tree changed — the caller then invalidates
/// its scene cache (the drag snapshot already consumed this gesture's
/// document revision) and repaints.
pub fn commit_node_drag(
    state: &mut EditorState,
    scene: &LayoutScene,
    total_dx: f64,
    total_dy: f64,
    excluded_ids: &[NodeId],
) -> bool {
    let ids = state.selection.set.clone();
    let mut mutated = false;
    for id in &ids {
        let Some(plan) = plan_drag_commit(state, scene, id, total_dx, total_dy, excluded_ids)
        else {
            continue;
        };
        mutated |= apply_drag_commit(state, id, plan);
    }
    mutated
}

/// Recompute the drop indicator for the anchor node without mutating
/// the tree (the multi-selection preview path).
pub fn refresh_drop_indicator(
    state: &mut EditorState,
    scene: &LayoutScene,
    total_dx: f64,
    total_dy: f64,
    excluded_ids: &[NodeId],
) {
    let id = state.selection.anchor.clone();
    let next = if id.is_real() {
        plan_drag_commit(state, scene, &id, total_dx, total_dy, excluded_ids)
            .and_then(|plan| plan.indicator)
    } else {
        None
    };
    if state.editor_ui.canvas_drop_indicator != next {
        state.editor_ui.canvas_drop_indicator = next;
    }
}

/// Single-selection live preview: reorder inside the current
/// auto-layout parent as the cursor travels, lift a node out of its
/// parent as soon as it leaves, and otherwise only paint the indicator.
///
/// `None` means the node has no plan at all — the indicator was
/// cleared and the caller must leave its overlay bounds untouched.
pub fn apply_live_drag_preview(
    state: &mut EditorState,
    scene: &LayoutScene,
    id: &NodeId,
    total_dx: f64,
    total_dy: f64,
    excluded_ids: &[NodeId],
) -> Option<LiveDragPreview> {
    let Some(plan) = plan_drag_commit(state, scene, id, total_dx, total_dy, excluded_ids) else {
        state.editor_ui.canvas_drop_indicator = None;
        return None;
    };
    let (current_parent, current_index) =
        op_editor_core::walkers::find_parent_and_index(state.active_children(), id)?;
    Some(apply_live_drag_preview_plan(
        state,
        scene,
        id,
        current_parent,
        current_index,
        plan,
    ))
}

/// Single-selection live preview backed by a gesture-scoped drop index.
pub fn apply_live_drag_preview_indexed(
    state: &mut EditorState,
    scene: &LayoutScene,
    index: &CanvasDropIndex,
    id: &NodeId,
    total_dx: f64,
    total_dy: f64,
) -> Option<LiveDragPreview> {
    let Some(plan) = plan_drag_commit_indexed(scene, index, total_dx, total_dy) else {
        state.editor_ui.canvas_drop_indicator = None;
        return None;
    };
    Some(apply_live_drag_preview_plan(
        state,
        scene,
        id,
        index.current_parent().cloned(),
        index.current_index(),
        plan,
    ))
}

fn apply_live_drag_preview_plan(
    state: &mut EditorState,
    scene: &LayoutScene,
    id: &NodeId,
    current_parent: Option<NodeId>,
    current_index: usize,
    plan: DragCommitPlan,
) -> LiveDragPreview {
    let bounds = plan.dropped_bounds;
    let planned_indicator = plan.indicator.clone();
    let mut indicator = None;
    let mut mutated = false;
    let mut overlay_bounds = None;
    let mut before_scene = None;
    if let Some(target) = plan.target.clone() {
        match target {
            DragDropTarget::Container {
                ref parent_id,
                index,
                ..
            } if current_parent.as_ref() == Some(parent_id) => {
                overlay_bounds = Some(bounds);
                if current_index != index {
                    before_scene = Some(scene.clone());
                    mutated |= apply_drag_commit(state, id, plan);
                }
            }
            DragDropTarget::PageRoot { .. } if current_parent.is_some() => {
                before_scene = Some(scene.clone());
                mutated |= apply_drag_commit(state, id, plan);
            }
            DragDropTarget::Container { .. } if current_parent.is_some() => {
                before_scene = Some(scene.clone());
                mutated |= state.move_node_to_drop_target(
                    id,
                    DragDropTarget::PageRoot { index: 0 },
                    bounds.origin.x as f64,
                    bounds.origin.y as f64,
                    bounds.size.x as f64,
                    bounds.size.y as f64,
                );
                indicator = planned_indicator;
            }
            _ => {
                indicator = planned_indicator;
            }
        }
    }
    if state.editor_ui.canvas_drop_indicator != indicator {
        state.editor_ui.canvas_drop_indicator = indicator;
    }
    LiveDragPreview {
        mutated,
        overlay_bounds,
        before_scene,
    }
}

/// Scene ids the incremental drag patch may translate: exactly what
/// `translate_selected` moved in the document — editable nodes only
/// (locked / hidden are skipped there) and not flex-flow children
/// (positioned by their parent). Otherwise the scene would drift nodes
/// the doc never moved, then snap back on the release-time
/// reconversion.
pub fn drag_scene_translate_ids(state: &EditorState) -> Vec<String> {
    let children = state.active_children();
    state
        .selection
        .set
        .iter()
        .filter(|id| {
            state.is_editable(id) && !op_editor_core::walkers::is_flow_child_of_flex(children, id)
        })
        .map(|id| id.as_str().to_string())
        .collect()
}

fn container_drop_candidate(
    state: &EditorState,
    page: &ScenePage,
    dragged_id: &NodeId,
    point: Point2D,
    dragged_bounds: Rect,
    excluded_ids: &[NodeId],
) -> Option<ContainerDropCandidate> {
    let children = state.active_children();
    let source = op_editor_core::walkers::find_node(children, dragged_id)?;
    deepest_container_at(children, source, point, page, excluded_ids).map(
        |(parent_id, bounds, flex)| {
            let (index, insertion) = if let Some(dir) = flex {
                flex_insert_preview(children, page, &parent_id, dragged_id, dragged_bounds, dir)
            } else {
                (0, None)
            };
            ContainerDropCandidate {
                parent_id,
                bounds,
                flex,
                index,
                insertion,
            }
        },
    )
}

fn deepest_container_at(
    nodes: &[PenNode],
    source: &PenNode,
    point: Point2D,
    page: &ScenePage,
    excluded_ids: &[NodeId],
) -> Option<(NodeId, Rect, Option<FlexDirection>)> {
    let mut hit = None;
    for node in nodes {
        if node.children().is_none() {
            continue;
        }
        let node_id = NodeId::new(node.id_str());
        if excluded_ids.contains(&node_id) {
            continue;
        }
        if op_editor_core::walkers::descendant_contains(source, &node_id) {
            continue;
        }
        let Some(scene) = page.find(node.id_str()) else {
            continue;
        };
        let bounds = scene.bounds;
        if !rect_contains(bounds, point) {
            continue;
        }
        hit = Some((node_id, bounds, auto_layout_direction(node)));
        if let Some(children) = node.children() {
            if let Some(deeper) = deepest_container_at(children, source, point, page, excluded_ids)
            {
                hit = Some(deeper);
            }
        }
    }
    hit
}

fn flex_insert_preview(
    nodes: &[PenNode],
    page: &ScenePage,
    parent_id: &NodeId,
    dragged_id: &NodeId,
    dragged_bounds: Rect,
    dir: FlexDirection,
) -> (usize, Option<CanvasOverlayLine>) {
    let Some(parent) = op_editor_core::walkers::find_node(nodes, parent_id) else {
        return (0, None);
    };
    let Some(parent_scene) = page.find(parent_id.as_str()) else {
        return (0, None);
    };
    let parent_bounds = parent_scene.bounds;
    let vertical = matches!(dir, FlexDirection::Vertical);
    let drag_mid = if vertical {
        dragged_bounds.origin.y + dragged_bounds.size.y / 2.0
    } else {
        dragged_bounds.origin.x + dragged_bounds.size.x / 2.0
    };
    let mut index = parent
        .children()
        .map(|children| {
            children
                .iter()
                .filter(|node| node.id_str() != dragged_id.as_str())
                .count()
        })
        .unwrap_or(0);
    if let Some(children) = parent.children() {
        for (i, child) in children
            .iter()
            .filter(|node| node.id_str() != dragged_id.as_str())
            .enumerate()
        {
            let Some(scene) = page.find(child.id_str()) else {
                continue;
            };
            let bounds = scene.aggregate_bounds();
            let mid = if vertical {
                bounds.origin.y + bounds.size.y / 2.0
            } else {
                bounds.origin.x + bounds.size.x / 2.0
            };
            if drag_mid < mid {
                index = i;
                break;
            }
        }
    }
    let insertion = flex_insertion_line(parent, page, parent_bounds, dragged_id, index, vertical);
    (index, insertion)
}

fn flex_insertion_line(
    parent: &PenNode,
    page: &ScenePage,
    parent_bounds: Rect,
    dragged_id: &NodeId,
    index: usize,
    vertical: bool,
) -> Option<CanvasOverlayLine> {
    let siblings: Vec<Rect> = parent
        .children()?
        .iter()
        .filter(|node| node.id_str() != dragged_id.as_str())
        .filter_map(|node| {
            page.find(node.id_str())
                .map(|scene| scene.aggregate_bounds())
        })
        .collect();
    let inset = 8.0_f32.min(parent_bounds.size.x.max(parent_bounds.size.y) / 4.0);
    if vertical {
        let y = if siblings.is_empty() {
            parent_bounds.origin.y + parent_bounds.size.y / 2.0
        } else if index == 0 {
            siblings[0].origin.y
        } else if index >= siblings.len() {
            let last = siblings[siblings.len() - 1];
            last.origin.y + last.size.y
        } else {
            let prev = siblings[index - 1];
            let next = siblings[index];
            (prev.origin.y + prev.size.y + next.origin.y) / 2.0
        };
        Some(CanvasOverlayLine::new(
            (parent_bounds.origin.x + inset) as f64,
            y as f64,
            (parent_bounds.origin.x + parent_bounds.size.x - inset) as f64,
            y as f64,
        ))
    } else {
        let x = if siblings.is_empty() {
            parent_bounds.origin.x + parent_bounds.size.x / 2.0
        } else if index == 0 {
            siblings[0].origin.x
        } else if index >= siblings.len() {
            let last = siblings[siblings.len() - 1];
            last.origin.x + last.size.x
        } else {
            let prev = siblings[index - 1];
            let next = siblings[index];
            (prev.origin.x + prev.size.x + next.origin.x) / 2.0
        };
        Some(CanvasOverlayLine::new(
            x as f64,
            (parent_bounds.origin.y + inset) as f64,
            x as f64,
            (parent_bounds.origin.y + parent_bounds.size.y - inset) as f64,
        ))
    }
}

fn rect_contains(rect: Rect, point: Point2D) -> bool {
    point.x >= rect.origin.x
        && point.x <= rect.origin.x + rect.size.x
        && point.y >= rect.origin.y
        && point.y <= rect.origin.y + rect.size.y
}

fn overlay_rect(rect: Rect) -> CanvasOverlayRect {
    CanvasOverlayRect::new(
        rect.origin.x as f64,
        rect.origin.y as f64,
        rect.size.x as f64,
        rect.size.y as f64,
    )
}
