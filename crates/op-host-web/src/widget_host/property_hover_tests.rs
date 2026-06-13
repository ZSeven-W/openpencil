use super::WidgetHost;
use op_editor_core::codegen::{CodeSelection, CodegenHover, CodegenPhase};
use op_editor_core::{EditorState, PropertyTab};
use op_editor_ui::widgets::property_panel_action::CodegenAction;
use op_editor_ui::widgets::property_panel_code;
use op_editor_ui::widgets::property_panel_inputs::TAB_HEIGHT;
use op_editor_ui::widgets::TOP_BAR_HEIGHT;

#[test]
fn property_tab_hover_tracks_inactive_design_tab() {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::sample();
    host.mark_dirty();
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state.editor_ui.property_tab = PropertyTab::Code;

    let panel_x = host.last_viewport_w - host.editor_state.editor_ui.property_panel_width;
    assert!(host.apply_cursor_move(panel_x + 40.0, TOP_BAR_HEIGHT + 18.0));
    assert_eq!(
        host.editor_state.editor_ui.property_tab_hover,
        Some(PropertyTab::Design)
    );

    assert!(host.apply_cursor_move(panel_x - 12.0, TOP_BAR_HEIGHT + 18.0));
    assert_eq!(host.editor_state.editor_ui.property_tab_hover, None);
}

#[test]
fn codegen_hover_tracks_idle_generate_button() {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::sample();
    host.mark_dirty();
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state.editor_ui.property_tab = PropertyTab::Code;

    let pw = host.editor_state.editor_ui.property_panel_width;
    let panel_x = host.last_viewport_w - pw;
    let (_, generate_rect) = property_panel_code::code_action_rects(
        panel_x,
        TOP_BAR_HEIGHT + TAB_HEIGHT,
        pw,
        &host.editor_state.codegen,
    )
    .into_iter()
    .find(|(action, _)| matches!(action, CodegenAction::Generate))
    .expect("Generate rect present");

    let point_x = generate_rect.origin.x + generate_rect.size.x / 2.0;
    let point_y = generate_rect.origin.y + generate_rect.size.y / 2.0;
    assert!(host.apply_cursor_move(point_x, point_y));
    assert_eq!(
        host.editor_state.codegen.action_hover,
        Some(CodegenHover::Generate)
    );

    assert!(host.apply_cursor_move(panel_x - 12.0, TOP_BAR_HEIGHT + TAB_HEIGHT));
    assert_eq!(host.editor_state.codegen.action_hover, None);
}

#[test]
fn codegen_preview_drag_selects_code_text() {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::sample();
    host.mark_dirty();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.property_tab = PropertyTab::Code;
    host.editor_state.codegen.phase = CodegenPhase::Complete;
    host.editor_state.codegen.code = "import React\nconst n = 1".into();

    let panel_x = viewport_w - host.editor_state.editor_ui.property_panel_width;
    let char_w = 11.0 * 0.55;
    let start = op_editor_ui::Point2D::new(panel_x + 66.0, TOP_BAR_HEIGHT + 112.0);
    let end = op_editor_ui::Point2D::new(panel_x + 66.0 + char_w * 6.0, TOP_BAR_HEIGHT + 112.0);

    assert!(host.apply_press(start.x, start.y, viewport_w, viewport_h));
    assert!(host.apply_cursor_move(end.x, end.y));
    assert_eq!(
        host.editor_state.codegen.code_selection,
        Some(CodeSelection {
            anchor: 0,
            focus: 6
        })
    );
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
}

#[test]
fn status_bar_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1200.0, 800.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    let r = host
        .status_bar_rect(viewport_w, viewport_h)
        .expect("status bar visible at this size");
    let x = r.origin.x + 5.0;
    let y = r.origin.y + r.size.y / 2.0;

    assert!(host.apply_press(x, y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::StatusBar(
            op_editor_core::StatusBarButton::Search
        ))
    );

    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn export_dialog_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1200.0, 800.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.export_dialog_open = true;
    let dlg = op_editor_ui::widgets::ExportDialog::centered(viewport_w, viewport_h);
    let mut point = None;
    let r = dlg.rect();
    let mut y = r.origin.y;
    while y <= r.origin.y + r.size.y && point.is_none() {
        let mut x = r.origin.x;
        while x <= r.origin.x + r.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if dlg.hit_test(p)
                == Some(op_editor_ui::widgets::export_dialog::ExportDialogHit::Scale(1))
            {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = point.expect("scale 1 pill is hittable");

    assert!(host.apply_press(point.x, point.y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::ExportDialog(
            op_editor_core::ExportDialogButton::Scale(1)
        ))
    );

    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn codegen_preview_wheel_scrolls_code_not_property_panel() {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::sample();
    host.mark_dirty();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.property_tab = PropertyTab::Code;
    host.editor_state.codegen.phase = CodegenPhase::Complete;
    host.editor_state.codegen.code = (0..100)
        .map(|i| format!("line-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");

    let panel_x = viewport_w - host.editor_state.editor_ui.property_panel_width;
    assert!(host.apply_wheel(
        panel_x + 80.0,
        TOP_BAR_HEIGHT + 112.0,
        -160.0,
        viewport_w,
        viewport_h
    ));

    assert!(host.editor_state.codegen.code_scroll.offset > 0.0);
    assert_eq!(
        host.editor_state.editor_ui.property_panel_scroll.offset,
        0.0
    );
}
