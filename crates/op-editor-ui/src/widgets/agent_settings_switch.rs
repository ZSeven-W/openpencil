use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

pub(super) const SETTINGS_SWITCH_W: f32 = 36.0;
pub(super) const SETTINGS_SWITCH_H: f32 = 20.0;
const SETTINGS_SWITCH_THUMB: f32 = 16.0;
const SETTINGS_SWITCH_INSET: f32 = 2.0;

/// Paint the settings switch with the same geometry as the TS shadcn
/// Switch (`w-9 h-5`, `h-4 w-4` thumb).
pub(super) fn paint_settings_switch(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    enabled: bool,
) {
    let track = if enabled { theme.primary } else { theme.border };
    cx.backend
        .fill_round_rect(rect, SETTINGS_SWITCH_H / 2.0, track);

    let thumb_x = if enabled {
        rect.origin.x + rect.size.x - SETTINGS_SWITCH_THUMB - SETTINGS_SWITCH_INSET
    } else {
        rect.origin.x + SETTINGS_SWITCH_INSET
    };
    let thumb = Rect {
        origin: Point2D::new(thumb_x, rect.origin.y + SETTINGS_SWITCH_INSET),
        size: Point2D::new(SETTINGS_SWITCH_THUMB, SETTINGS_SWITCH_THUMB),
    };
    cx.backend.fill_oval(thumb, theme.primary_foreground);
}
