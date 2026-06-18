use super::WidgetHost;
use op_editor_core::codegen::{CodeSelection, CodegenHover, CodegenPhase};
use op_editor_core::{EditorState, PropertyTab};
use op_editor_ui::widgets::property_panel_action::CodegenAction;
use op_editor_ui::widgets::property_panel_code;
use op_editor_ui::widgets::property_panel_inputs::TAB_HEIGHT;
use op_editor_ui::widgets::{PropertyPanel, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

fn property_rect(host: &WidgetHost) -> Rect {
    Rect {
        origin: Point2D::new(
            host.last_viewport_w - host.editor_state.editor_ui.property_panel_width,
            TOP_BAR_HEIGHT,
        ),
        size: Point2D::new(
            host.editor_state.editor_ui.property_panel_width,
            (host.last_viewport_h - TOP_BAR_HEIGHT).max(0.0),
        ),
    }
}

fn point_inside_property_panel_without_target(host: &WidgetHost) -> Point2D {
    let panel = PropertyPanel::for_selection(&host.editor_state).expect("property panel");
    let rect = property_rect(host);
    let mut y = rect.origin.y + rect.size.y - 12.0;
    while y > rect.origin.y {
        let mut x = rect.origin.x + 12.0;
        while x < rect.origin.x + rect.size.x - 12.0 {
            let point = Point2D::new(x, y);
            let no_action = panel.hit_test_action(rect, point).is_none();
            let no_input = panel.hit_test(rect, point).is_none();
            let no_tab = panel.tab_hover_at(rect, point).is_none();
            let no_fill_type = panel.fill_type_picker_row_at(rect, point).is_none();
            if no_action && no_input && no_tab && no_fill_type {
                return point;
            }
            x += 8.0;
        }
        y -= 8.0;
    }
    panic!("no empty property-panel point found");
}

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
fn property_panel_blank_hover_consumes_and_clears_lower_hover() {
    let mut host = WidgetHost::new();
    host.editor_state = EditorState::sample();
    host.mark_dirty();
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state.editor_ui.canvas_hover_node = Some(op_editor_core::NodeId::new("Title"));

    let point = point_inside_property_panel_without_target(&host);

    assert!(
        host.apply_cursor_move(point.x, point.y),
        "right inspector should own cursor movement inside its bounds"
    );
    assert_eq!(host.editor_state.editor_ui.canvas_hover_node, None);
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
fn figma_import_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1200.0, 800.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.figma_import_open = true;
    let modal =
        op_editor_ui::widgets::figma_import::FigmaImportModal::for_editor(&host.editor_state);
    let panel = modal.rect(viewport_w, viewport_h);
    let mut point = None;
    let mut y = panel.origin.y;
    while y <= panel.origin.y + panel.size.y && point.is_none() {
        let mut x = panel.origin.x;
        while x <= panel.origin.x + panel.size.x {
            let p = op_editor_ui::Point2D::new(x, y);
            if modal.hit_test(panel, p)
                == op_editor_ui::widgets::figma_import::FigmaImportHit::DropZone
            {
                point = Some(p);
                break;
            }
            x += 4.0;
        }
        y += 4.0;
    }
    let point = point.expect("drop zone is hittable");

    assert!(host.apply_press(point.x, point.y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::FigmaImport(
            op_editor_core::FigmaImportButton::DropZone
        ))
    );

    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn component_browser_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1200.0, 800.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.component_browser_open = true;
    let panel = host
        .component_browser_panel_rect(viewport_w, viewport_h)
        .expect("component browser panel visible");
    let point =
        op_editor_ui::Point2D::new(panel.origin.x + panel.size.x - 82.0, panel.origin.y + 20.0);

    assert!(host.apply_press(point.x, point.y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::ComponentBrowser(
            op_editor_core::ComponentBrowserButton::ExportKit
        ))
    );

    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn component_browser_header_buttons_queue_kit_io_requests_like_native() {
    let mut host = WidgetHost::new();
    let (viewport_w, viewport_h) = (1440.0, 900.0);
    host.last_viewport_w = viewport_w;
    host.last_viewport_h = viewport_h;
    host.editor_state.editor_ui.component_browser_open = true;
    let rect = host
        .component_browser_panel_rect(viewport_w, viewport_h)
        .expect("component browser panel visible");
    let right = rect.origin.x + rect.size.x;
    let y = rect.origin.y + 20.0;

    assert!(host.apply_press(right - 54.0, y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state.editor_ui.component_browser_kit_request,
        Some(op_editor_core::KitIoRequest::Import)
    );

    host.editor_state.editor_ui.component_browser_kit_request = None;
    assert!(host.apply_press(right - 82.0, y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state.editor_ui.component_browser_kit_request,
        Some(op_editor_core::KitIoRequest::Export)
    );
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
