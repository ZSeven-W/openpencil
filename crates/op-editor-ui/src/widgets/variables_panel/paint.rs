use super::*;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, Point2D, Rect};

const VARIABLE_NAME_PREFIX_STEP: f32 = 8.0;
const VARIABLE_NAME_PREFIX_TEXT_GAP: f32 = 2.0;
const INPUT_RADIUS: f32 = 8.0;
const INPUT_BORDER_WIDTH: f32 = 1.5;
const INPUT_FONT_SIZE: f32 = 13.0;
const INPUT_CARET_HEIGHT: f32 = 18.0;
const INPUT_PADDING_X: f32 = 8.0;
const VALUE_INPUT_MIN_WIDTH: f32 = 96.0;
const VALUE_INPUT_MAX_WIDTH: f32 = 160.0;
const FOOTER_CHEVRON_LABEL_GAP: f32 = 12.0;

pub(super) fn paint_panel(panel: &VariablesPanel, cx: &mut PaintCx<'_>, rect: Rect) {
    let theme = panel.theme;
    let labels = panel.labels();
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
    paint_search_row(panel, cx, rect, &labels);
    paint_rows(panel, cx, rect, &labels);
    paint_footer(panel, cx, rect, footer_top, &labels);
    menus::paint_menus(panel, cx, rect, &labels);
}

/// Search filter strip below the column header (TS shows it past 6
/// entries): magnifier icon + live text + caret-at-end while focused
/// + muted placeholder when empty.
fn paint_search_row(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    labels: &VariablePanelLabels,
) {
    if !panel.search_visible() {
        return;
    }
    let theme = panel.theme;
    let input = panel.search_input_rect(rect);
    cx.backend.fill_round_rect(input, INPUT_RADIUS, theme.muted);
    if panel.search_focus {
        cx.backend
            .stroke_round_rect(input, INPUT_RADIUS, theme.primary, INPUT_BORDER_WIDTH);
    }
    let icon_size = 13.0;
    draw_icon(
        cx.backend,
        Icon::Search,
        Point2D::new(
            input.origin.x + 8.0,
            input.origin.y + (input.size.y - icon_size) / 2.0,
        ),
        icon_size,
        theme.muted_foreground,
        1.6,
    );
    let text_x = input.origin.x + 8.0 + icon_size + 7.0;
    let baseline_y = input.origin.y + input.size.y / 2.0 + 4.0;
    if panel.search.is_empty() {
        paint_text(
            cx,
            labels.search_placeholder,
            12.0,
            theme.muted_foreground,
            text_x,
            baseline_y,
        );
    } else {
        paint_text(
            cx,
            &panel.search,
            12.0,
            theme.foreground,
            text_x,
            baseline_y,
        );
    }
    if panel.search_focus
        && jian_core::anim::blink_visible(panel.now_ms, panel.caret_anchor_ms, 500)
    {
        let caret_x = text_x
            + if panel.search.is_empty() {
                0.0
            } else {
                cx.backend.measure_text(&panel.search, 12.0)
            };
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(caret_x + 1.0, input.origin.y + 5.0),
                size: Point2D::new(1.5, 18.0),
            },
            theme.foreground,
        );
    }
    paint_hairline(
        cx,
        rect.origin.x,
        input.origin.y + input.size.y + 7.0,
        rect.size.x,
        theme.border,
    );
}

fn paint_theme_header(panel: &VariablesPanel, cx: &mut PaintCx<'_>, rect: Rect) {
    let theme = panel.theme;
    let mut x = rect.origin.x + PAD_X;
    let active_axis = panel.active_axis_label();
    for (idx, axis) in panel.theme_tab_labels().iter().enumerate() {
        let is_active = *axis == active_axis;
        if panel.hover == Some(VariablesPanelButton::ThemeTab(idx)) {
            cx.backend
                .fill_round_rect(panel.theme_tab_rect(rect, idx), 8.0, theme.button_hover);
        }
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
            paint_text_input(
                cx,
                theme,
                input,
                panel.editing_draft.as_str(),
                panel.rename_text_caret(RenameTarget::Theme(axis)),
                INPUT_PADDING_X,
            );
        } else {
            paint_text(cx, axis, 13.0, color, x, header::text_baseline(rect, 13.0));
        }
        if is_active && panel.renaming_theme.as_deref() != Some(axis) {
            let chevron_x = x + cx.backend.measure_text(axis, 13.0) + 5.0;
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
    if panel.hover == Some(VariablesPanelButton::AddTheme) {
        cx.backend
            .fill_round_rect(add_theme, 8.0, theme.button_hover);
    }
    draw_icon(
        cx.backend,
        Icon::Plus,
        header::icon_origin(rect, add_theme.origin.x - rect.origin.x + 6.0, 16.0),
        16.0,
        theme.muted_foreground,
        1.8,
    );

    let preset = panel.preset_rect(rect);
    if panel.hover == Some(VariablesPanelButton::PresetMenu) {
        cx.backend.fill_round_rect(preset, 8.0, theme.button_hover);
    }
    let preset_label = panel.labels().preset;
    let preset_label_size = 13.0;
    let preset_label_x = preset.origin.x + 29.0;
    draw_icon(
        cx.backend,
        Icon::BookOpen,
        header::icon_origin(rect, preset.origin.x - rect.origin.x + 7.0, 15.0),
        15.0,
        theme.muted_foreground,
        1.6,
    );
    paint_text(
        cx,
        preset_label,
        preset_label_size,
        theme.muted_foreground,
        preset_label_x,
        header::text_baseline(rect, preset_label_size),
    );
    let preset_chevron_x =
        preset_label_x + cx.backend.measure_text(preset_label, preset_label_size) + 7.0;
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        header::icon_origin(rect, preset_chevron_x - rect.origin.x, 11.0),
        11.0,
        theme.muted_foreground,
        1.5,
    );

    let close = close_rect(rect);
    if panel.hover == Some(VariablesPanelButton::Close) {
        cx.backend.fill_round_rect(close, 8.0, theme.button_hover);
    }
    draw_icon(
        cx.backend,
        Icon::Close,
        header::icon_origin(rect, close.origin.x - rect.origin.x + 5.0, 16.0),
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
        if panel.hover == Some(VariablesPanelButton::VariantHeader(idx)) {
            cx.backend.fill_round_rect(
                panel.variant_header_rect(rect, idx),
                8.0,
                theme.button_hover,
            );
        }
        if panel.renaming_variant.as_deref() == Some(*variant) {
            let input = Rect {
                origin: Point2D::new(x - 2.0, header_bottom + 5.0),
                size: Point2D::new(
                    (label_width(&panel.editing_draft, 13.0) + 28.0).max(128.0),
                    26.0,
                ),
            };
            paint_text_input(
                cx,
                theme,
                input,
                panel.editing_draft.as_str(),
                panel.rename_text_caret(RenameTarget::Variant(variant)),
                INPUT_PADDING_X,
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
            let variant_width = cx.backend.measure_text(variant, 13.0);
            draw_icon(
                cx.backend,
                Icon::ChevronDown,
                Point2D::new(x + variant_width + 6.0, header_bottom + 12.0),
                11.0,
                theme.muted_foreground,
                1.5,
            );
        }
    }
    if panel.hover == Some(VariablesPanelButton::AddVariant) {
        cx.backend
            .fill_round_rect(add_variant_rect(rect), 8.0, theme.button_hover);
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
    labels: &VariablePanelLabels,
) {
    let theme = panel.theme;
    if panel.rows.is_empty() {
        // Filtered-empty vs truly-empty (TS `noMatch` vs `noDefined`).
        let empty = if panel.search.is_empty() {
            labels.empty
        } else {
            labels.no_match
        };
        paint_text(
            cx,
            empty,
            14.0,
            theme.muted_foreground,
            rect.origin.x + rect.size.x / 2.0 - 52.0,
            rect.origin.y + rect.size.y / 2.0,
        );
        return;
    }

    // Rows scroll inside a clipped viewport between the (optional)
    // search strip and the footer.
    let viewport = panel.rows_viewport(rect);
    cx.backend.save();
    cx.backend.clip_rect(viewport);
    let variants = panel.variant_column_labels();
    let active_axis = panel.active_axis_label();
    let scroll = panel.effective_scroll(rect);
    for (idx, var) in panel.rows.iter().enumerate() {
        let y = viewport.origin.y - scroll + ROW_HEIGHT * idx as f32;
        // Cull rows fully outside the viewport.
        if y + ROW_HEIGHT <= viewport.origin.y {
            continue;
        }
        if y >= viewport.origin.y + viewport.size.y {
            break;
        }
        let source = var.source_idx;
        let row_hovered = matches!(
            panel.hover,
            Some(VariablesPanelButton::Row(i)) if i == source
        ) || matches!(
            panel.hover,
            Some(VariablesPanelButton::RowMenuButton(i)) if i == source
        );
        if row_hovered {
            cx.backend.fill_round_rect(
                Rect {
                    origin: Point2D::new(rect.origin.x + 8.0, y + 3.0),
                    size: Point2D::new(rect.size.x - 16.0, ROW_HEIGHT - 6.0),
                },
                8.0,
                theme.button_hover,
            );
        }
        paint_variable_name_cell(panel, cx, rect, var, idx, y);
        for (variant_idx, variant) in variants.iter().enumerate() {
            let scalar = panel.variant_scalar_for(var, active_axis, variant);
            let cell_rect = panel.value_cell_rect_at(rect, idx, variant_idx, variants.len().max(1));
            paint_value_cell(panel, cx, var, (source, variant_idx), scalar, cell_rect);
        }
        // `⋯` overflow button — TS shows it on row hover only
        // (`opacity-0 group-hover:opacity-100`); it also stays while
        // its menu is open.
        let menu_open_here = panel.row_menu_open == Some(source);
        if row_hovered || menu_open_here {
            let button = panel.row_menu_button_rect(rect, idx);
            if menu_open_here
                || matches!(
                    panel.hover,
                    Some(VariablesPanelButton::RowMenuButton(i)) if i == source
                )
            {
                cx.backend.fill_round_rect(button, 8.0, theme.muted);
            }
            draw_icon(
                cx.backend,
                Icon::MoreHorizontal,
                Point2D::new(button.origin.x + 5.5, button.origin.y + 5.5),
                15.0,
                theme.muted_foreground,
                1.6,
            );
        }
    }
    cx.backend.restore();
}

fn paint_variable_name_cell(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    var: &VarRow,
    display_idx: usize,
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
    let pill = panel.name_cell_rect_at(rect, display_idx);
    cx.backend.fill_round_rect(pill, 8.0, theme.muted);
    // Focus indices are UNFILTERED positions.
    let is_editing = panel.editing_name_row == Some(var.source_idx);
    let name = if is_editing {
        panel.editing_draft.as_str()
    } else {
        var.name.as_str()
    };
    let text_x = pill.origin.x + 10.0;
    let text_y = pill.origin.y + 20.0;
    if is_editing {
        paint_text_input(
            cx,
            theme,
            pill,
            panel.editing_draft.as_str(),
            panel.name_caret_for_row(var.source_idx),
            text_x - pill.origin.x,
        );
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
    cell_rect: Rect,
) {
    let theme = panel.theme;
    let origin = Point2D::new(cell_rect.origin.x, cell_rect.origin.y + 10.0);
    match var.kind {
        VariableKind::Color => {
            let rgba = scalar.and_then(scalar_as_color).unwrap_or(Color::WHITE);
            let swatch = Rect {
                origin,
                size: Point2D::new(SWATCH_SIZE, SWATCH_SIZE),
            };
            cx.backend.fill_round_rect(swatch, 3.0, rgba);
            cx.backend.stroke_round_rect(swatch, 3.0, theme.border, 1.0);
            // Inline hex editing (TS ColorCell's text `<input>`) — the
            // swatch stays painted, the hex label swaps for an input.
            if panel.editing_value_cell == Some(cell) {
                let input_rect = Rect {
                    origin: Point2D::new(
                        cell_rect.origin.x + SWATCH_SIZE + 6.0,
                        cell_rect.origin.y + 7.0,
                    ),
                    size: Point2D::new(92.0, 30.0),
                };
                paint_text_input(
                    cx,
                    theme,
                    input_rect,
                    panel.editing_draft.as_str(),
                    panel.value_caret_for_cell(cell.0, cell.1),
                    INPUT_PADDING_X,
                );
                return;
            }
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
            // Opacity derives from a `#rrggbbaa` alpha channel — 100
            // when absent. TS tints a sub-100 value brighter
            // (`text-foreground/80` vs muted).
            let opacity = scalar.and_then(scalar_alpha_percent).unwrap_or(100);
            let opacity_color = if opacity < 100 {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            paint_text(
                cx,
                &format!("{opacity} %"),
                13.0,
                opacity_color,
                origin.x + 118.0,
                origin.y + 15.0,
            );
        }
        _ => {
            let is_editing = panel.editing_value_cell == Some(cell);
            let text = scalar
                .map(scalar_to_label)
                .or_else(|| var.resolved.as_ref().map(scalar_to_label))
                .unwrap_or_else(|| "—".into());
            if is_editing {
                let draft_w = cx
                    .backend
                    .measure_text(panel.editing_draft.as_str(), INPUT_FONT_SIZE)
                    + INPUT_PADDING_X * 2.0
                    + 8.0;
                let max_cell_w = (cell_rect.size.x - 12.0).max(VALUE_INPUT_MIN_WIDTH);
                let input_w = draft_w
                    .clamp(VALUE_INPUT_MIN_WIDTH, VALUE_INPUT_MAX_WIDTH)
                    .min(max_cell_w);
                let input_rect = Rect {
                    origin: Point2D::new(
                        cell_rect.origin.x - INPUT_PADDING_X,
                        cell_rect.origin.y + 7.0,
                    ),
                    size: Point2D::new(input_w, 30.0),
                };
                paint_text_input(
                    cx,
                    theme,
                    input_rect,
                    panel.editing_draft.as_str(),
                    panel.value_caret_for_cell(cell.0, cell.1),
                    INPUT_PADDING_X,
                );
                return;
            }
            paint_text(
                cx,
                &truncate(&text, 18),
                13.0,
                theme.foreground,
                origin.x,
                origin.y + 15.0,
            );
        }
    }
}

fn paint_footer(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    _footer_top: f32,
    labels: &VariablePanelLabels,
) {
    let theme = panel.theme;
    let button = add_variable_rect(rect);
    if panel.hover == Some(VariablesPanelButton::AddVariable) {
        cx.backend.fill_round_rect(button, 8.0, theme.button_hover);
    }
    let center_y = button.origin.y + button.size.y / 2.0;
    let icon_size = 16.0;
    let label_size = 14.0;
    let label_x = button.origin.x + icon_size + 12.0;
    let label_baseline_y = center_y + 5.0;
    let chevron_size = 12.0;
    let chevron_x = label_x
        + cx.backend.measure_text(labels.add_variable, label_size)
        + FOOTER_CHEVRON_LABEL_GAP;
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(button.origin.x, center_y - icon_size / 2.0),
        icon_size,
        theme.muted_foreground,
        1.8,
    );
    paint_text(
        cx,
        labels.add_variable,
        label_size,
        theme.muted_foreground,
        label_x,
        label_baseline_y,
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(chevron_x, center_y - chevron_size / 2.0),
        chevron_size,
        theme.muted_foreground,
        1.5,
    );
}

pub(super) fn paint_hairline(cx: &mut PaintCx<'_>, x: f32, y: f32, w: f32, color: Color) {
    cx.backend.fill_rect(
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(w, 1.0),
        },
        color,
    );
}

fn paint_text_input(
    cx: &mut PaintCx<'_>,
    theme: Theme,
    rect: Rect,
    value: &str,
    caret_pos: Option<usize>,
    padding_x: f32,
) {
    cx.backend.fill_round_rect(rect, INPUT_RADIUS, theme.muted);
    cx.backend
        .stroke_round_rect(rect, INPUT_RADIUS, theme.primary, INPUT_BORDER_WIDTH);
    let value_x = rect.origin.x + padding_x;
    let baseline_y = rect.origin.y + rect.size.y / 2.0 + 4.0;
    paint_text(
        cx,
        value,
        INPUT_FONT_SIZE,
        theme.foreground,
        value_x,
        baseline_y,
    );
    if let Some(pos) = caret_pos {
        let caret_y = rect.origin.y + ((rect.size.y - INPUT_CARET_HEIGHT) / 2.0).max(0.0);
        paint_caret_in_text(cx, theme, value, pos, value_x, caret_y);
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

pub(super) fn paint_text(
    cx: &mut PaintCx<'_>,
    text: &str,
    size: f32,
    color: Color,
    x: f32,
    baseline_y: f32,
) {
    let layout = crate::TextLayout::single_run(
        text,
        "system-ui",
        size,
        (color).to_jian(),
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

/// Alpha channel of a `#rrggbbaa` hex as 0-100 (TS
/// `variable-row.tsx getOpacityForTheme`); `None` for non-9-char
/// strings → caller falls back to 100.
fn scalar_alpha_percent(s: &VariableScalar) -> Option<u32> {
    let VariableScalar::Str(hex) = s else {
        return None;
    };
    if !hex.starts_with('#') || hex.len() != 9 {
        return None;
    }
    let alpha = u8::from_str_radix(&hex[7..9], 16).ok()?;
    Some(((alpha as f32 / 255.0) * 100.0).round() as u32)
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
