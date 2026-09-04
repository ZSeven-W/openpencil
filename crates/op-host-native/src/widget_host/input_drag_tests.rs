//! Drag/release tests split from `input_tests.rs` so each test module
//! stays under the repository file-size ceiling.

use super::{CreateDragState, HandleDragState, NodeDragState, RotateDragState, WidgetHostNative};
use op_editor_core::{NodeId, PenNodeExt};
use op_editor_ui::{widgets::SelectionHandle, Point2D, Rect};

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
        r#"{{"version":"1.0.0","children":[{},{},{}]}}"#,
        node(ids[0], boxes[0]),
        node(ids[1], boxes[1]),
        node(ids[2], boxes[2]),
    )
}

#[test]
fn bottom_right_handle_resizes_only_selected_container() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{
          "type":"frame","id":"frame","name":"frame","x":100,"y":80,
          "width":200,"height":160,"layout":"none","children":[
            {"type":"rectangle","id":"child","name":"child","x":12,"y":18,
             "width":60,"height":30}
          ]
        }]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("frame"));
    let child_before = authored_geometry(&host, "child");
    host.handle_drag = Some(HandleDragState {
        handle: SelectionHandle::BottomRight,
        start_screen_x: 500.0,
        start_screen_y: 500.0,
        start_bounds: Rect {
            origin: Point2D::new(100.0, 80.0),
            size: Point2D::new(200.0, 160.0),
        },
        start_authored_x: Some(100.0),
        start_authored_y: Some(80.0),
    });

    assert!(host.apply_cursor_move(540.0, 525.0));

    assert_eq!(
        authored_geometry(&host, "frame"),
        (Some(100.0), Some(80.0), Some(240.0), Some(185.0))
    );
    assert_eq!(
        authored_geometry(&host, "child"),
        child_before,
        "normal container resize must not scale or translate fixed descendants"
    );
}

#[test]
fn consecutive_resize_frames_keep_layout_scene_in_sync() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{
          "type":"rectangle","id":"shape","name":"shape","x":100,"y":80,
          "width":120,"height":80
        }]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("shape"));
    host.mark_paint_dirty_for_test();
    let _ = host.layout_scene();
    host.editor_state_mut().commit_history();
    host.handle_drag = Some(HandleDragState {
        handle: SelectionHandle::Right,
        start_screen_x: 500.0,
        start_screen_y: 500.0,
        start_bounds: Rect::xywh(100.0, 80.0, 120.0, 80.0),
        start_authored_x: Some(100.0),
        start_authored_y: Some(80.0),
    });

    for (cursor_x, expected_width) in [(520.0, 140.0), (540.0, 160.0)] {
        assert!(host.apply_cursor_move(cursor_x, 500.0));
        assert_eq!(authored_geometry(&host, "shape").2, Some(expected_width));
        let scene_width = host
            .layout_scene()
            .active_page()
            .and_then(|page| page.find("shape"))
            .expect("scene shape")
            .bounds
            .size
            .x;
        assert_eq!(scene_width, expected_width as f32);
    }
}

#[test]
fn consecutive_rotation_frames_keep_layout_scene_in_sync() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{
          "type":"rectangle","id":"shape","name":"shape","x":100,"y":80,
          "width":120,"height":80
        }]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("shape"));
    let _ = host.layout_scene();
    host.editor_state_mut().commit_history();
    host.rotate_drag = Some(RotateDragState {
        center_screen_x: 500.0,
        center_screen_y: 500.0,
        start_cursor_angle: 0.0,
        start_rotation: 0.0,
    });

    for (cursor, expected) in [
        ((500.0, 510.0), std::f32::consts::FRAC_PI_2),
        ((490.0, 500.0), std::f32::consts::PI),
    ] {
        assert!(host.apply_cursor_move(cursor.0, cursor.1));
        let scene_rotation = host
            .layout_scene()
            .active_page()
            .and_then(|page| page.find("shape"))
            .expect("scene shape")
            .rotation;
        assert!((scene_rotation - expected).abs() < 0.0001);
    }
}

#[test]
fn consecutive_create_frames_keep_layout_scene_in_sync() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().tool = op_editor_core::Tool::Rect;
    let start = Point2D::new(700.0, 400.0);
    let id = host
        .create_node_for_active_tool(start)
        .expect("rectangle tool creates a node");
    host.editor_state_mut().set_single_selection(id.clone());
    host.create_drag = Some(CreateDragState {
        start_doc_x: start.x,
        start_doc_y: start.y,
    });
    let (canvas_x, canvas_y) =
        op_editor_ui::widgets::host_canvas_geometry::canvas_origin(host.editor_state());

    for (doc_cursor, expected_size) in [
        (Point2D::new(740.0, 440.0), Point2D::new(40.0, 40.0)),
        (Point2D::new(760.0, 460.0), Point2D::new(60.0, 60.0)),
    ] {
        assert!(host.apply_cursor_move(canvas_x + doc_cursor.x, canvas_y + doc_cursor.y));
        let scene_bounds = host
            .layout_scene()
            .active_page()
            .and_then(|page| page.find(id.as_str()))
            .expect("created scene shape")
            .bounds;
        assert_eq!(scene_bounds.size, expected_size);
    }
}

#[test]
fn geometry_drags_enable_canvas_fast_paint_without_enabling_pan_cache() {
    let mut host = WidgetHostNative::new();
    assert!(!host.fast_interaction_active());
    assert!(!host.canvas_fast_interaction_active());
    host.handle_drag = Some(HandleDragState {
        handle: SelectionHandle::Right,
        start_screen_x: 0.0,
        start_screen_y: 0.0,
        start_bounds: Rect::xywh(0.0, 0.0, 100.0, 100.0),
        start_authored_x: Some(0.0),
        start_authored_y: Some(0.0),
    });

    assert!(host.canvas_fast_interaction_active());
    assert!(!host.fast_interaction_active());
}

#[test]
fn edge_handle_freezes_only_its_axis_and_preserves_descendants() {
    for (handle, move_to, expected_width, expected_height) in [
        (
            SelectionHandle::Right,
            Point2D::new(550.0, 500.0),
            serde_json::json!(250.0),
            serde_json::json!("fit_content"),
        ),
        (
            SelectionHandle::Bottom,
            Point2D::new(500.0, 540.0),
            serde_json::json!("fill_container"),
            serde_json::json!(120.0),
        ),
    ] {
        let mut host = WidgetHostNative::new();
        seed(
            &mut host,
            r#"{"version":"1.0.0","children":[{
              "type":"frame","id":"frame","name":"frame","x":100,"y":80,
              "width":"fill_container","height":"fit_content","layout":"vertical",
              "children":[
                {"type":"rectangle","id":"child","name":"child","x":12,"y":18,
                 "width":60,"height":30}
              ]
            }]}"#,
        );
        host.editor_state_mut()
            .set_single_selection(NodeId::new("frame"));
        let child_before = authored_geometry(&host, "child");
        host.handle_drag = Some(HandleDragState {
            handle,
            start_screen_x: 500.0,
            start_screen_y: 500.0,
            start_bounds: Rect {
                origin: Point2D::new(100.0, 80.0),
                size: Point2D::new(200.0, 80.0),
            },
            start_authored_x: Some(100.0),
            start_authored_y: Some(80.0),
        });

        assert!(host.apply_cursor_move(move_to.x, move_to.y));

        let frame = op_editor_core::walkers::find_node(
            host.editor_state().active_children(),
            &NodeId::new("frame"),
        )
        .expect("frame present");
        let frame_json = serde_json::to_value(frame).expect("frame serializes");
        assert_eq!(frame_json["width"], expected_width, "handle: {handle:?}");
        assert_eq!(frame_json["height"], expected_height, "handle: {handle:?}");
        assert_eq!(
            authored_geometry(&host, "child"),
            child_before,
            "edge resize must leave fixed descendant geometry untouched; handle: {handle:?}"
        );
    }
}

#[test]
fn fill_child_reflows_to_resized_parent_without_authored_mutation() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{
          "type":"frame","id":"frame","name":"frame","x":100,"y":80,
          "width":200,"height":160,"layout":"vertical","children":[
            {"type":"rectangle","id":"child","name":"child",
             "width":"fill_container","height":30}
          ]
        }]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("frame"));
    host.handle_drag = Some(HandleDragState {
        handle: SelectionHandle::Right,
        start_screen_x: 500.0,
        start_screen_y: 500.0,
        start_bounds: Rect {
            origin: Point2D::new(100.0, 80.0),
            size: Point2D::new(200.0, 160.0),
        },
        start_authored_x: Some(100.0),
        start_authored_y: Some(80.0),
    });

    assert!(host.apply_cursor_move(560.0, 500.0));
    host.refresh_layout_scene();

    let child = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("child"),
    )
    .expect("child present");
    assert_eq!(child.width_px(), None, "Fill keyword must stay authored");
    let resolved = host
        .layout_scene
        .active_page()
        .and_then(|page| page.find("child"))
        .expect("resolved child present");
    assert_eq!(resolved.bounds.size.x, 260.0);
    assert_eq!(resolved.bounds.size.y, 30.0);
}

#[test]
fn resize_drag_updates_the_scene_the_paint_reads() {
    // The whole point of a live drag: every cursor move must leave a scene
    // that shows the new size. The press paints one frame first, so by the
    // time the moves arrive the scene cache already holds a settled build —
    // a mutator that does not record a document change would be skipped by
    // it and the drag would only appear once some later edit forced a
    // rebuild.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{
          "type":"frame","id":"frame","name":"frame","x":100,"y":80,
          "width":200,"height":160,"layout":"none","children":[
            {"type":"rectangle","id":"child","name":"child","x":12,"y":18,
             "width":60,"height":30}
          ]
        }]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("frame"));
    // Stand in for the press: `commit_history` records a change, then the
    // frame that follows settles the scene cache on it.
    host.editor_state_mut().commit_history();
    let settled = scene_size(&mut host, "frame");
    assert_eq!(settled, (200.0, 160.0), "pre-drag scene");

    host.handle_drag = Some(HandleDragState {
        handle: SelectionHandle::BottomRight,
        start_screen_x: 500.0,
        start_screen_y: 500.0,
        start_bounds: Rect {
            origin: Point2D::new(100.0, 80.0),
            size: Point2D::new(200.0, 160.0),
        },
        start_authored_x: Some(100.0),
        start_authored_y: Some(80.0),
    });

    assert!(host.apply_cursor_move(540.0, 525.0));
    assert_eq!(
        scene_size(&mut host, "frame"),
        (240.0, 185.0),
        "the resized bounds must reach the scene on the drag frame itself"
    );
}

/// Resolved `(width, height)` of `id` in the scene the paint pass reads —
/// via `layout_scene()`, so the cache gets the same chance to skip that it
/// gets in the real paint.
fn scene_size(host: &mut WidgetHostNative, id: &str) -> (f32, f32) {
    let n = host
        .layout_scene()
        .active_page()
        .and_then(|p| p.find(id))
        .expect("scene node present");
    (n.bounds.size.x, n.bounds.size.y)
}

#[test]
fn anchor_press_release_without_motion_does_not_push_history() {
    // Codex CONCERN: a press-release on an anchor without any
    // cursor motion must NOT pollute the undo stack.
    use op_editor_ui::Point2D;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
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
        grab_offset: None,
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
        r#"{"version":"1.0.0","children":[
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
        grab_offset: None,
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
        r#"{"version":"1.0.0","children":[
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
        grab_offset: None,
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
        r#"{"version":"1.0.0","children":[
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
        press_screen_x: 500.0,
        press_screen_y: 500.0,
        // This test exercises smart-guide accumulation on an already-
        // in-progress drag, not the press-time threshold gate.
        moved: true,
        total_dx: 0.0,
        total_dy: 0.0,
        overlay_bounds: None,
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
fn node_drag_skips_smart_guides_for_large_documents() {
    let mut nodes = vec![
        r#"{"type":"rectangle","id":"moving","name":"moving","x":90,"y":0,"width":10,"height":10}"#
            .to_string(),
        r#"{"type":"rectangle","id":"guide","name":"guide","x":105,"y":100,"width":100,"height":20}"#
            .to_string(),
    ];
    for i in 0..1000 {
        nodes.push(format!(
            r#"{{"type":"rectangle","id":"filler-{i}","name":"filler-{i}","x":{},"y":{},"width":8,"height":8}}"#,
            1000 + i * 10,
            1000 + i * 10
        ));
    }
    let doc = format!(r#"{{"version":"1.0.0","children":[{}]}}"#, nodes.join(","));
    let mut host = WidgetHostNative::new();
    seed(&mut host, &doc);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("moving"));
    host.editor_state_mut().viewport.zoom = 1.0;
    host.node_drag = Some(NodeDragState {
        last_screen_x: 500.0,
        last_screen_y: 500.0,
        press_screen_x: 500.0,
        press_screen_y: 500.0,
        moved: true,
        total_dx: 0.0,
        total_dy: 0.0,
        overlay_bounds: None,
    });

    assert!(host.apply_cursor_move(502.0, 500.0));

    let moved = node_xy(&host, "moving");
    assert_eq!(
        moved.0, 92.0,
        "large documents should prioritize direct cursor tracking over smart-guide snap"
    );
    assert!(host.editor_state().editor_ui.active_guides.is_empty());
}

#[test]
fn node_drag_keeps_flex_child_in_layout_flow() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{
          "type":"frame","id":"frame","name":"frame","x":100,"y":100,
          "width":200,"height":200,"layout":"vertical","gap":8,
          "children":[
            {"type":"rectangle","id":"child","name":"child","width":80,"height":24}
          ]
        }]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("child"));
    host.mark_paint_dirty_for_test();
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);
    let scene_before = scene_node_xy(&host, "child");
    host.node_drag = Some(NodeDragState {
        last_screen_x: 500.0,
        last_screen_y: 500.0,
        press_screen_x: 500.0,
        press_screen_y: 500.0,
        moved: true,
        total_dx: 0.0,
        total_dy: 0.0,
        overlay_bounds: None,
    });

    assert!(host.apply_cursor_move(520.0, 500.0));

    let child = op_editor_core::walkers::find_node(
        host.editor_state().active_children(),
        &NodeId::new("child"),
    )
    .expect("child node");
    assert_eq!(child.base().x, None, "flex child must not materialize x");
    assert_eq!(child.base().y, None, "flex child must not materialize y");
    assert_eq!(
        scene_node_xy(&host, "child"),
        scene_before,
        "dragging a flex child should not patch the layout scene away from flow"
    );
}

mod scene_sync;

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

fn scene_node_xy(host: &WidgetHostNative, id: &str) -> (f32, f32) {
    let n = host
        .layout_scene
        .active_page()
        .and_then(|p| p.find(id))
        .expect("scene node present");
    (n.bounds.origin.x, n.bounds.origin.y)
}

fn authored_geometry(
    host: &WidgetHostNative,
    id: &str,
) -> (Option<f64>, Option<f64>, Option<f64>, Option<f64>) {
    let node =
        op_editor_core::walkers::find_node(host.editor_state().active_children(), &NodeId::new(id))
            .expect("node present");
    (
        node.base().x,
        node.base().y,
        node.width_px(),
        node.height_px(),
    )
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
