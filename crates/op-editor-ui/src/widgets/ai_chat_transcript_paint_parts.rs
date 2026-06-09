use super::ai_chat_transcript::{
    draw_line, ActionStep, Collapsible, ACTION_DETAIL_GAP, ACTION_DETAIL_LINE_H, ACTION_STEP_H,
    BODY_FONT, HEADER_H, LINE_H,
};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

pub(crate) fn paint_action_step(cx: &mut PaintCx<'_>, theme: &Theme, step: &ActionStep) {
    cx.backend.fill_round_rect(step.rect, 6.0, theme.muted);
    cx.backend
        .stroke_round_rect(step.rect, 6.0, theme.border, 1.0);
    let icon_box = Rect::xywh(
        step.rect.origin.x + 10.0,
        step.rect.origin.y + 7.0,
        14.0,
        14.0,
    );
    if step.failed {
        draw_icon(
            cx.backend,
            Icon::XCircle,
            Point2D::new(icon_box.origin.x, icon_box.origin.y),
            14.0,
            theme.destructive,
            1.7,
        );
    } else if step.done {
        draw_icon(
            cx.backend,
            Icon::Check,
            Point2D::new(icon_box.origin.x, icon_box.origin.y),
            14.0,
            theme.primary,
            2.1,
        );
    } else {
        let color = if step.active {
            theme.primary
        } else {
            theme.muted_foreground
        };
        let r = if step.active { 4.0 } else { 3.0 };
        let center = Point2D::new(icon_box.origin.x + 7.0, icon_box.origin.y + 7.0);
        cx.backend.fill_oval(
            Rect::xywh(center.x - r, center.y - r, r * 2.0, r * 2.0),
            color,
        );
    }
    let label_color = if step.active {
        theme.foreground
    } else if step.failed {
        theme.destructive
    } else {
        theme.muted_foreground
    };
    draw_line(
        cx,
        &step.label,
        step.rect.origin.x + 32.0,
        step.rect.origin.y + 18.0,
        11.0,
        label_color,
    );
    draw_icon(
        cx.backend,
        if step.expanded {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        Point2D::new(
            step.rect.origin.x + step.rect.size.x - 20.0,
            step.rect.origin.y + 7.0,
        ),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    if step.expanded && !step.details.is_empty() {
        cx.backend.save();
        cx.backend.clip_rect(step.rect);
        let mut baseline = step.rect.origin.y + ACTION_STEP_H + ACTION_DETAIL_GAP + 9.0;
        for line in &step.details {
            draw_line(
                cx,
                line,
                step.rect.origin.x + 32.0,
                baseline,
                10.0,
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
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(
            block.header.origin.x + 6.0,
            block.header.origin.y + (HEADER_H - 14.0) / 2.0,
        ),
        14.0,
        theme.muted_foreground,
        1.5,
    );
    draw_line(
        cx,
        &block.label,
        block.header.origin.x + 26.0,
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
