//! Paint for the touch-tablet Home's 作品 grid (geometry in
//! `home_surface_tablet.rs`): the section heading with 新建空白画布, the
//! current-work card and one card per recent document.

use super::super::copy::{self, SANS};
use super::super::palette::fade;
use super::super::{HomeLayout, HomeSurface, StudioPalette};
use super::cards::{text, text_weighted};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};
use op_editor_core::HomeHit;

fn shifted(rect: Rect, dy: f32) -> Rect {
    Rect::xywh(rect.origin.x, rect.origin.y + dy, rect.size.x, rect.size.y)
}

/// Cut to `max_w` with an ellipsis.
fn fit(cx: &mut PaintCx<'_>, content: &str, size: f32, max_w: f32) -> String {
    if cx.backend.measure_text_family(content, size, SANS) <= max_w {
        return content.to_string();
    }
    let mut chars: Vec<char> = content.chars().collect();
    while chars.pop().is_some() {
        let candidate = chars.iter().collect::<String>() + "…";
        if cx.backend.measure_text_family(&candidate, size, SANS) <= max_w {
            return candidate;
        }
    }
    String::new()
}

pub(in super::super) fn paint_works_grid(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &HomeLayout,
    (dy, alpha): (f32, f32),
    palette: StudioPalette,
) {
    let Some(grid) = surface.works_grid(layout) else {
        return;
    };
    let locale = surface.ui.locale;
    let palette = palette.faded(alpha);
    let heading = shifted(grid.heading, dy);
    cx.backend.stroke_line(
        Point2D::new(heading.origin.x, heading.origin.y - 13.0),
        Point2D::new(heading.origin.x + heading.size.x, heading.origin.y - 13.0),
        palette.line,
        1.0,
    );
    text_weighted(
        cx,
        copy::home_str(locale, "home.nav.projects"),
        Point2D::new(
            heading.origin.x,
            jian_widgets::centered_text_baseline_y(heading, 18.0),
        ),
        18.0,
        palette.ink,
        680,
    );
    paint_new_canvas(surface, cx, shifted(layout.new_canvas, dy), palette);
    if let Some(empty) = grid.empty {
        let empty = shifted(empty, dy);
        let note = fit(
            cx,
            op_i18n::translate(locale, "works.empty"),
            13.0,
            empty.size.x,
        );
        text(
            cx,
            &note,
            Point2D::new(
                empty.origin.x,
                jian_widgets::centered_text_baseline_y(empty, 13.0),
            ),
            13.0,
            palette.muted,
        );
    }
    for &(hit, card) in &grid.cards {
        paint_card(surface, cx, hit, shifted(card, dy), palette);
    }
}

fn paint_card(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    hit: HomeHit,
    card: Rect,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let pressed = surface.state.pressed == Some(hit);
    let current = hit == HomeHit::WorksCurrent;
    cx.backend.fill_round_rect(
        card,
        16.0,
        if pressed {
            palette.button_hover
        } else {
            palette.panel
        },
    );
    cx.backend.stroke_round_rect(
        card,
        16.0,
        if current { palette.blue } else { palette.line },
        if current { 1.5 } else { 1.0 },
    );
    let tile = Rect::xywh(card.origin.x + 14.0, card.origin.y + 14.0, 68.0, 68.0);
    cx.backend.fill_round_rect(tile, 12.0, palette.preview);
    draw_icon(
        cx.backend,
        if current { Icon::Frame } else { Icon::FileText },
        Point2D::new(tile.origin.x + 22.0, tile.origin.y + 22.0),
        24.0,
        palette.link,
        1.6,
    );
    let text_x = tile.origin.x + tile.size.x + 14.0;
    let text_w = (card.origin.x + card.size.x - 14.0 - text_x).max(0.0);
    let (eyebrow, title, subtitle) = match hit {
        HomeHit::WorksCurrent => match surface.current_work.as_ref() {
            Some(work) => (
                Some(op_i18n::translate(locale, "works.current")),
                work.title.clone(),
                Some(work.subtitle.clone()),
            ),
            None => return,
        },
        HomeHit::WorksRecent(index) => match surface.works_recent.get(index) {
            Some(name) => (None, name.trim_end_matches(".op").to_string(), None),
            None => return,
        },
        _ => return,
    };
    // Current: eyebrow / title / subtitle; recent: the name alone,
    // centred on the card.
    let title_y = if eyebrow.is_some() {
        card.origin.y + 52.0
    } else {
        jian_widgets::centered_text_baseline_y(card, 15.0)
    };
    if let Some(eyebrow) = eyebrow {
        text_weighted(
            cx,
            eyebrow,
            Point2D::new(text_x, card.origin.y + 30.0),
            11.0,
            palette.eyebrow,
            600,
        );
    }
    let title = fit(cx, &title, 15.0, text_w);
    text_weighted(
        cx,
        &title,
        Point2D::new(text_x, title_y),
        15.0,
        palette.ink,
        650,
    );
    if let Some(subtitle) = subtitle {
        let subtitle = fit(cx, &subtitle, 12.0, text_w);
        text_weighted(
            cx,
            &subtitle,
            Point2D::new(text_x, card.origin.y + 72.0),
            12.0,
            fade(palette.ink, 0.55),
            500,
        );
    }
}

/// ＋ 新建空白画布 at its 44 pt tablet size.
fn paint_new_canvas(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    button: Rect,
    palette: StudioPalette,
) {
    let pressed = surface.state.pressed == Some(HomeHit::NewCanvas);
    cx.backend.fill_round_rect(
        button,
        12.0,
        if pressed {
            palette.button_hover
        } else {
            palette.raised
        },
    );
    cx.backend
        .stroke_round_rect(button, 12.0, palette.raised_line, 1.0);
    let label = copy::home_str(surface.ui.locale, "home.recent.newCanvas");
    let label_w = cx.backend.measure_text_family(label, 13.0, SANS);
    let x = button.origin.x + (button.size.x - label_w - 14.0 - 6.0).max(0.0) / 2.0;
    draw_icon(
        cx.backend,
        Icon::Plus,
        Point2D::new(x, button.origin.y + (button.size.y - 14.0) / 2.0),
        14.0,
        palette.ink,
        1.8,
    );
    text(
        cx,
        label,
        Point2D::new(
            x + 20.0,
            jian_widgets::centered_text_baseline_y(button, 13.0),
        ),
        13.0,
        palette.ink,
    );
}
