//! Nonzero-scroll paint / hit-test / selection tests for the transcript.
//!
//! The scroll paint path is `save → clip(body) → translate(0, -scroll) →
//! items → restore`. Earlier coverage only asserted the build counter, and the
//! default test backends treat `translate` as a no-op, so the transform was
//! never actually exercised. These tests use a transform-aware backend that
//! folds the recorded translate into every primitive's coordinates, and assert
//! hit-test / selection equivalence between a scrolled click and its unscrolled
//! position.

use super::*;
use op_editor_core::chat::ChatMessage;

fn scroll_body() -> Rect {
    Rect::xywh(0.0, 0.0, 340.0, 300.0)
}

fn resolve(
    msgs: &[ChatMessage],
) -> std::rc::Rc<crate::widgets::ai_chat_transcript_cache::CanonicalTranscript> {
    crate::widgets::ai_chat_transcript_cache::unowned_for_tests(
        msgs,
        scroll_body(),
        op_editor_core::Locale::EnUs,
    )
}

/// One recorded drawing op, in call order, with the active translate folded
/// into any coordinate it carries.
#[derive(Debug, Clone, PartialEq)]
enum Op {
    Save,
    Restore,
    Clip,
    Translate(f32),
    /// A rounded rect: translated top-y and its corner radius.
    RoundRect(f32, f32),
}

/// A backend that models the skia translate stack: `save`/`restore` push/pop
/// the current translate, `translate` accumulates it, and every primitive is
/// recorded with the translate applied — the way a real canvas paints it.
#[derive(Default)]
struct TransformBackend {
    ops: Vec<Op>,
    translate_y: f32,
    stack: Vec<f32>,
    saves: usize,
    restores: usize,
}

impl TransformBackend {
    /// Translated top-y of the first rounded rect painted at `radius`.
    fn first_round_rect_y(&self, radius: f32) -> Option<f32> {
        self.ops.iter().find_map(|op| match op {
            Op::RoundRect(y, r) if (*r - radius).abs() < 1e-4 => Some(*y),
            _ => None,
        })
    }
}

impl crate::RenderBackend for TransformBackend {
    fn begin_frame(&mut self) {}
    fn end_frame(&mut self) {}
    fn fill_rect(&mut self, _: Rect, _: crate::Color) {}
    fn stroke_rect(&mut self, _: Rect, _: crate::Color, _: f32) {}
    fn draw_text(&mut self, _: &crate::TextLayout, _: Point2D) {}
    fn clip_rect(&mut self, _: Rect) {
        self.ops.push(Op::Clip);
    }
    fn save(&mut self) {
        self.stack.push(self.translate_y);
        self.saves += 1;
        self.ops.push(Op::Save);
    }
    fn restore(&mut self) {
        if let Some(prev) = self.stack.pop() {
            self.translate_y = prev;
        }
        self.restores += 1;
        self.ops.push(Op::Restore);
    }
    fn translate(&mut self, p: Point2D) {
        self.translate_y += p.y;
        self.ops.push(Op::Translate(p.y));
    }
    fn stroke_line(&mut self, _: Point2D, _: Point2D, _: crate::Color, _: f32) {}
    fn fill_round_rect(&mut self, rect: Rect, radius: f32, _: crate::Color) {
        self.ops
            .push(Op::RoundRect(rect.origin.y + self.translate_y, radius));
    }
    fn stroke_round_rect(&mut self, _: Rect, _: f32, _: crate::Color, _: f32) {}
    fn stroke_svg_path(&mut self, _: &str, _: Point2D, _: f32, _: crate::Color, _: f32) {}
    fn fill_oval(&mut self, _: Rect, _: crate::Color) {}
    fn resize(&mut self, _: u32, _: u32) {}
    fn dpi_scale(&self) -> f32 {
        1.0
    }
}

fn paint_at_scroll(messages: &[ChatMessage], scroll: f32) -> TransformBackend {
    let canonical = resolve(messages);
    let mut backend = TransformBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };
    paint_transcript_with_selection(
        &mut cx,
        &crate::Theme::dark(),
        scroll_body(),
        messages,
        &canonical,
        0,
        None,
        None,
        scroll,
    );
    backend
}

#[test]
fn paint_translates_items_by_scroll_and_balances_save_restore() {
    // A single user bubble → exactly one radius-14 rounded rect to track.
    let messages = [ChatMessage::user("scroll paint probe — one bubble")];
    let scroll = 100.0_f32;

    let unscrolled = paint_at_scroll(&messages, 0.0);
    let scrolled = paint_at_scroll(&messages, scroll);

    let y0 = unscrolled
        .first_round_rect_y(14.0)
        .expect("user bubble paints a radius-14 rounded rect at scroll 0");
    let y_s = scrolled
        .first_round_rect_y(14.0)
        .expect("user bubble paints a radius-14 rounded rect at scroll S");

    // The painted y at scroll S equals its y at scroll 0 minus S — the transform
    // is a pure constant vertical shift, applied by `translate`, not a rebuild.
    assert!(
        (y_s - (y0 - scroll)).abs() < 1e-4,
        "painted y at scroll {scroll} ({y_s}) should equal scroll-0 y ({y0}) minus {scroll}"
    );

    // Every save is matched by a restore in both passes.
    assert_eq!(
        unscrolled.saves, unscrolled.restores,
        "save/restore balanced at scroll 0"
    );
    assert_eq!(
        scrolled.saves, scrolled.restores,
        "save/restore balanced at scroll S"
    );
    assert!(scrolled.saves >= 1);

    // Ordering: the scrolled pass opens save → clip(body) → translate before it
    // paints any item.
    assert_eq!(scrolled.ops[0], Op::Save);
    assert_eq!(scrolled.ops[1], Op::Clip);
    assert_eq!(scrolled.ops[2], Op::Translate(-scroll));
    // At scroll 0 the translate is skipped entirely (no redundant transform).
    assert!(
        !unscrolled
            .ops
            .iter()
            .any(|op| matches!(op, Op::Translate(_))),
        "a scroll-0 paint issues no translate"
    );
}

#[test]
fn hit_test_at_nonzero_scroll_matches_unscrolled_position() {
    // Two messages so the assistant's clickable thinking header sits below the
    // body top, leaving room to scroll it up and still land inside the body.
    let mut assistant = ChatMessage::assistant("answer body");
    assistant.thinking = "reasoning long enough to render a clickable header".into();
    let messages = [ChatMessage::user("hello there"), assistant];
    let canonical = resolve(&messages);

    let header = build_transcript(&messages, scroll_body(), op_editor_core::Locale::EnUs)[1]
        .thinking
        .as_ref()
        .expect("assistant thinking header")
        .header;
    let hx = header.origin.x + header.size.x / 2.0;
    let hy = header.origin.y + header.size.y / 2.0;
    let scroll = 20.0_f32;
    assert!(
        hy - scroll >= scroll_body().origin.y,
        "scrolled click stays in body"
    );

    let unscrolled = transcript_hit(&canonical, scroll_body(), hx, hy, 0.0);
    // Same item, now scrolled up by `scroll` — click its on-screen position.
    let scrolled = transcript_hit(&canonical, scroll_body(), hx, hy - scroll, scroll);

    assert_eq!(unscrolled, Some(TranscriptHit::ToggleThinking(1)));
    assert_eq!(
        scrolled, unscrolled,
        "clicking a scrolled item hits the same message as clicking its unscrolled position"
    );
}

#[test]
fn selection_at_nonzero_scroll_matches_unscrolled_position() {
    // Assistant first so the selectable user bubble is low enough to scroll up
    // while its on-screen click stays inside the body.
    let prompt = "生成一个设计精良的美食应用移动端首页";
    let messages = [ChatMessage::assistant("intro"), ChatMessage::user(prompt)];
    let canonical = resolve(&messages);

    let bubble = build_transcript(&messages, scroll_body(), op_editor_core::Locale::EnUs)[1]
        .bubble
        .as_ref()
        .expect("user prompt bubble")
        .rect;
    let point = Point2D::new(
        bubble.origin.x + USER_BUBBLE_PAD + 22.0,
        bubble.origin.y + USER_BUBBLE_PAD + 2.0,
    );
    let scroll = 20.0_f32;
    assert!(
        point.y - scroll >= scroll_body().origin.y,
        "scrolled click stays in body"
    );

    let unscrolled = transcript_text_offset_at(&messages, &canonical, scroll_body(), point, 0.0);
    let scrolled = transcript_text_offset_at(
        &messages,
        &canonical,
        scroll_body(),
        Point2D::new(point.x, point.y - scroll),
        scroll,
    );

    assert!(
        unscrolled.is_some(),
        "the unscrolled click selects user message text"
    );
    assert_eq!(
        scrolled, unscrolled,
        "selecting a scrolled bubble resolves the same message + offset as unscrolled"
    );
}
