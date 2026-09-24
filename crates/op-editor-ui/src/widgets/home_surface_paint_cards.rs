//! Artwork for the Studio Home surface: the headline's yellow marker,
//! the 示例 sticker, the template previews with their paper edge +
//! tape, and the explore cards (two-piece art in
//! `paint_explore_art`). The App task's phone/desktop mock-ups live in
//! `home_surface_paint_art.rs`.

use super::super::copy::{self, SANS};
use super::super::fade;
use super::super::paint::art::draw_coffee_photo;
use super::super::{HomeSurface, StudioPalette};
use super::explore_copy::fit_lines;
use super::hover_lift_dy;
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::file_menu::truncate_to_width_measured;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, ImageAdjustments, ImageDrawMode, Point2D, Rect};
use op_editor_core::{HomeFamily, HomeHit, InfoKind};

/// The scene-template card previews are uniformly 1024×640 (the card
/// baker's fixed canvas), which is what the contain-fit math below
/// relies on.
const PREVIEW_ASPECT: f32 = 1024.0 / 640.0;

pub(super) fn text(cx: &mut PaintCx<'_>, content: &str, origin: Point2D, size: f32, color: Color) {
    let layout = crate::TextLayout::single_run(content, SANS, size, color.to_jian(), Point2D::ZERO);
    cx.backend.draw_text(&layout, origin);
}

pub(super) fn text_weighted(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    color: Color,
    weight: u16,
) {
    let layout = crate::TextLayout::single_run(content, SANS, size, color.to_jian(), Point2D::ZERO)
        .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}

/// The torn silhouette of the headline marker, in percent of its box.
const VERTICES: [(f32, f32); 21] = [
    (0.0, 29.0),
    (4.0, 11.0),
    (7.0, 27.0),
    (17.0, 13.0),
    (22.0, 27.0),
    (34.0, 8.0),
    (45.0, 24.0),
    (61.0, 4.0),
    (72.0, 17.0),
    (94.0, 8.0),
    (99.0, 34.0),
    (97.0, 63.0),
    (100.0, 89.0),
    (85.0, 82.0),
    (72.0, 100.0),
    (61.0, 90.0),
    (44.0, 97.0),
    (31.0, 77.0),
    (18.0, 99.0),
    (2.0, 81.0),
    (5.0, 53.0),
];

/// The jagged yellow marker band under the 做点什么 run plus its two
/// rays right of the question mark (prototype `.marker` / `.rays`).
pub(super) fn paint_marker(
    cx: &mut PaintCx<'_>,
    x: f32,
    baseline_y: f32,
    width: f32,
    palette: StudioPalette,
) {
    let yellow = palette.marker;
    if palette.marker_underline {
        // Dark mode: the headline is near-white, so the band that sits
        // BEHIND the glyphs in light mode would swallow them. The
        // prototype pulls it down into a thin rule under the baseline
        // (`.marker:before{height:8px;bottom:-2px}`) and keeps the same
        // torn edge, at half the amplitude.
        let top = baseline_y + 2.0;
        let band_h = 8.0;
        let left = x - 5.0;
        let right = x + width + 2.0;
        let band_w = right - left;
        let points: Vec<Point2D> = VERTICES
            .iter()
            .map(|(px, py)| {
                // Compress the jag toward the strip's middle so an 8 px
                // rule keeps a torn silhouette instead of dissolving.
                let compressed = 50.0 + (py - 50.0) * 0.55;
                Point2D::new(
                    left + px / 100.0 * band_w,
                    top + compressed / 100.0 * band_h,
                )
            })
            .collect();
        cx.backend.fill_polygon(&points, yellow);
        for (offset, height, angle) in [(0.0f32, 14.0f32, 28.0f32), (10.0f32, 10.0f32, 68.0f32)] {
            let ray = Rect::xywh(right + 6.0 + offset, baseline_y - 12.0, 5.0, height);
            cx.backend.save();
            cx.backend.rotate(
                angle.to_radians(),
                Point2D::new(ray.origin.x + 2.5, ray.origin.y + ray.size.y),
            );
            cx.backend.fill_round_rect(ray, 3.0, yellow);
            cx.backend.restore();
        }
        return;
    }
    // The prototype seats the 15 px band at the text run's bottom
    // (`bottom:0; height:15px`, behind the glyphs): its top rides the
    // baseline − 6 so the band overlaps the lower 40 % of 做点什么 and
    // bottoms out at baseline + 9.
    let top = baseline_y - 6.0;
    let band_h = 15.0;
    let left = x - 5.0;
    let right = x + width + 2.0;
    let band_w = right - left;
    let points: Vec<Point2D> = VERTICES
        .iter()
        .map(|(px, py)| Point2D::new(left + px / 100.0 * band_w, top + py / 100.0 * band_h))
        .collect();
    cx.backend.fill_polygon(&points, yellow);
    // Two yellow rays past the question mark.
    for (offset, height, angle) in [(0.0f32, 20.0f32, 28.0f32), (12.0f32, 15.0f32, 68.0f32)] {
        let ray = Rect::xywh(right + 6.0 + offset, top - 6.0, 6.0, height);
        cx.backend.save();
        cx.backend.rotate(
            angle.to_radians(),
            Point2D::new(ray.origin.x + 3.0, ray.origin.y + ray.size.y),
        );
        cx.backend.fill_round_rect(ray, 3.0, yellow);
        cx.backend.restore();
    }
}

/// The 示例 sticker: a yellow tab rotated −5° with the blue brush
/// strokes fanning out to its left (prototype `.example-sticker`).
pub(super) fn paint_sticker(
    cx: &mut PaintCx<'_>,
    anchor: Point2D,
    label: &str,
    palette: StudioPalette,
) {
    let label_w = cx.backend.measure_text_family(label, 12.0, SANS);
    let sticker = Rect::xywh(anchor.x - label_w - 20.0, anchor.y, label_w + 20.0, 22.0);
    let pivot = Point2D::new(
        sticker.origin.x + sticker.size.x / 2.0,
        sticker.origin.y + sticker.size.y / 2.0,
    );
    cx.backend.save();
    cx.backend.rotate(-5.0_f32.to_radians(), pivot);
    cx.backend.fill_round_rect(sticker, 2.0, palette.yellow);
    text_weighted(
        cx,
        label,
        Point2D::new(
            sticker.origin.x + 10.0,
            jian_widgets::centered_text_baseline_y(sticker, 12.0),
        ),
        12.0,
        palette.sticker_ink,
        700,
    );
    cx.backend.restore();
    // The brush strokes: ~6 short blue lines sweeping up-left of the tab.
    let stroke_origin = Point2D::new(sticker.origin.x - 26.0, sticker.origin.y + 16.0);
    for (index, (dx, dy, len)) in [
        (0.0, 0.0, 10.0),
        (-7.0, -4.0, 12.0),
        (-13.0, -9.0, 9.0),
        (-4.0, -9.0, 8.0),
        (-17.0, -3.0, 7.0),
        (-9.0, -14.0, 6.0),
    ]
    .into_iter()
    .enumerate()
    {
        let x = stroke_origin.x + dx - index as f32 * 1.5;
        let y = stroke_origin.y + dy;
        cx.backend.stroke_line(
            Point2D::new(x, y),
            Point2D::new(x - len * 0.6, y - len),
            fade(palette.blue, 0.85),
            2.2,
        );
    }
}

/// Contain-fit a template preview into `area`, framed by a white paper
/// edge and topped with a small rotated tape strip. `crop_aspect`
/// overrides the natural aspect (the 4:3 presentation variant);
/// `opacity` carries the art-switch fade.
pub(super) fn paint_template_paper(
    cx: &mut PaintCx<'_>,
    area: Rect,
    template_id: &str,
    palette: StudioPalette,
    crop_aspect: Option<f32>,
    opacity: f32,
) {
    paint_paper_card(cx, area, template_id, palette, crop_aspect, 0.0, opacity);
}

/// One papered template card; `rotation_deg` tilts the whole card
/// (paper, image, tape and shadow) around its centre — the explore
/// decks' tilted second piece.
fn paint_paper_card(
    cx: &mut PaintCx<'_>,
    area: Rect,
    template_id: &str,
    palette: StudioPalette,
    crop_aspect: Option<f32>,
    rotation_deg: f32,
    opacity: f32,
) {
    let Some(asset) = crate::widgets::scene_template_previews::scene_template_preview(template_id)
    else {
        return;
    };
    let aspect = crop_aspect.unwrap_or(PREVIEW_ASPECT);
    // Paper padding around the fitted image, plus headroom for the tape.
    const PAPER_PAD: f32 = 8.0;
    const TAPE_HEADROOM: f32 = 12.0;
    let avail_w = (area.size.x - PAPER_PAD * 2.0).max(10.0);
    let avail_h = (area.size.y - PAPER_PAD * 2.0 - TAPE_HEADROOM).max(10.0);
    let fit_w = avail_w.min(avail_h * aspect);
    let fit_h = fit_w / aspect;
    let image = Rect::xywh(
        area.origin.x + (area.size.x - fit_w) / 2.0,
        area.origin.y + TAPE_HEADROOM + (area.size.y - TAPE_HEADROOM - fit_h) / 2.0,
        fit_w,
        fit_h,
    );
    let paper = Rect::xywh(
        image.origin.x - PAPER_PAD,
        image.origin.y - PAPER_PAD,
        image.size.x + PAPER_PAD * 2.0,
        image.size.y + PAPER_PAD * 2.0,
    );
    cx.backend.save();
    if rotation_deg != 0.0 {
        cx.backend.rotate(
            rotation_deg.to_radians(),
            Point2D::new(
                paper.origin.x + paper.size.x / 2.0,
                paper.origin.y + paper.size.y / 2.0,
            ),
        );
    }
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            paper.origin.x + 4.0,
            paper.origin.y + 8.0,
            paper.size.x - 8.0,
            paper.size.y - 8.0,
        ),
        8.0,
        10.0,
        fade(palette.ink, 0.14),
    );
    cx.backend.fill_round_rect(paper, 4.0, palette.panel);
    cx.backend.stroke_round_rect(paper, 4.0, palette.line, 1.0);
    // The tape strip overlaps the paper's top edge, slightly rotated.
    let tape = Rect::xywh(
        paper.origin.x + paper.size.x * 0.38,
        paper.origin.y - 6.0,
        38.0,
        14.0,
    );
    cx.backend.save();
    cx.backend.rotate(
        -8.0_f32.to_radians(),
        Point2D::new(
            tape.origin.x + tape.size.x / 2.0,
            tape.origin.y + tape.size.y / 2.0,
        ),
    );
    cx.backend
        .fill_round_rect(tape, 2.0, Color::rgba_u8(0xE3, 0xCC, 0xA6, 0.49));
    cx.backend.restore();

    let Some(bytes) = asset.bytes else {
        op_editor_core::web_assets::request(asset.route);
        cx.backend.restore();
        return;
    };
    if !has_cached_image_bytes(asset.image_id) {
        store_remote_image_bytes(asset.image_id, bytes.to_vec());
    }
    let max_edge = required_raster_edge(image, cx.backend.dpi_scale());
    let sharp = cx.backend.image_decoded(asset.image_id, bytes, max_edge);
    if !sharp {
        note_pending_decode(asset.image_id, max_edge);
    }
    if sharp || cx.backend.image_resident(asset.image_id) {
        cx.backend.draw_image_with_options(
            image,
            asset.image_id,
            bytes,
            ImageDrawMode::Fill,
            ImageAdjustments::default(),
            opacity,
            0.0,
        );
    } else {
        cx.backend.fill_rect(image, palette.preview);
    }
    cx.backend.restore();
}

/// The template id a task's preview panel shows.
pub(super) fn preview_template_for(family: HomeFamily, info_kind: InfoKind) -> &'static str {
    match family {
        HomeFamily::Web => "product-landing-light",
        HomeFamily::Presentation => "pitch-deck-dark",
        HomeFamily::KnowledgeCards => "knowledge-carousel",
        HomeFamily::ScreenshotTutorial => "screenshot-tutorial",
        HomeFamily::Infographic => match info_kind {
            InfoKind::Data => "data-report-infographic",
            InfoKind::Flow => "steps-flow-infographic",
            InfoKind::Comparison => "do-dont-comparison",
        },
        _ => "music-fest-poster-card",
    }
}

/// One 看看还能做什么 card: tinted ground, copy column (icon tile,
/// name, two-line description, 查看示例 pill), and the family's
/// two-piece art (`paint_explore_art`).
#[allow(clippy::too_many_lines)]
pub(super) fn paint_explore_card(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    family: HomeFamily,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let (tint, tile, desc_key) = match family {
        HomeFamily::KnowledgeCards => (
            palette.tint_knowledge,
            palette.tile_knowledge,
            "home.explore.knowledgeDesc",
        ),
        HomeFamily::ScreenshotTutorial => (
            palette.tint_tutorial,
            palette.tile_tutorial,
            "home.explore.tutorialDesc",
        ),
        _ => (
            palette.tint_poster,
            palette.tile_poster,
            "home.explore.posterDesc",
        ),
    };
    let hit = HomeHit::ExploreCard(family);
    let hovered = surface.state.hover == Some(hit);
    let pressed = surface.state.pressed == Some(hit);
    // Only the card the cursor just left plays the descent; cards that
    // were never hovered must not dip when the stamp refreshes.
    let leaving = surface.state.card_hover_leaving == Some(family);
    let lift = if hovered || leaving {
        hover_lift_dy(
            hovered,
            surface.ui.motion_stamp(surface.state.card_hover_since_ms),
            surface.now_ms,
        )
    } else {
        0.0
    };
    let press = if pressed { 1.0 } else { 0.0 };
    let paint_rect = Rect::xywh(
        rect.origin.x,
        rect.origin.y + lift + press,
        rect.size.x,
        rect.size.y,
    );
    // The soft hover shadow rides the lift: full strength at −4 px,
    // gone at rest (prototype `0 9px 24px #28354812`).
    let lift_amount = (-lift / 4.0).clamp(0.0, 1.0);
    if lift_amount > 0.0 {
        let shadow = Color::rgba_u8(0x28, 0x35, 0x48, 0.07);
        cx.backend.fill_drop_shadow(
            Rect::xywh(
                paint_rect.origin.x + 9.0,
                paint_rect.origin.y + 9.0,
                paint_rect.size.x - 18.0,
                paint_rect.size.y - 9.0,
            ),
            13.0,
            24.0,
            fade(shadow, lift_amount),
        );
    }
    cx.backend.fill_round_rect(paint_rect, 13.0, tint);
    let copy_w = (paint_rect.size.x * 0.45).min(190.0);
    let pad = 16.0;
    let copy = copy::task_copy(locale, family, surface.state.draft_for(family));
    // Icon tile + name row.
    let tile_rect = Rect::xywh(
        paint_rect.origin.x + pad,
        paint_rect.origin.y + pad,
        28.0,
        28.0,
    );
    cx.backend.fill_round_rect(tile_rect, 7.0, tile);
    draw_icon(
        cx.backend,
        copy::task_icon(family),
        Point2D::new(tile_rect.origin.x + 6.0, tile_rect.origin.y + 6.0),
        16.0,
        palette.tile_ink,
        1.6,
    );
    // The copy column ends where the art begins; nothing painted in it
    // may run past that edge (long-text locales used to run under it).
    let column_right = paint_rect.origin.x + copy_w + 2.0;
    let name_x = tile_rect.origin.x + 36.0;
    let name = truncate_to_width_measured(copy.name, column_right - name_x, |s| {
        cx.backend.measure_text_family(s, 14.0, SANS)
    });
    text_weighted(
        cx,
        &name,
        Point2D::new(
            name_x,
            jian_widgets::centered_text_baseline_y(tile_rect, 14.0),
        ),
        14.0,
        palette.ink,
        650,
    );
    // The description, wrapped to the copy column: as many 19 px lines
    // as fit above the 查看示例 pill (two on the regular card, at most
    // three), ellipsized past that.
    let desc = copy::home_str(locale, desc_key);
    let mut line_y = paint_rect.origin.y + pad + 44.0;
    let pill_top = paint_rect.origin.y + paint_rect.size.y - 14.0 - 28.0;
    let max_lines = (((pill_top - 6.0 - line_y) / 19.0).floor() as usize + 1).clamp(1, 3);
    let desc_x = paint_rect.origin.x + pad;
    let lines = fit_lines(desc, column_right - desc_x, max_lines, |s| {
        cx.backend.measure_text_family(s, 12.0, SANS)
    });
    for line in &lines {
        text(
            cx,
            line,
            Point2D::new(paint_rect.origin.x + pad, line_y),
            12.0,
            fade(palette.ink, 0.62),
        );
        line_y += 19.0;
    }
    // 查看示例 pill pinned to the card's bottom-left.
    let view = copy::home_str(locale, "home.explore.view");
    // 13 px lead, the label, a 5 px gap, the 13 px arrow, 8 px trail —
    // the arrow sits AFTER the label (it used to be drawn over its last
    // glyph, which long labels such as "Смотреть пример" made obvious).
    let view_w = cx.backend.measure_text_family(view, 12.0, SANS) + 13.0 + 5.0 + 13.0 + 8.0;
    let pill = Rect::xywh(
        paint_rect.origin.x + pad,
        paint_rect.origin.y + paint_rect.size.y - 14.0 - 28.0,
        view_w,
        28.0,
    );
    cx.backend
        .fill_round_rect(pill, 7.0, palette.card_link_fill);
    cx.backend
        .stroke_round_rect(pill, 7.0, palette.card_link_line, 1.0);
    text(
        cx,
        view,
        Point2D::new(
            pill.origin.x + 13.0,
            jian_widgets::centered_text_baseline_y(pill, 12.0),
        ),
        12.0,
        palette.card_link_ink,
    );
    draw_icon(
        cx.backend,
        Icon::ArrowUpRight,
        Point2D::new(
            pill.origin.x + pill.size.x - 8.0 - 13.0,
            pill.origin.y + 7.0,
        ),
        13.0,
        palette.link,
        1.5,
    );
    // Art column: the family's two-piece composition.
    let art = Rect::xywh(
        paint_rect.origin.x + copy_w + 6.0,
        paint_rect.origin.y + 10.0,
        (paint_rect.origin.x + paint_rect.size.x - 14.0 - (paint_rect.origin.x + copy_w + 6.0))
            .max(60.0),
        paint_rect.size.y - 26.0,
    );
    paint_explore_art(cx, art, family, palette);
}

/// The explore cards' two-piece art decks (prototype `knowledgeArt` /
/// `tutorialArt` / `posterArt`, built here from the baked template
/// previews): 图文卡片 = two overlapping paper cards, 截图教程 = the
/// tutorial sheet plus a small phone frame overlapping bottom-right,
/// 活动海报 = two poster crops side by side.
fn paint_explore_art(cx: &mut PaintCx<'_>, area: Rect, family: HomeFamily, palette: StudioPalette) {
    match family {
        HomeFamily::KnowledgeCards => {
            let gap = 4.0;
            let card_w = ((area.size.x - gap) / 2.0).max(30.0);
            let first = Rect::xywh(area.origin.x, area.origin.y, card_w, area.size.y);
            let second = Rect::xywh(
                area.origin.x + card_w + gap,
                area.origin.y,
                card_w,
                area.size.y,
            );
            paint_paper_card(
                cx,
                first,
                "knowledge-card-vertical",
                palette,
                None,
                -3.0,
                1.0,
            );
            paint_paper_card(cx, second, "knowledge-carousel", palette, None, 6.0, 1.0);
        }
        HomeFamily::ScreenshotTutorial => {
            let sheet_w = (area.size.x * 0.62).max(50.0);
            let sheet = Rect::xywh(area.origin.x, area.origin.y, sheet_w, area.size.y);
            paint_paper_card(cx, sheet, "screenshot-tutorial", palette, None, 0.0, 1.0);
            let phone_w = (area.size.x * 0.30).clamp(34.0, 64.0);
            let phone_h = (phone_w / 0.475).min(area.size.y * 0.86);
            let phone = Rect::xywh(
                area.origin.x + area.size.x - phone_w + 6.0,
                area.origin.y + area.size.y - phone_h + 8.0,
                phone_w,
                phone_h,
            );
            paint_mini_phone(cx, phone, palette);
        }
        _ => {
            // Two portrait poster crops (aspect .69), the second
            // tilted 4° like the prototype's poster deck.
            let poster_h = area.size.y;
            let poster_w = (poster_h * 0.69).min((area.size.x - 5.0) / 2.0);
            let first = Rect::xywh(area.origin.x, area.origin.y, poster_w, poster_h);
            let second = Rect::xywh(
                area.origin.x + area.size.x - poster_w,
                area.origin.y,
                poster_w,
                poster_h,
            );
            paint_paper_card(
                cx,
                first,
                "music-fest-poster-card",
                palette,
                Some(0.69),
                0.0,
                1.0,
            );
            paint_paper_card(
                cx,
                second,
                "hiring-poster-card",
                palette,
                Some(0.69),
                4.0,
                1.0,
            );
        }
    }
}

/// A small simplified phone frame for the tutorial card's corner
/// overlap: bezel, notch, coffee hero, price strip and nav row —
/// readable at the ~60 px width the card allots it.
fn paint_mini_phone(cx: &mut PaintCx<'_>, phone: Rect, palette: StudioPalette) {
    let w = phone.size.x;
    let s = w / 162.0;
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            phone.origin.x + 2.0,
            phone.origin.y + 3.0,
            w - 4.0,
            phone.size.y - 4.0,
        ),
        23.0 * s,
        6.0,
        fade(palette.ink, 0.16),
    );
    cx.backend
        .fill_round_rect(phone, 23.0 * s, Color::rgb_u8(0xFF, 0xFA, 0xF5));
    cx.backend
        .stroke_round_rect(phone, 23.0 * s, Color::rgb_u8(0x20, 0x23, 0x27), 3.5 * s);
    let border = 3.5 * s;
    let inner = Rect::xywh(
        phone.origin.x + border,
        phone.origin.y + border,
        w - border * 2.0,
        phone.size.y - border * 2.0,
    );
    cx.backend.save();
    cx.backend.clip_round_rect(inner, (23.0 - 3.5) * s);
    // Notch + status dots.
    cx.backend.fill_round_rect(
        Rect::xywh(
            phone.origin.x + w * 0.37,
            phone.origin.y + 5.0 * s,
            w * 0.26,
            6.0 * s,
        ),
        5.0 * s,
        Color::rgb_u8(0x18, 0x1B, 0x20),
    );
    let nav_h = w * 0.23;
    let strip_h = w * 0.30;
    // Coffee hero between the status row and the price strip.
    let hero = Rect::xywh(
        inner.origin.x,
        inner.origin.y + w * 0.14,
        inner.size.x,
        (inner.size.y - nav_h - strip_h - w * 0.14).max(10.0),
    );
    cx.backend
        .clip_round_rect_per_corner(hero, [7.0 * s, 7.0 * s, 0.0, 0.0]);
    if !draw_coffee_photo(cx.backend, hero, 1.0, 0.0) {
        cx.backend.fill_rect(hero, Color::rgb_u8(0xF3, 0xE9, 0xDD));
    }
    cx.backend.restore();
    // Price strip + nav row.
    let strip = Rect::xywh(
        inner.origin.x,
        hero.origin.y + hero.size.y,
        inner.size.x,
        strip_h,
    );
    cx.backend.fill_round_rect_per_corner(
        strip,
        [0.0, 0.0, 8.0 * s, 8.0 * s],
        Color::rgb_u8(0xF6, 0xEA, 0xDC),
    );
    cx.backend.fill_rect(
        Rect::xywh(strip.origin.x + 3.0, strip.origin.y + 4.0, w * 0.34, 3.0),
        Color::rgb_u8(0x3D, 0x2B, 0x1F),
    );
    cx.backend.fill_oval(
        Rect::xywh(
            strip.origin.x + strip.size.x - w * 0.14 - 3.0,
            strip.origin.y + strip.size.y - w * 0.16,
            w * 0.14,
            w * 0.14,
        ),
        Color::rgb_u8(0x3D, 0x2B, 0x1F),
    );
    let nav_y = inner.origin.y + inner.size.y - nav_h;
    cx.backend.fill_rect(
        Rect::xywh(inner.origin.x, nav_y, inner.size.x, nav_h),
        Color::WHITE,
    );
    let icons = [Icon::Home, Icon::LayoutGrid, Icon::ShoppingBag, Icon::User];
    for (index, icon) in icons.into_iter().enumerate() {
        let icon_w = 6.0 * s.max(0.55);
        let centre_x = inner.origin.x + inner.size.x * ((index as f32 + 0.5) / 4.0);
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(centre_x - icon_w / 2.0, nav_y + nav_h * 0.22),
            icon_w,
            if index == 0 {
                palette.blue
            } else {
                Color::rgb_u8(0x9A, 0xA3, 0xB0)
            },
            1.4,
        );
    }
}
