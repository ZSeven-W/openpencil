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
use jian_ops_schema::node::{PenPathAnchor, PenPathHandle, PenPathPointType};

/// Which bezier control handle of a path anchor is being edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathHandleSide {
    /// The incoming handle (controls the curve arriving at the anchor).
    In,
    /// The outgoing handle (controls the curve leaving the anchor).
    Out,
}

/// Re-fit a Path node's `base.x` / `base.y` / `width` / `height` to
/// its current anchors — handle-aware, via the canonical
/// [`crate::path_bounds`] so the loader's absolutize pass reads the
/// identical native span (scale stays `1.0`). No-op on non-Path
/// nodes.
fn refit_path_bounds(node: &mut PenNode) {
    if let PenNode::Path(p) = node {
        let anchors = p.anchors.clone().unwrap_or_default();
        let closed = p.closed.unwrap_or(false);
        let (x, y, w, h) = crate::path_bounds::path_bounds_from_anchors(&anchors, closed);
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

    /// Set (or clear, with `delta = None`) a bezier control handle on
    /// a path anchor. `delta` is the handle offset relative to the
    /// anchor. When the anchor's `point_type` is `Mirrored`, the
    /// opposite handle is set to the negated offset so the two stay
    /// collinear + equal-length. History is the caller's
    /// responsibility.
    pub fn set_path_anchor_handle(
        &mut self,
        node_id: NodeId,
        index: usize,
        side: PathHandleSide,
        delta: Option<(f64, f64)>,
    ) -> bool {
        if !self.is_editable(&node_id) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &node_id) else {
            return false;
        };
        {
            let PenNode::Path(path) = &mut *node else {
                return false;
            };
            let Some(anchors) = path.anchors.as_mut() else {
                return false;
            };
            let Some(anchor) = anchors.get_mut(index) else {
                return false;
            };
            let handle = delta.map(|(x, y)| PenPathHandle { x, y });
            match side {
                PathHandleSide::In => anchor.handle_in = handle,
                PathHandleSide::Out => anchor.handle_out = handle,
            }
            // Mirrored anchors keep both handles collinear + equal length.
            if anchor.point_type == Some(PenPathPointType::Mirrored) {
                if let Some((x, y)) = delta {
                    let mirror = Some(PenPathHandle { x: -x, y: -y });
                    match side {
                        PathHandleSide::In => anchor.handle_out = mirror,
                        PathHandleSide::Out => anchor.handle_in = mirror,
                    }
                }
            }
        }
        // Handles bow the curve past the endpoints — re-fit the
        // handle-aware bounds so the loader's absolutize scale stays 1.
        refit_path_bounds(node);
        true
    }

    /// Set a path anchor's point type. Switching to `Mirrored` snaps
    /// the two handles collinear (the existing handle defines the
    /// axis; the opposite becomes its negation). History is the
    /// caller's responsibility.
    pub fn set_path_anchor_point_type(
        &mut self,
        node_id: NodeId,
        index: usize,
        point_type: PenPathPointType,
    ) -> bool {
        if !self.is_editable(&node_id) {
            return false;
        }
        let Some(node) = find_node_mut(self.active_children_mut(), &node_id) else {
            return false;
        };
        {
            let PenNode::Path(path) = &mut *node else {
                return false;
            };
            let Some(anchors) = path.anchors.as_mut() else {
                return false;
            };
            let Some(anchor) = anchors.get_mut(index) else {
                return false;
            };
            let is_mirrored = point_type == PenPathPointType::Mirrored;
            anchor.point_type = Some(point_type);
            if is_mirrored {
                match (anchor.handle_out.clone(), anchor.handle_in.clone()) {
                    (Some(h), _) => {
                        anchor.handle_in = Some(PenPathHandle { x: -h.x, y: -h.y });
                    }
                    (None, Some(h)) => {
                        anchor.handle_out = Some(PenPathHandle { x: -h.x, y: -h.y });
                    }
                    (None, None) => {}
                }
            }
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
