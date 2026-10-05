//! Transcript layout geometry: scrolling, pinning, bubble widths and heights.
//!
//! Split out of `ai_chat_transcript_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn long_transcript_scrolls_and_pins_to_bottom() {
    // Enough messages to overflow the 300px body.
    let msgs: Vec<_> = (0..40)
        .map(|i| ChatMessage::user(format!("message {i}")))
        .collect();
    let b = body();
    let loc = op_editor_core::Locale::EnUs;
    let content = transcript_content_height(&msgs, b, loc);
    assert!(content > b.size.y, "content {content} should overflow body");
    let max = content - b.size.y;

    // Pinned → render at the bottom regardless of the stored offset.
    let pinned = transcript_effective_offset(&msgs, b, loc, 0.0, true);
    assert!((pinned - max).abs() < 0.5);
    // Unpinned → the stored offset, clamped into `[0, max]`.
    assert!((transcript_effective_offset(&msgs, b, loc, 50.0, false) - 50.0).abs() < 0.01);
    assert!((transcript_effective_offset(&msgs, b, loc, 1.0e6, false) - max).abs() < 0.5);

    // At the pinned offset the final message sits within the body.
    let items = build_transcript_with_design_hover(&msgs, b, loc, None, pinned);
    let last = items.last().unwrap().bubble.as_ref().unwrap().rect;
    assert!(
        last.origin.y + last.size.y <= b.origin.y + b.size.y + 0.5,
        "last bubble bottom should rest within the body"
    );
}

#[test]
fn short_transcript_has_no_scroll_range() {
    let msgs = [ChatMessage::user("hi")];
    let b = body();
    let loc = op_editor_core::Locale::EnUs;
    assert!(transcript_content_height(&msgs, b, loc) <= b.size.y);
    // Nothing to scroll → effective offset is 0 whether pinned or not.
    assert_eq!(transcript_effective_offset(&msgs, b, loc, 0.0, true), 0.0);
    assert_eq!(transcript_effective_offset(&msgs, b, loc, 99.0, false), 0.0);
}

#[test]
fn build_transcript_empty_messages_is_empty() {
    assert!(build_transcript(&[], body(), op_editor_core::Locale::EnUs).is_empty());
}

#[test]
fn assistant_blocks_use_full_body_width_like_ts_transcript() {
    let msg = ChatMessage::assistant("assistant answer");
    let body = body();
    let items = build_transcript(
        std::slice::from_ref(&msg),
        body,
        op_editor_core::Locale::EnUs,
    );
    let bubble = items[0].bubble.as_ref().expect("assistant answer bubble");

    assert!((bubble.rect.origin.x - body.origin.x).abs() < 1e-4);
    assert!((bubble.rect.size.x - body.size.x).abs() < 1e-4);
}

#[test]
fn assistant_answer_uses_plain_text_height_without_bubble_padding() {
    let msg = ChatMessage::assistant("first line\nsecond line");
    let items = build_transcript(
        std::slice::from_ref(&msg),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let bubble = items[0].bubble.as_ref().expect("assistant answer text");

    assert_eq!(bubble.lines.len(), 2);
    assert!((bubble.rect.size.y - LINE_H * 2.0).abs() < 1e-4);
}

#[test]
fn done_summary_renders_as_plain_assistant_narration() {
    let msg = ChatMessage::assistant("Done — 4 subtask(s) succeeded, 0 failed, 4 node(s) total.");
    let body = body();
    let items = build_transcript(
        std::slice::from_ref(&msg),
        body,
        op_editor_core::Locale::EnUs,
    );
    let bubble = items[0].bubble.as_ref().expect("completion bubble");

    assert_eq!(bubble.rect.size.x, body.size.x);
    assert!(bubble.lines.join(" ").starts_with("Done —"));

    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    paint_transcript(
        &mut cx,
        &crate::Theme::light(),
        body,
        &[msg],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.round_rects.is_empty(),
        "the retired blue Done status surface must not be painted"
    );
}

#[test]
fn structured_completion_is_metadata_only_without_narration() {
    let mut msg = ChatMessage::assistant("");
    msg.completion = Some(op_editor_core::ChatCompletion {
        succeeded: 3,
        failed: 0,
        nodes: 42,
    });
    let items = build_transcript(
        std::slice::from_ref(&msg),
        body(),
        op_editor_core::Locale::EnUs,
    );
    assert!(
        items[0].bubble.is_none(),
        "structured metadata alone must not resurrect the old blue Done card"
    );
    assert!(msg.content.is_empty());
}

#[test]
fn structured_completion_keeps_final_narration_visible() {
    let mut msg = ChatMessage::assistant("All requested sections are ready.");
    msg.completion = Some(op_editor_core::ChatCompletion {
        succeeded: 3,
        failed: 0,
        nodes: 42,
    });
    let items = build_transcript(
        std::slice::from_ref(&msg),
        body(),
        op_editor_core::Locale::EnUs,
    );

    let bubble = items[0].bubble.as_ref().expect("final narration");
    assert_eq!(bubble.lines.join(" "), "All requested sections are ready.");
}

#[test]
fn user_bubbles_remain_compact_and_right_aligned() {
    // #27 restyle: user bubble width now accounts for USER_BUBBLE_PAD (14px)
    // instead of BUBBLE_PAD (8px), so the bubble is slightly wider per line.
    let prompt = "user prompt";
    let msg = ChatMessage::user(prompt);
    let body = body();
    let items = build_transcript(
        std::slice::from_ref(&msg),
        body,
        op_editor_core::Locale::EnUs,
    );
    let bubble = items[0].bubble.as_ref().expect("user bubble");

    let expected_w = (text_unit_width(prompt) + 2.0 * USER_BUBBLE_PAD)
        .max(USER_BUBBLE_MIN_W)
        .min(body.size.x * USER_BUBBLE_MAX_FRAC);
    assert!((bubble.rect.size.x - expected_w).abs() < 1e-4);
    assert!(
        (bubble.rect.origin.x + bubble.rect.size.x - (body.origin.x + body.size.x)).abs() < 1e-4
    );
}

#[test]
fn tight_final_turn_pins_completion_and_scrolls_to_reveal_prompt() {
    // A short final turn (prompt + Done summary) squeezed into a 64px body by
    // the fixed checklist overflows, so it can no longer show both at once.
    // The pinned (default) view keeps the latest content — the completion
    // summary — anchored to the bottom; the prompt is one scroll-up away. This
    // replaces the old no-scroll "keep the prompt attached" tail-fit hack,
    // whose only way to keep both visible was to never scroll at all.
    let messages = [
        ChatMessage::user("生成一个设计精良的美食应用移动端首页"),
        ChatMessage::assistant("Done — 4 subtask(s) succeeded, 0 failed, 4 node(s) total."),
    ];
    let tight_body = Rect::xywh(0.0, 0.0, 340.0, 64.0);
    let loc = op_editor_core::Locale::EnUs;

    let max = (transcript_content_height(&messages, tight_body, loc) - tight_body.size.y).max(0.0);
    assert!(
        max > 0.0,
        "tight body should overflow → a scroll range exists"
    );

    // Pinned: the completion summary rests against the body bottom.
    let pinned = transcript_effective_offset(&messages, tight_body, loc, 0.0, true);
    assert!((pinned - max).abs() < 0.5);
    let items = build_transcript_with_design_hover(&messages, tight_body, loc, None, pinned);
    let completion = items[1].bubble.as_ref().expect("completion summary");
    assert!(
        completion.rect.origin.y + completion.rect.size.y
            <= tight_body.origin.y + tight_body.size.y + 0.5,
        "completion summary pins to the bottom of the body"
    );

    // Scrolling to the top (un-pinned, offset 0) brings the prompt fully
    // into view — content the old layout could never reach.
    let top = build_transcript_with_design_hover(&messages, tight_body, loc, None, 0.0);
    assert_eq!(top[0].role, ChatRole::User);
    let user = top[0].bubble.as_ref().expect("user prompt bubble");
    assert!(
        user.rect.origin.y >= tight_body.origin.y - 0.5,
        "prompt sits at the top when scrolled up"
    );
}

#[test]
fn rate_limit_copy_is_localized_in_layout_without_rewriting_raw_error_history() {
    let raw = "error: openai-compatible http 429";
    let message = ChatMessage::assistant(raw);
    for locale in [op_editor_core::Locale::ZhCn, op_editor_core::Locale::EnUs] {
        let (item, _) = build_item(&message, 0, 0.0, body(), locale);
        let displayed = item.bubble.unwrap().lines.concat();
        let expected = op_i18n::translate(locale, "ai.error.rateLimited");
        assert_eq!(displayed.replace(' ', ""), expected.replace(' ', ""));
        assert_eq!(message.content, raw);
    }
    let user = ChatMessage::user(raw);
    let (item, _) = build_item(&user, 0, 0.0, body(), op_editor_core::Locale::ZhCn);
    assert!(item.bubble.unwrap().lines.concat().contains("error:"));
}
