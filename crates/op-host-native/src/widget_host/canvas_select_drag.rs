//! Canvas selection semantics (enter-group on double-click) + the
//! node-drag release commit (auto-layout reorder / reparent-to-root).
//!
//! TS sources: `skia-interaction.ts:1182-1256` (`handleDragEnd`),
//! `:1262-1294` (double-click enter-group) and
//! `drag-reparent-policy.ts`. The selection-resolution rules live in
//! `op_editor_core::selection_resolve`; this module is the host glue
//! that reads the layout scene for absolute bounds.

use super::{NodeDragState, WidgetHostNative};
use jian_ops_schema::node::PenNode;
use op_editor_core::drag_mutators::{
    auto_layout_direction, parent_of, should_auto_reparent_outside_parent, FlexDirection,
};
use op_editor_core::{NodeId, PenNodeExt};

/// Read-phase summary of one dragged node — collected before any
/// mutation so no document / scene borrow survives into the mutators.
struct DragCommitPlan {
    parent_id: NodeId,
    /// `Some(direction)` when the parent is an auto-layout container.
    flex: Option<FlexDirection>,
    /// Dropped absolute bounds `(x, y, w, h)` of the dragged node.
    bounds: (f32, f32, f32, f32),
    /// Whether the node sits fully outside its parent's bounds AND
    /// the reparent policy allows detaching it.
    reparent_to_root: bool,
    /// Sibling ids (dragged node excluded) in document order, with
    /// each sibling's main-axis midpoint from the layout scene.
    sibling_mids: Vec<f32>,
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
            if self.try_enter_selected_container_on_double_click(&ec_id) {
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
        };
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
        self.mark_dirty();
        true
    }

    /// Node-drag release commit (TS `handleDragEnd`): a node dropped
    /// fully outside its parent reparents to the page root preserving
    /// its visual position (content primitives only — see
    /// `drag-reparent-policy.ts`); a child dropped within an
    /// auto-layout parent re-inserts at the midpoint-derived sibling
    /// index. Free-layout children were already translated live.
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
        // Top-level nodes have no reorder / reparent policy to run.
        let parent_id = parent_of(children, id)?;
        let parent = op_editor_core::walkers::find_node(children, &parent_id)?;
        let flex = auto_layout_direction(parent);
        let page = self.layout_scene.active_page()?;
        let node_scene = page.find(id.as_str())?;
        let mut nb = node_scene.aggregate_bounds();
        if flex.is_some() {
            // Flex children never doc-translate during the drag — the
            // accumulated cursor delta is where the user dropped them.
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
        // Sibling main-axis midpoints in document order (dragged node
        // excluded) — TS treats a missing render node as midpoint 0.
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

    /// Mutation phase — apply the planned reparent / reorder.
    fn apply_drag_commit(&mut self, id: &NodeId, plan: DragCommitPlan) -> bool {
        let (bx, by, bw, bh) = plan.bounds;
        if plan.reparent_to_root {
            return self
                .editor_state
                .reparent_to_page_root(id, bx as f64, by as f64);
        }
        let Some(dir) = plan.flex else {
            // Free-layout child inside its parent: the live translate
            // already committed the move.
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
