use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

pub(crate) const COMPLETION_CARD_H: f32 = 48.0;
pub(crate) const COMPLETION_CARD_MAX_W: f32 = 248.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletionSummary {
    pub succeeded: u32,
    pub failed: u32,
    pub nodes: u32,
}

impl CompletionSummary {
    pub(crate) fn title(self) -> &'static str {
        if self.failed == 0 {
            "Done"
        } else {
            "Finished with issues"
        }
    }

    pub(crate) fn detail(self) -> String {
        if self.failed == 0 {
            format!("{} subtasks · {} nodes", self.succeeded, self.nodes)
        } else {
            format!(
                "{} succeeded · {} failed · {} nodes",
                self.succeeded, self.failed, self.nodes
            )
        }
    }
}

pub(crate) fn parse_completion_summary(text: &str) -> Option<CompletionSummary> {
    let trimmed = text.trim();
    if !trimmed.starts_with("Done")
        || !trimmed.contains("subtask(s) succeeded")
        || !trimmed.contains("node(s) total")
    {
        return None;
    }
    let numbers: Vec<u32> = trimmed
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse::<u32>().ok())
        .collect();
    if numbers.len() < 3 {
        return None;
    }
    Some(CompletionSummary {
        succeeded: numbers[0],
        failed: numbers[1],
        nodes: numbers[2],
    })
}

pub(crate) fn completion_card_rect(x: f32, y: f32, width: f32) -> Rect {
    Rect::xywh(x, y, width.min(COMPLETION_CARD_MAX_W), COMPLETION_CARD_H)
}

pub(crate) fn paint_completion_card(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    summary: CompletionSummary,
) {
    let tint = if summary.failed == 0 {
        theme.primary
    } else {
        theme.destructive
    };
    let bg = tinted_surface(theme, tint);
    cx.backend.fill_round_rect(rect, 10.0, bg);
    cx.backend
        .stroke_round_rect(rect, 10.0, tinted_border(tint), 1.0);

    let icon_box = Rect::xywh(rect.origin.x + 12.0, rect.origin.y + 12.0, 24.0, 24.0);
    cx.backend.fill_round_rect(icon_box, 7.0, tint);
    draw_icon(
        cx.backend,
        if summary.failed == 0 {
            Icon::Check
        } else {
            Icon::AlertTriangle
        },
        Point2D::new(icon_box.origin.x + 6.0, icon_box.origin.y + 6.0),
        12.0,
        theme.primary_foreground,
        1.9,
    );

    draw_text(
        cx,
        summary.title(),
        12.0,
        theme.foreground,
        rect.origin.x + 46.0,
        rect.origin.y + 20.0,
    );
    draw_text(
        cx,
        &summary.detail(),
        10.0,
        theme.muted_foreground,
        rect.origin.x + 46.0,
        rect.origin.y + 35.0,
    );
}

fn tinted_surface(theme: &Theme, tint: Color) -> Color {
    Color {
        r: theme.card.r * 0.86 + tint.r * 0.14,
        g: theme.card.g * 0.86 + tint.g * 0.14,
        b: theme.card.b * 0.86 + tint.b * 0.14,
        a: 1.0,
    }
}

fn tinted_border(tint: Color) -> Color {
    Color {
        r: tint.r,
        g: tint.g,
        b: tint.b,
        a: 0.24,
    }
}

fn draw_text(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, baseline_y: f32) {
    let layout = TextLayout::single_run(
        text,
        "system-ui",
        size,
        (color).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&layout, Point2D::new(x, baseline_y));
}
