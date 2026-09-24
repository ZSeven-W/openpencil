//! Paint for the workspace's quality report: the header chip (painted with
//! the header) and the expanded panel (painted as the topmost workspace
//! layer, over the canvas and the docked chat).
//!
//! Every rect comes from `workspace_quality.rs` through the surface — the
//! same rows the hit-test walks.

use super::{StudioPalette, WorkspaceLayout, WorkspaceSurface};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::workspace_quality::{
    QualityRowKind, QUALITY_CHIP_DOT, QUALITY_CHIP_FONT, QUALITY_CHIP_PAD_X,
};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};
use op_editor_core::{QualityReport, WorkspaceHit};

const SANS: &str = "system-ui";
/// "Needs attention" amber — legible on both the light and the dark panel.
const ATTENTION: Color = Color::rgb_u8(0xE0, 0x8A, 0x12);

fn text(cx: &mut PaintCx<'_>, content: &str, origin: Point2D, size: f32, color: Color) {
    let layout = crate::TextLayout::single_run(content, SANS, size, color.to_jian(), Point2D::ZERO);
    cx.backend.draw_text(&layout, origin);
}

fn text_weighted(cx: &mut PaintCx<'_>, content: &str, origin: Point2D, size: f32, color: Color) {
    let layout = crate::TextLayout::single_run(content, SANS, size, color.to_jian(), Point2D::ZERO)
        .with_font_weight(600);
    cx.backend.draw_text(&layout, origin);
}

/// Trim `value` with an ellipsis until it measures within `max_w`.
fn ellipsize(cx: &mut PaintCx<'_>, value: &str, max_w: f32, size: f32) -> String {
    if cx.backend.measure_text_family(value, size, SANS) <= max_w {
        return value.to_string();
    }
    let mut out: String = value.to_string();
    while !out.is_empty()
        && cx
            .backend
            .measure_text_family(&format!("{out}…"), size, SANS)
            > max_w
    {
        out.pop();
    }
    format!("{out}…")
}

fn tr(locale: op_i18n::Locale, key: &'static str) -> &'static str {
    op_i18n::translate(locale, key)
}

/// The header chip: a status dot (amber when something needs attention,
/// green when nothing does) and the counts.
pub(super) fn paint_quality_chip(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let (Some(chip), Some(label), Some(report)) = (
        surface.quality_chip(layout),
        surface.quality_label(),
        surface.state.finished_quality(),
    ) else {
        return;
    };
    let hovered = surface.state.hover == Some(WorkspaceHit::QualityChip);
    let open = surface.state.quality_open;
    cx.backend.fill_round_rect(
        chip,
        chip.size.y / 2.0,
        if open || hovered {
            palette.button_hover
        } else {
            palette.chip_bg
        },
    );
    cx.backend.stroke_round_rect(
        chip,
        chip.size.y / 2.0,
        if open || hovered {
            palette.button_hover_line
        } else {
            palette.chip_line
        },
        1.0,
    );
    let dot = Rect::xywh(
        chip.origin.x + QUALITY_CHIP_PAD_X,
        chip.origin.y + (chip.size.y - QUALITY_CHIP_DOT) / 2.0,
        QUALITY_CHIP_DOT,
        QUALITY_CHIP_DOT,
    );
    let dot_color = if report.total_remaining() > 0 {
        ATTENTION
    } else {
        palette.status_green
    };
    cx.backend
        .fill_round_rect(dot, QUALITY_CHIP_DOT / 2.0, dot_color);
    let label_x = dot.origin.x + QUALITY_CHIP_DOT + 8.0;
    let max_w = chip.origin.x + chip.size.x - QUALITY_CHIP_PAD_X - label_x;
    let label = ellipsize(cx, &label, max_w, QUALITY_CHIP_FONT);
    text(
        cx,
        &label,
        Point2D::new(
            label_x,
            jian_widgets::centered_text_baseline_y(chip, QUALITY_CHIP_FONT),
        ),
        QUALITY_CHIP_FONT,
        palette.ink,
    );
}

/// The expanded panel. No-op unless the chip is open.
pub(super) fn paint_quality_panel(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    layout: &WorkspaceLayout,
    palette: StudioPalette,
) {
    let (Some(panel), Some(report)) = (
        surface.quality_panel(layout),
        surface.state.finished_quality(),
    ) else {
        return;
    };
    let locale = surface.ui.locale;
    // A soft drop shadow, then the raised card.
    let shadow = Rect::xywh(
        panel.panel.origin.x,
        panel.panel.origin.y + 4.0,
        panel.panel.size.x,
        panel.panel.size.y,
    );
    cx.backend
        .fill_round_rect(shadow, 12.0, Color::rgba_u8(0, 0, 0, 0.10));
    cx.backend
        .fill_round_rect(panel.panel, 12.0, palette.raised);
    cx.backend
        .stroke_round_rect(panel.panel, 12.0, palette.raised_line, 1.0);

    text_weighted(
        cx,
        tr(locale, "workspace.quality.title"),
        Point2D::new(panel.head.origin.x, panel.head.origin.y + 26.0),
        14.0,
        palette.ink,
    );
    let summary = ellipsize(cx, &report.summary_text(locale), panel.head.size.x, 12.0);
    text(
        cx,
        &summary,
        Point2D::new(panel.head.origin.x, panel.head.origin.y + 45.0),
        12.0,
        palette.muted,
    );

    for row in &panel.rows {
        paint_row(surface, cx, row.kind, row.rect, report, palette);
    }
    if panel.truncated {
        let last = panel.rows.last().map_or(panel.head, |row| row.rect);
        text(
            cx,
            "…",
            Point2D::new(last.origin.x, last.origin.y + last.size.y + 10.0),
            12.0,
            palette.muted,
        );
    }
}

fn paint_row(
    surface: &WorkspaceSurface<'_>,
    cx: &mut PaintCx<'_>,
    kind: QualityRowKind,
    rect: Rect,
    report: &QualityReport,
    palette: StudioPalette,
) {
    let locale = surface.ui.locale;
    let baseline = |size: f32| jian_widgets::centered_text_baseline_y(rect, size);
    match kind {
        QualityRowKind::Topic { topic } => {
            let Some(entry) = report.topics.get(topic) else {
                return;
            };
            cx.backend.stroke_line(
                Point2D::new(rect.origin.x, rect.origin.y + 2.0),
                Point2D::new(rect.origin.x + rect.size.x, rect.origin.y + 2.0),
                palette.line,
                1.0,
            );
            text_weighted(
                cx,
                tr(locale, entry.topic.label_key()),
                Point2D::new(rect.origin.x, baseline(13.0) + 2.0),
                13.0,
                palette.ink,
            );
            let (counts, color) = if entry.found_count() == 0 {
                (
                    tr(locale, "workspace.quality.passed").to_string(),
                    palette.status_green,
                )
            } else {
                (
                    tr(locale, "workspace.quality.topicCounts")
                        .replace("{{fixed}}", &entry.fixed_count().to_string())
                        .replace("{{remaining}}", &entry.remaining.len().to_string()),
                    if entry.remaining.is_empty() {
                        palette.muted
                    } else {
                        ATTENTION
                    },
                )
            };
            let w = cx.backend.measure_text_family(&counts, 12.0, SANS);
            text(
                cx,
                &counts,
                Point2D::new(rect.origin.x + rect.size.x - w, baseline(12.0) + 2.0),
                12.0,
                color,
            );
        }
        QualityRowKind::Remaining { topic, item } => {
            let Some(entry) = report.remaining_item(topic, item) else {
                return;
            };
            let hovered = surface.state.hover == Some(WorkspaceHit::QualityItem { topic, item });
            if hovered && entry.node_id.is_some() {
                cx.backend.fill_round_rect(rect, 6.0, palette.button_hover);
            }
            let dot = Rect::xywh(
                rect.origin.x + 4.0,
                rect.origin.y + rect.size.y / 2.0 - 3.0,
                6.0,
                6.0,
            );
            cx.backend.fill_round_rect(dot, 3.0, ATTENTION);
            let full = entry.localized_label(surface.ui.locale, false);
            let label = ellipsize(cx, &full, rect.size.x - 24.0, 12.0);
            text(
                cx,
                &label,
                Point2D::new(rect.origin.x + 18.0, baseline(12.0)),
                12.0,
                if entry.node_id.is_some() {
                    palette.link
                } else {
                    palette.ink
                },
            );
        }
        QualityRowKind::Fixed { topic, item } => {
            let Some(entry) = report
                .topics
                .get(topic)
                .and_then(|entry| entry.fixed.get(item))
            else {
                return;
            };
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(
                    rect.origin.x + 1.0,
                    rect.origin.y + (rect.size.y - 12.0) / 2.0,
                ),
                12.0,
                palette.status_green,
                1.8,
            );
            let full = entry.localized_label(surface.ui.locale, true);
            let label = ellipsize(cx, &full, rect.size.x - 24.0, 12.0);
            text(
                cx,
                &label,
                Point2D::new(rect.origin.x + 18.0, baseline(12.0)),
                12.0,
                palette.sub,
            );
        }
        QualityRowKind::CountOnly { topic } => {
            let Some(entry) = report.topics.get(topic) else {
                return;
            };
            draw_icon(
                cx.backend,
                Icon::Check,
                Point2D::new(
                    rect.origin.x + 1.0,
                    rect.origin.y + (rect.size.y - 12.0) / 2.0,
                ),
                12.0,
                palette.status_green,
                1.8,
            );
            let label = tr(locale, "workspace.quality.countOnly")
                .replace("{{count}}", &entry.fixed_without_detail.to_string());
            text(
                cx,
                &label,
                Point2D::new(rect.origin.x + 18.0, baseline(12.0)),
                12.0,
                palette.sub,
            );
        }
        QualityRowKind::More { count } => {
            let label =
                tr(locale, "workspace.quality.more").replace("{{count}}", &count.to_string());
            text(
                cx,
                &label,
                Point2D::new(rect.origin.x + 18.0, baseline(11.0)),
                11.0,
                palette.muted,
            );
        }
        QualityRowKind::BoardsHeader => {
            cx.backend.stroke_line(
                Point2D::new(rect.origin.x, rect.origin.y + 2.0),
                Point2D::new(rect.origin.x + rect.size.x, rect.origin.y + 2.0),
                palette.line,
                1.0,
            );
            text_weighted(
                cx,
                tr(locale, "workspace.quality.boards"),
                Point2D::new(rect.origin.x, baseline(13.0) + 2.0),
                13.0,
                palette.ink,
            );
        }
        QualityRowKind::Board { index } => {
            let Some(board) = report.boards.get(index) else {
                return;
            };
            let counts = tr(locale, "workspace.quality.topicCounts")
                .replace("{{fixed}}", &board.fixed.to_string())
                .replace("{{remaining}}", &board.remaining.to_string());
            let counts_w = cx.backend.measure_text_family(&counts, 12.0, SANS);
            let name = ellipsize(cx, &board.board_name, rect.size.x - counts_w - 16.0, 12.0);
            text(
                cx,
                &name,
                Point2D::new(rect.origin.x, baseline(12.0)),
                12.0,
                palette.sub,
            );
            text(
                cx,
                &counts,
                Point2D::new(rect.origin.x + rect.size.x - counts_w, baseline(12.0)),
                12.0,
                if board.remaining > 0 {
                    ATTENTION
                } else {
                    palette.muted
                },
            );
        }
    }
}
