use crate::theme::Theme;
use crate::widgets::button::tokens_from_theme;
use crate::widgets::PaintCx;
use crate::Rect;
use jian_widgets::components::switch::Switch;

pub(super) const SETTINGS_SWITCH_W: f32 = 36.0;
pub(super) const SETTINGS_SWITCH_H: f32 = 20.0;

/// Paint the settings switch via the canonical jian `Switch` (shadcn
/// `w-9 h-5`, `h-4 w-4` thumb). `enabled` here is the switch's ON state — the
/// control is always interactive, so jian's `enabled` is fixed `true`. Off-track
/// reads the `input` token (shadcn), the thumb white, matching the hand-rolled
/// geometry it replaces (knob 16, inset 2).
pub(super) fn paint_settings_switch(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    enabled: bool,
) {
    Switch {
        on: enabled,
        enabled: true,
        hovered: false,
        pressed: false,
    }
    .paint(cx.backend, rect, &tokens_from_theme(theme));
}
