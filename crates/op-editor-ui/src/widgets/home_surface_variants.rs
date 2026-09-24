//! The composer's 3-directions toggle: a chip in the tools row that
//! makes the next send generate several distinct design directions side
//! by side instead of one design.

use super::copy::{self, SANS};
use super::{HomeSurface, StudioPalette};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::HomeHit;

/// Horizontal room around the label: icon (14) + gaps + side padding.
pub(crate) const VARIANTS_CHIP_PAD: f32 = 44.0;
const ICON_SIZE: f32 = 14.0;
const FONT: f32 = 13.0;
const ICON_GAP: f32 = 6.0;

/// Paint the toggle into `rect` (a zero rect paints nothing). On, it reads
/// as a selected chip (soft blue fill, blue ink); off, as an outlined one.
pub(crate) fn paint_variants_toggle(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    palette: StudioPalette,
) {
    if rect.size.x <= 0.0 {
        return;
    }
    let on = surface.state.variants_on;
    let hovered = surface.state.hover == Some(HomeHit::Variants);
    let radius = rect.size.y / 2.0;
    let (fill, line, ink) = if on {
        (palette.blue_soft, palette.blue, palette.blue)
    } else if hovered {
        (palette.button_hover, palette.button_hover_line, palette.ink)
    } else {
        (palette.chip_bg, palette.chip_line, palette.sub)
    };
    cx.backend.fill_round_rect(rect, radius, fill);
    cx.backend.stroke_round_rect(rect, radius, line, 1.0);
    // The rect is sized from an estimate; centre the measured content in it.
    let label = copy::home_str(surface.ui.locale, "home.tools.variants");
    let label_w = cx.backend.measure_text_family(label, FONT, SANS);
    let content_w = ICON_SIZE + ICON_GAP + label_w;
    let left = rect.origin.x + ((rect.size.x - content_w) / 2.0).max(10.0);
    draw_icon(
        cx.backend,
        Icon::LayoutGrid,
        Point2D::new(left, rect.origin.y + (rect.size.y - ICON_SIZE) / 2.0),
        ICON_SIZE,
        ink,
        1.6,
    );
    let layout = crate::TextLayout::single_run(label, SANS, FONT, ink.to_jian(), Point2D::ZERO);
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            left + ICON_SIZE + ICON_GAP,
            jian_widgets::centered_text_baseline_y(rect, FONT),
        ),
    );
}
