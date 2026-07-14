use super::*;
use op_editor_core::chat::ChatToolCall;

#[test]
fn wrap_units_breaks_ascii_at_word_boundaries() {
    // Budget 10 units — "hello world" (11) must split after the
    // space, not mid-word.
    let lines = wrap_units("hello world", 10);
    assert_eq!(lines, vec!["hello".to_string(), "world".to_string()]);
}

#[test]
fn wrap_units_preserves_explicit_newlines() {
    let lines = wrap_units("a\nb", 80);
    assert_eq!(lines, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn wrap_units_counts_cjk_as_two_units_each() {
    // Five CJK glyphs = 10 units. Budget 6 fits 3 per line.
    let lines = wrap_units("设计登录页", 6);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].chars().count(), 3);
    assert_eq!(lines[1].chars().count(), 2);
}

#[test]
fn wrap_units_hard_breaks_a_token_with_no_spaces() {
    // No space to rewind to — a long token still gets chopped so
    // it cannot overflow the bubble.
    let lines = wrap_units("aaaaaaaa", 3);
    assert!(lines.len() >= 3);
    assert!(lines.iter().all(|l| l.chars().count() <= 3));
}

#[test]
fn wrap_units_empty_text_yields_one_empty_line() {
    assert_eq!(wrap_units("", 40), vec![String::new()]);
}

#[test]
fn streaming_caret_uses_shared_text_input_blink_period() {
    let period = jian_core::text_input::CARET_BLINK_PERIOD_MS;

    assert!(streaming_caret_visible(0));
    assert!(streaming_caret_visible(period - 1));
    assert!(!streaming_caret_visible(period));
    assert!(!streaming_caret_visible(period * 2 - 1));
    assert!(streaming_caret_visible(period * 2));
}

fn body() -> Rect {
    Rect::xywh(0.0, 0.0, 340.0, 300.0)
}

fn rect_close(actual: Rect, expected: Rect) -> bool {
    (actual.origin.x - expected.origin.x).abs() < 0.01
        && (actual.origin.y - expected.origin.y).abs() < 0.01
        && (actual.size.x - expected.size.x).abs() < 0.01
        && (actual.size.y - expected.size.y).abs() < 0.01
}

#[allow(dead_code)] // test helper kept for future color-assertion tests
fn color_close(a: crate::Color, b: crate::Color) -> bool {
    (a.r - b.r).abs() < 1e-6
        && (a.g - b.g).abs() < 1e-6
        && (a.b - b.b).abs() < 1e-6
        && (a.a - b.a).abs() < 1e-6
}

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
fn transcript_text_offset_at_resolves_user_message_text() {
    let prompt = "生成一个设计精良的美食应用移动端首页";
    let messages = [ChatMessage::user(prompt)];
    let body = body();
    let items = build_transcript(&messages, body, op_editor_core::Locale::EnUs);
    let bubble = items[0].bubble.as_ref().expect("user prompt bubble");
    // Click within the user bubble text area (past the USER_BUBBLE_PAD inset).
    let point = Point2D::new(
        bubble.rect.origin.x + USER_BUBBLE_PAD + 22.0,
        bubble.rect.origin.y + USER_BUBBLE_PAD + 2.0,
    );

    let canonical = crate::widgets::ai_chat_transcript_cache::unowned_for_tests(
        &messages,
        body,
        op_editor_core::Locale::EnUs,
    );
    let hit = transcript_text_offset_at(&messages, &canonical, body, point, 0.0)
        .expect("user message text should be selectable");

    assert_eq!(hit.message_index, 0);
    assert!(hit.offset > 0);
    assert!(hit.offset <= prompt.len());
}

#[test]
fn paint_transcript_highlights_selected_user_text() {
    let prompt = "生成一个设计精良的美食应用移动端首页";
    let messages = [ChatMessage::user(prompt)];
    let selection = op_editor_core::chat::ChatTranscriptSelection {
        message_index: 0,
        anchor: 0,
        focus: prompt.len(),
    };
    let theme = crate::Theme::dark();
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    let canonical = crate::widgets::ai_chat_transcript_cache::unowned_for_tests(
        &messages,
        body(),
        op_editor_core::Locale::EnUs,
    );
    paint_transcript_with_selection(
        &mut cx,
        &theme,
        body(),
        &messages,
        &canonical,
        0,
        None,
        Some(selection),
        0.0,
    );

    assert!(
        backend
            .round_rect_colors
            .iter()
            .any(|color| *color == crate::widgets::text_selection::selection_color(&theme)),
        "selected transcript text should paint a visible selection wash"
    );
}

#[derive(Default)]
struct TranscriptPaintBackend {
    round_rects: Vec<(Rect, f32)>,
    round_rect_colors: Vec<crate::Color>,
    ovals: usize,
    texts: Vec<String>,
    text_colors: Vec<(String, jian_core::scene::Color)>,
    svg_strokes: Vec<(Point2D, f32)>,
    svg_stroke_colors: Vec<(Point2D, f32, crate::Color)>,
    rotations: Vec<f32>,
}

impl crate::RenderBackend for TranscriptPaintBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: crate::Color) {}
    fn stroke_rect(&mut self, _: Rect, _: crate::Color, _: f32) {}
    fn draw_text(&mut self, layout: &crate::TextLayout, _: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push(run.content.clone());
            self.text_colors.push((run.content.clone(), run.color));
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn rotate(&mut self, radians: f32, _: Point2D) {
        self.rotations.push(radians);
    }
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: crate::Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, color: crate::Color) {
        self.round_rects.push((rect, radius));
        self.round_rect_colors.push(color);
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: crate::Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, point: Point2D, size: f32, color: crate::Color, _: f32) {
        self.svg_strokes.push((point, size));
        self.svg_stroke_colors.push((point, size, color));
    }
    fn fill_oval(&mut self, _: Rect, _: crate::Color) {
        self.ovals += 1;
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

#[test]
fn cli_running_activity_uses_the_shared_rotating_loader() {
    let mut message = ChatMessage::assistant_streaming();
    message.activities.push(op_editor_core::ChatActivity {
        id: "build".into(),
        title: "Building the screen".into(),
        detail: None,
        status: op_editor_core::ChatActivityStatus::Running,
        content_offset: None,
    });
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &[message],
        250,
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(backend.rotations.len(), 1);
    assert!((backend.rotations[0] - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
}

#[test]
fn cli_pending_activity_uses_a_quiet_non_rotating_wait_ring() {
    let mut message = ChatMessage::assistant_streaming();
    message.activities.push(op_editor_core::ChatActivity {
        id: "queued".into(),
        title: "Queued section".into(),
        detail: None,
        status: op_editor_core::ChatActivityStatus::Pending,
        content_offset: None,
    });
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &[message],
        250,
        op_editor_core::Locale::EnUs,
    );

    assert!(backend.rotations.is_empty());
}

#[test]
fn paint_transcript_leaves_assistant_answer_unframed() {
    let messages = [ChatMessage::assistant("assistant answer")];
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &messages,
        0,
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(backend.round_rects.len(), 0);
}

#[test]
fn paint_transcript_keeps_user_answer_bubble_background() {
    // #27 restyle: user bubble uses theme.user_bubble (medium-gray),
    // replacing the old theme.row_selected_primary (blue-tinted wash).
    let messages = [ChatMessage::user("user prompt")];
    let theme = crate::Theme::dark();
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &theme,
        body(),
        &messages,
        0,
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(backend.round_rects.len(), 1);
    // Old: theme.row_selected_primary (blue-tinted). New: theme.user_bubble (medium-gray).
    assert_eq!(backend.round_rect_colors[0], theme.user_bubble);
    assert_ne!(backend.round_rect_colors[0], theme.primary);
}

#[test]
fn streaming_message_with_no_text_yields_a_typing_bubble() {
    let msgs = vec![ChatMessage::assistant_streaming()];
    let items = build_transcript(&msgs, body(), op_editor_core::Locale::EnUs);
    assert_eq!(items.len(), 1);
    assert!(items[0].streaming);
    let bubble = items[0].bubble.as_ref().expect("typing bubble present");
    assert!(bubble.typing, "empty in-flight message shows typing dots");
    assert!(bubble.lines.is_empty());
    assert!(
        bubble.rect.size.x < 120.0,
        "TS renders the empty streaming state as a compact w-fit pill"
    );
}

#[test]
fn paint_streaming_empty_assistant_shows_thinking_pill_label() {
    let messages = [ChatMessage::assistant_streaming()];
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &messages,
        0,
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(backend.round_rects.len(), 1);
    let (pill, radius) = backend.round_rects[0];
    assert!(pill.size.x < 120.0, "typing pill should not be full width");
    assert!(
        (radius - pill.size.y / 2.0).abs() < 1e-4,
        "TS uses rounded-full for the streaming pill"
    );
    assert!(
        backend.texts.iter().any(|text| text == "Thinking"),
        "TS shows the Thinking label before the animated dots"
    );
    assert_eq!(backend.ovals, 3);
}

#[test]
fn assistant_thinking_collapsed_has_header_but_no_body_lines() {
    let mut m = ChatMessage::assistant("the answer");
    m.thinking = "a long private chain of reasoning".into();
    // Default: thinking_collapsed == true.
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let t = items[0].thinking.as_ref().expect("thinking block present");
    assert!(t.collapsed);
    assert!(t.lines.is_empty(), "collapsed body carries no lines");
    assert!(t.header.size.y > 0.0, "header is still clickable");
    assert!((t.body.size.y - 0.0).abs() < 1e-4, "collapsed body is flat");
}

#[test]
fn assistant_thinking_expanded_has_wrapped_body_lines() {
    let mut m = ChatMessage::assistant("the answer");
    m.thinking = "a long private chain of reasoning that must wrap \
                  across several lines inside the narrow panel"
        .into();
    m.thinking_collapsed = false;
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let t = items[0].thinking.as_ref().unwrap();
    assert!(!t.collapsed);
    assert!(t.lines.len() > 1, "long reasoning wraps to many lines");
    assert!(t.body.size.y > 0.0);
}

#[test]
fn empty_design_progress_lines_do_not_render_inline_or_as_typing_placeholder() {
    let mut m = ChatMessage::assistant_streaming();
    m.thinking = "\n• Planning…\n• Scaffold ready".into();
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert!(
        items[0].steps.is_empty(),
        "empty plan/progress rows belong to the fixed checklist, matching TS ActionSteps"
    );
    assert!(
        items[0].thinking.is_none(),
        "design progress should not render as a reasoning block"
    );
    assert!(
        items[0].bubble.is_none(),
        "fixed checklist progress should suppress the empty streaming typing placeholder"
    );
}

#[test]
fn structured_activities_render_as_compact_rows_without_thinking_text() {
    let mut message = ChatMessage::assistant_streaming();
    message.activities = vec![
        op_editor_core::ChatActivity {
            id: "header".into(),
            title: "Greeting header".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Running,
            content_offset: None,
        },
        op_editor_core::ChatActivity {
            id: "rail".into(),
            title: "Recently played".into(),
            detail: Some("12 elements".into()),
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: None,
        },
    ];
    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 2);
    assert_eq!(items[0].steps[0].label, "Greeting header");
    assert!(items[0].steps[0].active);
    assert_eq!(items[0].steps[0].rect.size.y, ACTION_STEP_H);
    assert_eq!(items[0].steps[1].label, "Recently played");
    assert!(items[0].steps[1].done);
    assert!(items[0].thinking.is_none());
    assert!(items[0].bubble.is_none());
}

#[test]
fn structured_activities_interleave_with_cli_narration_by_offset() {
    let first = "I mapped the screen.";
    let second = "The sections are in place.";
    let final_text = "Done — the layout has been checked.";
    let content = format!("{first}\n\n{second}\n\n{final_text}");
    let second_offset = first.len() + 2 + second.len();
    let mut message = ChatMessage::assistant(&content);
    message.activities = vec![
        op_editor_core::ChatActivity {
            id: "build".into(),
            title: "Building sections".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: Some(first.len() as u32),
        },
        op_editor_core::ChatActivity {
            id: "check".into(),
            title: "Checking the design".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: Some(second_offset as u32),
        },
    ];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let item = &items[0];

    assert_eq!(item.steps.len(), 2);
    assert_eq!(item.flow_bubbles.len(), 3);
    assert!(item.bubble.is_none());
    assert!(item.flow_bubbles[0].rect.origin.y < item.steps[0].rect.origin.y);
    assert!(item.steps[0].rect.origin.y < item.flow_bubbles[1].rect.origin.y);
    assert!(item.flow_bubbles[1].rect.origin.y < item.steps[1].rect.origin.y);
    assert!(item.steps[1].rect.origin.y < item.flow_bubbles[2].rect.origin.y);
}

#[test]
fn legacy_and_interleaved_activity_steps_use_distinct_override_slots() {
    let mut message = ChatMessage::assistant("Narration");
    message.thinking = "• Legacy detail\n  ▸ diagnostic".into();
    message.activities.push(op_editor_core::ChatActivity {
        id: "build".into(),
        title: "Building section".into(),
        detail: Some("2 elements".into()),
        status: op_editor_core::ChatActivityStatus::Done,
        content_offset: Some(0),
    });
    message.action_step_expanded_overrides = vec![Some(false), Some(true)];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 2);
    assert_eq!(items[0].steps[0].source_index, 0);
    assert_eq!(items[0].steps[1].source_index, 1);
    assert!(!items[0].steps[0].expanded);
    assert!(items[0].steps[1].expanded);
}

#[test]
fn detail_less_structured_activity_has_no_invisible_toggle_hit() {
    let mut message = ChatMessage::assistant_streaming();
    message.activities.push(op_editor_core::ChatActivity {
        id: "header".into(),
        title: "Greeting header".into(),
        detail: None,
        status: op_editor_core::ChatActivityStatus::Running,
        content_offset: None,
    });
    let body = body();
    let canonical = super::ai_chat_transcript_cache::unowned_for_tests(
        std::slice::from_ref(&message),
        body,
        op_editor_core::Locale::EnUs,
    );
    let step = &canonical.items[0].steps[0];

    assert_eq!(
        transcript_hit(
            &canonical,
            body,
            step.rect.origin.x + 8.0,
            step.rect.origin.y + 8.0,
            0.0,
        ),
        None
    );
}

#[test]
fn current_step_with_content_is_active_until_terminal() {
    let mut m = ChatMessage::assistant_streaming();
    m.content = r#"<step title="Planning…">Drafting layout constraints</step>"#.into();
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(items[0].steps[0].label, "Planning…");
    assert!(items[0].steps[0].active);
    assert!(!items[0].steps[0].done);
}

#[test]
fn step_tag_content_renders_as_progress_not_raw_bubble() {
    let mut message = ChatMessage::assistant_streaming();
    message.content =
        r#"<step title="Checking guidelines" status="streaming">Analyzing request...</step>"#
            .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(items[0].steps[0].label, "Checking guidelines");
    assert!(items[0].steps[0].active);
    assert!(
        items[0].bubble.is_none(),
        "raw <step> markup should not render as answer text"
    );
}

#[test]
fn step_tag_content_surfaces_as_progress_details() {
    let mut message = ChatMessage::assistant_streaming();
    message.content = r#"<step title="Validate design" status="streaming">
lint: fixed spacing
render: captured frame
</step>"#
        .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(
        items[0].steps[0].details,
        vec![
            "lint: fixed spacing".to_string(),
            "render: captured frame".to_string()
        ]
    );
    assert!(
        items[0].steps[0].rect.size.y > 28.0,
        "step details should reserve space instead of being dropped"
    );
}

#[test]
fn completed_step_with_content_defaults_collapsed_like_ts_accordion() {
    let mut message = ChatMessage::assistant(
        r#"<step title="Validate design" status="done">
lint: fixed spacing
render: captured frame
</step>"#,
    );
    message.streaming = false;

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert!(items[0].steps[0].done);
    assert!(!items[0].steps[0].active);
    assert!(
        (items[0].steps[0].rect.size.y - ACTION_STEP_H).abs() < 1e-4,
        "TS ActionStepItem defaults completed accordions closed"
    );
}

#[test]
fn paint_completed_step_hides_details_like_collapsed_ts_accordion() {
    let message = ChatMessage::assistant(
        r#"<step title="Validate design" status="done">
lint: fixed spacing
render: captured frame
</step>"#,
    );
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &[message],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.texts.iter().any(|text| text == "Validate design"),
        "collapsed accordion still paints its title"
    );
    assert!(
        !backend
            .texts
            .iter()
            .any(|text| text.contains("lint: fixed spacing")
                || text.contains("render: captured frame")),
        "collapsed TS accordions hide details until opened"
    );
}

#[test]
fn assistant_design_json_code_fence_renders_compact_design_block() {
    let message = ChatMessage::assistant(
        r#"Here is the design:
```json
[{"id":"frame-1","type":"Frame"},{"id":"text-1","type":"Text"}]
```
Applied to canvas."#,
    );

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].design_blocks.len(), 1);
    assert_eq!(items[0].design_blocks[0].element_count, 2);
    assert_eq!(items[0].design_blocks[0].label, "2 design elements");
    let visible_text = items[0].bubble.as_ref().unwrap().lines.join("\n");
    assert!(visible_text.contains("Here is the design:"));
    assert!(visible_text.contains("Applied to canvas."));
    assert!(!visible_text.contains(r#""type":"Frame""#));
}

#[test]
fn assistant_applied_modify_json_without_ids_renders_localized_folded_card() {
    let mut message = ChatMessage::assistant(
        r#"```json
[{"type":"text","name":"Caption","content":"Updated"}]
```
<!-- APPLIED -->"#,
    );

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::ZhCn,
    );
    let block = &items[0].design_blocks[0];

    assert_eq!(items[0].design_blocks.len(), 1);
    assert_eq!(block.element_count, 1);
    assert_eq!(block.label, "已修改 · 1 元素");
    assert!(block.apply.is_none(), "applied cards must not offer Apply");
    assert!(!block.expanded, "applied cards are folded by default");

    message.design_block_expanded_overrides = vec![Some(true)];
    let expanded = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::ZhCn,
    );
    let block = &expanded[0].design_blocks[0];
    assert!(block.expanded, "applied cards remain expandable");
    assert!(block.body.size.y > 0.0);
    assert!(
        block.apply.is_none(),
        "expanded applied cards still omit Apply"
    );
    assert!(block.code_lines.iter().any(|line| line.contains("Caption")));
}

#[test]
fn assistant_plain_json_with_type_is_not_a_design_block() {
    let message = ChatMessage::assistant(
        r#"```json
{"id":"event-1","type":"audit","payload":{"ok":true}}
```"#,
    );

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert!(items[0].design_blocks.is_empty());
    let visible_text = items[0].bubble.as_ref().unwrap().lines.join("\n");
    assert!(visible_text.contains(r#""type":"audit""#));
}

#[test]
fn expanded_design_json_block_reserves_body_and_surfaces_code_like_ts() {
    let mut message = ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    );
    message.design_block_expanded_overrides = vec![Some(true)];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let block = &items[0].design_blocks[0];

    assert!(block.expanded);
    assert!(block.rect.size.y > 32.0);
    assert!(block.body.size.y > 0.0);
    assert!(
        block.apply.is_some(),
        "generation cards keep the Apply button"
    );
    assert!(
        (block.body.origin.y - (block.header.origin.y + block.header.size.y + 4.0)).abs() < 1e-4,
        "TS expanded design cards put the JSON preview in a separate mt-1 body box"
    );
    assert!(block
        .code_lines
        .iter()
        .any(|line| line.contains(r#""type":"Frame""#)));
}

#[test]
fn paint_design_json_block_shows_expand_affordance_like_ts() {
    let message = ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    );
    let body = body();
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body,
        &[message],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend
            .svg_strokes
            .iter()
            .any(|(point, size)| *size == 12.0 && point.x >= body.origin.x + body.size.x - 26.0),
        "TS design JSON blocks carry a right-side chevron affordance"
    );
}

#[test]
fn paint_expanded_design_json_block_draws_code_preview_like_ts() {
    let mut message = ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    );
    message.design_block_expanded_overrides = vec![Some(true)];
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &[message],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend
            .texts
            .iter()
            .any(|line| line.contains(r#""type":"Frame""#)),
        "expanded TS design cards show a JSON preview"
    );
}

#[test]
fn paint_expanded_design_json_block_draws_separate_body_box_like_ts() {
    let mut message = ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    );
    message.design_block_expanded_overrides = vec![Some(true)];
    let body = body();
    let expected = build_transcript(
        std::slice::from_ref(&message),
        body,
        op_editor_core::Locale::EnUs,
    )[0]
    .design_blocks[0]
        .body;
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body,
        &[message],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.round_rects.iter().any(|(rect, radius)| {
            (rect.origin.x - expected.origin.x).abs() < 1e-4
                && (rect.origin.y - expected.origin.y).abs() < 1e-4
                && (rect.size.x - expected.size.x).abs() < 1e-4
                && (rect.size.y - expected.size.y).abs() < 1e-4
                && (*radius - 6.0).abs() < 1e-4
        }),
        "expanded TS design cards paint the JSON preview in its own rounded body box"
    );
}

#[test]
fn streaming_design_json_shows_no_design_card() {
    let mut message = ChatMessage::assistant_streaming();
    message.content = r#"```json
[{"id":"frame-1","type":"Frame"}]"#
        .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    // Streaming design cards are suppressed — no "Generating design..." card
    // while the turn streams (the design JSON is not shown as a bubble either).
    assert!(items[0].design_blocks.is_empty());
    assert!(items[0].bubble.is_none());
}

#[test]
fn assistant_tool_call_xml_is_hidden_from_answer_bubble() {
    let message = ChatMessage::assistant(
        r#"before
<function_calls><invoke name="batch_design">secret</invoke></function_calls>
<result>{"ok":true}</result>
<!-- APPLIED -->
after"#,
    );

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let text = items[0].bubble.as_ref().unwrap().lines.join("\n");

    assert!(text.contains("before"));
    assert!(text.contains("after"));
    assert!(!text.contains("function_calls"));
    assert!(!text.contains("invoke"));
    assert!(!text.contains("secret"));
    assert!(!text.contains("APPLIED"));
}

#[test]
fn hidden_completed_assistant_action_shows_completion_placeholder() {
    let message = ChatMessage::assistant(
        r#"<function_calls><invoke name="batch_design">secret</invoke></function_calls>
<result>{"ok":true}</result>
<!-- APPLIED -->"#,
    );

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let text = items[0].bubble.as_ref().unwrap().lines.join("\n");

    assert_eq!(text, "(Automated action completed)");
}

#[test]
fn streaming_unclosed_invoke_is_hidden_from_answer_bubble() {
    let mut message = ChatMessage::assistant_streaming();
    message.content = r#"visible
<invoke name="batch_design"><parameter name="dsl">internal"#
        .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let text = items[0].bubble.as_ref().unwrap().lines.join("\n");

    assert!(text.contains("visible"));
    assert!(!text.contains("invoke"));
    assert!(!text.contains("parameter"));
    assert!(!text.contains("internal"));
}

#[test]
fn tool_calls_block_header_label_counts_the_calls() {
    let mut m = ChatMessage::assistant("done");
    m.tool_calls = vec![
        ChatToolCall {
            name: "insert_node".into(),
            args: "{}".into(),
            content_offset: None,
        },
        ChatToolCall {
            name: "set_fill_hex".into(),
            args: "{}".into(),
            content_offset: None,
        },
    ];
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let t = items[0].tools.as_ref().expect("tools block present");
    // The header label substitutes the call count into the
    // `ai.toolCalls` template's `{{count}}` placeholder.
    let expected =
        op_i18n::translate(op_editor_core::Locale::EnUs, "ai.toolCalls").replace("{{count}}", "2");
    assert_eq!(t.label, expected, "header label counts the calls");
}

#[test]
fn expanded_tool_card_surfaces_status_source_and_result() {
    let mut m = ChatMessage::assistant("done");
    m.tools_collapsed = false;
    m.tool_calls = vec![ChatToolCall {
        name: "batch_design".into(),
        args: r#"{"source":"designer-1","status":"error","args":{"dsl":"I(\"root\",{})"},"result":{"success":false,"error":"node not found"}}"#.into(),
        content_offset: None,
    }];

    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let t = items[0].tools.as_ref().expect("tools block present");

    assert!(
        t.lines.iter().any(|line| line == "  Source: designer-1"),
        "tool card should expose the originating agent/source"
    );
    assert!(
        t.lines.iter().any(|line| line == "  Status: error"),
        "tool card should expose the tool status"
    );
    assert!(
        t.lines
            .iter()
            .any(|line| line == "  Result: node not found"),
        "tool card should expose failure result text"
    );
    assert!(
        t.lines
            .iter()
            .any(|line| line.contains(r#""dsl":"I(\"root\",{})""#)),
        "tool card should still show the actual call arguments"
    );
}

#[test]
fn streaming_tool_card_falls_back_to_running_status() {
    let mut m = ChatMessage::assistant_streaming();
    m.tools_collapsed = false;
    m.tool_calls = vec![ChatToolCall {
        name: "snapshot_layout".into(),
        args: "{}".into(),
        content_offset: None,
    }];

    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let t = items[0].tools.as_ref().expect("tools block present");

    assert!(
        t.lines.iter().any(|line| line == "  Status: running"),
        "in-flight tool card should not look like a completed raw JSON dump"
    );
}

#[test]
fn paint_expanded_tool_calls_as_individual_cards_like_ts() {
    let mut m = ChatMessage::assistant("done");
    m.tools_collapsed = false;
    m.tool_calls = vec![
        ChatToolCall {
            name: "batch_design".into(),
            args: r#"{"args":{"dsl":"I(\"root\",{})"},"status":"running"}"#.into(),
            content_offset: None,
        },
        ChatToolCall {
            name: "delete_node".into(),
            args: r#"{"args":{"id":"old-node"},"result":{"success":false,"error":"missing"}}"#
                .into(),
            content_offset: None,
        },
    ];
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &[m],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.round_rects.len() >= 3,
        "TS renders each tool call as its own bordered card, not one text body"
    );
}

#[test]
fn mixed_tool_calls_expand_only_write_level_card_bodies_like_ts() {
    let mut m = ChatMessage::assistant("done");
    m.tools_collapsed = false;
    m.tool_calls = vec![
        ChatToolCall {
            name: "snapshot_layout".into(),
            args: r#"{"args":{"pageId":"page-1"}}"#.into(),
            content_offset: None,
        },
        ChatToolCall {
            name: "batch_design".into(),
            args: r#"{"args":{"dsl":"I(\"root\",{})"}}"#.into(),
            content_offset: None,
        },
    ];

    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let tools = items[0].tools.as_ref().expect("tools block present");

    assert_eq!(tools.cards.len(), 2);
    assert!(
        tools.cards[0].body.size.y == 0.0,
        "TS keeps read tool cards collapsed by default"
    );
    assert!(
        tools.cards[1].body.size.y > 0.0,
        "TS opens modify tool cards by default"
    );
}

#[test]
fn user_message_images_get_one_thumbnail_rect_each() {
    let mut m = ChatMessage::user("look");
    for i in 0..3 {
        m.images.push(op_editor_core::ChatImage {
            id: i,
            name: format!("{i}.png"),
            media_type: "image/png".into(),
            data: vec![1],
        });
    }
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );
    assert_eq!(items[0].images.len(), 3, "one thumbnail rect per image");
    // Thumbnails do not overlap.
    let (a, b) = (items[0].images[0], items[0].images[1]);
    assert!(a.origin.x != b.origin.x || a.origin.y != b.origin.y);
}

#[test]
fn narration_markdown_keeps_its_markers_and_re_breaks_glued_headings() {
    use super::normalize_narration_markdown;
    // The measured stream: batch headings glued back-to-back. The MARKERS stay
    // — the transcript renders them as typography now (bold labels, code chips,
    // bullets), so stripping them here would throw the styling away.
    let raw = "**Batch 1 — Skeleton****Batch 2 — Header**\nThe design features:**Header**";
    let out = normalize_narration_markdown(raw);
    assert!(
        out.contains("**Batch 1 — Skeleton**\n**Batch 2 — Header**"),
        "glued headings re-break onto their own lines: {out}"
    );
    assert!(
        out.contains("features:\n**Header**"),
        "a heading opening after a colon starts its own line: {out}"
    );
}

#[test]
fn narration_renders_as_typed_markdown_not_a_grey_wall() {
    use crate::widgets::ai_chat_transcript_richtext::{layout_rich, SpanStyle};

    let lines = layout_rich(
        "**Layout** — a page (`#F4F5F7`) with a card\n- 5-tab bottom navigation",
        60,
    );
    let first = &lines[0];
    assert_eq!(first.spans[0].text, "Layout");
    assert_eq!(first.spans[0].style, SpanStyle::Strong, "the label is bold");
    assert!(
        first
            .spans
            .iter()
            .any(|s| s.style == SpanStyle::Code && s.text == "#F4F5F7"),
        "the hex reads as code: {:?}",
        first.spans
    );
    let bullet = lines.iter().find(|l| l.bullet).expect("a bullet line");
    assert!(bullet.inset > 0.0, "bullet text hangs off the dot");
    assert!(
        bullet.spans[0].text.starts_with("5-tab"),
        "the dash marker is consumed by the bullet, not printed: {:?}",
        bullet.spans
    );
}

#[test]
fn an_unclosed_marker_stays_literal() {
    use crate::widgets::ai_chat_transcript_richtext::{parse_spans, SpanStyle};
    let spans = parse_spans("a ** dangling marker");
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].style, SpanStyle::Body);
    assert_eq!(spans[0].text, "a ** dangling marker");
}

// ── interleaved narration ↔ tool-chip flow ──

fn loop_call(name: &str, offset: Option<u32>) -> ChatToolCall {
    ChatToolCall {
        name: name.into(),
        args: "{}".into(),
        content_offset: offset,
    }
}

fn loop_message() -> ChatMessage {
    // "First I plan." → [batch_design ×2] → "Then I check." → [get_screenshot]
    // → "All done."
    let mut msg = ChatMessage::assistant("First I plan. Then I check. All done.");
    msg.tool_calls = vec![
        loop_call("batch_design", Some(14)),
        loop_call("batch_design", Some(14)),
        loop_call("get_screenshot", Some(27)),
    ];
    msg
}

#[test]
fn offset_stamped_calls_interleave_prose_and_headerless_panels() {
    let (item, _) = build_item(
        &loop_message(),
        0,
        0.0,
        body(),
        op_editor_core::Locale::EnUs,
    );
    assert!(item.tools.is_none(), "grouped panel replaced by the flow");
    assert!(
        item.bubble.is_none(),
        "monolithic bubble replaced by the flow"
    );
    assert_eq!(item.flow_bubbles.len(), 3, "three prose segments");
    assert_eq!(item.flow_panels.len(), 2, "two call groups");
    assert_eq!(
        item.flow_panels[0].cards.len(),
        2,
        "same-offset calls stack"
    );
    // Document order: prose, panel, prose, panel, prose.
    let b = &item.flow_bubbles;
    let p = &item.flow_panels;
    assert!(b[0].rect.origin.y < p[0].cards[0].rect.origin.y);
    assert!(p[0].cards[1].rect.origin.y < b[1].rect.origin.y);
    assert!(b[1].rect.origin.y < p[1].cards[0].rect.origin.y);
    assert!(p[1].cards[0].rect.origin.y < b[2].rect.origin.y);
    // Headerless: nothing for the group-toggle hit to land on.
    assert_eq!(p[0].header.size.y, 0.0);
    // Cards carry their ORIGINAL indices for the expand override.
    assert_eq!(p[0].cards[0].index, 0);
    assert_eq!(p[0].cards[1].index, 1);
    assert_eq!(p[1].cards[0].index, 2);
}

#[test]
fn calls_without_offsets_keep_the_grouped_panel() {
    let mut msg = ChatMessage::assistant("plain chat answer");
    msg.tool_calls = vec![loop_call("get_node", None)];
    let (item, _) = build_item(&msg, 0, 0.0, body(), op_editor_core::Locale::EnUs);
    assert!(item.tools.is_some(), "no offsets → classic grouped panel");
    assert!(item.flow_panels.is_empty());
    assert!(item.bubble.is_some());
}

#[test]
fn flow_card_hit_returns_original_tool_index() {
    let msgs = [loop_message()];
    let canonical = super::super::ai_chat_transcript_cache::unowned_for_tests(
        &msgs,
        body(),
        op_editor_core::Locale::EnUs,
    );
    let item = &canonical.items[0];
    let card = &item.flow_panels[1].cards[0];
    let hit = transcript_hit(
        &canonical,
        body(),
        card.header.origin.x + 4.0,
        card.header.origin.y + card.header.size.y / 2.0,
        0.0,
    );
    assert_eq!(
        hit,
        Some(TranscriptHit::SetToolCallCardExpanded(0, 2, !card.expanded)),
        "third call toggles override slot 2 even though it is its panel's first card"
    );
}

#[test]
fn trailing_calls_at_content_end_leave_no_empty_prose_segment() {
    let mut msg = ChatMessage::assistant("Building the header now.");
    let end = msg.content.len() as u32;
    msg.tool_calls = vec![loop_call("batch_design", Some(end))];
    msg.streaming = true;
    let (item, _) = build_item(&msg, 0, 0.0, body(), op_editor_core::Locale::EnUs);
    assert_eq!(item.flow_bubbles.len(), 1);
    assert_eq!(item.flow_panels.len(), 1);
    assert!(item.flow_bubbles[0].rect.origin.y < item.flow_panels[0].cards[0].rect.origin.y);
}

#[test]
fn prose_sits_equally_far_above_and_below_a_tool_chip() {
    let (item, _) = build_item(
        &loop_message(),
        0,
        0.0,
        body(),
        op_editor_core::Locale::EnUs,
    );
    let b = &item.flow_bubbles;
    let p = &item.flow_panels;
    let first_card = &p[0].cards[0];
    let last_card = p[0].cards.last().unwrap();
    let above = first_card.rect.origin.y - (b[0].rect.origin.y + b[0].rect.size.y);
    let below = b[1].rect.origin.y - (last_card.rect.origin.y + last_card.rect.size.y);
    assert!(
        (above - below).abs() < 0.01,
        "a chip must belong to the story on BOTH sides equally: {above} above vs {below} below"
    );
}
