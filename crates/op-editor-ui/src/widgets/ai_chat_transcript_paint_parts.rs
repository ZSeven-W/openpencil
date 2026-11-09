use super::ai_chat_transcript::{
    draw_line, ActionStep, Collapsible, ACTION_DETAIL_GAP, ACTION_DETAIL_LINE_H, ACTION_STEP_H,
    BODY_FONT, HEADER_H, LINE_H,
};
use super::ai_chat_transcript_activity::{
    paint_activity_chrome, TranscriptActivityChrome, TranscriptActivityStatus,
};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

/// Extra right-reserve width carved out for a retryable failed row's "Retry"
/// icon, on top of the base reserve (28 without details / 52 with — the
/// expander's own space). Kept separate from the shared
/// `paint_activity_chrome`/`TranscriptActivityChrome` — that chrome is also
/// used by tool-call cards, which never retry.
const RETRY_ICON_EXTRA_RESERVE: f32 = 24.0;
const RETRY_ICON_SIZE: f32 = 14.0;

fn base_right_reserve(step: &ActionStep) -> f32 {
    if step.details.is_empty() {
        28.0
    } else {
        52.0
    }
}

/// The retry icon's paint/hit rect for a retryable failed row — `None` when
/// the row has nothing to retry (see `ActionStep::retryable`). Exposed
/// `pub(crate)` so the transcript hit-test uses the SAME rect the paint
/// below draws, instead of a second hand-derived copy that could drift.
pub(crate) fn retry_icon_rect(step: &ActionStep) -> Option<Rect> {
    if !step.retryable {
        return None;
    }
    let right_reserve = base_right_reserve(step) + RETRY_ICON_EXTRA_RESERVE;
    let x = step.rect.origin.x + step.rect.size.x - right_reserve + 4.0;
    let y = step.rect.origin.y + (ACTION_STEP_H - RETRY_ICON_SIZE) / 2.0;
    Some(Rect::xywh(x, y, RETRY_ICON_SIZE, RETRY_ICON_SIZE))
}

pub(crate) fn paint_action_step(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    step: &ActionStep,
    now_ms: u64,
) {
    let label_color = if step.failed {
        theme.destructive
    } else {
        theme.muted_foreground
    };
    let status = if step.failed {
        TranscriptActivityStatus::Error
    } else if step.done {
        TranscriptActivityStatus::Done
    } else if step.active {
        TranscriptActivityStatus::Running
    } else {
        TranscriptActivityStatus::Pending
    };
    paint_activity_chrome(
        cx,
        theme,
        TranscriptActivityChrome {
            rect: step.rect,
            header: Rect::xywh(
                step.rect.origin.x,
                step.rect.origin.y,
                step.rect.size.x,
                ACTION_STEP_H,
            ),
            label: &step.label,
            label_color,
            background: theme.card,
            border: theme.border,
            status,
            expanded: (!step.details.is_empty()).then_some(step.expanded),
            right_reserve: base_right_reserve(step)
                + if step.retryable {
                    RETRY_ICON_EXTRA_RESERVE
                } else {
                    0.0
                },
        },
        now_ms,
    );
    if let Some(retry_rect) = retry_icon_rect(step) {
        draw_icon(
            cx.backend,
            Icon::RefreshCw,
            retry_rect.origin,
            RETRY_ICON_SIZE,
            theme.muted_foreground,
            1.4,
        );
    }

    if step.expanded && !step.details.is_empty() {
        cx.backend.stroke_line(
            Point2D::new(step.rect.origin.x, step.rect.origin.y + ACTION_STEP_H),
            Point2D::new(
                step.rect.origin.x + step.rect.size.x,
                step.rect.origin.y + ACTION_STEP_H,
            ),
            theme.border,
            1.0,
        );
        cx.backend.save();
        cx.backend.clip_rect(step.rect);
        let mut baseline = step.rect.origin.y + ACTION_STEP_H + ACTION_DETAIL_GAP + 9.0;
        for line in &step.details {
            draw_line(
                cx,
                line,
                step.rect.origin.x + 12.0,
                baseline,
                super::ai_chat_transcript_steps::DETAIL_FONT,
                theme.muted_foreground,
            );
            baseline += ACTION_DETAIL_LINE_H;
        }
        cx.backend.restore();
    }
}

pub(crate) fn paint_collapsible(cx: &mut PaintCx<'_>, theme: &Theme, block: &Collapsible) {
    cx.backend.fill_round_rect(block.header, 6.0, theme.muted);
    let icon = if block.collapsed {
        Icon::ChevronRight
    } else {
        Icon::ChevronDown
    };
    // Right-aligned chevron — consistent with the subtask step cards, whose
    // chevron sits at the right edge (their left side holds the status dot).
    // The label sits at a small left inset.
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            block.header.origin.x + block.header.size.x - 20.0,
            block.header.origin.y + (HEADER_H - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    draw_line(
        cx,
        &block.label,
        block.header.origin.x + 12.0,
        block.header.origin.y + HEADER_H / 2.0 + 4.0,
        11.0,
        theme.muted_foreground,
    );
    if !block.collapsed && !block.lines.is_empty() {
        cx.backend
            .fill_round_rect(block.body, 6.0, theme.background);
        cx.backend.save();
        cx.backend.clip_rect(block.body);
        let mut baseline = block.body.origin.y + super::ai_chat_transcript::BUBBLE_PAD + 11.0;
        for line in &block.lines {
            draw_line(
                cx,
                line,
                block.body.origin.x + super::ai_chat_transcript::BUBBLE_PAD,
                baseline,
                BODY_FONT,
                theme.muted_foreground,
            );
            baseline += LINE_H;
        }
        cx.backend.restore();
    }
}
