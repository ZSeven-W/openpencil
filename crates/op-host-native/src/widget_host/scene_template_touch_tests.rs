use super::WidgetHostNative;
use op_editor_core::size_class::EditorSizeClass;
use op_editor_ui::widgets::{SceneTemplateHit, SceneTemplatePanel};
use op_editor_ui::Point2D;

fn touch_asset_host(size_class: EditorSizeClass) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = size_class;
    ui.scene_template_generate_supported = true;
    ui.open_scene_template_center(0);
    host
}

fn card_point(host: &WidgetHostNative, width: f32, height: f32) -> Point2D {
    let rect = host
        .scene_template_panel_rect(width, height)
        .expect("asset center rect");
    let panel = SceneTemplatePanel::for_editor(host.editor_state()).expect("asset center model");
    let viewport = panel.cards_viewport(rect);
    let mut y = viewport.origin.y + 2.0;
    while y < viewport.origin.y + viewport.size.y - 2.0 {
        let mut x = viewport.origin.x + 2.0;
        while x < viewport.origin.x + viewport.size.x - 2.0 {
            let point = Point2D::new(x, y);
            if matches!(
                panel.hit_test(rect, point),
                Some(SceneTemplateHit::AddTemplateToCanvas(_))
            ) {
                return point;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    panic!("visible template card point");
}

#[test]
fn touch_open_waits_for_an_explicit_field_tap_before_activating_ime() {
    let mut host = touch_asset_host(EditorSizeClass::Compact);
    let (width, height) = (390.0, 844.0);
    assert!(!host.text_input_focus_active());
    assert!(!host
        .editor_state()
        .editor_ui
        .scene_template_center
        .input_active());

    let rect = host
        .scene_template_panel_rect(width, height)
        .expect("asset center rect");
    let panel = SceneTemplatePanel::for_editor(host.editor_state()).expect("asset center model");
    assert!(panel.cards_viewport(rect).size.y > 0.0);
    let search = panel.search_rect_for(rect);
    let point = Point2D::new(
        search.origin.x + search.size.x / 2.0,
        search.origin.y + search.size.y / 2.0,
    );

    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(host.touch_panel_gesture.is_some());
    assert!(!host.text_input_focus_active());
    assert!(host.apply_release_with_viewport(width, height));
    assert!(host.text_input_focus_active());
}

#[test]
fn compact_and_medium_card_taps_commit_once_on_stationary_release() {
    for (class, width, height) in [
        (EditorSizeClass::Compact, 390.0_f32, 844.0_f32),
        (EditorSizeClass::Medium, 834.0_f32, 1_112.0_f32),
    ] {
        let mut host = touch_asset_host(class);
        let point = card_point(&host, width, height);

        assert!(host.apply_press(point.x, point.y, width, height));
        assert!(host.touch_panel_gesture.is_some());
        assert!(
            host.editor_state()
                .editor_ui
                .scene_template_center
                .pending_open
                .is_none(),
            "down must not choose a card"
        );
        assert!(host.apply_release_with_viewport(width, height));
        let first = host
            .editor_state_mut()
            .editor_ui
            .scene_template_center
            .take_pending_open();
        assert!(first.is_some(), "stationary release chooses the card");

        let _ = host.apply_release_with_viewport(width, height);
        assert!(
            host.editor_state_mut()
                .editor_ui
                .scene_template_center
                .take_pending_open()
                .is_none(),
            "a second release cannot replay the same tap"
        );
    }
}

#[test]
fn one_finger_card_drag_scrolls_without_selecting_or_moving_the_canvas() {
    let (width, height) = (390.0, 844.0);
    let mut host = touch_asset_host(EditorSizeClass::Compact);
    let point = card_point(&host, width, height);
    let viewport = host.editor_state().viewport;

    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(host.apply_cursor_move(point.x, point.y - 24.0));
    assert!(
        host.editor_state()
            .editor_ui
            .scene_template_center
            .scroll
            .offset
            > 0.0
    );
    assert!(host.apply_release_with_viewport(width, height));
    assert!(host
        .editor_state()
        .editor_ui
        .scene_template_center
        .pending_open
        .is_none());
    assert!(host.editor_state().editor_ui.scene_template_center.open);
    assert_eq!(host.editor_state().viewport, viewport);
}

#[test]
fn a_second_pointer_cancel_seam_drops_the_pending_asset_tap() {
    let (width, height) = (390.0, 844.0);
    let mut host = touch_asset_host(EditorSizeClass::Compact);
    let point = card_point(&host, width, height);
    assert!(host.apply_press(point.x, point.y, width, height));
    assert!(host.touch_panel_gesture.is_some());

    assert!(host.cancel_native_touch_gestures());
    let _ = host.apply_release_with_viewport(width, height);
    assert!(host
        .editor_state()
        .editor_ui
        .scene_template_center
        .pending_open
        .is_none());
}
