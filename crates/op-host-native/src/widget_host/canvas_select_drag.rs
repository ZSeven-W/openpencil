//! Canvas selection semantics (one-level drill on double-click) + the
//! node-drag release commit (auto-layout reorder / cross-container
//! reparenting).
//!
//! TS sources: `skia-interaction.ts:1182-1256` (`handleDragEnd`),
//! `:1262-1294` (double-click enter-group) and
//! `op_editor_core::selection_resolve`; this module is the host glue
//! that reads the layout scene for absolute bounds.

use super::{NodeDragState, WidgetHostNative};
use jian_ops_schema::node::PenNode;
use op_editor_core::drag_mutators::{auto_layout_direction, DragDropTarget, FlexDirection};
use op_editor_core::editor_ui_state::{CanvasDropIndicator, CanvasOverlayLine, CanvasOverlayRect};
use op_editor_core::{NodeId, PenNodeExt};
use op_editor_ui::widgets::CanvasNodeDragOverlay;
use op_editor_ui::{Point2D, Rect};
use std::collections::{HashMap, HashSet};

#[cfg(test)]
thread_local! {
    static DROP_INDEX_BUILD_COUNT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(in crate::widget_host) fn reset_drop_index_build_count() {
    DROP_INDEX_BUILD_COUNT.with(|count| count.set(0));
}

#[cfg(test)]
pub(in crate::widget_host) fn drop_index_build_count() -> usize {
    DROP_INDEX_BUILD_COUNT.with(std::cell::Cell::get)
}

/// Read-phase summary of one dragged node — collected before any
/// mutation so no document / scene borrow survives into the mutators.
struct DragCommitPlan {
    target: Option<DragDropTarget>,
    dropped_bounds: Rect,
    indicator: Option<CanvasDropIndicator>,
}

struct ContainerDropCandidate {
    parent_id: NodeId,
    bounds: Rect,
    flex: Option<FlexDirection>,
    index: usize,
    insertion: Option<CanvasOverlayLine>,
}

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

#[derive(Debug)]
pub(in crate::widget_host) struct CanvasDropIndex {
    key: CanvasDropIndexKey,
    current_parent: Option<NodeId>,
    current_index: usize,
    current_parent_flex: Option<FlexDirection>,
    dragged_scene_path: Vec<usize>,
    containers: Vec<IndexedContainer>,
}

impl WidgetHostNative {
    /// Select-tool press over a resolved root-to-deepest hit path.
    /// A plain click selects the current level's primary node; a
    /// double-click drills exactly one level to the child under the
    /// pointer. The primary then becomes the entered sibling scope.
    /// This is the same hierarchy Pencil exposes with a solid primary
    /// hover outline and dashed direct-child guides.
    pub(in crate::widget_host) fn apply_canvas_node_press(
        &mut self,
        hit_path: Vec<NodeId>,
        x: f32,
        y: f32,
        text_edit_was_active: bool,
        viewport_height: f32,
    ) -> bool {
        self.canvas_drop_index = None;
        let Some(deepest) = hit_path.last().cloned() else {
            return false;
        };
        let Some(targets) = op_editor_core::selection_resolve::resolve_canvas_depth_targets(
            &hit_path,
            self.editor_state.editor_ui.entered_container.as_ref(),
        ) else {
            return false;
        };
        // Canvas double-click: 400 ms over the same deepest geometry.
        // Shift and an existing multi-selection deliberately disable
        // drill-down so a set-edit gesture cannot unexpectedly enter a
        // container.
        let is_double = matches!(
            &self.editor_state.editor_ui.last_canvas_click,
            Some((prev, t)) if *prev == deepest
                && self.now_ms.saturating_sub(*t) < 400
        ) && !self.shift_held
            && self.editor_state.selection_count() <= 1;
        self.editor_state.editor_ui.last_canvas_click = if self.shift_held || is_double {
            None
        } else {
            Some((deepest.clone(), self.now_ms))
        };
        // A leaf selected directly from the Layer panel can sit below the
        // canvas depth resolver's primary target. Preserve that exact crop
        // selection on the first press so the second press can activate crop
        // editing. A child hit does not qualify: it must retain ordinary
        // one-level drill semantics.
        let selected_crop_is_deepest = deepest == self.editor_state.selection.anchor
            && self.editor_state.selection_count() == 1
            && self.editor_state.can_edit_selected_image_crop();
        let primary = if selected_crop_is_deepest {
            deepest.clone()
        } else {
            targets.primary.clone()
        };
        if let Some(editing) = self.editor_state.editor_ui.image_crop_editing.clone() {
            // A crop can be selected directly from the Layer panel while its
            // ancestors remain the canvas depth resolver's primary target.
            // The rendered hit path is authoritative: any descendant hit
            // inside the editing node should pan that node's bitmap.
            if hit_path.contains(&editing)
                && self.start_active_image_crop_drag(&editing, &hit_path, x, y)
            {
                return true;
            }
            // A press on another node exits the dedicated crop editor, then
            // continues through ordinary selection/drag routing.
            self.exit_image_crop_edit();
        }
        if is_double && !text_edit_was_active {
            if selected_crop_is_deepest && self.enter_selected_image_crop_edit() {
                return true;
            }
            if let Some(secondary) = targets.secondary_under_pointer {
                self.editor_state.set_single_selection(secondary.clone());
                self.editor_state.editor_ui.entered_container = Some(targets.primary);
                // Rebase the stationary-pointer hover immediately. The
                // native 3 px probe cache would otherwise retain the old
                // level until the mouse moved again.
                self.editor_state.editor_ui.canvas_hover_node = Some(secondary);
                self.last_hover_probe = None;
                self.scroll_layer_panel_selection_into_view(viewport_height);
                self.mark_dirty();
                return true;
            }
            if self.editor_state.start_text_edit(targets.primary.clone()) {
                self.editor_state.ui.text_edit_input.touch(self.now_ms);
                self.mark_dirty();
                return true;
            }
        }
        let target = primary;
        let fresh_drag = NodeDragState {
            last_screen_x: x,
            last_screen_y: y,
            press_screen_x: x,
            press_screen_y: y,
            moved: false,
            total_dx: 0.0,
            total_dy: 0.0,
            overlay_bounds: None,
        };
        self.option_drag_source_ids.clear();
        if self.shift_held {
            let was_in_set = self.editor_state.is_selected(&target);
            self.editor_state.toggle_selection(target);
            if !was_in_set {
                self.node_drag = Some(fresh_drag);
            }
            if self
                .editor_state
                .editor_ui
                .entered_container
                .as_ref()
                .is_some_and(|entered| !hit_path.contains(entered))
            {
                self.editor_state.editor_ui.entered_container = None;
            }
            self.scroll_layer_panel_selection_into_view(viewport_height);
            return true;
        }
        // Plain click selects the solid-outline primary. Clicking a
        // sibling inside an entered scope therefore moves selection at
        // that level instead of dragging an arbitrary deepest leaf.
        let already_in_set = self.editor_state.is_selected(&target);
        if !already_in_set || self.editor_state.selection_count() == 1 {
            self.editor_state.set_single_selection(target);
        }
        if self
            .editor_state
            .editor_ui
            .entered_container
            .as_ref()
            .is_some_and(|entered| !hit_path.contains(entered))
        {
            self.editor_state.editor_ui.entered_container = None;
        }
        self.scroll_layer_panel_selection_into_view(viewport_height);
        self.editor_state.commit_history();
        self.node_drag = Some(fresh_drag);
        true
    }

    pub(in crate::widget_host) fn update_node_drag_preview(&mut self, drag: &NodeDragState) {
        let id = self.editor_state.selection.anchor.clone();
        let next = if id.is_real() {
            self.plan_drag_commit(&id, drag)
                .and_then(|plan| plan.indicator)
        } else {
            None
        };
        if self.editor_state.editor_ui.canvas_drop_indicator != next {
            self.editor_state.editor_ui.canvas_drop_indicator = next;
        }
    }

    pub(in crate::widget_host) fn apply_live_node_drag_preview(&mut self, drag: &NodeDragState) {
        if self.editor_state.selection_count() != 1 {
            self.update_node_drag_preview(drag);
            return;
        }
        self.refresh_layout_scene();
        let id = self.editor_state.selection.anchor.clone();
        let Some(plan) = self.plan_drag_commit(&id, drag) else {
            self.editor_state.editor_ui.canvas_drop_indicator = None;
            return;
        };
        let (current_parent, current_index) = self
            .canvas_drop_index
            .as_ref()
            .map(|index| (index.current_parent.clone(), index.current_index))
            .unwrap_or((None, 0));
        let bounds = plan.dropped_bounds;
        let mut indicator = None;
        let mut mutated = false;
        let mut overlay_bounds = None;
        let mut before_scene = None;
        if let Some(target) = plan.target.clone() {
            match &target {
                DragDropTarget::Container {
                    parent_id, index, ..
                } if current_parent.as_ref() == Some(parent_id) => {
                    overlay_bounds = Some(bounds);
                    if current_index != *index {
                        before_scene = Some(self.layout_scene.clone());
                        mutated |= self.apply_drag_commit(&id, plan);
                    }
                }
                DragDropTarget::PageRoot { .. } if current_parent.is_some() => {
                    before_scene = Some(self.layout_scene.clone());
                    mutated |= self.apply_drag_commit(&id, plan);
                }
                DragDropTarget::Container { .. } if current_parent.is_some() => {
                    before_scene = Some(self.layout_scene.clone());
                    mutated |= self.editor_state.move_node_to_drop_target(
                        &id,
                        DragDropTarget::PageRoot { index: 0 },
                        bounds.origin.x as f64,
                        bounds.origin.y as f64,
                        bounds.size.x as f64,
                        bounds.size.y as f64,
                    );
                    indicator = plan.indicator;
                }
                _ => {
                    indicator = plan.indicator;
                }
            }
        }
        if mutated {
            self.canvas_drop_index = None;
            // Drag history advances the document revision when the gesture
            // starts, before this live reorder mutates the tree. Invalidate the
            // revision-keyed scene cache so sibling reflow observes the new
            // order instead of reusing the pre-mutation scene.
            self.scene_cache.invalidate();
            self.mark_dirty();
            if let Some(before_scene) = before_scene {
                self.start_layout_transition_from_scene_excluding(before_scene, &id);
            }
        }
        if self.editor_state.editor_ui.canvas_drop_indicator != indicator {
            self.editor_state.editor_ui.canvas_drop_indicator = indicator;
        }
        if let Some(active_drag) = self.node_drag.as_mut() {
            active_drag.overlay_bounds = overlay_bounds;
        }
    }

    pub(in crate::widget_host) fn node_drag_overlay_for_paint(
        &self,
    ) -> Option<CanvasNodeDragOverlay> {
        let drag = self.node_drag?;
        let overlay_bounds = drag.overlay_bounds?;
        let node_id = self.editor_state.selection.anchor.clone();
        Some(CanvasNodeDragOverlay {
            node_id: node_id.as_str().to_string(),
            target_origin_doc: overlay_bounds.origin,
        })
    }

    /// Node-drag release commit: a node dropped into another container
    /// reparents there; a node dropped outside every container becomes
    /// a page root; a child dropped within an auto-layout parent
    /// re-inserts at the midpoint-derived sibling index. Free-layout
    /// children were already translated live.
    pub(in crate::widget_host) fn commit_node_drag(&mut self, drag: &NodeDragState) -> bool {
        if !drag.moved {
            return false;
        }
        // Free-node translations were committed live into the doc;
        // re-derive the scene so the policy reads dropped positions.
        self.refresh_layout_scene();
        let ids = self.editor_state.selection.set.clone();
        let mut mutated = false;
        for id in &ids {
            let Some(plan) = self.plan_drag_commit(id, drag) else {
                continue;
            };
            mutated |= self.apply_drag_commit(id, plan);
        }
        if mutated {
            self.canvas_drop_index = None;
            // The drag snapshot already consumed this gesture's document
            // revision, so the final tree mutation must invalidate the scene
            // cache explicitly.
            self.scene_cache.invalidate();
            self.mark_dirty();
        }
        mutated
    }

    /// Read phase — gather everything the commit needs as owned data.
    fn plan_drag_commit(&mut self, id: &NodeId, drag: &NodeDragState) -> Option<DragCommitPlan> {
        self.ensure_canvas_drop_index(id)?;
        let index = self.canvas_drop_index.as_ref()?;
        let current_parent = index.current_parent.clone();
        let current_parent_flex = index.current_parent_flex;
        let page = self.layout_scene.active_page()?;
        let node_scene = scene_node_at_path(&page.children, &index.dragged_scene_path)?;
        let mut nb = node_scene.aggregate_bounds();
        if current_parent_flex.is_some() {
            // Flex children never doc-translate during the drag — the
            // accumulated cursor delta is where the user dropped them.
            nb.origin.x += drag.total_dx as f32;
            nb.origin.y += drag.total_dy as f32;
        }
        let center = Point2D::new(nb.origin.x + nb.size.x / 2.0, nb.origin.y + nb.size.y / 2.0);
        let candidate = self.container_drop_candidate(center, nb);
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
    fn apply_drag_commit(&mut self, id: &NodeId, plan: DragCommitPlan) -> bool {
        let Some(target) = plan.target else {
            return false;
        };
        let bounds = plan.dropped_bounds;
        self.editor_state.move_node_to_drop_target(
            id,
            target,
            bounds.origin.x as f64,
            bounds.origin.y as f64,
            bounds.size.x as f64,
            bounds.size.y as f64,
        )
    }

    fn container_drop_candidate(
        &self,
        point: Point2D,
        dragged_bounds: Rect,
    ) -> Option<ContainerDropCandidate> {
        let container =
            deepest_indexed_container_at(&self.canvas_drop_index.as_ref()?.containers, point)?;
        let (index, insertion) = if let Some(dir) = container.flex {
            flex_insert_preview(container, dragged_bounds, dir)
        } else {
            (0, None)
        };
        Some(ContainerDropCandidate {
            parent_id: container.id.clone(),
            bounds: container.bounds,
            flex: container.flex,
            index,
            insertion,
        })
    }

    fn ensure_canvas_drop_index(&mut self, dragged_id: &NodeId) -> Option<()> {
        let key = CanvasDropIndexKey {
            document_generation: self.editor_state.document_generation(),
            document_revision: self.editor_state.document_revision(),
            active_page_index: self.layout_scene.active_page_index,
            dragged_id: dragged_id.as_str().to_string(),
            excluded_ids: self
                .option_drag_source_ids
                .iter()
                .map(|id| id.as_str().to_string())
                .collect(),
        };
        if self
            .canvas_drop_index
            .as_ref()
            .is_some_and(|index| index.key == key)
        {
            return Some(());
        }
        self.canvas_drop_index = build_canvas_drop_index(
            key,
            &self.editor_state,
            &self.layout_scene,
            dragged_id,
            &self.option_drag_source_ids,
        );
        self.canvas_drop_index.as_ref().map(|_| ())
    }
}

fn build_canvas_drop_index(
    key: CanvasDropIndexKey,
    state: &op_editor_core::EditorState,
    scene: &op_editor_ui::layout_scene::LayoutScene,
    dragged_id: &NodeId,
    excluded_ids: &[NodeId],
) -> Option<CanvasDropIndex> {
    #[cfg(test)]
    DROP_INDEX_BUILD_COUNT.with(|count| count.set(count.get() + 1));
    let nodes = state.active_children();
    let source = op_editor_core::walkers::find_node(nodes, dragged_id)?;
    let (current_parent, current_index) =
        op_editor_core::walkers::find_parent_and_index(nodes, dragged_id)?;
    let current_parent_flex = current_parent
        .as_ref()
        .and_then(|parent_id| op_editor_core::walkers::find_node(nodes, parent_id))
        .and_then(auto_layout_direction);
    let mut scene_bounds = HashMap::new();
    collect_scene_bounds(&scene.active_page()?.children, &mut scene_bounds);
    let mut dragged_scene_path = Vec::new();
    if !find_scene_path(
        &scene.active_page()?.children,
        dragged_id.as_str(),
        &mut dragged_scene_path,
    ) {
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

fn find_scene_path(
    nodes: &[op_editor_ui::layout_scene::SceneNode],
    id: &str,
    path: &mut Vec<usize>,
) -> bool {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        if node.id == id || find_scene_path(&node.children, id, path) {
            return true;
        }
        path.pop();
    }
    false
}

fn scene_node_at_path<'a>(
    nodes: &'a [op_editor_ui::layout_scene::SceneNode],
    path: &[usize],
) -> Option<&'a op_editor_ui::layout_scene::SceneNode> {
    let (&index, rest) = path.split_first()?;
    let node = nodes.get(index)?;
    if rest.is_empty() {
        Some(node)
    } else {
        scene_node_at_path(&node.children, rest)
    }
}

fn collect_scene_bounds(
    nodes: &[op_editor_ui::layout_scene::SceneNode],
    out: &mut HashMap<String, SceneBounds>,
) {
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

fn deepest_indexed_container_at(
    containers: &[IndexedContainer],
    point: Point2D,
) -> Option<&IndexedContainer> {
    let mut hit = None;
    for container in containers {
        if !rect_contains(container.bounds, point) {
            continue;
        }
        hit = Some(container);
        if let Some(deeper) = deepest_indexed_container_at(&container.children, point) {
            hit = Some(deeper);
        }
    }
    hit
}

fn flex_insert_preview(
    parent: &IndexedContainer,
    dragged_bounds: Rect,
    dir: FlexDirection,
) -> (usize, Option<CanvasOverlayLine>) {
    let parent_bounds = parent.bounds;
    let vertical = matches!(dir, FlexDirection::Vertical);
    let drag_mid = if vertical {
        dragged_bounds.origin.y + dragged_bounds.size.y / 2.0
    } else {
        dragged_bounds.origin.x + dragged_bounds.size.x / 2.0
    };
    let mut index = parent.flex_children.len();
    for (i, bounds) in parent.flex_children.iter().copied().enumerate() {
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
    let insertion = flex_insertion_line(parent_bounds, &parent.flex_children, index, vertical);
    (index, insertion)
}

fn flex_insertion_line(
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
