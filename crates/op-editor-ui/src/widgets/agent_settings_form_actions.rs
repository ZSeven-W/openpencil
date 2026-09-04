//! Shared Save / Cancel row for settings draft forms.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::button::{paint_ghost_button_feedback, tokens_from_theme};
use crate::widgets::text_metrics;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use jian_widgets::components::button::{Button, ButtonVariant};
use op_editor_core::editor_ui_state::EditorUiState;

const FORM_BTN_W: f32 = 68.0;
const FORM_BTN_H: f32 = 26.0;
const TOUCH_FORM_BTN_W: f32 = 92.0;
const TOUCH_FORM_BTN_H: f32 = 44.0;

#[allow(clippy::too_many_arguments)]
pub fn paint_form_actions(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    card: Rect,
    form_h: f32,
    can_save: bool,
    cancel_pressed: bool,
    save_pressed: bool,
) {
    paint_form_actions_for_ui(
        cx,
        theme,
        ui,
        card,
        form_h,
        can_save,
        cancel_pressed,
        save_pressed,
        false,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn paint_form_actions_for_ui(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    card: Rect,
    form_h: f32,
    can_save: bool,
    cancel_pressed: bool,
    save_pressed: bool,
    touch: bool,
) {
    paint_text_button(
        cx,
        theme,
        cancel_button_rect_for_ui(card, form_h, touch),
        t_settings(ui, "common.cancel"),
        ButtonVariant::Outline,
        true,
        cancel_pressed,
        touch,
    );
    paint_text_button(
        cx,
        theme,
        save_button_rect_for_ui(card, form_h, touch),
        t_settings(ui, "common.save"),
        ButtonVariant::Primary,
        can_save,
        save_pressed,
        touch,
    );
}

pub fn save_button_rect(card: Rect, form_h: f32) -> Rect {
    save_button_rect_for_ui(card, form_h, false)
}

pub fn save_button_rect_for_ui(card: Rect, form_h: f32, touch: bool) -> Rect {
    let (width, height, bottom_pad) = if touch {
        (TOUCH_FORM_BTN_W, TOUCH_FORM_BTN_H, 8.0)
    } else {
        (FORM_BTN_W, FORM_BTN_H, 5.0)
    };
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - if touch { 16.0 } else { 12.0 } - width,
            card.origin.y + form_h + bottom_pad,
        ),
        size: Point2D::new(width, height),
    }
}

pub fn cancel_button_rect(card: Rect, form_h: f32) -> Rect {
    cancel_button_rect_for_ui(card, form_h, false)
}

pub fn cancel_button_rect_for_ui(card: Rect, form_h: f32, touch: bool) -> Rect {
    let width = if touch { TOUCH_FORM_BTN_W } else { FORM_BTN_W };
    Rect {
        origin: Point2D::new(
            save_button_rect_for_ui(card, form_h, touch).origin.x
                - if touch { 12.0 } else { 8.0 }
                - width,
            save_button_rect_for_ui(card, form_h, touch).origin.y,
        ),
        size: Point2D::new(width, if touch { TOUCH_FORM_BTN_H } else { FORM_BTN_H }),
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_text_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    label: &str,
    variant: ButtonVariant,
    enabled: bool,
    pressed: bool,
    touch: bool,
) {
    let is_primary = matches!(variant, ButtonVariant::Primary);
    // A disabled primary CTA renders as a neutral Outline (border + no fill →
    // white in light mode) rather than a washed-out pale-primary fill.
    let effective_variant = if is_primary && !enabled {
        ButtonVariant::Outline
    } else {
        variant
    };
    Button {
        label: "",
        icon_paths: None,
        variant: effective_variant,
        enabled,
        hovered: !is_primary,
        pressed,
        font_size: if touch { 15.0 } else { 11.0 },
    }
    .paint(cx.backend, rect, &tokens_from_theme(theme));
    if is_primary && enabled && pressed {
        paint_ghost_button_feedback(cx.backend, theme, rect, false, true);
    }
    let fg = match (is_primary, enabled) {
        (true, true) => theme.primary_foreground,
        (_, true) => theme.foreground,
        _ => theme.muted_foreground,
    };
    let font_size = if touch { 15.0 } else { 11.0 };
    let tw = text_metrics::measure_chrome(cx.backend, label, font_size);
    draw_text(
        cx,
        label,
        font_size,
        fg,
        rect.origin.x + (rect.size.x - tw) / 2.0,
        if touch {
            jian_widgets::centered_text_baseline_y(rect, font_size)
        } else {
            rect.origin.y + 17.0
        },
    );
}

fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y));
}
