use super::WidgetHostNative;

#[test]
fn icon_picker_load_more_press_sets_and_release_clears_pressed() {
    let mut host = WidgetHostNative::new();
    let viewport_w = 1440.0;
    let viewport_h = 900.0;
    host.editor_state_mut().editor_ui.icon_picker.open = true;
    host.editor_state_mut().editor_ui.icon_picker_search = "unlikely-remote-only".to_string();

    let panel_rect = host
        .icon_picker_panel_rect(viewport_w, viewport_h)
        .expect("icon picker rect");
    let panel = op_editor_ui::widgets::IconPickerPanel::for_editor(host.editor_state())
        .expect("open icon picker");
    let mut point = None;
    let mut y = panel_rect.origin.y;
    while y <= panel_rect.origin.y + panel_rect.size.y && point.is_none() {
        let mut x = panel_rect.origin.x;
        while x <= panel_rect.origin.x + panel_rect.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if matches!(
                panel.hit_test(panel_rect, p),
                Some(op_editor_ui::widgets::IconPickerHit::LoadMore)
            ) {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = point.expect("load more row is hittable");

    assert!(host.apply_press(point.x, point.y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state().editor_ui.icon_picker.pressed,
        Some(op_editor_ui::widgets::icon_picker_panel::ICON_PICKER_LOAD_MORE_HOVER)
    );

    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert_eq!(host.editor_state().editor_ui.icon_picker.pressed, None);
}
