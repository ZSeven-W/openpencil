//! Text-specific property section for the native right panel.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{
    NodeSnapshot, PropertyPanelAction, TextAlignValue, TextGrowthValue, TextVerticalAlignValue,
};
use crate::widgets::property_panel_inputs::{
    paint_input_with_prefix_focused, paint_section_divider, paint_section_label, to_jian_color,
    INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::EditContext;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

const FAMILY_ROW_GAP: f32 = 6.0;
const ALIGN_LABEL_H: f32 = 18.0;
const BUTTON_H: f32 = 28.0;
const TEXT_LAYOUT_BLOCK_H: f32 = SECTION_HEADER_HEIGHT + BUTTON_H + 12.0;

pub fn text_section_height() -> f32 {
    TEXT_LAYOUT_BLOCK_H
        + SECTION_HEADER_HEIGHT
        + INPUT_HEIGHT
        + FAMILY_ROW_GAP
        + INPUT_HEIGHT
        + 6.0
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
    rects.push((
        PropertyFocus::FontWeight,
        Rect {
            origin: Point2D::new(x0 + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
    ));
    rects.push((
        PropertyFocus::FontSize,
        Rect {
            origin: Point2D::new(x0 + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
    ));
    y += INPUT_HEIGHT + 6.0;
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
    let weight = text.font_weight.to_string();
    paint_input_with_prefix_focused(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        "W",
        edit.value_for(PropertyFocus::FontWeight, &weight),
        edit.focus == Some(PropertyFocus::FontWeight),
        edit.caret_at(PropertyFocus::FontWeight),
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
    );
    y += INPUT_HEIGHT + 6.0;

    let line_height = format_panel_number(text.line_height_percent);
    paint_input_with_prefix_focused(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        "LH",
        edit.value_for(PropertyFocus::LineHeight, &line_height),
        edit.focus == Some(PropertyFocus::LineHeight),
        edit.caret_at(PropertyFocus::LineHeight),
    );
    let letter_spacing = format_panel_number(text.letter_spacing);
    paint_input_with_prefix_focused(
        cx,
        theme,
        Rect {
            origin: Point2D::new(x + PAD_X + half_w + 8.0, y),
            size: Point2D::new(half_w, INPUT_HEIGHT),
        },
        "LS",
        edit.value_for(PropertyFocus::LetterSpacing, &letter_spacing),
        edit.focus == Some(PropertyFocus::LetterSpacing),
        edit.caret_at(PropertyFocus::LetterSpacing),
    );
    y += INPUT_HEIGHT + 8.0;

    y = paint_horizontal_align_row(cx, theme, locale, x, y, width, text.align);
    y = paint_vertical_align_row(cx, theme, locale, x, y + 6.0, width, text.vertical_align);
    y += 12.0;
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
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
