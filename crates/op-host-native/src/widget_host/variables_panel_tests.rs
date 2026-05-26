use super::{
    helpers::{TOOLBAR_INSET_X, TOOLBAR_INSET_Y},
    WidgetHostNative,
};
use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};
use op_editor_core::editor_ui_state::VariableRowFocus;
use op_editor_ui::widgets::{TOOLBAR_WIDTH, TOP_BAR_HEIGHT};

const VIEWPORT_W: f32 = 1280.0;
const VIEWPORT_H: f32 = 900.0;

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
    assert_eq!(host.editor_state().ui.property_input_draft, "color-1");
    assert!(
        !host.editor_state().ui.property_draft_select_all,
        "variable-name rename should start with a caret, not selected text"
    );

    assert!(host.apply_text('b'));
    assert_eq!(host.editor_state().ui.property_input_draft, "color-1b");
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
    assert_eq!(host.editor_state().ui.property_input_draft, "0");
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
    assert_eq!(host.editor_state().ui.property_input_draft, "");
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
    host.editor_state_mut().ui.property_input_draft = "brand".into();

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
    host.editor_state_mut().ui.property_input_draft = "color-1".into();
    host.editor_state_mut().ui.property_draft_select_all = true;

    assert!(host.apply_send());

    let vars = host.editor_state().doc.variables.as_ref().unwrap();
    assert!(vars.contains_key("color-1"));
    assert_eq!(host.editor_state().ui.property_input_draft, "color-1");
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
    host.editor_state_mut().ui.property_input_draft = "color-1".into();
    host.editor_state_mut().ui.property_caret_pos = 5;

    assert!(host.apply_text('X'));

    assert_eq!(host.editor_state().ui.property_input_draft, "colorX-1");
    assert_eq!(host.editor_state().ui.property_caret_pos, 6);
}

#[test]
fn variables_panel_name_edit_backspace_and_delete_are_caret_aware() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state_mut().ui.property_input_draft = "color-1".into();
    host.editor_state_mut().ui.property_caret_pos = 5;

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().ui.property_input_draft, "colo-1");
    assert_eq!(host.editor_state().ui.property_caret_pos, 4);

    assert!(host.apply_delete());
    assert_eq!(host.editor_state().ui.property_input_draft, "colo1");
    assert_eq!(host.editor_state().ui.property_caret_pos, 4);
}

#[test]
fn variables_panel_name_edit_arrow_keys_move_caret() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state_mut().ui.property_input_draft = "color-1".into();
    host.editor_state_mut().ui.property_caret_pos = "color-1".len();

    assert!(host.apply_property_caret(false));
    assert_eq!(host.editor_state().ui.property_caret_pos, "color-".len());
    assert!(host.apply_property_caret(false));
    assert_eq!(host.editor_state().ui.property_caret_pos, "color".len());
    assert!(host.apply_property_caret(true));
    assert_eq!(host.editor_state().ui.property_caret_pos, "color-".len());
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
    host.editor_state_mut().ui.property_input_draft.clear();

    assert!(host.apply_send());

    let vars = host.editor_state().doc.variables.as_ref().unwrap();
    assert!(vars.contains_key("color-1"));
    assert_eq!(host.editor_state().ui.property_input_draft, "color-1");
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
    state.ui.property_input_draft = "Mode".into();

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
    state.ui.property_input_draft = "Dark".into();

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
    assert!(!host.editor_state().ui.property_draft_select_all);
    assert!(host.apply_text('d'));
    assert_eq!(host.editor_state().ui.property_input_draft, "Theme-1d");
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
    assert!(!host.editor_state().ui.property_draft_select_all);
    assert!(host.apply_text('d'));
    assert_eq!(host.editor_state().ui.property_input_draft, "Variant-1d");
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
    assert_eq!(host.editor_state().ui.property_input_draft, "中文");
    assert_eq!(
        host.editor_state().ui.property_caret_pos,
        "中文".len(),
        "caret is a valid byte offset for multibyte names"
    );

    assert!(host.apply_backspace());
    assert_eq!(host.editor_state().ui.property_input_draft, "中");
    assert_eq!(host.editor_state().ui.property_caret_pos, "中".len());
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

#[test]
fn variables_panel_add_theme_button_creates_theme() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + 24.0,
        rect.origin.y + 22.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    let state = host.editor_state();
    let themes = state.doc.themes.as_ref().unwrap();
    assert_eq!(themes.get("Theme-1").unwrap(), &vec!["Default".to_string()]);
    assert_eq!(
        state.ui.variables.active_theme.get("Theme-1"),
        Some(&"Default".to_string())
    );
}

#[test]
fn variables_panel_preset_button_toggles_menu() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + 82.0,
        rect.origin.y + 22.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));
    assert!(host.editor_state().editor_ui.variables_preset_menu_open);

    assert!(host.apply_press(
        rect.origin.x + 82.0,
        rect.origin.y + 58.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));
    assert!(!host.editor_state().editor_ui.variables_preset_menu_open);
}

#[test]
fn variables_panel_add_variant_button_appends_variant() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    state
        .doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into()]);
    state
        .ui
        .variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + rect.size.x - 24.0,
        rect.origin.y + 44.0 + 18.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert_eq!(
        host.editor_state()
            .doc
            .themes
            .as_ref()
            .unwrap()
            .get("Theme-1")
            .unwrap(),
        &vec!["Default".to_string(), "Variant-1".to_string()]
    );
}

#[test]
fn variables_panel_theme_tab_selects_current_axis() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    let themes = state.doc.themes.get_or_insert_with(Default::default);
    themes.insert("Theme-1".into(), vec!["Default".into()]);
    themes.insert("Theme-2".into(), vec!["Default".into(), "Compact".into()]);
    state
        .ui
        .variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    state.editor_ui.variables_current_axis = Some("Theme-1".into());
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + 120.0,
        rect.origin.y + 22.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    let state = host.editor_state();
    assert_eq!(
        state.editor_ui.variables_current_axis.as_deref(),
        Some("Theme-2")
    );
    assert_eq!(
        state.ui.variables.active_theme.get("Theme-2"),
        Some(&"Default".to_string())
    );
}

#[test]
fn variables_panel_add_variant_uses_current_axis() {
    let mut host = WidgetHostNative::new();
    let state = host.editor_state_mut();
    state.editor_ui.variables_panel_open = true;
    let themes = state.doc.themes.get_or_insert_with(Default::default);
    themes.insert("Theme-1".into(), vec!["Default".into()]);
    themes.insert("Theme-2".into(), vec!["Default".into()]);
    state
        .ui
        .variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    state.editor_ui.variables_current_axis = Some("Theme-2".into());
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + rect.size.x - 24.0,
        rect.origin.y + 44.0 + 18.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    let themes = host.editor_state().doc.themes.as_ref().unwrap();
    assert_eq!(themes.get("Theme-1").unwrap(), &vec!["Default".to_string()]);
    assert_eq!(
        themes.get("Theme-2").unwrap(),
        &vec!["Default".to_string(), "Variant-1".to_string()]
    );
}

#[test]
fn variables_panel_footer_add_menu_creates_color_variable() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + 62.0,
        rect.origin.y + rect.size.y - 20.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));
    assert!(host.editor_state().editor_ui.variables_add_menu_open);

    assert!(host.apply_press(
        rect.origin.x + 30.0,
        rect.origin.y + rect.size.y - 40.0 - 90.0 - 6.0 + 15.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    let vars = host.editor_state().doc.variables.as_ref().unwrap();
    assert!(vars.contains_key("color-1"));
    let state = host.editor_state();
    assert_eq!(
        state
            .doc
            .themes
            .as_ref()
            .and_then(|themes| themes.get("Theme-1")),
        Some(&vec!["Default".to_string()])
    );
    assert_eq!(
        state.ui.variables.active_theme.get("Theme-1"),
        Some(&"Default".to_string())
    );
    assert_eq!(
        state.editor_ui.variables_current_axis.as_deref(),
        Some("Theme-1")
    );
    assert!(!host.editor_state().editor_ui.variables_add_menu_open);
}

#[test]
fn variables_panel_add_number_focuses_new_default_value() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + 62.0,
        rect.origin.y + rect.size.y - 20.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));
    assert!(host.apply_press(
        rect.origin.x + 30.0,
        rect.origin.y + rect.size.y - 40.0 - 90.0 - 6.0 + 45.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert_eq!(
        host.editor_state().editor_ui.variable_row_focus,
        Some(VariableRowFocus::NumberCell { row: 0, variant: 0 })
    );
    assert_eq!(host.editor_state().ui.property_input_draft, "0");
    assert!(host.editor_state().ui.property_draft_select_all);
    assert!(host.apply_text('2'));
    assert!(host.apply_text('1'));
    assert!(host.apply_text('3'));
    assert_eq!(host.editor_state().ui.property_input_draft, "213");
    assert!(host.apply_send());

    let vars = host.editor_state().doc.variables.as_ref().unwrap();
    let VariableValue::Themed(values) = &vars.get("number-1").unwrap().value else {
        panic!("editing the focused default column should write a themed value");
    };
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].value, VariableScalar::Num(213.0));
    assert_eq!(
        values[0]
            .theme
            .as_ref()
            .and_then(|theme| theme.get("Theme-1")),
        Some(&"Default".to_string())
    );
}

#[test]
fn variables_panel_add_string_focuses_new_default_value() {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut().editor_ui.variables_panel_open = true;
    let rect = host.variables_panel_rect(VIEWPORT_W, VIEWPORT_H).unwrap();

    assert!(host.apply_press(
        rect.origin.x + 62.0,
        rect.origin.y + rect.size.y - 20.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));
    assert!(host.apply_press(
        rect.origin.x + 30.0,
        rect.origin.y + rect.size.y - 40.0 - 90.0 - 6.0 + 75.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    assert_eq!(
        host.editor_state().editor_ui.variable_row_focus,
        Some(VariableRowFocus::StringCell { row: 0, variant: 0 })
    );
    assert_eq!(host.editor_state().ui.property_input_draft, "string");
    assert!(host.editor_state().ui.property_draft_select_all);
    assert!(host.apply_text('a'));
    assert_eq!(host.editor_state().ui.property_input_draft, "a");
    assert!(host.apply_send());

    let vars = host.editor_state().doc.variables.as_ref().unwrap();
    let VariableValue::Themed(values) = &vars.get("string-1").unwrap().value else {
        panic!("editing the focused default column should write a themed value");
    };
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].value, VariableScalar::Str("a".into()));
    assert_eq!(
        values[0]
            .theme
            .as_ref()
            .and_then(|theme| theme.get("Theme-1")),
        Some(&"Default".to_string())
    );
}
