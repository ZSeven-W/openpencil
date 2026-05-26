use super::*;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, Point2D, Rect};

const VARIABLE_NAME_PREFIX_STEP: f32 = 8.0;
const VARIABLE_NAME_PREFIX_TEXT_GAP: f32 = 2.0;

pub(super) fn paint_panel(panel: &VariablesPanel, cx: &mut PaintCx<'_>, rect: Rect) {
    let theme = panel.theme;
    let labels = panel.labels();
    paint_panel_shadow(cx, rect);
    cx.backend.fill_round_rect(rect, PANEL_RADIUS, theme.card);
    cx.backend
        .stroke_round_rect(rect, PANEL_RADIUS, theme.border, 1.0);

    let header_bottom = rect.origin.y + HEADER_HEIGHT;
    let column_bottom = header_bottom + COLUMN_HEADER_HEIGHT;
    let footer_top = rect.origin.y + rect.size.y - FOOTER_HEIGHT;
    paint_hairline(cx, rect.origin.x, header_bottom, rect.size.x, theme.border);
    paint_hairline(cx, rect.origin.x, column_bottom, rect.size.x, theme.border);
    paint_hairline(cx, rect.origin.x, footer_top, rect.size.x, theme.border);

    paint_theme_header(panel, cx, rect);
    paint_variant_header(panel, cx, rect, &labels);
    paint_rows(panel, cx, rect, footer_top, &labels);
    paint_footer(panel, cx, rect, footer_top, &labels);
    paint_menus(panel, cx, rect, &labels);
}

fn paint_theme_header(panel: &VariablesPanel, cx: &mut PaintCx<'_>, rect: Rect) {
    let theme = panel.theme;
    let mut x = rect.origin.x + PAD_X;
    let active_axis = panel.active_axis_label();
    for axis in panel.theme_tab_labels() {
        let is_active = axis == active_axis;
        let color = if is_active {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        if panel.renaming_theme.as_deref() == Some(axis) {
            let input = Rect {
                origin: Point2D::new(x - 2.0, rect.origin.y + 8.0),
                size: Point2D::new(panel.theme_rename_input_width(), 28.0),
            };
            paint_inline_input(
                panel,
                cx,
                input,
                RenameTarget::Theme(axis),
                panel.editing_draft.as_str(),
            );
        } else {
            paint_text(cx, axis, 13.0, color, x, header::text_baseline(rect, 13.0));
        }
        if is_active && panel.renaming_theme.as_deref() != Some(axis) {
            let chevron_x = x + label_width(axis, 13.0) + 5.0;
            draw_icon(
                cx.backend,
                Icon::ChevronDown,
                header::icon_origin(rect, chevron_x - rect.origin.x, 11.0),
                11.0,
                theme.muted_foreground,
                1.5,
            );
        }
        x += panel.theme_tab_advance_width(axis);
    }

    let add_theme = panel.add_theme_rect(rect);
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(add_theme.origin.x + 6.0, add_theme.origin.y + 6.0),
        16.0,
        theme.muted_foreground,
        1.8,
    );

    let preset = panel.preset_rect(rect);
    draw_icon(
        cx.backend,
        Icon::BookOpen,
        Point2D::new(preset.origin.x + 7.0, preset.origin.y + 8.0),
        15.0,
        theme.muted_foreground,
        1.6,
    );
    paint_text(
        cx,
        panel.labels().preset,
        13.0,
        theme.muted_foreground,
        preset.origin.x + 29.0,
        header::text_baseline(rect, 13.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(preset.origin.x + 92.0, preset.origin.y + 10.0),
        11.0,
        theme.muted_foreground,
        1.5,
    );

    let close = close_rect(rect);
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(close.origin.x + 5.0, close.origin.y + 5.0),
        16.0,
        theme.muted_foreground,
        1.8,
    );
}

fn paint_variant_header(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    labels: &VariablePanelLabels,
) {
    let theme = panel.theme;
    let header_bottom = rect.origin.y + HEADER_HEIGHT;
    paint_text(
        cx,
        labels.name,
        13.0,
        theme.muted_foreground,
        rect.origin.x + PAD_X,
        header_bottom + 23.0,
    );

    let value_x = value_column_x(rect);
    let variants = panel.variant_column_labels();
    let col_w = variant_column_width(rect, variants.len());
    for (idx, variant) in variants.iter().enumerate() {
        let x = value_x + col_w * idx as f32;
        if panel.renaming_variant.as_deref() == Some(*variant) {
            let input = Rect {
                origin: Point2D::new(x - 2.0, header_bottom + 5.0),
                size: Point2D::new(
                    (label_width(&panel.editing_draft, 13.0) + 28.0).max(128.0),
                    26.0,
                ),
            };
            paint_inline_input(
                panel,
                cx,
                input,
                RenameTarget::Variant(variant),
                panel.editing_draft.as_str(),
            );
        } else {
            paint_text(
                cx,
                variant,
                13.0,
                theme.muted_foreground,
                x,
                header_bottom + 23.0,
            );
            draw_icon(
                cx.backend,
                Icon::ChevronDown,
                Point2D::new(x + label_width(variant, 13.0) + 6.0, header_bottom + 12.0),
                11.0,
                theme.muted_foreground,
                1.5,
            );
        }
    }
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(
            rect.origin.x + rect.size.x - PAD_X - 16.0,
            header_bottom + 10.0,
        ),
        16.0,
        theme.muted_foreground,
        1.7,
    );
}

fn paint_rows(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    footer_top: f32,
    labels: &VariablePanelLabels,
) {
    let theme = panel.theme;
    if panel.rows.is_empty() {
        paint_text(
            cx,
            labels.empty,
            14.0,
            theme.muted_foreground,
            rect.origin.x + rect.size.x / 2.0 - 52.0,
            rect.origin.y + rect.size.y / 2.0,
        );
        return;
    }

    let variants = panel.variant_column_labels();
    let active_axis = panel.active_axis_label();
    let col_w = variant_column_width(rect, variants.len());
    let value_x = value_column_x(rect);
    let mut y = rows_start_y(rect);
    for (idx, var) in panel.rows.iter().enumerate() {
        if y + ROW_HEIGHT > footer_top {
            break;
        }
        paint_variable_name_cell(panel, cx, rect, var, idx, y);
        for (variant_idx, variant) in variants.iter().enumerate() {
            let cell_x = value_x + col_w * variant_idx as f32;
            let scalar = panel.variant_scalar_for(var, active_axis, variant);
            paint_value_cell(
                panel,
                cx,
                var,
                (idx, variant_idx),
                scalar,
                Point2D::new(cell_x, y + 10.0),
            );
        }
        y += ROW_HEIGHT;
    }
}

fn paint_variable_name_cell(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    var: &VarRow,
    idx: usize,
    y: f32,
) {
    let theme = panel.theme;
    let icon = match var.kind {
        VariableKind::Color => Icon::Circle,
        VariableKind::Number => Icon::Hash,
        VariableKind::Boolean | VariableKind::String => Icon::Type,
    };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + PAD_X, y + 14.0),
        15.0,
        theme.muted_foreground,
        1.6,
    );
    let pill = name_cell_rect(rect, idx);
    cx.backend.fill_round_rect(pill, 8.0, theme.muted);
    let is_editing = panel.editing_name_row == Some(idx);
    let name = if is_editing {
        panel.editing_draft.as_str()
    } else {
        var.name.as_str()
    };
    let text_x = pill.origin.x + 10.0;
    let text_y = pill.origin.y + 20.0;
    if is_editing {
        let display = truncate(name, 24);
        paint_text(cx, &display, 13.0, theme.foreground, text_x, text_y);
        let Some(pos) = panel.name_caret_for_row(idx) else {
            return;
        };
        paint_caret_in_text(cx, theme, &display, pos, text_x, pill.origin.y + 6.0);
    } else {
        let display = truncate(name, 24);
        paint_text(cx, "-", 13.0, theme.foreground, text_x, text_y);
        paint_text(
            cx,
            "-",
            13.0,
            theme.foreground,
            text_x + VARIABLE_NAME_PREFIX_STEP,
            text_y,
        );
        paint_text(
            cx,
            &display,
            13.0,
            theme.foreground,
            text_x + VARIABLE_NAME_PREFIX_STEP * 2.0 + VARIABLE_NAME_PREFIX_TEXT_GAP,
            text_y,
        );
    }
}

fn paint_value_cell(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    var: &VarRow,
    cell: (usize, usize),
    scalar: Option<&VariableScalar>,
    origin: Point2D,
) {
    let theme = panel.theme;
    match var.kind {
        VariableKind::Color => {
            let rgba = scalar.and_then(scalar_as_color).unwrap_or(Color::WHITE);
            let swatch = Rect {
                origin,
                size: Point2D::new(SWATCH_SIZE, SWATCH_SIZE),
            };
            cx.backend.fill_round_rect(swatch, 3.0, rgba);
            cx.backend.stroke_round_rect(swatch, 3.0, theme.border, 1.0);
            let label = scalar
                .and_then(scalar_hex_label)
                .unwrap_or_else(|| "#000000".to_string());
            paint_text(
                cx,
                &label,
                13.0,
                theme.foreground,
                origin.x + 32.0,
                origin.y + 15.0,
            );
            paint_text(
                cx,
                "100 %",
                13.0,
                theme.muted_foreground,
                origin.x + 118.0,
                origin.y + 15.0,
            );
        }
        _ => {
            let text = if panel.editing_value_cell == Some(cell) {
                panel.editing_draft.clone()
            } else {
                scalar
                    .map(scalar_to_label)
                    .or_else(|| var.resolved.as_ref().map(scalar_to_label))
                    .unwrap_or_else(|| "—".into())
            };
            paint_text(
                cx,
                &truncate(&text, 18),
                13.0,
                theme.foreground,
                origin.x,
                origin.y + 15.0,
            );
            if let Some(pos) = panel.value_caret_for_cell(cell.0, cell.1) {
                paint_caret_in_text(cx, theme, &text, pos, origin.x, origin.y);
            }
        }
    }
}

fn paint_footer(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    footer_top: f32,
    labels: &VariablePanelLabels,
) {
    let theme = panel.theme;
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(rect.origin.x + PAD_X, footer_top + 12.0),
        16.0,
        theme.muted_foreground,
        1.8,
    );
    paint_text(
        cx,
        labels.add_variable,
        14.0,
        theme.muted_foreground,
        rect.origin.x + PAD_X + 28.0,
        footer_top + 27.0,
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(rect.origin.x + PAD_X + 84.0, footer_top + 14.0),
        12.0,
        theme.muted_foreground,
        1.5,
    );
}

fn paint_menus(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    labels: &VariablePanelLabels,
) {
    let theme = panel.theme;
    if panel.preset_menu_open {
        paint_popover_rows(
            cx,
            theme,
            panel.preset_menu_rect(rect),
            &[
                labels.save_preset,
                labels.no_presets,
                labels.import,
                labels.export,
            ],
        );
    }
    if panel.add_menu_open {
        paint_popover_rows(
            cx,
            theme,
            add_variable_menu_rect(rect),
            &[labels.color, labels.number, labels.string],
        );
    }
    if let Some(axis) = panel.theme_menu_open.as_deref() {
        let mut rows = vec![labels.rename];
        if panel.theme_tab_labels().len() > 1 {
            rows.push(labels.delete);
        }
        paint_popover_rows(cx, theme, panel.theme_menu_rect(rect, axis), &rows);
    }
    if let Some(value) = panel.variant_menu_open.as_deref() {
        let mut rows = vec![labels.rename];
        if panel.variant_column_labels().len() > 1 {
            rows.push(labels.delete);
        }
        paint_popover_rows(cx, theme, panel.variant_menu_rect(rect, value), &rows);
    }
    paint_axis_dropdown(panel, cx, rect);
}

fn paint_axis_dropdown(panel: &VariablesPanel, cx: &mut PaintCx<'_>, rect: Rect) {
    let theme = panel.theme;
    let Some(open_axis) = panel.dropdown_open.as_deref() else {
        return;
    };
    let Some((chip_idx, _)) = panel
        .chips
        .iter()
        .enumerate()
        .find(|(_, c)| c.axis == open_axis)
    else {
        return;
    };
    let Some(values) = panel.axis_values(open_axis) else {
        return;
    };
    let chip_rect = panel.chip_rect(rect, chip_idx);
    let menu_y = chip_rect.origin.y + chip_rect.size.y + 4.0;
    let menu_rect = Rect {
        origin: Point2D::new(chip_rect.origin.x, menu_y),
        size: Point2D::new(DROPDOWN_WIDTH, DROPDOWN_ROW_HEIGHT * (values.len() as f32)),
    };
    cx.backend.fill_round_rect(menu_rect, 12.0, theme.popover);
    cx.backend
        .stroke_round_rect(menu_rect, 12.0, theme.border, 1.0);
    let active_value = panel
        .chips
        .iter()
        .find(|c| c.axis == open_axis)
        .map(|c| c.value.clone())
        .unwrap_or_default();
    for (i, v) in values.iter().enumerate() {
        let row_y = menu_y + (i as f32) * DROPDOWN_ROW_HEIGHT;
        if *v == active_value {
            let highlight = Rect {
                origin: Point2D::new(menu_rect.origin.x + 4.0, row_y + 3.0),
                size: Point2D::new(menu_rect.size.x - 8.0, DROPDOWN_ROW_HEIGHT - 6.0),
            };
            cx.backend.fill_round_rect(highlight, 8.0, theme.muted);
        }
        paint_text(
            cx,
            v,
            13.0,
            theme.foreground,
            menu_rect.origin.x + 12.0,
            row_y + 23.0,
        );
    }
}

fn paint_hairline(cx: &mut PaintCx<'_>, x: f32, y: f32, w: f32, color: Color) {
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(w, 1.0),
        },
        color,
    );
}

fn paint_panel_shadow(cx: &mut PaintCx<'_>, rect: Rect) {
    for (dy, alpha) in [(18.0, 0.08), (34.0, 0.04)] {
        cx.backend.fill_round_rect(
            Rect {
                origin: Point2D::new(rect.origin.x, rect.origin.y + dy),
                size: rect.size,
            },
            PANEL_RADIUS,
            Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        );
    }
}

fn paint_popover_rows(cx: &mut PaintCx<'_>, theme: Theme, rect: Rect, rows: &[&str]) {
    cx.backend.fill_round_rect(rect, 12.0, theme.popover);
    cx.backend.stroke_round_rect(rect, 12.0, theme.border, 1.0);
    for (idx, label) in rows.iter().enumerate() {
        let row_y = rect.origin.y + idx as f32 * ADD_VARIABLE_MENU_ROW_HEIGHT;
        paint_text(
            cx,
            label,
            12.0,
            theme.popover_foreground,
            rect.origin.x + 12.0,
            row_y + 20.0,
        );
    }
}

fn paint_inline_input(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    target: RenameTarget<'_>,
    value: &str,
) {
    let theme = panel.theme;
    cx.backend.fill_round_rect(rect, 8.0, theme.muted);
    cx.backend.stroke_round_rect(rect, 8.0, theme.primary, 1.5);
    let value_x = rect.origin.x + 8.0;
    let baseline_y = rect.origin.y + rect.size.y / 2.0 + 4.0;
    paint_text(cx, value, 13.0, theme.foreground, value_x, baseline_y);
    if let Some(pos) = panel.rename_text_caret(target) {
        paint_caret_in_text(cx, theme, value, pos, value_x, rect.origin.y + 6.0);
    }
}

fn paint_caret_in_text(
    cx: &mut PaintCx<'_>,
    theme: Theme,
    value: &str,
    pos: usize,
    x: f32,
    y: f32,
) {
    let clipped = text_boundary_at_or_before(value, pos);
    let value_w = cx.backend.measure_text(&value[..clipped], 13.0);
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x + value_w, y),
            size: Point2D::new(1.5, 18.0),
        },
        theme.foreground,
    );
}

fn text_boundary_at_or_before(value: &str, pos: usize) -> usize {
    let mut clipped = pos.min(value.len());
    while clipped > 0 && !value.is_char_boundary(clipped) {
        clipped -= 1;
    }
    clipped
}

fn paint_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, baseline_y: f32) {
    let layout = crate::TextLayout::single_run(
        text,
        "system-ui",
        size,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
}

fn scalar_as_color(s: &VariableScalar) -> Option<Color> {
    let hex = match s {
        VariableScalar::Str(hex) => hex,
        _ => return None,
    };
    let (r, g, b) = op_editor_core::color_picker::parse_hex_rgb(hex)?;
    Some(Color { r, g, b, a: 1.0 })
}

fn scalar_hex_label(s: &VariableScalar) -> Option<String> {
    let VariableScalar::Str(hex) = s else {
        return None;
    };
    if hex.starts_with('#') && hex.len() >= 7 {
        Some(hex[..7].to_string())
    } else {
        Some(hex.clone())
    }
}

fn scalar_to_label(s: &VariableScalar) -> String {
    match s {
        VariableScalar::Str(s) => s.clone(),
        VariableScalar::Num(n) => format!("{n}"),
        VariableScalar::Bool(b) => if *b { "true" } else { "false" }.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
