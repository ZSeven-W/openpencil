//! Multi-anchor Pen-tool mutators. Builds a `PenNode::Path` whose
//! `anchors` are document-space points, recomputing the bounding box
//! after each anchor. History is captured on the first anchor and
//! pushed on commit only when the path has ≥ 2 anchors (a 1-anchor
//! path is invisible — no real change to undo).

use crate::node_id::NodeId;
use crate::pen_node_ext::{make_path, PenNodeExt};
use crate::state::EditorState;
use crate::walkers::{self, find_node, find_node_mut};
use jian_ops_schema::node::PenNode;
use jian_ops_schema::node::PenPathAnchor;

/// Bounding box of a set of anchors: `(x, y, w, h)`.
fn anchor_bbox(anchors: &[PenPathAnchor]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for a in anchors {
        min_x = min_x.min(a.x);
        min_y = min_y.min(a.y);
        max_x = max_x.max(a.x);
        max_y = max_y.max(a.y);
    }
    if min_x.is_finite() {
        (min_x, min_y, max_x - min_x, max_y - min_y)
    } else {
        (0.0, 0.0, 0.0, 0.0)
    }
}

/// Re-fit a Path node's `base.x` / `base.y` / `width` / `height` to
/// its current anchors. No-op on non-Path nodes.
fn refit_path_bounds(node: &mut PenNode) {
    if let PenNode::Path(_) = node {
        let anchors = match node {
            PenNode::Path(p) => p.anchors.clone().unwrap_or_default(),
            _ => unreachable!(),
        };
        let (x, y, w, h) = anchor_bbox(&anchors);
        node.base_mut().x = Some(x);
        node.base_mut().y = Some(y);
        node.set_width_px(w);
        node.set_height_px(h);
    }
}

impl EditorState {
    /// Start a fresh Pen-tool path at `first` (document coords).
    /// Returns the new node's id, or `None` on allocator overflow.
    pub fn start_pen_path(&mut self, next_id: &mut u64, first: (f64, f64)) -> Option<NodeId> {
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let mut taken = self.collect_node_ids();
        let id = walkers::alloc_n_id(next_id, &mut taken)?;
        // Snapshot BEFORE any mutation so a later undo restores the
        // pre-pen document.
        let pre = self.snapshot_for_history();
        let node = make_path(id.clone().into(), "Path", first);
        self.active_children_mut().push(node);
        self.ui.pending_pen_history = Some(pre);
        self.ui.pen_in_progress = Some(id.clone());
        self.set_single_selection(id.clone());
        Some(id)
    }

    /// Append an anchor to the in-progress path, re-fitting bounds.
    pub fn add_pen_point(&mut self, p: (f64, f64)) -> bool {
        let Some(id) = self.ui.pen_in_progress.clone() else {
            return false;
        };
        let Some(node) = find_node_mut(self.active_children_mut(), &id) else {
            return false;
        };
        if let PenNode::Path(path) = node {
            path.anchors
                .get_or_insert_with(Vec::new)
                .push(PenPathAnchor {
                    x: p.0,
                    y: p.1,
                    handle_in: None,
                    handle_out: None,
                    point_type: None,
                });
        } else {
            return false;
        }
        refit_path_bounds(node);
        true
    }

    /// Move a single anchor on an existing Path node. History is the
    /// caller's responsibility. False on a missing / non-Path node,
    /// an out-of-range index, or a locked / hidden node.
    pub fn set_path_anchor_position(
        &mut self,
        node_id: NodeId,
        index: usize,
        pos: (f64, f64),
    ) -> bool {
        if !self.is_editable(&node_id) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &node_id) else {
            return false;
        };
        if let PenNode::Path(path) = node {
            let Some(anchors) = path.anchors.as_mut() else {
                return false;
            };
            if index >= anchors.len() {
                return false;
            }
            anchors[index].x = pos.0;
            anchors[index].y = pos.1;
        } else {
            return false;
        }
        refit_path_bounds(node);
        true
    }

    /// Commit the in-progress Pen path. Pushes the pre-pen snapshot
    /// onto the undo stack only when the path has ≥ 2 anchors —
    /// otherwise the lone-anchor node is stripped without polluting
    /// history. True when a session was active.
    pub fn finish_pen_path(&mut self) -> bool {
        let Some(id) = self.ui.pen_in_progress.take() else {
            self.ui.pending_pen_history = None;
            self.ui.pen_cursor_doc = None;
            return false;
        };
        let pending = self.ui.pending_pen_history.take();
        let anchor_count = find_node(self.active_children(), &id)
            .and_then(|n| match n {
                PenNode::Path(p) => Some(p.anchors.as_ref().map(|a| a.len()).unwrap_or(0)),
                _ => None,
            })
            .unwrap_or(0);
        if anchor_count >= 2 {
            if let Some(snap) = pending {
                self.history_push_past(snap);
            }
        } else {
            // 1-anchor path is invisible — strip it, skip history.
            self.active_children_mut()
                .retain(|n| n.id_str() != id.as_str());
            if self.selection.anchor == id {
                self.clear_selection();
            }
        }
        self.ui.pen_cursor_doc = None;
        true
    }
}
