use super::WidgetHostNative;

#[test]
fn design_md_import_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.editor_state_mut().editor_ui.design_md_panel_open = true;

    let panel_rect = host
        .design_md_panel_rect(viewport_w, viewport_h)
        .expect("design md panel rect");
    let panel = op_editor_ui::widgets::DesignMdPanel::for_editor(host.editor_state())
        .expect("open design md panel");
    let mut point = None;
    let mut y = panel_rect.origin.y;
    while y <= panel_rect.origin.y + panel_rect.size.y && point.is_none() {
        let mut x = panel_rect.origin.x;
        while x <= panel_rect.origin.x + panel_rect.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if matches!(
                panel.hit_test(panel_rect, p),
                Some(op_editor_ui::widgets::DesignMdHit::Import)
            ) {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = point.expect("import button is hittable");

    assert!(host.apply_press(point.x, point.y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::DesignMd(
            op_editor_core::DesignMdButton::Import
        ))
    );

    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}
