use super::WidgetHostNative;
use jian_ops_schema::variable::{VariableScalar, VariableValue};
use op_editor_core::editor_ui_state::VariableRowFocus;

const VIEWPORT_W: f32 = 1280.0;
const VIEWPORT_H: f32 = 900.0;

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
        rect.origin.y + 22.0,
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
    assert_eq!(host.editor_state().editor_ui.variable_row_input.text(), "0");
    assert!(host
        .editor_state()
        .editor_ui
        .variable_row_input
        .is_select_all());
    assert!(host.apply_text('2'));
    assert!(host.apply_text('1'));
    assert!(host.apply_text('3'));
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "213"
    );
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
    assert_eq!(
        host.editor_state().editor_ui.variable_row_input.text(),
        "string"
    );
    assert!(host
        .editor_state()
        .editor_ui
        .variable_row_input
        .is_select_all());
    assert!(host.apply_text('a'));
    assert_eq!(host.editor_state().editor_ui.variable_row_input.text(), "a");
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
