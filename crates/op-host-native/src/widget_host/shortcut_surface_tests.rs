//! `#[cfg(test)]` companion for the shortcut-surface host methods
//! added for TS parity (panel toggles, create-component, space-pan).

use super::WidgetHostNative;
use op_editor_core::NodeId;

fn seed(host: &mut WidgetHostNative, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.mark_paint_dirty_for_test();
}

const ONE_RECT: &str = r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":400,"y":300,"width":100,"height":50}]}"#;

const ONE_FRAME: &str = r#"{"version":"1.0.0","children":[{"type":"frame","id":"f1","name":"Card","x":0,"y":0,"width":100,"height":50}]}"#;

#[test]
fn panel_toggle_shortcuts_flip_their_flags() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);

    assert!(host.apply_toggle_variables_panel());
    assert!(host.editor_state().editor_ui.variables_panel_open);
    assert!(host.apply_toggle_variables_panel());
    assert!(!host.editor_state().editor_ui.variables_panel_open);

    assert!(host.apply_toggle_design_md_panel());
    assert!(host.editor_state().editor_ui.design_md_panel_open);

    assert!(host.apply_toggle_component_browser());
    assert!(host.editor_state().editor_ui.component_browser_open);

    assert!(host.apply_open_figma_import());
    assert!(host.editor_state().editor_ui.figma_import_open);
}

#[test]
fn create_component_shortcut_promotes_selection() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);
    host.editor_state_mut()
        .set_single_selection(NodeId::new("f1"));

    assert!(host.apply_create_component());
    assert_eq!(host.editor_state().components.len(), 1);

    // No selection → no-op.
    host.editor_state_mut().clear_selection();
    assert!(!host.apply_create_component());
}

#[test]
fn space_pan_press_pans_canvas_regardless_of_tool() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);
    let before = host.editor_state().viewport.pan_x;

    host.set_space_pan(true);
    // Press empty canvas (Select tool active) — space-pan must start
    // a pan drag instead of a marquee.
    host.apply_press(700.0, 400.0, 1200.0, 800.0);
    host.apply_cursor_move(750.0, 400.0);
    let _ = host.apply_release_with_viewport(1200.0, 800.0);
    host.set_space_pan(false);

    let after = host.editor_state().viewport.pan_x;
    assert!(
        (after - before).abs() > 25.0,
        "space-drag must pan the viewport (delta {})",
        after - before
    );
}

#[test]
fn paste_figma_nodes_centres_fresh_ids_and_selects() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_FRAME);
    let incoming = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[
            {"type":"rectangle","id":"r1","name":"A","x":0,"y":0,"width":100,"height":100},
            {"type":"rectangle","id":"r2","name":"B","x":100,"y":0,"width":100,"height":100}
        ]}"#,
    )
    .expect("fixture parses")
    .value
    .children;

    // Capture the canvas centre BEFORE the paste — the new selection
    // reveals the property panel, which narrows the canvas region.
    let (_cx0, _cy0, cw, ch) = host.canvas_region(1200.0, 800.0);
    let expected = host
        .editor_state()
        .viewport
        .to_document(op_editor_ui::Point2D::new(cw / 2.0, ch / 2.0))
        .x as f64;

    assert!(host.paste_figma_nodes(incoming, 1200.0, 800.0));

    let state = host.editor_state();
    // Both roots landed with fresh ids (originals r1/r2 not reused
    // since they don't collide here — fresh mint always renames).
    assert_eq!(state.selection.set.len(), 2, "both pasted roots selected");
    assert!(state.history.can_undo(), "paste is one undoable batch");
    // The 200x100 union is centred on the viewport centre.
    let ids: Vec<_> = state.selection.set.clone();
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    for id in &ids {
        let node = op_editor_core::walkers::find_node(state.active_children(), id)
            .expect("pasted node present");
        let b = op_editor_core::own_bounds(node);
        min_x = min_x.min(b.x);
        max_x = max_x.max(b.x + b.w);
    }
    let centre_x = (min_x + max_x) / 2.0;
    assert!(
        (centre_x - expected).abs() < 60.0,
        "pasted union roughly centres on the viewport (got {centre_x}, expected ~{expected})"
    );
}

#[test]
fn cursor_move_tracks_canvas_hover_node() {
    let mut host = WidgetHostNative::new();
    seed(&mut host, ONE_RECT);
    // Hover reads the CURRENT scene without refreshing — build it.
    let _ = host.layout_scene();
    // Cursor-move derives the canvas region from the CACHED viewport
    // dims (normally written by apply_press) — seed them directly.
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;

    let (cx0, cy0) = host.canvas_origin();
    // Over the rect at doc (400, 300) — clear of the floating
    // toolbar column (over_topmost suppresses canvas hover).
    assert!(host.apply_cursor_move(cx0 + 450.0, cy0 + 325.0));
    assert_eq!(
        host.editor_state().editor_ui.canvas_hover_node,
        Some(NodeId::new("n1")),
        "hovering the node sets the canvas hover id"
    );
    // Empty canvas clears it.
    assert!(host.apply_cursor_move(cx0 + 700.0, cy0 + 600.0));
    assert_eq!(host.editor_state().editor_ui.canvas_hover_node, None);
}
