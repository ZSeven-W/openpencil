//! Toolbar panel actions, chat-input caret/selection, the Escape overlay
//! ladder, rename caret fall-through, and the floating-pill / dialog
//! press-feedback round trips.
//!
//! Split out of `input_tests.rs` to keep every file under the repo's
//! 800-line cap.

use super::*;

#[test]
fn toolbar_panel_actions_open_variables_and_design_panels() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed(
        &mut host,
        r#"{"version":"1.0.0","children":[{"type":"rectangle","id":"n1","name":"n1","x":0,"y":0,"width":100,"height":50}]}"#,
    );
    host.editor_state_mut().editor_ui.property_tab = op_editor_core::PropertyTab::Design;
    host.editor_state_mut().chat.minimize();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;

    let (variables_x, variables_y) = toolbar_action_point_for_test(
        &host,
        op_editor_ui::widgets::ToolbarAction::ToggleVariablesPanel,
        viewport_w,
        viewport_h,
    );
    let toolbar = op_editor_ui::widgets::Toolbar::for_editor(host.editor_state());
    assert_eq!(
        toolbar.hit_test(
            host.toolbar_rect(viewport_w, viewport_h),
            op_editor_ui::Point2D::new(variables_x, variables_y)
        ),
        Some(op_editor_ui::widgets::ToolbarHit::Action(
            op_editor_ui::widgets::ToolbarAction::ToggleVariablesPanel
        ))
    );
    // The VariablesPanel is a floating canvas overlay, not a right-rail
    // tab. With no node selected the rail stays hidden, so toggling
    // Variables must not affect its visibility.
    assert!(!host.editor_state().right_rail_visible());
    let (canvas_left, _, canvas_width, _) = host.canvas_region(viewport_w, viewport_h);
    assert_eq!(
        canvas_width,
        viewport_w - canvas_left,
        "an empty Design selection must not reserve a blank right rail"
    );
    assert!(host.apply_press(variables_x, variables_y, viewport_w, viewport_h));
    assert!(host.editor_state().editor_ui.variables_panel_open);
    assert!(!host.editor_state().right_rail_visible());
    assert_eq!(
        host.editor_state().editor_ui.property_tab,
        op_editor_core::PropertyTab::Design
    );
    assert!(host.apply_press(variables_x, variables_y, viewport_w, viewport_h));
    assert!(!host.editor_state().editor_ui.variables_panel_open);
    assert!(!host.editor_state().right_rail_visible());

    let (design_x, design_y) = toolbar_action_point_for_test(
        &host,
        op_editor_ui::widgets::ToolbarAction::ToggleDesignPanel,
        viewport_w,
        viewport_h,
    );
    assert!(host.apply_press(design_x, design_y, viewport_w, viewport_h));
    assert!(host.editor_state().editor_ui.design_md_panel.open);
}

#[test]
fn explicit_variables_toolbar_opens_floating_variables_panel() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed(
        &mut host,
        r##"{"version":"1.0.0","children":[{"type":"frame","id":"frame-1","name":"Frame","x":0,"y":0,"width":100,"height":50}]}"##,
    );
    host.editor_state_mut().selection.anchor = NodeId::new("frame-1");
    host.editor_state_mut().selection.set = vec![NodeId::new("frame-1")];
    host.editor_state_mut().create_variable(
        "spacing-md",
        jian_ops_schema::variable::VariableKind::Number,
        jian_ops_schema::variable::VariableScalar::Num(16.0),
    );
    host.editor_state_mut().editor_ui.variables_panel_open = true;

    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let vars_rect = host.variables_panel_rect(viewport_w, viewport_h).unwrap();

    assert!(
        host.editor_state().right_rail_visible(),
        "selected frame should still own the right rail"
    );
    assert!(host.apply_press(
        vars_rect.origin.x + 16.0,
        vars_rect.origin.y + 44.0 + 36.0 + 18.0,
        viewport_w,
        viewport_h
    ));
    assert!(host.editor_state().editor_ui.variables_panel_open);
    assert!(host.editor_state().editor_ui.variable_row_focus.is_some());
}

#[test]
fn chat_input_click_clears_select_all_without_erasing_text() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    host.editor_state_mut()
        .chat
        .set_input_text("设计一个现代的移动端登录页面");
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut().chat.select_all_input(0);
    // The input block is bottom-anchored in the rail's tall column, so
    // the probe comes from the live text rect, not the old floating
    // panel's fixed offset from the top.
    let text = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state())
        .input_text_rect(rect);

    assert!(host.apply_press(
        text.origin.x + 64.0,
        text.origin.y + text.size.y / 2.0,
        viewport_w,
        viewport_h
    ));

    assert_eq!(
        host.editor_state().chat.input.text(),
        "设计一个现代的移动端登录页面"
    );
    assert!(host.editor_state().chat.focused);
    assert!(host.editor_state().chat.input.highlight_range().is_none());
}

#[test]
fn chat_input_drag_selects_partial_text_and_replaces_it() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let viewport_w = 1200.0;
    let viewport_h = 800.0;
    let rect = host.ai_chat_rect(viewport_w, viewport_h).unwrap();
    host.editor_state_mut().chat.set_input_text("abcdef");
    host.editor_state_mut().chat.focused = true;
    // Offsets are measured from the live text rect's own inner inset
    // (the old constants were offsets from the floating panel's edge).
    let text = op_editor_ui::widgets::AIChatPlaceholder::from_editor(host.editor_state())
        .input_text_rect(rect);
    let text_x = text.origin.x + 8.0;
    let text_y = text.origin.y + text.size.y / 2.0;

    assert!(host.apply_press(text_x + 6.6, text_y, viewport_w, viewport_h));
    assert_eq!(
        host.editor_state().chat.input.selection(),
        jian_core::text_input::Selection {
            anchor: 1,
            focus: 1
        }
    );
    assert!(host.apply_cursor_move(text_x + 19.8, text_y));
    assert_eq!(
        host.editor_state().chat.input.selection(),
        jian_core::text_input::Selection {
            anchor: 1,
            focus: 3
        }
    );
    assert!(host.apply_release());
    assert!(host.apply_text('X'));

    assert_eq!(host.editor_state().chat.input.text(), "aXdef");
}

#[test]
fn escape_closes_one_overlay_per_press_in_priority_order() {
    // Codex CONCERN-2 regression: Escape used to clear all
    // three pickers in a single press. TS parity is one-at-a-
    // time, in the order property-focus → locale → shape →
    // fill-type → chat → selection.
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::PositionX);
    host.editor_state_mut().ui.property_input.set_text("12");
    host.editor_state_mut().editor_ui.locale_picker.open = true;
    host.editor_state_mut().editor_ui.shape_picker.open = true;
    host.editor_state_mut().editor_ui.fill_type_picker.open = true;
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut()
        .set_single_selection(NodeId::new("n10"));

    // 1. Property focus clears first.
    assert!(host.apply_escape());
    assert!(host.editor_state().ui.property_focus.is_none());
    assert!(host.editor_state().ui.property_input.text().is_empty());
    assert!(host.editor_state().editor_ui.locale_picker.open);

    // 2. Locale picker next.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.locale_picker.open);
    assert!(host.editor_state().editor_ui.shape_picker.open);

    // 3. Shape picker.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.shape_picker.open);
    assert!(host.editor_state().editor_ui.fill_type_picker.open);

    // 4. Fill-type picker.
    assert!(host.apply_escape());
    assert!(!host.editor_state().editor_ui.fill_type_picker.open);
    assert!(host.editor_state().chat.focused);

    // 5. Chat focus.
    assert!(host.apply_escape());
    assert!(!host.editor_state().chat.focused);
    assert!(!host.editor_state().selection.is_empty());

    // 6. Selection.
    assert!(host.apply_escape());
    assert!(host.editor_state().selection.is_empty());

    // 7. Nothing left — returns false.
    assert!(!host.apply_escape());
}

#[test]
fn rename_caret_arrows_move_caret_then_fall_through() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed(
        &mut host,
        &three_rects(
            [
                (0.0, 0.0, 10.0, 10.0),
                (20.0, 0.0, 10.0, 10.0),
                (40.0, 0.0, 10.0, 10.0),
            ],
            ["ab", "b", "c"],
        ),
    );
    assert!(host
        .editor_state_mut()
        .start_rename_layer(NodeId::new("ab")));
    // Draft "ab" seeds caret at the end (2).
    assert_eq!(
        host.editor_state()
            .ui
            .layer_rename
            .as_ref()
            .unwrap()
            .input
            .caret(),
        2
    );
    // Left arrow during rename is consumed and moves the caret.
    assert!(host.apply_rename_caret(false));
    assert_eq!(
        host.editor_state()
            .ui
            .layer_rename
            .as_ref()
            .unwrap()
            .input
            .caret(),
        1
    );
    assert!(host.apply_rename_caret(true));
    assert_eq!(
        host.editor_state()
            .ui
            .layer_rename
            .as_ref()
            .unwrap()
            .input
            .caret(),
        2
    );
    // With no rename active the arrow falls through (not consumed).
    host.editor_state_mut().rename_cancel();
    assert!(!host.apply_rename_caret(false));
}

#[test]
fn status_bar_search_click_frames_content_in_viewport() {
    // Three rects spread across doc space (union ≈ x[100,400] y[100,300]).
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    seed(
        &mut host,
        &three_rects(
            [
                (100.0, 100.0, 100.0, 100.0),
                (300.0, 200.0, 100.0, 100.0),
                (150.0, 150.0, 50.0, 50.0),
            ],
            ["a", "b", "c"],
        ),
    );
    // Pan + zoom far away so the design is off-screen.
    host.editor_state_mut().viewport.pan_x = -5000.0;
    host.editor_state_mut().viewport.pan_y = -5000.0;
    host.editor_state_mut().viewport.zoom = 0.2;

    let (vw, vh) = (1200.0, 800.0);
    let (_, _, canvas_w, canvas_h) = host.canvas_region(vw, vh);
    let r = host
        .status_bar_rect(vw, vh)
        .expect("status bar visible at this size");
    // Click the search icon (left section of the pill).
    let consumed = host.apply_press(r.origin.x + 5.0, r.origin.y + r.size.y / 2.0, vw, vh);

    assert!(consumed, "search-icon click must be consumed");
    let v = host.editor_state().viewport;
    let expected_pan_x = canvas_w / 2.0 - 250.0;
    let expected_pan_y = canvas_h / 2.0 - 200.0;
    assert!((v.zoom - 1.0).abs() < 1e-3, "zoom {}", v.zoom);
    assert!((v.pan_x - expected_pan_x).abs() < 1e-2, "pan_x {}", v.pan_x);
    assert!((v.pan_y - expected_pan_y).abs() < 1e-2, "pan_y {}", v.pan_y);
}

#[test]
fn status_bar_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (vw, vh) = (1200.0, 800.0);
    let r = host
        .status_bar_rect(vw, vh)
        .expect("status bar visible at this size");
    let x = r.origin.x + 5.0;
    let y = r.origin.y + r.size.y / 2.0;

    assert!(host.apply_press(x, y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::StatusBar(
            op_editor_core::StatusBarButton::Search
        ))
    );

    assert!(host.apply_release_with_viewport(vw, vh));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn export_dialog_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (vw, vh) = (1200.0, 800.0);
    host.editor_state_mut().editor_ui.export_dialog_open = true;
    let dlg = op_editor_ui::widgets::ExportDialog::centered(vw, vh);
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

    assert!(host.apply_press(point.x, point.y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::ExportDialog(
            op_editor_core::ExportDialogButton::Scale(1)
        ))
    );

    assert!(host.apply_release_with_viewport(vw, vh));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn figma_import_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    let (vw, vh) = (1200.0, 800.0);
    host.editor_state_mut().editor_ui.figma_import_open = true;
    let modal =
        op_editor_ui::widgets::figma_import::FigmaImportModal::for_editor(host.editor_state());
    let panel = modal.rect(vw, vh);
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

    assert!(host.apply_press(point.x, point.y, vw, vh));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(op_editor_core::ButtonPressTarget::FigmaImport(
            op_editor_core::FigmaImportButton::DropZone
        ))
    );

    assert!(host.apply_release_with_viewport(vw, vh));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}
