//! Controls + attachment rows for [`super::ai_chat_panel::
//! AIChatPlaceholder`].
//!
//! Two fixed-height strips inside the chat input block:
//!  - the **controls row** — a thinking-mode chip, an effort chip and
//!    an attach button (always present);
//!  - the **attachment row** — one chip per staged file (present only
//!    when `pending_attachments` is non-empty), so every staged
//!    attachment is visible and removable.
//!
//! Paint + hit-test share the layout helpers so a click always lands
//! on the chip the user sees — no text measurement in the hot path.

use super::ai_chat_panel::AIChatHit;
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::chat::{ChatState, EffortLevel, ThinkingMode};

/// Height of the per-turn controls strip.
pub const CONTROLS_ROW_HEIGHT: f32 = 30.0;

/// Height of the staged-attachment strip — shown only when at least
/// one attachment is staged.
pub const ATTACHMENT_ROW_HEIGHT: f32 = 28.0;

const CHIP_H: f32 = 22.0;
const GAP: f32 = 6.0;
const THINKING_CHIP_W: f32 = 92.0;
const EFFORT_CHIP_W: f32 = 78.0;
const ATTACH_BTN_W: f32 = 26.0;
/// Attachment-chip width — sized so the per-turn cap of four chips
/// fits the ~348 px input strip (`4 * 80 + 3 * GAP = 338`).
const ATTACH_CHIP_W: f32 = 80.0;

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

/// Fixed-width rects for the controls row: thinking chip, effort
/// chip, attach button.
struct ControlsLayout {
    thinking: Rect,
    effort: Rect,
    attach: Rect,
}

fn controls_layout(controls_rect: Rect) -> ControlsLayout {
    let y = controls_rect.origin.y + (CONTROLS_ROW_HEIGHT - CHIP_H) / 2.0;
    let mut x = controls_rect.origin.x;
    let thinking = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(THINKING_CHIP_W, CHIP_H),
    };
    x += THINKING_CHIP_W + GAP;
    let effort = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(EFFORT_CHIP_W, CHIP_H),
    };
    x += EFFORT_CHIP_W + GAP;
    let attach = Rect {
        origin: Point2D::new(x, y),
        size: Point2D::new(ATTACH_BTN_W, CHIP_H),
    };
    ControlsLayout {
        thinking,
        effort,
        attach,
    }
}

/// Rects for each staged-attachment chip within the attachment row.
/// A chip that would overflow the row is not laid out — but the row
/// is sized so the per-turn cap of four always fits.
pub(crate) fn attachment_chip_rects(row_rect: Rect, count: usize) -> Vec<Rect> {
    let y = row_rect.origin.y + (ATTACHMENT_ROW_HEIGHT - CHIP_H) / 2.0;
    let right_edge = row_rect.origin.x + row_rect.size.x;
    let mut rects = Vec::with_capacity(count);
    let mut x = row_rect.origin.x;
    for _ in 0..count {
        if x + ATTACH_CHIP_W > right_edge {
            break;
        }
        rects.push(Rect {
            origin: Point2D::new(x, y),
            size: Point2D::new(ATTACH_CHIP_W, CHIP_H),
        });
        x += ATTACH_CHIP_W + GAP;
    }
    rects
}

/// Resolve a click inside the controls strip.
pub fn controls_row_hit(controls_rect: Rect, point: Point2D) -> Option<AIChatHit> {
    let layout = controls_layout(controls_rect);
    if rect_contains(layout.thinking, point) {
        return Some(AIChatHit::CycleThinking);
    }
    if rect_contains(layout.effort, point) {
        return Some(AIChatHit::CycleEffort);
    }
    if rect_contains(layout.attach, point) {
        return Some(AIChatHit::AddAttachment);
    }
    None
}

/// Resolve a click inside the attachment strip — a chip click removes
/// that attachment.
pub fn attachment_row_hit(
    row_rect: Rect,
    point: Point2D,
    attachment_count: usize,
) -> Option<AIChatHit> {
    for (i, chip) in attachment_chip_rects(row_rect, attachment_count)
        .iter()
        .enumerate()
    {
        if rect_contains(*chip, point) {
            return Some(AIChatHit::RemoveAttachment(i));
        }
    }
    None
}

/// Human label for the thinking-mode chip.
fn thinking_label(mode: ThinkingMode) -> &'static str {
    match mode {
        ThinkingMode::Adaptive => "Adaptive",
        ThinkingMode::Disabled => "Disabled",
        ThinkingMode::Enabled => "Enabled",
    }
}

/// Human label for the effort chip.
fn effort_label(level: EffortLevel) -> &'static str {
    match level {
        EffortLevel::Low => "Low",
        EffortLevel::Medium => "Medium",
        EffortLevel::High => "High",
        EffortLevel::Max => "Max",
    }
}

/// Shorten an attachment file name so it fits the fixed chip width.
fn truncate_name(name: &str) -> String {
    let mut chars: Vec<char> = name.chars().collect();
    if chars.len() > 8 {
        chars.truncate(7);
        let mut s: String = chars.into_iter().collect();
        s.push('…');
        s
    } else {
        chars.into_iter().collect()
    }
}

fn paint_chip(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, icon: Icon, label: &str) {
    cx.backend.fill_round_rect(rect, 6.0, theme.muted);
    cx.backend.stroke_round_rect(rect, 6.0, theme.border, 1.0);
    let cy = rect.origin.y + rect.size.y / 2.0;
    draw_icon(
        cx.backend,
        icon,
        Point2D::new(rect.origin.x + 6.0, cy - 6.0),
        12.0,
        theme.muted_foreground,
        1.4,
    );
    let text = TextLayout::single_run(
        label,
        "system-ui",
        11.0,
        to_jian_color(theme.foreground),
        Point2D::new(0.0, 0.0),
    );
    cx.backend
        .draw_text(&text, Point2D::new(rect.origin.x + 22.0, cy + 4.0));
}

/// Paint the controls strip — thinking / effort / attach.
pub fn paint_controls_row(cx: &mut PaintCx<'_>, theme: &Theme, controls_rect: Rect, state: &ChatState) {
    let layout = controls_layout(controls_rect);
    paint_chip(
        cx,
        theme,
        layout.thinking,
        Icon::Sparkles,
        thinking_label(state.thinking_mode),
    );
    paint_chip(
        cx,
        theme,
        layout.effort,
        Icon::Zap,
        effort_label(state.effort_level),
    );
    cx.backend.fill_round_rect(layout.attach, 6.0, theme.muted);
    cx.backend
        .stroke_round_rect(layout.attach, 6.0, theme.border, 1.0);
    draw_icon(
        cx.backend,
        Icon::ImagePlus,
        Point2D::new(
            layout.attach.origin.x + 7.0,
            layout.attach.origin.y + layout.attach.size.y / 2.0 - 6.0,
        ),
        12.0,
        theme.muted_foreground,
        1.4,
    );
}

/// Paint the staged-attachment strip — one chip (name + trailing ✕)
/// per pending attachment.
pub fn paint_attachment_row(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    row_rect: Rect,
    state: &ChatState,
) {
    let rects = attachment_chip_rects(row_rect, state.pending_attachments.len());
    for (chip, att) in rects.iter().zip(state.pending_attachments.iter()) {
        cx.backend.fill_round_rect(*chip, 6.0, theme.muted);
        cx.backend.stroke_round_rect(*chip, 6.0, theme.border, 1.0);
        let cy = chip.origin.y + chip.size.y / 2.0;
        let name = TextLayout::single_run(
            &truncate_name(&att.name),
            "system-ui",
            11.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&name, Point2D::new(chip.origin.x + 8.0, cy + 4.0));
        draw_icon(
            cx.backend,
            Icon::Close,
            Point2D::new(chip.origin.x + chip.size.x - 16.0, cy - 6.0),
            12.0,
            theme.muted_foreground,
            1.4,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(height: f32) -> Rect {
        Rect {
            origin: Point2D::new(20.0, 100.0),
            size: Point2D::new(348.0, height),
        }
    }

    #[test]
    fn hit_thinking_chip() {
        let r = strip(CONTROLS_ROW_HEIGHT);
        let p = Point2D::new(r.origin.x + 10.0, r.origin.y + CONTROLS_ROW_HEIGHT / 2.0);
        assert_eq!(controls_row_hit(r, p), Some(AIChatHit::CycleThinking));
    }

    #[test]
    fn hit_effort_chip() {
        let r = strip(CONTROLS_ROW_HEIGHT);
        let x = r.origin.x + THINKING_CHIP_W + GAP + 10.0;
        let p = Point2D::new(x, r.origin.y + CONTROLS_ROW_HEIGHT / 2.0);
        assert_eq!(controls_row_hit(r, p), Some(AIChatHit::CycleEffort));
    }

    #[test]
    fn hit_attach_button() {
        let r = strip(CONTROLS_ROW_HEIGHT);
        let x = r.origin.x + THINKING_CHIP_W + GAP + EFFORT_CHIP_W + GAP + 8.0;
        let p = Point2D::new(x, r.origin.y + CONTROLS_ROW_HEIGHT / 2.0);
        assert_eq!(controls_row_hit(r, p), Some(AIChatHit::AddAttachment));
    }

    #[test]
    fn controls_row_empty_gap_is_no_hit() {
        let r = strip(CONTROLS_ROW_HEIGHT);
        let p = Point2D::new(r.origin.x + 330.0, r.origin.y + CONTROLS_ROW_HEIGHT / 2.0);
        assert_eq!(controls_row_hit(r, p), None);
    }

    #[test]
    fn all_four_attachment_chips_fit_the_strip() {
        // The per-turn cap is four; every chip must lie within the
        // ~348 px strip so none is hidden / un-removable.
        let r = strip(ATTACHMENT_ROW_HEIGHT);
        let rects = attachment_chip_rects(r, 4);
        assert_eq!(rects.len(), 4);
        for chip in &rects {
            assert!(chip.origin.x + chip.size.x <= r.origin.x + r.size.x);
        }
    }

    #[test]
    fn attachment_row_hit_resolves_chip_index() {
        let r = strip(ATTACHMENT_ROW_HEIGHT);
        let rects = attachment_chip_rects(r, 4);
        // Click the third chip.
        let third = rects[2];
        let p = Point2D::new(
            third.origin.x + 10.0,
            third.origin.y + third.size.y / 2.0,
        );
        assert_eq!(
            attachment_row_hit(r, p, 4),
            Some(AIChatHit::RemoveAttachment(2))
        );
    }

    #[test]
    fn truncate_name_keeps_short_names() {
        assert_eq!(truncate_name("a.png"), "a.png");
        assert_eq!(truncate_name("screenshot.png").chars().count(), 8);
    }
}
