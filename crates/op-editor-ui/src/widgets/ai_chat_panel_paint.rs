//! Body painter for [`super::ai_chat_panel::AIChatPlaceholder`] — the
//! empty-state pill stack. The active-transcript painter
//! lives in `ai_chat_transcript.rs`. Split out of `ai_chat_panel.rs`
//! to keep that file under the 800-line cap.

use super::ai_chat_panel::{ExampleCard, HEADER_HEIGHT, PAD};
use crate::theme::Theme;
use crate::widgets::button::paint_button_feedback_wash;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::PaintCx;
use crate::{Color, Point2D, Rect, TextLayout};

/// Gap between stacked example pills.
pub(crate) const EXAMPLE_CARD_GAP: f32 = 8.0;
/// Height of each example pill (~40px, compact rounded / stadium).
pub(crate) const EXAMPLE_CARD_HEIGHT: f32 = 40.0;
/// Corner radius for each pill — large enough to look like a stadium.
const EXAMPLE_PILL_RADIUS: f32 = 14.0;
/// Left padding inside each pill for the prompt text.
const EXAMPLE_PILL_TEXT_PAD: f32 = 16.0;
const EXAMPLE_TITLE_FONT: f32 = 13.0;
/// Top offset (below the header) where the hint line sits.
const HINT_OFFSET: f32 = 20.0;
/// Gap between the hint line and the first pill.
const HINT_PILL_GAP: f32 = 14.0;
/// Gap between the last pill and the tip lines.
const TIP_TOP_GAP: f32 = 14.0;
/// Line height for tip text rows.
const TIP_LINE_H: f32 = 16.0;

/// Returns the four pill rects, stacked full-width below the hint line.
pub(crate) fn example_card_rects(rect: Rect) -> [Rect; 4] {
    // First pill starts below header + hint line + gap.
    let first_pill_y = rect.origin.y + HEADER_HEIGHT + HINT_OFFSET + 16.0 + HINT_PILL_GAP;
    let pill_w = rect.size.x - PAD * 2.0;
    std::array::from_fn(|i| Rect {
        origin: Point2D::new(
            rect.origin.x + PAD,
            first_pill_y + i as f32 * (EXAMPLE_CARD_HEIGHT + EXAMPLE_CARD_GAP),
        ),
        size: Point2D::new(pill_w, EXAMPLE_CARD_HEIGHT),
    })
}

/// Ellipsize `text` so it fits within `max_w` at the given font size.
fn ellipsize(cx: &mut PaintCx<'_>, text: &str, max_w: f32, size: f32) -> String {
    if cx.backend.measure_text(text, size) <= max_w {
        return text.to_string();
    }
    let mut s = text.to_string();
    while !s.is_empty() && cx.backend.measure_text(&format!("{s}…"), size) > max_w {
        s.pop();
    }
    format!("{s}…")
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
        .fill_round_rect(shadow_far, 14.0, (Color::BLACK).with_alpha(0.025));
    cx.backend
        .fill_round_rect(shadow_near, 14.0, (Color::BLACK).with_alpha(0.07));
    cx.backend
        .fill_round_rect(rect, 14.0, (theme.card).with_alpha(0.98));
    cx.backend
        .stroke_round_rect(rect, 14.0, (theme.border).with_alpha(0.82), 1.0);
}

/// Paint the message body's TS-style background and internal dividers.
pub(crate) fn paint_panel_body_chrome(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, sep_y: f32) {
    let inner_x = rect.origin.x + 1.0;
    let inner_w = (rect.size.x - 2.0).max(0.0);
    cx.backend.fill_rect(
        Rect::xywh(inner_x, rect.origin.y + HEADER_HEIGHT, inner_w, 1.0),
        (theme.border).with_alpha(0.75),
    );
    let body_y = rect.origin.y + HEADER_HEIGHT + 1.0;
    if sep_y > body_y {
        cx.backend.fill_rect(
            Rect::xywh(inner_x, body_y, inner_w, sep_y - body_y),
            (theme.background).with_alpha(0.72),
        );
    }
    // Full-width divider above the input (TS `border-t` spans the whole
    // panel, not just the padded content column).
    cx.backend.fill_rect(
        Rect::xywh(inner_x, sep_y, inner_w, 1.0),
        (theme.border).with_alpha(0.75),
    );
}

/// Paint the empty-state: centered hint, 4 full-width rounded pills, then two tip lines.
///
/// Layout (top→bottom):
///   • centered muted header line ("Try an example to design…") at HEADER_HEIGHT + HINT_OFFSET
///   • 4 stacked full-width pills (EXAMPLE_CARD_HEIGHT each, EXAMPLE_CARD_GAP gap)
///   • tip line 1: "Tip: Export design to code via Claude Code in terminal."
///   • tip line 2: "Drop image / text file to chat or via 📎"
#[allow(clippy::too_many_arguments)]
pub(crate) fn paint_examples(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    rect: Rect,
    hint_label: &str,
    _tip_label: &str,
    examples: &[ExampleCard; 4],
    disabled: bool,
    hover: Option<usize>,
    pressed_index: Option<usize>,
) {
    let opacity = if disabled { 0.6 } else { 1.0 };

    // ── Centered hint header ─────────────────────────────────────────────────
    let hint_font = 12.0;
    let hint = TextLayout::single_run(
        hint_label,
        "system-ui",
        hint_font,
        (theme.muted_foreground).with_alpha(opacity).to_jian(),
        Point2D::new(0.0, 0.0),
    );
    let hint_y = rect.origin.y + HEADER_HEIGHT + HINT_OFFSET + hint_font * 0.35;
    let hint_w = cx.backend.measure_text(hint_label, hint_font);
    cx.backend.draw_text(
        &hint,
        Point2D::new(rect.origin.x + (rect.size.x - hint_w) / 2.0, hint_y),
    );

    // ── 4 stacked full-width pills ───────────────────────────────────────────
    // Pill fill is slightly lighter than the panel bg — use card color with
    // a subtle alpha so it reads as a distinct surface.
    let pill_bg = (theme.secondary).with_alpha(0.85 * opacity);
    let pill_border = (theme.border).with_alpha(0.7 * opacity);
    let text_color = (theme.foreground).with_alpha(opacity);

    for (idx, (pill, ex)) in example_card_rects(rect)
        .iter()
        .zip(examples.iter())
        .enumerate()
    {
        // Base pill fill.
        cx.backend
            .fill_round_rect(*pill, EXAMPLE_PILL_RADIUS, pill_bg);

        let hovered = !disabled && hover == Some(idx);
        let pressed = !disabled && pressed_index == Some(idx);
        if hovered || pressed {
            paint_button_feedback_wash(
                cx.backend,
                theme,
                *pill,
                EXAMPLE_PILL_RADIUS,
                hovered,
                pressed,
            );
        }

        // Border: slightly accent-tinted on hover/press, subtle otherwise.
        let border = if hovered || pressed {
            (theme.primary).with_alpha(0.35)
        } else {
            pill_border
        };
        cx.backend
            .stroke_round_rect(*pill, EXAMPLE_PILL_RADIUS, border, 1.0);

        // Single-line prompt label, left-padded, ellipsized if needed.
        let text_max_w = (pill.size.x - EXAMPLE_PILL_TEXT_PAD * 2.0).max(0.0);
        let label = ellipsize(cx, &ex.title, text_max_w, EXAMPLE_TITLE_FONT);
        let label_layout = TextLayout::single_run(
            &label,
            "system-ui",
            EXAMPLE_TITLE_FONT,
            text_color.to_jian(),
            Point2D::new(0.0, 0.0),
        );
        // Vertically center text in pill (baseline-relative: center + fs*0.35).
        let baseline_y = pill.origin.y + EXAMPLE_CARD_HEIGHT / 2.0 + EXAMPLE_TITLE_FONT * 0.35;
        cx.backend.draw_text(
            &label_layout,
            Point2D::new(pill.origin.x + EXAMPLE_PILL_TEXT_PAD, baseline_y),
        );
    }

    // ── Two muted tip lines below the pills ─────────────────────────────────
    let pills = example_card_rects(rect);
    let last_pill_bottom = pills[3].origin.y + pills[3].size.y;
    let tip_color = (theme.muted_foreground)
        .with_alpha(0.55 * opacity)
        .to_jian();
    let tip_font = 10.0;

    // Tip line 1 baseline: below the last pill by TIP_TOP_GAP, then baseline shift.
    // tip1_top = last_pill_bottom + TIP_TOP_GAP
    // tip1_baseline = tip1_top + tip_font * 0.35  (draw_text is baseline-relative)
    let tip1_top = last_pill_bottom + TIP_TOP_GAP;
    let tip1_y = tip1_top + tip_font * 0.35;

    // Tip line 2: one full TIP_LINE_H below tip1's TOP (not its baseline) so they
    // stack cleanly without overlap.
    let tip2_top = tip1_top + TIP_LINE_H;
    let tip2_y = tip2_top + tip_font * 0.35;

    // Tip line 1: "Tip: Export design to code via Claude Code in terminal."
    let tip1 = "Tip: Export design to code via Claude Code in terminal.";
    let tip1_layout = TextLayout::single_run(
        tip1,
        "system-ui",
        tip_font,
        tip_color,
        Point2D::new(0.0, 0.0),
    );
    let tip1_w = cx.backend.measure_text(tip1, tip_font);
    cx.backend.draw_text(
        &tip1_layout,
        Point2D::new(rect.origin.x + (rect.size.x - tip1_w) / 2.0, tip1_y),
    );

    // Tip line 2: "Drop image / text file to chat or via 📎"
    let tip2_text = "Drop image / text file to chat or via";
    let tip2_layout = TextLayout::single_run(
        tip2_text,
        "system-ui",
        tip_font,
        tip_color,
        Point2D::new(0.0, 0.0),
    );
    let tip2_w = cx.backend.measure_text(tip2_text, tip_font);
    // Inline paperclip icon width estimate: tip_font * 1.2.
    let clip_size = tip_font * 1.2;
    let clip_gap = 4.0;
    let total_w = tip2_w + clip_gap + clip_size;
    let tip2_x = rect.origin.x + (rect.size.x - total_w) / 2.0;
    cx.backend
        .draw_text(&tip2_layout, Point2D::new(tip2_x, tip2_y));
    // Inline paperclip icon after the text.
    draw_icon(
        cx.backend,
        Icon::Paperclip,
        Point2D::new(tip2_x + tip2_w + clip_gap, tip2_y - clip_size * 0.8),
        clip_size,
        (theme.muted_foreground).with_alpha(0.55 * opacity),
        1.2,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Color, RenderBackend};

    #[derive(Default)]
    struct CaptureBackend {
        text_runs: Vec<String>,
        round_rects: Vec<(Rect, f32)>,
    }

    impl RenderBackend for CaptureBackend {
        fn begin_frame(&mut self) {}
        fn end_frame(&mut self) {}
        fn fill_rect(&mut self, _: Rect, _: Color) {}
        fn stroke_rect(&mut self, _: Rect, _: Color, _: f32) {}
        fn draw_text(&mut self, layout: &TextLayout, _: Point2D) {
            self.text_runs
                .extend(layout.runs().iter().map(|run| run.content.clone()));
        }
        fn clip_rect(&mut self, _: Rect) {}
        fn save(&mut self) {}
        fn restore(&mut self) {}
        fn translate(&mut self, _: Point2D) {}
        fn stroke_line(&mut self, _: Point2D, _: Point2D, _: Color, _: f32) {}
        fn fill_round_rect(&mut self, r: Rect, radius: f32, _: Color) {
            self.round_rects.push((r, radius));
        }
        fn stroke_round_rect(&mut self, r: Rect, radius: f32, _: Color, _: f32) {
            self.round_rects.push((r, radius));
        }
        fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: Color, _: f32) {}
        fn resize(&mut self, _: u32, _: u32) {}
        fn dpi_scale(&self) -> f32 {
            1.0
        }
        fn measure_text(&mut self, text: &str, size: f32) -> f32 {
            text.chars().count() as f32 * size * 0.55
        }
    }

    fn make_examples() -> [ExampleCard; 4] {
        [
            ExampleCard {
                title: "Technical dashboard web app for a utilities company".into(),
                subtitle: "".into(),
                prompt: "Technical dashboard web app for a utilities company".into(),
                emoji: "",
            },
            ExampleCard {
                title: "Terminal-style dashboard web app for a sports team".into(),
                subtitle: "".into(),
                prompt: "Terminal-style dashboard web app for a sports team".into(),
                emoji: "",
            },
            ExampleCard {
                title: "Dark bold website for a coffee shop in Prague".into(),
                subtitle: "".into(),
                prompt: "Dark bold website for a coffee shop in Prague".into(),
                emoji: "",
            },
            ExampleCard {
                title: "Luxury webapp for managing barbershop clients".into(),
                subtitle: "".into(),
                prompt: "Luxury webapp for managing barbershop clients".into(),
                emoji: "",
            },
        ]
    }

    /// The 4 pills are stacked vertically (full-width), not in a 2×2 grid.
    #[test]
    fn example_card_rects_are_full_width_stacked_pills() {
        let rect = Rect::xywh(0.0, 0.0, 360.0, 600.0);
        let pills = example_card_rects(rect);
        let expected_w = rect.size.x - PAD * 2.0;

        for pill in &pills {
            // Full width (panel minus padding on each side).
            assert!(
                (pill.size.x - expected_w).abs() < 0.01,
                "pill width should equal panel_w - 2*PAD, got {}",
                pill.size.x
            );
            // Each pill has the stadium height.
            assert!(
                (pill.size.y - EXAMPLE_CARD_HEIGHT).abs() < 0.01,
                "pill height should be EXAMPLE_CARD_HEIGHT"
            );
            // Pill starts at PAD from the left edge.
            assert!(
                (pill.origin.x - PAD).abs() < 0.01,
                "pill x origin should be PAD"
            );
        }

        // Verify stacking: each pill is directly below the previous + gap.
        for i in 1..4 {
            let expected_y = pills[i - 1].origin.y + EXAMPLE_CARD_HEIGHT + EXAMPLE_CARD_GAP;
            assert!(
                (pills[i].origin.y - expected_y).abs() < 0.01,
                "pill {} top should be prev_bottom + gap",
                i
            );
        }
    }

    /// Pills must not overlap each other.
    #[test]
    fn example_pills_do_not_overlap() {
        let rect = Rect::xywh(0.0, 0.0, 360.0, 600.0);
        let pills = example_card_rects(rect);
        for i in 0..3 {
            let bottom_i = pills[i].origin.y + pills[i].size.y;
            assert!(
                bottom_i < pills[i + 1].origin.y,
                "pill {} bottom ({}) must be above pill {} top ({})",
                i,
                bottom_i,
                i + 1,
                pills[i + 1].origin.y
            );
        }
    }

    /// paint_examples paints rounded rects with large radius (pill look).
    #[test]
    fn paint_examples_paints_pill_shaped_rects() {
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let rect = Rect::xywh(0.0, 0.0, 360.0, 600.0);
        let examples = make_examples();

        paint_examples(
            &mut cx,
            &Theme::dark(),
            rect,
            "Try an example",
            "tip",
            &examples,
            false,
            None,
            None,
        );

        // At least 4 round rects with large radius should be painted (one per pill).
        let pill_rects: Vec<_> = backend
            .round_rects
            .iter()
            .filter(|(_, r)| *r >= EXAMPLE_PILL_RADIUS - 0.01)
            .collect();
        assert!(
            pill_rects.len() >= 4,
            "expected >= 4 large-radius round rects (pills), got {}",
            pill_rects.len()
        );
    }

    /// paint_examples paints the prompt text for all 4 pills.
    #[test]
    fn paint_examples_renders_prompt_labels() {
        let mut backend = CaptureBackend::default();
        let mut cx = PaintCx {
            backend: &mut backend,
        };
        let rect = Rect::xywh(0.0, 0.0, 360.0, 600.0);
        let examples = make_examples();

        paint_examples(
            &mut cx,
            &Theme::dark(),
            rect,
            "Try an example",
            "tip",
            &examples,
            false,
            None,
            None,
        );

        // Each example title (or its ellipsized form) should appear in text runs.
        for ex in &examples {
            let first_word = ex.title.split_whitespace().next().unwrap_or("");
            assert!(
                backend.text_runs.iter().any(|r| r.contains(first_word)),
                "paint should render text containing '{}' for pill label",
                first_word
            );
        }
    }

    /// #37: pill height is the new compact value (40px).
    #[test]
    fn example_card_height_is_compact_40px() {
        assert!(
            (EXAMPLE_CARD_HEIGHT - 40.0).abs() < 0.01,
            "pill height should be 40px (#37 compact), got {}",
            EXAMPLE_CARD_HEIGHT
        );
    }

    /// #37: pill radius is the new compact value (14px).
    #[test]
    fn example_pill_radius_is_compact_14px() {
        assert!(
            (EXAMPLE_PILL_RADIUS - 14.0).abs() < 0.01,
            "pill radius should be 14px (#37 compact), got {}",
            EXAMPLE_PILL_RADIUS
        );
    }

    /// #37: two tip lines must not overlap each other.
    /// tip1_top = last_pill_bottom + TIP_TOP_GAP; tip2_top = tip1_top + TIP_LINE_H.
    /// tip2_top > tip1_top + font_height (10px) is sufficient.
    #[test]
    fn tip_lines_are_non_overlapping() {
        let rect = Rect::xywh(0.0, 0.0, 360.0, 600.0);
        let pills = example_card_rects(rect);
        let last_pill_bottom = pills[3].origin.y + pills[3].size.y;
        let tip1_top = last_pill_bottom + TIP_TOP_GAP;
        let tip2_top = tip1_top + TIP_LINE_H;
        let tip_font = 10.0;
        // Tip 2's visual top must be below tip 1's visual bottom.
        // Visual bottom ≈ tip_top + font_height (conservative: use tip_font).
        let tip1_visual_bottom = tip1_top + tip_font;
        assert!(
            tip2_top >= tip1_visual_bottom,
            "tip2 top ({tip2_top}) must be at or below tip1 bottom ({tip1_visual_bottom}) to avoid overlap"
        );
    }
}
