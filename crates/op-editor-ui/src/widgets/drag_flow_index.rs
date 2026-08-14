//! Gesture-scoped lookup index for pointer-rate canvas drop previews.

use super::drag_flow::DragCommitPlan;
use crate::layout_scene::{LayoutScene, SceneNode};
use crate::{Point2D, Rect};
use jian_ops_schema::node::PenNode;
use op_editor_core::drag_mutators::{auto_layout_direction, DragDropTarget, FlexDirection};
use op_editor_core::editor_ui_state::{CanvasDropIndicator, CanvasOverlayLine, CanvasOverlayRect};
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CanvasDropIndexKey {
    document_generation: u64,
    document_revision: u64,
    active_page_index: usize,
    dragged_id: String,
    excluded_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct SceneBounds {
    bounds: Rect,
    aggregate_bounds: Rect,
}

#[derive(Debug)]
struct IndexedContainer {
    id: NodeId,
    bounds: Rect,
    flex: Option<FlexDirection>,
    flex_children: Vec<Rect>,
    children: Vec<IndexedContainer>,
}

struct ContainerDropCandidate {
    parent_id: NodeId,
    bounds: Rect,
    flex: Option<FlexDirection>,
    index: usize,
    insertion: Option<CanvasOverlayLine>,
}

/// Static document and scene lookups reused for one drag gesture.
#[derive(Debug)]
pub struct CanvasDropIndex {
    key: CanvasDropIndexKey,
    current_parent: Option<NodeId>,
    current_index: usize,
    current_parent_flex: Option<FlexDirection>,
    dragged_scene_path: Vec<usize>,
    containers: Vec<IndexedContainer>,
}

impl CanvasDropIndex {
    /// Whether this index still represents the current gesture inputs.
    pub fn matches(
        &self,
        state: &EditorState,
        scene: &LayoutScene,
        dragged_id: &NodeId,
        excluded_ids: &[NodeId],
    ) -> bool {
        self.key == canvas_drop_index_key(state, scene, dragged_id, excluded_ids)
    }

    pub(super) fn current_parent(&self) -> Option<&NodeId> {
        self.current_parent.as_ref()
    }

    pub(super) fn current_index(&self) -> usize {
        self.current_index
    }
}

/// Build all static tree and scene lookups once for a drag gesture.
pub fn build_canvas_drop_index(
    state: &EditorState,
    scene: &LayoutScene,
    dragged_id: &NodeId,
    excluded_ids: &[NodeId],
) -> Option<CanvasDropIndex> {
    let key = canvas_drop_index_key(state, scene, dragged_id, excluded_ids);
    let nodes = state.active_children();
    let source = op_editor_core::walkers::find_node(nodes, dragged_id)?;
    let (current_parent, current_index) =
        op_editor_core::walkers::find_parent_and_index(nodes, dragged_id)?;
    let current_parent_flex = current_parent
        .as_ref()
        .and_then(|parent_id| op_editor_core::walkers::find_node(nodes, parent_id))
        .and_then(auto_layout_direction);
    let page = scene.active_page()?;
    let mut scene_bounds = HashMap::new();
    collect_scene_bounds(&page.children, &mut scene_bounds);
    let mut dragged_scene_path = Vec::new();
    if !find_scene_path(&page.children, dragged_id.as_str(), &mut dragged_scene_path) {
        return None;
    }
    let mut excluded = HashSet::new();
    collect_subtree_ids(source, &mut excluded);
    for id in excluded_ids {
        if let Some(node) = op_editor_core::walkers::find_node(nodes, id) {
            collect_subtree_ids(node, &mut excluded);
        }
    }
    let containers = index_containers(nodes, &scene_bounds, &excluded, dragged_id);
    Some(CanvasDropIndex {
        key,
        current_parent,
        current_index,
        current_parent_flex,
        dragged_scene_path,
        containers,
    })
}

/// Pointer-rate read phase using a gesture-scoped drop index.
pub fn plan_drag_commit_indexed(
    scene: &LayoutScene,
    index: &CanvasDropIndex,
    total_dx: f64,
    total_dy: f64,
) -> Option<DragCommitPlan> {
    let page = scene.active_page()?;
    let node_scene = scene_node_at_path(&page.children, &index.dragged_scene_path)?;
    let mut bounds = node_scene.aggregate_bounds();
    if index.current_parent_flex.is_some() {
        bounds.origin.x += total_dx as f32;
        bounds.origin.y += total_dy as f32;
    }
    let center = Point2D::new(
        bounds.origin.x + bounds.size.x / 2.0,
        bounds.origin.y + bounds.size.y / 2.0,
    );
    let candidate = container_drop_candidate(index, center, bounds);
    let mut indicator = None;
    let target = if let Some(candidate) = candidate {
        let same_parent = index.current_parent.as_ref() == Some(&candidate.parent_id);
        if same_parent && candidate.flex.is_none() {
            None
        } else {
            indicator = Some(CanvasDropIndicator {
                ghost: overlay_rect(bounds),
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
    } else if index.current_parent.is_some() {
        indicator = Some(CanvasDropIndicator {
            ghost: overlay_rect(bounds),
            target: None,
            insertion: None,
        });
        Some(DragDropTarget::PageRoot { index: 0 })
    } else {
        None
    };
    Some(DragCommitPlan {
        target,
        dropped_bounds: bounds,
        indicator,
    })
}

fn canvas_drop_index_key(
    state: &EditorState,
    scene: &LayoutScene,
    dragged_id: &NodeId,
    excluded_ids: &[NodeId],
) -> CanvasDropIndexKey {
    CanvasDropIndexKey {
        document_generation: state.document_generation(),
        document_revision: state.document_revision(),
        active_page_index: scene.active_page_index,
        dragged_id: dragged_id.as_str().to_string(),
        excluded_ids: excluded_ids
            .iter()
            .map(|id| id.as_str().to_string())
            .collect(),
    }
}

fn find_scene_path(nodes: &[SceneNode], id: &str, path: &mut Vec<usize>) -> bool {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        if node.id == id || find_scene_path(&node.children, id, path) {
            return true;
        }
        path.pop();
    }
    false
}

fn scene_node_at_path<'a>(nodes: &'a [SceneNode], path: &[usize]) -> Option<&'a SceneNode> {
    let (&index, rest) = path.split_first()?;
    let node = nodes.get(index)?;
    if rest.is_empty() {
        Some(node)
    } else {
        scene_node_at_path(&node.children, rest)
    }
}

fn collect_scene_bounds(nodes: &[SceneNode], out: &mut HashMap<String, SceneBounds>) {
    for node in nodes {
        out.insert(
            node.id.clone(),
            SceneBounds {
                bounds: node.bounds,
                aggregate_bounds: node.aggregate_bounds(),
            },
        );
        collect_scene_bounds(&node.children, out);
    }
}

fn collect_subtree_ids(node: &PenNode, out: &mut HashSet<String>) {
    out.insert(node.id_str().to_string());
    if let Some(children) = node.children() {
        for child in children {
            collect_subtree_ids(child, out);
        }
    }
}

fn index_containers(
    nodes: &[PenNode],
    scene_bounds: &HashMap<String, SceneBounds>,
    excluded: &HashSet<String>,
    dragged_id: &NodeId,
) -> Vec<IndexedContainer> {
    let mut indexed = Vec::new();
    for node in nodes {
        let Some(children) = node.children() else {
            continue;
        };
        if excluded.contains(node.id_str()) {
            continue;
        }
        let Some(scene) = scene_bounds.get(node.id_str()) else {
            continue;
        };
        let flex = auto_layout_direction(node);
        let flex_children = if flex.is_some() {
            children
                .iter()
                .filter(|child| child.id_str() != dragged_id.as_str())
                .filter_map(|child| {
                    scene_bounds
                        .get(child.id_str())
                        .map(|scene| scene.aggregate_bounds)
                })
                .collect()
        } else {
            Vec::new()
        };
        indexed.push(IndexedContainer {
            id: NodeId::new(node.id_str()),
            bounds: scene.bounds,
            flex,
            flex_children,
            children: index_containers(children, scene_bounds, excluded, dragged_id),
        });
    }
    indexed
}

fn container_drop_candidate(
    index: &CanvasDropIndex,
    point: Point2D,
    dragged_bounds: Rect,
) -> Option<ContainerDropCandidate> {
    let container = deepest_container_at(&index.containers, point)?;
    let (insert_index, insertion) = if let Some(direction) = container.flex {
        flex_insert_preview(container, dragged_bounds, direction)
    } else {
        (0, None)
    };
    Some(ContainerDropCandidate {
        parent_id: container.id.clone(),
        bounds: container.bounds,
        flex: container.flex,
        index: insert_index,
        insertion,
    })
}

fn deepest_container_at(
    containers: &[IndexedContainer],
    point: Point2D,
) -> Option<&IndexedContainer> {
    let mut hit = None;
    for container in containers {
        if !rect_contains(container.bounds, point) {
            continue;
        }
        hit = Some(container);
        if let Some(deeper) = deepest_container_at(&container.children, point) {
            hit = Some(deeper);
        }
    }
    hit
}

fn flex_insert_preview(
    parent: &IndexedContainer,
    dragged_bounds: Rect,
    direction: FlexDirection,
) -> (usize, Option<CanvasOverlayLine>) {
    let vertical = matches!(direction, FlexDirection::Vertical);
    let drag_mid = if vertical {
        dragged_bounds.origin.y + dragged_bounds.size.y / 2.0
    } else {
        dragged_bounds.origin.x + dragged_bounds.size.x / 2.0
    };
    let mut index = parent.flex_children.len();
    for (position, bounds) in parent.flex_children.iter().copied().enumerate() {
        let mid = if vertical {
            bounds.origin.y + bounds.size.y / 2.0
        } else {
            bounds.origin.x + bounds.size.x / 2.0
        };
        if drag_mid < mid {
            index = position;
            break;
        }
    }
    let insertion = insertion_line(parent.bounds, &parent.flex_children, index, vertical);
    (index, insertion)
}

fn insertion_line(
    parent_bounds: Rect,
    siblings: &[Rect],
    index: usize,
    vertical: bool,
) -> Option<CanvasOverlayLine> {
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
            let previous = siblings[index - 1];
            let next = siblings[index];
            (previous.origin.y + previous.size.y + next.origin.y) / 2.0
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
            let previous = siblings[index - 1];
            let next = siblings[index];
            (previous.origin.x + previous.size.x + next.origin.x) / 2.0
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
