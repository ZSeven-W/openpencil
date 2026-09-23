//! The Studio Home surface's two lower sections: the 看看还能做什么
//! explore cards and the 最近项目 row.
//!
//! Sibling of `home_surface_paint.rs`, split out at the 800-line cap the
//! collab security-boundary check enforces over this crate.

use super::super::copy::{self, SANS};
use super::super::paint::cards::{paint_explore_card, text, text_weighted};
use super::super::palette::fade;
use super::super::{HomeLayout, HomeSurface, StudioPalette, EXPLORE_FAMILIES};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::HomeHit;

pub(super) fn paint_explore(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    (dy, alpha): (f32, f32),
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let heading = Rect::xywh(
        layout.explore_heading.origin.x,
        layout.explore_heading.origin.y + dy,
        layout.explore_heading.size.x,
        layout.explore_heading.size.y,
    );
    text_weighted(
        cx,
        copy::home_str(locale, "home.explore.title"),
        Point2D::new(heading.origin.x, heading.origin.y + 17.0),
        17.0,
        fade(palette.ink, alpha),
        650,
    );
    // The blue ink underline, slightly rotated.
    let title_w =
        cx.backend
            .measure_text_family(copy::home_str(locale, "home.explore.title"), 17.0, SANS);
    let underline = Rect::xywh(
        heading.origin.x,
        heading.origin.y + 21.0,
        title_w * 0.92,
        3.0,
    );
    cx.backend.save();
    cx.backend.rotate(
        -2.0_f32.to_radians(),
        Point2D::new(
            underline.origin.x,
            underline.origin.y + underline.size.y / 2.0,
        ),
    );
    cx.backend
        .fill_round_rect(underline, 1.5, fade(palette.link, alpha));
    cx.backend.restore();
    let sub_w =
        cx.backend
            .measure_text_family(copy::home_str(locale, "home.explore.sub"), 12.0, SANS);
    text(
        cx,
        copy::home_str(locale, "home.explore.sub"),
        Point2D::new(heading.origin.x + title_w + 18.0, heading.origin.y + 17.0),
        12.0,
        fade(palette.muted, alpha),
    );
    let _ = sub_w;
    for (index, family) in EXPLORE_FAMILIES.iter().enumerate() {
        let rect = Rect::xywh(
            layout.explore_cards[index].origin.x,
            layout.explore_cards[index].origin.y + dy,
            layout.explore_cards[index].size.x,
            layout.explore_cards[index].size.y,
        );
        paint_explore_card(surface, cx, rect, *family, palette);
    }
}

pub(super) fn paint_recent(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    (dy, alpha): (f32, f32),
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let palette = fade_all(palette, alpha);
    let row = Rect::xywh(
        layout.recent.origin.x,
        layout.recent.origin.y + dy,
        layout.recent.size.x,
        layout.recent.size.y,
    );
    // Top hairline 13 px above the row's content.
    cx.backend.stroke_line(
        Point2D::new(row.origin.x, row.origin.y - 13.0),
        Point2D::new(row.origin.x + row.size.x, row.origin.y - 13.0),
        palette.line,
        1.0,
    );
    text_weighted(
        cx,
        copy::home_str(locale, "home.recent.title"),
        Point2D::new(
            row.origin.x,
            jian_widgets::centered_text_baseline_y(
                Rect::xywh(row.origin.x, row.origin.y, 96.0, 32.0),
                16.0,
            ),
        ),
        16.0,
        palette.ink,
        650,
    );
    if surface.recent_files.is_empty() {
        text(
            cx,
            copy::home_str(locale, "home.recent.empty"),
            Point2D::new(
                row.origin.x + 96.0,
                jian_widgets::centered_text_baseline_y(
                    Rect::xywh(row.origin.x, row.origin.y, row.size.x, 32.0),
                    12.0,
                ),
            ),
            12.0,
            palette.muted,
        );
    } else {
        for (index, name) in surface.recent_files.iter().take(5).enumerate() {
            let chip = layout.recent_chips[index];
            let chip = Rect::xywh(chip.origin.x, chip.origin.y + dy, chip.size.x, chip.size.y);
            cx.backend
                .fill_round_rect(chip, 6.0, Color::rgb_u8(0xEA, 0xF2, 0xFF));
            let label: String = name.chars().take(9).collect();
            text(
                cx,
                &label,
                Point2D::new(
                    chip.origin.x + 11.0,
                    jian_widgets::centered_text_baseline_y(chip, 12.0),
                ),
                12.0,
                Color::rgb_u8(0x37, 0x61, 0x99),
            );
        }
    }
    // ＋ 新建空白画布, right-aligned 32 px outline button.
    let button = Rect::xywh(
        layout.new_canvas.origin.x,
        layout.new_canvas.origin.y + dy,
        layout.new_canvas.size.x,
        layout.new_canvas.size.y,
    );
    let hovered = surface.state.hover == Some(HomeHit::NewCanvas);
    cx.backend.fill_round_rect(
        button,
        8.0,
        if hovered {
            palette.button_hover
        } else {
            palette.raised
        },
    );
    cx.backend.stroke_round_rect(
        button,
        8.0,
        if hovered {
            palette.button_hover_line
        } else {
            palette.raised_line
        },
        1.0,
    );
    let label = copy::home_str(locale, "home.recent.newCanvas");
    let label_w = cx.backend.measure_text_family(label, 12.0, SANS);
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(
            button.origin.x + (button.size.x - label_w - 12.0 - 6.0) / 2.0,
            button.origin.y + (button.size.y - 12.0) / 2.0,
        ),
        12.0,
        palette.ink,
        1.8,
    );
    text(
        cx,
        label,
        Point2D::new(
            button.origin.x + (button.size.x - label_w - 12.0 - 6.0) / 2.0 + 12.0 + 6.0,
            jian_widgets::centered_text_baseline_y(button, 12.0),
        ),
        12.0,
        palette.ink,
    );
}

/// Every palette token at `factor` of its alpha (an entrance fade).
pub(super) fn fade_all(palette: StudioPalette, factor: f32) -> StudioPalette {
    palette.faded(factor)
}
