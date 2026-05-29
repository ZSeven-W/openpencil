//! Drag/release tests split from `input_tests.rs` so each test module
//! stays under the repository file-size ceiling.

use super::{NodeDragState, WidgetHostNative};
use op_editor_core::{NodeId, PenNodeExt};

/// Seed a host's `editor_state` from a canonical `.op` JSON snippet.
fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

/// Three top-level rect nodes at the given `(x, y, w, h)` boxes.
fn three_rects(boxes: [(f64, f64, f64, f64); 3], ids: [&str; 3]) -> String {
    let node = |id: &str, b: (f64, f64, f64, f64)| {
        format!(
            r#"{{"type":"rectangle","id":"{id}","name":"{id}",
               "x":{},"y":{},"width":{},"height":{}}}"#,
            b.0, b.1, b.2, b.3
        )
    };
    format!(
        r#"{{"version":"0.8.0","children":[{},{},{}]}}"#,
        node(ids[0], boxes[0]),
        node(ids[1], boxes[1]),
        node(ids[2], boxes[2]),
    )
}

#[test]
fn anchor_press_release_without_motion_does_not_push_history() {
    // Codex CONCERN: a press-release on an anchor without any
    // cursor motion must NOT pollute the undo stack.
    use op_editor_ui::Point2D;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"path","id":"n60","name":"p","x":0,"y":0,
           "anchors":[{"x":0,"y":0},{"x":50,"y":25}]}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    let snap = host.editor_state().snapshot_for_history();
    host.path_anchor_drag = Some(crate::widget_host::PathAnchorDragState {
        node_id: NodeId::new("n60"),
        anchor_index: 1,
        target: crate::widget_host::AnchorDragTarget::Anchor,
        anchor_doc: Point2D::new(50.0, 25.0),
        start_doc: Point2D::new(50.0, 25.0),
        shift: false,
        moved: false,
        pre_drag_snapshot: snap,
    });
    let history_before = host.editor_state().history.past.len();
    let consumed = host.apply_release_with_viewport(1440.0, 900.0);
    assert!(host.path_anchor_drag.is_none(), "drag state cleared");
    assert!(!consumed, "release with no motion is not a UI change");
    assert_eq!(
        host.editor_state().history.past.len(),
        history_before,
        "no-motion press-release must not push a history entry"
    );
}

#[test]
fn anchor_drag_back_to_start_lands_at_start() {
    // Codex BLOCK: dragging away and back must write the final
    // position — the anchor must follow the cursor home.
    use op_editor_ui::Point2D;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"path","id":"n60","name":"p","x":0,"y":0,
           "anchors":[{"x":0,"y":0},{"x":50,"y":25}]}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    host.editor_state_mut().tool = op_editor_core::Tool::Pen;
    let snap = host.editor_state().snapshot_for_history();
    host.path_anchor_drag = Some(crate::widget_host::PathAnchorDragState {
        node_id: NodeId::new("n60"),
        anchor_index: 1,
        target: crate::widget_host::AnchorDragTarget::Anchor,
        anchor_doc: Point2D::new(50.0, 25.0),
        start_doc: Point2D::new(50.0, 25.0),
        shift: false,
        moved: false,
        pre_drag_snapshot: snap,
    });
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    // Drag away to doc (80, 25).
    host.apply_cursor_move(cx0 + 80.0, cy0 + 25.0);
    let after_first = anchor_at(&host, "n60", 1);
    assert!((after_first.0 - 80.0).abs() < 0.5);
    // Drag BACK to start (50, 25).
    host.apply_cursor_move(cx0 + 50.0, cy0 + 25.0);
    let after_return = anchor_at(&host, "n60", 1);
    assert!(
        (after_return.0 - 50.0).abs() < 0.5,
        "anchor must follow cursor back to start; got {after_return:?}"
    );
}

#[test]
fn anchor_drag_with_motion_pushes_one_history_entry() {
    use op_editor_ui::Point2D;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"path","id":"n60","name":"p","x":0,"y":0,
           "anchors":[{"x":0,"y":0},{"x":50,"y":25}]}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n60"));
    let snap = host.editor_state().snapshot_for_history();
    host.path_anchor_drag = Some(crate::widget_host::PathAnchorDragState {
        node_id: NodeId::new("n60"),
        anchor_index: 1,
        target: crate::widget_host::AnchorDragTarget::Anchor,
        anchor_doc: Point2D::new(50.0, 25.0),
        start_doc: Point2D::new(50.0, 25.0),
        shift: false,
        moved: true,
        pre_drag_snapshot: snap,
    });
    let history_before = host.editor_state().history.past.len();
    let consumed = host.apply_release_with_viewport(1440.0, 900.0);
    assert!(consumed, "release after motion is a UI change");
    assert_eq!(
        host.editor_state().history.past.len(),
        history_before + 1,
        "exactly one history entry per drag"
    );
}

#[test]
fn node_drag_not_intercepted_by_align_toolbar_hover() {
    // Codex CONCERN: with 2+ selected, an active node-drag must
    // keep moving the nodes when the cursor sweeps the align
    // toolbar's hit region.
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (50.0, 200.0, 20.0, 20.0),
                (120.0, 200.0, 20.0, 20.0),
                (9000.0, 9000.0, 10.0, 10.0),
            ],
            ["n90", "n91", "n92"],
        ),
    );
    // Two-node selection so the align toolbar is shown.
    host.editor_state_mut().selection.set = vec![NodeId::new("n90"), NodeId::new("n91")];
    host.editor_state_mut().selection.anchor = NodeId::new("n91");
    host.mark_paint_dirty_for_test();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    let press_x = cx0 + 60.0;
    let press_y = cy0 + 210.0;
    host.apply_press(press_x, press_y, viewport_w, viewport_h);
    assert!(host.node_drag.is_some(), "node_drag must seed on press");
    let zoom = host.editor_state().viewport.zoom.max(0.0001);
    let target_x = host.editor_state().editor_ui.layer_panel_width + 400.0;
    let target_y = TOP_BAR_HEIGHT + 24.0;
    let expected_dx = (target_x - press_x) / zoom;
    let expected_dy = (target_y - press_y) / zoom;
    host.apply_cursor_move(target_x, target_y);
    // Nodes must have translated by (expected_dx, expected_dy).
    let a = node_xy(&host, "n90");
    let b = node_xy(&host, "n91");
    assert!(
        (a.0 - (50.0 + expected_dx as f64)).abs() < 0.5
            && (a.1 - (200.0 + expected_dy as f64)).abs() < 0.5,
        "node-drag delta lost on a; got {a:?}",
    );
    assert!(
        (b.0 - (120.0 + expected_dx as f64)).abs() < 0.5,
        "node-drag delta lost on b; got {b:?}",
    );
}

#[test]
fn node_drag_snap_does_not_trap_incremental_cursor_motion() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"0.8.0","children":[
          {"type":"rectangle","id":"moving","name":"moving","x":90,"y":0,"width":10,"height":10},
          {"type":"rectangle","id":"guide","name":"guide","x":105,"y":100,"width":100,"height":20}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("moving"));
    host.editor_state_mut().viewport.zoom = 1.0;
    host.node_drag = Some(NodeDragState {
        last_screen_x: 500.0,
        last_screen_y: 500.0,
    });

    for x in (502..=522).step_by(2) {
        host.apply_cursor_move(x as f32, 500.0);
    }

    let moved = node_xy(&host, "moving");
    assert!(
        moved.0 > 110.0,
        "small cursor moves must accumulate enough to leave a smart-guide snap; got {moved:?}"
    );
}

#[test]
fn host_carries_editor_state_as_source_of_truth() {
    // A fresh host opens with the demo sample seeded onto
    // `EditorState` — the host's single source of truth.
    let host = WidgetHostNative::new();
    assert!(!host.editor_state().doc.children.is_empty());
    assert!(!host.editor_state().selection.is_empty());
}

#[test]
fn editor_state_is_mutable_through_the_accessor() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().tool = op_editor_core::Tool::Rect;
    assert_eq!(host.editor_state().tool, op_editor_core::Tool::Rect);
}

/// Read a node's `(x, y)` from the host's `editor_state.doc`.
fn node_xy(host: &WidgetHostNative, id: &str) -> (f64, f64) {
    let n = host
        .editor_state()
        .doc
        .children
        .iter()
        .find(|n| n.base().id == id)
        .expect("node present");
    (n.base().x.unwrap_or(0.0), n.base().y.unwrap_or(0.0))
}

/// Read a path anchor's `(x, y)` from the host's `editor_state.doc`.
fn anchor_at(host: &WidgetHostNative, id: &str, idx: usize) -> (f64, f64) {
    let n = host
        .editor_state()
        .doc
        .children
        .iter()
        .find(|n| n.base().id == id)
        .expect("node present");
    match n {
        jian_ops_schema::node::PenNode::Path(p) => {
            let a = &p.anchors.as_ref().expect("anchors")[idx];
            (a.x, a.y)
        }
        _ => panic!("not a path node"),
    }
}
