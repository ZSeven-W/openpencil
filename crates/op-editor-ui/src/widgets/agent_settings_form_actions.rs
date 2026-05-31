//! Shared Save / Cancel row for settings draft forms.

use crate::theme::Theme;
use crate::widgets::agent_settings_i18n::t as t_settings;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::editor_ui_state::EditorUiState;

const FORM_BTN_W: f32 = 68.0;
const FORM_BTN_H: f32 = 26.0;

pub fn paint_form_actions(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    ui: &EditorUiState,
    card: Rect,
    form_h: f32,
    can_save: bool,
) {
    paint_text_button(
        cx,
        theme,
        cancel_button_rect(card, form_h),
        t_settings(ui, "common.cancel"),
        false,
        true,
    );
    paint_text_button(
        cx,
        theme,
        save_button_rect(card, form_h),
        t_settings(ui, "common.save"),
        true,
        can_save,
    );
}

pub fn save_button_rect(card: Rect, form_h: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            card.origin.x + card.size.x - 12.0 - FORM_BTN_W,
            card.origin.y + form_h + 5.0,
        ),
        size: Point2D::new(FORM_BTN_W, FORM_BTN_H),
    }
}

pub fn cancel_button_rect(card: Rect, form_h: f32) -> Rect {
    Rect {
        origin: Point2D::new(
            save_button_rect(card, form_h).origin.x - 8.0 - FORM_BTN_W,
            card.origin.y + form_h + 5.0,
        ),
        size: Point2D::new(FORM_BTN_W, FORM_BTN_H),
    }
}

fn paint_text_button(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    label: &str,
    primary: bool,
    enabled: bool,
) {
    let bg = if primary && enabled {
        theme.primary
    } else {
        theme.button_hover
    };
    let fg = if primary && enabled {
        theme.primary_foreground
    } else if enabled {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    cx.backend.fill_round_rect(rect, 6.0, bg);
    cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
    let tw = cx.backend.measure_text(label, 11.0);
    draw_text(
        cx,
        label,
        11.0,
        fg,
        rect.origin.x + (rect.size.x - tw) / 2.0,
        rect.origin.y + 17.0,
    );
}

fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        to_jian(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, y));
}

fn to_jian(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}
