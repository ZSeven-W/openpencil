use super::{
    helpers::{TOOLBAR_INSET_X, TOOLBAR_INSET_Y},
    WidgetHostNative,
};
use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};
use op_editor_core::editor_ui_state::VariableRowFocus;
use op_editor_core::{ButtonPressTarget, VariablesPanelButton};
use op_editor_ui::widgets::variables_panel::{VariablesPanel, VariablesPanelHit};
use op_editor_ui::widgets::{TOOLBAR_WIDTH, TOP_BAR_HEIGHT};
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1280.0;
const VIEWPORT_H: f32 = 900.0;

fn point_for_hit(host: &WidgetHostNative, want: &VariablesPanelHit) -> (f32, f32) {
    let rect = host
        .variables_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("variables panel rect");
    let panel = VariablesPanel::for_editor(host.editor_state());
    let mut y = rect.origin.y;
    while y < rect.origin.y + rect.size.y {
        let mut x = rect.origin.x;
        while x < rect.origin.x + rect.size.x {
            if panel
                .hit_test(rect, Point2D::new(x, y))
                .is_some_and(|hit| &hit == want)
            {
                return (x, y);
            }
            x += 2.0;
        }
        y += 2.0;
    }
    panic!("no panel point maps to {want:?}");
}

#[test]
fn variables_panel_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    assert!(host.editor_state_mut().create_variable(
        "spacing",
        VariableKind::Number,
        VariableScalar::Num(8.0),
    ));

    let (x, y) = point_for_hit(&host, &VariablesPanelHit::AddTheme);
    assert!(host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.pressed_button,
        Some(ButtonPressTarget::VariablesPanel(
            VariablesPanelButton::AddTheme
        ))
    );

    assert!(host.apply_release_with_viewport(VIEWPORT_W, VIEWPORT_H));
    assert_eq!(host.editor_state().editor_ui.pressed_button, None);
}

#[test]
fn variables_panel_floats_next_to_toolbar_like_ts() {
    let mut host = WidgetHostNative::new();
    assert!(host.editor_state_mut().create_variable(
        "spacing",
        VariableKind::Number,
        VariableScalar::Num(8.0),
    ));
    host.editor_state_mut().editor_ui.variables_panel_open = true;

    let (canvas_left, _canvas_top, _canvas_w, _canvas_h) =
        host.canvas_region(VIEWPORT_W, VIEWPORT_H);
    let panel_x = canvas_left + TOOLBAR_INSET_X + TOOLBAR_WIDTH + 8.0;
    let panel_y = TOP_BAR_HEIGHT + TOOLBAR_INSET_Y;
    let row_point_x = panel_x + 16.0;
    let row_point_y = panel_y + 44.0 + 36.0 + 18.0;

    assert!(host.apply_press(row_point_x, row_point_y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_focus,
        Some(VariableRowFocus::Number(0))
    );
}

#[test]
fn variables_panel_name_pill_double_click_starts_rename_without_opening_color_picker() {
    let mut host = WidgetHostNative::new();
    assert!(host.editor_state_mut().create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    let x = rect.origin.x + 16.0 + 42.0;
    let y = rect.origin.y + 44.0 + 36.0 + 22.0;

    host.set_now_ms(100);
    assert!(host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H));
    assert!(host.editor_state().ui.color_picker.is_none());
    assert_eq!(host.editor_state().editor_ui.variable_row_focus, None);

    host.set_now_ms(240);
    assert!(host.apply_press(x, y, VIEWPORT_W, VIEWPORT_H));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_focus,
        Some(VariableRowFocus::Name(0))
    );
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "color-1"
    );
    assert!(
        !host
            .editor_state()
            .editor_ui
            .variable_row_input
            .is_select_all(),
        "variable-name rename should start with a caret, not selected text"
    );

    assert!(host.apply_text('b'));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "color-1b"
    );
}

#[test]
fn variables_panel_number_value_cell_edits_clicked_variant_only() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into(), "Variant-1".into()]);
    state.editor_ui.variables_current_axis = Some("Theme-1".into());
    state
        .ui
        .variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    assert!(state.create_variable("number", VariableKind::Number, VariableScalar::Num(0.0),));
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    let second_variant_x = rect.origin.x + 16.0 + 220.0 + 262.0 + 12.0;
    let first_row_y = rect.origin.y + 44.0 + 36.0 + 22.0;

    assert!(host.apply_press(second_variant_x, first_row_y, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(
        host.editor_state().editor_ui.variable_row_focus,
        Some(VariableRowFocus::NumberCell { row: 0, variant: 1 })
    );
    assert_eq!(host.editor_state().editor_ui.variable_row_input.text(), "0");
    assert!(host.apply_text('7'));
    assert!(host.apply_send());

    let def = host
        .editor_state()
        .doc
        .variables
        .as_ref()
        .unwrap()
        .get("number")
        .unwrap();
    let VariableValue::Themed(values) = &def.value else {
        panic!("variant edit should convert scalar variable to themed values");
    };
    let default = values
        .iter()
        .find(|entry| entry.theme.as_ref().and_then(|t| t.get("Theme-1")).unwrap() == "Default")
        .unwrap();
    let variant = values
        .iter()
        .find(|entry| entry.theme.as_ref().and_then(|t| t.get("Theme-1")).unwrap() == "Variant-1")
        .unwrap();
    assert_eq!(default.value, VariableScalar::Num(0.0));
    assert_eq!(variant.value, VariableScalar::Num(7.0));
}

#[test]
fn variables_panel_string_value_cell_edits_clicked_variant_only() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into(), "Variant-1".into()]);
    state.editor_ui.variables_current_axis = Some("Theme-1".into());
    state
        .ui
        .variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    assert!(state.create_variable(
        "string",
        VariableKind::String,
        VariableScalar::Str(String::new()),
    ));
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    let second_variant_x = rect.origin.x + 16.0 + 220.0 + 262.0 + 12.0;
    let first_row_y = rect.origin.y + 44.0 + 36.0 + 22.0;

    assert!(host.apply_press(second_variant_x, first_row_y, VIEWPORT_W, VIEWPORT_H));

    assert_eq!(
        host.editor_state().editor_ui.variable_row_focus,
        Some(VariableRowFocus::StringCell { row: 0, variant: 1 })
    );
    assert_eq!(host.editor_state().editor_ui.variable_row_input.text(), "");
    assert!(host.apply_text('a'));
    assert!(host.apply_text('b'));
    assert!(host.apply_send());

    let def = host
        .editor_state()
        .doc
        .variables
        .as_ref()
        .unwrap()
        .get("string")
        .unwrap();
    let VariableValue::Themed(values) = &def.value else {
        panic!("variant edit should convert scalar variable to themed values");
    };
    let default = values
        .iter()
        .find(|entry| entry.theme.as_ref().and_then(|t| t.get("Theme-1")).unwrap() == "Default")
        .unwrap();
    let variant = values
        .iter()
        .find(|entry| entry.theme.as_ref().and_then(|t| t.get("Theme-1")).unwrap() == "Variant-1")
        .unwrap();
    assert_eq!(default.value, VariableScalar::Str(String::new()));
    assert_eq!(variant.value, VariableScalar::Str("ab".into()));
}

#[test]
fn variables_panel_name_edit_commits_variable_rename() {
    let mut host = WidgetHostNative::new();
    assert!(host.editor_state_mut().create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    host.editor_state_mut().editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .set_text("brand");

    host.commit_variable_row_focus_if_any_pub();

    let vars = host.editor_state().doc.variables.as_ref().unwrap();
    assert!(vars.contains_key("brand"));
    assert!(!vars.contains_key("color-1"));
}

#[test]
fn variables_panel_name_edit_enter_does_not_clear_unchanged_name() {
    let mut host = WidgetHostNative::new();
    assert!(host.editor_state_mut().create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    host.editor_state_mut().editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .set_text("color-1");
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .select_all();

    assert!(host.apply_send());

    let vars = host.editor_state().doc.variables.as_ref().unwrap();
    assert!(vars.contains_key("color-1"));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "color-1"
    );
}

#[test]
fn variables_panel_name_edit_typing_inserts_at_caret() {
    let mut host = WidgetHostNative::new();
    assert!(host.editor_state_mut().create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    host.editor_state_mut().editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .set_text("color-1");
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .set_caret(5, 0);

    assert!(host.apply_text('X'));

    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "colorX-1"
    );
    assert_eq!(host.editor_state().editor_ui.variable_row_input.caret(), 6);
}

#[test]
fn variables_panel_name_edit_backspace_and_delete_are_caret_aware() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .set_text("color-1");
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .set_caret(5, 0);

    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "colo-1"
    );
    assert_eq!(host.editor_state().editor_ui.variable_row_input.caret(), 4);

    assert!(host.apply_delete());
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "colo1"
    );
    assert_eq!(host.editor_state().editor_ui.variable_row_input.caret(), 4);
}

#[test]
fn variables_panel_name_edit_arrow_keys_move_caret() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .set_text("color-1");

    assert!(host.apply_property_caret(false));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.caret(),
        "color-".len()
    );
    assert!(host.apply_property_caret(false));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.caret(),
        "color".len()
    );
    assert!(host.apply_property_caret(true));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.caret(),
        "color-".len()
    );
}

#[test]
fn variables_panel_name_edit_enter_with_empty_name_restores_old_name() {
    let mut host = WidgetHostNative::new();
    assert!(host.editor_state_mut().create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    host.editor_state_mut().editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state_mut()
        .editor_ui
        .variable_row_input
        .set_text("");

    assert!(host.apply_send());

    let vars = host.editor_state().doc.variables.as_ref().unwrap();
    assert!(vars.contains_key("color-1"));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "color-1"
    );
}

#[test]
fn variables_panel_color_picker_anchors_near_clicked_value_cell() {
    let mut host = WidgetHostNative::new();
    assert!(host.editor_state_mut().create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    ));
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();
    let click_x = rect.origin.x + 16.0 + 220.0 + 4.0;
    let click_y = rect.origin.y + 44.0 + 36.0 + 18.0;

    assert!(host.apply_press(click_x, click_y, VIEWPORT_W, VIEWPORT_H));

    let state = host
        .editor_state()
        .ui
        .color_picker
        .clone()
        .expect("color picker open");
    let picker =
        op_editor_ui::widgets::color_picker::ColorPicker::for_state(host.editor_state(), state);
    let picker_rect = picker.rect(VIEWPORT_W, VIEWPORT_H);
    assert!(
        picker_rect.origin.x >= click_x && picker_rect.origin.x <= click_x + 40.0,
        "picker should open near clicked variable swatch; click_x={click_x}, picker_x={}",
        picker_rect.origin.x
    );
}

#[test]
fn variables_panel_theme_menu_rename_commits_axis_name() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into()]);
    state.editor_ui.variables_theme_rename_axis = Some("Theme-1".into());
    state.editor_ui.variables_header_input.set_text("Mode");

    assert!(host.apply_send());

    let themes = host.editor_state().doc.themes.as_ref().unwrap();
    assert!(themes.contains_key("Mode"));
    assert!(!themes.contains_key("Theme-1"));
}

#[test]
fn variables_panel_variant_menu_rename_commits_variant_name() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into(), "Variant-1".into()]);
    state.editor_ui.variables_current_axis = Some("Theme-1".into());
    state.editor_ui.variables_variant_rename_value = Some("Variant-1".into());
    state.editor_ui.variables_header_input.set_text("Dark");

    assert!(host.apply_send());

    assert_eq!(
        host.editor_state()
            .doc
            .themes
            .as_ref()
            .unwrap()
            .get("Theme-1")
            .unwrap(),
        &vec!["Default".to_string(), "Dark".to_string()]
    );
}

#[test]
fn variables_panel_theme_menu_rename_starts_with_caret_not_selection() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into()]);
    state.editor_ui.variables_theme_menu_axis = Some("Theme-1".into());
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + 18.0,
        rect.origin.y + 44.0 + 22.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert_eq!(
        host.editor_state().editor_ui.variables_theme_rename_axis,
        Some("Theme-1".into())
    );
    assert!(!host
        .editor_state()
        .editor_ui
        .variables_header_input
        .is_select_all());
    assert!(host.apply_text('d'));
    assert_eq!(
        host.editor_state().editor_ui.variables_header_input.text(),
        "Theme-1d"
    );
}

#[test]
fn variables_panel_variant_menu_rename_starts_with_caret_not_selection() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into(), "Variant-1".into()]);
    state.editor_ui.variables_current_axis = Some("Theme-1".into());
    state.editor_ui.variables_variant_menu_value = Some("Variant-1".into());
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + 510.0,
        rect.origin.y + 99.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert_eq!(
        host.editor_state().editor_ui.variables_variant_rename_value,
        Some("Variant-1".into())
    );
    assert!(!host
        .editor_state()
        .editor_ui
        .variables_header_input
        .is_select_all());
    assert!(host.apply_text('d'));
    assert_eq!(
        host.editor_state().editor_ui.variables_header_input.text(),
        "Variant-1d"
    );
}

#[test]
fn variables_panel_header_rename_owns_keyboard_input() {
    let mut host = WidgetHostNative::new();

    host.editor_state_mut()
        .editor_ui
        .variables_theme_rename_axis = Some("Theme-1".into());
    assert!(
        host.input_active_pub(),
        "theme rename should route keyboard input to the inline editor"
    );

    let state = host.editor_state_mut();
    state.editor_ui.variables_theme_rename_axis = None;
    state.editor_ui.variables_variant_rename_value = Some("Default".into());
    assert!(
        host.input_active_pub(),
        "variant rename should route keyboard input to the inline editor"
    );
}

#[test]
fn variables_panel_header_rename_accepts_unicode_text() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into()]);
    state.editor_ui.variables_theme_rename_axis = Some("Theme-1".into());

    assert!(host.apply_text('中'));
    assert!(host.apply_text('文'));
    assert_eq!(
        host.editor_state().editor_ui.variables_header_input.text(),
        "中文"
    );
    assert_eq!(
        host.editor_state().editor_ui.variables_header_input.caret(),
        "中文".len(),
        "caret is a valid byte offset for multibyte names"
    );

    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state().editor_ui.variables_header_input.text(),
        "中"
    );
    assert_eq!(
        host.editor_state().editor_ui.variables_header_input.caret(),
        "中".len()
    );
}

#[test]
fn variables_panel_blank_click_closes_open_header_menus() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into()]);
    state.editor_ui.variables_theme_menu_axis = Some("Theme-1".into());
    state.editor_ui.variables_variant_menu_value = Some("Default".into());
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + rect.size.x - 24.0,
        rect.origin.y + rect.size.y / 2.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert_eq!(
        host.editor_state().editor_ui.variables_theme_menu_axis,
        None
    );
    assert_eq!(
        host.editor_state().editor_ui.variables_variant_menu_value,
        None
    );
}
