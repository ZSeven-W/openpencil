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

fn body() -> Rect {
    Rect::xywh(0.0, 0.0, 340.0, 300.0)
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
fn user_bubbles_remain_compact_and_right_aligned() {
    let msg = ChatMessage::user("user prompt");
    let body = body();
    let items = build_transcript(
        std::slice::from_ref(&msg),
        body,
        op_editor_core::Locale::EnUs,
    );
    let bubble = items[0].bubble.as_ref().expect("user bubble");

    assert!((bubble.rect.size.x - body.size.x * BUBBLE_FRAC).abs() < 1e-4);
    assert!(
        (bubble.rect.origin.x + bubble.rect.size.x - (body.origin.x + body.size.x)).abs() < 1e-4
    );
}

#[derive(Default)]
struct TranscriptPaintBackend {
    round_rects: Vec<(Rect, f32)>,
    ovals: usize,
    texts: Vec<String>,
}

impl crate::RenderBackend for TranscriptPaintBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: crate::Color) {}
    fn stroke_rect(&mut self, _: Rect, _: crate::Color, _: f32) {}
    fn draw_text(&mut self, layout: &crate::TextLayout, _: Point2D) {
        if let Some(run) = layout.runs().first() {
            self.texts.push(run.content.clone());
        }
    }
    fn clip_rect(&mut self, _: Rect) {}
    fn save(&mut self) {}
    fn restore(&mut self) {}
    fn translate(&mut self, _: Point2D) {}
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: crate::Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, _: crate::Color) {
        self.round_rects.push((rect, radius));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: crate::Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: crate::Color, _: f32) {}
    fn fill_oval(&mut self, _: Rect, _: crate::Color) {
        self.ovals += 1;
    }
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
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
    let messages = [ChatMessage::user("user prompt")];
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
        },
        ChatToolCall {
            name: "set_fill_hex".into(),
            args: "{}".into(),
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
fn transcript_hit_resolves_a_click_on_the_thinking_header() {
    let mut m = ChatMessage::assistant("answer");
    m.thinking = "reasoning".into();
    let msgs = std::slice::from_ref(&m);
    let header = build_transcript(msgs, body(), op_editor_core::Locale::EnUs)[0]
        .thinking
        .as_ref()
        .unwrap()
        .header;
    let cx = header.origin.x + header.size.x / 2.0;
    let cy = header.origin.y + header.size.y / 2.0;
    assert_eq!(
        transcript_hit(msgs, body(), cx, cy, op_editor_core::Locale::EnUs),
        Some(TranscriptHit::ToggleThinking(0))
    );
}

#[test]
fn transcript_hit_resolves_a_click_on_the_tool_header() {
    let mut m = ChatMessage::assistant("answer");
    m.tool_calls = vec![ChatToolCall {
        name: "insert_node".into(),
        args: "{}".into(),
    }];
    let msgs = std::slice::from_ref(&m);
    let header = build_transcript(msgs, body(), op_editor_core::Locale::EnUs)[0]
        .tools
        .as_ref()
        .unwrap()
        .header;
    let cx = header.origin.x + header.size.x / 2.0;
    let cy = header.origin.y + header.size.y / 2.0;
    assert_eq!(
        transcript_hit(msgs, body(), cx, cy, op_editor_core::Locale::EnUs),
        Some(TranscriptHit::ToggleToolCalls(0))
    );
}

#[test]
fn transcript_hit_misses_when_the_click_is_not_on_a_header() {
    let m = ChatMessage::assistant("plain answer, no thinking, no tools");
    let msgs = std::slice::from_ref(&m);
    // Click far below the single short message.
    assert_eq!(
        transcript_hit(msgs, body(), 20.0, 280.0, op_editor_core::Locale::EnUs),
        None
    );
}
