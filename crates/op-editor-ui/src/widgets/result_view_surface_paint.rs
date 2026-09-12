//! Immediate-mode paint pass for the 成品视图 result surface.

use super::{ResultHit, ResultViewSurface, RESULT_BUTTON_HITS};
use crate::widgets::{draw_icon, Icon, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::{HomeFamily, ThemeMode};

const SANS: &str = "system-ui";
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

/// Multiply a color's alpha by `factor` (entrance fades multiply the
/// baked alpha rather than replacing it — the dot grid stays subtle).
fn fade(color: Color, factor: f32) -> Color {
    Color {
        a: color.a * factor,
        ..color
    }
}

/// Same serif resolution order as the Home headline: the first family
/// the host has reported available wins, sans as the fallback.
fn resolve_headline_family(ui: &op_editor_core::EditorUiState) -> &'static str {
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

fn family_label(surface: &ResultViewSurface<'_>) -> &'static str {
    surface
        .state
        .family
        .map(HomeFamily::label)
        .unwrap_or("界面")
}

/// "{N 个界面|N 张}" — App families count screens, the rest count sheets.
fn count_label(family: Option<HomeFamily>, count: usize) -> String {
    if family == Some(HomeFamily::AppUi) {
        format!("{count} 个界面")
    } else {
        format!("{count} 张")
    }
}

/// The breadcrumb's brief segment, capped at 24 characters.
fn truncate_brief(brief: &str) -> String {
    let mut chars = brief.chars();
    match (chars.by_ref().take(24).collect::<String>(), chars.next()) {
        (head, Some(_)) => format!("{head}…"),
        (head, None) => head,
    }
}

fn paint_wordmark(cx: &mut PaintCx<'_>, ink: Color, blue: Color) {
    let mark = Rect::xywh(80.0, 22.0, 18.0, 18.0);
    cx.backend.stroke_round_rect(mark, 4.0, ink, 1.5);
    cx.backend
        .fill_round_rect(Rect::xywh(84.0, 26.0, 8.0, 8.0), 1.0, blue);
    text(cx, "OpenPencil", Point2D::new(108.0, 36.0), 14.0, ink, SANS);
}

#[allow(clippy::too_many_arguments)]
fn paint_button(
    surface: &ResultViewSurface<'_>,
    cx: &mut PaintCx<'_>,
    rect: Rect,
    hit: ResultHit,
    primary: bool,
    label: &str,
    icon: Icon,
    palette: super::HomePalette,
) {
    let hovered = surface.state.hover == Some(hit);
    let pressed = surface.state.pressed == Some(hit);
    if primary {
        cx.backend.fill_round_rect(
            rect,
            10.0,
            if pressed {
                palette.blue_2
            } else {
                palette.blue
            },
        );
    } else {
        cx.backend.fill_round_rect(rect, 10.0, palette.sheet);
        if hovered || pressed {
            cx.backend.fill_round_rect(
                rect,
                10.0,
                if pressed {
                    palette.blue.with_alpha(0.18)
                } else {
                    palette.blue_soft
                },
            );
        }
        cx.backend.stroke_round_rect(rect, 10.0, palette.line, 1.0);
    }
    let fg = if primary { palette.paper } else { palette.ink };
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 16.0, rect.origin.y + 14.0),
        16.0,
        fg,
        1.6,
    );
    text(
        cx,
        label,
        Point2D::new(rect.origin.x + 44.0, rect.origin.y + 28.0),
        14.0,
        fg,
        SANS,
    );
}

pub(super) fn paint_result_view(surface: &ResultViewSurface<'_>, cx: &mut PaintCx<'_>, rect: Rect) {
    let layout = surface.layout(rect.size.x, rect.size.y);
    let palette = super::HomePalette::for_mode(surface.ui.effective_theme_mode());
    cx.backend.fill_rect(rect, palette.paper);

    paint_wordmark(cx, palette.ink, palette.blue);
    let graphite = palette.graphite;
    text(
        cx,
        "专业模式 · 直接进画布 →",
        Point2D::new(
            layout.professional.origin.x,
            layout.professional.origin.y + 19.0,
        ),
        13.0,
        graphite,
        SANS,
    );
    if surface.state.hover == Some(ResultHit::Professional) {
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

    // Breadcrumb: "← 制图台 · {family} · {brief ≤ 24 chars}".
    let back_hovered = surface.state.hover == Some(ResultHit::BackHome);
    let back_color = if back_hovered { palette.blue } else { graphite };
    text(
        cx,
        "← 制图台",
        Point2D::new(
            layout.breadcrumb_back.origin.x,
            layout.breadcrumb_back.origin.y + 16.0,
        ),
        13.0,
        back_color,
        SANS,
    );
    if back_hovered {
        cx.backend.stroke_line(
            Point2D::new(
                layout.breadcrumb_back.origin.x,
                layout.breadcrumb_back.origin.y + 21.0,
            ),
            Point2D::new(
                layout.breadcrumb_back.origin.x + 62.0,
                layout.breadcrumb_back.origin.y + 21.0,
            ),
            palette.blue,
            1.0,
        );
    }
    let crumb = format!(
        " · {} · {}",
        family_label(surface),
        truncate_brief(&surface.state.brief)
    );
    text(
        cx,
        &crumb,
        Point2D::new(
            layout.breadcrumb_back.origin.x + 70.0,
            layout.breadcrumb_back.origin.y + 16.0,
        ),
        13.0,
        graphite,
        SANS,
    );

    // Title: "你的 {family}" serif ink + the ash suffix on one baseline.
    let serif = resolve_headline_family(surface.ui);
    let title = format!("你的 {}", family_label(surface));
    let suffix = format!(
        " · {} · 可编辑",
        count_label(surface.state.family, surface.boards.len())
    );
    text_weighted(
        cx,
        &title,
        Point2D::new(super::RESULT_MARGIN_X, super::TITLE_BASELINE),
        34.0,
        palette.ink,
        serif,
        600,
    );
    let title_w = cx.backend.measure_text_family(&title, 34.0, serif);
    text(
        cx,
        &suffix,
        Point2D::new(super::RESULT_MARGIN_X + title_w, super::TITLE_BASELINE),
        34.0,
        palette.ash,
        serif,
    );

    // Stage panel with the drafting dot grid.
    cx.backend
        .fill_round_rect(layout.stage, super::STAGE_RADIUS, palette.paper_2);
    cx.backend
        .stroke_round_rect(layout.stage, super::STAGE_RADIUS, palette.line, 1.0);
    let dots = palette.dots;
    let mut dot_x = layout.stage.origin.x + 12.0;
    while dot_x < layout.stage.origin.x + layout.stage.size.x - 6.0 {
        let mut dot_y = layout.stage.origin.y + 12.0;
        while dot_y < layout.stage.origin.y + layout.stage.size.y - 6.0 {
            cx.backend
                .fill_oval(Rect::xywh(dot_x - 0.75, dot_y - 0.75, 1.5, 1.5), dots);
            dot_y += 24.0;
        }
        dot_x += 24.0;
    }

    // Boards: sheet placeholder slots; the native host blits the real
    // rasters over them. Hover lifts, selection rings blue.
    for index in 0..layout.screens.len() {
        let Some(slot) = surface.screen_draw_rect(&layout, index) else {
            continue;
        };
        let (_, alpha) = super::board_enter(index, surface.state.shown_at_ms, surface.now_ms);
        if alpha <= 0.0 {
            continue;
        }
        let selected = surface.state.selected == index;
        let pressed = surface.state.pressed == Some(ResultHit::Screen(index));
        if surface.ui.effective_theme_mode() == ThemeMode::Light && alpha > 0.95 {
            cx.backend.fill_drop_shadow(
                Rect::xywh(
                    slot.origin.x - 2.0,
                    slot.origin.y + 2.0,
                    slot.size.x + 4.0,
                    slot.size.y + 4.0,
                ),
                super::BOARD_RADIUS,
                8.0,
                Color::BLACK.with_alpha(0.10),
            );
        }
        cx.backend
            .fill_round_rect(slot, super::BOARD_RADIUS, fade(palette.sheet, alpha));
        if selected {
            cx.backend
                .stroke_round_rect(slot, super::BOARD_RADIUS, fade(palette.blue, alpha), 1.0);
        } else if pressed {
            cx.backend.stroke_round_rect(
                slot,
                super::BOARD_RADIUS,
                fade(palette.blue.with_alpha(0.45), alpha),
                1.0,
            );
        } else {
            cx.backend
                .stroke_round_rect(slot, super::BOARD_RADIUS, fade(palette.line, alpha), 1.0);
        }
        // Caption: the board's own name, centered under the slot.
        if let Some(caption) = layout.captions.get(index) {
            if let Some(board) = surface.boards.get(index) {
                let label_w = cx.backend.measure_text_family(&board.name, 12.0, SANS);
                text(
                    cx,
                    &board.name,
                    Point2D::new(
                        caption.origin.x + (caption.size.x - label_w) / 2.0,
                        caption.origin.y + 14.0,
                    ),
                    12.0,
                    fade(palette.ash, alpha),
                    SANS,
                );
            }
        }
    }
    // Thin "→" connectors between consecutive boards.
    for pair in layout.screens.windows(2) {
        let gap_center = (pair[0].origin.x + pair[0].size.x + pair[1].origin.x) / 2.0;
        let arrow_w = cx.backend.measure_text_family("→", 13.0, SANS);
        text(
            cx,
            "→",
            Point2D::new(
                gap_center - arrow_w / 2.0,
                pair[0].origin.y + pair[0].size.y / 2.0 + 4.0,
            ),
            13.0,
            graphite,
            SANS,
        );
    }

    // Right panel: slides in from +16 px over its own window.
    let panel = {
        let mut panel = layout.panel;
        panel.origin.y += super::panel_enter(surface.state.shown_at_ms, surface.now_ms);
        panel
    };
    if surface.ui.effective_theme_mode() == ThemeMode::Light {
        cx.backend.fill_drop_shadow(
            Rect::xywh(
                panel.origin.x - 2.0,
                panel.origin.y + 2.0,
                panel.size.x + 4.0,
                panel.size.y + 4.0,
            ),
            super::PANEL_RADIUS,
            10.0,
            Color::BLACK.with_alpha(0.08),
        );
    }
    cx.backend
        .fill_round_rect(panel, super::PANEL_RADIUS, palette.sheet);
    cx.backend
        .stroke_round_rect(panel, super::PANEL_RADIUS, palette.line, 1.0);
    text_weighted(
        cx,
        "接下来",
        Point2D::new(
            panel.origin.x + super::PANEL_PAD,
            panel.origin.y + super::PANEL_TITLE_BASELINE,
        ),
        20.0,
        palette.ink,
        serif,
        600,
    );
    let hint = "这是可编辑的设计稿，不是截图：组件、变量、图层都在。先改一处，再试点或导出。";
    let hint_lines: Vec<String> = hint
        .chars()
        .collect::<Vec<_>>()
        .chunks(super::HINT_CHARS_PER_LINE)
        .map(|chunk| chunk.iter().collect())
        .collect();
    for (line, chunk) in hint_lines.iter().enumerate() {
        text(
            cx,
            chunk,
            Point2D::new(
                panel.origin.x + super::PANEL_PAD,
                panel.origin.y
                    + super::PANEL_TITLE_BASELINE
                    + super::HINT_LINE_H
                    + line as f32 * super::HINT_LINE_H
                    + 14.0,
            ),
            13.0,
            graphite,
            SANS,
        );
    }

    let labels = [
        ("改这一页", Icon::Pen),
        ("试点原型（Play）", Icon::Play),
        ("组件与变量", Icon::LayoutGrid),
        ("换风格", Icon::Rows3),
        ("导出 PNG / 代码", Icon::Download),
        ("完整编辑", Icon::LayoutDashboard),
    ];
    for (index, (label, icon)) in labels.into_iter().enumerate() {
        let mut button = layout.buttons[index];
        button.origin.y += panel.origin.y - layout.panel.origin.y;
        paint_button(
            surface,
            cx,
            button,
            RESULT_BUTTON_HITS[index],
            index == 0,
            label,
            icon,
            palette,
        );
    }
    text(
        cx,
        "刚刚生成 · 本地草稿",
        Point2D::new(
            panel.origin.x + super::PANEL_PAD,
            panel.origin.y + panel.size.y - 16.0,
        ),
        12.0,
        palette.ash,
        SANS,
    );
}
