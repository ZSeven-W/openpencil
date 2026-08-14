//! Compact Effects rows + the Effects add-menu.

use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::property_panel::{EffectKind, EffectSummary, PropertyPanelAction};
use crate::widgets::property_panel_inputs::{
    paint_section_divider, paint_section_label_with_add, INPUT_RADIUS, PAD_X, SECTION_GAP,
};
use crate::widgets::property_panel_sections::{EditContext, PropertyLabels};
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect, TextLayout};
use op_editor_core::PropertyFocus;

pub const EFFECT_ROW_HEIGHT: f32 = 36.0;
pub const EFFECT_ROW_GAP: f32 = 6.0;
const TOUCH_EFFECT_ACTION_SIZE: f32 = 30.0;

#[derive(Debug, Clone, Copy)]
pub struct EffectRowRects {
    pub row: Rect,
    pub slider: Rect,
    pub value: Rect,
    pub eye: Rect,
    pub remove: Rect,
}

pub fn effect_row_rects(x: f32, y: f32, width: f32, touch_controls: bool) -> EffectRowRects {
    let row = Rect::xywh(x + PAD_X, y, width - PAD_X * 2.0, EFFECT_ROW_HEIGHT);
    let action_size = if touch_controls {
        TOUCH_EFFECT_ACTION_SIZE
    } else {
        24.0
    };
    let action_y = y + (EFFECT_ROW_HEIGHT - action_size) / 2.0;
    let remove = Rect::xywh(
        row.origin.x + row.size.x - action_size - 2.0,
        action_y,
        action_size,
        action_size,
    );
    let eye = Rect::xywh(
        remove.origin.x - action_size - 2.0,
        action_y,
        action_size,
        action_size,
    );
    let value = Rect::xywh(eye.origin.x - 42.0, y + 4.0, 38.0, 28.0);
    let slider_h = if touch_controls { 30.0 } else { 20.0 };
    let slider = Rect::xywh(
        value.origin.x - 58.0,
        y + (EFFECT_ROW_HEIGHT - slider_h) / 2.0,
        54.0,
        slider_h,
    );
    EffectRowRects {
        row,
        slider,
        value,
        eye,
        remove,
    }
}

pub fn slider_value(rect: Rect, x: f32) -> f32 {
    (((x - rect.origin.x) / rect.size.x).clamp(0.0, 1.0) * 100.0).round()
}

#[allow(clippy::too_many_arguments)]
pub fn paint_effects_section(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    effects: &[EffectSummary],
    edit: &EditContext<'_>,
    touch_controls: bool,
    x: f32,
    y: f32,
    width: f32,
) -> f32 {
    let mut row_y = paint_section_label_with_add(cx, theme, labels.effects, x, y, width);
    if effects.is_empty() {
        row_y += 8.0;
    } else {
        for (index, effect) in effects.iter().enumerate() {
            paint_effect_row(
                cx,
                theme,
                labels,
                effect,
                index,
                edit,
                touch_controls,
                x,
                row_y,
                width,
            );
            row_y += EFFECT_ROW_HEIGHT + EFFECT_ROW_GAP;
        }
    }
    paint_section_divider(cx, theme, x, row_y, width);
    row_y + SECTION_GAP
}

pub(crate) const EFFECT_ADD_MENU_ROWS: [(PropertyPanelAction, EffectKind); 3] = [
    (
        PropertyPanelAction::AddEffect(EffectKind::Shadow),
        EffectKind::Shadow,
    ),
    (
        PropertyPanelAction::AddEffect(EffectKind::LayerBlur),
        EffectKind::LayerBlur,
    ),
    (
        PropertyPanelAction::AddEffect(EffectKind::BackgroundBlur),
        EffectKind::BackgroundBlur,
    ),
];

pub(crate) const EFFECT_ADD_MENU_ROW_H: f32 = 30.0;
pub(crate) const EFFECT_ADD_MENU_W: f32 = 160.0;

pub(crate) fn effect_add_menu_rect(add_rect: Rect) -> Rect {
    let h = EFFECT_ADD_MENU_ROWS.len() as f32 * EFFECT_ADD_MENU_ROW_H + 8.0;
    let right = add_rect.origin.x + add_rect.size.x;
    Rect::xywh(
        right - EFFECT_ADD_MENU_W,
        add_rect.origin.y + add_rect.size.y,
        EFFECT_ADD_MENU_W,
        h,
    )
}

pub(crate) fn effect_add_menu_row_rects(menu: Rect) -> Vec<(PropertyPanelAction, Rect)> {
    EFFECT_ADD_MENU_ROWS
        .iter()
        .enumerate()
        .map(|(index, (action, _))| {
            (
                action.clone(),
                Rect::xywh(
                    menu.origin.x,
                    menu.origin.y + 4.0 + index as f32 * EFFECT_ADD_MENU_ROW_H,
                    menu.size.x,
                    EFFECT_ADD_MENU_ROW_H,
                ),
            )
        })
        .collect()
}

pub(crate) fn paint_effect_add_menu(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    add_rect: Rect,
    hover: Option<usize>,
) {
    let menu = effect_add_menu_rect(add_rect);
    cx.backend
        .fill_round_rect(menu, INPUT_RADIUS, theme.popover);
    cx.backend
        .stroke_round_rect(menu, INPUT_RADIUS, theme.border, 1.0);
    for (index, (_, kind)) in EFFECT_ADD_MENU_ROWS.iter().enumerate() {
        let row = Rect::xywh(
            menu.origin.x + 4.0,
            menu.origin.y + 4.0 + index as f32 * EFFECT_ADD_MENU_ROW_H,
            menu.size.x - 8.0,
            EFFECT_ADD_MENU_ROW_H,
        );
        if hover == Some(index) {
            cx.backend.fill_round_rect(row, 6.0, theme.muted);
        }
        let label = effect_label(labels, *kind);
        let text = TextLayout::single_run(
            label,
            "system-ui",
            12.0,
            theme.foreground.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &text,
            Point2D::new(row.origin.x + 10.0, row.origin.y + 19.0),
        );
    }
}

fn effect_label(labels: &PropertyLabels, kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::Shadow => labels.effects_add_shadow,
        EffectKind::LayerBlur => labels.effects_add_layer_blur,
        EffectKind::BackgroundBlur => labels.effects_add_background_blur,
    }
}

fn effect_icon(kind: EffectKind) -> Icon {
    match kind {
        EffectKind::Shadow => Icon::Square,
        EffectKind::LayerBlur => Icon::Focus,
        EffectKind::BackgroundBlur => Icon::SquareRoundCorner,
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_effect_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    labels: &PropertyLabels,
    effect: &EffectSummary,
    index: usize,
    edit: &EditContext<'_>,
    touch_controls: bool,
    x: f32,
    y: f32,
    width: f32,
) {
    let rects = effect_row_rects(x, y, width, touch_controls);
    cx.backend.fill_round_rect(rects.row, 8.0, theme.muted);
    cx.backend
        .stroke_round_rect(rects.row, 8.0, theme.border, 1.0);
    let foreground = if effect.visible {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    draw_icon(
        cx.backend,
        effect_icon(effect.kind),
        Point2D::new(rects.row.origin.x + 7.0, y + 10.0),
        16.0,
        foreground,
        1.3,
    );
    let label = effect_label(labels, effect.kind);
    let max_label_w = (rects.slider.origin.x - (rects.row.origin.x + 27.0) - 4.0).max(1.0);
    let measured = text_metrics::measure_chrome(cx.backend, label, 10.0).max(1.0);
    let label_size = (10.0 * max_label_w / measured).clamp(8.0, 10.0);
    let label_layout = TextLayout::single_run(
        label,
        "system-ui",
        label_size,
        foreground.to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &label_layout,
        Point2D::new(rects.row.origin.x + 27.0, y + 22.0),
    );

    let value = effect.blur.clamp(0.0, 100.0);
    let track = Rect::xywh(
        rects.slider.origin.x,
        rects.row.origin.y + (rects.row.size.y - 4.0) / 2.0,
        rects.slider.size.x,
        4.0,
    );
    cx.backend.fill_round_rect(track, 2.0, theme.border);
    cx.backend.fill_round_rect(
        Rect::xywh(
            track.origin.x,
            track.origin.y,
            track.size.x * value / 100.0,
            4.0,
        ),
        2.0,
        theme.primary,
    );
    cx.backend.fill_oval(
        Rect::xywh(
            track.origin.x + track.size.x * value / 100.0 - 4.0,
            track.origin.y - 2.0,
            8.0,
            8.0,
        ),
        theme.primary_foreground,
    );

    let focus = PropertyFocus::EffectRadius(index);
    cx.backend
        .fill_round_rect(rects.value, 6.0, theme.background);
    cx.backend.stroke_round_rect(
        rects.value,
        6.0,
        if edit.focus == Some(focus) {
            theme.ring
        } else {
            theme.border
        },
        if edit.focus == Some(focus) { 1.5 } else { 1.0 },
    );
    let fallback = format!("{}", value.round() as i32);
    let text = edit.value_for(focus, &fallback);
    if !edit.paint_input_view_at(
        cx,
        theme,
        focus,
        rects.value,
        11.0,
        7.0,
        rects.value.origin.y + 18.0,
    ) {
        let layout = TextLayout::single_run(
            text,
            "system-ui",
            11.0,
            foreground.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(rects.value.origin.x + 7.0, rects.value.origin.y + 18.0),
        );
    }

    draw_icon(
        cx.backend,
        if effect.visible {
            Icon::Eye
        } else {
            Icon::EyeOff
        },
        Point2D::new(
            rects.eye.origin.x + (rects.eye.size.x - 16.0) / 2.0,
            rects.eye.origin.y + (rects.eye.size.y - 16.0) / 2.0,
        ),
        16.0,
        foreground,
        1.3,
    );
    draw_icon(
        cx.backend,
        Icon::Close,
        Point2D::new(
            rects.remove.origin.x + (rects.remove.size.x - 14.0) / 2.0,
            rects.remove.origin.y + (rects.remove.size.y - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.3,
    );
}
