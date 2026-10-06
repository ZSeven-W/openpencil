//! Paint-assertion tests for the AI chat panel's minimized form — the
//! compact input bar. Sibling of `tests_paint.rs` so the bar's own
//! assertions live next to each other and neither file approaches the
//! 800-line cap.

use super::tests::{seed_available_model, PanelPaintBackend};
use super::tests_paint::{assert_close, color_close, rect_close};
use super::*;
use crate::widgets::{AI_CHAT_MINIMIZED_HEIGHT, AI_CHAT_WIDTH};

#[test]
fn paint_minimized_bar_reads_as_a_compact_input() {
    let mut s = EditorState::new();
    s.chat.minimize();
    let panel = {
        // The minimized bar is the TOUCH sheet's collapsed form now: on
        // desktop the composer-only card replaced it, so these tests ask
        // for the bar explicitly rather than relying on a default that
        // no longer produces one.
        let mut panel = AIChatPlaceholder::from_editor(&s);
        panel.composer_only = false;
        panel
    };
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_MINIMIZED_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    // A single, very light shadow layer under the surface — the bar is a
    // strip, not a window; a visible drop shadow makes it read as a slab.
    let shadow = backend.round_rects[0];
    assert_close(shadow.0.origin.y, rect.origin.y + 2.0);
    assert!(
        shadow.2.a <= 0.06,
        "the bar's shadow must stay barely-there, got alpha {}",
        shadow.2.a
    );
    // Capsule surface next, at the bar's own radius.
    assert_eq!(backend.round_rects[1].0, rect);
    assert_close(backend.round_rects[1].1, 12.0);
    // The placeholder is the same string the expanded textarea shows —
    // the bar must not invent its own prompt copy.
    assert!(
        backend.texts.iter().any(
            |(text, size, color, _)| text == &panel.label_input_placeholder
                && (*size - 12.0).abs() < 1e-4
                && *color == (panel.theme.muted_foreground).to_jian()
        ),
        "minimized bar paints the shared input placeholder"
    );
    // Sparkle glyph at the left, submit arrow inside the right circle
    // (each icon emits one stroke per path segment).
    assert!(backend.svg_strokes.first().expect("sparkle painted").0.x < rect.size.x / 2.0);
    assert!(backend.svg_strokes.last().expect("arrow painted").0.x > rect.size.x / 2.0);
    // Submit circle — the last round rect, drawn at half its diameter.
    let submit = backend.round_rects.last().expect("submit circle painted");
    assert_close(submit.0.size.x, 26.0);
    assert_close(submit.1, 13.0);
}

#[test]
fn the_model_name_sits_a_step_below_the_placeholder() {
    // Secondary chrome: same muted hue, lower alpha, smaller size — the
    // eye should land on the prompt, not on the model.
    let mut s = EditorState::new();
    s.chat.minimize();
    seed_available_model(&mut s);
    let panel = {
        // The minimized bar is the TOUCH sheet's collapsed form now: on
        // desktop the composer-only card replaced it, so these tests ask
        // for the bar explicitly rather than relying on a default that
        // no longer produces one.
        let mut panel = AIChatPlaceholder::from_editor(&s);
        panel.composer_only = false;
        panel
    };
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_MINIMIZED_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    let placeholder = backend
        .texts
        .iter()
        .find(|(text, _, _, _)| text == &panel.label_input_placeholder)
        .expect("placeholder painted");
    let model = backend
        .texts
        .iter()
        .find(|(text, size, _, _)| text != &panel.label_input_placeholder && *size < placeholder.1)
        .expect("model name painted smaller than the placeholder");
    assert!(
        model.2.a() < placeholder.2.a(),
        "model name must be fainter than the placeholder"
    );
}

#[test]
fn paint_minimized_bar_prefers_the_unsent_draft_over_the_placeholder() {
    let mut s = EditorState::new();
    s.chat.minimize();
    s.chat
        .set_input_text("a pricing page\nsecond line".to_string());
    let panel = {
        // The minimized bar is the TOUCH sheet's collapsed form now: on
        // desktop the composer-only card replaced it, so these tests ask
        // for the bar explicitly rather than relying on a default that
        // no longer produces one.
        let mut panel = AIChatPlaceholder::from_editor(&s);
        panel.composer_only = false;
        panel
    };
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_MINIMIZED_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend
            .texts
            .iter()
            .any(|(text, _, color, _)| text == "a pricing page"
                && *color == (panel.theme.foreground).to_jian()),
        "a staged draft shows in full-strength text instead of the placeholder"
    );
    assert!(
        !backend
            .texts
            .iter()
            .any(|(text, _, _, _)| text == &panel.label_input_placeholder),
        "the placeholder must not paint under the draft"
    );
}

#[test]
fn paint_minimized_bar_hover_adds_visible_feedback_across_the_bar() {
    let mut s = EditorState::new();
    s.chat.minimize();
    s.editor_ui.chat_header_hover = Some(op_editor_core::ChatHeaderButton::ToggleCollapse);
    let panel = {
        // The minimized bar is the TOUCH sheet's collapsed form now: on
        // desktop the composer-only card replaced it, so these tests ask
        // for the bar explicitly rather than relying on a default that
        // no longer produces one.
        let mut panel = AIChatPlaceholder::from_editor(&s);
        panel.composer_only = false;
        panel
    };
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_MINIMIZED_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(
        backend.round_rects.iter().any(|(r, radius, color)| {
            rect_close(*r, rect)
                && *radius >= 8.0
                && color_close(*color, chat_neutral_hover_color(&panel.theme))
        }),
        "hovering the minimized bar should wash its whole surface — the bar is one button"
    );
}
