use super::WidgetHostNative;
use op_editor_ui::widgets::LayerPanelHit;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn page_switch_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0.0","children":[],"pages":[
            {"id":"p1","name":"Near","children":[
                {"type":"rectangle","id":"near","name":"Near","x":0,"y":0,
                 "width":100,"height":100}]},
            {"id":"p2","name":"Far","children":[
                {"type":"frame","id":"far","name":"Far","x":2000,"y":1500,
                 "width":400,"height":200,"clipContent":false,"children":[
                    {"type":"rectangle","id":"spill","x":1200,"y":20,
                     "width":100,"height":100}]}]}
        ]}"#,
    )
    .expect("page fixture parses")
    .value;
    *host.editor_state_mut() = op_editor_core::EditorState::from_document(doc);
    host.editor_state_mut().editor_ui.preserve_authored_geometry = true;
    host.mark_paint_dirty_for_test();
    host
}

fn point_for_page_row(host: &WidgetHostNative, page_index: usize) -> Point2D {
    let panel = host.layer_panel();
    // Ask the host where the tree actually is instead of rebuilding the
    // panel rect here: the rail carries a tab row now (对话 / 图层 /
    // 幻灯片), so the tree starts below it and a hand-rolled rect probes
    // 36 px above every row.
    let rect = host.layers_content_rect(VIEWPORT_W, VIEWPORT_H);
    let regions = panel.regions(rect);
    let mut y = regions.pages_rows_top + 2.0;
    while y < regions.pages_rows_top + regions.pages_view_h {
        let point = Point2D::new(rect.origin.x + 48.0, y);
        if matches!(
            panel.hit_test(rect, point),
            Some(LayerPanelHit::Page(index)) if index == page_index
        ) {
            return point;
        }
        y += 2.0;
    }
    panic!("no page row point found for index {page_index}");
}

#[test]
fn page_row_switch_fits_new_page_after_cached_scene() {
    let mut host = page_switch_host();

    // Match a real post-paint host: page 0 owns the derived scene and the host
    // dirty bit has been consumed before the user switches pages.
    host.refresh_layout_scene();
    assert_eq!(host.layout_scene.active_page_index, 0);
    assert!(!host.editor_state_dirty);

    let near_rows = host.layer_panel().items;
    let near_rows_again = host.layer_panel().items;
    assert!(
        std::rc::Rc::ptr_eq(&near_rows, &near_rows_again),
        "event-time panel resolves must reuse this host's owned row cache"
    );

    let page_row = point_for_page_row(&host, 1);
    assert!(host.apply_press(page_row.x, page_row.y, VIEWPORT_W, VIEWPORT_H,));
    assert_eq!(host.editor_state().ui.active_page_index, 1);

    let far_rows = host.layer_panel().items;
    assert!(
        !std::rc::Rc::ptr_eq(&near_rows, &far_rows),
        "the active-page cache key must rebuild rows after a switch"
    );
    assert!(far_rows.iter().any(|row| row.node_id.as_str() == "far"));
    assert!(std::rc::Rc::ptr_eq(&far_rows, &host.layer_panel().items));

    // Resolve the active scene so the assertion derives its expected centre
    // from the same layout path used by paint, not from authored fixture math.
    host.refresh_layout_scene();
    let bounds = host
        .layout_scene
        .content_bounds()
        .expect("the switched-to page has content");
    let (_, _, canvas_w, canvas_h) = host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let viewport = host.editor_state().viewport;
    let content_center_x = bounds.origin.x + bounds.size.x / 2.0;
    let content_center_y = bounds.origin.y + bounds.size.y / 2.0;
    let expected_zoom = ((canvas_w - 128.0).max(1.0) / bounds.size.x)
        .min((canvas_h - 128.0).max(1.0) / bounds.size.y)
        .clamp(0.1, 1.0);

    assert!(
        bounds.size.x > 1200.0,
        "explicitly open frame must include descendant overflow: {bounds:?}"
    );
    assert!(
        (viewport.zoom - expected_zoom).abs() < 0.0001,
        "fit scale must include open-frame overflow: viewport={viewport:?}, bounds={bounds:?}"
    );

    assert!(
        (viewport.pan_x + content_center_x * viewport.zoom - canvas_w / 2.0).abs() < 0.01,
        "new page must be horizontally centered: viewport={viewport:?}, bounds={bounds:?}"
    );
    assert!(
        (viewport.pan_y + content_center_y * viewport.zoom - canvas_h / 2.0).abs() < 0.01,
        "new page must be vertically centered: viewport={viewport:?}, bounds={bounds:?}"
    );
}

#[test]
fn clicking_active_page_row_preserves_viewport() {
    let mut host = page_switch_host();
    host.editor_state_mut().viewport = op_editor_core::Viewport {
        pan_x: -321.0,
        pan_y: 147.0,
        zoom: 0.42,
    };
    host.refresh_layout_scene();
    let viewport_before = host.editor_state().viewport;

    let active_page_row = point_for_page_row(&host, 0);
    assert!(host.apply_press(active_page_row.x, active_page_row.y, VIEWPORT_W, VIEWPORT_H,));

    assert_eq!(host.editor_state().ui.active_page_index, 0);
    assert_eq!(
        host.editor_state().viewport,
        viewport_before,
        "reselecting the active page must not zoom-to-fit"
    );
}

#[test]
fn page_switch_refresh_clears_transition_and_resolves_only_the_target_page() {
    let mut host = page_switch_host();
    host.refresh_layout_scene();
    let near_bounds = host
        .layout_scene
        .active_page()
        .and_then(|page| page.find("near"))
        .expect("near node is in the settled first-page scene")
        .bounds;
    host.start_layout_transition_from_bounds(&op_editor_core::NodeId::new("near"), near_bounds);
    assert!(host.layout_transition.is_some());

    assert!(host.editor_state_mut().set_active_page(1));
    host.mark_paint_dirty_for_test();
    host.refresh_layout_scene();

    assert_eq!(host.layout_scene.active_page_index, 1);
    assert_eq!(host.layout_scene.pages.len(), 2);
    assert!(
        host.layout_scene.pages[0].children.is_empty(),
        "the inactive page must stay metadata-only"
    );
    assert!(
        host.layout_scene.pages[1].find("far").is_some(),
        "the switched-to page must own the rebuilt render tree"
    );
    assert!(
        host.layout_transition.is_none(),
        "a transition from the old page must not survive the switch"
    );
}

#[test]
fn same_page_document_rebuild_preserves_layout_transition() {
    let mut host = page_switch_host();
    host.refresh_layout_scene();
    let before_bounds = host
        .layout_scene
        .active_page()
        .and_then(|page| page.find("near"))
        .expect("near node is in the settled first-page scene")
        .bounds;
    host.start_layout_transition_from_bounds(&op_editor_core::NodeId::new("near"), before_bounds);

    {
        let state = host.editor_state_mut();
        state.set_single_selection(op_editor_core::NodeId::new("near"));
        assert!(state.translate_selected(25.0, 0.0));
        state.mark_document_changed();
    }
    host.mark_paint_dirty_for_test();
    host.refresh_layout_scene();

    let after_bounds = host
        .layout_scene
        .active_page()
        .and_then(|page| page.find("near"))
        .expect("same-page rebuild must keep the edited node")
        .bounds;
    assert_eq!(host.layout_scene.active_page_index, 0);
    assert!((after_bounds.origin.x - before_bounds.origin.x - 25.0).abs() < 0.01);
    assert!(
        host.layout_transition.is_some(),
        "an ordinary same-page document rebuild must not clear its transition"
    );
}
