//! Stroke-section paint and geometry helpers for the property panel.

use crate::theme::Theme;
use crate::util::format_panel_number;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::NodeSnapshot;
use crate::widgets::property_panel_action::PropertyPanelAction;
use crate::widgets::property_panel_color_variables::paint_color_variable_button;
use crate::widgets::property_panel_inputs::{
    format_color_hex, paint_input_with_prefix_focused_state, paint_section_divider,
    paint_section_label, COLOR_VARIABLE_BUTTON_W, COLOR_VARIABLE_GAP, INPUT_HEIGHT, INPUT_RADIUS,
    PAD_X, SECTION_GAP,
};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::{PaddingEditMode, PropertyFocus};

const SIDE_GRID_TOP_GAP: f32 = 8.0;
const SIDE_GRID_GAP: f32 = 6.0;
const SECTION_TRAILING_GAP: f32 = 12.0;
const RADIO_SIZE: f32 = 14.0;
const RADIO_GUTTER: f32 = 26.0;

struct StrokeSideGridRefs<'a, 'b> {
    theme: &'a Theme,
    snapshot: &'a NodeSnapshot,
    edit: &'a EditContext<'b>,
}

pub(crate) fn stroke_section_body_height(mode: PaddingEditMode) -> f32 {
    let rows = if mode == PaddingEditMode::Individual {
        2.0
    } else {
        1.0
    };
    INPUT_HEIGHT
        + SIDE_GRID_TOP_GAP
        + INPUT_HEIGHT * rows
        + SIDE_GRID_GAP * (rows - 1.0)
        + SECTION_TRAILING_GAP
}

fn side_grid_y(row_y: f32) -> f32 {
    row_y + INPUT_HEIGHT + SIDE_GRID_TOP_GAP
}

pub(crate) fn stroke_side_input_rects(
    x: f32,
    row_y: f32,
    width: f32,
    mode: PaddingEditMode,
) -> Vec<(PropertyFocus, Rect)> {
    let usable_w = width - PAD_X * 2.0;
    let half_w = (usable_w - 8.0) / 2.0;
    let grid_y = side_grid_y(row_y);
    let cell = |col: f32, row: f32| Rect {
        origin: Point2D::new(
            x + PAD_X + col * (half_w + 8.0),
            grid_y + row * (INPUT_HEIGHT + SIDE_GRID_GAP),
        ),
        size: Point2D::new(half_w, INPUT_HEIGHT),
    };
    match mode {
        PaddingEditMode::Single => vec![(
            PropertyFocus::StrokeTopWidth,
            Rect {
                origin: Point2D::new(x + PAD_X, grid_y),
                size: Point2D::new(usable_w, INPUT_HEIGHT),
            },
        )],
        PaddingEditMode::Axis => vec![
            (PropertyFocus::StrokeRightWidth, cell(0.0, 0.0)),
            (PropertyFocus::StrokeTopWidth, cell(1.0, 0.0)),
        ],
        PaddingEditMode::Individual => vec![
            (PropertyFocus::StrokeTopWidth, cell(0.0, 0.0)),
            (PropertyFocus::StrokeRightWidth, cell(1.0, 0.0)),
            (PropertyFocus::StrokeBottomWidth, cell(0.0, 1.0)),
            (PropertyFocus::StrokeLeftWidth, cell(1.0, 1.0)),
        ],
    }
}

fn stroke_mode_popover_layout(x0: f32, y: f32, width: f32) -> (Rect, Rect, [Rect; 3]) {
    let gear = Rect {
        origin: Point2D::new(x0 + width - PAD_X - 18.0, y + 2.0),
        size: Point2D::new(18.0, 18.0),
    };
    let pop_w = 190.0_f32.min(width - PAD_X * 2.0);
    let pop_x = gear.origin.x + 18.0 - pop_w;
    let pad = 6.0;
    let title_h = 22.0;
    let row_h = 26.0;
    let pop_box = Rect {
        origin: Point2D::new(pop_x, y + 24.0),
        size: Point2D::new(pop_w, pad * 2.0 + title_h + row_h * 3.0),
    };
    let first_row = pop_box.origin.y + pad + title_h;
    let row = |i: usize| Rect {
        origin: Point2D::new(pop_x + pad, first_row + i as f32 * row_h),
        size: Point2D::new(pop_w - pad * 2.0, row_h),
    };
    (gear, pop_box, [row(0), row(1), row(2)])
}

pub(crate) fn push_stroke_action_rects(
    out: &mut Vec<(PropertyPanelAction, Rect)>,
    x0: f32,
    y: f32,
    width: f32,
    popover_open: bool,
) {
    let (gear, _box, rows) = stroke_mode_popover_layout(x0, y, width);
    out.push((PropertyPanelAction::ToggleStrokeModePopover, gear));
    if popover_open {
        for (i, rect) in rows.into_iter().enumerate() {
            out.push((
                PropertyPanelAction::SetStrokeMode(PaddingEditMode::ALL[i]),
                rect,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_stroke_mode_popover(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    active_mode: PaddingEditMode,
    hover: Option<usize>,
    x0: f32,
    y: f32,
    width: f32,
) {
    let (_gear, pop_box, rows) = stroke_mode_popover_layout(x0, y, width);
    cx.backend.fill_round_rect(pop_box, 8.0, theme.popover);
    cx.backend
        .stroke_round_rect(pop_box, 8.0, theme.border, 1.0);
    let title = TextLayout::single_run(
        op_i18n::translate(locale, "stroke.title"),
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

fn paint_radio_circle(cx: &mut PaintCx<'_>, theme: &Theme, x: f32, y: f32, selected: bool) {
    jian_widgets::components::radio::Radio {
        selected,
        enabled: true,
    }
    .paint(
        cx.backend,
        Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(RADIO_SIZE, RADIO_SIZE),
        },
        &crate::widgets::button::tokens_from_theme(theme),
    );
}

#[allow(clippy::too_many_arguments)]
pub fn paint_stroke_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    labels: &PropertyLabels,
    stroke_variable_ref: Option<&str>,
    show_variable_button: bool,
    x: f32,
    y: f32,
    width: f32,
    mode: PaddingEditMode,
) -> f32 {
    let section_y = y;
    let mut y = paint_section_label(cx, theme, labels.stroke, x, y, width);
    draw_icon(
        cx.backend,
        Icon::Settings,
        // Centre the 14px glyph in its 18×18 gear action rect
        // (x+width-PAD_X-18, section_y+2 — see stroke_mode_popover_layout).
        Point2D::new(x + width - PAD_X - 16.0, section_y + 4.0),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    paint_stroke_main_row(
        cx,
        theme,
        snapshot,
        edit,
        stroke_variable_ref,
        show_variable_button,
        x,
        y,
        width,
    );
    paint_stroke_side_grid(
        cx,
        StrokeSideGridRefs {
            theme,
            snapshot,
            edit,
        },
        x,
        y,
        width,
        mode,
    );
    y += stroke_section_body_height(mode);
    paint_section_divider(cx, theme, x, y, width);
    y + SECTION_GAP
}

#[allow(clippy::too_many_arguments)]
fn paint_stroke_main_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    snapshot: &NodeSnapshot,
    edit: &EditContext<'_>,
    stroke_variable_ref: Option<&str>,
    show_variable_button: bool,
    x: f32,
    y: f32,
    width: f32,
) {
    let usable_w = width - PAD_X * 2.0;
    let stroke_color = snapshot.stroke_swatch_color();
    let variable_w = if show_variable_button {
        COLOR_VARIABLE_BUTTON_W + COLOR_VARIABLE_GAP
    } else {
        0.0
    };
    // The stroke WIDTH lives in the mode grid below (Single = 1 uniform,
    // Axis = 2, Individual = 4) — the main row carries only the colour, so
    // there is no duplicate width input. Let the hex fill the freed space.
    let hex_rect = Rect {
        origin: Point2D::new(x + PAD_X, y),
        size: Point2D::new(usable_w - variable_w, INPUT_HEIGHT),
    };
    let hex_focused = edit.focus == Some(PropertyFocus::StrokeHex);
    cx.backend
        .fill_round_rect(hex_rect, INPUT_RADIUS, theme.muted);
    if hex_focused && stroke_variable_ref.is_none() {
        cx.backend
            .stroke_round_rect(hex_rect, INPUT_RADIUS, theme.primary, 1.5);
    }
    cx.backend.fill_round_rect(
        Rect {
            origin: Point2D::new(hex_rect.origin.x + 6.0, hex_rect.origin.y + 7.0),
            size: Point2D::new(16.0, 16.0),
        },
        3.0,
        stroke_color,
    );
    paint_stroke_hex_text(cx, theme, edit, stroke_variable_ref, stroke_color, hex_rect);
    if show_variable_button {
        paint_color_variable_button(
            cx,
            theme,
            Rect {
                origin: Point2D::new(hex_rect.origin.x + hex_rect.size.x + COLOR_VARIABLE_GAP, y),
                size: Point2D::new(COLOR_VARIABLE_BUTTON_W, INPUT_HEIGHT),
            },
            stroke_variable_ref.is_some(),
        );
    }
}

fn paint_stroke_hex_text(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    edit: &EditContext<'_>,
    stroke_variable_ref: Option<&str>,
    stroke_color: crate::Color,
    hex_rect: Rect,
) {
    let hex_owned = format_color_hex(stroke_color);
    let variable_text = stroke_variable_ref.map(|name| format!("${name}"));
    let hex_text = variable_text
        .as_deref()
        .unwrap_or_else(|| edit.value_for(PropertyFocus::StrokeHex, &hex_owned));
    let hex_x = hex_rect.origin.x + 30.0;
    let painted_hex = stroke_variable_ref.is_none()
        && edit.paint_input_view_at(
            cx,
            theme,
            PropertyFocus::StrokeHex,
            Rect {
                origin: Point2D::new(hex_x, hex_rect.origin.y),
                size: Point2D::new(
                    (hex_rect.origin.x + hex_rect.size.x - 8.0 - hex_x).max(0.0),
                    hex_rect.size.y,
                ),
            },
            12.0,
            0.0,
            hex_rect.origin.y + 19.0,
        );
    if painted_hex {
        return;
    }
    let hex_layout = TextLayout::single_run(
        hex_text,
        "system-ui",
        12.0,
        (theme.foreground).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    if stroke_variable_ref.is_none() {
        edit.paint_selection_at(
            cx,
            theme,
            PropertyFocus::StrokeHex,
            hex_text,
            hex_x,
            hex_rect.origin.y + 19.0,
            12.0,
            hex_rect.origin.x + hex_rect.size.x - 8.0,
        );
    }
    cx.backend
        .draw_text(&hex_layout, Point2D::new(hex_x, hex_rect.origin.y + 19.0));
    if stroke_variable_ref.is_none() {
        if let Some(pos) = edit.caret_at(PropertyFocus::StrokeHex) {
            let w = cx
                .backend
                .measure_text(&hex_text[..pos.min(hex_text.len())], 12.0);
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(hex_x + w, hex_rect.origin.y + 6.0),
                    size: Point2D::new(1.5, hex_rect.size.y - 12.0),
                },
                theme.foreground,
            );
        }
    }
}

fn paint_stroke_side_grid(
    cx: &mut PaintCx<'_>,
    refs: StrokeSideGridRefs<'_, '_>,
    x: f32,
    row_y: f32,
    width: f32,
    mode: PaddingEditMode,
) {
    let widths = refs.snapshot.stroke_side_widths();
    for (focus, rect) in stroke_side_input_rects(x, row_y, width, mode) {
        let label = stroke_side_label(mode, focus);
        let value = match focus {
            PropertyFocus::StrokeTopWidth => widths[0],
            PropertyFocus::StrokeRightWidth => widths[1],
            PropertyFocus::StrokeBottomWidth => widths[2],
            PropertyFocus::StrokeLeftWidth => widths[3],
            _ => 0.0,
        };
        let value = format_panel_number(value);
        paint_input_with_prefix_focused_state(
            cx,
            refs.theme,
            rect,
            label,
            refs.edit.value_for(focus, &value),
            refs.edit.focus == Some(focus),
            refs.edit.caret_at(focus),
            refs.edit.select_all_at(focus),
            refs.edit.input_at(focus),
            refs.edit.now_ms,
        );
    }
}

fn stroke_side_label(mode: PaddingEditMode, focus: PropertyFocus) -> &'static str {
    match mode {
        PaddingEditMode::Single => "",
        PaddingEditMode::Axis => match focus {
            PropertyFocus::StrokeRightWidth | PropertyFocus::StrokeLeftWidth => "H",
            _ => "V",
        },
        PaddingEditMode::Individual => match focus {
            PropertyFocus::StrokeTopWidth => "T",
            PropertyFocus::StrokeRightWidth => "R",
            PropertyFocus::StrokeBottomWidth => "B",
            PropertyFocus::StrokeLeftWidth => "L",
            _ => "",
        },
    }
}
