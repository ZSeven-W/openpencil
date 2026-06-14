//! Colour-variable affordances for the property panel.
//!
//! Kept separate from the fill / stroke paint modules so the TS-parity
//! variable picker can be shared without growing the already-large
//! section files.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{ColorVariableOption, EffectSummary, PropertyPanelAction};
use crate::widgets::property_panel_inputs::{INPUT_RADIUS, PAD_X};
use crate::widgets::property_panel_layout::{
    action_button_rects_with_fill_picker, COLOR_VARIABLE_MENU_PAD_Y, COLOR_VARIABLE_MENU_ROW_H,
    COLOR_VARIABLE_MENU_W,
};
use crate::widgets::property_panel_snapshot::color_from_hex;
use crate::widgets::property_panel_visibility::VisibleSections;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

pub fn paint_color_variable_button(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, active: bool) {
    if active {
        cx.backend
            .fill_round_rect(rect, INPUT_RADIUS, theme.row_selected_primary);
    }
    draw_icon(
        cx.backend,
        Icon::Braces,
        Point2D::new(rect.origin.x + 6.0, rect.origin.y + 7.0),
        16.0,
        if active {
            theme.primary
        } else {
            theme.muted_foreground
        },
        1.6,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn paint_color_variable_picker(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
    variables: &[ColorVariableOption],
    fill_ref: Option<&str>,
    stroke_ref: Option<&str>,
    target: op_editor_core::ColorTarget,
    locale: op_editor_core::Locale,
    fill_picker_open: bool,
    font_picker_open: bool,
    font_weight_picker_open: bool,
    export_scale_picker_open: bool,
    export_format_picker_open: bool,
    padding_mode_popover_open: bool,
) {
    let Some(anchor) = color_variable_anchor(
        panel_rect,
        visible,
        effects,
        target,
        fill_picker_open,
        font_picker_open,
        font_weight_picker_open,
        export_scale_picker_open,
        export_format_picker_open,
        padding_mode_popover_open,
    ) else {
        return;
    };
    let current = match target {
        op_editor_core::ColorTarget::Fill => fill_ref,
        op_editor_core::ColorTarget::Stroke => stroke_ref,
        _ => None,
    };
    let bound = current.is_some();
    let row_count = variables.len() + usize::from(bound);
    if row_count == 0 {
        return;
    }
    let menu_x =
        (anchor.origin.x + anchor.size.x - COLOR_VARIABLE_MENU_W).max(panel_rect.origin.x + PAD_X);
    let menu_rect = Rect {
        origin: Point2D::new(menu_x, anchor.origin.y + anchor.size.y + 4.0),
        size: Point2D::new(
            COLOR_VARIABLE_MENU_W,
            COLOR_VARIABLE_MENU_PAD_Y * 2.0 + row_count as f32 * COLOR_VARIABLE_MENU_ROW_H,
        ),
    };
    cx.backend
        .fill_round_rect(menu_rect, INPUT_RADIUS, theme.popover);
    cx.backend
        .stroke_round_rect(menu_rect, INPUT_RADIUS, theme.border, 1.0);

    let mut row_y = menu_rect.origin.y + COLOR_VARIABLE_MENU_PAD_Y;
    if bound {
        paint_unbind_row(cx, theme, locale, menu_rect.origin.x, row_y);
        row_y += COLOR_VARIABLE_MENU_ROW_H;
    }
    for variable in variables {
        paint_variable_row(
            cx,
            theme,
            variable,
            current == Some(variable.name.as_str()),
            menu_rect.origin.x,
            row_y,
        );
        row_y += COLOR_VARIABLE_MENU_ROW_H;
    }
}

#[allow(clippy::too_many_arguments)]
fn color_variable_anchor(
    panel_rect: Rect,
    visible: VisibleSections,
    effects: &[EffectSummary],
    target: op_editor_core::ColorTarget,
    fill_picker_open: bool,
    font_picker_open: bool,
    font_weight_picker_open: bool,
    export_scale_picker_open: bool,
    export_format_picker_open: bool,
    padding_mode_popover_open: bool,
) -> Option<Rect> {
    action_button_rects_with_fill_picker(
        panel_rect,
        visible,
        effects,
        fill_picker_open,
        font_picker_open,
        font_weight_picker_open,
        export_scale_picker_open,
        export_format_picker_open,
        padding_mode_popover_open,
    )
    .into_iter()
    .find_map(|(action, rect)| match action {
        PropertyPanelAction::ToggleColorVariablePicker(t) if t == target => Some(rect),
        _ => None,
    })
}

fn paint_unbind_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    locale: op_editor_core::Locale,
    x: f32,
    y: f32,
) {
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(x + 12.0, y + 8.0),
        16.0,
        theme.destructive,
        1.6,
    );
    let text = op_i18n::translate(locale, "variablePicker.unbind");
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        12.0,
        (theme.destructive).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&layout, Point2D::new(x + 36.0, y + 21.0));
}

fn paint_variable_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    variable: &ColorVariableOption,
    active: bool,
    x: f32,
    y: f32,
) {
    let row_rect = Rect {
        origin: Point2D::new(x + 4.0, y),
        size: Point2D::new(COLOR_VARIABLE_MENU_W - 8.0, COLOR_VARIABLE_MENU_ROW_H),
    };
    if active {
        cx.backend
            .fill_round_rect(row_rect, 6.0, theme.row_selected_primary);
    }
    let swatch = Rect {
        origin: Point2D::new(row_rect.origin.x + 8.0, y + 8.0),
        size: Point2D::new(16.0, 16.0),
    };
    let color = variable
        .resolved_hex
        .as_deref()
        .and_then(color_from_hex)
        .unwrap_or(Color::BLACK);
    cx.backend.fill_round_rect(swatch, 4.0, color);
    cx.backend.stroke_round_rect(swatch, 4.0, theme.border, 1.0);

    let name = format!("--{}", variable.name);
    let name_layout = TextLayout::single_run(
        &name,
        "ui-monospace",
        12.0,
        (if active {
            theme.primary
        } else {
            theme.foreground
        })
        .to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &name_layout,
        Point2D::new(row_rect.origin.x + 34.0, y + 21.0),
    );

    if let Some(hex) = variable.resolved_hex.as_deref() {
        let hex_layout = TextLayout::single_run(
            hex,
            "ui-monospace",
            12.0,
            (theme.muted_foreground).to_jian(),
            Point2D::new(0.0, 0.0),
        );
        let hex_w = cx.backend.measure_text(hex, 12.0);
        cx.backend.draw_text(
            &hex_layout,
            Point2D::new(row_rect.origin.x + row_rect.size.x - hex_w - 10.0, y + 21.0),
        );
    }
}
