//! `EditorCommand::RefineDesign` tests.

#![cfg(test)]

use crate::command::EditorCommand;
use crate::node_id::NodeId;
use crate::pen_node_ext::PenNodeExt;
use crate::test_support::{flex_frame, rect, state_with};
use crate::walkers::find_node;

fn id(s: &str) -> NodeId {
    NodeId::new(s)
}

#[test]
fn refine_design_sanitizes_auto_layout_child_positions() {
    let mut s = state_with(vec![flex_frame(
        "root",
        "Root",
        0.0,
        0.0,
        375.0,
        100.0,
        vec![rect("child", "Child", 24.0, 32.0, 120.0, 40.0)],
    )]);

    assert!(s.apply(EditorCommand::RefineDesign {
        root_id: id("root"),
        canvas_width: Some(375),
        page_id: None,
    }));

    let child = find_node(s.active_children(), &id("child")).expect("child remains");
    assert_eq!(child.base().x, None);
    assert_eq!(child.base().y, None);
}
