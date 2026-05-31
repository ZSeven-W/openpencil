//! Page / clipboard / pen-tool mutator tests.

#![cfg(test)]

use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{rect, state_with};
use crate::walkers::find_node;
use jian_ops_schema::node::PenNode;

// --- Pages -----------------------------------------------------------

#[test]
fn add_page_promotes_single_page_document_and_migrates_root() {
    let mut s = state_with(vec![rect("n1", "A", 0.0, 0.0, 10.0, 10.0)]);
    assert!(s.doc.pages.is_none());
    let idx = s.add_page().expect("add_page");
    assert_eq!(idx, 1);
    let pages = s.doc.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 2);
    // The migrated "Page 1" keeps the root node.
    assert_eq!(pages[0].children.len(), 1);
    assert_eq!(pages[0].children[0].id_str(), "n1");
    // New page mirrors the TS blank-page default frame + active.
    assert_eq!(pages[1].children.len(), 1);
    assert_eq!(s.ui.active_page_index, 1);
}

#[test]
fn add_page_seeds_ts_blank_frame_geometry() {
    let mut s = state_with(vec![rect("n1", "A", 0.0, 0.0, 10.0, 10.0)]);
    let idx = s.add_page().expect("add_page");
    let pages = s.doc.pages.as_ref().unwrap();
    let child = pages[idx]
        .children
        .first()
        .expect("new page should contain the default frame");
    let PenNode::Frame(frame) = child else {
        panic!("new page should contain a frame, got {:?}", child);
    };

    assert_eq!(frame.base.name.as_deref(), Some("Frame"));
    assert_eq!(frame.base.x, Some(0.0));
    assert_eq!(frame.base.y, Some(0.0));
    assert!(matches!(
        frame.container.width,
        Some(jian_ops_schema::sizing::SizingBehavior::Number(1200.0))
    ));
    assert!(matches!(
        frame.container.height,
        Some(jian_ops_schema::sizing::SizingBehavior::Number(800.0))
    ));
    assert!(frame.container.stroke.is_none());
    assert!(matches!(
        frame.container.fill.as_deref(),
        Some([jian_ops_schema::style::PenFill::Solid(body)]) if body.color == "#FFFFFF"
    ));
    assert_eq!(frame.children.as_deref(), Some(&[][..]));
}

#[test]
fn set_active_page_switches_and_clears_selection() {
    let mut s = state_with(vec![rect("n1", "A", 0.0, 0.0, 10.0, 10.0)]);
    s.add_page();
    s.set_active_page(1);
    s.add_page(); // page 2, active
    assert!(s.set_active_page(0));
    assert_eq!(s.ui.active_page_index, 0);
    assert!(!s.set_active_page(99));
}

#[test]
fn rename_page_rejects_blank_and_writes_name() {
    let mut s = state_with(vec![]);
    s.add_page();
    assert!(!s.rename_page(0, "   "));
    assert!(s.rename_page(0, "Renamed"));
    assert_eq!(s.doc.pages.as_ref().unwrap()[0].name, "Renamed");
}

#[test]
fn duplicate_page_clones_children_with_fresh_ids() {
    let mut s = state_with(vec![rect("n1", "A", 0.0, 0.0, 10.0, 10.0)]);
    s.add_page(); // migrate + new page
    let idx = s.duplicate_page(0).expect("duplicate_page");
    assert_eq!(idx, 1);
    let pages = s.doc.pages.as_ref().unwrap();
    assert_eq!(pages.len(), 3);
    // Cloned page keeps one child but with a fresh id.
    assert_eq!(pages[1].children.len(), 1);
    assert_ne!(pages[1].children[0].id_str(), "n1");
    assert!(s.validate().is_ok());
}

#[test]
fn remove_page_keeps_at_least_one_page() {
    let mut s = state_with(vec![]);
    s.add_page(); // 2 pages now
    assert!(s.remove_page(1));
    assert_eq!(s.doc.pages.as_ref().unwrap().len(), 1);
    // Last page can't be removed.
    assert!(!s.remove_page(0));
}

#[test]
fn reorder_page_moves_and_tracks_active_index() {
    let mut s = state_with(vec![]);
    s.add_page();
    s.add_page(); // 3 pages, active = 2
    assert!(s.reorder_page(2, 0));
    assert_eq!(s.ui.active_page_index, 0);
}

// --- Clipboard -------------------------------------------------------

#[test]
fn copy_then_paste_clones_with_fresh_ids() {
    let mut s = state_with(vec![rect("n1", "A", 10.0, 10.0, 50.0, 50.0)]);
    s.set_single_selection(NodeId::new("n1"));
    assert!(s.copy_selected());
    let mut next_id = 1u64;
    let new_ids = s.paste_clipboard(&mut next_id, 8.0);
    assert_eq!(new_ids.len(), 1);
    assert_eq!(s.active_children().len(), 2);
    // Pasted node offset by 8 px.
    let pasted = find_node(s.active_children(), &new_ids[0]).unwrap();
    assert_eq!(pasted.base().x, Some(18.0));
    assert_eq!(s.selection.set, new_ids);
}

#[test]
fn cut_removes_selection_and_fills_clipboard() {
    let mut s = state_with(vec![
        rect("n1", "A", 0.0, 0.0, 10.0, 10.0),
        rect("n2", "B", 0.0, 0.0, 10.0, 10.0),
    ]);
    s.set_single_selection(NodeId::new("n1"));
    assert!(s.cut_selected());
    assert_eq!(s.active_children().len(), 1);
    assert!(!s.clipboard.is_empty());
}

#[test]
fn cut_is_atomic_when_delete_is_refused() {
    let mut locked = rect("n1", "locked", 0.0, 0.0, 10.0, 10.0);
    locked.base_mut().locked = Some(true);
    let mut s = state_with(vec![locked]);
    // Pre-fill clipboard so we can prove it isn't clobbered.
    s.clipboard = vec![rect("keep", "K", 0.0, 0.0, 1.0, 1.0)];
    s.set_single_selection(NodeId::new("n1"));
    assert!(!s.cut_selected());
    // A failed cut restores the prior clipboard.
    assert_eq!(s.clipboard.len(), 1);
    assert_eq!(s.clipboard[0].id_str(), "keep");
}

#[test]
fn paste_empty_clipboard_is_noop() {
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    assert!(s.paste_clipboard(&mut next_id, 8.0).is_empty());
}

// --- Pen tool --------------------------------------------------------

#[test]
fn pen_path_with_two_anchors_commits_and_pushes_history() {
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    let id = s.start_pen_path(&mut next_id, (10.0, 10.0)).expect("start");
    assert!(s.add_pen_point((50.0, 30.0)));
    assert!(s.finish_pen_path());
    // Two-anchor path stays + history captured the pre-pen state.
    assert!(find_node(s.active_children(), &id).is_some());
    assert!(s.history.can_undo());
}

#[test]
fn pen_path_with_one_anchor_is_stripped_without_history() {
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    let id = s.start_pen_path(&mut next_id, (10.0, 10.0)).expect("start");
    assert!(s.finish_pen_path());
    // Lone-anchor path is invisible — removed, no undo entry.
    assert!(find_node(s.active_children(), &id).is_none());
    assert!(!s.history.can_undo());
}

#[test]
fn add_pen_point_refits_path_bounds() {
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    let id = s.start_pen_path(&mut next_id, (0.0, 0.0)).expect("start");
    s.add_pen_point((100.0, 60.0));
    let node = find_node(s.active_children(), &id).unwrap();
    assert_eq!(node.width_px(), Some(100.0));
    assert_eq!(node.height_px(), Some(60.0));
}

#[test]
fn set_path_anchor_position_moves_one_anchor() {
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    let id = s.start_pen_path(&mut next_id, (0.0, 0.0)).expect("start");
    s.add_pen_point((50.0, 30.0));
    s.add_pen_point((100.0, 0.0));
    s.finish_pen_path();
    assert!(s.set_path_anchor_position(id.clone(), 1, (50.0, 90.0)));
    let node = find_node(s.active_children(), &id).unwrap();
    if let PenNode::Path(p) = node {
        let a = &p.anchors.as_ref().unwrap()[1];
        assert_eq!((a.x, a.y), (50.0, 90.0));
    } else {
        panic!("expected Path");
    }
    // Bounds re-fit: y range now 0..90.
    assert_eq!(node.height_px(), Some(90.0));
}

#[test]
fn set_path_anchor_position_rejects_out_of_range() {
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    let id = s.start_pen_path(&mut next_id, (0.0, 0.0)).expect("start");
    s.add_pen_point((10.0, 10.0));
    s.finish_pen_path();
    assert!(!s.set_path_anchor_position(id, 99, (0.0, 0.0)));
}

#[test]
fn set_path_anchor_handle_writes_and_clears() {
    use crate::pen::PathHandleSide;
    use jian_ops_schema::node::PenNode;
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    let id = s.start_pen_path(&mut next_id, (0.0, 0.0)).expect("start");
    s.add_pen_point((100.0, 0.0));
    s.finish_pen_path();
    // Set the outgoing handle on anchor 0.
    assert!(s.set_path_anchor_handle(id.clone(), 0, PathHandleSide::Out, Some((20.0, 10.0))));
    if let Some(PenNode::Path(p)) = find_node(s.active_children(), &id) {
        let h = p.anchors.as_ref().unwrap()[0].handle_out.as_ref().unwrap();
        assert_eq!((h.x, h.y), (20.0, 10.0));
    } else {
        panic!("expected path");
    }
    // Clearing it sets the handle back to None.
    assert!(s.set_path_anchor_handle(id.clone(), 0, PathHandleSide::Out, None));
    if let Some(PenNode::Path(p)) = find_node(s.active_children(), &id) {
        assert!(p.anchors.as_ref().unwrap()[0].handle_out.is_none());
    }
}

#[test]
fn mirrored_point_type_mirrors_the_opposite_handle() {
    use crate::pen::PathHandleSide;
    use jian_ops_schema::node::{PenNode, PenPathPointType};
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    let id = s.start_pen_path(&mut next_id, (0.0, 0.0)).expect("start");
    s.add_pen_point((100.0, 0.0));
    s.finish_pen_path();
    s.set_path_anchor_point_type(id.clone(), 0, PenPathPointType::Mirrored);
    // Dragging the outgoing handle mirrors the incoming one.
    s.set_path_anchor_handle(id.clone(), 0, PathHandleSide::Out, Some((30.0, 12.0)));
    if let Some(PenNode::Path(p)) = find_node(s.active_children(), &id) {
        let a = &p.anchors.as_ref().unwrap()[0];
        let hin = a.handle_in.as_ref().unwrap();
        assert_eq!((hin.x, hin.y), (-30.0, -12.0));
    } else {
        panic!("expected path");
    }
}

#[test]
fn set_path_anchor_handle_rejects_bad_index() {
    use crate::pen::PathHandleSide;
    let mut s = state_with(vec![]);
    let mut next_id = 1u64;
    let id = s.start_pen_path(&mut next_id, (0.0, 0.0)).expect("start");
    s.add_pen_point((10.0, 10.0));
    s.finish_pen_path();
    assert!(!s.set_path_anchor_handle(id, 99, PathHandleSide::In, Some((1.0, 1.0))));
}
