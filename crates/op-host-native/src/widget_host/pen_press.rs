//! Pen-tool authoring + bezier path-editing dispatch.
//!
//! Carved out of `press.rs` / `input.rs` / `keyboard.rs` (all over
//! the 800-line cap) — those files keep 1-3-line hooks into here.
//!
//! TS sources ported verbatim:
//! - `apps/web/src/canvas/skia/skia-pen-tool.ts` — press-drag mints
//!   mirrored handles, click-on-first-anchor closes (≥ 3 anchors,
//!   8 px / zoom), Backspace pops the last anchor (lone anchor →
//!   cancel), double-click finishes, Escape cancels.
//! - `apps/web/src/canvas/skia/skia-interaction.ts:277-306` — Select
//!   tool hits path controls FIRST (before arc / resize / rotation);
//!   `:1356-1378` — Select-tool right-click on a path control opens
//!   the anchor menu, a miss closes it.
//! - `apps/web/src/canvas/skia/skia-canvas.tsx:38-71` — the menu's
//!   four actions route through `setPathPointType` /
//!   `resetPathPointHandles` and commit one history entry.
//!
//! Known cosmetic divergence: TS flips the hover cursor to `pointer`
//! over a path control (`skia-interaction.ts:1004-1009`); the native
//! `CursorHint` enum has no pointer variant, so hovering an anchor
//! keeps the Move/Default cursor. Press routing is unaffected.
//!
//! Scope note: neither stack supports adding / removing anchors on a
//! COMMITTED path — TS's only anchor-count edits are Backspace during
//! authoring and the pen tool itself (`skia-pen-tool.ts`); the anchor
//! context menu edits point types only. Rust matches deliberately.

use super::WidgetHostNative;
use op_editor_ui::widgets::path_anchor_context_menu::{
    MenuHit, PathAnchorContextMenu, PathAnchorMenuAction,
};
use op_editor_ui::Point2D;

/// TS `dblclick` stand-in: two presses within 400 ms and 4 screen px
/// finish the in-progress path (the browser event fires after the
/// second mousedown; here the second press is detected directly and
/// the anchor it would have added is skipped — TS pops that anchor in
/// `onDblClick`, so the net anchor set is identical).
const PEN_DOUBLE_CLICK_MS: u64 = 400;
const PEN_DOUBLE_CLICK_PX: f32 = 4.0;

impl WidgetHostNative {
    /// Canvas press with the Pen tool active. Order mirrors TS:
    /// anchor-edit hit first (idle pen only), then close-path /
    /// double-click-finish / add-anchor for an in-flight session.
    pub(in crate::widget_host) fn apply_pen_tool_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if self.editor_state.ui.pen_in_progress.is_none()
            && self.try_path_anchor_press(x, y, viewport_w, viewport_h)
        {
            return true;
        }
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc_point = self.editor_state.viewport.to_document(canvas_local);
        let doc = (doc_point.x as f64, doc_point.y as f64);
        if self.editor_state.ui.pen_in_progress.is_some() {
            // 1. Click near the FIRST anchor (≥ 3 anchors) closes the
            //    path (TS skia-pen-tool.ts:71-79). Checked before the
            //    double-click so the second press of a double-click
            //    near the start still closes (TS event order).
            let zoom = self.editor_state.viewport.zoom;
            if self.editor_state.pen_close_hit(doc, zoom) {
                let _ = self.editor_state.finish_pen_path_with(true);
                self.mark_dirty();
                return true;
            }
            // 2. Double-click finishes the path open.
            let is_double = matches!(
                self.editor_state.ui.pen_last_press,
                Some((t, px, py)) if self.now_ms.saturating_sub(t) < PEN_DOUBLE_CLICK_MS
                    && (x - px).abs() < PEN_DOUBLE_CLICK_PX
                    && (y - py).abs() < PEN_DOUBLE_CLICK_PX
            );
            if is_double {
                let _ = self.editor_state.finish_pen_path_with(false);
                self.mark_dirty();
                return true;
            }
            // 3. Plain press appends an anchor (+ arms the handle drag).
            self.editor_state.add_pen_point(doc);
        } else {
            let _ = self
                .editor_state
                .start_pen_path(&mut self.next_node_id, doc);
        }
        self.editor_state.ui.pen_last_press = Some((self.now_ms, x, y));
        self.mark_dirty();
        true
    }

    /// Press on a path anchor / bezier handle of the selected Path —
    /// starts the anchor drag. Shared by the Pen tool (idle session)
    /// and the Select tool (TS edits path controls with Select,
    /// `skia-interaction.ts:277-306`, BEFORE arc / resize / rotation).
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
        // Un-rotate the press cursor into the node's local frame —
        // anchors / handles are stored unrotated, and the move handler
        // computes its cumulative delta in the same frame.
        let start_doc = match scene_node.filter(|n| n.rotation.abs() > f32::EPSILON) {
            Some(n) => {
                let b = n.aggregate_bounds();
                let centre = Point2D::new(b.origin.x + b.size.x / 2.0, b.origin.y + b.size.y / 2.0);
                op_editor_ui::widgets::rotate_point(doc_point, centre, -n.rotation)
            }
            None => doc_point,
        };
        // The anchor's fixed absolute position — handle drags offset
        // their delta against it.
        let anchor_doc = scene_node
            .and_then(|n| {
                n.path_anchors
                    .get(anchor_index)
                    .map(|a| a.pos)
                    .or_else(|| n.points.get(anchor_index).copied())
            })
            .unwrap_or(start_doc);
        // An EXISTING handle records its press offset (TS
        // `movePathControl` accumulates deltas from it); an unset
        // (ghost) handle leaves `None` → mint path.
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

    /// Cursor move while a path-anchor / handle drag is in flight —
    /// TS `handlePathControlMove` (`skia-interaction.ts:875-944`) via
    /// `movePathControl` (`path-editing.ts:66-114`): the cumulative
    /// cursor delta since press is applied to the pressed control, so
    /// the grab offset is preserved (no snap-to-cursor). True while a
    /// drag is active (the move is consumed either way).
    pub(in crate::widget_host) fn apply_path_anchor_drag_move(&mut self, x: f32, y: f32) -> bool {
        use super::AnchorDragTarget;
        if self.path_anchor_drag.is_none() {
            return false;
        }
        let (cx0, cy0) = self.canvas_origin();
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
        // Un-rotate the cursor into the path's local frame (anchor /
        // handle coords are stored unrotated).
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
        // Writes start after the first real motion so a press-release
        // in place pushes no history.
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
                // Existing handle — TS movePathControl semantics
                // (resolved point type decides mirroring).
                (AnchorDragTarget::Handle(side), Some(grab)) => {
                    self.editor_state.move_path_anchor_handle_ts(
                        &id,
                        idx,
                        side,
                        (grab.x as f64 + delta.0, grab.y as f64 + delta.1),
                    );
                }
                // Ghost mint (Pen tool, Rust superset): the first move
                // types the anchor — Mirrored, or Independent with
                // Shift — then the handle tracks the cursor.
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

    /// Cursor move while a pen session is in flight. `Some(consumed)`
    /// when the pen owns the move: a held press drags mirrored
    /// handles onto the just-placed anchor (TS `onMouseMove`,
    /// `skia-pen-tool.ts:94-114`); either way the rubber-band cursor
    /// updates.
    pub(in crate::widget_host) fn apply_pen_cursor_move(&mut self, x: f32, y: f32) -> Option<bool> {
        self.editor_state.ui.pen_in_progress.as_ref()?;
        let (cx0, cy0) = self.canvas_origin();
        let canvas_local = Point2D::new(x - cx0, y - cy0);
        let doc = self.editor_state.viewport.to_document(canvas_local);
        if self.editor_state.ui.pen_dragging_handle {
            let _ = self
                .editor_state
                .pen_drag_handle_to((doc.x as f64, doc.y as f64));
        }
        self.editor_state.ui.pen_cursor_doc = Some(doc);
        self.mark_dirty();
        Some(true)
    }

    /// Pointer release while a pen session is in flight — ends the
    /// handle drag (TS `onMouseUp` consumes whenever the pen is
    /// active).
    pub(in crate::widget_host) fn apply_pen_release(&mut self) -> bool {
        if self.editor_state.ui.pen_in_progress.is_none() {
            return false;
        }
        self.editor_state.pen_release();
        true
    }

    /// Enter while authoring — commits the in-flight pen path as an
    /// open path (TS `onKeyDown('Enter')` → `finalizePen(false)`).
    /// `None` when no session is active.
    pub(in crate::widget_host) fn apply_pen_enter(&mut self) -> Option<bool> {
        self.editor_state.ui.pen_in_progress.as_ref()?;
        let ok = self.editor_state.finish_pen_path();
        if ok {
            self.mark_dirty();
        }
        Some(ok)
    }

    /// Backspace while authoring — pops the last anchor; a lone
    /// anchor cancels the whole path and returns to the Select tool
    /// (TS `cancel()`).
    pub(in crate::widget_host) fn apply_pen_backspace(&mut self) -> bool {
        if self.editor_state.ui.pen_in_progress.is_none() {
            return false;
        }
        let handled = self.editor_state.pen_backspace();
        if handled {
            if self.editor_state.ui.pen_in_progress.is_none() {
                self.editor_state.tool = op_editor_core::Tool::Select;
            }
            self.mark_dirty();
        }
        handled
    }

    /// Escape — closes the path-anchor menu first, then cancels an
    /// in-flight pen session (TS Escape CANCELS — the path is
    /// discarded, not committed — and lands on the Select tool).
    pub(in crate::widget_host) fn apply_pen_escape(&mut self) -> bool {
        if self.editor_state.ui.path_anchor_menu.take().is_some() {
            self.mark_dirty();
            return true;
        }
        if self.editor_state.cancel_pen_path() {
            self.editor_state.tool = op_editor_core::Tool::Select;
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Tool switch while authoring — TS `onToolChange` DISCARDS the
    /// in-progress path (it was never committed) unless the new tool
    /// is the Pen itself.
    pub(in crate::widget_host) fn cancel_pen_on_tool_switch(
        &mut self,
        new_tool: op_editor_core::Tool,
    ) {
        if !matches!(new_tool, op_editor_core::Tool::Pen) {
            let _ = self.editor_state.cancel_pen_path();
        }
    }

    /// Right-press over the canvas with the Select tool — a hit on a
    /// path control selects the path + opens the anchor menu; a miss
    /// closes any open menu (TS `onContextMenu`).
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
        // The TS contextmenu listener is canvas-element-scoped — a
        // right-press over chrome never opens the anchor menu.
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

    /// Press routing while the path-anchor menu is open. A row hit
    /// dispatches the action + commits ONE history entry (TS
    /// `updateNode` via `mutateWithHistory`); a miss closes the menu
    /// WITHOUT consuming — TS closes via a document-level listener
    /// and the canvas press still routes (unlike the layer context
    /// menu, which swallows its dismissing click).
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
            // TS re-selects the edited path after the action.
            self.editor_state.set_single_selection(id);
        }
        self.editor_state.ui.path_anchor_menu = None;
        self.mark_dirty();
        true
    }

    /// Cursor-move row hover for the open path-anchor menu. True when
    /// the hover row changed (repaint).
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
