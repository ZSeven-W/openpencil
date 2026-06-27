use super::ai_chat_transcript::{
    draw_line, ActionStep, Collapsible, ACTION_DETAIL_GAP, ACTION_DETAIL_LINE_H, ACTION_STEP_H,
    BODY_FONT, HEADER_H, LINE_H,
};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Point2D, Rect};

pub(crate) fn paint_action_step(cx: &mut PaintCx<'_>, theme: &Theme, step: &ActionStep) {
    // #27 restyle: elevated-dark fill, ~10px radius, subtle 1px border.
    cx.backend.fill_round_rect(step.rect, 10.0, theme.card);
    cx.backend
        .stroke_round_rect(step.rect, 10.0, theme.border, 1.0);

    // Status indicator in the left area, vertically centered.
    let icon_top_left = Point2D::new(
        step.rect.origin.x + 12.0,
        step.rect.origin.y + (ACTION_STEP_H - 14.0) / 2.0,
    );
    if step.failed {
        draw_icon(
            cx.backend,
            Icon::XCircle,
            icon_top_left,
            14.0,
            theme.destructive,
            1.5,
        );
    } else if step.done {
        // #27 restyle: thin success-green ring + check glyph.
        let success_color = theme.status_success;
        let ring_cx = icon_top_left.x + 7.0;
        let ring_cy = icon_top_left.y + 7.0;
        cx.backend.stroke_oval(
            Rect::xywh(ring_cx - 9.0, ring_cy - 9.0, 18.0, 18.0),
            success_color,
            1.0,
        );
        draw_icon(
            cx.backend,
            Icon::Check,
            icon_top_left,
            14.0,
            success_color,
            1.5,
        );
    } else {
        // Running / pending: dot spinner.
        let color = if step.active {
            theme.primary
        } else {
            theme.muted_foreground
        };
        let r = if step.active { 4.0 } else { 3.0 };
        let center = Point2D::new(icon_top_left.x + 7.0, icon_top_left.y + 7.0);
        cx.backend.fill_oval(
            Rect::xywh(center.x - r, center.y - r, r * 2.0, r * 2.0),
            color,
        );
    }

    // Label: starts right of the status icon, truncated before the chevron.
    let label_color = if step.active {
        theme.foreground
    } else if step.failed {
        theme.destructive
    } else {
        theme.muted_foreground
    };
    // Reserve space for status icon (12+14=26) + chevron (24) + small gap.
    let label_x = step.rect.origin.x + 34.0;
    let label_w = (step.rect.size.x - 62.0).max(1.0);
    let label_text = super::layer_panel_paint::truncate_to_fit(&step.label, 11.0, label_w);
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        label_x,
        step.rect.origin.y,
        label_w,
        ACTION_STEP_H,
    ));
    draw_line(
        cx,
        &label_text,
        label_x,
        step.rect.origin.y + ACTION_STEP_H / 2.0 + 4.0,
        11.0,
        label_color,
    );
    cx.backend.restore();

    // Expand/collapse chevron at the far right, vertically centered.
    draw_icon(
        cx.backend,
        if step.expanded {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        },
        Point2D::new(
            step.rect.origin.x + step.rect.size.x - 24.0,
            step.rect.origin.y + (ACTION_STEP_H - 14.0) / 2.0,
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
                step.rect.origin.x + 34.0,
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
