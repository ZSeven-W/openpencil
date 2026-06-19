// Navigation tests — pure math, no `canvaskit` feature required.

#[test]
fn zoom_to_fit_sets_nonzero_zoom_for_loaded_doc() {
    let mut v = super::Viewer::placeholder();
    v.load(r#"{"version":"1.0","pages":[{"id":"p","name":"P","children":[{"type":"rectangle","id":"r","x":0,"y":0,"width":100,"height":50}]}]}"#).unwrap();
    v.rebuild_scene();
    v.zoom_to_fit(800.0, 600.0);
    let (_, _, zoom) = v.viewport();
    assert!(zoom > 0.0);
}

#[test]
fn set_viewport_round_trips() {
    let mut v = super::Viewer::placeholder();
    v.set_viewport(10.0, -20.0, 2.5);
    let (px, py, z) = v.viewport();
    assert!((px - 10.0).abs() < 1e-5);
    assert!((py + 20.0).abs() < 1e-5);
    assert!((z - 2.5).abs() < 1e-5);
}

#[test]
fn zoom_to_fit_noop_when_no_scene() {
    let mut v = super::Viewer::placeholder();
    // No doc loaded — should not panic and zoom stays at 1.0 (identity).
    v.zoom_to_fit(800.0, 600.0);
    let (_, _, zoom) = v.viewport();
    assert!((zoom - 1.0).abs() < 1e-5);
}

/// Regression test: set_viewport must be visible via viewport() after the call.
/// The push_viewport path is canvaskit-gated; without canvaskit the push_viewport
/// stub is a no-op, but the Viewer's own viewport field is always updated.
#[test]
fn set_viewport_reflected_by_accessor() {
    let mut v = super::Viewer::placeholder();
    v.set_viewport(50.0, 75.0, 1.5);
    let (px, py, z) = v.viewport();
    assert!((px - 50.0).abs() < 1e-5, "pan_x must be reflected");
    assert!((py - 75.0).abs() < 1e-5, "pan_y must be reflected");
    assert!((z - 1.5).abs() < 1e-5, "zoom must be reflected");
    // Note: the push to RenderInner.viewport (so the RAF pump renders the new
    // viewport) is tested via the canvaskit integration path; here we verify
    // that the Viewer-side state is updated and the push path does not panic.
}

/// Finding (3): zoom_to_fit must be a no-op (no panic, identity viewport)
/// when the loaded document's only node has zero size.
#[test]
fn zoom_to_fit_noop_for_zero_size_content() {
    let mut v = super::Viewer::placeholder();
    // Node with 0×0 size — content_bounds yields zero-area AABB.
    v.load(r#"{"version":"1.0","pages":[{"id":"p","name":"P","children":[{"type":"rectangle","id":"r","x":0,"y":0,"width":0,"height":0}]}]}"#).unwrap();
    v.rebuild_scene();
    let (pan_x_before, pan_y_before, zoom_before) = v.viewport();
    v.zoom_to_fit(800.0, 600.0);
    let (pan_x_after, pan_y_after, zoom_after) = v.viewport();
    // Viewport must be unchanged when content has no positive area.
    assert!(
        (pan_x_after - pan_x_before).abs() < 1e-5,
        "pan_x must not change for zero-size content"
    );
    assert!(
        (pan_y_after - pan_y_before).abs() < 1e-5,
        "pan_y must not change for zero-size content"
    );
    assert!(
        (zoom_after - zoom_before).abs() < 1e-5,
        "zoom must not change for zero-size content"
    );
}
