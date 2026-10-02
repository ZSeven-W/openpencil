//! Image thumbnails and markdown narration rendering.
//!
//! Split out of `ai_chat_transcript_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn continuous_cjk_narration_wraps_without_clipping_or_losing_styled_text() {
    use crate::widgets::ai_chat_transcript_richtext::{layout_rich, SpanStyle};
    use crate::widgets::ai_chat_transcript_text::char_display_units;
    let text = "看了整个页面，**搜索框的提示文字预留空间太窄**，请把`提示文字`完整显示出来。";
    let lines = layout_rich(text, 28);
    assert!(lines.len() > 2, "continuous Chinese must wrap");
    for line in &lines {
        let units: u32 = line
            .spans
            .iter()
            .flat_map(|s| s.text.chars())
            .map(char_display_units)
            .sum();
        assert!(units <= 28, "a clipped line would hide its tail: {line:?}");
    }
    let joined: String = lines
        .iter()
        .flat_map(|l| &l.spans)
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(
        joined,
        "看了整个页面，搜索框的提示文字预留空间太窄，请把提示文字完整显示出来。"
    );
    assert!(lines
        .iter()
        .flat_map(|l| &l.spans)
        .any(|s| s.style == SpanStyle::Strong));
    assert!(lines
        .iter()
        .flat_map(|l| &l.spans)
        .any(|s| s.style == SpanStyle::Code));
}

#[test]
fn long_narration_links_wrap_and_bullets_keep_their_indent() {
    use crate::widgets::ai_chat_transcript_richtext::{layout_rich, BULLET_INDENT};
    use crate::widgets::ai_chat_transcript_text::char_display_units;
    let url = "https://example.com/a/very/long/reference/with/no/spaces";
    let lines = layout_rich(&format!("- {url}"), 24);
    assert!(lines.len() > 2);
    assert!(lines[0].bullet);
    assert!(lines.iter().skip(1).all(|l| !l.bullet));
    assert!(lines.iter().all(|l| l.inset == BULLET_INDENT));
    for line in &lines {
        let units: u32 = line
            .spans
            .iter()
            .flat_map(|s| s.text.chars())
            .map(char_display_units)
            .sum();
        assert!(units <= 22, "bullet inset must stay inside the wrap budget");
    }
    let joined: String = lines
        .iter()
        .flat_map(|l| &l.spans)
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(joined, url);
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
