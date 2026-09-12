//! Immediate-mode paint pass for the 制图台 Home surface.

use super::{HomeLayout, HomePalette, HomeSurface, HOME_TOPBAR_H};
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::property_panel_text_input::paint_text_input_view;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, ImageDrawMode, Point2D, Rect, TextLayout, Theme};
use op_editor_core::{EditorUiState, HomeDevice, HomeFamily, HomeHit, ThemeMode};

const SANS: &str = "system-ui";
const MONO: &str = "SF Mono";
const SERIF_CANDIDATES: [&str; 4] = [
    "Songti SC",
    "STSong",
    "Noto Serif CJK SC",
    "Source Han Serif SC",
];

fn text(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    color: Color,
    family: &str,
) {
    let layout = TextLayout::single_run(content, family, size, color.to_jian(), Point2D::ZERO);
    cx.backend.draw_text(&layout, origin);
}

fn text_weighted(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    color: Color,
    family: &str,
    weight: u16,
) {
    let layout = TextLayout::single_run(content, family, size, color.to_jian(), Point2D::ZERO)
        .with_font_weight(weight);
    cx.backend.draw_text(&layout, origin);
}

#[allow(clippy::too_many_arguments)]
fn draw_spaced_text(
    cx: &mut PaintCx<'_>,
    content: &str,
    origin: Point2D,
    size: f32,
    color: Color,
    family: &str,
    weight: u16,
    spacing: f32,
) {
    let mut x = origin.x;
    for character in content.chars() {
        let glyph = character.to_string();
        text_weighted(
            cx,
            &glyph,
            Point2D::new(x, origin.y),
            size,
            color,
            family,
            weight,
        );
        x += cx.backend.measure_text_family(&glyph, size, family) + spacing;
    }
}

fn spaced_width(cx: &mut PaintCx<'_>, content: &str, size: f32, family: &str, spacing: f32) -> f32 {
    content
        .chars()
        .map(|character| {
            cx.backend
                .measure_text_family(&character.to_string(), size, family)
                + spacing
        })
        .sum::<f32>()
        - spacing
}

fn label_color(surface: &HomeSurface<'_>) -> Color {
    home_palette(surface).ink
}

fn home_palette(surface: &HomeSurface<'_>) -> HomePalette {
    HomePalette::for_mode(surface.ui.effective_theme_mode())
}

fn line(surface: &HomeSurface<'_>) -> Color {
    home_palette(surface).line
}

fn graphite(surface: &HomeSurface<'_>) -> Color {
    home_palette(surface).graphite
}

fn blue(surface: &HomeSurface<'_>) -> Color {
    home_palette(surface).blue
}

fn home_input_theme(palette: HomePalette) -> Theme {
    let mut theme = Theme::light();
    theme.background = palette.sheet;
    theme.foreground = palette.ink;
    theme.card = palette.sheet;
    theme.card_foreground = palette.ink;
    theme.primary = palette.blue;
    theme.primary_foreground = palette.sheet;
    theme.muted = palette.paper_2;
    theme.muted_foreground = palette.ash;
    theme.border = palette.line;
    theme.input = palette.line;
    theme.ring = palette.blue;
    theme.accent = palette.blue_soft;
    theme.accent_foreground = palette.blue_2;
    theme
}

fn paint_button(
    surface: &HomeSurface<'_>,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    hit: HomeHit,
    active: bool,
    text_label: &str,
) {
    let hovered = surface.state.hover == Some(hit);
    let pressed = surface.state.pressed == Some(hit);
    let palette = home_palette(surface);
    let fill = if active { palette.ink } else { palette.sheet };
    let fg = if active { palette.paper } else { palette.ink };
    cx.backend.fill_round_rect(rect, rect.size.y / 2.0, fill);
    if hovered || pressed {
        cx.backend.fill_round_rect(
            rect,
            rect.size.y / 2.0,
            if pressed {
                palette.blue.with_alpha(0.20)
            } else {
                palette.graphite.with_alpha(0.10)
            },
        );
    }
    cx.backend.stroke_round_rect(
        rect,
        rect.size.y / 2.0,
        if active { palette.ink } else { line(surface) },
        1.0,
    );
    let width = rect.size.x;
    let approx = cx.backend.measure_text_family(text_label, 14.0, SANS);
    text(
        cx,
        text_label,
        Point2D::new(
            rect.origin.x + (width - approx) / 2.0,
            rect.origin.y + rect.size.y / 2.0 + 5.0,
        ),
        14.0,
        fg,
        SANS,
    );
}

pub(super) fn paint_home(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect) {
    let layout = surface.layout(rect.size.x, rect.size.y);
    let palette = home_palette(surface);
    let bg = palette.paper;
    cx.backend.fill_rect(rect, bg);
    // Drafting ground: static dot grid and blue margin rule.
    let dots = palette.dots;
    let mut x = 12.0;
    while x < rect.size.x {
        let mut y = 12.0;
        while y < rect.size.y {
            cx.backend
                .fill_oval(Rect::xywh(x - 0.75, y - 0.75, 1.5, 1.5), dots);
            y += 24.0;
        }
        x += 24.0;
    }
    cx.backend.stroke_line(
        Point2D::new(56.0, 0.0),
        Point2D::new(56.0, rect.size.y),
        palette.margin_rule,
        1.0,
    );

    paint_wordmark(surface, cx, rect);
    text(
        cx,
        "专业模式 · 直接进画布 →",
        Point2D::new(
            layout.professional.origin.x,
            layout.professional.origin.y + 19.0,
        ),
        13.0,
        graphite(surface),
        SANS,
    );
    if surface.state.hover == Some(HomeHit::Professional) {
        cx.backend.stroke_line(
            Point2D::new(
                layout.professional.origin.x,
                layout.professional.origin.y + 25.0,
            ),
            Point2D::new(
                layout.professional.origin.x + layout.professional.size.x,
                layout.professional.origin.y + 25.0,
            ),
            palette.blue,
            1.0,
        );
    }
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        0.0,
        HOME_TOPBAR_H,
        rect.size.x,
        (layout.footer.origin.y - HOME_TOPBAR_H).max(0.0),
    ));
    let headline_family = headline_family(surface);
    let headline_text = "你想做成什么？";
    let spacing = 52.0 * 0.01;
    let headline_width = spaced_width(cx, headline_text, 52.0, headline_family, spacing);
    draw_spaced_text(
        cx,
        headline_text,
        Point2D::new(
            layout.headline.origin.x + (layout.headline.size.x - headline_width) / 2.0,
            layout.headline.origin.y + 43.0,
        ),
        52.0,
        palette.ink,
        headline_family,
        600,
        spacing,
    );
    let underline_start = layout.headline.origin.x
        + (layout.headline.size.x - headline_width) / 2.0
        + cx.backend
            .measure_text_family("你想", 52.0, headline_family)
        + spacing * 2.0;
    let underline_width = spaced_width(cx, "做成什么", 52.0, headline_family, spacing);
    paint_wavy_underline(
        cx,
        underline_start,
        layout.headline.origin.y + 50.0,
        underline_width,
        palette.blue,
    );
    text(
        cx,
        "先说要做成的东西，再放你的文字或截图。画布还在，随时进。",
        Point2D::new(
            layout.subtitle.origin.x + 44.0,
            layout.subtitle.origin.y + 16.0,
        ),
        15.0,
        graphite(surface),
        SANS,
    );

    paint_sheet(surface, cx, layout);
    for (index, family) in HomeFamily::ALL.into_iter().enumerate() {
        paint_button(
            surface,
            cx,
            layout.chips[index],
            HomeHit::Chip(family),
            surface.state.bound == Some(family),
            family.label(),
        );
    }
    paint_expected(surface, cx, layout);
    for (index, family) in HomeFamily::ALL.into_iter().enumerate() {
        paint_card(surface, cx, layout.cards[index], family);
    }
    cx.backend.restore();
    paint_footer(surface, cx, layout);
}

fn headline_family(surface: &HomeSurface<'_>) -> &'static str {
    resolve_headline_family(surface.ui)
}

fn resolve_headline_family(ui: &EditorUiState) -> &'static str {
    for candidate in SERIF_CANDIDATES {
        if ui
            .system_font_families
            .iter()
            .chain(ui.bundled_font_families.iter())
            .any(|family| family.eq_ignore_ascii_case(candidate))
        {
            return candidate;
        }
    }
    SANS
}

fn paint_wavy_underline(cx: &mut PaintCx<'_>, x: f32, y: f32, width: f32, color: Color) {
    let segment = width / 6.0;
    let points = [
        Point2D::new(x, y),
        Point2D::new(x + segment, y - 1.0),
        Point2D::new(x + segment * 2.0, y + 0.5),
        Point2D::new(x + segment * 3.0, y - 0.5),
        Point2D::new(x + segment * 4.0, y + 0.8),
        Point2D::new(x + segment * 5.0, y - 0.4),
        Point2D::new(x + width, y),
    ];
    for pair in points.windows(2) {
        cx.backend.stroke_line(pair[0], pair[1], color, 2.2);
    }
}

fn paint_wordmark(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect) {
    let mark = Rect::xywh(80.0, 22.0, 18.0, 18.0);
    cx.backend
        .stroke_round_rect(mark, 4.0, label_color(surface), 1.5);
    cx.backend
        .fill_round_rect(Rect::xywh(84.0, 26.0, 8.0, 8.0), 1.0, blue(surface));
    text(
        cx,
        "OpenPencil",
        Point2D::new(108.0, 36.0),
        14.0,
        label_color(surface),
        SANS,
    );
    let _ = rect;
}

fn paint_sheet(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, layout: HomeLayout) {
    let palette = home_palette(surface);
    // Home owns its focus treatment. The editor's normal blue ring must not
    // leak into the drafting-table surface.
    cx.backend.fill_drop_shadow(
        Rect::xywh(
            layout.sheet.origin.x - 3.0,
            layout.sheet.origin.y - 3.0,
            layout.sheet.size.x + 6.0,
            layout.sheet.size.y + 6.0,
        ),
        14.0,
        6.0,
        palette.blue_soft.with_alpha(0.55),
    );
    cx.backend
        .fill_round_rect(layout.sheet, 14.0, palette.sheet);
    cx.backend
        .stroke_round_rect(layout.sheet, 14.0, palette.blue.with_alpha(0.55), 1.0);
    for tick in 1..28 {
        let x = layout.sheet.origin.x + tick as f32 * 22.0;
        cx.backend.stroke_line(
            Point2D::new(x, layout.sheet.origin.y),
            Point2D::new(x, layout.sheet.origin.y + 9.0),
            palette.line.with_alpha(0.45),
            1.0,
        );
    }
    let placeholder = surface
        .state
        .bound
        .unwrap_or(HomeFamily::AppUi)
        .placeholder();
    let input_theme = home_input_theme(palette);
    paint_text_input_view(
        cx,
        &input_theme,
        &surface.state.input,
        layout.sheet_text,
        16.0,
        0.0,
        layout.sheet_text.origin.y + 25.0,
        surface.now_ms,
        if surface.state.draft.is_empty() {
            placeholder
        } else {
            ""
        },
        surface.state.visible,
    );
    let refs = [
        (layout.screenshot, HomeHit::Attachment, "截图", Icon::Image),
        (
            layout.reference_link,
            HomeHit::ReferenceLink,
            "参考链接",
            Icon::ArrowUpRight,
        ),
        (layout.figma, HomeHit::Figma, "Figma", Icon::Pen),
        (
            layout.example,
            HomeHit::TryExample,
            "试试这个示例",
            Icon::Sparkles,
        ),
    ];
    for (rect, hit, label, icon) in refs {
        let disabled = matches!(hit, HomeHit::ReferenceLink | HomeHit::Figma);
        let color = if disabled {
            palette.graphite.with_alpha(0.65)
        } else {
            palette.graphite
        };
        if surface.state.hover == Some(hit) && !disabled {
            cx.backend.fill_round_rect(rect, 8.0, palette.paper_2);
        }
        draw_icon(
            cx.backend,
            icon,
            Point2D::new(rect.origin.x, rect.origin.y + 6.0),
            14.0,
            color,
            1.25,
        );
        text(
            cx,
            label,
            Point2D::new(rect.origin.x + 19.0, rect.origin.y + 19.0),
            13.0,
            color,
            SANS,
        );
        if disabled && surface.state.hover == Some(hit) {
            let tooltip = Rect::xywh(rect.origin.x, rect.origin.y - 28.0, 68.0, 22.0);
            cx.backend.fill_round_rect(tooltip, 7.0, palette.ink);
            text(
                cx,
                "即将支持",
                Point2D::new(tooltip.origin.x + 10.0, tooltip.origin.y + 15.0),
                11.0,
                palette.paper,
                SANS,
            );
        }
    }
    let send_fill = if surface.state.draft.trim().is_empty() {
        palette.ink.with_alpha(0.28)
    } else {
        palette.blue
    };
    cx.backend.fill_oval(layout.send, send_fill);
    if surface.state.pressed == Some(HomeHit::Send) {
        cx.backend
            .stroke_oval(layout.send, palette.ink.with_alpha(0.30), 2.0);
    }
    draw_icon(
        cx.backend,
        Icon::ArrowUp,
        Point2D::new(layout.send.origin.x + 11.0, layout.send.origin.y + 11.0),
        18.0,
        palette.paper,
        1.8,
    );
    text(
        cx,
        "⏎ 发送",
        Point2D::new(layout.send.origin.x - 58.0, layout.send.origin.y + 25.0),
        12.0,
        palette.ash,
        MONO,
    );
}

fn paint_expected(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, layout: HomeLayout) {
    let palette = home_palette(surface);
    let Some(family) = surface.state.bound else {
        return;
    };
    text(
        cx,
        "预期产物：",
        Point2D::new(layout.expected.origin.x, layout.expected.origin.y + 20.0),
        13.0,
        palette.graphite,
        SANS,
    );
    let mut x = layout.expected.origin.x + 72.0;
    for output in family.expected_outputs() {
        let w = cx.backend.measure_text_family(output, 12.0, SANS) + 20.0;
        let pill = Rect::xywh(x, layout.expected.origin.y + 1.0, w, 24.0);
        cx.backend.fill_round_rect(pill, 12.0, palette.blue_soft);
        text(
            cx,
            output,
            Point2D::new(x + 10.0, layout.expected.origin.y + 18.0),
            12.0,
            palette.blue_2,
            SANS,
        );
        x += w + 6.0;
    }
    if family == HomeFamily::AppUi {
        text(
            cx,
            "给哪种设备？",
            Point2D::new(
                layout.expected.origin.x + 374.0,
                layout.expected.origin.y + 18.0,
            ),
            13.0,
            palette.graphite,
            SANS,
        );
        let active = if surface.state.device == HomeDevice::Mobile {
            layout.device_mobile
        } else {
            layout.device_desktop
        };
        let inactive = if surface.state.device == HomeDevice::Mobile {
            layout.device_desktop
        } else {
            layout.device_mobile
        };
        cx.backend.fill_round_rect(inactive, 13.0, palette.sheet);
        cx.backend
            .stroke_round_rect(inactive, 13.0, palette.line, 1.0);
        cx.backend.fill_round_rect(active, 13.0, palette.ink);
        text(
            cx,
            "手机",
            Point2D::new(
                layout.device_mobile.origin.x + 9.0,
                layout.device_mobile.origin.y + 18.0,
            ),
            12.0,
            if surface.state.device == HomeDevice::Mobile {
                palette.paper
            } else {
                palette.graphite
            },
            SANS,
        );
        text(
            cx,
            "桌面",
            Point2D::new(
                layout.device_desktop.origin.x + 9.0,
                layout.device_desktop.origin.y + 18.0,
            ),
            12.0,
            if surface.state.device == HomeDevice::Desktop {
                palette.paper
            } else {
                palette.graphite
            },
            SANS,
        );
    }
}

fn paint_card(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect, family: HomeFamily) {
    let hover = surface.state.hover == Some(HomeHit::Card(family));
    let pressed = surface.state.pressed == Some(HomeHit::Card(family));
    let palette = home_palette(surface);
    let paint_rect = if hover {
        Rect::xywh(rect.origin.x, rect.origin.y - 6.0, rect.size.x, rect.size.y)
    } else {
        rect
    };
    if surface.ui.effective_theme_mode() == ThemeMode::Light {
        cx.backend.fill_drop_shadow(
            Rect::xywh(
                paint_rect.origin.x - 2.0,
                paint_rect.origin.y + 2.0,
                paint_rect.size.x + 4.0,
                paint_rect.size.y + 4.0,
            ),
            14.0,
            8.0,
            Color::BLACK.with_alpha(0.10),
        );
    }
    cx.backend.fill_round_rect(paint_rect, 14.0, palette.sheet);
    if hover || pressed {
        cx.backend.fill_round_rect(
            paint_rect,
            14.0,
            palette.blue.with_alpha(if pressed { 0.18 } else { 0.08 }),
        );
    }
    cx.backend.stroke_round_rect(
        paint_rect,
        14.0,
        if surface.state.bound == Some(family) {
            palette.blue
        } else {
            palette.line
        },
        1.0,
    );
    let art = Rect::xywh(
        paint_rect.origin.x,
        paint_rect.origin.y,
        paint_rect.size.x,
        (paint_rect.size.y.min(200.0) - 64.0).max(40.0),
    );
    cx.backend.fill_rect(art, palette.paper_2);
    let tag = Rect::xywh(art.origin.x + 12.0, art.origin.y + 12.0, 44.0, 22.0);
    cx.backend
        .fill_round_rect(tag, 7.0, palette.sheet.with_alpha(0.86));
    text(
        cx,
        "示例",
        Point2D::new(tag.origin.x + 8.0, tag.origin.y + 15.0),
        11.0,
        palette.graphite,
        MONO,
    );
    match family {
        HomeFamily::AppUi => paint_app_flow(surface, cx, art),
        HomeFamily::KnowledgeCards => {
            paint_template_preview(surface, cx, art, "knowledge-carousel")
        }
        HomeFamily::ScreenshotTutorial => {
            paint_template_preview(surface, cx, art, "screenshot-tutorial")
        }
        HomeFamily::EventPoster => paint_template_preview(surface, cx, art, "event-poster-deck"),
    }
    let title_y = paint_rect.origin.y + paint_rect.size.y - 43.0;
    text_weighted(
        cx,
        family.label(),
        Point2D::new(paint_rect.origin.x + 14.0, title_y),
        15.0,
        palette.ink,
        SANS,
        650,
    );
    let desc = match family {
        HomeFamily::AppUi => "一句话或一张截图，到可编辑的高保真界面",
        HomeFamily::KnowledgeCards => "把这段文字做成一套今天能发的图",
        HomeFamily::ScreenshotTutorial => "把几张截图串成一篇步骤图",
        HomeFamily::EventPoster => "做一组完整一致的活动视觉",
    };
    text(
        cx,
        desc,
        Point2D::new(paint_rect.origin.x + 14.0, title_y + 22.0),
        12.5,
        palette.graphite,
        SANS,
    );
}

fn paint_app_flow(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, art: Rect) {
    let palette = home_palette(surface);
    let phone_w = 56.0;
    let phone_h = art.size.y.min(136.0) - 30.0;
    let y = art.origin.y + (art.size.y - phone_h) / 2.0;
    let start_x = art.origin.x + (art.size.x - phone_w * 3.0 - 48.0) / 2.0;
    for index in 0..3 {
        let x = start_x + index as f32 * (phone_w + 24.0);
        let nudged = if surface.state.hover == Some(HomeHit::Card(HomeFamily::AppUi)) && index == 1
        {
            Rect::xywh(x + 3.0, y - 3.0, phone_w, phone_h)
        } else {
            Rect::xywh(x, y, phone_w, phone_h)
        };
        let phone = nudged;
        cx.backend.fill_round_rect(phone, 8.0, palette.sheet);
        cx.backend
            .stroke_round_rect(phone, 8.0, palette.graphite, 1.2);
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 12.0, phone_w - 14.0, 7.0),
            palette.line,
        );
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 28.0, phone_w - 14.0, 6.0),
            palette.line,
        );
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 42.0, phone_w - 14.0, 6.0),
            palette.line,
        );
        cx.backend.fill_round_rect(
            Rect::xywh(x + 7.0, y + phone_h - 22.0, phone_w - 14.0, 10.0),
            3.0,
            if index == 1 {
                palette.blue
            } else {
                palette.graphite.with_alpha(0.35)
            },
        );
        if index < 2 {
            cx.backend.stroke_line(
                Point2D::new(x + phone_w + 5.0, y + phone_h / 2.0),
                Point2D::new(x + phone_w + 18.0, y + phone_h / 2.0),
                palette.blue,
                1.5,
            );
            cx.backend.stroke_line(
                Point2D::new(x + phone_w + 14.0, y + phone_h / 2.0 - 4.0),
                Point2D::new(x + phone_w + 18.0, y + phone_h / 2.0),
                palette.blue,
                1.5,
            );
        }
    }
}

fn paint_template_preview(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect, id: &str) {
    let Some(asset) = crate::widgets::scene_template_previews::scene_template_preview(id) else {
        return;
    };
    let Some(bytes) = asset.bytes else {
        op_editor_core::web_assets::request(asset.route);
        return;
    };
    if !has_cached_image_bytes(asset.image_id) {
        store_remote_image_bytes(asset.image_id, bytes.to_vec());
    }
    let max_edge = required_raster_edge(rect, cx.backend.dpi_scale());
    let sharp = cx.backend.image_decoded(asset.image_id, bytes, max_edge);
    if !sharp {
        note_pending_decode(asset.image_id, max_edge);
    }
    if sharp || cx.backend.image_resident(asset.image_id) {
        cx.backend
            .draw_image_with_mode(rect, asset.image_id, bytes, ImageDrawMode::Fill);
    } else {
        cx.backend.fill_rect(rect, home_palette(surface).paper_2);
    }
}

fn paint_footer(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, layout: HomeLayout) {
    let palette = home_palette(surface);
    text(
        cx,
        "最近项目（空）",
        Point2D::new(layout.footer.origin.x, layout.footer.origin.y + 16.0),
        13.0,
        palette.graphite,
        SANS,
    );
    text(
        cx,
        "·",
        Point2D::new(layout.footer.origin.x + 88.0, layout.footer.origin.y + 16.0),
        13.0,
        palette.graphite,
        SANS,
    );
    text(
        cx,
        "新建空白画布",
        Point2D::new(
            layout.footer.origin.x + 104.0,
            layout.footer.origin.y + 16.0,
        ),
        13.0,
        palette.ink,
        SANS,
    );
    text(
        cx,
        "·  打开文件",
        Point2D::new(
            layout.footer.origin.x + 206.0,
            layout.footer.origin.y + 16.0,
        ),
        13.0,
        palette.ink,
        SANS,
    );
}

#[cfg(test)]
mod tests {
    use super::{resolve_headline_family, EditorUiState};
    use std::sync::Arc;

    #[test]
    fn headline_serif_resolution_follows_the_authored_candidate_order() {
        let ui = EditorUiState {
            system_font_families: Arc::new(vec!["Source Han Serif SC".into(), "Songti SC".into()]),
            ..EditorUiState::default()
        };
        assert_eq!(resolve_headline_family(&ui), "Songti SC");
    }

    #[test]
    fn headline_serif_resolution_falls_back_to_sans_when_unavailable() {
        let ui = EditorUiState {
            system_font_families: Arc::new(vec!["PingFang SC".into()]),
            bundled_font_families: Arc::new(vec!["Inter".into()]),
            ..EditorUiState::default()
        };
        assert_eq!(resolve_headline_family(&ui), "system-ui");
    }
}
