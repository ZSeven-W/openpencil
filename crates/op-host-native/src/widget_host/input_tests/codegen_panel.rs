//! Right-rail Code tab: framework select, generate/cancel phases, copy,
//! preview drag + wheel routing, and the import-menu / rail-routing tails.
//!
//! Split out of `input_tests.rs` to keep every file under the repo's
//! 800-line cap.

use super::*;

#[test]
fn compact_touch_shortcut_and_direct_action_cannot_open_codegen() {
    use op_editor_core::size_class::EditorSizeClass;
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;

    let mut host = WidgetHostNative::new();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = EditorSizeClass::Compact;
    ui.property_tab = PropertyTab::Code;

    assert!(host.apply_toggle_code_panel());
    assert_eq!(
        host.editor_state().editor_ui.property_tab,
        PropertyTab::Design
    );
    assert!(host.apply_toggle_code_panel());
    assert_eq!(
        host.editor_state().editor_ui.property_tab,
        PropertyTab::Design
    );

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));
    assert!(!host.editor_state().codegen.pending_generate);
}

#[test]
fn medium_touch_shortcut_and_codegen_action_remain_available() {
    use op_editor_core::size_class::EditorSizeClass;
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;

    let mut host = WidgetHostNative::new();
    let ui = &mut host.editor_state_mut().editor_ui;
    ui.touch = true;
    ui.size_class = EditorSizeClass::Medium;

    assert!(host.apply_toggle_code_panel());
    assert_eq!(
        host.editor_state().editor_ui.property_tab,
        PropertyTab::Code
    );
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));
    assert!(host.editor_state().codegen.pending_generate);
}

#[test]
fn codegen_select_framework_updates_state() {
    use op_editor_core::codegen::Framework;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();

    host.apply_property_action(PropertyPanelAction::Codegen(
        CodegenAction::SelectFramework(Framework::Vue),
    ));

    assert_eq!(host.editor_state().codegen.framework, Framework::Vue);
}

#[test]
fn codegen_generate_raises_pending_and_generating_phase() {
    use op_editor_core::codegen::CodegenPhase;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));

    assert!(host.editor_state().codegen.pending_generate);
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Generating);
}

#[test]
fn codegen_copy_queues_code_to_system_clipboard() {
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();
    host.set_now_ms(4242);
    host.editor_state_mut().codegen.code = "export const App = () => null;".to_string();

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Copy));

    assert_eq!(host.editor_state().codegen.copied_at, Some(4242));
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("export const App = () => null;"),
    );
}

#[test]
fn codegen_copy_button_queues_full_code_even_with_selection() {
    use op_editor_core::codegen::CodeSelection;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();
    host.set_now_ms(4242);
    host.editor_state_mut().codegen.code = "export const App = () => null;".to_string();
    host.editor_state_mut().codegen.code_selection = Some(CodeSelection {
        anchor: 7,
        focus: 12,
    });

    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Copy));

    assert_eq!(host.editor_state().codegen.copied_at, Some(4242));
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("export const App = () => null;"),
    );
}

#[test]
fn shortcut_copy_prefers_selected_code_text() {
    use op_editor_core::codegen::CodeSelection;
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().codegen.code = "export const App = () => null;".to_string();
    host.editor_state_mut().codegen.code_selection = Some(CodeSelection {
        anchor: 7,
        focus: 12,
    });

    assert!(host.apply_copy());
    assert_eq!(
        host.editor_state().chat.pending_copy_text.as_deref(),
        Some("const"),
    );
}

#[test]
fn codegen_hover_tracks_idle_generate_button() {
    use op_editor_core::codegen::CodegenHover;
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::property_panel_inputs::TAB_HEIGHT;
    use op_editor_ui::widgets::{property_panel_code, TOP_BAR_HEIGHT};
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n-code","name":"n-code","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-code"));
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;

    let pw = host.editor_state().editor_ui.property_panel_width;
    let panel_x = host.last_viewport_w - pw;
    let (_, generate_rect) = property_panel_code::code_action_rects(
        panel_x,
        TOP_BAR_HEIGHT + TAB_HEIGHT,
        pw,
        &host.editor_state().codegen,
    )
    .into_iter()
    .find(|(action, _)| matches!(action, CodegenAction::Generate))
    .expect("Generate rect present");
    let point = op_editor_ui::Point2D::new(
        generate_rect.origin.x + generate_rect.size.x / 2.0,
        generate_rect.origin.y + generate_rect.size.y / 2.0,
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().codegen.action_hover,
        Some(CodegenHover::Generate)
    );

    assert!(host.apply_cursor_move(panel_x - 12.0, TOP_BAR_HEIGHT + TAB_HEIGHT));
    assert_eq!(host.editor_state().codegen.action_hover, None);
}

#[test]
fn codegen_preview_drag_selects_code_text() {
    use op_editor_core::codegen::{CodeSelection, CodegenPhase};
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n-code","name":"n-code","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-code"));
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;
    host.editor_state_mut().codegen.phase = CodegenPhase::Complete;
    host.editor_state_mut().codegen.code = "import React\nconst n = 1".into();

    let panel_x = viewport_w - host.editor_state().editor_ui.property_panel_width;
    let char_w = 11.0 * 0.55;
    let start = op_editor_ui::Point2D::new(panel_x + 66.0, TOP_BAR_HEIGHT + 112.0);
    let end = op_editor_ui::Point2D::new(panel_x + 66.0 + char_w * 6.0, TOP_BAR_HEIGHT + 112.0);

    assert!(host.apply_press(start.x, start.y, viewport_w, viewport_h));
    assert!(host.apply_cursor_move(end.x, end.y));
    assert_eq!(
        host.editor_state().codegen.code_selection,
        Some(CodeSelection {
            anchor: 0,
            focus: 6
        })
    );
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));
}

#[test]
fn codegen_preview_wheel_scrolls_code_not_property_panel() {
    use op_editor_core::codegen::CodegenPhase;
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n-code","name":"n-code","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-code"));
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;
    host.editor_state_mut().codegen.phase = CodegenPhase::Complete;
    host.editor_state_mut().codegen.code = (0..100)
        .map(|i| format!("line-{i:02}"))
        .collect::<Vec<_>>()
        .join("\n");

    let panel_x = viewport_w - host.editor_state().editor_ui.property_panel_width;
    assert!(host.apply_wheel(
        panel_x + 80.0,
        TOP_BAR_HEIGHT + 112.0,
        -160.0,
        viewport_w,
        viewport_h
    ));

    assert!(host.editor_state().codegen.code_scroll.offset > 0.0);
    assert_eq!(
        host.editor_state().editor_ui.property_panel_scroll.offset,
        0.0
    );
}

#[test]
fn property_tab_hover_tracks_inactive_design_tab() {
    use op_editor_core::PropertyTab;
    use op_editor_ui::widgets::TOP_BAR_HEIGHT;
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n-tabs","name":"n-tabs","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.last_viewport_w = 1200.0;
    host.last_viewport_h = 800.0;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-tabs"));
    host.editor_state_mut().editor_ui.property_tab = PropertyTab::Code;

    let panel_x = host.last_viewport_w - host.editor_state().editor_ui.property_panel_width;
    assert!(host.apply_cursor_move(panel_x + 40.0, TOP_BAR_HEIGHT + 18.0));
    assert_eq!(
        host.editor_state().editor_ui.property_tab_hover,
        Some(PropertyTab::Design)
    );

    assert!(host.apply_cursor_move(panel_x - 12.0, TOP_BAR_HEIGHT + 18.0));
    assert_eq!(host.editor_state().editor_ui.property_tab_hover, None);
}

#[test]
fn codegen_cancel_resets_phase_by_code_presence() {
    use op_editor_core::codegen::CodegenPhase;
    use op_editor_ui::widgets::property_panel_action::CodegenAction;
    use op_editor_ui::widgets::PropertyPanelAction;
    let mut host = WidgetHostNative::new();
    // No code yet → Cancel falls back to Idle.
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Cancel));
    assert!(!host.editor_state().codegen.pending_generate);
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Idle);

    // With code present → Cancel returns to Complete.
    host.editor_state_mut().codegen.code = "rendered".to_string();
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Generate));
    host.apply_property_action(PropertyPanelAction::Codegen(CodegenAction::Cancel));
    assert_eq!(host.editor_state().codegen.phase, CodegenPhase::Complete);
}

/// The top-bar import button opens a two-row menu; the Figma row keeps
/// the existing modal, and the HTML row raises the file action the
/// desktop runner turns into a file dialog + background import.
#[test]
fn import_menu_routes_its_two_rows_to_figma_and_html() {
    use op_editor_ui::widgets::{ImportMenu, TopBar};

    let (vw, vh) = (1200.0, 800.0);
    let button = {
        let host = WidgetHostNative::new();
        let top_bar_rect = op_editor_ui::Rect {
            origin: op_editor_ui::Point2D::new(0.0, 0.0),
            size: op_editor_ui::Point2D::new(vw, op_editor_ui::widgets::TOP_BAR_HEIGHT),
        };
        TopBar::for_editor_ui(&host.editor_state().editor_ui).import_button_rect(top_bar_rect)
    };
    let button_center = op_editor_ui::Point2D::new(
        button.origin.x + button.size.x / 2.0,
        button.origin.y + button.size.y / 2.0,
    );

    let row_point = |host: &WidgetHostNative, idx: usize| {
        let menu = ImportMenu::for_editor_ui(&host.editor_state().editor_ui);
        let anchor_viewport = (
            op_editor_ui::Rect {
                origin: button.origin,
                size: op_editor_ui::Point2D::new(
                    op_editor_ui::widgets::IMPORT_MENU_WIDTH,
                    button.size.y,
                ),
            },
            op_editor_ui::Rect {
                origin: op_editor_ui::Point2D::new(0.0, 0.0),
                size: op_editor_ui::Point2D::new(vw, vh),
            },
        );
        let panel = menu.popup_rect(anchor_viewport.0, anchor_viewport.1);
        let row_h = panel.size.y / 2.0;
        op_editor_ui::Point2D::new(
            panel.origin.x + panel.size.x / 2.0,
            panel.origin.y + row_h * idx as f32 + row_h / 2.0,
        )
    };

    // Figma row → the existing import modal.
    let mut host = WidgetHostNative::new();
    assert!(host.apply_press(button_center.x, button_center.y, vw, vh));
    assert!(host.editor_state().editor_ui.import_menu_open);
    let figma_row = row_point(&host, 0);
    assert!(host.apply_press(figma_row.x, figma_row.y, vw, vh));
    assert!(!host.editor_state().editor_ui.import_menu_open);
    assert!(host.editor_state().editor_ui.figma_import_open);
    assert_eq!(
        host.editor_state().editor_ui.import_source,
        op_editor_core::figma_import_state::ImportSource::Figma
    );
    assert_eq!(host.editor_state().editor_ui.pending_file_action, None);

    // HTML row → the same modal, showing the HTML source.
    let mut host = WidgetHostNative::new();
    assert!(host.apply_press(button_center.x, button_center.y, vw, vh));
    let html_row = row_point(&host, 1);
    assert!(host.apply_press(html_row.x, html_row.y, vw, vh));
    assert!(!host.editor_state().editor_ui.import_menu_open);
    assert!(host.editor_state().editor_ui.figma_import_open);
    assert_eq!(
        host.editor_state().editor_ui.import_source,
        op_editor_core::figma_import_state::ImportSource::Html
    );
    assert_eq!(host.editor_state().editor_ui.pending_file_action, None);

    // A second press on the button closes the menu instead of reopening.
    let mut host = WidgetHostNative::new();
    assert!(host.apply_press(button_center.x, button_center.y, vw, vh));
    assert!(host.apply_press(button_center.x, button_center.y, vw, vh));
    assert!(!host.editor_state().editor_ui.import_menu_open);
}

#[test]
fn right_rail_host_routing_tracks_design_selection_and_code_fallback() {
    let mut host = WidgetHostNative::new();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n-rail","name":"Rail probe","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.editor_state_mut().chat.minimize();
    host.editor_state_mut().editor_ui.property_tab = op_editor_core::PropertyTab::Design;
    host.editor_state_mut().clear_selection();

    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let property_width = host.editor_state().editor_ui.property_panel_width;
    let old_property_gutter_x = viewport_w - property_width;
    let press_y = TOP_BAR_HEIGHT + 180.0;

    let (canvas_left, _, canvas_width, _) = host.canvas_region(viewport_w, viewport_h);
    assert_eq!(
        canvas_left + canvas_width,
        viewport_w,
        "empty Design selection must release the entire right rail to the canvas"
    );
    assert_eq!(
        host.panel_resize_hover(old_property_gutter_x, press_y, viewport_w),
        None,
        "the former property-panel gutter must not remain interactive"
    );

    let right_edge_x = viewport_w - 2.0;
    assert!(host.over_canvas(right_edge_x, press_y, viewport_w, viewport_h));
    host.apply_press(right_edge_x, press_y, viewport_w, viewport_h);
    assert!(
        host.marquee_drag.is_some(),
        "a blank press at the viewport's right edge must route to the canvas"
    );
    assert!(!host.is_resizing_panel());
    assert!(host.apply_release_with_viewport(viewport_w, viewport_h));

    host.editor_state_mut()
        .set_single_selection(NodeId::new("n-rail"));
    assert!(host.editor_state().right_rail_visible());
    let (selected_left, _, selected_width, _) = host.canvas_region(viewport_w, viewport_h);
    assert_eq!(selected_left + selected_width, old_property_gutter_x);
    assert!(matches!(
        host.panel_resize_hover(old_property_gutter_x, press_y, viewport_w),
        Some(crate::widget_host::PanelResizeKind::PropertyLeft)
    ));
    assert!(!host.over_canvas(right_edge_x, press_y, viewport_w, viewport_h));

    host.editor_state_mut().clear_selection();
    host.editor_state_mut().editor_ui.property_tab = op_editor_core::PropertyTab::Code;
    assert!(
        host.editor_state().right_rail_visible(),
        "Code remains selection-independent"
    );
    let (code_left, _, code_width, _) = host.canvas_region(viewport_w, viewport_h);
    assert_eq!(code_left + code_width, old_property_gutter_x);
    assert!(matches!(
        host.panel_resize_hover(old_property_gutter_x, press_y, viewport_w),
        Some(crate::widget_host::PanelResizeKind::PropertyLeft)
    ));
}
