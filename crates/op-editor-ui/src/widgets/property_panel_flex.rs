//! Flex-layout section paint and geometry helpers.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{
    LayoutAlignValue, LayoutJustifyValue, NodeSnapshot, PropertyPanelAction,
};
use crate::widgets::property_panel_inputs::{
    paint_input_with_prefix_focused, paint_section_divider, paint_section_label, to_jian_color,
    INPUT_HEIGHT, PAD_X, SECTION_GAP, SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::{FlexLayout, PropertyFocus};

const DIR_BUTTON_W: f32 = 56.0;
const DIR_BUTTON_H: f32 = 32.0;
const DIR_GAP: f32 = 8.0;
const ADVANCED_TOP_GAP: f32 = 10.0;
const SUB_LABEL_H: f32 = 18.0;
const GRID_CELL_W: f32 = 34.0;
const GRID_CELL_H: f32 = 22.0;
const GRID_GAP: f32 = 3.0;
const GAP_BUTTON_H: f32 = 24.0;
const PADDING_ROW_GAP: f32 = 6.0;

fn alignment_grid_h() -> f32 {
    GRID_CELL_H * 3.0 + GRID_GAP * 2.0
}

fn gap_column_h() -> f32 {
    INPUT_HEIGHT + 4.0 + GAP_BUTTON_H * 3.0
}

fn alignment_block_body_h() -> f32 {
    alignment_grid_h().max(gap_column_h())
}

pub fn flex_section_height(active: FlexLayout) -> f32 {
    let base = SECTION_HEADER_HEIGHT + DIR_BUTTON_H + 12.0;
    if active == FlexLayout::Free {
        base + SECTION_GAP
    } else {
        base + ADVANCED_TOP_GAP
            + SUB_LABEL_H
            + alignment_block_body_h()
            + 8.0
            + SUB_LABEL_H
            + INPUT_HEIGHT * 2.0
            + PADDING_ROW_GAP
            + 12.0
            + SECTION_GAP
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_flex_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut y = paint_section_label(cx, theme, labels.flex_layout, x, y, width);
    paint_direction_buttons(cx, theme, snapshot.flex_layout, x, y);
    y += DIR_BUTTON_H + 12.0;
    if snapshot.flex_layout != FlexLayout::Free {
        y += ADVANCED_TOP_GAP;
        y = paint_alignment_and_gap(cx, theme, snapshot, edit, locale, x, y, width);
        y = paint_padding_inputs(cx, theme, snapshot, edit, locale, x, y, width);
    }
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

pub fn push_flex_input_rects(
    rects: &mut Vec<(PropertyFocus, Rect)>,
    x0: f32,
    y: f32,
    width: f32,
    active: FlexLayout,
) {
    if active == FlexLayout::Free {
        return;
    }
    let usable_w = width - PAD_X * 2.0;
    let mut y = y + SECTION_HEADER_HEIGHT + DIR_BUTTON_H + 12.0 + ADVANCED_TOP_GAP;
    let grid_w = GRID_CELL_W * 3.0 + GRID_GAP * 2.0;
    let gap_x = x0 + PAD_X + grid_w + 16.0;
    let gap_w = usable_w - grid_w - 16.0;
    y += SUB_LABEL_H;
    rects.push((
        PropertyFocus::LayoutGap,
        Rect {
            origin: Point2D::new(gap_x, y),
            size: Point2D::new(gap_w, INPUT_HEIGHT),
        },
    ));
    y += alignment_block_body_h() + 8.0;
    y += SUB_LABEL_H;
    let half_w = (usable_w - 8.0) / 2.0;
    let focuses = [
        PropertyFocus::PaddingTop,
        PropertyFocus::PaddingRight,
        PropertyFocus::PaddingBottom,
        PropertyFocus::PaddingLeft,
    ];
    for (i, focus) in focuses.into_iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        rects.push((
            focus,
            Rect {
                origin: Point2D::new(
                    x0 + PAD_X + col as f32 * (half_w + 8.0),
                    y + row as f32 * (INPUT_HEIGHT + PADDING_ROW_GAP),
                ),
                size: Point2D::new(half_w, INPUT_HEIGHT),
            },
        ));
    }
}

pub fn push_flex_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    x0: f32,
    y: f32,
    width: f32,
    active: FlexLayout,
    justify: LayoutJustifyValue,
) {
    let row_x = x0 + PAD_X;
    let modes = [
        FlexLayout::Free,
        FlexLayout::Vertical,
        FlexLayout::Horizontal,
    ];
    for (i, mode) in modes.iter().enumerate() {
        out.push((
            PropertyPanelAction::SetFlexLayout(*mode),
            Rect {
                origin: Point2D::new(row_x + i as f32 * (DIR_BUTTON_W + DIR_GAP), y),
                size: Point2D::new(DIR_BUTTON_W, DIR_BUTTON_H),
            },
        ));
    }
    if active == FlexLayout::Free {
        return;
    }
    let usable_w = width - PAD_X * 2.0;
    let grid_w = GRID_CELL_W * 3.0 + GRID_GAP * 2.0;
    let grid_y = y + DIR_BUTTON_H + 12.0 + ADVANCED_TOP_GAP + SUB_LABEL_H;
    let is_space = matches!(
        justify,
        LayoutJustifyValue::SpaceBetween | LayoutJustifyValue::SpaceAround
    );
    for row in 0..3 {
        for col in 0..3 {
            let justify_value = if active == FlexLayout::Vertical {
                position_to_justify(row)
            } else {
                position_to_justify(col)
            };
            let align_value = if active == FlexLayout::Vertical {
                position_to_align(col)
            } else {
                position_to_align(row)
            };
            let action = if is_space {
                PropertyPanelAction::SetLayoutAlign(align_value)
            } else {
                PropertyPanelAction::SetLayoutAlignment {
                    justify: justify_value,
                    align: align_value,
                }
            };
            out.push((
                action,
                Rect {
                    origin: Point2D::new(
                        x0 + PAD_X + col as f32 * (GRID_CELL_W + GRID_GAP),
                        grid_y + row as f32 * (GRID_CELL_H + GRID_GAP),
                    ),
                    size: Point2D::new(GRID_CELL_W, GRID_CELL_H),
                },
            ));
        }
    }
    let gap_x = x0 + PAD_X + grid_w + 16.0;
    let gap_w = usable_w - grid_w - 16.0;
    let mut gap_y = grid_y + INPUT_HEIGHT + 4.0;
    for (action, _) in [
        (
            PropertyPanelAction::SetLayoutJustify(LayoutJustifyValue::Start),
            "numeric",
        ),
        (
            PropertyPanelAction::SetLayoutJustify(LayoutJustifyValue::SpaceBetween),
            "between",
        ),
        (
            PropertyPanelAction::SetLayoutJustify(LayoutJustifyValue::SpaceAround),
            "around",
        ),
    ] {
        out.push((
            action,
            Rect {
                origin: Point2D::new(gap_x, gap_y),
                size: Point2D::new(gap_w, GAP_BUTTON_H),
            },
        ));
        gap_y += GAP_BUTTON_H;
    }
}

fn paint_direction_buttons(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    active: FlexLayout,
    x: f32,
    y: f32,
) {
    let modes = [
        (FlexLayout::Free, Icon::LayoutGrid),
        (FlexLayout::Vertical, Icon::Rows3),
        (FlexLayout::Horizontal, Icon::Columns3),
    ];
    for (i, (mode, icon)) in modes.iter().enumerate() {
        let rect = Rect {
            origin: Point2D::new(x + PAD_X + i as f32 * (DIR_BUTTON_W + DIR_GAP), y),
            size: Point2D::new(DIR_BUTTON_W, DIR_BUTTON_H),
        };
        let is_active = *mode == active;
        if is_active {
            cx.backend.fill_round_rect(rect, 6.0, theme.primary);
        } else {
            cx.backend.fill_round_rect(rect, 6.0, theme.muted);
            cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
        }
        let icon_color = if is_active {
            theme.primary_foreground
        } else {
            theme.muted_foreground
        };
        draw_icon(
            cx.backend,
            *icon,
            Point2D::new(
                rect.origin.x + (DIR_BUTTON_W - 18.0) / 2.0,
                rect.origin.y + 7.0,
            ),
            18.0,
            icon_color,
            1.4,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_alignment_and_gap(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let usable_w = width - PAD_X * 2.0;
    let grid_w = GRID_CELL_W * 3.0 + GRID_GAP * 2.0;
    paint_sub_label(
        cx,
        theme,
        op_i18n::translate(locale, "layout.alignment"),
        x + PAD_X,
        y,
    );
    paint_sub_label(
        cx,
        theme,
        op_i18n::translate(locale, "layout.gap"),
        x + PAD_X + grid_w + 16.0,
        y,
    );
    let grid_y = y + SUB_LABEL_H;
    paint_alignment_grid(cx, theme, snapshot, x + PAD_X, grid_y);
    let gap_x = x + PAD_X + grid_w + 16.0;
    let gap_w = usable_w - grid_w - 16.0;
    let gap_text = format_panel_number(snapshot.layout_gap);
    paint_input_with_prefix_focused(
        cx,
        theme,
        Rect {
            origin: Point2D::new(gap_x, grid_y),
            size: Point2D::new(gap_w, INPUT_HEIGHT),
        },
        "G",
        edit.value_for(PropertyFocus::LayoutGap, &gap_text),
        edit.focus == Some(PropertyFocus::LayoutGap),
        edit.caret_at(PropertyFocus::LayoutGap),
    );
    let mut yy = grid_y + INPUT_HEIGHT + 4.0;
    yy = paint_gap_mode_button(
        cx,
        theme,
        locale,
        gap_x,
        yy,
        gap_w,
        snapshot.layout_justify == LayoutJustifyValue::Start,
        "layout.gap",
    );
    yy = paint_gap_mode_button(
        cx,
        theme,
        locale,
        gap_x,
        yy,
        gap_w,
        snapshot.layout_justify == LayoutJustifyValue::SpaceBetween,
        "layout.spaceBetween",
    );
    let _ = paint_gap_mode_button(
        cx,
        theme,
        locale,
        gap_x,
        yy,
        gap_w,
        snapshot.layout_justify == LayoutJustifyValue::SpaceAround,
        "layout.spaceAround",
    );
    grid_y + alignment_block_body_h() + 8.0
}

#[allow(clippy::too_many_arguments)]
fn paint_padding_inputs(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    paint_sub_label(
        cx,
        theme,
        op_i18n::translate(locale, "padding.title"),
        x + PAD_X,
        y,
    );
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let mut yy = y + SUB_LABEL_H;
    let rows = [
        (PropertyFocus::PaddingTop, "T", snapshot.layout_padding.top),
        (
            PropertyFocus::PaddingRight,
            "R",
            snapshot.layout_padding.right,
        ),
        (
            PropertyFocus::PaddingBottom,
            "B",
            snapshot.layout_padding.bottom,
        ),
        (
            PropertyFocus::PaddingLeft,
            "L",
            snapshot.layout_padding.left,
        ),
    ];
    for pair in rows.chunks(2) {
        for (col, (focus, label, value)) in pair.iter().enumerate() {
            let value = format_panel_number(*value);
            paint_input_with_prefix_focused(
                cx,
                theme,
                Rect {
                    origin: Point2D::new(x + PAD_X + col as f32 * (half_w + 8.0), yy),
                    size: Point2D::new(half_w, INPUT_HEIGHT),
                },
                label,
                edit.value_for(*focus, &value),
                edit.focus == Some(*focus),
                edit.caret_at(*focus),
            );
        }
        yy += INPUT_HEIGHT + PADDING_ROW_GAP;
    }
    yy - PADDING_ROW_GAP + 12.0
}

fn paint_alignment_grid(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    x: f32,
    y: f32,
) {
    let is_space = matches!(
        snapshot.layout_justify,
        LayoutJustifyValue::SpaceBetween | LayoutJustifyValue::SpaceAround
    );
    let is_vertical = snapshot.flex_layout == FlexLayout::Vertical;
    let bg = Rect {
        origin: Point2D::new(x - 6.0, y - 6.0),
        size: Point2D::new(
            GRID_CELL_W * 3.0 + GRID_GAP * 2.0 + 12.0,
            GRID_CELL_H * 3.0 + GRID_GAP * 2.0 + 12.0,
        ),
    };
    cx.backend.fill_round_rect(bg, 6.0, theme.muted);
    for row in 0..3 {
        for col in 0..3 {
            let justify = if is_vertical {
                position_to_justify(row)
            } else {
                position_to_justify(col)
            };
            let align = if is_vertical {
                position_to_align(col)
            } else {
                position_to_align(row)
            };
            let active = if is_space {
                snapshot.layout_align == align
            } else {
                snapshot.layout_justify == justify && snapshot.layout_align == align
            };
            let cell = Rect {
                origin: Point2D::new(
                    x + col as f32 * (GRID_CELL_W + GRID_GAP),
                    y + row as f32 * (GRID_CELL_H + GRID_GAP),
                ),
                size: Point2D::new(GRID_CELL_W, GRID_CELL_H),
            };
            let dot = if active {
                Rect {
                    origin: Point2D::new(cell.origin.x + 12.0, cell.origin.y + 6.0),
                    size: Point2D::new(10.0, 10.0),
                }
            } else {
                Rect {
                    origin: Point2D::new(cell.origin.x + 15.0, cell.origin.y + 9.0),
                    size: Point2D::new(4.0, 4.0),
                }
            };
            cx.backend.fill_round_rect(
                dot,
                if active { 2.0 } else { 4.0 },
                if active {
                    theme.primary
                } else {
                    theme.muted_foreground
                },
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_gap_mode_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
    width: f32,
    active: bool,
    label_key: &'static str,
) -> f32 {
    let rect = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(width, GAP_BUTTON_H),
    };
    if active {
        cx.backend.fill_round_rect(rect, 5.0, theme.primary);
    }
    let color = if active {
        theme.primary_foreground
    } else {
        theme.muted_foreground
    };
    let label = TextLayout::single_run(
        op_i18n::translate(locale, label_key),
        "system-ui",
        10.0,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label,
        Point2D::new(rect.origin.x + 8.0, rect.origin.y + 16.0),
    );
    y + GAP_BUTTON_H
}

fn paint_sub_label(cx: &mut PaintCx<'_>, theme: &Theme, label: &str, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        10.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y + 13.0));
}

fn position_to_justify(index: usize) -> LayoutJustifyValue {
    match index {
        0 => LayoutJustifyValue::Start,
        1 => LayoutJustifyValue::Center,
        _ => LayoutJustifyValue::End,
    }
}

fn position_to_align(index: usize) -> LayoutAlignValue {
    match index {
        0 => LayoutAlignValue::Start,
        1 => LayoutAlignValue::Center,
        _ => LayoutAlignValue::End,
    }
}

fn format_panel_number(value: f32) -> String {
    if value.fract().abs() < f32::EPSILON {
        format!("{}", value.round() as i32)
    } else {
        format!("{value:.2}")
    }
}
