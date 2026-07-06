//! Canvas selection semantics (enter-group on double-click) + the
//! node-drag release commit (auto-layout reorder / cross-container
//! reparenting).
//!
//! TS sources: `skia-interaction.ts:1182-1256` (`handleDragEnd`),
//! `:1262-1294` (double-click enter-group) and
//! `op_editor_core::selection_resolve`; this module is the host glue
//! that reads the layout scene for absolute bounds.

use super::{NodeDragState, WidgetHostNative};
use jian_ops_schema::node::PenNode;
use op_editor_core::drag_mutators::{
    auto_layout_direction, parent_of, DragDropTarget, FlexDirection,
};
use op_editor_core::editor_ui_state::{CanvasDropIndicator, CanvasOverlayLine, CanvasOverlayRect};
use op_editor_core::{NodeId, PenNodeExt};
use op_editor_ui::widgets::CanvasNodeDragOverlay;
use op_editor_ui::{Point2D, Rect};

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

impl WidgetHostNative {
    /// Select-tool press on a canvas node (the deepest hit). Routes
    /// double-clicks to enter-group / text-edit, then applies the TS
    /// click-resolution rules (`skia-interaction.ts:368-405`): promote
    /// a nested hit to its outermost frame/group ancestor (unit
    /// selection) unless the user entered that container; a hit on a
    /// child of an already-selected node keeps the selection. Always
    /// consumes the press.
    pub(in crate::widget_host) fn apply_canvas_node_press(
        &mut self,
        ec_id: NodeId,
        x: f32,
        y: f32,
        text_edit_was_active: bool,
        viewport_height: f32,
    ) -> bool {
        use op_editor_core::selection_resolve::{
            resolve_canvas_selection_target, SelectionResolution,
        };
        // Canvas double-click: 400 ms same-node → enter a selected
        // container, else text-edit on Text nodes.
        let is_double = matches!(
            &self.editor_state.editor_ui.last_canvas_click,
            Some((prev, t)) if *prev == ec_id
                && self.now_ms.saturating_sub(*t) < 400
        );
        self.editor_state.editor_ui.last_canvas_click = Some((ec_id.clone(), self.now_ms));
        if is_double && !text_edit_was_active {
            // TS dblclick order (skia-interaction.ts:1279-1296):
            // entering a selected frame/group wins over the
            // text-edit fallback.
            if self.try_enter_selected_container_on_double_click(&ec_id, viewport_height) {
                return true;
            }
            if self.editor_state.start_text_edit(ec_id.clone()) {
                self.editor_state.ui.text_edit_input.touch(self.now_ms);
                self.mark_dirty();
                return true;
            }
        }
        let resolved = resolve_canvas_selection_target(
            self.editor_state.active_children(),
            &ec_id,
            self.editor_state.editor_ui.entered_container.as_ref(),
            &self.editor_state.selection.set,
        );
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
            // Shift+click toggles set membership of the resolved
            // target; a child of a selected node keeps the set and
            // just drags it.
            if let SelectionResolution::Select(target) = resolved {
                let was_in_set = self.editor_state.is_selected(&target);
                self.editor_state.toggle_selection(target);
                self.editor_state.sync_entered_container_with_selection();
                if !was_in_set {
                    self.node_drag = Some(fresh_drag);
                }
            } else {
                self.node_drag = Some(fresh_drag);
            }
            self.scroll_layer_panel_selection_into_view(viewport_height);
            return true;
        }
        // Plain click: keep a multi-set when clicking inside it (TS
        // parity), else single-select the resolved target.
        if let SelectionResolution::Select(target) = resolved {
            let already_in_set = self.editor_state.is_selected(&target);
            if !already_in_set || self.editor_state.selection_count() == 1 {
                self.editor_state.set_single_selection(target);
            }
        }
        self.editor_state.sync_entered_container_with_selection();
        self.scroll_layer_panel_selection_into_view(viewport_height);
        self.editor_state.commit_history();
        self.node_drag = Some(fresh_drag);
        true
    }

    /// TS double-click enter-group (`skia-interaction.ts:1279-1294`):
    /// when exactly one frame/group with children is selected, a
    /// double-click selects the deepest hit under the cursor; when
    /// that hit is inside the selected container, the container
    /// becomes the entered context (promotion then stops at its
    /// children). Returns `true` when the double-click was consumed.
    pub(in crate::widget_host) fn try_enter_selected_container_on_double_click(
        &mut self,
        deepest: &NodeId,
        viewport_height: f32,
    ) -> bool {
        if self.editor_state.selection_count() != 1 {
            return false;
        }
        let anchor = self.editor_state.selection.anchor.clone();
        if !anchor.is_real() || anchor == *deepest {
            return false;
        }
        let children = self.editor_state.active_children();
        let Some(node) = op_editor_core::walkers::find_node(children, &anchor) else {
            return false;
        };
        if !matches!(node, PenNode::Frame(_) | PenNode::Group(_)) {
            return false;
        }
        if node.children().map(|c| c.is_empty()).unwrap_or(true) {
            return false;
        }
        let enters =
            op_editor_core::selection_resolve::is_strict_descendant(children, &anchor, deepest);
        if enters {
            self.editor_state.editor_ui.entered_container = Some(anchor);
        }
        self.editor_state.set_single_selection(deepest.clone());
        self.editor_state.sync_entered_container_with_selection();
        self.scroll_layer_panel_selection_into_view(viewport_height);
        self.mark_dirty();
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
        let before_scene = self.layout_scene.clone();
        let current_parent = parent_of(self.editor_state.active_children(), &id);
        let bounds = plan.dropped_bounds;
        let mut indicator = None;
        let mut mutated = false;
        let mut overlay_bounds = None;
        if let Some(target) = plan.target.clone() {
            match &target {
                DragDropTarget::Container { parent_id, .. }
                    if current_parent.as_ref() == Some(parent_id) =>
                {
                    overlay_bounds = Some(bounds);
                    mutated |= self.apply_drag_commit(&id, plan);
                }
                DragDropTarget::PageRoot { .. } if current_parent.is_some() => {
                    mutated |= self.apply_drag_commit(&id, plan);
                }
                DragDropTarget::Container { .. } if current_parent.is_some() => {
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
            self.mark_dirty();
            self.start_layout_transition_from_scene_excluding(before_scene, &id);
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
            self.mark_dirty();
        }
        mutated
    }

    /// Read phase — gather everything the commit needs as owned data.
    fn plan_drag_commit(&self, id: &NodeId, drag: &NodeDragState) -> Option<DragCommitPlan> {
        let children = self.editor_state.active_children();
        let current_parent = parent_of(children, id);
        let current_parent_flex = current_parent
            .as_ref()
            .and_then(|parent_id| op_editor_core::walkers::find_node(children, parent_id))
            .and_then(auto_layout_direction);
        let page = self.layout_scene.active_page()?;
        let node_scene = page.find(id.as_str())?;
        let mut nb = node_scene.aggregate_bounds();
        if current_parent_flex.is_some() {
            // Flex children never doc-translate during the drag — the
            // accumulated cursor delta is where the user dropped them.
            nb.origin.x += drag.total_dx as f32;
            nb.origin.y += drag.total_dy as f32;
        }
        let center = Point2D::new(nb.origin.x + nb.size.x / 2.0, nb.origin.y + nb.size.y / 2.0);
        let candidate = self.container_drop_candidate(id, center, nb);
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
        dragged_id: &NodeId,
        point: Point2D,
        dragged_bounds: Rect,
    ) -> Option<ContainerDropCandidate> {
        let children = self.editor_state.active_children();
        let source = op_editor_core::walkers::find_node(children, dragged_id)?;
        let page = self.layout_scene.active_page()?;
        deepest_container_at(children, source, point, page, &self.option_drag_source_ids).map(
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
}

fn deepest_container_at(
    nodes: &[PenNode],
    source: &PenNode,
    point: Point2D,
    page: &op_editor_ui::layout_scene::ScenePage,
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
    page: &op_editor_ui::layout_scene::ScenePage,
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
    page: &op_editor_ui::layout_scene::ScenePage,
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
