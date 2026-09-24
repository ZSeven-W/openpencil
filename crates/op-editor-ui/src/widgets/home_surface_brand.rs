//! The composer's brand chip and the 加链接 tool's hints.
//!
//! While a brand is being read, and once a kit is staged, a chip sits at
//! the input box's bottom-right (attachments own the bottom-left): four
//! swatches of the kit, its name, and a remove (×) button. The tool's
//! transient hints (nothing to read / extraction failed) float above the
//! tool button like the "coming soon" tooltip does.
//!
//! Geometry is derived from the input box plus an ESTIMATED label width so
//! hit-testing (no backend) and paint agree without a measuring pass.

use super::copy::{self, SANS};
use super::{fade, HomeSurface, StudioPalette};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::{HomeBrandStatus, HomeHit};

const CHIP_H: f32 = 26.0;
const FONT: f32 = 11.0;
const DOT: f32 = 10.0;
const DOT_GAP: f32 = 3.0;
const PAD: f32 = 8.0;
const CLOSE: f32 = 16.0;
const LABEL_MAX_CHARS: usize = 28;

/// `(chip, close)` rects inside `input_box`, or `None` when no chip shows.
pub(crate) fn chip_rects(surface: &HomeSurface<'_>, input_box: Rect) -> Option<(Rect, Rect)> {
    if !surface.state.brand.chip_visible() || input_box.size.x <= 0.0 {
        return None;
    }
    let label = chip_label(surface);
    let dots = swatches(surface).len() as f32;
    let dots_w = if dots > 0.0 {
        dots * DOT + (dots - 1.0) * DOT_GAP + PAD
    } else {
        0.0
    };
    let w = PAD + dots_w + copy::estimate_text_w(&label, FONT) + PAD + CLOSE + 4.0;
    let w = w.min(input_box.size.x * 0.6);
    let chip = Rect::xywh(
        input_box.origin.x + input_box.size.x - 10.0 - w,
        input_box.origin.y + input_box.size.y - CHIP_H - 6.0,
        w,
        CHIP_H,
    );
    let close = Rect::xywh(
        chip.origin.x + chip.size.x - CLOSE - 5.0,
        chip.origin.y + (CHIP_H - CLOSE) / 2.0,
        CLOSE,
        CLOSE,
    );
    Some((chip, close))
}

fn chip_label(surface: &HomeSurface<'_>) -> String {
    let locale = surface.ui.locale;
    let text = match &surface.state.brand.status {
        HomeBrandStatus::Extracting { label } => {
            copy::home_str(locale, "home.brand.extracting").replace("{{name}}", label)
        }
        HomeBrandStatus::Ready(kit) => {
            copy::home_str(locale, "home.brand.chip").replace("{{name}}", &kit.label)
        }
        _ => String::new(),
    };
    if text.chars().count() > LABEL_MAX_CHARS {
        let mut cut: String = text.chars().take(LABEL_MAX_CHARS - 1).collect();
        cut.push('…');
        cut
    } else {
        text
    }
}

fn swatches(surface: &HomeSurface<'_>) -> Vec<Color> {
    surface
        .state
        .brand
        .staged()
        .map(|kit| {
            kit.swatches
                .iter()
                .filter_map(|h| hex_color(h))
                .take(4)
                .collect()
        })
        .unwrap_or_default()
}

fn hex_color(hex: &str) -> Option<Color> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 || !h.is_ascii() {
        return None;
    }
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some(Color::rgb_u8(c(0)?, c(2)?, c(4)?))
}

/// Paint the chip into the (entrance-shifted) input box.
pub(crate) fn paint_brand_chip(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    input_box: Rect,
    palette: StudioPalette,
) {
    let Some((chip, close)) = chip_rects(surface, input_box) else {
        return;
    };
    cx.backend.fill_round_rect(chip, 7.0, palette.chip_bg);
    cx.backend
        .stroke_round_rect(chip, 7.0, palette.chip_line, 1.0);
    let mut x = chip.origin.x + PAD;
    let dots = swatches(surface);
    for dot in &dots {
        let r = Rect::xywh(x, chip.origin.y + (CHIP_H - DOT) / 2.0, DOT, DOT);
        cx.backend.fill_round_rect(r, DOT / 2.0, *dot);
        cx.backend
            .stroke_round_rect(r, DOT / 2.0, fade(palette.ink, 0.18), 1.0);
        x += DOT + DOT_GAP;
    }
    if !dots.is_empty() {
        x += PAD - DOT_GAP;
    }
    let ink = if surface.state.brand.is_extracting() {
        palette.sub
    } else {
        fade(palette.ink, 0.8)
    };
    let label = chip_label(surface);
    let layout = crate::TextLayout::single_run(&label, SANS, FONT, ink.to_jian(), Point2D::ZERO);
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        x,
        chip.origin.y,
        (close.origin.x - 4.0 - x).max(0.0),
        CHIP_H,
    ));
    cx.backend.draw_text(
        &layout,
        Point2D::new(x, jian_widgets::centered_text_baseline_y(chip, FONT)),
    );
    cx.backend.restore();

    let hovered = surface.state.hover == Some(HomeHit::BrandClear);
    let cross = if hovered { palette.blue } else { palette.sub };
    if hovered {
        cx.backend
            .fill_round_rect(close, CLOSE / 2.0, palette.button_hover);
    }
    let (cx0, cy0) = (close.origin.x + CLOSE / 2.0, close.origin.y + CLOSE / 2.0);
    let arm = 3.5;
    cx.backend.stroke_line(
        Point2D::new(cx0 - arm, cy0 - arm),
        Point2D::new(cx0 + arm, cy0 + arm),
        cross,
        1.4,
    );
    cx.backend.stroke_line(
        Point2D::new(cx0 + arm, cy0 - arm),
        Point2D::new(cx0 - arm, cy0 + arm),
        cross,
        1.4,
    );
}

/// The i18n key of the hint to float over the 加链接 tool right now, if any:
/// a timed needs-source / failure line, else the hover explanation.
pub(crate) fn link_hint_key(surface: &HomeSurface<'_>) -> Option<&'static str> {
    let brand = &surface.state.brand;
    if !brand.available {
        return None;
    }
    if brand.hint_visible(surface.now_ms) {
        return Some(match brand.status {
            HomeBrandStatus::Failed { .. } => "home.brand.failed",
            _ => "home.brand.needSource",
        });
    }
    (surface.state.hover == Some(HomeHit::ReferenceLink)).then_some("home.tools.brandHint")
}

/// Paint the hint above the (shifted) 加链接 tool rect.
pub(crate) fn paint_link_hint(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    anchor: Rect,
    palette: StudioPalette,
) {
    let Some(key) = link_hint_key(surface) else {
        return;
    };
    let label = copy::home_str(surface.ui.locale, key);
    let w = cx.backend.measure_text_family(label, FONT, SANS) + 20.0;
    let tooltip = Rect::xywh(anchor.origin.x, anchor.origin.y - 28.0, w, 22.0);
    cx.backend
        .fill_round_rect(tooltip, 7.0, palette.tooltip_fill);
    let layout = crate::TextLayout::single_run(
        label,
        SANS,
        FONT,
        palette.tooltip_ink.to_jian(),
        Point2D::ZERO,
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            tooltip.origin.x + 10.0,
            jian_widgets::centered_text_baseline_y(tooltip, FONT),
        ),
    );
}

#[cfg(test)]
#[path = "home_surface_brand_tests.rs"]
mod tests;
