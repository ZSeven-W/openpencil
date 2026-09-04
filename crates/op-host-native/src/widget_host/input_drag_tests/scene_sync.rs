use super::*;

#[test]
fn dragging_a_selection_with_a_locked_node_does_not_drift_it_in_the_scene() {
    // Parity with the web host: the incremental scene patch must move exactly
    // what `translate_selected` moved — editable nodes only. A selected locked
    // node must not drift in the scene (it would otherwise jump during the drag
    // and snap back on the release-time reconversion).
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"free","name":"free","x":100,"y":100,"width":80,"height":60},
          {"type":"rectangle","id":"locked","name":"locked","x":600,"y":100,"width":80,"height":60,"locked":true}
        ]}"#,
    );
    host.editor_state_mut().selection.set = vec![NodeId::new("free"), NodeId::new("locked")];
    host.mark_paint_dirty_for_test();
    let _ = host.layout_scene();
    assert!(!host.editor_state_dirty);

    let free_before = scene_node_xy(&host, "free");
    let locked_before = scene_node_xy(&host, "locked");

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
    assert!(host.apply_cursor_move(540.0, 500.0));

    let free_after = scene_node_xy(&host, "free");
    let locked_after = scene_node_xy(&host, "locked");

    assert!(
        (free_after.0 - free_before.0).abs() > 1.0,
        "editable node should move in the scene during the drag"
    );
    assert_eq!(
        locked_after, locked_before,
        "locked node must not drift in the scene"
    );
}

#[test]
fn incremental_drag_then_doc_restored_to_cached_value_rebuilds_the_scene() {
    // The incremental drag patches `layout_scene` without updating the scene
    // cache. If the doc later returns to the cached build's value (e.g. undo, or
    // dirty flipping mid-drag), `maybe_rebuild` must NOT skip and leave the stale
    // patch on screen — the incremental path invalidates the cache to prevent it.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"rectangle","id":"free","name":"free","x":100,"y":100,"width":80,"height":60}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("free"));
    host.mark_paint_dirty_for_test();
    let _ = host.layout_scene(); // caches the build for the @100 doc
    let before = scene_node_xy(&host, "free");
    let cached_doc = host.editor_state().doc.clone();

    // Incremental drag: patches the scene (and invalidates the cache).
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
    assert!(host.apply_cursor_move(540.0, 500.0));
    let patched = scene_node_xy(&host, "free");
    assert!(
        (patched.0 - before.0).abs() > 1.0,
        "incremental patch moved the scene"
    );

    // Restore the doc to EXACTLY the cached build's inputs, then refresh.
    host.editor_state_mut().doc = cached_doc;
    host.mark_paint_dirty_for_test();
    let _ = host.layout_scene();
    let after = scene_node_xy(&host, "free");
    assert!(
        (after.0 - before.0).abs() < 1.0,
        "restoring the doc to the cached value must rebuild, not serve the stale patch"
    );
}

#[test]
fn host_carries_editor_state_as_source_of_truth() {
    // A fresh host opens with the blank starter document seeded onto
    // `EditorState` — the host's single source of truth.
    let host = WidgetHostNative::new();
    assert!(!host.editor_state().doc.children.is_empty());
    assert!(host.editor_state().selection.is_empty());
}

#[test]
fn editor_state_is_mutable_through_the_accessor() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().tool = op_editor_core::Tool::Rect;
    assert_eq!(host.editor_state().tool, op_editor_core::Tool::Rect);
}
