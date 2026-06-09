//! Text-specific property section for the native right panel.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{
    FontFamilyChoice, FontWeightChoice, NodeSnapshot, PropertyPanelAction, TextAlignValue,
    TextGrowthValue, TextVerticalAlignValue,
};
use crate::widgets::property_panel_inputs::{
    paint_input_with_icon_focused, paint_input_with_prefix_focused, paint_section_divider,
    paint_section_label, to_jian_color, INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP,
    SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::EditContext;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

const FAMILY_ROW_GAP: f32 = 6.0;
const ALIGN_LABEL_H: f32 = 18.0;
const BUTTON_H: f32 = 28.0;
const TEXT_LAYOUT_BLOCK_H: f32 = SECTION_HEADER_HEIGHT + BUTTON_H + 12.0;
/// Height of the small 行高 / 字间距 caption row painted above the
/// line-height / letter-spacing inputs (TS `text-[9px]` label row).
const LH_LS_LABEL_H: f32 = 14.0;

pub fn text_section_height() -> f32 {
    TEXT_LAYOUT_BLOCK_H
        + SECTION_HEADER_HEIGHT
        + INPUT_HEIGHT
        + FAMILY_ROW_GAP
        + INPUT_HEIGHT
        + 6.0
        + LH_LS_LABEL_H
        + INPUT_HEIGHT
        + 8.0
        + ALIGN_LABEL_H
        + BUTTON_H
        + 6.0
        + ALIGN_LABEL_H
        + BUTTON_H
        + 12.0
}

pub fn push_text_input_rects(
    rects: &mut Vec<(PropertyFocus, Rect)>,
    x0: f32,
    y: f32,
    usable_w: f32,
) {
    let half_w = (usable_w - 8.0) / 2.0;
    let mut y = y + TEXT_LAYOUT_BLOCK_H + SECTION_HEADER_HEIGHT + INPUT_HEIGHT + FAMILY_ROW_GAP;
    // Weight (left half) is now a dropdown (a ToggleFontWeightPicker
    // action rect), not a focusable input — only Font Size remains here.
    rects.push((
        PropertyFocus::FontSize,
        Rect {
            origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
    ));
    // +LH_LS_LABEL_H to skip the 行高/字间距 caption row that paints
    // above the inputs (keeps hit-test aligned with paint).
    y += INPUT_HEIGHT + 6.0 + LH_LS_LABEL_H;
    rects.push((
        PropertyFocus::LineHeight,
        Rect {
            origin: Point2D::new(x0 + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
    ));
    rects.push((
        PropertyFocus::LetterSpacing,
        Rect {
            origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
    ));
}

pub fn text_action_rects(x0: f32, y: f32, usable_w: f32) -> Vec<(PropertyPanelAction, Rect)> {
    let mut out = Vec::new();
    let growth_w = (usable_w - 2.0 * 6.0) / 3.0;
    let growth_y = y + SECTION_HEADER_HEIGHT;
    let growth_actions = [
        (
            PropertyPanelAction::SetTextGrowth(TextGrowthValue::Auto),
            0_usize,
        ),
        (
            PropertyPanelAction::SetTextGrowth(TextGrowthValue::FixedWidth),
            1_usize,
        ),
        (
            PropertyPanelAction::SetTextGrowth(TextGrowthValue::FixedWidthHeight),
            2_usize,
        ),
    ];
    for (action, i) in growth_actions {
        out.push((
            action,
            Rect {
                origin: Point2D::new(x0 + PAD_X + i as f32 * (growth_w + 6.0), growth_y),
                size: Point2D::new(growth_w, BUTTON_H),
            },
        ));
    }
    out.push((
        PropertyPanelAction::ToggleFontFamilyPicker,
        Rect {
            origin: Point2D::new(x0 + PAD_X, y + TEXT_LAYOUT_BLOCK_H + SECTION_HEADER_HEIGHT),
            size: Point2D::new(usable_w, INPUT_HEIGHT),
        },
    ));
    // Weight dropdown trigger — left half of the weight/size row.
    let weight_row_y =
        y + TEXT_LAYOUT_BLOCK_H + SECTION_HEADER_HEIGHT + INPUT_HEIGHT + FAMILY_ROW_GAP;
    let weight_half_w = (usable_w - 8.0) / 2.0;
    out.push((
        PropertyPanelAction::ToggleFontWeightPicker,
        Rect {
            origin: Point2D::new(x0 + PAD_X, weight_row_y),
            size: Point2D::new(weight_half_w, INPUT_HEIGHT),
        },
    ));
    let mut y = y
        + TEXT_LAYOUT_BLOCK_H
        + SECTION_HEADER_HEIGHT
        + INPUT_HEIGHT
        + FAMILY_ROW_GAP
        + INPUT_HEIGHT
        + 6.0
        + INPUT_HEIGHT
        + 8.0
        + ALIGN_LABEL_H;
    let h_buttons = [
        (PropertyPanelAction::SetTextAlign(TextAlignValue::Left), 0),
        (PropertyPanelAction::SetTextAlign(TextAlignValue::Center), 1),
        (PropertyPanelAction::SetTextAlign(TextAlignValue::Right), 2),
        (
            PropertyPanelAction::SetTextAlign(TextAlignValue::Justify),
            3,
        ),
    ];
    let h_w = (usable_w - 3.0 * 6.0) / 4.0;
    for (action, i) in h_buttons {
        out.push((
            action,
            Rect {
                origin: Point2D::new(x0 + PAD_X + i as f32 * (h_w + 6.0), y),
                size: Point2D::new(h_w, BUTTON_H),
            },
        ));
    }
    y += BUTTON_H + 6.0 + ALIGN_LABEL_H;
    let v_buttons = [
        (
            PropertyPanelAction::SetTextVerticalAlign(TextVerticalAlignValue::Top),
            0,
        ),
        (
            PropertyPanelAction::SetTextVerticalAlign(TextVerticalAlignValue::Middle),
            1,
        ),
        (
            PropertyPanelAction::SetTextVerticalAlign(TextVerticalAlignValue::Bottom),
            2,
        ),
    ];
    let v_w = (usable_w - 2.0 * 6.0) / 3.0;
    for (action, i) in v_buttons {
        out.push((
            action,
            Rect {
                origin: Point2D::new(x0 + PAD_X + i as f32 * (v_w + 6.0), y),
                size: Point2D::new(v_w, BUTTON_H),
            },
        ));
    }
    out
}

pub fn font_family_picker_action_rects(
    x0: f32,
    y: f32,
    usable_w: f32,
) -> Vec<(PropertyPanelAction, Rect)> {
    let family_y = y + TEXT_LAYOUT_BLOCK_H + SECTION_HEADER_HEIGHT + INPUT_HEIGHT + 4.0;
    FontFamilyChoice::ALL
        .into_iter()
        .enumerate()
        .map(|(i, choice)| {
            (
                PropertyPanelAction::SetFontFamily(choice),
                Rect {
                    origin: Point2D::new(x0 + PAD_X, family_y + i as f32 * 28.0),
                    size: Point2D::new(usable_w, 28.0),
                },
            )
        })
        .collect()
}

/// Dropdown rows for the weight picker — opens below the left-half
/// weight trigger of the weight/size row.
pub fn font_weight_picker_action_rects(
    x0: f32,
    y: f32,
    usable_w: f32,
) -> Vec<(PropertyPanelAction, Rect)> {
    // Full-width rows so the "number + name" labels (e.g. "800 Extra
    // Bold") fit; the trigger sits on the left half but a dropdown may
    // be wider than its trigger.
    let weight_y = y
        + TEXT_LAYOUT_BLOCK_H
        + SECTION_HEADER_HEIGHT
        + INPUT_HEIGHT
        + FAMILY_ROW_GAP
        + INPUT_HEIGHT
        + 4.0;
    FontWeightChoice::ALL
        .into_iter()
        .enumerate()
        .map(|(i, choice)| {
            (
                PropertyPanelAction::SetFontWeight(choice),
                Rect {
                    origin: Point2D::new(x0 + PAD_X, weight_y + i as f32 * 28.0),
                    size: Point2D::new(usable_w, 28.0),
                },
            )
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn paint_text_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let Some(text) = snapshot.text.as_ref() else {
        return y;
    };
    let mut y = paint_section_label(
        cx,
        theme,
        op_i18n::translate(locale, "textLayout.title"),
        x,
        y,
        width,
    );
    y = paint_text_growth_row(cx, theme, locale, x, y, width, text.growth);
    y += 12.0;
    let mut y = paint_section_label(
        cx,
        theme,
        op_i18n::translate(locale, "text.typography"),
        x,
        y,
        width,
    );
    let usable_w = width - PAD_X * 2.0;
    let family_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(family_rect, INPUT_RADIUS, theme.muted);
    let family = TextLayout::single_run(
        &text.font_family,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &family,
        Point2D::new(family_rect.origin.x + 10.0, family_rect.origin.y + 19.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(
            family_rect.origin.x + family_rect.size.x - 18.0,
            family_rect.origin.y + 8.0,
        ),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    y += INPUT_HEIGHT + FAMILY_ROW_GAP;

    let half_w = (usable_w - 8.0) / 2.0;
    // Weight dropdown trigger — named weight (粗体 / 常规 / …) + chevron,
    // mirroring the font-family trigger (TS Select parity).
    let weight_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    cx.backend
        .fill_round_rect(weight_rect, INPUT_RADIUS, theme.muted);
    let weight_label = op_i18n::translate(
        locale,
        FontWeightChoice::nearest(text.font_weight).label_key(),
    );
    let weight_text = TextLayout::single_run(
        weight_label,
        "system-ui",
        12.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &weight_text,
        Point2D::new(weight_rect.origin.x + 10.0, weight_rect.origin.y + 19.0),
    );
    draw_icon(
        cx.backend,
        Icon::ChevronDown,
        Point2D::new(
            weight_rect.origin.x + weight_rect.size.x - 18.0,
            weight_rect.origin.y + 8.0,
        ),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    let font_size = format_panel_number(text.font_size);
    paint_input_with_prefix_focused(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        "S",
        edit.value_for(PropertyFocus::FontSize, &font_size),
        edit.focus == Some(PropertyFocus::FontSize),
        edit.caret_at(PropertyFocus::FontSize),
        edit.select_all_at(PropertyFocus::FontSize),
    );
    y += INPUT_HEIGHT + 6.0;

    // Caption row — 行高 (left) / 字间距 (right), small muted labels
    // above the inputs (TS `text-[9px] justify-between`).
    let caption_color = to_jian_color(theme.muted_foreground);
    let lh_caption = TextLayout::single_run(
        op_i18n::translate(locale, "text.lineHeight"),
        "system-ui",
        9.0,
        caption_color,
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&lh_caption, Point2D::new(x + PAD_X + 2.0, y + 10.0));
    let ls_label = op_i18n::translate(locale, "text.letterSpacing");
    let ls_caption = TextLayout::single_run(
        ls_label,
        "system-ui",
        9.0,
        caption_color,
        Point2D::new(0.0, 0.0),
    );
    let ls_caption_w = cx.backend.measure_text(ls_label, 9.0);
    cx.backend.draw_text(
        &ls_caption,
        Point2D::new(x + width - PAD_X - 2.0 - ls_caption_w, y + 10.0),
    );
    y += LH_LS_LABEL_H;

    // Line-height — icon prefix + value + `%` suffix (TS NumberInput
    // with `icon={LineHeightIcon}` + `suffix="%"`).
    let line_height = format_panel_number(text.line_height_percent);
    paint_input_with_icon_focused(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        Icon::LineHeight,
        edit.value_for(PropertyFocus::LineHeight, &line_height),
        Some("%"),
        edit.focus == Some(PropertyFocus::LineHeight),
        edit.caret_at(PropertyFocus::LineHeight),
        edit.select_all_at(PropertyFocus::LineHeight),
    );
    // Letter-spacing — `|A|` text prefix (TS NumberInput `label="|A|"`).
    let letter_spacing = format_panel_number(text.letter_spacing);
    paint_input_with_prefix_focused(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        "|A|",
        edit.value_for(PropertyFocus::LetterSpacing, &letter_spacing),
        edit.focus == Some(PropertyFocus::LetterSpacing),
        edit.caret_at(PropertyFocus::LetterSpacing),
        edit.select_all_at(PropertyFocus::LetterSpacing),
    );
    y += INPUT_HEIGHT + 8.0;

    y = paint_horizontal_align_row(cx, theme, locale, x, y, width, text.align);
    y = paint_vertical_align_row(cx, theme, locale, x, y + 6.0, width, text.vertical_align);
    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

pub fn paint_font_family_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: crate::widgets::property_panel_layout::VisibleSections,
    active_family: &str,
) {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let Some(text_y) = text_section_top(panel_rect, visible) else {
        return;
    };
    let rows = font_family_picker_action_rects(x0, text_y, usable_w);
    if rows.is_empty() {
        return;
    }
    let first = rows.first().map(|(_, r)| *r).unwrap();
    let last = rows.last().map(|(_, r)| *r).unwrap();
    let pop = Rect {
        origin: Point2D::new(first.origin.x, first.origin.y - 6.0),
        size: Point2D::new(
            first.size.x,
            last.origin.y + last.size.y - first.origin.y + 12.0,
        ),
    };
    cx.backend.fill_round_rect(pop, 8.0, theme.popover);
    cx.backend.stroke_round_rect(pop, 8.0, theme.border, 1.0);
    let active = display_font_family(active_family);
    for (action, row) in rows {
        let PropertyPanelAction::SetFontFamily(choice) = action else {
            continue;
        };
        let is_active = choice.family() == active;
        if is_active {
            cx.backend
                .fill_round_rect(row, 6.0, theme.row_selected_primary);
        }
        let label = TextLayout::single_run(
            choice.family(),
            choice.family(),
            12.0,
            to_jian_color(if is_active {
                theme.primary
            } else {
                theme.foreground
            }),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label,
            Point2D::new(row.origin.x + 10.0, row.origin.y + 19.0),
        );
        if is_active {
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(row.origin.x + row.size.x - 22.0, row.origin.y + 7.0),
                14.0,
                theme.primary,
                1.6,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_font_weight_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: crate::widgets::property_panel_layout::VisibleSections,
    locale: op_editor_core::Locale,
    active_weight: u16,
    hover: Option<usize>,
) {
    let x0 = panel_rect.origin.x;
    let w = panel_rect.size.x;
    let usable_w = w - PAD_X * 2.0;
    let Some(text_y) = text_section_top(panel_rect, visible) else {
        return;
    };
    let rows = font_weight_picker_action_rects(x0, text_y, usable_w);
    if rows.is_empty() {
        return;
    }
    let first = rows.first().map(|(_, r)| *r).unwrap();
    let last = rows.last().map(|(_, r)| *r).unwrap();
    let pop = Rect {
        origin: Point2D::new(first.origin.x, first.origin.y - 6.0),
        size: Point2D::new(
            first.size.x,
            last.origin.y + last.size.y - first.origin.y + 12.0,
        ),
    };
    cx.backend.fill_round_rect(pop, 8.0, theme.popover);
    cx.backend.stroke_round_rect(pop, 8.0, theme.border, 1.0);
    let active = FontWeightChoice::nearest(active_weight);
    for (i, (action, row)) in rows.into_iter().enumerate() {
        let PropertyPanelAction::SetFontWeight(choice) = action else {
            continue;
        };
        let is_active = choice == active;
        if is_active {
            cx.backend
                .fill_round_rect(row, 6.0, theme.row_selected_primary);
        } else if hover == Some(i) {
            // Muted hover wash matching the other dropdowns.
            cx.backend.fill_round_rect(row, 6.0, theme.button_hover);
        }
        // "number + name" — e.g. `400 Regular`, `800 Extra Bold`.
        let row_label = format!(
            "{}  {}",
            choice.numeric_label(),
            op_i18n::translate(locale, choice.label_key())
        );
        let label = TextLayout::single_run(
            &row_label,
            "system-ui",
            12.0,
            to_jian_color(if is_active {
                theme.primary
            } else {
                theme.foreground
            }),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label,
            Point2D::new(row.origin.x + 10.0, row.origin.y + 19.0),
        );
        if is_active {
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(row.origin.x + row.size.x - 22.0, row.origin.y + 7.0),
                14.0,
                theme.primary,
                1.6,
            );
        }
    }
}

fn text_section_top(
    panel_rect: Rect,
    visible: crate::widgets::property_panel_layout::VisibleSections,
) -> Option<f32> {
    if !visible.text {
        return None;
    }
    let mut y = panel_rect.origin.y;
    y += crate::widgets::property_panel_inputs::TAB_HEIGHT;
    y += crate::widgets::property_panel_inputs::HEADER_HEIGHT;
    if visible.create_component {
        y += crate::widgets::property_panel_inputs::CREATE_COMPONENT_BLOCK_H;
    }
    y += SECTION_HEADER_HEIGHT;
    y += INPUT_HEIGHT + 6.0;
    y += INPUT_HEIGHT + 12.0;
    y += SECTION_GAP;
    if visible.flex_layout {
        y += crate::widgets::property_panel_flex::flex_section_height(
            visible.flex_layout_mode,
            visible.padding_edit_mode,
        );
    }
    if visible.size_options {
        y += SECTION_HEADER_HEIGHT;
        y += INPUT_HEIGHT + 10.0;
        y += 22.0 * if visible.clip_content { 3.0 } else { 2.0 };
        y += 12.0 + SECTION_GAP;
    }
    if visible.icon {
        y += crate::widgets::property_panel_icon::icon_section_height();
    }
    Some(y)
}

fn paint_text_growth_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
    active: TextGrowthValue,
) -> f32 {
    let usable_w = width - PAD_X * 2.0;
    let gap = 6.0;
    let button_w = (usable_w - gap * 2.0) / 3.0;
    let specs = [
        (TextGrowthValue::Auto, "textLayout.autoWidth"),
        (TextGrowthValue::FixedWidth, "textLayout.autoHeight"),
        (TextGrowthValue::FixedWidthHeight, "textLayout.fixed"),
    ];
    for (i, (value, key)) in specs.iter().enumerate() {
        let rect = Rect {
            origin: Point2D::new(x + PAD_X + i as f32 * (button_w + gap), y),
            size: Point2D::new(button_w, BUTTON_H),
        };
        let is_active = *value == active;
        if is_active {
            cx.backend.fill_round_rect(rect, 6.0, theme.primary);
        } else {
            cx.backend.fill_round_rect(rect, 6.0, theme.muted);
        }
        let color = if is_active {
            theme.primary_foreground
        } else {
            theme.muted_foreground
        };
        let label = op_i18n::translate(locale, key);
        let layout = TextLayout::single_run(
            label,
            "system-ui",
            10.0,
            to_jian_color(color),
            Point2D::new(0.0, 0.0),
        );
        let label_w = cx.backend.measure_text(label, 10.0);
        cx.backend.draw_text(
            &layout,
            Point2D::new(
                rect.origin.x + (rect.size.x - label_w) / 2.0,
                rect.origin.y + 18.0,
            ),
        );
    }
    y + BUTTON_H
}

fn paint_horizontal_align_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
    active: TextAlignValue,
) -> f32 {
    paint_align_row(
        cx,
        theme,
        locale,
        x,
        y,
        width,
        "text.horizontal",
        &H_ALIGN_SPECS,
        active,
    )
}

fn paint_vertical_align_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
    active: TextVerticalAlignValue,
) -> f32 {
    paint_align_row(
        cx,
        theme,
        locale,
        x,
        y,
        width,
        "text.vertical",
        &V_ALIGN_SPECS,
        active,
    )
}

#[allow(clippy::too_many_arguments)]
fn paint_align_row<T: Copy + PartialEq>(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
    label_key: &'static str,
    specs: &[AlignButtonSpec<T>],
    active: T,
) -> f32 {
    let label = TextLayout::single_run(
        op_i18n::translate(locale, label_key),
        "system-ui",
        11.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label, Point2D::new(x + PAD_X, y + 13.0));
    let y = y + ALIGN_LABEL_H;
    let gap = 6.0;
    let usable_w = width - PAD_X * 2.0;
    let button_w = (usable_w - gap * (specs.len().saturating_sub(1) as f32)) / specs.len() as f32;
    for (i, spec) in specs.iter().enumerate() {
        let rect = Rect {
            origin: Point2D::new(x + PAD_X + i as f32 * (button_w + gap), y),
            size: Point2D::new(button_w, BUTTON_H),
        };
        let is_active = spec.value == active;
        if is_active {
            cx.backend.fill_round_rect(rect, 6.0, theme.primary);
        } else {
            cx.backend.fill_round_rect(rect, 6.0, theme.muted);
            cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
        }
        draw_icon(
            cx.backend,
            spec.icon,
            Point2D::new(rect.origin.x + (button_w - 16.0) / 2.0, rect.origin.y + 6.0),
            16.0,
            if is_active {
                theme.primary_foreground
            } else {
                theme.muted_foreground
            },
            1.4,
        );
    }
    y + BUTTON_H
}

#[derive(Clone, Copy)]
struct AlignButtonSpec<T> {
    value: T,
    icon: Icon,
}

const H_ALIGN_SPECS: [AlignButtonSpec<TextAlignValue>; 4] = [
    AlignButtonSpec {
        value: TextAlignValue::Left,
        icon: Icon::AlignLeft,
    },
    AlignButtonSpec {
        value: TextAlignValue::Center,
        icon: Icon::AlignCenterH,
    },
    AlignButtonSpec {
        value: TextAlignValue::Right,
        icon: Icon::AlignRight,
    },
    AlignButtonSpec {
        value: TextAlignValue::Justify,
        icon: Icon::AlignCenterH,
    },
];

const V_ALIGN_SPECS: [AlignButtonSpec<TextVerticalAlignValue>; 3] = [
    AlignButtonSpec {
        value: TextVerticalAlignValue::Top,
        icon: Icon::AlignTop,
    },
    AlignButtonSpec {
        value: TextVerticalAlignValue::Middle,
        icon: Icon::AlignCenterV,
    },
    AlignButtonSpec {
        value: TextVerticalAlignValue::Bottom,
        icon: Icon::AlignBottom,
    },
];

fn format_panel_number(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.2}")
    }
}

fn display_font_family(value: &str) -> &str {
    value
        .split(',')
        .next()
        .unwrap_or(value)
        .trim()
        .trim_matches(['"', '\''])
}
