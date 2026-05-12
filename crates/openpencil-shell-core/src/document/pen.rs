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
        let raw = (*next_id).max(safe);
        *next_id = raw.checked_add(1)?;
        let id = NodeId::new(raw);
        let mut node = Node::leaf(raw, NodeKind::Path, "Path")
            .with_stroke(crate::Color::BLACK, 2.0)
            .with_bounds(crate::Rect {
                origin: first,
                size: Point2D::new(0.0, 0.0),
            });
        node.points.push(first);
        let active = self.active_page_index;
        let page = self.pages.get_mut(active)?;
        page.children.push(node);
        self.ui.pending_pen_history = Some(self.snapshot_for_history());
        self.ui.pen_in_progress = Some(id);
        self.set_single_selection(id);
        Some(id)
    }

    /// Append an anchor to the in-progress path. Recomputes the
    /// node's `bounds` so the layer panel + property panel stay
    /// in sync with the visible geometry.
    pub fn add_pen_point(&mut self, p: Point2D) -> bool {
        let Some(id) = self.ui.pen_in_progress else {
            return false;
        };
        let active = self.active_page_index;
        let Some(page) = self.pages.get_mut(active) else {
            return false;
        };
        for child in &mut page.children {
            if let Some(points) = path_points_mut_walk(child, id) {
                points.push(p);
                let (origin, size) = bbox_of(points);
                // Have to re-walk to set bounds because we borrow
                // `points` exclusively above; drop and walk again.
                let _ = (origin, size);
                let mut found = false;
                set_path_bounds_walk(child, id, &mut found);
                return found || true;
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
            if let Some(node) = page.find(id) {
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

fn set_path_bounds_walk(node: &mut Node, target: NodeId, found: &mut bool) {
    if node.id == target {
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
