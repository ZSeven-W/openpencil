//! Select-tool path-anchor context-menu dispatch for the web host.

use super::WidgetHost;
use op_editor_ui::widgets::path_anchor_context_menu::{
    MenuHit, PathAnchorContextMenu, PathAnchorMenuAction,
};
use op_editor_ui::Point2D;

impl WidgetHost {
    pub(in crate::widget_host) fn try_path_anchor_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        let Some((node_id, anchor_index, target)) =
            self.path_anchor_hit(x, y, viewport_w, viewport_h)
        else {
            return false;
        };
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc_point = self.editor_state.viewport.to_document(canvas_local);
        let ec_id = op_editor_core::NodeId::new(&node_id);
        let scene_node = self
            .layout_scene
            .active_page()
            .and_then(|p| p.find(&node_id));
        let start_doc = match scene_node.filter(|n| n.rotation.abs() > f32::EPSILON) {
            Some(n) => {
                let b = n.aggregate_bounds();
                let centre = Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
                op_editor_ui::widgets::rotate_point(doc_point, centre, -n.rotation)
            }
            None => doc_point,
        };
        let anchor_doc = scene_node
            .and_then(|n| {
                n.path_anchors
                    .get(anchor_index)
                    .map(|a| a.pos)
                    .or_else(|| n.points.get(anchor_index).copied())
            })
            .unwrap_or(start_doc);
        let grab_offset = match target {
            super::AnchorDragTarget::Handle(side) => scene_node
                .and_then(|n| n.path_anchors.get(anchor_index))
                .and_then(|a| match side {
                    op_editor_core::pen::PathHandleSide::In => a.handle_in,
                    op_editor_core::pen::PathHandleSide::Out => a.handle_out,
                })
                .map(|abs| Point2D::new(abs.x - anchor_doc.x, abs.y - anchor_doc.y)),
            super::AnchorDragTarget::Anchor => None,
        };
        let pre = self.editor_state.snapshot_for_history();
        self.path_anchor_drag = Some(super::PathAnchorDragState {
            node_id: ec_id,
            anchor_index,
            target,
            anchor_doc,
            start_doc,
            grab_offset,
            shift: self.shift_held,
            moved: false,
            pre_drag_snapshot: pre,
        });
        true
    }

    pub(in crate::widget_host) fn apply_path_anchor_drag_move(&mut self, x: f32, y: f32) -> bool {
        use super::AnchorDragTarget;
        if self.path_anchor_drag.is_none() {
            return false;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(self.last_viewport_w, self.last_viewport_h);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc = self.editor_state.viewport.to_document(canvas_local);
        let (id, idx, target, anchor_doc, start, grab, shift, already_moved) = {
            let d = self.path_anchor_drag.as_ref().unwrap();
            (
                d.node_id.clone(),
                d.anchor_index,
                d.target,
                d.anchor_doc,
                d.start_doc,
                d.grab_offset,
                d.shift,
                d.moved,
            )
        };
        self.refresh_layout_scene();
        let local = match self
            .layout_scene
            .active_page()
            .and_then(|p| p.find(id.as_str()))
            .filter(|n| n.rotation.abs() > f32::EPSILON)
            .map(|n| (n.rotation, n.aggregate_bounds()))
        {
            Some((rot, b)) => {
                let c = Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
                op_editor_ui::widgets::rotate_point(doc, c, -rot)
            }
            None => doc,
        };
        let delta = ((local.x - start.x) as f64, (local.y - start.y) as f64);
        let is_move = delta.0.abs() > 0.001 || delta.1.abs() > 0.001;
        if is_move || already_moved {
            match (target, grab) {
                (AnchorDragTarget::Anchor, _) => {
                    self.editor_state.set_path_anchor_position(
                        id,
                        idx,
                        (anchor_doc.x as f64 + delta.0, anchor_doc.y as f64 + delta.1),
                    );
                }
                (AnchorDragTarget::Handle(side), Some(grab)) => {
                    self.editor_state.move_path_anchor_handle_ts(
                        &id,
                        idx,
                        side,
                        (grab.x as f64 + delta.0, grab.y as f64 + delta.1),
                    );
                }
                (AnchorDragTarget::Handle(side), None) => {
                    if !already_moved {
                        let pt = if shift {
                            jian_ops_schema::node::PenPathPointType::Independent
                        } else {
                            jian_ops_schema::node::PenPathPointType::Mirrored
                        };
                        self.editor_state
                            .set_path_anchor_point_type(id.clone(), idx, pt);
                    }
                    let offset = (
                        (local.x - anchor_doc.x) as f64,
                        (local.y - anchor_doc.y) as f64,
                    );
                    self.editor_state
                        .set_path_anchor_handle(id, idx, side, Some(offset));
                }
            }
            self.mark_dirty();
            if let Some(d) = self.path_anchor_drag.as_mut() {
                d.moved = true;
            }
        }
        true
    }

    pub(in crate::widget_host) fn try_open_path_anchor_menu(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if !matches!(self.editor_state.tool, op_editor_core::Tool::Select) {
            return false;
        }
        if !self.over_canvas(x, y, viewport_w, viewport_h) {
            return false;
        }
        self.refresh_layout_scene();
        if let Some((node_id, anchor_index, _)) = self.path_anchor_hit(x, y, viewport_w, viewport_h)
        {
            let ec_id = op_editor_core::NodeId::new(&node_id);
            self.editor_state.set_single_selection(ec_id.clone());
            self.editor_state.ui.path_anchor_menu = Some(op_editor_core::PathAnchorMenuState {
                node_id: ec_id,
                anchor_index,
                x,
                y,
                menu: Default::default(),
            });
            self.mark_dirty();
            return true;
        }
        if self.editor_state.ui.path_anchor_menu.take().is_some() {
            self.mark_dirty();
        }
        false
    }

    pub(in crate::widget_host) fn dispatch_path_anchor_menu_press(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(state) = self.editor_state.ui.path_anchor_menu.clone() else {
            return false;
        };
        let menu = PathAnchorContextMenu::for_state(&self.editor_state, state.clone());
        let action = match menu.hit(Point2D::new(x, y)) {
            MenuHit::Row(_) => menu.hit_test(Point2D::new(x, y)),
            MenuHit::Inside => return true,
            MenuHit::Outside => {
                self.editor_state.ui.path_anchor_menu = None;
                self.mark_dirty();
                return false;
            }
        };
        let Some(action) = action else {
            return true;
        };
        use jian_ops_schema::node::PenPathPointType as P;
        let id = state.node_id.clone();
        let idx = state.anchor_index;
        let snap = self.editor_state.snapshot_for_history();
        let changed = match action {
            PathAnchorMenuAction::Corner => {
                self.editor_state
                    .set_path_anchor_point_type_ts(&id, idx, P::Corner)
            }
            PathAnchorMenuAction::Mirrored => {
                self.editor_state
                    .set_path_anchor_point_type_ts(&id, idx, P::Mirrored)
            }
            PathAnchorMenuAction::Independent => {
                self.editor_state
                    .set_path_anchor_point_type_ts(&id, idx, P::Independent)
            }
            PathAnchorMenuAction::Reset => self.editor_state.reset_path_anchor_handles(&id, idx),
        };
        if changed {
            self.editor_state.history_push_past(snap);
            self.editor_state.set_single_selection(id);
        }
        self.editor_state.ui.path_anchor_menu = None;
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn update_path_anchor_menu_hover(&mut self, x: f32, y: f32) -> bool {
        let Some(state) = self.editor_state.ui.path_anchor_menu.clone() else {
            return false;
        };
        let menu = PathAnchorContextMenu::for_state(&self.editor_state, state.clone());
        let new_hover = menu.hovered_row_at(Point2D::new(x, y));
        if new_hover != state.menu.hover {
            let mut next = state;
            next.menu.hover = new_hover;
            self.editor_state.ui.path_anchor_menu = Some(next);
            self.mark_dirty();
            return true;
        }
        false
    }
}
