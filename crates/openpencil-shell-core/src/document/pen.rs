//! Multi-anchor Pen-tool mutators. Builds a `NodeKind::Path` node
//! whose `points` are document-space anchors, recomputing the
//! bounding box after each anchor. History is captured on the
//! first anchor and pushed on commit only when the path actually
//! has ≥ 2 anchors (a 1-anchor path is invisible — no real change).

use super::walkers::path_points_mut_walk;
use super::*;
use crate::Point2D;

impl Document {
    /// Start a fresh Pen-tool path at `first` (document coords).
    /// Returns the new node's id or `None` on allocator overflow.
    /// Mints the id past `max_node_id() + 1` like the dup path.
    pub fn start_pen_path(&mut self, next_id: &mut u64, first: Point2D) -> Option<NodeId> {
        let safe = self.max_node_id().checked_add(1)?;
        *next_id = (*next_id).max(safe);
        let mut taken = self.collect_node_ids();
        let id = super::walkers::alloc_n_id(next_id, &mut taken)?;
        // Snapshot BEFORE any mutation so a later undo restores the
        // pre-pen document state — otherwise the snapshot already
        // contains the new path node and undo is a no-op visually.
        let pre = self.snapshot_for_history();
        let mut node = Node::leaf(id.as_str(), NodeKind::Path, "Path")
            .with_stroke(crate::Color::BLACK, 2.0)
            .with_bounds(crate::Rect {
                origin: first,
                size: Point2D::new(0.0, 0.0),
            });
        node.points.push(first);
        let active = self.active_page_index;
        let page = self.pages.get_mut(active)?;
        page.children.push(node);
        self.ui.pending_pen_history = Some(pre);
        self.ui.pen_in_progress = Some(id.clone());
        self.set_single_selection(id.clone());
        Some(id)
    }

    /// Append an anchor to the in-progress path. Recomputes the
    /// node's `bounds` so the layer panel + property panel stay
    /// in sync with the visible geometry.
    pub fn add_pen_point(&mut self, p: Point2D) -> bool {
        let Some(id) = self.ui.pen_in_progress.clone() else {
            return false;
        };
        let active = self.active_page_index;
        let Some(page) = self.pages.get_mut(active) else {
            return false;
        };
        for child in &mut page.children {
            if let Some(points) = path_points_mut_walk(child, &id) {
                points.push(p);
                let (origin, size) = bbox_of(points);
                // Have to re-walk to set bounds because we borrow
                // `points` exclusively above; drop and walk again.
                let _ = (origin, size);
                let mut found = false;
                set_path_bounds_walk(child, &id, &mut found);
                return found || true;
            }
        }
        false
    }

    /// Move a single anchor on an existing Path node to `pos`
    /// (document coords). Used by the anchor-drag interaction on the
    /// canvas — picks a specific anchor index to translate while the
    /// rest of the path stays put. Returns true when the anchor was
    /// found + updated. No-op (false) when the node isn't a Path,
    /// the index is out of range, or the path isn't on the active
    /// page. History is the caller's responsibility (anchor drag
    /// commits one snapshot per drag start, not per cursor move).
    pub fn set_path_anchor_position(
        &mut self,
        node_id: NodeId,
        index: usize,
        pos: Point2D,
    ) -> bool {
        if !self.is_editable(&node_id) {
            return false;
        }
        let active = self.active_page_index;
        let Some(page) = self.pages.get_mut(active) else {
            return false;
        };
        for child in &mut page.children {
            if let Some(points) = path_points_mut_walk(child, &node_id) {
                if index >= points.len() {
                    return false;
                }
                points[index] = pos;
                let mut found = false;
                set_path_bounds_walk(child, &node_id, &mut found);
                return true;
            }
        }
        false
    }

    /// Commit the in-progress Pen path. Returns true when one was
    /// active (UI changed → repaint). Pushes the pre-pen snapshot
    /// onto the undo stack only when the path has ≥ 2 anchors —
    /// otherwise the lone-anchor node is removed so undo doesn't
    /// see a phantom entry for an invisible click.
    pub fn finish_pen_path(&mut self) -> bool {
        let Some(id) = self.ui.pen_in_progress.take() else {
            self.ui.pending_pen_history = None;
            self.ui.pen_cursor_doc = None;
            return false;
        };
        let pending = self.ui.pending_pen_history.take();
        let mut anchor_count = 0;
        if let Some(page) = self.pages.get(self.active_page_index) {
            if let Some(node) = page.find(&id) {
                anchor_count = node.points.len();
            }
        }
        if anchor_count >= 2 {
            if let Some(snap) = pending {
                self.history_push_past(snap);
            }
        } else {
            // 1-anchor path is invisible — strip it and skip history.
            if let Some(page) = self.pages.get_mut(self.active_page_index) {
                page.children.retain(|n| n.id != id);
            }
            if self.selected == id {
                self.clear_selection();
            }
        }
        self.ui.pen_cursor_doc = None;
        true
    }
}

fn bbox_of(points: &[Point2D]) -> (Point2D, Point2D) {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for p in points {
        if p.x < min_x {
            min_x = p.x;
        }
        if p.y < min_y {
            min_y = p.y;
        }
        if p.x > max_x {
            max_x = p.x;
        }
        if p.y > max_y {
            max_y = p.y;
        }
    }
    if min_x.is_finite() {
        (
            Point2D::new(min_x, min_y),
            Point2D::new(max_x - min_x, max_y - min_y),
        )
    } else {
        (Point2D::new(0.0, 0.0), Point2D::new(0.0, 0.0))
    }
}

fn set_path_bounds_walk(node: &mut Node, target: &NodeId, found: &mut bool) {
    if node.id == *target {
        if matches!(node.kind, NodeKind::Path) {
            let (origin, size) = bbox_of(&node.points);
            node.bounds = crate::Rect { origin, size };
        }
        *found = true;
        return;
    }
    for child in &mut node.children {
        if !*found {
            set_path_bounds_walk(child, target, found);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn path_doc(points: &[(f32, f32)]) -> Document {
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut n = Node::leaf("n10", NodeKind::Path, "p");
        for (x, y) in points {
            n.points.push(Point2D::new(*x, *y));
        }
        let (origin, size) = bbox_of(&n.points);
        n.bounds = crate::Rect { origin, size };
        page.children.push(n);
        doc.selected_set = vec![NodeId::new("n10")];
        doc.selected = NodeId::new("n10");
        doc
    }

    #[test]
    fn set_path_anchor_position_moves_one_anchor_and_recomputes_bounds() {
        let mut doc = path_doc(&[(0.0, 0.0), (50.0, 30.0), (100.0, 0.0)]);
        let ok = doc.set_path_anchor_position(NodeId::new("n10"), 1, Point2D::new(50.0, 90.0));
        assert!(ok);
        let page = doc.active_page().unwrap();
        let node = page.find(&NodeId::new("n10")).unwrap();
        assert_eq!(node.points[1], Point2D::new(50.0, 90.0));
        // Bounds should re-fit: y now goes 0..90.
        assert_eq!(node.bounds.origin.y, 0.0);
        assert_eq!(node.bounds.size.y, 90.0);
    }

    #[test]
    fn set_path_anchor_position_out_of_range_returns_false() {
        let mut doc = path_doc(&[(0.0, 0.0), (10.0, 10.0)]);
        let ok = doc.set_path_anchor_position(NodeId::new("n10"), 99, Point2D::new(0.0, 0.0));
        assert!(!ok);
    }

    #[test]
    fn set_path_anchor_position_rejects_non_path_node() {
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut n = Node::leaf("n10", NodeKind::Rect, "r");
        n.bounds = crate::Rect::xywh(0.0, 0.0, 50.0, 50.0);
        page.children.push(n);
        let ok = doc.set_path_anchor_position(NodeId::new("n10"), 0, Point2D::new(0.0, 0.0));
        // Non-Path: path_points_mut_walk returns None → false.
        assert!(!ok);
    }
}
