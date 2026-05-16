//! Align / distribute mutators — ported from shell-core's
//! `document/align.rs`, retargeted onto `EditorState` +
//! `jian_ops_schema::PenNode`.
//!
//! Reference frame:
//!   - 2+ selected → union of the selection's aggregate bounds.
//!   - 1 selected → parent container's aggregate bounds (a top-level
//!     node has no useful reference and silently no-ops).
//! Distribute requires 3+ nodes; fewer silently no-ops.
//!
//! All deltas go through [`crate::walkers::translate_subtree`], so
//! containers cascade to their descendants exactly like drag-move.
//! An ancestor-already-in-set dedup stops a descendant moving twice.

use crate::geometry::{aggregate_bounds, union_aggregate_bounds, DocRect};
use crate::node_id::NodeId;
use crate::state::EditorState;
use crate::walkers::{find_node, is_ancestor_in_set, translate_subtree};
use jian_ops_schema::node::PenNode;

/// The six align edges + two distribute axes the PropertyPanel's
/// Align section exposes. Ported verbatim from shell-core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignAction {
    Left,
    CenterH,
    Right,
    Top,
    CenterV,
    Bottom,
    DistributeH,
    DistributeV,
}

impl AlignAction {
    fn is_distribute(self) -> bool {
        matches!(self, Self::DistributeH | Self::DistributeV)
    }
    fn is_horizontal(self) -> bool {
        matches!(
            self,
            Self::Left | Self::CenterH | Self::Right | Self::DistributeH
        )
    }
}

impl EditorState {
    /// Apply alignment or distribution to the active selection.
    /// Returns true when at least one node moved; pushes history
    /// only on real motion.
    pub fn align_selected(&mut self, action: AlignAction) -> bool {
        let editable: Vec<NodeId> = self
            .selection
            .set
            .iter()
            .filter(|id| self.is_editable(id))
            .cloned()
            .collect();
        if editable.is_empty() {
            return false;
        }
        if action.is_distribute() && editable.len() < 3 {
            return false;
        }
        // Resolve the reference rect against an immutable borrow
        // before taking the mutable one.
        let reference = {
            let children = self.active_children();
            if editable.len() >= 2 {
                match union_aggregate_bounds(children, &editable) {
                    Some(r) => r,
                    None => return false,
                }
            } else {
                match parent_aggregate_bounds(children, &editable[0]) {
                    Some(r) => r,
                    None => return false,
                }
            }
        };
        let pre = self.snapshot_for_history();
        let children = self.active_children_mut();
        let moved = if action.is_distribute() {
            apply_distribute(children, &editable, action)
        } else {
            apply_align(children, &editable, reference, action)
        };
        if moved {
            self.history_push_past(pre);
        }
        moved
    }
}

/// Move each editable node so its edge / center matches `reference`.
///
/// Ancestor-in-set dedup: when both an ancestor and a descendant are
/// selected, only the ancestor moves — the descendant cascades via
/// `translate_subtree`.
fn apply_align(
    children: &mut [PenNode],
    editable: &[NodeId],
    reference: DocRect,
    action: AlignAction,
) -> bool {
    let ref_min_x = reference.x;
    let ref_max_x = reference.x + reference.w;
    let ref_mid_x = reference.x + reference.w / 2.0;
    let ref_min_y = reference.y;
    let ref_max_y = reference.y + reference.h;
    let ref_mid_y = reference.y + reference.h / 2.0;
    let mut moved = false;
    for id in editable {
        if is_ancestor_in_set(children, id, editable) {
            continue;
        }
        let Some(cur) = find_node(children, id).map(aggregate_bounds) else {
            continue;
        };
        let (cx, cy, cw, ch) = (cur.x, cur.y, cur.w, cur.h);
        let (dx, dy) = match action {
            AlignAction::Left => (ref_min_x - cx, 0.0),
            AlignAction::Right => (ref_max_x - (cx + cw), 0.0),
            AlignAction::CenterH => (ref_mid_x - (cx + cw / 2.0), 0.0),
            AlignAction::Top => (0.0, ref_min_y - cy),
            AlignAction::Bottom => (0.0, ref_max_y - (cy + ch)),
            AlignAction::CenterV => (0.0, ref_mid_y - (cy + ch / 2.0)),
            AlignAction::DistributeH | AlignAction::DistributeV => unreachable!(),
        };
        if dx == 0.0 && dy == 0.0 {
            continue;
        }
        if let Some(node) = crate::walkers::find_node_mut(children, id) {
            translate_subtree(node, dx, dy);
            moved = true;
        }
    }
    moved
}

/// Sort by center along the distribution axis, then evenly space the
/// inner nodes between the outermost two. Endpoints stay put.
fn apply_distribute(children: &mut [PenNode], editable: &[NodeId], action: AlignAction) -> bool {
    let horizontal = action.is_horizontal();
    // Ancestor-in-set dedup before sorting — a selected ancestor +
    // descendant must not contribute two anchors.
    let filtered: Vec<NodeId> = editable
        .iter()
        .cloned()
        .filter(|id| !is_ancestor_in_set(children, id, editable))
        .collect();
    let mut sorted: Vec<(NodeId, DocRect)> = filtered
        .iter()
        .filter_map(|id| find_node(children, id).map(|n| (id.clone(), aggregate_bounds(n))))
        .collect();
    if sorted.len() < 3 {
        return false;
    }
    let center = |r: &DocRect| -> f64 {
        if horizontal {
            r.x + r.w / 2.0
        } else {
            r.y + r.h / 2.0
        }
    };
    sorted.sort_by(|a, b| {
        center(&a.1)
            .partial_cmp(&center(&b.1))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let n = sorted.len();
    let first_c = center(&sorted[0].1);
    let last_c = center(&sorted[n - 1].1);
    let step = (last_c - first_c) / (n - 1) as f64;
    let mut moved = false;
    for i in 1..n - 1 {
        let (id, cur) = (sorted[i].0.clone(), sorted[i].1);
        let cur_c = center(&cur);
        let target_c = first_c + step * i as f64;
        let delta = target_c - cur_c;
        if delta == 0.0 {
            continue;
        }
        let (dx, dy) = if horizontal { (delta, 0.0) } else { (0.0, delta) };
        if let Some(node) = crate::walkers::find_node_mut(children, &id) {
            translate_subtree(node, dx, dy);
            moved = true;
        }
    }
    moved
}

/// Walk `children` for the node whose own `children` vec holds
/// `target`, returning that parent's aggregate bounds. `None` when
/// `target` is top-level or absent.
fn parent_aggregate_bounds(children: &[PenNode], target: &NodeId) -> Option<DocRect> {
    use crate::pen_node_ext::PenNodeExt;
    for child in children {
        if let Some(grand) = child.children() {
            if grand.iter().any(|c| c.id_str() == target.as_str()) {
                return Some(aggregate_bounds(child));
            }
            if let Some(rect) = parent_aggregate_bounds(grand, target) {
                return Some(rect);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::aggregate_bounds;
    use crate::test_support::{frame, rect, state_with};
    use crate::walkers::find_node;

    /// A state with `n` rectangles at known offsets, all selected.
    fn rects(positions: &[(f64, f64, f64, f64)]) -> EditorState {
        let roots: Vec<PenNode> = positions
            .iter()
            .enumerate()
            .map(|(i, &(x, y, w, h))| rect(&format!("n{}", 10 + i), "r", x, y, w, h))
            .collect();
        let mut s = state_with(roots);
        let ids: Vec<NodeId> = positions
            .iter()
            .enumerate()
            .map(|(i, _)| NodeId::new(format!("n{}", 10 + i)))
            .collect();
        s.selection.anchor = ids.last().cloned().unwrap();
        s.selection.set = ids;
        s
    }

    fn bx(s: &EditorState, id: &str) -> DocRect {
        aggregate_bounds(find_node(s.active_children(), &NodeId::new(id)).unwrap())
    }

    #[test]
    fn align_left_snaps_to_union_min_x() {
        let mut s = rects(&[(10.0, 0.0, 40.0, 20.0), (50.0, 100.0, 30.0, 20.0)]);
        assert!(s.align_selected(AlignAction::Left));
        assert_eq!(bx(&s, "n10").x, 10.0);
        assert_eq!(bx(&s, "n11").x, 10.0);
        assert_eq!(s.history.past.len(), 1);
    }

    #[test]
    fn align_right_snaps_to_union_max_x() {
        let mut s = rects(&[(0.0, 0.0, 40.0, 20.0), (50.0, 100.0, 30.0, 20.0)]);
        assert!(s.align_selected(AlignAction::Right));
        // Union max-x = 80; first node right=80 → x=40.
        assert_eq!(bx(&s, "n10").x, 40.0);
        assert_eq!(bx(&s, "n11").x, 50.0);
    }

    #[test]
    fn align_center_h_snaps_to_union_mid_x() {
        let mut s = rects(&[(0.0, 0.0, 40.0, 20.0), (60.0, 100.0, 20.0, 20.0)]);
        assert!(s.align_selected(AlignAction::CenterH));
        assert_eq!(bx(&s, "n10").x, 20.0);
        assert_eq!(bx(&s, "n11").x, 30.0);
    }

    #[test]
    fn align_top_snaps_to_union_min_y() {
        let mut s = rects(&[(0.0, 10.0, 20.0, 20.0), (50.0, 80.0, 20.0, 20.0)]);
        assert!(s.align_selected(AlignAction::Top));
        assert_eq!(bx(&s, "n10").y, 10.0);
        assert_eq!(bx(&s, "n11").y, 10.0);
    }

    #[test]
    fn align_bottom_snaps_to_union_max_y() {
        let mut s = rects(&[(0.0, 0.0, 20.0, 20.0), (50.0, 50.0, 20.0, 30.0)]);
        assert!(s.align_selected(AlignAction::Bottom));
        assert_eq!(bx(&s, "n10").y, 60.0);
        assert_eq!(bx(&s, "n11").y, 50.0);
    }

    #[test]
    fn align_center_v_snaps_to_union_mid_y() {
        let mut s = rects(&[(0.0, 0.0, 20.0, 40.0), (50.0, 60.0, 20.0, 20.0)]);
        assert!(s.align_selected(AlignAction::CenterV));
        assert_eq!(bx(&s, "n10").y, 20.0);
        assert_eq!(bx(&s, "n11").y, 30.0);
    }

    #[test]
    fn distribute_h_equal_spacing_between_centers() {
        let mut s = rects(&[
            (0.0, 0.0, 20.0, 20.0),
            (20.0, 0.0, 10.0, 20.0),
            (80.0, 0.0, 20.0, 20.0),
        ]);
        assert!(s.align_selected(AlignAction::DistributeH));
        // Middle node center → 50; w=10 → x=45.
        assert_eq!(bx(&s, "n11").x, 45.0);
        assert_eq!(bx(&s, "n10").x, 0.0);
        assert_eq!(bx(&s, "n12").x, 80.0);
    }

    #[test]
    fn distribute_v_equal_spacing_between_centers() {
        let mut s = rects(&[
            (0.0, 0.0, 20.0, 20.0),
            (0.0, 30.0, 20.0, 10.0),
            (0.0, 80.0, 20.0, 20.0),
        ]);
        assert!(s.align_selected(AlignAction::DistributeV));
        assert_eq!(bx(&s, "n11").y, 45.0);
    }

    #[test]
    fn distribute_under_three_is_no_op() {
        let mut s = rects(&[(0.0, 0.0, 20.0, 20.0), (40.0, 0.0, 20.0, 20.0)]);
        assert!(!s.align_selected(AlignAction::DistributeH));
        assert_eq!(s.history.past.len(), 0);
    }

    #[test]
    fn empty_selection_no_ops() {
        let mut s = rects(&[(0.0, 0.0, 20.0, 20.0)]);
        s.clear_selection();
        assert!(!s.align_selected(AlignAction::Left));
        assert_eq!(s.history.past.len(), 0);
    }

    #[test]
    fn already_aligned_skips_history() {
        let mut s = rects(&[(10.0, 0.0, 20.0, 20.0), (10.0, 50.0, 30.0, 20.0)]);
        assert!(!s.align_selected(AlignAction::Left));
        assert_eq!(s.history.past.len(), 0);
    }

    #[test]
    fn single_select_aligns_to_parent_frame() {
        let child = rect("n20", "c", 50.0, 50.0, 20.0, 20.0);
        let f = frame("n10", "f", 0.0, 0.0, 200.0, 100.0, vec![child]);
        let mut s = state_with(vec![f]);
        s.set_single_selection(NodeId::new("n20"));
        assert!(s.align_selected(AlignAction::Left));
        assert_eq!(bx(&s, "n20").x, 0.0);
    }

    #[test]
    fn single_select_top_level_no_ops() {
        let mut s = rects(&[(50.0, 50.0, 20.0, 20.0)]);
        assert!(!s.align_selected(AlignAction::Left));
        assert_eq!(s.history.past.len(), 0);
    }

    #[test]
    fn ancestor_in_set_skips_descendant_align() {
        let child = rect("n20", "c", 150.0, 50.0, 20.0, 20.0);
        let f = frame("n10", "f", 0.0, 0.0, 200.0, 200.0, vec![child]);
        let sibling = rect("n30", "s", 400.0, 0.0, 100.0, 100.0);
        let mut s = state_with(vec![f, sibling]);
        s.selection.set = vec![
            NodeId::new("n10"),
            NodeId::new("n20"),
            NodeId::new("n30"),
        ];
        s.selection.anchor = NodeId::new("n30");
        assert!(s.align_selected(AlignAction::Left));
        // Frame already at x=0; child cascades, so it keeps x=150.
        assert_eq!(bx(&s, "n10").x, 0.0);
        assert_eq!(bx(&s, "n20").x, 150.0);
        assert_eq!(bx(&s, "n30").x, 0.0);
    }
}
