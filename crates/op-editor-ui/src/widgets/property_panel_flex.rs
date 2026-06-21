//! Flex-layout section paint and geometry helpers.

use crate::theme::Theme;
use crate::util::format_panel_number;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{
    LayoutAlignValue, LayoutJustifyValue, NodeSnapshot, PropertyPanelAction,
};
use crate::widgets::property_panel_inputs::{
    paint_input_with_prefix_focused_state, paint_input_with_suffix_focused_state,
    paint_section_divider, paint_section_label, INPUT_HEIGHT, INPUT_RADIUS, PAD_X, SECTION_GAP,
    SECTION_HEADER_HEIGHT,
};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::{FlexLayout, PaddingEditMode, PropertyFocus};

const DIR_BUTTON_W: f32 = 56.0;
const DIR_BUTTON_H: f32 = 32.0;
const DIR_GAP: f32 = 8.0;
const ADVANCED_TOP_GAP: f32 = 10.0;
const SUB_LABEL_H: f32 = 18.0;
const GRID_CELL_W: f32 = 34.0;
const GRID_CELL_H: f32 = 22.0;
const GRID_GAP: f32 = 3.0;
/// Internal padding of the alignment grid box (TS `p-2` == 8px) —
/// insets the dot cells inside the `theme.muted` box so the box top
/// stays flush with the gap-input top in the right column.
const GRID_PAD: f32 = 8.0;
/// Horizontal offset from the grid's cell-area left edge to the gap
/// column. The grid box is `grid_w + 2*GRID_PAD` wide; this leaves an
/// 8px (TS `gap-2`) visual gap so the muted grid box and the muted gap
/// pill don't merge into one blob.
const GRID_COL_GAP: f32 = GRID_PAD * 2.0 + 8.0;
/// Gap radio-row height (the row-0 number input is `GAP_ROW_H` tall —
/// shorter than the standard 30px input for a tighter cluster) and the
/// gap between rows. Three rows = 20×3 + 2×2 = 64, which fits inside the
/// alignment grid's 88px block (so the block height — and every section
/// below — is unchanged), just more compact.
const GAP_ROW_H: f32 = 20.0;
const GAP_ROW_GAP: f32 = 2.0;
/// RadioCircle geometry (compact ring + dot).
const RADIO_SIZE: f32 = 13.0;
/// x-offset from a radio row's left to its content (TS `gap-1.5` = 6px
/// + the 14px circle).
const RADIO_GUTTER: f32 = 6.0 + RADIO_SIZE;
const PADDING_ROW_GAP: f32 = 6.0;

fn alignment_grid_h() -> f32 {
    GRID_CELL_H * 3.0 + GRID_GAP * 2.0 + GRID_PAD * 2.0
}

fn gap_column_h() -> f32 {
    GAP_ROW_H * 3.0 + GAP_ROW_GAP * 2.0
}

/// Top-Y of gap radio row `i` (0 = numeric/input, 1 = space-between,
/// 2 = space-around). Single source of truth so paint + both hit-test
/// walkers can never drift.
fn gap_row_y(grid_y: f32, i: usize) -> f32 {
    grid_y + i as f32 * (GAP_ROW_H + GAP_ROW_GAP)
}

fn alignment_block_body_h() -> f32 {
    alignment_grid_h().max(gap_column_h())
}

/// Geometry of the padding-section gear + its mode popover, anchored off
/// the flex section's top `y` (same origin `push_flex_action_rects`
/// uses). Returns `(gear_rect, popover_box, [single, axis, individual]
/// row rects)` so paint + hit-test share one source of truth.
fn padding_mode_popover_layout(x0: f32, y: f32, width: f32) -> (Rect, Rect, [Rect; 3]) {
    let grid_y = y + DIR_BUTTON_H + 12.0 + ADVANCED_TOP_GAP + SUB_LABEL_H;
    let sublabel_y = grid_y + alignment_block_body_h() + 8.0;
    let gear = Rect {
        origin: Point2D::new(x0 + width - PAD_X - 18.0, sublabel_y - 2.0),
        size: Point2D::new(18.0, 18.0),
    };
    let pop_w = 190.0_f32.min(width - PAD_X * 2.0);
    let pop_x = gear.origin.x + 18.0 - pop_w; // right-aligned under the gear
    let pad = 6.0;
    let title_h = 22.0;
    let row_h = 26.0;
    let pop_box = Rect {
        origin: Point2D::new(pop_x, sublabel_y + 20.0),
        size: Point2D::new(pop_w, pad * 2.0 + title_h + row_h * 3.0),
    };
    let first_row = pop_box.origin.y + pad + title_h;
    let row = |i: usize| Rect {
        origin: Point2D::new(pop_x + pad, first_row + i as f32 * row_h),
        size: Point2D::new(pop_w - pad * 2.0, row_h),
    };
    (gear, pop_box, [row(0), row(1), row(2)])
}

pub fn flex_section_height(active: FlexLayout, padding_mode: PaddingEditMode) -> f32 {
    let base = SECTION_HEADER_HEIGHT + DIR_BUTTON_H + 12.0;
    if active == FlexLayout::Free {
        return base + SECTION_GAP;
    }
    // Padding input ROWS: Individual is a 2×2 grid (2 rows); Single
    // (1 input) and Axis (2 inputs side-by-side) are a single row.
    let rows = if padding_mode == PaddingEditMode::Individual {
        2.0
    } else {
        1.0
    };
    let padding_block = SUB_LABEL_H + INPUT_HEIGHT * rows + PADDING_ROW_GAP * (rows - 1.0) + 12.0;
    base + ADVANCED_TOP_GAP
        + SUB_LABEL_H
        + alignment_block_body_h()
        + 8.0
        + padding_block
        + SECTION_GAP
}

#[allow(clippy::too_many_arguments)]
pub fn paint_flex_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    locale: op_editor_core::Locale,
    padding_mode: PaddingEditMode,
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
        y = paint_padding_inputs(cx, theme, snapshot, edit, locale, padding_mode, x, y, width);
    }
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

#[allow(clippy::too_many_arguments)]
pub fn push_flex_input_rects(
    rects: &mut Vec<(PropertyFocus, Rect)>,
    x0: f32,
    y: f32,
    width: f32,
    active: FlexLayout,
    justify: LayoutJustifyValue,
    padding_mode: PaddingEditMode,
) {
    if active == FlexLayout::Free {
        return;
    }
    let usable_w = width - PAD_X * 2.0;
    let mut y = y + SECTION_HEADER_HEIGHT + DIR_BUTTON_H + 12.0 + ADVANCED_TOP_GAP;
    let grid_w = GRID_CELL_W * 3.0 + GRID_GAP * 2.0;
    let gap_x = x0 + PAD_X + grid_w + GRID_COL_GAP;
    let gap_w = usable_w - grid_w - GRID_COL_GAP;
    y += SUB_LABEL_H;
    // Row 0 of the gap radio column: the number-input hit area starts
    // PAST the radio circle so clicking the circle dispatches
    // SetLayoutJustify(Start) (action rect) while the number area
    // focuses the input. Space-between / space-around auto-distribute
    // the gap, so the input is disabled — emit no focus rect for it.
    let gap_disabled = matches!(
        justify,
        LayoutJustifyValue::SpaceBetween | LayoutJustifyValue::SpaceAround
    );
    if !gap_disabled {
        rects.push((
            PropertyFocus::LayoutGap,
            Rect {
                origin: Point2D::new(gap_x + RADIO_GUTTER, gap_row_y(y, 0)),
                size: Point2D::new(gap_w - RADIO_GUTTER, GAP_ROW_H),
            },
        ));
    }
    y += alignment_block_body_h() + 8.0;
    y += SUB_LABEL_H;
    let half_w = (usable_w - 8.0) / 2.0;
    let cell = |col: f32, row: f32| Rect {
        origin: Point2D::new(
            x0 + PAD_X + col * (half_w + 8.0),
            y + row * (INPUT_HEIGHT + PADDING_ROW_GAP),
        ),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    // Emit only the focus rects the active mode paints; the multi-side
    // write happens at commit time (see commit_property_edit).
    match padding_mode {
        PaddingEditMode::Single => {
            rects.push((
                PropertyFocus::PaddingTop,
                Rect {
                    origin: Point2D::new(x0 + PAD_X, y),
                    size: Point2D::new(usable_w, INPUT_HEIGHT),
                },
            ));
        }
        PaddingEditMode::Axis => {
            rects.push((PropertyFocus::PaddingRight, cell(0.0, 0.0)));
            rects.push((PropertyFocus::PaddingTop, cell(1.0, 0.0)));
        }
        PaddingEditMode::Individual => {
            rects.push((PropertyFocus::PaddingTop, cell(0.0, 0.0)));
            rects.push((PropertyFocus::PaddingRight, cell(1.0, 0.0)));
            rects.push((PropertyFocus::PaddingBottom, cell(0.0, 1.0)));
            rects.push((PropertyFocus::PaddingLeft, cell(1.0, 1.0)));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn push_flex_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    x0: f32,
    y: f32,
    width: f32,
    active: FlexLayout,
    justify: LayoutJustifyValue,
    padding_mode_popover_open: bool,
) {
    let row_x = x0 + PAD_X;
    let modes = [
        FlexLayout::Free,
        FlexLayout::Vertical,
        FlexLayout::Horizontal,
    ];
    // Continuous even cells over the same footprint the ToggleGroup paints.
    let cell_w = (DIR_BUTTON_W * 3.0 + DIR_GAP * 2.0) / 3.0;
    for (i, mode) in modes.iter().enumerate() {
        out.push((
            PropertyPanelAction::SetFlexLayout(*mode),
            Rect {
                origin: Point2D::new(row_x + i as f32 * cell_w, y),
                size: Point2D::new(cell_w, DIR_BUTTON_H),
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
                        x0 + PAD_X + GRID_PAD + col as f32 * (GRID_CELL_W + GRID_GAP),
                        grid_y + GRID_PAD + row as f32 * (GRID_CELL_H + GRID_GAP),
                    ),
                    size: Point2D::new(GRID_CELL_W, GRID_CELL_H),
                },
            ));
        }
    }
    let gap_x = x0 + PAD_X + grid_w + GRID_COL_GAP;
    let gap_w = usable_w - grid_w - GRID_COL_GAP;
    // 3 gap radio rows. Row 0 (numeric) only takes the circle as its
    // select target — the rest of that row is the number input. Rows
    // 1/2 are whole-row select targets (TS cursor-pointer rows).
    let rows = [
        (LayoutJustifyValue::Start, true),
        (LayoutJustifyValue::SpaceBetween, false),
        (LayoutJustifyValue::SpaceAround, false),
    ];
    for (i, (justify_value, circle_only)) in rows.into_iter().enumerate() {
        // The numeric/Start row's target is just the radio circle — centre its
        // hover-wash cell on the RADIO_SIZE glyph (the radio paints at gap_x,
        // column-aligned with the other rows, so shift the cell left instead).
        let (row_x, row_w) = if circle_only {
            (gap_x - (RADIO_GUTTER - RADIO_SIZE) / 2.0, RADIO_GUTTER)
        } else {
            (gap_x, gap_w)
        };
        out.push((
            PropertyPanelAction::SetLayoutJustify(justify_value),
            Rect {
                origin: Point2D::new(row_x, gap_row_y(grid_y, i)),
                size: Point2D::new(row_w, GAP_ROW_H),
            },
        ));
    }

    // Padding gear + (when open) the 3 mode-radio rows. The radio rects
    // are appended LAST so the open popover wins the hit-test over the
    // sections it overlaps; the host gates them behind dismiss-on-press.
    let (gear, _box, mode_rows) = padding_mode_popover_layout(x0, y, width);
    out.push((PropertyPanelAction::TogglePaddingModePopover, gear));
    if padding_mode_popover_open {
        for (i, rect) in mode_rows.into_iter().enumerate() {
            out.push((
                PropertyPanelAction::SetPaddingMode(PaddingEditMode::ALL[i]),
                rect,
            ));
        }
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
    // Fixed-width icon segmented control via jian ToggleGroup; font_size 14
    // yields an 18px icon (font + 4), matching the previous cells. The outer
    // rect spans the same footprint (3 buttons + 2 gaps) the hit walker covers.
    let labels: Vec<&str> = modes.iter().map(|_| "").collect();
    let icon_paths: Vec<&[&str]> = modes.iter().map(|(_, icon)| icon.paths()).collect();
    let active_idx = modes.iter().position(|(m, _)| *m == active).unwrap_or(0);
    let total_w = DIR_BUTTON_W * 3.0 + DIR_GAP * 2.0;
    jian_widgets::components::toggle_group::ToggleGroup {
        options: &labels,
        icons: Some(&icon_paths),
        active: active_idx,
        hover: None,
        font_size: 14.0,
    }
    .paint(
        cx.backend,
        Rect {
            origin: Point2D::new(x + PAD_X, y),
            size: Point2D::new(total_w, DIR_BUTTON_H),
        },
        &crate::widgets::button::tokens_from_theme(theme),
    );
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
    // No "间距" sub-label over the gap column — TS GapSection has no
    // header; the three radio rows are the whole control.
    let grid_y = y + SUB_LABEL_H;
    paint_alignment_grid(cx, theme, snapshot, x + PAD_X, grid_y);
    let gap_x = x + PAD_X + grid_w + GRID_COL_GAP;
    let gap_w = usable_w - grid_w - GRID_COL_GAP;
    let content_x = gap_x + RADIO_GUTTER;
    let content_w = gap_w - RADIO_GUTTER;
    let circle_y = |row_y: f32| row_y + (GAP_ROW_H - RADIO_SIZE) / 2.0;
    let justify = snapshot.layout_justify;

    // Row 0 — numeric: radio + bare number input (no "G" prefix).
    let r0_y = gap_row_y(grid_y, 0);
    paint_radio_circle(
        cx,
        theme,
        gap_x,
        circle_y(r0_y),
        justify == LayoutJustifyValue::Start,
    );
    let gap_text = format_panel_number(snapshot.layout_gap);
    let gap_rect = Rect {
        origin: Point2D::new(content_x, r0_y),
        size: Point2D::new(content_w, GAP_ROW_H),
    };
    // Space-between / space-around distribute the gap automatically, so
    // the explicit gap value is ignored. Paint the input disabled
    // (muted, no caret) — it also emits no focus rect, so edits are
    // rejected (see `push_flex_input_rects`).
    let is_space = matches!(
        justify,
        LayoutJustifyValue::SpaceBetween | LayoutJustifyValue::SpaceAround
    );
    if is_space {
        cx.backend
            .fill_round_rect(gap_rect, INPUT_RADIUS, theme.muted);
        let muted = TextLayout::single_run(
            &gap_text,
            "system-ui",
            12.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&muted, Point2D::new(content_x + 10.0, r0_y + 14.0));
    } else {
        paint_input_with_suffix_focused_state(
            cx,
            theme,
            gap_rect,
            edit.value_for(PropertyFocus::LayoutGap, &gap_text),
            "",
            edit.focus == Some(PropertyFocus::LayoutGap),
            edit.caret_at(PropertyFocus::LayoutGap),
            edit.select_all_at(PropertyFocus::LayoutGap),
            edit.input_at(PropertyFocus::LayoutGap),
            edit.now_ms,
        );
    }

    // Row 1 — space-between, Row 2 — space-around: radio + label.
    let r1_y = gap_row_y(grid_y, 1);
    paint_radio_circle(
        cx,
        theme,
        gap_x,
        circle_y(r1_y),
        justify == LayoutJustifyValue::SpaceBetween,
    );
    paint_gap_row_label(cx, theme, locale, content_x, r1_y, "layout.spaceBetween");
    let r2_y = gap_row_y(grid_y, 2);
    paint_radio_circle(
        cx,
        theme,
        gap_x,
        circle_y(r2_y),
        justify == LayoutJustifyValue::SpaceAround,
    );
    paint_gap_row_label(cx, theme, locale, content_x, r2_y, "layout.spaceAround");

    grid_y + alignment_block_body_h() + 8.0
}

#[allow(clippy::too_many_arguments)]
fn paint_padding_inputs(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    locale: op_editor_core::Locale,
    padding_mode: PaddingEditMode,
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
    // Gear at the right of the label row — opens the mode popover.
    // Centre the 14px glyph inside its 18×18 hover-wash cell (the
    // TogglePaddingModePopover action rect at x+width-PAD_X-18, y-2,
    // where this row's `y` == that rect's sublabel_y), so the gear
    // sits centred in the gray wash on hover.
    draw_icon(
        cx.backend,
        Icon::Settings,
        Point2D::new(x + width - PAD_X - 16.0, y),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let yy = y + SUB_LABEL_H;
    let p = &snapshot.layout_padding;
    let mut input = |focus: PropertyFocus, prefix: &str, value: f32, rect: Rect| {
        let value = format_panel_number(value);
        paint_input_with_prefix_focused_state(
            cx,
            theme,
            rect,
            prefix,
            edit.value_for(focus, &value),
            edit.focus == Some(focus),
            edit.caret_at(focus),
            edit.select_all_at(focus),
            edit.input_at(focus),
            edit.now_ms,
        );
    };
    let cell = |col: f32, row: f32| Rect {
        origin: Point2D::new(
            x + PAD_X + col * (half_w + 8.0),
            yy + row * (INPUT_HEIGHT + PADDING_ROW_GAP),
        ),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    let full = Rect {
        origin: Point2D::new(x + PAD_X, yy),
        size: Point2D::new(usable_w, INPUT_HEIGHT),
    };
    match padding_mode {
        PaddingEditMode::Single => {
            input(PropertyFocus::PaddingTop, "", p.top, full);
        }
        PaddingEditMode::Axis => {
            // Horizontal (left/right) then Vertical (top/bottom).
            input(PropertyFocus::PaddingRight, "H", p.right, cell(0.0, 0.0));
            input(PropertyFocus::PaddingTop, "V", p.top, cell(1.0, 0.0));
        }
        PaddingEditMode::Individual => {
            input(PropertyFocus::PaddingTop, "T", p.top, cell(0.0, 0.0));
            input(PropertyFocus::PaddingRight, "R", p.right, cell(1.0, 0.0));
            input(PropertyFocus::PaddingBottom, "B", p.bottom, cell(0.0, 1.0));
            input(PropertyFocus::PaddingLeft, "L", p.left, cell(1.0, 1.0));
        }
    }
    let rows = if padding_mode == PaddingEditMode::Individual {
        2.0
    } else {
        1.0
    };
    yy + INPUT_HEIGHT * rows + PADDING_ROW_GAP * (rows - 1.0) + 12.0
}

/// Paint the padding-mode gear popover (title + 3 radio rows). Anchored
/// off the flex section's top `y`, painted as a top overlay so it sits
/// above the sections below the gear.
#[allow(clippy::too_many_arguments)]
pub fn paint_padding_mode_popover(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    active_mode: PaddingEditMode,
    hover: Option<usize>,
    x0: f32,
    y: f32,
    width: f32,
) {
    let (_gear, pop_box, rows) = padding_mode_popover_layout(x0, y, width);
    cx.backend.fill_round_rect(pop_box, 8.0, theme.popover);
    cx.backend
        .stroke_round_rect(pop_box, 8.0, theme.border, 1.0);
    let title = TextLayout::single_run(
        op_i18n::translate(locale, "padding.paddingValues"),
        "system-ui",
        11.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &title,
        Point2D::new(pop_box.origin.x + 12.0, pop_box.origin.y + 16.0),
    );
    for (i, rect) in rows.iter().enumerate() {
        let mode = PaddingEditMode::ALL[i];
        if hover == Some(i) {
            // jian-standard button_hover row wash, matching the other dropdowns.
            cx.backend.fill_round_rect(*rect, 6.0, theme.button_hover);
        }
        paint_radio_circle(
            cx,
            theme,
            rect.origin.x + 4.0,
            rect.origin.y + (rect.size.y - RADIO_SIZE) / 2.0,
            mode == active_mode,
        );
        let label = TextLayout::single_run(
            op_i18n::translate(locale, mode.label_key()),
            "system-ui",
            11.0,
            (theme.foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &label,
            Point2D::new(
                rect.origin.x + 4.0 + RADIO_GUTTER,
                rect.origin.y + rect.size.y / 2.0 + 4.0,
            ),
        );
    }
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
    // Match TS `p-2` (8px) grid padding: the box top sits flush with
    // the body-top (`y` == gap-input top); the dot cells are inset by
    // GRID_PAD so the grid reads as one box aligned with the right
    // column instead of overshooting 6px upward into the sub-label.
    let bg = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(
            GRID_CELL_W * 3.0 + GRID_GAP * 2.0 + GRID_PAD * 2.0,
            GRID_CELL_H * 3.0 + GRID_GAP * 2.0 + GRID_PAD * 2.0,
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
                    x + GRID_PAD + col as f32 * (GRID_CELL_W + GRID_GAP),
                    y + GRID_PAD + row as f32 * (GRID_CELL_H + GRID_GAP),
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
/// 14px radio circle — a 1.5px ring (primary when selected, else
/// muted) plus an 8px filled primary dot when selected. Drawn with
/// `stroke/fill_round_rect` at full radius (a circle) to stay
/// dependency-free.
fn paint_radio_circle(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32, selected: bool) {
    let ring = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(RADIO_SIZE, RADIO_SIZE),
    };
    jian_widgets::components::radio::Radio {
        selected,
        enabled: true,
    }
    .paint(
        cx.backend,
        ring,
        &crate::widgets::button::tokens_from_theme(theme),
    );
}

/// 10px muted label vertically centred in a `GAP_ROW_H` radio row.
fn paint_gap_row_label(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    x: f32,
    row_y: f32,
    label_key: &'static str,
) {
    let label = TextLayout::single_run(
        op_i18n::translate(locale, label_key),
        "system-ui",
        10.0,
        (theme.muted_foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&label, Point2D::new(x, row_y + GAP_ROW_H / 2.0 + 4.0));
}

fn paint_sub_label(cx: &mut PaintCx<'_>, theme: &Theme, label: &str, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        label,
        "system-ui",
        10.0,
        (theme.muted_foreground).to_jian(),
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
