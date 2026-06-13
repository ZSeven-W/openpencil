use super::WidgetHost;

#[test]
fn locale_picker_wheel_scrolls_select_state_without_zooming_canvas() {
    let mut host = WidgetHost::new();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.editor_state.editor_ui.locale_picker.open = true;
    host.editor_state.editor_ui.locale_picker.hover = Some(0);
    let zoom = host.editor_state.viewport.zoom;
    let picker = host.locale_picker_rect(viewport_w);

    assert!(host.apply_wheel(
        picker.origin.x + picker.size.x / 2.0,
        picker.origin.y + op_editor_ui::widgets::LocalePicker::row_height(),
        -80.0,
        viewport_w,
        viewport_h
    ));

    let state = &host.editor_state.editor_ui.locale_picker;
    assert!(state.open);
    assert!(state.scroll.offset > 0.0);
    assert_eq!(state.hover, None);
    assert_eq!(host.editor_state.viewport.zoom, zoom);
}
