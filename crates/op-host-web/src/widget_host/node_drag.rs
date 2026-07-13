use super::WidgetHost;
use jian_ops_schema::node::PenNode;
use op_editor_core::drag_mutators::{
    auto_layout_direction, parent_of, DragDropTarget, FlexDirection,
};
use op_editor_core::editor_ui_state::{CanvasDropIndicator, CanvasOverlayLine, CanvasOverlayRect};
use op_editor_core::{NodeId, PenNodeExt};
use op_editor_ui::widgets::CanvasNodeDragOverlay;
use op_editor_ui::{Point2D, Rect};

const NODE_DRAG_THRESHOLD_PX: f32 = 2.0;

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

#[derive(Debug, Clone, Copy)]
pub(in crate::widget_host) struct NodeDragState {
    pub(in crate::widget_host) last_screen_x: f32,
    pub(in crate::widget_host) last_screen_y: f32,
    pub(in crate::widget_host) press_screen_x: f32,
    pub(in crate::widget_host) press_screen_y: f32,
    pub(in crate::widget_host) moved: bool,
    pub(in crate::widget_host) total_dx: f64,
    pub(in crate::widget_host) total_dy: f64,
    pub(in crate::widget_host) overlay_bounds: Option<Rect>,
}

impl WidgetHost {
    /// Resolve a canvas press from the rendered root-to-deepest hit
    /// path. Plain click selects the solid-outline primary; a second
    /// press inside 400 ms drills exactly one level to the direct child
    /// under the pointer and enters the primary as the sibling scope.
    pub(in crate::widget_host) fn apply_canvas_node_press(
        &mut self,
        hit_path: Vec<NodeId>,
        x: f32,
        y: f32,
        text_edit_was_active: bool,
        viewport_height: f32,
    ) -> bool {
        let Some(deepest) = hit_path.last().cloned() else {
            return false;
        };
        let Some(targets) = op_editor_core::selection_resolve::resolve_canvas_depth_targets(
            &hit_path,
            self.editor_state.editor_ui.entered_container.as_ref(),
        ) else {
            return false;
        };
        let is_double = matches!(
            &self.editor_state.editor_ui.last_canvas_click,
            Some((prev, t)) if *prev == deepest
                && self.now_ms.saturating_sub(*t) < 400
        ) && !self.shift_held
            && self.editor_state.selection_count() <= 1;
        self.editor_state.editor_ui.last_canvas_click = if self.shift_held || is_double {
            None
        } else {
            Some((deepest, self.now_ms))
        };
        if is_double && !text_edit_was_active {
            if let Some(secondary) = targets.secondary_under_pointer {
                self.editor_state.set_single_selection(secondary.clone());
                self.editor_state.editor_ui.entered_container = Some(targets.primary);
                self.editor_state.editor_ui.canvas_hover_node = Some(secondary);
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

        let target = targets.primary;
        let mut should_start_drag = true;
        if self.shift_held {
            let was_in_set = self.editor_state.is_selected(&target);
            self.editor_state.toggle_selection(target);
            should_start_drag = !was_in_set;
        } else {
            let already_in_set = self.editor_state.is_selected(&target);
            if !already_in_set || self.editor_state.selection_count() == 1 {
                self.editor_state.set_single_selection(target);
            }
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
        if should_start_drag {
            self.start_node_drag(x, y);
        }
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn start_node_drag(&mut self, x: f32, y: f32) {
        self.editor_state.commit_history();
        self.option_drag_source_ids.clear();
        self.node_drag = Some(NodeDragState {
            last_screen_x: x,
            last_screen_y: y,
            press_screen_x: x,
            press_screen_y: y,
            moved: false,
            total_dx: 0.0,
            total_dy: 0.0,
            overlay_bounds: None,
        });
    }

    pub(in crate::widget_host) fn apply_node_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> Option<bool> {
        let drag = self.node_drag?;
        if !drag.moved
            && (x - drag.press_screen_x).abs() <= NODE_DRAG_THRESHOLD_PX
            && (y - drag.press_screen_y).abs() <= NODE_DRAG_THRESHOLD_PX
        {
            return Some(false);
        }
        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        let total_dx = ((x - drag.press_screen_x) / zoom) as f64;
        let total_dy = ((y - drag.press_screen_y) / zoom) as f64;
        if !drag.moved {
            self.editor_state.editor_ui.last_canvas_click = None;
            let option_source_ids: Vec<NodeId> = self.editor_state.selection.set.to_vec();
            if self.alt_held
                && !option_source_ids.is_empty()
                && self
                    .editor_state
                    .duplicate_selected(&mut self.next_node_id, 0.0)
                    .is_some()
            {
                self.option_drag_source_ids = option_source_ids;
                let _ = self
                    .editor_state
                    .move_selected_in_layout_direction(total_dx, total_dy);
                self.mark_dirty();
            }
            if let Some(d) = self.node_drag.as_mut() {
                d.moved = true;
            }
        }

        if let Some(d) = self.node_drag.as_mut() {
            d.total_dx = total_dx;
            d.total_dy = total_dy;
        }
        let dx = (x - drag.last_screen_x) / zoom;
        let dy = (y - drag.last_screen_y) / zoom;
        if dx == 0.0 && dy == 0.0 {
            return Some(false);
        }

        if let Some(drag) = self.node_drag.as_mut() {
            drag.last_screen_x = x;
            drag.last_screen_y = y;
        }
        if self.editor_state.translate_selected(dx as f64, dy as f64) {
            // Incremental scene patch instead of a full serde reconversion per
            // moved pixel (mirrors the native host). A plain node drag only
            // moves absolute-positioned nodes, so translate just those scene
            // nodes in place and defer the full rebuild to release. Flex-flow
            // children are positioned by their parent, not absolute coords, so
            // they still need the full pass. Skip the fast path when a
            // reconversion is already pending (`editor_state_dirty`).
            if !self.editor_state_dirty {
                let children = self.editor_state.active_children();
                let ids: Vec<String> = self
                    .editor_state
                    .selection
                    .set
                    .iter()
                    .filter(|id| {
                        // Move exactly what `translate_selected` moved in the
                        // document: editable nodes only (locked / hidden are
                        // skipped there) and not flex-flow children (positioned
                        // by their parent). Otherwise the scene would drift nodes
                        // the doc never moved, then snap back on the release-time
                        // reconversion.
                        self.editor_state.is_editable(id)
                            && !op_editor_core::walkers::is_flow_child_of_flex(children, id)
                    })
                    .map(|id| id.as_str().to_string())
                    .collect();
                let _ = self.layout_scene.translate_nodes(&ids, dx, dy);
                // The scene is now patched away from the last cached build while
                // `scene_cache.last` still holds the pre-drag inputs. Invalidate
                // so a later refresh always rebuilds — otherwise a doc returning
                // to the cached value (undo, or dirty flipping mid-drag) would
                // skip the rebuild and leave this stale patch on screen.
                self.scene_cache.invalidate();
            } else {
                self.mark_dirty();
            }
        } else {
            self.editor_state.editor_ui.active_guides.clear();
        }
        if let Some(drag) = self.node_drag {
            self.apply_live_node_drag_preview(&drag);
        }
        Some(true)
    }

    pub(in crate::widget_host) fn release_node_drag(&mut self) -> bool {
        let Some(drag) = self.node_drag.take() else {
            return false;
        };
        self.refresh_layout_scene();
        let before_scene = self.layout_scene.clone();
        let release_overlay = (self.editor_state.selection_count() == 1)
            .then(|| {
                drag.overlay_bounds
                    .map(|bounds| (self.editor_state.selection.anchor.clone(), bounds))
            })
            .flatten();
        let should_commit_drop = self
            .editor_state
            .editor_ui
            .canvas_drop_indicator
            .as_ref()
            .map(|indicator| indicator.target.is_some())
            .unwrap_or(false)
            || self.editor_state.selection_count() != 1;
        self.editor_state.editor_ui.active_guides.clear();
        self.editor_state.editor_ui.canvas_drop_indicator = None;
        if should_commit_drop {
            let _ = self.commit_node_drag(&drag);
        }
        self.option_drag_source_ids.clear();
        self.mark_dirty();
        if should_commit_drop {
            self.start_layout_transition_from_scene(before_scene);
        } else {
            self.refresh_layout_scene();
            if let Some((node_id, bounds)) = release_overlay {
                self.start_layout_transition_from_bounds(&node_id, bounds);
            }
        }
        true
    }

    fn commit_node_drag(&mut self, drag: &NodeDragState) -> bool {
        if !drag.moved {
            return false;
        }
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

    fn update_node_drag_preview(&mut self, drag: &NodeDragState) {
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

    fn apply_live_node_drag_preview(&mut self, drag: &NodeDragState) {
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
