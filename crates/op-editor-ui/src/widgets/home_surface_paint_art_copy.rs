//! Localized copy for the App task's example art (the coffee-app phone
//! screens and the desktop counter window).
//!
//! The art was redrawn from a Chinese prototype and painted its strings
//! as literals, so every locale saw a Chinese mock-up. No shipped scene
//! template is an app-screen set (see `example_draft_template`), so there
//! is no real thumbnail to show instead; the mock-up stays, and its
//! strings come from the `home.art.*` i18n keys. The layout was tuned to
//! CJK widths, so every string that shares a row with something else is
//! painted through [`text_fit`], which shrinks it into the space the
//! prototype gave the Chinese copy instead of letting a longer
//! translation run over its neighbours.

use super::super::copy::SANS;
use crate::widgets::PaintCx;
use crate::{Color, Point2D};
use op_i18n::Locale;

/// Smallest size [`text_fit`] shrinks to, as a fraction of the design
/// size. Below this the mock-up stops reading as text at all, so the run
/// is clipped by its container instead.
const MIN_FIT_SCALE: f32 = 0.6;

/// The art string `key` in `locale`.
pub(super) fn art(locale: Locale, key: &'static str) -> &'static str {
    op_i18n::translate(locale, key)
}

/// Width of `content` at `size` / `weight` in the art's face.
pub(super) fn measure(cx: &mut PaintCx<'_>, content: &str, size: f32, weight: u16) -> f32 {
    cx.backend
        .measure_text_family_styled(content, size, SANS, weight, false)
}

/// The size `content` must be painted at to fit `max_w` — `size` when it
/// already fits, shrunk proportionally (never below [`MIN_FIT_SCALE`])
/// when it does not.
pub(super) fn fit_size(
    cx: &mut PaintCx<'_>,
    content: &str,
    size: f32,
    weight: u16,
    max_w: f32,
) -> f32 {
    let width = measure(cx, content, size, weight);
    if width <= max_w || width <= 0.0 {
        return size;
    }
    (size * max_w / width).max(size * MIN_FIT_SCALE)
}

/// Paint `content` with its baseline at `origin`, shrunk to fit `max_w`.
/// A run still too wide at the smallest size is clipped to `max_w` so it
/// never paints over its neighbour. Returns the painted width.
pub(super) fn text_fit(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    max_w: f32,
    color: Color,
    weight: u16,
) -> f32 {
    let size = fit_size(cx, content, size, weight, max_w);
    let width = measure(cx, content, size, weight);
    let clip = width > max_w + 0.5;
    if clip {
        cx.backend.save();
        cx.backend.clip_rect(crate::Rect::xywh(
            origin.x,
            origin.y - size * 1.2,
            max_w.max(0.0),
            size * 1.6,
        ));
    }
    let layout = crate::TextLayout::single_run(content, SANS, size, color.to_jian(), Point2D::ZERO)
        .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
    if clip {
        cx.backend.restore();
    }
    width.min(max_w)
}

/// [`text_fit`] centred on `centre_x`.
#[allow(clippy::too_many_arguments)]
pub(super) fn text_fit_centred(
    cx: &mut PaintCx<'_>,
    content: &str,
    centre_x: f32,
    baseline_y: f32,
    size: f32,
    max_w: f32,
    color: Color,
    weight: u16,
) {
    let size = fit_size(cx, content, size, weight, max_w);
    let width = measure(cx, content, size, weight).min(max_w);
    text_fit(
        cx,
        content,
        Point2D::new(centre_x - width / 2.0, baseline_y),
        size,
        max_w,
        color,
        weight,
    );
}

/// [`text_fit`] right-aligned to `right_x`.
#[allow(clippy::too_many_arguments)]
pub(super) fn text_fit_right(
    cx: &mut PaintCx<'_>,
    content: &str,
    right_x: f32,
    baseline_y: f32,
    size: f32,
    max_w: f32,
    color: Color,
    weight: u16,
) -> f32 {
    let size = fit_size(cx, content, size, weight, max_w);
    let width = measure(cx, content, size, weight).min(max_w);
    text_fit(
        cx,
        content,
        Point2D::new(right_x - width, baseline_y),
        size,
        max_w,
        color,
        weight,
    );
    width
}

/// A menu item's secondary Latin name, shown only when it adds something:
/// in a locale whose own name for the drink already IS the Latin name,
/// repeating it underneath would read as a typo.
pub(super) fn latin_subtitle(localized: &str, latin: &'static str) -> Option<&'static str> {
    (!localized.eq_ignore_ascii_case(latin)).then_some(latin)
}

#[cfg(test)]
#[path = "home_surface_paint_art_tests.rs"]
mod tests;
