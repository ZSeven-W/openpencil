//! Body painter for [`super::ai_chat_panel::AIChatPlaceholder`] — the
//! empty-state example-card grid. The active-transcript painter
//! lives in `ai_chat_transcript.rs`. Split out of `ai_chat_panel.rs`
//! to keep that file under the 800-line cap.

use super::ai_chat_panel::{to_jian_color, ExampleCard, HEADER_HEIGHT, PAD};
use crate::theme::Theme;
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

pub(crate) const EXAMPLE_CARD_GAP: f32 = 8.0;
pub(crate) const EXAMPLE_CARD_HEIGHT: f32 = 72.0;
const EXAMPLE_CARD_PAD: f32 = 12.0;
const EXAMPLE_TITLE_FONT: f32 = 12.0;
const EXAMPLE_SUBTITLE_FONT: f32 = 10.0;
const EXAMPLE_SUBTITLE_LINE_H: f32 = 12.0;

pub(crate) fn example_card_rects(rect: Rect) -> [Rect; 4] {
    let grid_y = rect.origin.y + HEADER_HEIGHT + 32.0;
    let card_w = (rect.size.x - PAD * 2.0 - EXAMPLE_CARD_GAP) / 2.0;
    std::array::from_fn(|i| {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        Rect {
            origin: Point2D::new(
                rect.origin.x + PAD + col * (card_w + EXAMPLE_CARD_GAP),
                grid_y + row * (EXAMPLE_CARD_HEIGHT + EXAMPLE_CARD_GAP),
            ),
            size: Point2D::new(card_w, EXAMPLE_CARD_HEIGHT),
        }
    })
}

fn with_alpha(color: Color, a: f32) -> Color {
    Color { a, ..color }
}

fn wrapped_lines(
    cx: &mut PaintCx<'_>,
    value: &str,
    max_w: f32,
    size: f32,
    max_lines: usize,
) -> Vec<String> {
    if max_lines == 0 || value.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = value.chars().collect();
    let mut lines = Vec::new();
    let mut idx = 0;
    while idx < chars.len() && lines.len() < max_lines {
        while idx < chars.len() && chars[idx].is_whitespace() {
            idx += 1;
        }
        let mut line = String::new();
        while idx < chars.len() {
            let candidate = format!("{}{}", line, chars[idx]);
            if !line.is_empty() && cx.backend.measure_text(&candidate, size) > max_w {
                break;
            }
            line.push(chars[idx]);
            idx += 1;
        }
        if line.is_empty() && idx < chars.len() {
            line.push(chars[idx]);
            idx += 1;
        }
        if !line.is_empty() {
            lines.push(line.trim_end().to_string());
        }
    }
    if idx < chars.len() {
        if let Some(last) = lines.last_mut() {
            while !last.is_empty() && cx.backend.measure_text(&format!("{last}..."), size) > max_w {
                last.pop();
            }
            last.push_str("...");
        }
    }
    lines
}

/// Paint the floating chat panel shell. TS uses
/// `rounded-xl border bg-card/95 shadow-lg backdrop-blur-sm`; Skia here
/// has no blur primitive, so we fake one with two translucent rounded
/// rects behind the card. The near layer carries most of the (light)
/// shadow right at the panel edge; the far layer is very faint so the
/// shadow fades out instead of ending in a hard full-width grey band.
pub(crate) fn paint_panel_surface(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect) {
    let shadow_far = Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y + 8.0),
        size: rect.size,
    };
    let shadow_near = Rect {
        origin: Point2D::new(rect.origin.x, rect.origin.y + 3.0),
        size: rect.size,
    };
    cx.backend
        .fill_round_rect(shadow_far, 14.0, with_alpha(Color::BLACK, 0.02));
    cx.backend
        .fill_round_rect(shadow_near, 14.0, with_alpha(Color::BLACK, 0.06));
    cx.backend
        .fill_round_rect(rect, 14.0, with_alpha(theme.card, 0.95));
    cx.backend.stroke_round_rect(rect, 14.0, theme.border, 1.0);
}

/// Paint the message body's TS-style background and internal dividers.
pub(crate) fn paint_panel_body_chrome(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, sep_y: f32) {
    let inner_x = rect.origin.x + 1.0;
    let inner_w = (rect.size.x - 2.0).max(0.0);
    cx.backend.fill_rect(
        Rect::xywh(inner_x, rect.origin.y + HEADER_HEIGHT, inner_w, 1.0),
        theme.border,
    );
    let body_y = rect.origin.y + HEADER_HEIGHT + 1.0;
    if sep_y > body_y {
        cx.backend.fill_rect(
            Rect::xywh(inner_x, body_y, inner_w, sep_y - body_y),
            with_alpha(theme.background, 0.8),
        );
    }
    cx.backend.fill_rect(
        Rect::xywh(rect.origin.x + PAD, sep_y, rect.size.x - PAD * 2.0, 1.0),
        theme.border,
    );
}

/// Paint the empty-state hint line + the 2×2 example-card grid.
pub(crate) fn paint_examples(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    hint_label: &str,
    tip_label: &str,
    examples: &[ExampleCard; 4],
    disabled: bool,
    hover: Option<usize>,
) {
    let opacity = if disabled { 0.6 } else { 1.0 };
    let hint = TextLayout::single_run(
        hint_label,
        "system-ui",
        12.0,
        to_jian_color(with_alpha(theme.muted_foreground, opacity)),
        Point2D::new(0.0, 0.0),
    );
    let hint_y = rect.origin.y + HEADER_HEIGHT + 16.0;
    let hint_w = cx.backend.measure_text(hint_label, 12.0);
    cx.backend.draw_text(
        &hint,
        Point2D::new(rect.origin.x + (rect.size.x - hint_w) / 2.0, hint_y),
    );

    let card_bg = with_alpha(theme.muted, 0.3 * opacity);
    let card_border = with_alpha(theme.border, opacity);
    let title_color = with_alpha(theme.foreground, opacity);
    let subtitle_color = with_alpha(theme.muted_foreground, opacity);
    for (idx, (card, ex)) in example_card_rects(rect)
        .iter()
        .zip(examples.iter())
        .enumerate()
    {
        cx.backend.fill_round_rect(*card, 8.0, card_bg);
        let hovered = !disabled && hover == Some(idx);
        if hovered {
            cx.backend.fill_round_rect(*card, 8.0, theme.button_hover);
        }
        let border = if hovered {
            with_alpha(theme.primary, 0.35)
        } else {
            card_border
        };
        cx.backend.stroke_round_rect(*card, 8.0, border, 1.0);
        cx.backend.save();
        cx.backend.clip_rect(*card);
        let title_lines = wrapped_lines(
            cx,
            &ex.title,
            card.size.x - EXAMPLE_CARD_PAD * 2.0 - 20.0,
            EXAMPLE_TITLE_FONT,
            2,
        );
        // Vertically center the emoji against the (possibly two-line) title
        // block so a wrapped title doesn't leave the icon stuck at the top.
        let emoji_layout = TextLayout::single_run(
            ex.emoji,
            "system-ui",
            14.0,
            to_jian_color(title_color),
            Point2D::new(0.0, 0.0),
        );
        let emoji_center_offset =
            title_lines.len().saturating_sub(1) as f32 * EXAMPLE_SUBTITLE_LINE_H / 2.0;
        cx.backend.draw_text(
            &emoji_layout,
            Point2D::new(
                card.origin.x + EXAMPLE_CARD_PAD,
                card.origin.y + EXAMPLE_CARD_PAD + 10.0 + emoji_center_offset,
            ),
        );
        for (line_index, line) in title_lines.iter().enumerate() {
            let title_layout = TextLayout::single_run(
                line,
                "system-ui",
                EXAMPLE_TITLE_FONT,
                to_jian_color(title_color),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &title_layout,
                Point2D::new(
                    card.origin.x + EXAMPLE_CARD_PAD + 20.0,
                    card.origin.y
                        + EXAMPLE_CARD_PAD
                        + 10.0
                        + line_index as f32 * EXAMPLE_SUBTITLE_LINE_H,
                ),
            );
        }
        let title_extra = title_lines.len().saturating_sub(1) as f32 * EXAMPLE_SUBTITLE_LINE_H;
        let subtitle_lines = wrapped_lines(
            cx,
            &ex.subtitle,
            card.size.x - EXAMPLE_CARD_PAD * 2.0,
            EXAMPLE_SUBTITLE_FONT,
            2,
        );
        for (line_index, line) in subtitle_lines.iter().enumerate() {
            let subtitle_layout = TextLayout::single_run(
                line,
                "system-ui",
                EXAMPLE_SUBTITLE_FONT,
                to_jian_color(subtitle_color),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &subtitle_layout,
                Point2D::new(
                    card.origin.x + EXAMPLE_CARD_PAD,
                    card.origin.y
                        + EXAMPLE_CARD_PAD
                        + 29.0
                        + title_extra
                        + line_index as f32 * EXAMPLE_SUBTITLE_LINE_H,
                ),
            );
        }
        cx.backend.restore();
    }
    let tip = TextLayout::single_run(
        tip_label,
        "system-ui",
        10.0,
        to_jian_color(with_alpha(theme.muted_foreground, 0.5)),
        Point2D::new(0.0, 0.0),
    );
    let tip_w = cx.backend.measure_text(tip_label, 10.0);
    cx.backend.draw_text(
        &tip,
        Point2D::new(
            rect.origin.x + (rect.size.x - tip_w) / 2.0,
            rect.origin.y
                + HEADER_HEIGHT
                + 32.0
                + EXAMPLE_CARD_HEIGHT * 2.0
                + EXAMPLE_CARD_GAP
                + 22.0,
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[derive(Default)]
    struct CaptureBackend {
        clips: usize,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, _: &TextLayout, _: Point2D) {}
        fn clip_rect(&mut self, _: Rect) {
            self.clips += 1;
        }
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, _: Rect, _: f32, _: Color) {}
        fn stroke_round_rect(&mut self, _: Rect, _: f32, _: Color, _: f32) {}
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
    }

    #[test]
    fn quick_action_card_contents_are_clipped_to_card_bounds() {
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let rect = Rect::xywh(0.0, 0.0, 380.0, 480.0);
        let examples = [
            ExampleCard {
                emoji: "🎨",
                title: "Suggest a color palette for my app".into(),
                subtitle: "Color palette recommendation".into(),
                prompt: "prompt".into(),
            },
            ExampleCard {
                emoji: "⬇️",
                title: "Design a bottom navigation".into(),
                subtitle: "5-tab navigation bar".into(),
                prompt: "prompt".into(),
            },
            ExampleCard {
                emoji: "📱",
                title: "Design a mobile login screen".into(),
                subtitle: "Mobile login with social auth".into(),
                prompt: "prompt".into(),
            },
            ExampleCard {
                emoji: "🍕",
                title: "Food app homepage".into(),
                subtitle: "App homepage design".into(),
                prompt: "prompt".into(),
            },
        ];

        paint_examples(
            &mut cx,
            &Theme::dark(),
            rect,
            "Start designing with AI",
            "Tip",
            &examples,
            false,
            None,
        );

        assert_eq!(
            backend.clips, 4,
            "each quick-action card should clip its own text to avoid bleeding into neighboring cards"
        );
    }
}
