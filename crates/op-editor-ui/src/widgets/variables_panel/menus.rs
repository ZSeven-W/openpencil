//! VariablesPanel dropdown / popover painting (row overflow menu,
//! preset / add-variable / theme / variant menus, axis dropdown) —
//! split from `paint.rs` to honor the 800-line cap.

use super::paint::paint_text;
use super::*;
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Point2D, Rect};
use op_editor_core::variables_panel_state::PresetMenuButton;
use op_editor_core::VariablesPanelButton;

fn paint_feedback(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    target: VariablesPanelButton,
    rect: Rect,
    radius: f32,
) {
    let hovered = panel.hover == Some(target);
    let pressed = panel.pressed == Some(target);
    if hovered || pressed {
        paint_button_feedback_wash(cx.backend, &panel.theme, rect, radius, hovered, pressed);
    }
}

fn preset_menu_row(button: PresetMenuButton) -> usize {
    match button {
        PresetMenuButton::SaveCurrent | PresetMenuButton::NameInput => 0,
        PresetMenuButton::Load(idx) | PresetMenuButton::Delete(idx) => 1 + idx,
        PresetMenuButton::Import => 2,
        PresetMenuButton::Export => 3,
    }
}

pub(super) fn paint_menus(
    panel: &VariablesPanel,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    labels: &VariablePanelLabels,
) {
    let theme = panel.theme;
    // Row `⋯` overflow menu — Rename + a destructive Delete (TS
    // `variable-row.tsx:183-211`). Painted manually instead of via
    // `paint_popover_rows` for the destructive tint on Delete.
    if let Some((_, menu)) = panel.row_menu_rect(rect) {
        cx.backend.fill_round_rect(menu, 12.0, theme.popover);
        cx.backend.stroke_round_rect(menu, 12.0, theme.border, 1.0);
        let rows = [
            (labels.rename, Icon::Pencil, theme.popover_foreground),
            (labels.delete, Icon::Trash, theme.destructive),
        ];
        for (idx, (label, icon, color)) in rows.iter().enumerate() {
            let row_y = menu.origin.y + idx as f32 * ADD_VARIABLE_MENU_ROW_HEIGHT;
            paint_feedback(
                panel,
                cx,
                VariablesPanelButton::RowMenuItem(idx),
                Rect {
                    origin: Point2D::new(menu.origin.x + 4.0, row_y + 3.0),
                    size: Point2D::new(menu.size.x - 8.0, ADD_VARIABLE_MENU_ROW_HEIGHT - 6.0),
                },
                8.0,
            );
            draw_icon(
                cx.backend,
                *icon,
                Point2D::new(menu.origin.x + 12.0, row_y + 8.5),
                13.0,
                *color,
                1.6,
            );
            paint_text(cx, label, 12.0, *color, menu.origin.x + 33.0, row_y + 20.0);
        }
    }
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
            match panel.hover {
                Some(VariablesPanelButton::PresetMenuItem(button)) => Some(preset_menu_row(button)),
                _ => None,
            },
            match panel.pressed {
                Some(VariablesPanelButton::PresetMenuItem(button)) => Some(preset_menu_row(button)),
                _ => None,
            },
        );
    }
    if panel.add_menu_open {
        paint_popover_rows(
            cx,
            theme,
            add_variable_menu_rect(rect),
            &[labels.color, labels.number, labels.string],
            match panel.hover {
                Some(VariablesPanelButton::AddVariableMenuItem(idx)) => Some(idx),
                _ => None,
            },
            match panel.pressed {
                Some(VariablesPanelButton::AddVariableMenuItem(idx)) => Some(idx),
                _ => None,
            },
        );
    }
    if let Some(axis) = panel.theme_menu_open.as_deref() {
        let mut rows = vec![labels.rename];
        if panel.theme_tab_labels().len() > 1 {
            rows.push(labels.delete);
        }
        paint_popover_rows(
            cx,
            theme,
            panel.theme_menu_rect(rect, axis),
            &rows,
            match panel.hover {
                Some(VariablesPanelButton::ThemeMenuItem(idx)) => Some(idx),
                _ => None,
            },
            match panel.pressed {
                Some(VariablesPanelButton::ThemeMenuItem(idx)) => Some(idx),
                _ => None,
            },
        );
    }
    if let Some(value) = panel.variant_menu_open.as_deref() {
        let mut rows = vec![labels.rename];
        if panel.variant_column_labels().len() > 1 {
            rows.push(labels.delete);
        }
        paint_popover_rows(
            cx,
            theme,
            panel.variant_menu_rect(rect, value),
            &rows,
            match panel.hover {
                Some(VariablesPanelButton::VariantMenuItem(idx)) => Some(idx),
                _ => None,
            },
            match panel.pressed {
                Some(VariablesPanelButton::VariantMenuItem(idx)) => Some(idx),
                _ => None,
            },
        );
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
        paint_feedback(
            panel,
            cx,
            VariablesPanelButton::DropdownItem(i),
            Rect {
                origin: Point2D::new(menu_rect.origin.x + 4.0, row_y + 3.0),
                size: Point2D::new(menu_rect.size.x - 8.0, DROPDOWN_ROW_HEIGHT - 6.0),
            },
            8.0,
        );
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

fn paint_popover_rows(
    cx: &mut PaintCx<'_>,
    theme: Theme,
    rect: Rect,
    rows: &[&str],
    hover_row: Option<usize>,
    pressed_row: Option<usize>,
) {
    cx.backend.fill_round_rect(rect, 12.0, theme.popover);
    cx.backend.stroke_round_rect(rect, 12.0, theme.border, 1.0);
    for (idx, label) in rows.iter().enumerate() {
        let row_y = rect.origin.y + idx as f32 * ADD_VARIABLE_MENU_ROW_HEIGHT;
        let hovered = hover_row == Some(idx);
        let pressed = pressed_row == Some(idx);
        if hovered || pressed {
            paint_button_feedback_wash(
                cx.backend,
                &theme,
                Rect {
                    origin: Point2D::new(rect.origin.x + 4.0, row_y + 3.0),
                    size: Point2D::new(rect.size.x - 8.0, ADD_VARIABLE_MENU_ROW_HEIGHT - 6.0),
                },
                8.0,
                hovered,
                pressed,
            );
        }
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
