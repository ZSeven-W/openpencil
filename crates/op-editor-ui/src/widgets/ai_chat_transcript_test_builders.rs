//! Test-only transcript builders / painters.
//!
//! Split out of `ai_chat_transcript.rs` to keep that production file under the
//! 800-line cap. These helpers materialise a scroll-applied layout or paint a
//! transcript without a pre-resolved canonical build — conveniences the tests
//! lean on. Production never uses them: it resolves the canonical (scroll-0)
//! build once at the panel entry points, applies scroll with a `translate`, and
//! shifts hit-test points. Re-exported from `ai_chat_transcript` so existing
//! `build_transcript` / `paint_transcript` / `transcript_effective_offset`
//! call sites resolve unchanged.

use super::{
    build_item, effective_offset_of, paint_transcript_with_selection, TranscriptItem, MSG_GAP,
};
use crate::theme::Theme;
use crate::widgets::ai_chat_transcript_cache::unowned_for_tests;
use crate::widgets::PaintCx;
use crate::Rect;
use op_editor_core::chat::ChatMessage;

/// Lay out the full transcript from the body top (no scroll offset).
/// Each item carries absolute rects ready for paint and hit-test.
/// Test-only convenience over [`build_transcript_with_design_hover`];
/// production reads the shared cache via
/// [`crate::widgets::ai_chat_transcript_cache::unowned_for_tests`].
pub(crate) fn build_transcript(
    messages: &[ChatMessage],
    body_rect: Rect,
    locale: op_editor_core::Locale,
) -> Vec<TranscriptItem> {
    build_transcript_with_design_hover(messages, body_rect, locale, None, 0.0)
}

/// Lay out every message top-to-bottom, shifted up by `scroll_offset`
/// (px), returning owned items with absolute rects. Kept for tests that
/// assert scroll-applied geometry; production paints the canonical
/// (scroll-0) build under a `translate` and shifts hit-test points, so it
/// never materialises a scroll-applied copy. `design_hover` is accepted
/// for signature compatibility but no longer affects layout — the copy
/// affordance it drove is a paint-time flag (see `paint_design_block`).
pub(crate) fn build_transcript_with_design_hover(
    messages: &[ChatMessage],
    body_rect: Rect,
    locale: op_editor_core::Locale,
    _design_hover: Option<(usize, usize)>,
    scroll_offset: f32,
) -> Vec<TranscriptItem> {
    if messages.is_empty() {
        return Vec::new();
    }
    let mut items = Vec::new();
    let mut top = body_rect.origin.y - scroll_offset;
    for (i, msg) in messages.iter().enumerate() {
        let (item, bottom) = build_item(msg, i, top, body_rect, locale);
        items.push(item);
        top = bottom + MSG_GAP;
    }
    items
}

/// Effective scroll offset resolved straight from `messages` — a thin wrapper
/// over [`effective_offset_of`] that fingerprints the transcript itself. Kept
/// for tests; production resolves the canonical build once at the panel entry
/// points and calls [`effective_offset_of`] to avoid a second fingerprint.
pub(crate) fn transcript_effective_offset(
    messages: &[ChatMessage],
    body_rect: Rect,
    locale: op_editor_core::Locale,
    offset: f32,
    pinned: bool,
) -> f32 {
    let canonical = unowned_for_tests(messages, body_rect, locale);
    effective_offset_of(&canonical, body_rect, offset, pinned)
}

/// Paint the chat transcript — the tail of `messages` that fits in
/// `body_rect`, with collapsible thinking / tool blocks, image
/// thumbnails and the streaming animation on the in-flight message.
pub(crate) fn paint_transcript(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    body_rect: Rect,
    messages: &[ChatMessage],
    now_ms: u64,
    locale: op_editor_core::Locale,
) {
    paint_transcript_with_design_hover(cx, theme, body_rect, messages, now_ms, locale, None);
}

pub(crate) fn paint_transcript_with_design_hover(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    body_rect: Rect,
    messages: &[ChatMessage],
    now_ms: u64,
    locale: op_editor_core::Locale,
    design_hover: Option<(usize, usize)>,
) {
    let canonical = unowned_for_tests(messages, body_rect, locale);
    paint_transcript_with_selection(
        cx,
        theme,
        body_rect,
        messages,
        &canonical,
        now_ms,
        design_hover,
        None,
        0.0,
    );
}
