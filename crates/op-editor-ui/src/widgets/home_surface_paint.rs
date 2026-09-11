//! Immediate-mode paint pass for the 制图台 Home surface.

use super::{HomeLayout, HomeSurface};
use crate::widgets::canvas_viewport_image::{
    has_cached_image_bytes, note_pending_decode, required_raster_edge, store_remote_image_bytes,
};
use crate::widgets::property_panel_text_input::paint_text_input_view;
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, ImageDrawMode, Point2D, Rect, TextLayout};
use op_editor_core::{HomeDevice, HomeFamily, HomeHit, ThemeMode};

const SANS: &str = "system-ui";
const SERIF: &str = "Songti SC";

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

fn label_color(surface: &HomeSurface<'_>) -> Color {
    surface.theme.foreground
}

fn paper(surface: &HomeSurface<'_>) -> Color {
    if surface.ui.effective_theme_mode() == ThemeMode::Light {
        Color::rgb_u8(0xF4, 0xF1, 0xEA)
    } else {
        surface.theme.background
    }
}

fn line(surface: &HomeSurface<'_>) -> Color {
    if surface.ui.effective_theme_mode() == ThemeMode::Light {
        Color::rgb_u8(0xD9, 0xD3, 0xC6)
    } else {
        surface.theme.border
    }
}

fn sheet(surface: &HomeSurface<'_>) -> Color {
    if surface.ui.effective_theme_mode() == ThemeMode::Light {
        Color::rgb_u8(0xFF, 0xFD, 0xF9)
    } else {
        surface.theme.card
    }
}

fn graphite(surface: &HomeSurface<'_>) -> Color {
    if surface.ui.effective_theme_mode() == ThemeMode::Light {
        Color::rgb_u8(0x5E, 0x5A, 0x52)
    } else {
        surface.theme.muted_foreground
    }
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
    let fill = if active {
        surface.theme.foreground
    } else {
        sheet(surface)
    };
    let fg = if active {
        paper(surface)
    } else {
        label_color(surface)
    };
    cx.backend.fill_round_rect(rect, rect.size.y / 2.0, fill);
    if hovered || pressed {
        cx.backend.fill_round_rect(
            rect,
            rect.size.y / 2.0,
            if pressed {
                surface.theme.primary.with_alpha(0.20)
            } else {
                surface.theme.primary.with_alpha(0.10)
            },
        );
    }
    cx.backend.stroke_round_rect(
        rect,
        rect.size.y / 2.0,
        if active {
            surface.theme.foreground
        } else {
            line(surface)
        },
        1.0,
    );
    let width = rect.size.x;
    let approx = text_label.chars().count() as f32 * 7.0;
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
    let bg = paper(surface);
    cx.backend.fill_rect(rect, bg);
    // Drafting ground: static dot grid and blue margin rule.
    let dots =
        label_color(surface).with_alpha(if surface.ui.effective_theme_mode() == ThemeMode::Light {
            0.10
        } else {
            0.08
        });
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
        surface.theme.primary.with_alpha(0.28),
        1.0,
    );

    // M1: entrance choreography.
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
    text(
        cx,
        "你想做成什么？",
        Point2D::new(
            layout.headline.origin.x + 42.0,
            layout.headline.origin.y + 42.0,
        ),
        44.0,
        label_color(surface),
        headline_family(surface),
    );
    cx.backend.stroke_line(
        Point2D::new(
            layout.headline.origin.x + 120.0,
            layout.headline.origin.y + 51.0,
        ),
        Point2D::new(
            layout.headline.origin.x + 282.0,
            layout.headline.origin.y + 55.0,
        ),
        surface.theme.primary,
        2.0,
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
    if surface.state.bound.is_some() {
        paint_expected(surface, cx, layout);
    }
    for (index, family) in HomeFamily::ALL.into_iter().enumerate() {
        paint_card(surface, cx, layout.cards[index], family);
    }
    paint_footer(surface, cx, layout);
}

fn headline_family(surface: &HomeSurface<'_>) -> &'static str {
    let has_serif = surface
        .ui
        .bundled_font_families
        .iter()
        .chain(surface.ui.system_font_families.iter())
        .any(|family| family.contains("Song") || family.contains("Serif") || family.contains("宋"));
    if has_serif {
        SERIF
    } else {
        SANS
    }
}

fn paint_wordmark(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect) {
    let mark = Rect::xywh(80.0, 22.0, 18.0, 18.0);
    cx.backend
        .stroke_round_rect(mark, 4.0, label_color(surface), 1.5);
    cx.backend
        .fill_round_rect(Rect::xywh(84.0, 26.0, 8.0, 8.0), 1.0, surface.theme.primary);
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
    cx.backend
        .fill_round_rect(layout.sheet, 14.0, sheet(surface));
    cx.backend
        .stroke_round_rect(layout.sheet, 14.0, line(surface), 1.0);
    for tick in 1..28 {
        let x = layout.sheet.origin.x + tick as f32 * 22.0;
        cx.backend.stroke_line(
            Point2D::new(x, layout.sheet.origin.y),
            Point2D::new(x, layout.sheet.origin.y + 8.0),
            line(surface).with_alpha(0.45),
            1.0,
        );
    }
    let placeholder = surface
        .state
        .bound
        .map_or("先写一句你想做的东西……", HomeFamily::placeholder);
    paint_text_input_view(
        cx,
        &surface.theme,
        &surface.state.input,
        layout.sheet_text,
        16.0,
        4.0,
        layout.sheet_text.origin.y + 20.0,
        surface.now_ms,
        if surface.state.draft.is_empty() {
            placeholder
        } else {
            ""
        },
        surface.state.visible,
    );
    let refs = [
        (layout.screenshot, HomeHit::Attachment, "截图"),
        (layout.reference_link, HomeHit::ReferenceLink, "参考链接"),
        (layout.figma, HomeHit::Figma, "Figma"),
        (layout.example, HomeHit::TryExample, "试试这个示例"),
    ];
    for (rect, hit, label) in refs {
        let disabled = matches!(hit, HomeHit::ReferenceLink | HomeHit::Figma);
        let color = if disabled {
            graphite(surface).with_alpha(0.45)
        } else {
            graphite(surface)
        };
        if surface.state.hover == Some(hit) && !disabled {
            cx.backend
                .fill_round_rect(rect, 8.0, surface.theme.primary.with_alpha(0.08));
        }
        text(
            cx,
            label,
            Point2D::new(rect.origin.x + 4.0, rect.origin.y + 19.0),
            13.0,
            color,
            SANS,
        );
        if disabled && surface.state.hover == Some(hit) {
            let tooltip = Rect::xywh(rect.origin.x, rect.origin.y - 28.0, 68.0, 22.0);
            cx.backend
                .fill_round_rect(tooltip, 7.0, surface.theme.foreground);
            text(
                cx,
                "即将支持",
                Point2D::new(tooltip.origin.x + 10.0, tooltip.origin.y + 15.0),
                11.0,
                paper(surface),
                SANS,
            );
        }
    }
    let send_fill = if surface.state.draft.trim().is_empty() {
        graphite(surface).with_alpha(0.30)
    } else {
        surface.theme.primary
    };
    cx.backend.fill_oval(layout.send, send_fill);
    if surface.state.pressed == Some(HomeHit::Send) {
        cx.backend
            .stroke_oval(layout.send, surface.theme.foreground.with_alpha(0.30), 2.0);
    }
    draw_icon(
        cx.backend,
        Icon::ArrowUp,
        Point2D::new(layout.send.origin.x + 10.0, layout.send.origin.y + 10.0),
        18.0,
        paper(surface),
        1.8,
    );
    text(
        cx,
        "⏎ 发送",
        Point2D::new(layout.send.origin.x - 58.0, layout.send.origin.y + 23.0),
        12.0,
        graphite(surface),
        "SF Mono",
    );
}

fn paint_expected(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, layout: HomeLayout) {
    text(
        cx,
        "预期产物",
        Point2D::new(layout.expected.origin.x, layout.expected.origin.y + 20.0),
        13.0,
        graphite(surface),
        SANS,
    );
    if let Some(family) = surface.state.bound {
        let mut x = layout.expected.origin.x + 62.0;
        for output in family.expected_outputs() {
            let w = output.chars().count() as f32 * 13.0 + 20.0;
            let pill = Rect::xywh(x, layout.expected.origin.y + 1.0, w, 28.0);
            cx.backend
                .fill_round_rect(pill, 14.0, surface.theme.primary.with_alpha(0.18));
            text(
                cx,
                output,
                Point2D::new(x + 10.0, layout.expected.origin.y + 20.0),
                12.0,
                surface.theme.primary,
                SANS,
            );
            x += w + 6.0;
        }
        if surface.state.bound == Some(HomeFamily::AppUi) {
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
            cx.backend.fill_round_rect(inactive, 14.0, sheet(surface));
            cx.backend
                .stroke_round_rect(inactive, 14.0, line(surface), 1.0);
            cx.backend
                .fill_round_rect(active, 14.0, surface.theme.foreground);
            text(
                cx,
                "手机",
                Point2D::new(
                    layout.device_mobile.origin.x + 15.0,
                    layout.device_mobile.origin.y + 19.0,
                ),
                13.0,
                if surface.state.device == HomeDevice::Mobile {
                    paper(surface)
                } else {
                    graphite(surface)
                },
                SANS,
            );
            text(
                cx,
                "桌面",
                Point2D::new(
                    layout.device_desktop.origin.x + 15.0,
                    layout.device_desktop.origin.y + 19.0,
                ),
                13.0,
                if surface.state.device == HomeDevice::Desktop {
                    paper(surface)
                } else {
                    graphite(surface)
                },
                SANS,
            );
        }
    }
}

fn paint_card(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect, family: HomeFamily) {
    let hover = surface.state.hover == Some(HomeHit::Card(family));
    let pressed = surface.state.pressed == Some(HomeHit::Card(family));
    cx.backend.fill_round_rect(rect, 14.0, sheet(surface));
    if hover || pressed {
        cx.backend.fill_round_rect(
            rect,
            14.0,
            surface
                .theme
                .primary
                .with_alpha(if pressed { 0.18 } else { 0.08 }),
        );
    }
    cx.backend.stroke_round_rect(
        rect,
        14.0,
        if surface.state.bound == Some(family) {
            surface.theme.primary
        } else {
            line(surface)
        },
        1.0,
    );
    let art = Rect::xywh(
        rect.origin.x,
        rect.origin.y,
        rect.size.x,
        (rect.size.y - 66.0).max(60.0),
    );
    cx.backend.fill_rect(
        art,
        if surface.ui.effective_theme_mode() == ThemeMode::Light {
            Color::rgb_u8(0xED, 0xE8, 0xDE)
        } else {
            surface.theme.muted
        },
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
    let title_y = rect.origin.y + rect.size.y - 43.0;
    text(
        cx,
        family.label(),
        Point2D::new(rect.origin.x + 14.0, title_y),
        15.0,
        label_color(surface),
        SANS,
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
        Point2D::new(rect.origin.x + 14.0, title_y + 22.0),
        11.5,
        graphite(surface),
        SANS,
    );
}

fn paint_app_flow(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, art: Rect) {
    let phone_w = 48.0;
    let phone_h = (art.size.y - 28.0).min(112.0);
    let y = art.origin.y + (art.size.y - phone_h) / 2.0;
    let start_x = art.origin.x + (art.size.x - phone_w * 3.0 - 48.0) / 2.0;
    for index in 0..3 {
        let x = start_x + index as f32 * (phone_w + 24.0);
        let phone = Rect::xywh(x, y, phone_w, phone_h);
        cx.backend.fill_round_rect(phone, 8.0, sheet(surface));
        cx.backend
            .stroke_round_rect(phone, 8.0, graphite(surface), 1.0);
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 12.0, phone_w - 14.0, 7.0),
            line(surface),
        );
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 28.0, phone_w - 14.0, 6.0),
            line(surface),
        );
        cx.backend.fill_rect(
            Rect::xywh(x + 7.0, y + 42.0, phone_w - 14.0, 6.0),
            line(surface),
        );
        cx.backend.fill_round_rect(
            Rect::xywh(x + 7.0, y + phone_h - 22.0, phone_w - 14.0, 10.0),
            3.0,
            if index == 1 {
                surface.theme.primary
            } else {
                graphite(surface).with_alpha(0.35)
            },
        );
        if index < 2 {
            cx.backend.stroke_line(
                Point2D::new(x + phone_w + 5.0, y + phone_h / 2.0),
                Point2D::new(x + phone_w + 18.0, y + phone_h / 2.0),
                surface.theme.primary,
                1.5,
            );
            cx.backend.stroke_line(
                Point2D::new(x + phone_w + 14.0, y + phone_h / 2.0 - 4.0),
                Point2D::new(x + phone_w + 18.0, y + phone_h / 2.0),
                surface.theme.primary,
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
        cx.backend.fill_rect(rect, surface.theme.muted);
    }
}

fn paint_footer(surface: &HomeSurface<'_>, cx: &mut PaintCx<'_>, layout: HomeLayout) {
    text(
        cx,
        "最近项目",
        Point2D::new(layout.footer.origin.x, layout.footer.origin.y + 16.0),
        13.0,
        graphite(surface),
        SANS,
    );
    text(
        cx,
        "·",
        Point2D::new(layout.footer.origin.x + 72.0, layout.footer.origin.y + 16.0),
        13.0,
        graphite(surface),
        SANS,
    );
    text(
        cx,
        "新建空白画布",
        Point2D::new(layout.footer.origin.x + 88.0, layout.footer.origin.y + 16.0),
        13.0,
        label_color(surface),
        SANS,
    );
    text(
        cx,
        "打开文件",
        Point2D::new(
            layout.footer.origin.x + 190.0,
            layout.footer.origin.y + 16.0,
        ),
        13.0,
        label_color(surface),
        SANS,
    );
}
