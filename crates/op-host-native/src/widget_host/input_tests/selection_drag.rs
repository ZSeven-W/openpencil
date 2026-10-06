//! Canvas marquee, LayerPanel drag-to-reorder, and the create-component
//! promotions driven from the layer context menu / property panel.
//!
//! Split out of `input_tests.rs` to keep every file under the repo's
//! 800-line cap.

use super::*;

#[test]
fn marquee_drag_replaces_selection_with_intersecting_nodes() {
    let mut host = WidgetHostNative::new();
    // 3 rects: two close together near origin, one far away.
    seed(
        &mut host,
        &three_rects(
            [
                (50.0, 10.0, 20.0, 20.0),
                (90.0, 10.0, 20.0, 20.0),
                (200.0, 200.0, 20.0, 20.0),
            ],
            ["n50", "n51", "n52"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    let press_x = cx0 + 5.0;
    let press_y = cy0 + 5.0;
    host.apply_press(press_x, press_y, viewport_w, viewport_h);
    assert!(
        host.marquee_drag.is_some(),
        "empty-canvas press should start a marquee"
    );
    host.apply_cursor_move(cx0 + 130.0, cy0 + 50.0);
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert!(host.marquee_drag.is_none(), "marquee consumed on release");
    let mut hits: Vec<String> = host
        .editor_state()
        .selection
        .set
        .iter()
        .map(|i| i.as_str().to_string())
        .collect();
    hits.sort();
    assert_eq!(hits, vec!["n50", "n51"]);
}

#[test]
fn marquee_drag_with_shift_preserves_already_selected_hit() {
    // Codex CONCERN-Q2 regression: shift-marquee must be ADD-only.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (50.0, 50.0, 20.0, 20.0),
                (300.0, 300.0, 20.0, 20.0),
                (900.0, 900.0, 20.0, 20.0),
            ],
            ["n70", "n71", "n72"],
        ),
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n70"));
    host.set_modifier_shift(true);
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 5.0, cy0 + 5.0, viewport_w, viewport_h);
    host.apply_cursor_move(cx0 + 90.0, cy0 + 90.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    // "n70" stays in the set (shift-marquee is ADD-only).
    assert!(host.editor_state().is_selected(&NodeId::new("n70")));
    assert_eq!(host.editor_state().selection.set.len(), 1);
}

#[test]
fn marquee_drag_below_screen_threshold_is_a_no_op() {
    // Codex CONCERN-Q5 regression: threshold is screen-px.
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 100.0, 100.0),
                (5000.0, 5000.0, 10.0, 10.0),
                (6000.0, 6000.0, 10.0, 10.0),
            ],
            ["n80", "n81", "n82"],
        ),
    );
    host.editor_state_mut().viewport.zoom = 0.1;
    host.editor_state_mut().clear_selection();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 100.0, cy0 + 50.0, viewport_w, viewport_h);
    assert!(host.marquee_drag.is_some());
    // Tiny drag: 1 screen-px — below the 2-px threshold.
    host.apply_cursor_move(cx0 + 101.0, cy0 + 50.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    assert!(host.editor_state().selection.set.is_empty());
}

#[test]
fn marquee_drag_with_shift_extends_existing_selection() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (10.0, 10.0, 20.0, 20.0),
                (50.0, 10.0, 20.0, 20.0),
                (300.0, 300.0, 20.0, 20.0),
            ],
            ["n60", "n61", "n62"],
        ),
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n62"));
    host.set_modifier_shift(true);
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let (cx0, cy0, _cw, _ch) = host.canvas_region(viewport_w, viewport_h);
    host.apply_press(cx0 + 5.0, cy0 + 5.0, viewport_w, viewport_h);
    assert!(host.marquee_drag.is_some());
    host.apply_cursor_move(cx0 + 130.0, cy0 + 50.0);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    let mut ids: Vec<String> = host
        .editor_state()
        .selection
        .set
        .iter()
        .map(|i| i.as_str().to_string())
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["n60", "n61", "n62"]);
}

#[test]
fn layer_drag_to_reorder_commits_on_release_with_threshold_move() {
    let mut host = WidgetHostNative::new();
    // Three top-level nodes painted as flat layer rows.
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
            ],
            ["n70", "n71", "n72"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let row_h = 28.0; // LAYER_ROW_HEIGHT
    let page_row_h = 32.0; // PAGE_ROW_HEIGHT
    let section_header_h = 28.0;
    let section_gap = 8.0;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    // The rail's tree no longer starts at the window's top bar: the tab
    // row (对话 / 图层 / 幻灯片) sits above it, so measure from the
    // rail's live CONTENT rect.
    let content_top = host.layers_content_rect(viewport_w, viewport_h).origin.y;
    let layers_top =
        content_top + 8.0 + section_header_h + page_row_h + section_gap + section_header_h;
    let row_y = |i: usize| layers_top + (i as f32) * row_h + row_h / 2.0;
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    host.apply_press(row_x, row_y(0), viewport_w, viewport_h);
    assert!(host.layer_drag.is_some());
    assert!(!host.layer_drag.as_ref().unwrap().active);
    host.apply_cursor_move(row_x, row_y(2) + row_h / 2.0 - 4.0);
    assert!(host.layer_drag.as_ref().unwrap().active);
    host.apply_release_with_viewport(viewport_w, viewport_h);
    assert!(host.layer_drag.is_none(), "drag must be cleared on release");
    // A moved after C → final order [B, C, A].
    let order: Vec<String> = host
        .editor_state()
        .doc
        .children
        .iter()
        .map(|n| n.base().id.clone())
        .collect();
    assert_eq!(order, vec!["n71", "n72", "n70"]);
}

#[test]
fn layer_drag_below_activation_threshold_is_a_click_not_a_reorder() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (0.0, 0.0, 10.0, 10.0),
                (5000.0, 5000.0, 10.0, 10.0),
            ],
            ["n80", "n81", "n82"],
        ),
    );
    host.editor_state_mut().clear_selection();
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let row_y_first = host.layers_content_rect(viewport_w, viewport_h).origin.y
        + 8.0
        + 28.0
        + 32.0
        + 8.0
        + 28.0
        + 14.0;
    host.apply_press(row_x, row_y_first, viewport_w, viewport_h);
    host.apply_cursor_move(row_x, row_y_first + 2.0);
    assert!(
        host.layer_drag.is_some() && !host.layer_drag.as_ref().unwrap().active,
        "sub-threshold move must not activate"
    );
    host.apply_release_with_viewport(viewport_w, viewport_h);
    let order: Vec<String> = host
        .editor_state()
        .doc
        .children
        .iter()
        .map(|n| n.base().id.clone())
        .collect();
    assert_eq!(order, vec!["n80", "n81", "n82"]);
    assert_eq!(host.editor_state().selection.anchor, NodeId::new("n80"));
}

#[test]
fn layer_context_create_component_click_promotes_frame() {
    use op_editor_core::editor_ui_state::LayerContextMenuState;
    use op_editor_core::ui_draft::LayerContextTarget;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"frame","id":"n10","name":"Card","x":0,"y":0,"width":100,"height":80,
           "children":[]}
        ]}"#,
    );
    host.editor_state_mut().editor_ui.layer_context_menu = Some(LayerContextMenuState {
        target: LayerContextTarget::Layer(NodeId::new("n10")),
        anchor_x: 100.0,
        anchor_y: 100.0,
        menu: Default::default(),
    });

    let create_row_y = 100.0 + 6.0 + 32.0 * 2.0 + 16.0;
    assert!(host.apply_press(120.0, create_row_y, 1440.0, 900.0));
    assert!(host
        .editor_state()
        .components
        .find_by_id(&NodeId::new("n10"))
        .is_some());
    match &host.editor_state().doc.children[0] {
        jian_ops_schema::node::PenNode::Frame(f) => assert_eq!(f.reusable, Some(true)),
        _ => panic!("expected frame"),
    }
}

#[test]
fn property_panel_create_component_click_promotes_selected_frame() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[
          {"type":"frame","id":"n20","name":"Hero","x":0,"y":0,"width":120,"height":90,
           "children":[]}
        ]}"#,
    );
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n20"));

    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let panel_left = viewport_w - host.editor_state().editor_ui.property_panel_width;
    let button_x = panel_left + 24.0;
    let button_y = TOP_BAR_HEIGHT + 36.0 + 30.0 + 8.0 + 18.0;
    assert!(host.apply_press(button_x, button_y, viewport_w, viewport_h));
    assert!(host
        .editor_state()
        .components
        .find_by_id(&NodeId::new("n20"))
        .is_some());
}

#[test]
fn layer_context_group_preserves_multi_selection_and_groups() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 20.0, 20.0),
                (40.0, 0.0, 20.0, 20.0),
                (120.0, 0.0, 20.0, 20.0),
            ],
            ["n30", "n31", "n32"],
        ),
    );
    host.editor_state_mut().selection.set = vec![NodeId::new("n30"), NodeId::new("n31")];
    host.editor_state_mut().selection.anchor = NodeId::new("n30");

    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    let row_x = host.editor_state().editor_ui.layer_panel_width / 2.0;
    let first_row_y = host.layers_content_rect(viewport_w, viewport_h).origin.y
        + 8.0
        + 28.0
        + 32.0
        + 8.0
        + 28.0
        + 14.0;
    assert!(host.apply_right_press(row_x, first_row_y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state().selection.set.len(),
        2,
        "right-clicking an already-selected layer must keep the multi-selection"
    );

    let group_row_y = first_row_y + 6.0 + 32.0 * 2.0 + 16.0;
    assert!(host.apply_press(row_x + 20.0, group_row_y, viewport_w, viewport_h));
    assert!(matches!(
        host.editor_state().doc.children.first(),
        Some(jian_ops_schema::node::PenNode::Group(_))
    ));
}
