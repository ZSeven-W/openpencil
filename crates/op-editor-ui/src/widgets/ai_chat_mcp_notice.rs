//! The chat panel's pre-flight MCP notice.
//!
//! Sits directly above the input, and only when the selected agent cannot run
//! a canvas turn at all without an MCP integration the user has not switched
//! on (`EditorUiState::chat_agent_mcp_gap`). Before this row existed the same
//! condition only surfaced AFTER the send, as a red `failed to isolate CLI
//! turn` line in the transcript — the user paid a whole written prompt to
//! learn about a toggle.
//!
//! The row is a button, not a banner: clicking it opens Settings on the MCP
//! tab, so the fix is one click from the notice rather than a hunt through
//! the modal.

use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{text_metrics, PaintCx};
use crate::{Color, Point2D, Rect, TextLayout, Theme};

/// Row height when the notice is showing. `0` otherwise — see
/// `AIChatPlaceholder::mcp_notice_row_h`, which mirrors the
/// attachment/chip rows' "no gap when there is nothing to say" rule.
pub(crate) const MCP_NOTICE_ROW_HEIGHT: f32 = 30.0;

const ICON_SIZE: f32 = 14.0;
const PAD_X: f32 = 8.0;
const GAP: f32 = 6.0;
const FONT_SIZE: f32 = 11.0;
const RADIUS: f32 = 6.0;

/// The clickable rect for the notice inside `row`. The whole row is the
/// target — a 30pt-tall strip is already at the small end for a pointer, so
/// shrinking the hit area to the text would make it worse.
pub(crate) fn notice_rect(row: Rect) -> Rect {
    row
}

/// Paint the notice: a warning glyph and the reason, on a surface that reads
/// as raised rather than as an error.
pub(crate) fn paint_mcp_notice(cx: &mut PaintCx<'_>, row: Rect, theme: Theme, label: &str) {
    if row.size.x <= 0.0 || row.size.y <= 0.0 {
        return;
    }
    // A muted surface rather than a destructive one: nothing has failed yet,
    // and painting it red would read as an error the user already caused.
    let fill = mix(theme.background, theme.card, 0.6);
    cx.backend.fill_round_rect(row, RADIUS, fill);
    cx.backend.stroke_round_rect(row, RADIUS, theme.border, 1.0);

    let icon_rect = Rect {
        origin: Point2D::new(
            row.origin.x + PAD_X,
            row.origin.y + (row.size.y - ICON_SIZE) / 2.0,
        ),
        size: Point2D::new(ICON_SIZE, ICON_SIZE),
    };
    let accent = theme.accent;
    draw_icon(
        cx.backend,
        Icon::AlertTriangle,
        icon_rect.origin,
        ICON_SIZE,
        accent,
        1.6,
    );

    let text_x = icon_rect.origin.x + ICON_SIZE + GAP;
    let available = (row.origin.x + row.size.x - PAD_X - text_x).max(0.0);
    let text = text_metrics::fit_chrome(cx.backend, label, available, FONT_SIZE);
    let layout = TextLayout::single_run(
        &text,
        text_metrics::CHROME_FONT_FAMILY,
        FONT_SIZE,
        theme.foreground.with_alpha(0.9).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(
        &layout,
        Point2D::new(
            text_x,
            jian_widgets::centered_text_baseline_y(row, FONT_SIZE),
        ),
    );
}

fn mix(from: Color, to: Color, amount: f32) -> Color {
    Color {
        r: from.r + (to.r - from.r) * amount,
        g: from.g + (to.g - from.g) * amount,
        b: from.b + (to.b - from.b) * amount,
        a: from.a + (to.a - from.a) * amount,
    }
}

#[cfg(test)]
#[path = "ai_chat_mcp_notice_tests.rs"]
mod tests;
