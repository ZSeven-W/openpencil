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
/// identical native span (scale stays `1.0`). Also re-bakes the `d`
/// string from the anchors (TS parity: every path patch persists `d`
/// alongside `anchors` via `bakeSceneAnchorsToPathNode`). No-op on
/// non-Path nodes; nodes without anchors (e.g. preserve-`d` SVG
/// imports) keep their authored `d`.
pub(crate) fn refit_path_bounds(node: &mut PenNode) {
    if let PenNode::Path(p) = node {
        let anchors = p.anchors.clone().unwrap_or_default();
        let closed = p.closed.unwrap_or(false);
        if !anchors.is_empty() {
            p.d = Some(crate::path_edit::anchors_to_path_data(&anchors, closed));
        }
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
        let mut node = make_path(id.clone().into(), "Path", first);
        // TS commits the path with a transparent fill + a default
        // 1 px #374151 stroke (`skia-pen-tool.ts:199-209`). Seed them
        // at start so the in-progress node paints like the committed
        // one will.
        crate::fills::set_primary_fill_hex(&mut node, "transparent");
        crate::fills::set_primary_stroke_hex(&mut node, "#374151");
        self.active_children_mut().push(node);
        self.ui.pending_pen_history = Some(pre);
        self.ui.pen_in_progress = Some(id.clone());
        // TS arms the handle drag on every mousedown — moving before
        // release bows the just-placed anchor into mirrored handles.
        self.ui.pen_dragging_handle = true;
        self.set_single_selection(id.clone());
        Some(id)
    }

    /// Append an anchor to the in-progress path, re-fitting bounds.
    /// Arms the handle drag like TS `onMouseDown`.
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
        self.ui.pen_dragging_handle = true;
        true
    }

    /// Cursor move while the Pen press is held — port of the TS
    /// `penDraggingHandle` branch (`skia-pen-tool.ts:97-110`): drag
    /// past 2 doc px mints mirrored handles on the LAST anchor
    /// (`handle_out = cursor − anchor`, `handle_in` its negation);
    /// inside the threshold the handles clear back to a corner.
    pub fn pen_drag_handle_to(&mut self, cursor: (f64, f64)) -> bool {
        if !self.ui.pen_dragging_handle {
            return false;
        }
        let Some(id) = self.ui.pen_in_progress.clone() else {
            return false;
        };
        let Some(node) = find_node_mut(self.active_children_mut(), &id) else {
            return false;
        };
        if let PenNode::Path(path) = node {
            let Some(last) = path.anchors.as_mut().and_then(|a| a.last_mut()) else {
                return false;
            };
            let dx = cursor.0 - last.x;
            let dy = cursor.1 - last.y;
            if dx.hypot(dy) > 2.0 {
                last.handle_out = Some(PenPathHandle { x: dx, y: dy });
                last.handle_in = Some(PenPathHandle { x: -dx, y: -dy });
                last.point_type = Some(PenPathPointType::Mirrored);
            } else {
                last.handle_out = None;
                last.handle_in = None;
                last.point_type = Some(PenPathPointType::Corner);
            }
        } else {
            return false;
        }
        refit_path_bounds(node);
        true
    }

    /// Pen press released — stop minting handles (TS `onMouseUp`).
    pub fn pen_release(&mut self) {
        self.ui.pen_dragging_handle = false;
    }

    /// True when `p` (doc coords) lands within the close-path hit
    /// zone of the in-progress path's FIRST anchor — TS
    /// `skia-pen-tool.ts:71-79`: needs ≥ 3 anchors, threshold
    /// `PEN_CLOSE_HIT_THRESHOLD (8) / zoom`.
    pub fn pen_close_hit(&self, p: (f64, f64), zoom: f32) -> bool {
        let Some(id) = &self.ui.pen_in_progress else {
            return false;
        };
        let Some(PenNode::Path(path)) = find_node(self.active_children(), id) else {
            return false;
        };
        let Some(anchors) = path.anchors.as_ref() else {
            return false;
        };
        if anchors.len() < 3 {
            return false;
        }
        let first = &anchors[0];
        let threshold = 8.0 / zoom.max(0.0001) as f64;
        (p.0 - first.x).hypot(p.1 - first.y) < threshold
    }

    /// Backspace while authoring — TS `onKeyDown('Backspace')`
    /// (`skia-pen-tool.ts:142-150`): pop the last anchor while more
    /// than one remains; with a single anchor left the whole path
    /// cancels (the host then returns to the Select tool, mirroring
    /// TS `cancel()`). True when a pen session consumed the key.
    pub fn pen_backspace(&mut self) -> bool {
        let Some(id) = self.ui.pen_in_progress.clone() else {
            return false;
        };
        let count = find_node(self.active_children(), &id)
            .and_then(|n| match n {
                PenNode::Path(p) => Some(p.anchors.as_ref().map(|a| a.len()).unwrap_or(0)),
                _ => None,
            })
            .unwrap_or(0);
        if count > 1 {
            if let Some(node) = find_node_mut(self.active_children_mut(), &id) {
                if let PenNode::Path(path) = &mut *node {
                    if let Some(anchors) = path.anchors.as_mut() {
                        anchors.pop();
                    }
                }
                refit_path_bounds(node);
            }
        } else {
            self.cancel_pen_path();
        }
        true
    }

    /// Discard the in-progress path without committing — TS
    /// `SkiaPenTool.cancel()` / `onToolChange()`: the node is
    /// stripped and NO history is pushed (in TS the node was never
    /// inserted). Does NOT touch the tool: the Escape host path
    /// switches to Select (TS `cancel()`), a tool switch keeps the
    /// just-picked tool (TS `onToolChange()`).
    pub fn cancel_pen_path(&mut self) -> bool {
        let Some(id) = self.ui.pen_in_progress.take() else {
            return false;
        };
        self.ui.pending_pen_history = None;
        self.ui.pen_cursor_doc = None;
        self.ui.pen_dragging_handle = false;
        self.ui.pen_last_press = None;
        self.active_children_mut()
            .retain(|n| n.id_str() != id.as_str());
        if self.selection.anchor == id {
            self.clear_selection();
        }
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

    /// Commit the in-progress Pen path as an OPEN path. See
    /// [`Self::finish_pen_path_with`].
    pub fn finish_pen_path(&mut self) -> bool {
        self.finish_pen_path_with(false)
    }

    /// Commit the in-progress Pen path — TS `finalizePen(closed)`
    /// (`skia-pen-tool.ts:169-220`). Writes the `closed` flag,
    /// re-bakes `d` / bounds, then pushes the pre-pen snapshot onto
    /// the undo stack. A path with < 2 anchors OR degenerate bounds
    /// (< 0.001 doc px in BOTH spans — TS `bakeSceneAnchorsToPathNode`
    /// returns null) is stripped without polluting history. The tool
    /// returns to Select either way (TS `setActiveTool('select')`).
    /// True when a session was active.
    pub fn finish_pen_path_with(&mut self, closed: bool) -> bool {
        let Some(id) = self.ui.pen_in_progress.take() else {
            self.ui.pending_pen_history = None;
            self.ui.pen_cursor_doc = None;
            self.ui.pen_dragging_handle = false;
            self.ui.pen_last_press = None;
            return false;
        };
        let pending = self.ui.pending_pen_history.take();
        let mut anchor_count = 0;
        let mut degenerate = true;
        if let Some(node) = find_node_mut(self.active_children_mut(), &id) {
            if let PenNode::Path(p) = &mut *node {
                anchor_count = p.anchors.as_ref().map(|a| a.len()).unwrap_or(0);
                p.closed = Some(closed);
                let (_, _, w, h) = crate::path_bounds::path_bounds_from_anchors(
                    p.anchors.as_deref().unwrap_or(&[]),
                    closed,
                );
                degenerate = w < 0.001 && h < 0.001;
            }
            // Bake the final `closed`-aware `d` + bounds.
            refit_path_bounds(node);
        }
        if anchor_count >= 2 && !degenerate {
            if let Some(snap) = pending {
                self.history_push_past(snap);
            }
        } else {
            // Invisible path (lone anchor / zero-span) — strip it,
            // skip history.
            self.active_children_mut()
                .retain(|n| n.id_str() != id.as_str());
            if self.selection.anchor == id {
                self.clear_selection();
            }
        }
        self.ui.pen_cursor_doc = None;
        self.ui.pen_dragging_handle = false;
        self.ui.pen_last_press = None;
        // TS finalizePen always lands back on the Select tool.
        self.tool = crate::tool::Tool::Select;
        true
    }
}
