//! Align / distribute mutators backing the PropertyPanel's Align
//! section. Reference frame:
//! - 2+ selected → union of selection bounds.
//! - 1 selected → parent container's `aggregate_bounds` (top-level
//!   nodes have no useful reference and silently no-op).
//! Distribute requires 3+ nodes; <3 silently no-ops.
//!
//! All deltas use `translate_walk`, so containers cascade to their
//! descendants the same way drag-move does.

use super::walkers::*;
use super::*;

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

impl Document {
    /// Apply alignment or distribution to the active selection.
    /// Returns true when at least one node moved; pushes history
    /// only on real motion.
    pub fn align_selected(&mut self, action: AlignAction) -> bool {
        let editable: Vec<NodeId> = self
            .selected_set
            .iter()
            .copied()
            .filter(|id| self.is_editable(*id))
            .collect();
        if editable.is_empty() {
            return false;
        }
        if action.is_distribute() && editable.len() < 3 {
            return false;
        }
        let reference = {
            let Some(page) = self.active_page() else {
                return false;
            };
            if editable.len() >= 2 {
                match union_aggregate_bounds(page, &editable) {
                    Some(r) => r,
                    None => return false,
                }
            } else {
                match parent_aggregate_bounds_walk(&page.children, editable[0]) {
                    Some(r) => r,
                    None => return false,
                }
            }
        };
        let pre = self.snapshot_for_history();
        let active = self.active_page_index;
        let Some(page) = self.pages.get_mut(active) else {
            return false;
        };
        let any_change = if action.is_distribute() {
            apply_distribute(page, &editable, action)
        } else {
            apply_align(page, &editable, reference, action)
        };
        if any_change {
            self.history_push_past(pre);
        }
        any_change
    }
}

/// Move each editable node so its (left / center-h / right / top /
/// center-v / bottom) edge matches the reference rect.
///
/// Ancestor-in-set dedup: when both an ancestor and its descendant
/// are selected, only the ancestor moves — the descendant cascades
/// via `translate_walk`. Same invariant as `translate_selected` so
/// `align_selected(Left)` matches `translate_selected(dx, 0)` for
/// the same delta.
fn apply_align(
    page: &mut Page,
    editable: &[NodeId],
    reference: crate::Rect,
    action: AlignAction,
) -> bool {
    let ref_min_x = reference.origin.x;
    let ref_max_x = ref_min_x + reference.size.x;
    let ref_mid_x = ref_min_x + reference.size.x / 2.0;
    let ref_min_y = reference.origin.y;
    let ref_max_y = ref_min_y + reference.size.y;
    let ref_mid_y = ref_min_y + reference.size.y / 2.0;
    let mut moved = false;
    for id in editable {
        if is_ancestor_in_set(&page.children, *id, editable) {
            continue;
        }
        let Some(cur) = page.find(*id).map(Node::aggregate_bounds) else {
            continue;
        };
        let (cx, cy, cw, ch) = (cur.origin.x, cur.origin.y, cur.size.x, cur.size.y);
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
        for child in page.children.iter_mut() {
            if translate_walk(child, *id, dx, dy) {
                moved = true;
                break;
            }
        }
    }
    moved
}

/// Sort by center along the distribution axis, then redistribute
/// the inner nodes so their centers are equally spaced between the
/// outermost two. Endpoints stay put.
fn apply_distribute(page: &mut Page, editable: &[NodeId], action: AlignAction) -> bool {
    let horizontal = action.is_horizontal();
    // Ancestor-in-set dedup before sorting: a selected ancestor +
    // descendant must not contribute two anchors to the spacing
    // calculation. Same invariant as `apply_align`.
    let filtered: Vec<NodeId> = editable
        .iter()
        .copied()
        .filter(|id| !is_ancestor_in_set(&page.children, *id, editable))
        .collect();
    let mut sorted: Vec<(NodeId, crate::Rect)> = filtered
        .iter()
        .filter_map(|id| page.find(*id).map(|n| (*id, n.aggregate_bounds())))
        .collect();
    if sorted.len() < 3 {
        return false;
    }
    sorted.sort_by(|a, b| {
        let (ac, bc) = if horizontal {
            (
                a.1.origin.x + a.1.size.x / 2.0,
                b.1.origin.x + b.1.size.x / 2.0,
            )
        } else {
            (
                a.1.origin.y + a.1.size.y / 2.0,
                b.1.origin.y + b.1.size.y / 2.0,
            )
        };
        ac.partial_cmp(&bc).unwrap_or(std::cmp::Ordering::Equal)
    });
    let n = sorted.len();
    let (first_c, last_c) = if horizontal {
        (
            sorted[0].1.origin.x + sorted[0].1.size.x / 2.0,
            sorted[n - 1].1.origin.x + sorted[n - 1].1.size.x / 2.0,
        )
    } else {
        (
            sorted[0].1.origin.y + sorted[0].1.size.y / 2.0,
            sorted[n - 1].1.origin.y + sorted[n - 1].1.size.y / 2.0,
        )
    };
    let step = (last_c - first_c) / (n - 1) as f32;
    let mut moved = false;
    for i in 1..n - 1 {
        let (id, cur) = sorted[i];
        let cur_c = if horizontal {
            cur.origin.x + cur.size.x / 2.0
        } else {
            cur.origin.y + cur.size.y / 2.0
        };
        let target_c = first_c + step * i as f32;
        let delta = target_c - cur_c;
        if delta == 0.0 {
            continue;
        }
        let (dx, dy) = if horizontal {
            (delta, 0.0)
        } else {
            (0.0, delta)
        };
        for child in page.children.iter_mut() {
            if translate_walk(child, id, dx, dy) {
                moved = true;
                break;
            }
        }
    }
    moved
}

/// Walk `children` looking for the node whose own `children` vec
/// contains `target`. Returns that parent's `aggregate_bounds`.
/// None when `target` is top-level or absent.
fn parent_aggregate_bounds_walk(children: &[Node], target: NodeId) -> Option<crate::Rect> {
    for child in children {
        if child.children.iter().any(|c| c.id == target) {
            return Some(child.aggregate_bounds());
        }
        if let Some(rect) = parent_aggregate_bounds_walk(&child.children, target) {
            return Some(rect);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn rect_at(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::xywh(x, y, w, h)
    }

    /// Builds a fresh page with three rectangles at known offsets.
    fn three_rects(positions: &[(f32, f32, f32, f32)]) -> Document {
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        for (i, &(x, y, w, h)) in positions.iter().enumerate() {
            let mut node = Node::leaf(10 + i as u64, NodeKind::Rect, "r");
            node.bounds = rect_at(x, y, w, h);
            page.children.push(node);
        }
        let ids: Vec<NodeId> = positions
            .iter()
            .enumerate()
            .map(|(i, _)| NodeId::new(10 + i as u64))
            .collect();
        doc.selected_set = ids.clone();
        doc.selected = *ids.last().unwrap();
        doc
    }

    fn bounds_of(doc: &Document, id: NodeId) -> Rect {
        doc.active_page().unwrap().find(id).unwrap().aggregate_bounds()
    }

    #[test]
    fn align_left_snaps_to_union_min_x() {
        let mut doc = three_rects(&[(10.0, 0.0, 40.0, 20.0), (50.0, 100.0, 30.0, 20.0)]);
        assert!(doc.align_selected(AlignAction::Left));
        assert_eq!(bounds_of(&doc, NodeId::new(10)).origin.x, 10.0);
        assert_eq!(bounds_of(&doc, NodeId::new(11)).origin.x, 10.0);
        // History captured.
        assert_eq!(doc.history.past.len(), 1);
    }

    #[test]
    fn align_right_snaps_to_union_max_x() {
        let mut doc = three_rects(&[(0.0, 0.0, 40.0, 20.0), (50.0, 100.0, 30.0, 20.0)]);
        assert!(doc.align_selected(AlignAction::Right));
        // Union max-x = 80; first node moves so its right=80 → x=40.
        assert_eq!(bounds_of(&doc, NodeId::new(10)).origin.x, 40.0);
        assert_eq!(bounds_of(&doc, NodeId::new(11)).origin.x, 50.0);
    }

    #[test]
    fn align_center_h_snaps_to_union_mid_x() {
        let mut doc = three_rects(&[(0.0, 0.0, 40.0, 20.0), (60.0, 100.0, 20.0, 20.0)]);
        // Union x=[0..80], mid=40. Rect1 (w=40) → x=20. Rect2 (w=20) → x=30.
        assert!(doc.align_selected(AlignAction::CenterH));
        assert_eq!(bounds_of(&doc, NodeId::new(10)).origin.x, 20.0);
        assert_eq!(bounds_of(&doc, NodeId::new(11)).origin.x, 30.0);
    }

    #[test]
    fn align_top_snaps_to_union_min_y() {
        let mut doc = three_rects(&[(0.0, 10.0, 20.0, 20.0), (50.0, 80.0, 20.0, 20.0)]);
        assert!(doc.align_selected(AlignAction::Top));
        assert_eq!(bounds_of(&doc, NodeId::new(10)).origin.y, 10.0);
        assert_eq!(bounds_of(&doc, NodeId::new(11)).origin.y, 10.0);
    }

    #[test]
    fn align_bottom_snaps_to_union_max_y() {
        let mut doc = three_rects(&[(0.0, 0.0, 20.0, 20.0), (50.0, 50.0, 20.0, 30.0)]);
        // Union y=[0..80], max=80. Rect1 (h=20) → y=60. Rect2 (h=30) → y=50.
        assert!(doc.align_selected(AlignAction::Bottom));
        assert_eq!(bounds_of(&doc, NodeId::new(10)).origin.y, 60.0);
        assert_eq!(bounds_of(&doc, NodeId::new(11)).origin.y, 50.0);
    }

    #[test]
    fn align_center_v_snaps_to_union_mid_y() {
        let mut doc = three_rects(&[(0.0, 0.0, 20.0, 40.0), (50.0, 60.0, 20.0, 20.0)]);
        // Union y=[0..80], mid=40. Rect1 (h=40) → y=20. Rect2 (h=20) → y=30.
        assert!(doc.align_selected(AlignAction::CenterV));
        assert_eq!(bounds_of(&doc, NodeId::new(10)).origin.y, 20.0);
        assert_eq!(bounds_of(&doc, NodeId::new(11)).origin.y, 30.0);
    }

    #[test]
    fn distribute_h_equal_spacing_between_centers() {
        // Centers at x = 10, 25, 90. After distribute the middle
        // moves so step = (90 - 10) / 2 = 40 → middle center = 50.
        let mut doc = three_rects(&[
            (0.0, 0.0, 20.0, 20.0),
            (20.0, 0.0, 10.0, 20.0),
            (80.0, 0.0, 20.0, 20.0),
        ]);
        assert!(doc.align_selected(AlignAction::DistributeH));
        // Middle node center → 50; w=10 → x=45.
        assert_eq!(bounds_of(&doc, NodeId::new(11)).origin.x, 45.0);
        // Endpoints unchanged.
        assert_eq!(bounds_of(&doc, NodeId::new(10)).origin.x, 0.0);
        assert_eq!(bounds_of(&doc, NodeId::new(12)).origin.x, 80.0);
    }

    #[test]
    fn distribute_v_equal_spacing_between_centers() {
        let mut doc = three_rects(&[
            (0.0, 0.0, 20.0, 20.0),
            (0.0, 30.0, 20.0, 10.0),
            (0.0, 80.0, 20.0, 20.0),
        ]);
        // Centers y = 10, 35, 90 → step = 40 → middle center = 50.
        // Middle h=10 → y=45.
        assert!(doc.align_selected(AlignAction::DistributeV));
        assert_eq!(bounds_of(&doc, NodeId::new(11)).origin.y, 45.0);
    }

    #[test]
    fn distribute_under_three_is_no_op() {
        let mut doc = three_rects(&[(0.0, 0.0, 20.0, 20.0), (40.0, 0.0, 20.0, 20.0)]);
        // Only two selected — distribute returns false.
        assert!(!doc.align_selected(AlignAction::DistributeH));
        // History untouched.
        assert_eq!(doc.history.past.len(), 0);
    }

    #[test]
    fn empty_selection_no_ops() {
        let mut doc = three_rects(&[(0.0, 0.0, 20.0, 20.0)]);
        doc.selected_set.clear();
        doc.selected = NodeId::NONE;
        assert!(!doc.align_selected(AlignAction::Left));
        assert_eq!(doc.history.past.len(), 0);
    }

    #[test]
    fn already_aligned_skips_history() {
        // Both already at x=10 → align-left is a 0-delta no-op.
        let mut doc = three_rects(&[(10.0, 0.0, 20.0, 20.0), (10.0, 50.0, 30.0, 20.0)]);
        assert!(!doc.align_selected(AlignAction::Left));
        assert_eq!(doc.history.past.len(), 0);
    }

    #[test]
    fn single_select_aligns_to_parent_frame() {
        // Frame parent at (0, 0, 200, 100) with a 20x20 child at (50, 50).
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut child = Node::leaf(20, NodeKind::Rect, "c");
        child.bounds = rect_at(50.0, 50.0, 20.0, 20.0);
        let mut frame = Node::with_children(10, NodeKind::Frame, "f", vec![child]);
        frame.bounds = rect_at(0.0, 0.0, 200.0, 100.0);
        page.children.push(frame);
        doc.selected_set = vec![NodeId::new(20)];
        doc.selected = NodeId::new(20);
        // Align-left → child.x = 0 (parent's left edge).
        assert!(doc.align_selected(AlignAction::Left));
        assert_eq!(bounds_of(&doc, NodeId::new(20)).origin.x, 0.0);
    }

    #[test]
    fn selection_mutators_clear_align_toolbar_hover() {
        // Codex CONCERN-4: dropping below 2 selected must clear
        // align_toolbar_hover so no stale tint survives until the
        // next cursor move. set_single_selection / toggle_selection /
        // clear_selection all need this.
        let mut doc = three_rects(&[(0.0, 0.0, 20.0, 20.0), (40.0, 0.0, 20.0, 20.0)]);
        doc.ui.align_toolbar_hover = Some(AlignAction::Left);
        doc.set_single_selection(NodeId::new(10));
        assert_eq!(doc.ui.align_toolbar_hover, None);

        doc.selected_set = vec![NodeId::new(10), NodeId::new(11)];
        doc.selected = NodeId::new(11);
        doc.ui.align_toolbar_hover = Some(AlignAction::Right);
        doc.toggle_selection(NodeId::new(11));
        // selection_count now 1 → hover must clear.
        assert_eq!(doc.ui.align_toolbar_hover, None);

        doc.selected_set = vec![NodeId::new(10), NodeId::new(11)];
        doc.selected = NodeId::new(11);
        doc.ui.align_toolbar_hover = Some(AlignAction::CenterH);
        doc.clear_selection();
        assert_eq!(doc.ui.align_toolbar_hover, None);
    }

    #[test]
    fn toggle_adding_third_node_keeps_hover() {
        // Codex CONCERN-4 inverse: when toggle KEEPS count >= 2 (or
        // grows it), the hover should NOT be cleared — losing it
        // would break the visual feedback for a hovered button.
        let mut doc = three_rects(&[
            (0.0, 0.0, 20.0, 20.0),
            (40.0, 0.0, 20.0, 20.0),
            (80.0, 0.0, 20.0, 20.0),
        ]);
        // Pre-select 2 nodes, then toggle a 3rd in.
        doc.selected_set = vec![NodeId::new(10), NodeId::new(11)];
        doc.selected = NodeId::new(11);
        doc.ui.align_toolbar_hover = Some(AlignAction::DistributeH);
        doc.toggle_selection(NodeId::new(12));
        // Count goes to 3, hover persists.
        assert_eq!(doc.ui.align_toolbar_hover, Some(AlignAction::DistributeH));
    }

    #[test]
    fn ancestor_in_set_skips_descendant_align() {
        // Frame at (0, 0, 200, 200) with a 20x20 child at (150, 50).
        // Both Frame and child are in the selection. After align-left,
        // ONLY the Frame should move (delta = 0); the child cascades
        // along. The descendant's own apply_align entry must be
        // skipped or the child would move twice. (codex CONCERN-1)
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        let mut child = Node::leaf(20, NodeKind::Rect, "c");
        child.bounds = Rect::xywh(150.0, 50.0, 20.0, 20.0);
        let mut frame = Node::with_children(10, NodeKind::Frame, "f", vec![child]);
        frame.bounds = Rect::xywh(0.0, 0.0, 200.0, 200.0);
        // Separate sibling so the union has something to align against.
        let mut sibling = Node::leaf(30, NodeKind::Rect, "s");
        sibling.bounds = Rect::xywh(400.0, 0.0, 100.0, 100.0);
        page.children.push(frame);
        page.children.push(sibling);
        doc.selected_set = vec![NodeId::new(10), NodeId::new(20), NodeId::new(30)];
        doc.selected = NodeId::new(30);
        // Union x = [0, 500]. Align-left snaps everything to x=0.
        assert!(doc.align_selected(AlignAction::Left));
        // Frame already at x=0 — no delta needed; but the child
        // would be at x=350 if it had been moved twice, OR at x=150
        // if it had been moved once standalone (independent shift).
        // Correct cascade keeps child relative-to-parent: x=150.
        assert_eq!(bounds_of(&doc, NodeId::new(10)).origin.x, 0.0);
        assert_eq!(bounds_of(&doc, NodeId::new(20)).origin.x, 150.0);
        // Sibling does move: from x=400 to x=0.
        assert_eq!(bounds_of(&doc, NodeId::new(30)).origin.x, 0.0);
    }

    #[test]
    fn distribute_dedups_ancestor_in_set() {
        // Three top-level Frames each with a child. All 6 nodes in
        // selection. Distribute-H should sort by the THREE Frames'
        // centers, not by 6 anchors — descendants ride along.
        let mut doc = Document::empty();
        let page = doc.pages.get_mut(0).unwrap();
        page.children.clear();
        for i in 0..3 {
            let cx = i as f32 * 40.0 + 10.0;
            let mut child = Node::leaf(200 + i as u64, NodeKind::Rect, "c");
            child.bounds = Rect::xywh(cx, 0.0, 20.0, 20.0);
            let mut frame =
                Node::with_children(100 + i as u64, NodeKind::Frame, "f", vec![child]);
            frame.bounds = Rect::xywh(i as f32 * 80.0, 0.0, 40.0, 40.0);
            page.children.push(frame);
        }
        // Mess up middle frame: push x=20 so its center is 40 (not 100).
        // Frame centers: 20, 40, 200 → step (200-20)/2 = 90 → middle → 110.
        page.children[1].bounds = Rect::xywh(20.0, 0.0, 40.0, 40.0);
        page.children[2].bounds = Rect::xywh(180.0, 0.0, 40.0, 40.0);
        doc.selected_set = vec![
            NodeId::new(100), NodeId::new(101), NodeId::new(102),
            NodeId::new(200), NodeId::new(201), NodeId::new(202),
        ];
        doc.selected = NodeId::new(102);
        assert!(doc.align_selected(AlignAction::DistributeH));
        // Middle frame center should be 110 → frame.x = 90.
        assert_eq!(bounds_of(&doc, NodeId::new(101)).origin.x, 90.0);
        // Endpoints untouched.
        assert_eq!(bounds_of(&doc, NodeId::new(100)).origin.x, 0.0);
        assert_eq!(bounds_of(&doc, NodeId::new(102)).origin.x, 180.0);
    }

    #[test]
    fn single_select_top_level_no_ops() {
        // Single top-level node has no parent reference → no-op.
        let mut doc = three_rects(&[(50.0, 50.0, 20.0, 20.0)]);
        assert!(!doc.align_selected(AlignAction::Left));
        assert_eq!(doc.history.past.len(), 0);
    }
}
