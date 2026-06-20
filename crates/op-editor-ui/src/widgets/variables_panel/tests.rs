use super::*;
use crate::widgets::PaintCx;
use crate::Color;
use jian_ops_schema::variable::{ThemedValue, VariableScalar, VariableValue};
use std::collections::BTreeMap;

#[derive(Default)]
struct TextCaptureBackend {
    texts: Vec<String>,
    origins: Vec<Point2D>,
    fills: Vec<(Rect, Color)>,
    round_fills: Vec<(Rect, f32, Color)>,
    round_strokes: Vec<(Rect, f32, Color, f32)>,
    svg_origins: Vec<Point2D>,
    svg_sizes: Vec<f32>,
}

impl crate::RenderBackend for TextCaptureBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.fills.push((rect, color));
    }
    fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
    fn draw_text(&mut self, layout: &crate::TextLayout, origin: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push(run.content.clone());
            self.origins.push(origin);
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: Color) {
        self.round_fills.push((rect, radius, color));
    }
    fn stroke_round_rect(&mut self, rect: Rect, radius: f32, color: Color, width: f32) {
        self.round_strokes.push((rect, radius, color, width));
    }
    fn stroke_svg_path(&mut self, _: &str, origin: Point2D, size: f32, _: Color, _: f32) {
        self.svg_origins.push(origin);
        self.svg_sizes.push(size);
    }
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn color_eq(a: Color, b: Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
}

fn caret_fills(fills: &[(Rect, Color)], theme: Theme) -> Vec<Rect> {
    fills
        .iter()
        .filter_map(|(rect, color)| {
            let shared_input_caret = (rect.size.y - 16.0).abs() < 0.01;
            let legacy_caret = (rect.size.y - 18.0).abs() < 0.01;
            (color_eq(*color, theme.foreground)
                && (rect.size.x - 1.5).abs() < 0.01
                && (shared_input_caret || legacy_caret))
                .then_some(*rect)
        })
        .collect()
}

fn state_with_three_vars() -> EditorState {
    let mut s = EditorState::new();
    s.create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#ff8800".into()),
    );
    s.create_variable(
        "spacing-md",
        VariableKind::Number,
        VariableScalar::Num(16.0),
    );
    s.create_variable("is-dark", VariableKind::Boolean, VariableScalar::Bool(true));
    s.ui.variables
        .active_theme
        .insert("mode".into(), "dark".into());
    s
}

fn state_with_ts_like_themes() -> EditorState {
    let mut s = EditorState::new();
    s.doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into(), "Variant-1".into()]);
    s.doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-2".into(), vec!["Default".into(), "Compact".into()]);
    s.ui.variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    s
}

#[test]
fn row_count_matches_variable_count() {
    let s = state_with_three_vars();
    let p = VariablesPanel::for_editor(&s);
    assert_eq!(p.row_count(), 3);
}

#[test]
fn panel_does_not_paint_drop_shadow() {
    let p = VariablesPanel::for_editor(&EditorState::new());
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, 480.0),
    };
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    p.paint(&mut cx, rect);

    assert!(
        !backend.round_fills.iter().any(|(fill, radius, color)| {
            (*radius - PANEL_RADIUS).abs() < 0.01
                && fill.size == rect.size
                && fill.origin.x == rect.origin.x
                && fill.origin.y > rect.origin.y
                && color.r == 0.0
                && color.g == 0.0
                && color.b == 0.0
                && color.a > 0.0
        }),
        "variables panel should not paint a detached drop shadow behind itself"
    );
}

#[test]
fn axis_count_reflects_active_theme() {
    let s = state_with_three_vars();
    let p = VariablesPanel::for_editor(&s);
    assert_eq!(p.axis_count(), 1);
}

#[test]
fn theme_tabs_follow_document_axes_like_ts() {
    let s = state_with_ts_like_themes();
    let p = VariablesPanel::for_editor(&s);

    assert_eq!(p.theme_tab_labels(), vec!["Theme-1", "Theme-2"]);
    assert_eq!(p.active_axis_label(), "Theme-1");
}

#[test]
fn variant_columns_follow_active_axis_values_like_ts() {
    let s = state_with_ts_like_themes();
    let p = VariablesPanel::for_editor(&s);

    assert_eq!(p.variant_column_labels(), vec!["Default", "Variant-1"]);
    assert_eq!(p.variant_column_count(), 2);
}

#[test]
fn variables_without_themes_show_implicit_default_theme() {
    let mut s = EditorState::new();
    s.create_variable(
        "color-1",
        VariableKind::Color,
        VariableScalar::Str("#000000".into()),
    );
    let p = VariablesPanel::for_editor(&s);

    assert_eq!(p.theme_tab_labels(), vec!["Theme-1"]);
    assert_eq!(p.variant_column_labels(), vec!["Default"]);
}

#[test]
fn theme_tab_hit_targets_document_axis_like_ts() {
    let s = state_with_ts_like_themes();
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };

    match p.hit_test(rect, Point2D::new(120.0, 22.0)) {
        Some(VariablesPanelHit::ThemeTab(axis)) => assert_eq!(axis, "Theme-2"),
        other => panic!("expected ThemeTab(Theme-2), got {other:?}"),
    }
}

#[test]
fn active_theme_tab_hit_toggles_theme_menu_like_ts() {
    let s = state_with_ts_like_themes();
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };

    match p.hit_test(rect, Point2D::new(22.0, 22.0)) {
        Some(VariablesPanelHit::ToggleThemeMenu(axis)) => assert_eq!(axis, "Theme-1"),
        other => panic!("expected ToggleThemeMenu(Theme-1), got {other:?}"),
    }
}

#[test]
fn variant_header_hit_toggles_variant_menu_like_ts() {
    let s = state_with_ts_like_themes();
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let point = Point2D::new(value_column_x(rect) + 12.0, HEADER_HEIGHT + 20.0);

    match p.hit_test(rect, point) {
        Some(VariablesPanelHit::ToggleVariantMenu(value)) => assert_eq!(value, "Default"),
        other => panic!("expected ToggleVariantMenu(Default), got {other:?}"),
    }
}

#[test]
fn open_theme_and_variant_menus_route_rename_rows() {
    let mut s = state_with_ts_like_themes();
    s.editor_ui.variables_theme_menu_axis = Some("Theme-1".into());
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    match p.hit_test(rect, Point2D::new(18.0, HEADER_HEIGHT + 22.0)) {
        Some(VariablesPanelHit::ThemeMenuRename(axis)) => assert_eq!(axis, "Theme-1"),
        other => panic!("expected ThemeMenuRename(Theme-1), got {other:?}"),
    }

    s.editor_ui.variables_theme_menu_axis = None;
    s.editor_ui.variables_variant_menu_value = Some("Variant-1".into());
    let p = VariablesPanel::for_editor(&s);
    let menu_x = value_column_x(rect) + variant_column_width(rect, 2) + 8.0;
    match p.hit_test(
        rect,
        Point2D::new(menu_x, HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + 20.0),
    ) {
        Some(VariablesPanelHit::VariantMenuRename(value)) => assert_eq!(value, "Variant-1"),
        other => panic!("expected VariantMenuRename(Variant-1), got {other:?}"),
    }
}

#[test]
fn theme_rename_input_reserves_header_space_like_ts() {
    let mut s = EditorState::new();
    s.doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into()]);
    s.ui.variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    s.editor_ui.variables_current_axis = Some("Theme-1".into());
    s.editor_ui.variables_theme_rename_axis = Some("Theme-1".into());
    s.editor_ui.variables_header_input.set_text("ewe");
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let input_end = rect.origin.x + PAD_X - 2.0 + (label_width("ewe", 13.0) + 28.0).max(96.0);

    assert!(
        p.add_theme_rect(rect).origin.x >= input_end + 4.0,
        "add theme button must sit after the active rename input"
    );
}

#[test]
fn theme_rename_caret_hides_at_blink_off_phase() {
    let mut s = EditorState::new();
    s.doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into()]);
    s.ui.variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    s.editor_ui.variables_current_axis = Some("Theme-1".into());
    s.editor_ui.variables_theme_rename_axis = Some("Theme-1".into());
    s.editor_ui.variables_header_input.set_text("Theme-1");
    s.editor_ui
        .variables_header_input
        .set_caret("Theme-1".len(), 0);
    let p = VariablesPanel::for_editor_at(&s, 500);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    p.paint(&mut cx, rect);

    assert!(
        caret_fills(&backend.fills, p.theme).is_empty(),
        "variables-panel inline rename caret should blink off at the off phase"
    );
}

#[test]
fn variant_rename_caret_blinks_in_painted_header_input() {
    let mut s = EditorState::new();
    s.doc
        .themes
        .get_or_insert_with(Default::default)
        .insert("Theme-1".into(), vec!["Default".into()]);
    s.ui.variables
        .active_theme
        .insert("Theme-1".into(), "Default".into());
    s.editor_ui.variables_current_axis = Some("Theme-1".into());
    s.editor_ui.variables_variant_rename_value = Some("Default".into());
    s.editor_ui.variables_header_input.set_text("Default");
    s.editor_ui
        .variables_header_input
        .set_caret("Default".len(), 0);
    let p_visible = VariablesPanel::for_editor_at(&s, 0);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p_visible.intrinsic_height()),
    };
    let mut visible_backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut visible_backend,
    };
    p_visible.paint(&mut cx, rect);

    assert!(
        !caret_fills(&visible_backend.fills, p_visible.theme).is_empty(),
        "variant header input should paint its caret at the blink anchor"
    );

    let p_hidden = VariablesPanel::for_editor_at(&s, 500);
    let mut hidden_backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut hidden_backend,
    };
    p_hidden.paint(&mut cx, rect);

    assert!(
        caret_fills(&hidden_backend.fills, p_hidden.theme).is_empty(),
        "variant header input caret should disappear at the blink off phase"
    );
}

#[test]
fn editing_variable_name_caret_uses_raw_name_like_ts() {
    let mut s = state_with_three_vars();
    s.editor_ui.variable_row_focus = Some(VariableRowFocus::Name(0));
    s.editor_ui.variable_row_input.set_text("color-1");
    s.editor_ui.variable_row_input.set_caret(3, 0);
    let p = VariablesPanel::for_editor_at(&s, 0);

    assert_eq!(p.name_caret_for_row(0), Some(3));
}

#[test]
fn editing_value_cell_uses_shared_input_chrome() {
    let mut s = EditorState::new();
    s.create_variable("spacing", VariableKind::Number, VariableScalar::Num(16.0));
    s.editor_ui.variable_row_focus = Some(VariableRowFocus::Number(0));
    s.editor_ui.variable_row_input.set_text("24");
    s.editor_ui.variable_row_input.set_caret("24".len(), 0);
    let p = VariablesPanel::for_editor_at(&s, 0);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    p.paint(&mut cx, rect);

    let row_y = p.rows_start_y(rect);
    let value_input = backend
        .round_strokes
        .iter()
        .find_map(|(stroke, radius, color, width)| {
            (color_eq(*color, p.theme.primary)
                && (*radius - 8.0).abs() < 0.01
                && (*width - 1.5).abs() < 0.01
                && (stroke.origin.x - (value_column_x(rect) - 8.0)).abs() < 0.01
                && (stroke.origin.y - (row_y + 7.0)).abs() < 0.01
                && (stroke.size.y - 30.0).abs() < 0.01)
                .then_some(*stroke)
        })
        .expect("editing a variable value should use the same focused input chrome as header/name editing");

    assert!(
        value_input.size.x <= 160.0,
        "short variable value edits should not stretch across the full value column; got {}",
        value_input.size.x
    );
}

#[test]
fn variable_name_display_paints_two_literal_hyphens_like_ts() {
    let s = state_with_three_vars();
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    p.paint(&mut cx, rect);

    assert!(
        !backend.texts.iter().any(|text| text == "--color-1"),
        "display mode should not shape the variable prefix as one text run"
    );
    let idx = backend
        .texts
        .iter()
        .position(|text| text == "color-1")
        .expect("painted variable name text");
    assert!(idx >= 2, "name should be preceded by two hyphen runs");
    assert_eq!(backend.texts[idx - 2], "-");
    assert_eq!(backend.texts[idx - 1], "-");
    assert!(
        backend.origins[idx - 1].x - backend.origins[idx - 2].x >= 8.0,
        "the two variable prefix hyphens should be visually separated"
    );
    assert!(
        backend.origins[idx].x - backend.origins[idx - 1].x >= 8.0,
        "the variable name should start after the second prefix hyphen"
    );
}

#[test]
fn variable_rows_resolve_per_variant_values_like_ts() {
    let mut s = state_with_ts_like_themes();
    let mut default_theme = BTreeMap::new();
    default_theme.insert("Theme-1".to_string(), "Default".to_string());
    let mut variant_theme = BTreeMap::new();
    variant_theme.insert("Theme-1".to_string(), "Variant-1".to_string());
    s.doc.variables.get_or_insert_with(Default::default).insert(
        "color-1".into(),
        jian_ops_schema::variable::VariableDefinition {
            kind: VariableKind::Color,
            value: VariableValue::Themed(vec![
                ThemedValue {
                    value: VariableScalar::Str("#c81919".into()),
                    theme: Some(default_theme),
                },
                ThemedValue {
                    value: VariableScalar::Str("#0066ff".into()),
                    theme: Some(variant_theme),
                },
            ]),
        },
    );
    let p = VariablesPanel::for_editor(&s);

    assert_eq!(
        p.variant_scalar_for(&p.rows[0], "Theme-1", "Default"),
        Some(&VariableScalar::Str("#c81919".into()))
    );
    assert_eq!(
        p.variant_scalar_for(&p.rows[0], "Theme-1", "Variant-1"),
        Some(&VariableScalar::Str("#0066ff".into()))
    );
}

#[test]
fn intrinsic_height_grows_with_rows_and_chips() {
    let s_empty = EditorState::new();
    let p = VariablesPanel::for_editor(&s_empty);
    let empty_h = p.intrinsic_height();
    assert!(
        (empty_h - (HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + FOOTER_HEIGHT)).abs() < f32::EPSILON
    );
    let s = state_with_three_vars();
    let p2 = VariablesPanel::for_editor(&s);
    assert!(p2.intrinsic_height() > empty_h);
}

#[test]
fn axis_dropdown_hit_routes_to_named_value() {
    let mut s = state_with_three_vars();
    s.doc.themes.get_or_insert_with(Default::default).insert(
        "mode".into(),
        vec!["light".into(), "dark".into(), "system".into()],
    );
    let mut p = VariablesPanel::for_editor(&s);
    p.dropdown_open = Some("mode".into());
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let chip = p.chip_rect(rect, 0);
    let menu_y = chip.origin.y + chip.size.y + 4.0;
    let click_y = menu_y + DROPDOWN_ROW_HEIGHT * 0.5;
    let click_x = chip.origin.x + 10.0;
    match p.hit_test(rect, Point2D::new(click_x, click_y)) {
        Some(VariablesPanelHit::AxisDropdownItem { axis, value }) => {
            assert_eq!(axis, "mode");
            assert_eq!(value, "light");
        }
        other => panic!("expected AxisDropdownItem for row 0, got {other:?}"),
    }
    let click_y_sys = menu_y + DROPDOWN_ROW_HEIGHT * 2.5;
    match p.hit_test(rect, Point2D::new(click_x, click_y_sys)) {
        Some(VariablesPanelHit::AxisDropdownItem { axis, value }) => {
            assert_eq!(axis, "mode");
            assert_eq!(value, "system");
        }
        other => panic!("expected AxisDropdownItem for row 2, got {other:?}"),
    }
}

#[test]
fn hit_test_returns_row_index_for_in_row_click() {
    let s = state_with_three_vars();
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let y = HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + ROW_HEIGHT * 1.0 + ROW_HEIGHT / 2.0;
    match p.hit_test(rect, Point2D::new(PAD_X + 4.0, y)) {
        Some(VariablesPanelHit::Row(1)) => {}
        other => panic!("expected Row(1), got {other:?}"),
    }
}

#[test]
fn hit_test_returns_name_cell_for_variable_name_pill() {
    let s = state_with_three_vars();
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let y = HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + ROW_HEIGHT / 2.0;
    match p.hit_test(rect, Point2D::new(PAD_X + 42.0, y)) {
        Some(VariablesPanelHit::NameCell(0)) => {}
        other => panic!("expected NameCell(0), got {other:?}"),
    }
    match p.hit_test(rect, Point2D::new(PAD_X + 4.0, y)) {
        Some(VariablesPanelHit::Row(0)) => {}
        other => panic!("expected Row(0), got {other:?}"),
    }
}

#[test]
fn hit_test_returns_variant_menu_for_column_header_click() {
    let s = state_with_three_vars();
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let y = HEADER_HEIGHT + 8.0 + CHIP_HEIGHT / 2.0;
    match p.hit_test(rect, Point2D::new(value_column_x(rect) + 4.0, y)) {
        Some(VariablesPanelHit::ToggleVariantMenu(value)) => assert_eq!(value, "Default"),
        other => panic!("expected ToggleVariantMenu(Default), got {other:?}"),
    }
}

#[test]
fn hit_test_returns_value_cell_for_variant_value_click() {
    let mut s = state_with_ts_like_themes();
    s.create_variable("number", VariableKind::Number, VariableScalar::Num(0.0));
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let col_w = variant_column_width(rect, 2);
    let x = value_column_x(rect) + col_w + 12.0;
    let y = HEADER_HEIGHT + COLUMN_HEADER_HEIGHT + ROW_HEIGHT / 2.0;

    match p.hit_test(rect, Point2D::new(x, y)) {
        Some(VariablesPanelHit::ValueCell { row, variant }) => {
            assert_eq!(row, 0);
            assert_eq!(variant, 1);
        }
        other => panic!("expected ValueCell(row=0, variant=1), got {other:?}"),
    }
}

#[test]
fn panel_buttons_are_hittable() {
    let p = VariablesPanel::for_editor(&EditorState::new());
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, 480.0),
    };
    for point in [
        Point2D::new(24.0, 22.0),
        Point2D::new(82.0, 22.0),
        Point2D::new(rect.size.x - 24.0, HEADER_HEIGHT + 18.0),
        Point2D::new(62.0, rect.size.y - 20.0),
    ] {
        assert!(p.hit_test(rect, point).is_some(), "{point:?}");
    }
}

#[test]
fn header_controls_use_shared_vertical_center() {
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, 480.0),
    };
    let center = header::control_center_y(rect);
    assert_eq!(center, 22.0);
    assert_eq!(header::icon_origin(rect, 16.0, 16.0).y, 14.0);
    assert_eq!(header::icon_origin(rect, 118.0, 12.0).y, 16.0);
    assert!((header::text_baseline(rect, 14.0) - 27.0).abs() < 0.1);
}

#[test]
fn preset_chevron_sits_after_localized_label() {
    let mut s = EditorState::new();
    s.editor_ui.locale = Locale::ZhCn;
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    p.paint(&mut cx, rect);

    let preset = p.preset_rect(rect);
    let expected_x = preset.origin.x + 29.0 + label_width("预设", 13.0) + 7.0;
    let chevron = backend
        .svg_origins
        .iter()
        .zip(backend.svg_sizes.iter())
        .find(|(origin, size)| {
            (**size - 11.0).abs() < f32::EPSILON
                && origin.x >= preset.origin.x
                && origin.x < preset.origin.x + preset.size.x
        })
        .map(|(origin, _)| *origin)
        .expect("preset chevron should paint inside preset rect");

    assert!(
        (chevron.x - expected_x).abs() <= 1.0,
        "preset chevron should follow the localized label; got {}, expected {}",
        chevron.x,
        expected_x
    );
    assert!(
        ((chevron.y + 11.0 / 2.0) - header::control_center_y(rect)).abs() <= 0.1,
        "preset chevron should be vertically centered"
    );
}

#[test]
fn preset_chevron_clears_cjk_label_width() {
    let mut s = EditorState::new();
    s.editor_ui.locale = Locale::ZhCn;
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, p.intrinsic_height()),
    };
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    p.paint(&mut cx, rect);

    let preset = p.preset_rect(rect);
    let label_x = preset.origin.x + 29.0;
    let rendered_label_right = label_x + 2.0 * 13.0;
    let chevron = backend
        .svg_origins
        .iter()
        .zip(backend.svg_sizes.iter())
        .find(|(origin, size)| {
            (**size - 11.0).abs() < f32::EPSILON
                && origin.x >= preset.origin.x
                && origin.x < preset.origin.x + preset.size.x
        })
        .map(|(origin, _)| *origin)
        .expect("preset chevron should paint inside preset rect");

    assert!(
        chevron.x >= rendered_label_right + 6.0,
        "preset chevron should clear the CJK label; got {}, expected at least {}",
        chevron.x,
        rendered_label_right + 6.0
    );
}

#[test]
fn footer_add_variable_button_aligns_to_panel_padding() {
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, 480.0),
    };
    let button = add_variable_rect(rect);
    let footer_top = rect.origin.y + rect.size.y - FOOTER_HEIGHT;

    assert_eq!(button.origin.x, rect.origin.x + PAD_X);
    assert!(
        ((button.origin.y + button.size.y / 2.0) - (footer_top + FOOTER_HEIGHT / 2.0)).abs()
            < f32::EPSILON,
        "footer add button should be vertically centered in the footer"
    );
    assert_eq!(button.size.y, 30.0);
}

#[test]
fn footer_add_variable_chevron_clears_cjk_label() {
    let mut s = EditorState::new();
    s.editor_ui.locale = Locale::ZhCn;
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, 480.0),
    };
    let button = add_variable_rect(rect);
    let mut backend = TextCaptureBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    p.paint(&mut cx, rect);

    let label_x = button.origin.x + 16.0 + 12.0;
    let rendered_label_right = label_x + 4.0 * 14.0;
    let chevron = backend
        .svg_origins
        .iter()
        .zip(backend.svg_sizes.iter())
        .find(|(origin, size)| {
            (**size - 12.0).abs() < f32::EPSILON
                && origin.x >= button.origin.x
                && origin.x < button.origin.x + button.size.x
                && origin.y >= button.origin.y
                && origin.y < button.origin.y + button.size.y
        })
        .map(|(origin, _)| *origin)
        .expect("footer add-variable chevron should paint inside the button");

    assert!(
        chevron.x >= rendered_label_right + 10.0,
        "footer add-variable chevron should clear the CJK label; got {}, expected at least {}",
        chevron.x,
        rendered_label_right + 10.0
    );
}

#[test]
fn labels_follow_active_i18n_locale() {
    let mut s = EditorState::new();
    s.editor_ui.locale = Locale::Ja;
    let labels = VariablesPanel::for_editor(&s).labels();

    assert_eq!(labels.preset, "プリセット");
    assert_eq!(labels.name, "名前");
    assert_eq!(labels.empty, "変数が定義されていません");
    assert_eq!(labels.add_variable, "変数を追加");
    assert_eq!(labels.save_preset, "現在の設定をプリセットとして保存…");
    assert_eq!(labels.color, "色");
    assert_eq!(labels.number, "数値");
    assert_eq!(labels.string, "文字列");
}

#[test]
fn hit_test_returns_none_outside_rect() {
    let s = state_with_three_vars();
    let p = VariablesPanel::for_editor(&s);
    let rect = Rect {
        origin: Point2D::new(0.0, 0.0),
        size: Point2D::new(VARIABLES_PANEL_WIDTH, 200.0),
    };
    assert!(p.hit_test(rect, Point2D::new(-10.0, 50.0)).is_none());
    assert!(p.hit_test(rect, Point2D::new(50.0, 1000.0)).is_none());
}

#[test]
fn axis_chip_table_mirrors_active_theme_btree_order() {
    let mut s = EditorState::new();
    s.ui.variables
        .active_theme
        .insert("z-axis".into(), "alpha".into());
    s.ui.variables
        .active_theme
        .insert("a-axis".into(), "omega".into());
    let p = VariablesPanel::for_editor(&s);
    assert_eq!(p.chips.len(), 2);
    assert_eq!(p.chips[0].axis, "a-axis");
    assert_eq!(p.chips[1].axis, "z-axis");
}

// Fit-content hover-wash tests (#26 variant header + #3 add-variable footer)
// live in the sibling `variables_panel/wash_tests.rs` to keep this file under
// the 800-line cap.
