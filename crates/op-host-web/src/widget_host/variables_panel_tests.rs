//! Web-host VariablesPanel wiring tests (#21) — the floating panel
//! dispatches the same open/close, search typing, scroll, row menu,
//! and color-cell paths the native host does.

use super::WidgetHost;
use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};
use op_editor_core::editor_ui_state::VariableRowFocus;
use op_editor_core::{own_bounds, ButtonPressTarget, NodeId, PropertyFocus, VariablesPanelButton};
use op_editor_ui::widgets::variables_panel::{
    VariablesPanel, VariablesPanelHit, VariablesResizeEdge,
};
use op_editor_ui::widgets::{LayerPanel, LayerPanelHit};
use op_editor_ui::Point2D;

const W: f32 = 1280.0;
const H: f32 = 900.0;

fn two_variant_color_host() -> WidgetHost {
    let mut host = WidgetHost::new();
    let state = &mut host.editor_state;
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
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#112233".into()),
    ));
    host
}

fn seed(host: &mut WidgetHost, json: &str) {
    let doc = jian_ops_schema::load_str(json)
        .expect("fixture JSON parses")
        .value;
    host.editor_state = op_editor_core::EditorState::from_document(doc);
    host.editor_state_dirty = true;
}

fn selected_two_variant_color_host() -> WidgetHost {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    let state = &mut host.editor_state;
    state.set_single_selection(NodeId::new("n62"));
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
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#112233".into()),
    ));
    host
}

/// Locate the panel point that hit-tests to `want` by scanning.
fn point_for_hit(host: &WidgetHost, want: &VariablesPanelHit) -> (f32, f32) {
    let rect = host.variables_panel_rect(W, H).unwrap();
    let panel = VariablesPanel::for_editor(&host.editor_state);
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

fn point_for_layer_row(host: &WidgetHost, id: &str) -> Point2D {
    let panel = LayerPanel::from_editor(&host.editor_state);
    let rect = host.layer_panel_rect(H);
    let regions = panel.regions(rect);
    let mut y = regions.layers_rows_top + 2.0;
    while y < regions.layers_rows_top + regions.layers_view_h {
        let point = Point2D::new(rect.origin.x + 48.0, y);
        if matches!(
            panel.hit_test(rect, point),
            Some(LayerPanelHit::Layer(node_id)) if node_id == NodeId::new(id)
        ) {
            return point;
        }
        y += 2.0;
    }
    panic!("no layer row point found for {id}");
}

fn themed_value_for<'a>(
    state: &'a op_editor_core::EditorState,
    name: &str,
    variant: &str,
) -> Option<&'a VariableScalar> {
    let def = state.doc.variables.as_ref()?.get(name)?;
    let VariableValue::Themed(entries) = &def.value else {
        return None;
    };
    entries
        .iter()
        .find(|e| {
            e.theme
                .as_ref()
                .and_then(|t| t.get("Theme-1"))
                .is_some_and(|v| v == variant)
        })
        .map(|e| &e.value)
}

#[test]
fn variables_panel_press_sets_and_release_clears_pressed_button() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.variables_panel_open = true;
    assert!(host.editor_state.create_variable(
        "spacing",
        VariableKind::Number,
        VariableScalar::Num(8.0),
    ));

    let (x, y) = point_for_hit(&host, &VariablesPanelHit::AddTheme);
    assert!(host.apply_press(x, y, W, H));
    assert_eq!(
        host.editor_state.editor_ui.pressed_button,
        Some(ButtonPressTarget::VariablesPanel(
            VariablesPanelButton::AddTheme
        ))
    );

    assert!(host.apply_release_with_viewport(W, H));
    assert_eq!(host.editor_state.editor_ui.pressed_button, None);
}

#[test]
fn close_button_closes_floating_panel() {
    let mut host = two_variant_color_host();
    let (x, y) = point_for_hit(&host, &VariablesPanelHit::Close);
    assert!(host.apply_press(x, y, W, H));
    assert!(!host.editor_state.editor_ui.variables_panel_open);
}

#[test]
fn outside_press_falls_through_instead_of_closing() {
    // The floating panel is not modal: a press outside its rect
    // reaches the canvas.
    let mut host = two_variant_color_host();
    let rect = host.variables_panel_rect(W, H).unwrap();
    let below_x = rect.origin.x + rect.size.x / 2.0;
    let below_y = rect.origin.y + rect.size.y + 40.0;
    let _ = host.apply_press(below_x, below_y, W, H);
    assert!(
        host.editor_state.editor_ui.variables_panel_open,
        "panel stays open on an outside press"
    );
}

#[test]
fn shape_picker_hover_wins_when_variables_panel_overlaps() {
    let mut host = WidgetHost::new();
    host.last_viewport_w = W;
    host.last_viewport_h = H;
    host.editor_state.editor_ui.variables_panel_open = true;
    host.editor_state.editor_ui.shape_picker.open = true;

    let picker_rect = host.shape_picker_rect(W, H);
    let vars_rect = host
        .variables_panel_rect(W, H)
        .expect("variables panel rect");
    let picker = op_editor_ui::widgets::shape_picker::ShapePicker::for_editor_ui(
        &host.editor_state.editor_ui,
    );
    let x = picker_rect.origin.x + picker_rect.size.x / 2.0;
    let mut y = picker_rect.origin.y + 2.0;
    let mut hover = None;
    while y < picker_rect.origin.y + picker_rect.size.y {
        if let op_editor_ui::widgets::shape_picker::SelectHit::Row(idx) =
            picker.hit_popup(picker_rect, Point2D::new(x, y))
        {
            hover = Some((idx, y));
            break;
        }
        y += 2.0;
    }
    let (expected_hover, y) = hover.expect("shape picker row point");
    let point = Point2D::new(x, y);
    assert!(
        vars_rect.contains(point),
        "fixture should overlap the floating variables panel"
    );

    assert!(host.apply_cursor_move(x, y));
    assert_eq!(
        host.editor_state.editor_ui.shape_picker.hover,
        Some(expected_hover)
    );
    assert_eq!(host.editor_state.editor_ui.variables_panel_hover, None);
}

#[test]
fn color_swatch_press_targets_clicked_variant() {
    let mut host = two_variant_color_host();
    let (x, y) = point_for_hit(
        &host,
        &VariablesPanelHit::ColorSwatch { row: 0, variant: 1 },
    );
    assert!(host.apply_press(x, y, W, H));
    let picker = host
        .editor_state
        .ui
        .color_picker
        .as_ref()
        .expect("swatch press opens picker");
    assert_eq!(
        picker.variable_theme,
        Some(("Theme-1".to_string(), "Variant-1".to_string()))
    );
    assert!(host.editor_state.color_picker_set_hsv(0.0, 1.0, 1.0));
    assert_eq!(
        themed_value_for(&host.editor_state, "color-1", "Variant-1"),
        Some(&VariableScalar::Str("#ff0000".into()))
    );
    assert_eq!(
        themed_value_for(&host.editor_state, "color-1", "Default"),
        Some(&VariableScalar::Str("#112233".into()))
    );
}

#[test]
fn color_hex_cell_inline_edit_commits_to_variant() {
    let mut host = two_variant_color_host();
    let (x, y) = point_for_hit(&host, &VariablesPanelHit::ValueCell { row: 0, variant: 1 });
    assert!(host.apply_press(x, y, W, H));
    assert_eq!(
        host.editor_state.editor_ui.variable_row_focus,
        Some(VariableRowFocus::ColorCell { row: 0, variant: 1 })
    );
    assert_eq!(
        host.editor_state.editor_ui.variable_row_input.text(),
        "#112233"
    );
    for _ in 0..7 {
        assert!(host.apply_backspace());
    }
    for c in "#abcdef".chars() {
        assert!(host.apply_text(c));
    }
    assert!(!host.apply_text('g'));
    assert!(host.apply_send());
    assert_eq!(
        themed_value_for(&host.editor_state, "color-1", "Variant-1"),
        Some(&VariableScalar::Str("#abcdef".into()))
    );
}

#[test]
fn row_menu_delete_and_rename_work_on_web() {
    let mut host = two_variant_color_host();
    let (bx, by) = point_for_hit(&host, &VariablesPanelHit::RowMenuToggle(0));
    assert!(host.apply_press(bx, by, W, H));
    assert_eq!(host.editor_state.editor_ui.variables_row_menu, Some(0));

    let (rx, ry) = point_for_hit(&host, &VariablesPanelHit::RowMenuRename(0));
    assert!(host.apply_press(rx, ry, W, H));
    assert_eq!(
        host.editor_state.editor_ui.variable_row_focus,
        Some(VariableRowFocus::Name(0))
    );
    assert!(host
        .editor_state
        .editor_ui
        .variable_row_input
        .is_select_all());
    // Cancel the rename focus, then delete through the menu.
    host.commit_variable_row_focus_if_any();
    let (bx, by) = point_for_hit(&host, &VariablesPanelHit::RowMenuToggle(0));
    assert!(host.apply_press(bx, by, W, H));
    let (dx, dy) = point_for_hit(&host, &VariablesPanelHit::RowMenuDelete(0));
    assert!(host.apply_press(dx, dy, W, H));
    assert!(!host
        .editor_state
        .doc
        .variables
        .as_ref()
        .is_some_and(|vars| vars.contains_key("color-1")));
    // One undo restores it.
    assert!(host.editor_state.undo());
    assert!(host
        .editor_state
        .doc
        .variables
        .as_ref()
        .is_some_and(|vars| vars.contains_key("color-1")));
}

#[test]
fn variable_row_edit_commits_prior_property_focus_on_web() {
    let mut host = WidgetHost::new();
    seed(
        &mut host,
        r##"{ "version": "0.8.0", "children": [
              {"type":"rectangle","id":"n62","name":"Wide",
               "x":40,"y":40,"width":180,"height":120,
               "fill":[{"type":"solid","color":"#BDC7D9"}]}
        ]}"##,
    );
    host.editor_state.set_single_selection(NodeId::new("n62"));
    assert!(host.editor_state.create_variable(
        "spacing",
        VariableKind::Number,
        VariableScalar::Num(8.0)
    ));
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");

    assert!(host.press_variable_row(0, 0.0, 0.0));

    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state.ui.property_focus.is_none());
    assert_eq!(
        host.editor_state.editor_ui.variable_row_focus,
        Some(VariableRowFocus::Number(0))
    );
    assert_eq!(host.editor_state.editor_ui.variable_row_input.text(), "8");
}

#[test]
fn layer_right_press_commits_variable_row_focus_on_web_like_native() {
    let mut host = selected_two_variant_color_host();
    host.editor_state.editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    host.editor_state
        .editor_ui
        .variable_row_input
        .set_text("brand-color");

    let point = point_for_layer_row(&host, "n62");
    assert!(host.apply_right_press(point.x, point.y, W, H));

    assert!(host.editor_state.editor_ui.variable_row_focus.is_none());
    let vars = host.editor_state.doc.variables.as_ref().unwrap();
    assert!(
        vars.contains_key("brand-color"),
        "right press should commit the pending variable row rename"
    );
    assert!(!vars.contains_key("color-1"));
}

#[test]
fn theme_rename_commits_prior_property_focus_on_web() {
    let mut host = selected_two_variant_color_host();
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");
    let (tx, ty) = point_for_hit(&host, &VariablesPanelHit::ToggleThemeMenu("Theme-1".into()));
    assert!(host.apply_press(tx, ty, W, H));
    let (rx, ry) = point_for_hit(&host, &VariablesPanelHit::ThemeMenuRename("Theme-1".into()));

    assert!(host.apply_press(rx, ry, W, H));

    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state.ui.property_focus.is_none());
    assert_eq!(
        host.editor_state
            .editor_ui
            .variables_theme_rename_axis
            .as_deref(),
        Some("Theme-1")
    );
    assert_eq!(
        host.editor_state.editor_ui.variables_header_input.text(),
        "Theme-1"
    );
}

#[test]
fn variant_rename_commits_prior_property_focus_on_web() {
    let mut host = selected_two_variant_color_host();
    host.editor_state.ui.property_focus = Some(PropertyFocus::SizeW);
    host.editor_state.ui.property_input.set_text("321");
    let (tx, ty) = point_for_hit(
        &host,
        &VariablesPanelHit::ToggleVariantMenu("Default".into()),
    );
    assert!(host.apply_press(tx, ty, W, H));
    let (rx, ry) = point_for_hit(
        &host,
        &VariablesPanelHit::VariantMenuRename("Default".into()),
    );

    assert!(host.apply_press(rx, ry, W, H));

    let bounds = own_bounds(host.editor_state.selected_node().unwrap());
    assert_eq!(bounds.w, 321.0);
    assert!(host.editor_state.ui.property_focus.is_none());
    assert_eq!(
        host.editor_state
            .editor_ui
            .variables_variant_rename_value
            .as_deref(),
        Some("Default")
    );
    assert_eq!(
        host.editor_state.editor_ui.variables_header_input.text(),
        "Default"
    );
}

#[test]
fn search_typing_filters_rows_on_web() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.variables_panel_open = true;
    for i in 1..=7 {
        assert!(host.editor_state.create_variable(
            &format!("color-{i}"),
            VariableKind::Color,
            VariableScalar::Str("#000000".into()),
        ));
    }
    assert!(host.editor_state.create_variable(
        "spacing",
        VariableKind::Number,
        VariableScalar::Num(8.0),
    ));
    let (sx, sy) = point_for_hit(&host, &VariablesPanelHit::SearchBox);
    assert!(host.apply_press(sx, sy, W, H));
    assert!(host.editor_state.editor_ui.variables_search_focus);
    for c in "spac".chars() {
        assert!(host.apply_text(c));
    }
    assert_eq!(host.editor_state.editor_ui.variables_search, "spac");
    let panel = VariablesPanel::for_editor(&host.editor_state);
    assert_eq!(panel.row_count(), 1);
    assert!(host.apply_backspace());
    assert_eq!(host.editor_state.editor_ui.variables_search, "spa");
    // Escape blurs; the filter persists.
    assert!(host.apply_escape());
    assert!(!host.editor_state.editor_ui.variables_search_focus);
    assert_eq!(host.editor_state.editor_ui.variables_search, "spa");
}

#[test]
fn wheel_scrolls_rows_and_resize_drag_resizes() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.variables_panel_open = true;
    for i in 1..=20 {
        assert!(host.editor_state.create_variable(
            &format!("color-{i:02}"),
            VariableKind::Color,
            VariableScalar::Str("#000000".into()),
        ));
    }
    let rect = host.variables_panel_rect(W, H).unwrap();
    let cx = rect.origin.x + rect.size.x / 2.0;
    let cy = rect.origin.y + rect.size.y / 2.0;
    assert!(host.apply_wheel(cx, cy, -60.0, W, H));
    assert!(host.editor_state.editor_ui.variables_scroll.offset > 0.0);
    // Huge values clamp.
    assert!(host.apply_wheel(cx, cy, -1.0e6, W, H));
    let panel = VariablesPanel::for_editor(&host.editor_state);
    assert_eq!(
        host.editor_state.editor_ui.variables_scroll.offset,
        panel.max_scroll(rect)
    );
    assert!(host.apply_wheel(cx, cy, 1.0e6, W, H));
    assert_eq!(host.editor_state.editor_ui.variables_scroll.offset, 0.0);

    // Right-edge press arms a resize; drag narrows; release ends.
    let edge_x = rect.origin.x + rect.size.x - 2.0;
    let edge_y = rect.origin.y + rect.size.y / 2.0;
    assert!(host.apply_press(edge_x, edge_y, W, H));
    assert_eq!(host.variables_resize, Some(VariablesResizeEdge::Right));
    assert!(host.apply_cursor_move(edge_x - 200.0, edge_y));
    let resized = host.variables_panel_rect(W, H).unwrap();
    assert!(resized.size.x < rect.size.x);
    assert!(resized.size.x >= 480.0);
    assert!(host.apply_release_with_viewport(W, H));
    assert_eq!(host.variables_resize, None);
}

#[test]
fn number_cell_edit_targets_clicked_variant_on_web() {
    let mut host = WidgetHost::new();
    let state = &mut host.editor_state;
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
    assert!(state.create_variable("spacing", VariableKind::Number, VariableScalar::Num(4.0)));

    let (x, y) = point_for_hit(&host, &VariablesPanelHit::ValueCell { row: 0, variant: 1 });
    assert!(host.apply_press(x, y, W, H));
    assert_eq!(
        host.editor_state.editor_ui.variable_row_focus,
        Some(VariableRowFocus::NumberCell { row: 0, variant: 1 })
    );
    // Replace the seeded draft and commit.
    while host.apply_backspace() {}
    assert!(host.apply_text('9'));
    assert!(host.apply_send());
    assert_eq!(
        themed_value_for(&host.editor_state, "spacing", "Variant-1"),
        Some(&VariableScalar::Num(9.0))
    );
    assert_eq!(
        themed_value_for(&host.editor_state, "spacing", "Default"),
        Some(&VariableScalar::Num(4.0))
    );
}
