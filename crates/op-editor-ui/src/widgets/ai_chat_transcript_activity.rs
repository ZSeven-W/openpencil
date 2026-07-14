//! Shared status primitive for every transcript activity card.
//!
//! CLI-backed design activities and built-in tool calls have different
//! backend payloads, but their user-facing lifecycle is identical. Both
//! adapters paint through this module so loading, success, and failure cannot
//! drift visually again.

use super::ai_chat_transcript::draw_line;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, draw_spinning_loader, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect};

const CARD_RADIUS: f32 = 6.0;
const LABEL_X: f32 = 12.0;
const STATUS_GLYPH: f32 = 11.0;
const EXPANDER_GLYPH: f32 = 11.0;
const EXPANDER_RIGHT_OFFSET: f32 = 24.0;
const EXPANDER_NUDGE: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TranscriptActivityStatus {
    Pending,
    Running,
    Done,
    Error,
}

/// Shared visual chrome for CLI activities and built-in tool calls. Their
/// payload bodies remain backend-specific, while surface, label, lifecycle,
/// and disclosure affordance are painted once here.
pub(crate) struct TranscriptActivityChrome<'a> {
    pub rect: Rect,
    pub header: Rect,
    pub label: &'a str,
    pub label_color: Color,
    pub background: Color,
    pub border: Color,
    pub status: TranscriptActivityStatus,
    /// `None` means this row has no disclosure body.
    pub expanded: Option<bool>,
    /// Space kept clear for the status, expander, and optional source badge.
    pub right_reserve: f32,
}

pub(crate) fn paint_activity_chrome(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    chrome: TranscriptActivityChrome<'_>,
    now_ms: u64,
) {
    cx.backend
        .fill_round_rect(chrome.rect, CARD_RADIUS, chrome.background);
    cx.backend
        .stroke_round_rect(chrome.rect, CARD_RADIUS, chrome.border, 1.0);

    let label_x = chrome.header.origin.x + LABEL_X;
    let label_w = (chrome.header.size.x - LABEL_X - chrome.right_reserve).max(1.0);
    let label = super::layer_panel_paint::truncate_to_fit(chrome.label, 11.0, label_w);
    cx.backend.save();
    cx.backend.clip_rect(Rect::xywh(
        label_x,
        chrome.header.origin.y,
        label_w,
        chrome.header.size.y,
    ));
    draw_line(
        cx,
        &label,
        label_x,
        chrome.header.origin.y + chrome.header.size.y / 2.0 + 4.0,
        11.0,
        chrome.label_color,
    );
    cx.backend.restore();

    let label_units: u32 = label
        .chars()
        .map(super::ai_chat_transcript_text::char_display_units)
        .sum();
    let status_limit = chrome.header.origin.x + chrome.header.size.x - chrome.right_reserve;
    let status_x = (label_x + label_units as f32 * 6.0 + 8.0).min(status_limit);
    paint_activity_status(
        cx,
        theme,
        Point2D::new(
            status_x,
            chrome.header.origin.y + (chrome.header.size.y - STATUS_GLYPH) / 2.0,
        ),
        STATUS_GLYPH,
        chrome.status,
        now_ms,
    );

    let Some(expanded) = chrome.expanded else {
        return;
    };
    let expander_x = chrome.header.origin.x + chrome.header.size.x - EXPANDER_RIGHT_OFFSET;
    let center_y = chrome.header.origin.y + chrome.header.size.y / 2.0;
    if expanded {
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(expander_x, center_y - EXPANDER_GLYPH / 2.0),
            EXPANDER_GLYPH,
            theme.muted_foreground,
            1.4,
        );
    } else {
        draw_icon(
            cx.backend,
            Icon::ChevronUp,
            Point2D::new(expander_x, center_y - EXPANDER_GLYPH + EXPANDER_NUDGE),
            EXPANDER_GLYPH,
            theme.muted_foreground,
            1.4,
        );
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(expander_x, center_y - EXPANDER_NUDGE),
            EXPANDER_GLYPH,
            theme.muted_foreground,
            1.4,
        );
    }
}

pub(crate) fn paint_activity_status(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    top_left: Point2D,
    size: f32,
    status: TranscriptActivityStatus,
    now_ms: u64,
) {
    match status {
        TranscriptActivityStatus::Pending => {
            let mut color = theme.muted_foreground;
            color.a *= 0.45;
            cx.backend
                .stroke_oval(Rect::xywh(top_left.x, top_left.y, size, size), color, 1.0);
        }
        TranscriptActivityStatus::Running => draw_spinning_loader(
            cx.backend,
            top_left,
            size,
            theme.muted_foreground,
            1.3,
            now_ms,
        ),
        TranscriptActivityStatus::Error => draw_icon(
            cx.backend,
            Icon::Close,
            top_left,
            size,
            theme.destructive,
            1.3,
        ),
        TranscriptActivityStatus::Done => {
            let center_x = top_left.x + size / 2.0;
            let center_y = top_left.y + size / 2.0;
            let radius = size / 2.0 + 1.5;
            cx.backend.stroke_oval(
                Rect::xywh(
                    center_x - radius,
                    center_y - radius,
                    radius * 2.0,
                    radius * 2.0,
                ),
                theme.status_success,
                1.0,
            );
            draw_icon(
                cx.backend,
                Icon::Check,
                top_left,
                size,
                theme.status_success,
                1.3,
            );
        }
    }
}
