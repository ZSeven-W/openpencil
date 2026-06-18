use super::WidgetHost;

const VIEWPORT_W: f32 = 1200.0;
const VIEWPORT_H: f32 = 800.0;

fn long_design_md() -> String {
    let mut markdown = String::from("# Design System: Long\n\n## Color Palette\n");
    for index in 0..40 {
        markdown.push_str(&format!(
            "- **color-{index:02}** (#{index:02X}{index:02X}{index:02X}) - role {index}\n"
        ));
    }
    markdown
}

fn open_long_design_md(host: &mut WidgetHost) -> op_editor_ui::Rect {
    host.editor_state.editor_ui.design_md_panel_open = true;
    host.editor_state.doc.design_md = Some(op_editor_core::parse_design_md(&long_design_md()));
    host.design_md_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("design md panel rect")
}

#[test]
fn design_md_import_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (VIEWPORT_W, VIEWPORT_H);
    host.editor_state.editor_ui.design_md_panel_open = true;

    let panel_rect = host
        .design_md_panel_rect(viewport_w, viewport_h)
        .expect("design md panel rect");
    let panel = op_editor_ui::widgets::DesignMdPanel::for_editor(&host.editor_state)
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
        host.editor_state.editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::DesignMd(
            op_editor_core::DesignMdButton::Import
        ))
    );

    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn design_md_panel_wheel_scrolls_content_without_zooming_canvas() {
    let mut host = WidgetHost::new();
    let panel_rect = open_long_design_md(&mut host);
    let panel = op_editor_ui::widgets::DesignMdPanel::for_editor(&host.editor_state)
        .expect("open design md panel");
    assert!(panel.max_scroll(panel_rect) > 0.0);
    let zoom = host.editor_state.viewport.zoom;

    assert!(host.apply_wheel(
        panel_rect.origin.x + panel_rect.size.x / 2.0,
        panel_rect.origin.y + panel_rect.size.y / 2.0,
        -120.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert!(host.editor_state.editor_ui.design_md_scroll.offset > 0.0);
    assert_eq!(host.editor_state.viewport.zoom, zoom);
}

#[test]
fn design_md_panel_trackpad_pan_scrolls_content_without_panning_canvas() {
    let mut host = WidgetHost::new();
    let panel_rect = open_long_design_md(&mut host);
    let panel = op_editor_ui::widgets::DesignMdPanel::for_editor(&host.editor_state)
        .expect("open design md panel");
    assert!(panel.max_scroll(panel_rect) > 0.0);
    let pan_x = host.editor_state.viewport.pan_x;
    let pan_y = host.editor_state.viewport.pan_y;

    assert!(host.apply_pan_gesture(
        panel_rect.origin.x + panel_rect.size.x / 2.0,
        panel_rect.origin.y + panel_rect.size.y / 2.0,
        0.0,
        -120.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert!(host.editor_state.editor_ui.design_md_scroll.offset > 0.0);
    assert_eq!(host.editor_state.viewport.pan_x, pan_x);
    assert_eq!(host.editor_state.viewport.pan_y, pan_y);
}

#[test]
fn design_md_remove_press_clears_scroll_offset() {
    let mut host = WidgetHost::new();
    let panel_rect = open_long_design_md(&mut host);
    let max_scroll = op_editor_ui::widgets::DesignMdPanel::for_editor(&host.editor_state)
        .expect("open design md panel");
    let max_scroll = max_scroll.max_scroll(panel_rect);
    host.editor_state.editor_ui.design_md_scroll.offset = max_scroll;
    let panel = op_editor_ui::widgets::DesignMdPanel::for_editor(&host.editor_state)
        .expect("open design md panel");

    let mut point = None;
    let mut y = panel_rect.origin.y;
    while y <= panel_rect.origin.y + panel_rect.size.y && point.is_none() {
        let mut x = panel_rect.origin.x;
        while x <= panel_rect.origin.x + panel_rect.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if matches!(
                panel.hit_test(panel_rect, p),
                Some(op_editor_ui::widgets::DesignMdHit::Remove)
            ) {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = point.expect("remove action is hittable");

    assert!(host.apply_press(point.x, point.y, VIEWPORT_W, VIEWPORT_H));

    assert!(host.editor_state.doc.design_md.is_none());
    assert_eq!(host.editor_state.editor_ui.design_md_scroll.offset, 0.0);
}
