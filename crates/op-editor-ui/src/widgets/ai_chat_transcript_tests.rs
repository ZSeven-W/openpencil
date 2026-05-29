use super::*;

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
fn streaming_message_with_no_text_yields_a_typing_bubble() {
    let msgs = vec![ChatMessage::assistant_streaming()];
    let items = build_transcript(&msgs, body(), op_editor_core::Locale::EnUs);
    assert_eq!(items.len(), 1);
    assert!(items[0].streaming);
    let bubble = items[0].bubble.as_ref().expect("typing bubble present");
    assert!(bubble.typing, "empty in-flight message shows typing dots");
    assert!(bubble.lines.is_empty());
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
fn design_progress_lines_do_not_render_as_reasoning_or_typing_placeholder() {
    let mut m = ChatMessage::assistant_streaming();
    m.thinking = "\n• Planning…\n• Scaffold ready".into();
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 2);
    assert_eq!(items[0].steps[0].label, "Planning…");
    assert!(items[0].steps[0].done);
    assert!(items[0].steps[1].done);
    assert!(
        items[0].thinking.is_none(),
        "design progress should render as action steps, not a thinking block"
    );
    assert!(
        items[0].bubble.is_none(),
        "design progress should replace the empty streaming typing placeholder"
    );
}

#[test]
fn current_design_progress_step_is_active_until_terminal() {
    let mut m = ChatMessage::assistant_streaming();
    m.thinking = "• Planning…".into();
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
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
