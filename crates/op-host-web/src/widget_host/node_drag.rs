use super::WidgetHost;
use op_editor_core::drag_mutators::{
    auto_layout_direction, parent_of, should_auto_reparent_outside_parent, FlexDirection,
};
use op_editor_core::{NodeId, PenNodeExt};

const NODE_DRAG_THRESHOLD_PX: f32 = 2.0;

struct DragCommitPlan {
    parent_id: NodeId,
    flex: Option<FlexDirection>,
    bounds: (f32, f32, f32, f32),
    reparent_to_root: bool,
    sibling_mids: Vec<f32>,
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
}

impl WidgetHost {
    pub(in crate::widget_host) fn start_node_drag(&mut self, x: f32, y: f32) {
        self.editor_state.commit_history();
        self.node_drag = Some(NodeDragState {
            last_screen_x: x,
            last_screen_y: y,
            press_screen_x: x,
            press_screen_y: y,
            moved: false,
            total_dx: 0.0,
            total_dy: 0.0,
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
        if !drag.moved {
            if let Some(d) = self.node_drag.as_mut() {
                d.moved = true;
            }
        }

        let zoom = self.editor_state.viewport.zoom.max(0.0001);
        if let Some(d) = self.node_drag.as_mut() {
            d.total_dx = ((x - d.press_screen_x) / zoom) as f64;
            d.total_dy = ((y - d.press_screen_y) / zoom) as f64;
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
            } else {
                self.mark_dirty();
            }
        } else {
            self.editor_state.editor_ui.active_guides.clear();
        }
        Some(true)
    }

    pub(in crate::widget_host) fn release_node_drag(&mut self) -> bool {
        let Some(drag) = self.node_drag.take() else {
            return false;
        };
        self.editor_state.editor_ui.active_guides.clear();
        let _ = self.commit_node_drag(&drag);
        self.mark_dirty();
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

    fn plan_drag_commit(&self, id: &NodeId, drag: &NodeDragState) -> Option<DragCommitPlan> {
        let children = self.editor_state.active_children();
        let parent_id = parent_of(children, id)?;
        let parent = op_editor_core::walkers::find_node(children, &parent_id)?;
        let flex = auto_layout_direction(parent);
        let page = self.layout_scene.active_page()?;
        let node_scene = page.find(id.as_str())?;
        let mut nb = node_scene.aggregate_bounds();
        if flex.is_some() {
            nb.origin.x += drag.total_dx as f32;
            nb.origin.y += drag.total_dy as f32;
        }
        let pb = page.find(parent_id.as_str())?.aggregate_bounds();
        let outside = nb.origin.x + nb.size.x <= pb.origin.x
            || nb.origin.x >= pb.origin.x + pb.size.x
            || nb.origin.y + nb.size.y <= pb.origin.y
            || nb.origin.y >= pb.origin.y + pb.size.y;
        let node_ref = op_editor_core::walkers::find_node(children, id)?;
        let reparent_to_root = outside && should_auto_reparent_outside_parent(node_ref);
        let vertical = matches!(flex, Some(FlexDirection::Vertical));
        let sibling_mids = parent
            .children()
            .map(|siblings| {
                siblings
                    .iter()
                    .filter(|sib| sib.id_str() != id.as_str())
                    .map(|sib| {
                        page.find(sib.id_str())
                            .map(|sn| {
                                let b = sn.aggregate_bounds();
                                if vertical {
                                    b.origin.y + b.size.y / 2.0
                                } else {
                                    b.origin.x + b.size.x / 2.0
                                }
                            })
                            .unwrap_or(0.0)
                    })
                    .collect()
            })
            .unwrap_or_default();
        Some(DragCommitPlan {
            parent_id,
            flex,
            bounds: (nb.origin.x, nb.origin.y, nb.size.x, nb.size.y),
            reparent_to_root,
            sibling_mids,
        })
    }

    fn apply_drag_commit(&mut self, id: &NodeId, plan: DragCommitPlan) -> bool {
        let (bx, by, bw, bh) = plan.bounds;
        if plan.reparent_to_root {
            return self
                .editor_state
                .reparent_to_page_root(id, bx as f64, by as f64);
        }
        let Some(dir) = plan.flex else {
            return false;
        };
        let drag_mid = match dir {
            FlexDirection::Vertical => by + bh / 2.0,
            FlexDirection::Horizontal => bx + bw / 2.0,
        };
        let mut new_index = plan.sibling_mids.len();
        for (i, sib_mid) in plan.sibling_mids.iter().enumerate() {
            if drag_mid < *sib_mid {
                new_index = i;
                break;
            }
        }
        self.editor_state
            .reorder_child_to_index(&plan.parent_id, id, new_index)
    }
}
