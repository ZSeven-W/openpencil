//! Transcript layout-cache + canonical-build hit-test coverage. Split out of
//! `ai_chat_transcript_tests.rs` to keep that file under its pre-existing size
//! (the Task 4 cache tests would otherwise push it well over). These all drive
//! the shared canonical build directly (`unowned_for_tests` +
//! `transcript_hit`).

use super::*;
use op_editor_core::chat::ChatToolCall;

fn body() -> Rect {
    Rect::xywh(0.0, 0.0, 340.0, 300.0)
}

/// Resolve the canonical (scroll-0) build for `msgs` at the standard test
/// [`body`] rect. Mirrors what the panel entry points now do: fingerprint +
/// resolve once, then hand the build to the hit/selection helpers.
fn canonical_at_body(
    msgs: &[ChatMessage],
) -> std::rc::Rc<crate::widgets::ai_chat_transcript_cache::CanonicalTranscript> {
    crate::widgets::ai_chat_transcript_cache::unowned_for_tests(
        msgs,
        body(),
        op_editor_core::Locale::EnUs,
    )
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
        transcript_hit(&canonical_at_body(msgs), body(), cx, cy, 0.0),
        Some(TranscriptHit::ToggleThinking(0))
    );
}

#[test]
fn transcript_hit_resolves_a_click_on_the_tool_header() {
    let mut m = ChatMessage::assistant("answer");
    m.tool_calls = vec![ChatToolCall {
        name: "insert_node".into(),
        args: "{}".into(),
        content_offset: None,
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
        transcript_hit(&canonical_at_body(msgs), body(), cx, cy, 0.0),
        Some(TranscriptHit::ToggleToolCalls(0))
    );
}

#[test]
fn transcript_hit_resolves_a_click_on_an_individual_tool_card_header() {
    let mut m = ChatMessage::assistant("answer");
    m.tools_collapsed = false;
    m.tool_calls = vec![ChatToolCall {
        name: "snapshot_layout".into(),
        args: r#"{"args":{"pageId":"page-1"}}"#.into(),
        content_offset: None,
    }];
    let msgs = std::slice::from_ref(&m);
    let card_header = build_transcript(msgs, body(), op_editor_core::Locale::EnUs)[0]
        .tools
        .as_ref()
        .unwrap()
        .cards[0]
        .header;
    let cx = card_header.origin.x + card_header.size.x / 2.0;
    let cy = card_header.origin.y + card_header.size.y / 2.0;
    assert_eq!(
        transcript_hit(&canonical_at_body(msgs), body(), cx, cy, 0.0),
        Some(TranscriptHit::SetToolCallCardExpanded(0, 0, true))
    );
}

#[test]
fn transcript_hit_resolves_a_click_on_a_design_block_header() {
    let m = ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    );
    let msgs = std::slice::from_ref(&m);
    let header =
        build_transcript(msgs, body(), op_editor_core::Locale::EnUs)[0].design_blocks[0].header;
    let cx = header.origin.x + header.size.x / 2.0;
    let cy = header.origin.y + header.size.y / 2.0;
    assert_eq!(
        transcript_hit(&canonical_at_body(msgs), body(), cx, cy, 0.0),
        Some(TranscriptHit::SetDesignBlockExpanded(0, 0, true))
    );
}

#[test]
fn transcript_hit_misses_when_the_click_is_not_on_a_header() {
    let m = ChatMessage::assistant("plain answer, no thinking, no tools");
    let msgs = std::slice::from_ref(&m);
    // Click far below the single short message.
    assert_eq!(
        transcript_hit(&canonical_at_body(msgs), body(), 20.0, 280.0, 0.0),
        None
    );
}

// --- Task 4: transcript layout cache (build once per frame) ---------------
// The build counter is a monotonic per-thread total; each test reads a fresh
// baseline immediately before acting and asserts the *delta*, so tests sharing
// a worker thread don't interfere. Every message uses a unique string so its
// cache key can't collide with another test's primed entry.

#[test]
fn transcript_layout_builds_once_then_serves_from_cache() {
    use crate::widgets::ai_chat_transcript_cache::{transcript_build_count, unowned_for_tests};
    let msgs = [ChatMessage::assistant(
        "cache probe alpha — unchanged inputs",
    )];
    let b = body();
    let loc = op_editor_core::Locale::EnUs;

    let before = transcript_build_count();
    let _ = unowned_for_tests(&msgs, b, loc);
    let after_first = transcript_build_count();
    assert_eq!(after_first, before + 1, "first access builds exactly once");

    // Height, a repeat build request, and another all reuse the one build —
    // this is the two-builds-per-frame collapse (height + paint) plus extras.
    let _ = transcript_content_height(&msgs, b, loc);
    let _ = unowned_for_tests(&msgs, b, loc);
    let _ = unowned_for_tests(&msgs, b, loc);
    assert_eq!(
        transcript_build_count(),
        after_first,
        "unchanged inputs are served from the cache, never rebuilt"
    );
}

#[test]
fn transcript_layout_rebuilds_when_message_content_changes() {
    use crate::widgets::ai_chat_transcript_cache::{transcript_build_count, unowned_for_tests};
    let b = body();
    let loc = op_editor_core::Locale::EnUs;

    let short = [ChatMessage::assistant(
        "cache probe beta — streaming step 1",
    )];
    let _ = unowned_for_tests(&short, b, loc);
    let base = transcript_build_count();
    // Identical re-access: no rebuild.
    let _ = unowned_for_tests(&short, b, loc);
    assert_eq!(
        transcript_build_count(),
        base,
        "identical inputs reuse the build"
    );

    // A growing streaming message must invalidate — a stale transcript would be
    // a visible bug while a turn streams.
    let grown = [ChatMessage::assistant(
        "cache probe beta — streaming step 1 with more tokens now",
    )];
    let _ = unowned_for_tests(&grown, b, loc);
    assert_eq!(
        transcript_build_count(),
        base + 1,
        "changed message content forces a rebuild"
    );
}

#[test]
fn transcript_hit_test_reuses_the_cached_layout() {
    use crate::widgets::ai_chat_transcript_cache::{transcript_build_count, unowned_for_tests};
    let mut m = ChatMessage::assistant("cache probe gamma — clickable header");
    m.thinking = "reasoning long enough to render a clickable thinking header".into();
    let msgs = std::slice::from_ref(&m);
    let b = body();
    let loc = op_editor_core::Locale::EnUs;

    // Prime the cache as a paint frame would; the entry point resolves the
    // canonical once and hands it to the hit helper.
    let canonical = unowned_for_tests(msgs, b, loc);
    let base = transcript_build_count();

    // Repeated mouse-move hit-tests (the biggest win) reuse the one build and
    // never rebuild.
    for _ in 0..8 {
        let _ = transcript_hit(&canonical, b, 20.0, 30.0, 0.0);
    }
    assert_eq!(
        transcript_build_count(),
        base,
        "hit-tests read the cached layout instead of rebuilding per mouse move"
    );
}

#[test]
fn with_current_canonical_reads_stored_build_without_fingerprinting() {
    // F3(a): the read-only accessor returns whatever build is currently stored
    // (= last painted) as a PURE read — it must not fingerprint or rebuild.
    use crate::widgets::ai_chat_transcript_cache::{
        cached_canonical_transcript_owned, transcript_build_count, transcript_fingerprint_count,
        with_current_canonical,
    };
    let msgs = [ChatMessage::assistant(
        "current-build accessor probe — unique",
    )];
    let b = body();
    let loc = op_editor_core::Locale::EnUs;

    // Paint resolves the canonical under a real owner, storing it as the
    // "current" build. (UNOWNED can never read back — see the protocol — so a
    // real owner is required for this accessor test.)
    let owner = crate::widgets::AIChatPlaceholder::next_owner();
    let painted = cached_canonical_transcript_owned(owner, &msgs, b, loc);
    let fp_base = transcript_fingerprint_count();
    let build_base = transcript_build_count();

    // The accessor hands back the stored build (same items + height) and hashes
    // / rebuilds nothing. It reads under the SAME owner the store used.
    let (item_count, height) = with_current_canonical(owner, |current| {
        let (_, c) = current.expect("a build was just stored by paint");
        (c.items.len(), c.total_height)
    });
    assert_eq!(item_count, painted.items.len(), "same stored items");
    assert_eq!(height, painted.total_height, "same stored height");
    assert_eq!(
        transcript_fingerprint_count(),
        fp_base,
        "with_current_canonical must not fingerprint the transcript"
    );
    assert_eq!(
        transcript_build_count(),
        build_base,
        "with_current_canonical must not rebuild the transcript"
    );
}

#[test]
fn transcript_scroll_change_alone_does_not_rebuild() {
    use crate::widgets::ai_chat_transcript_cache::{transcript_build_count, unowned_for_tests};
    // Enough messages to overflow the body so a real scroll range exists.
    let msgs: Vec<_> = (0..40)
        .map(|i| ChatMessage::user(format!("cache probe delta scroll {i}")))
        .collect();
    let b = body();
    let loc = op_editor_core::Locale::EnUs;

    let canonical = unowned_for_tests(&msgs, b, loc);
    let base = transcript_build_count();
    // Hit-testing at different scroll offsets shares the one canonical build —
    // scroll is applied by shifting the query point, not by rebuilding.
    let _ = transcript_hit(&canonical, b, 20.0, 30.0, 0.0);
    let _ = transcript_hit(&canonical, b, 20.0, 30.0, 120.0);
    let _ = transcript_hit(&canonical, b, 20.0, 30.0, 260.0);
    assert_eq!(
        transcript_build_count(),
        base,
        "a scroll-only change stays on the cached layout"
    );
}

#[test]
fn current_build_read_is_scoped_to_the_resolving_owner() {
    // (a) Owner A resolves (stores) the canonical build. A read under a DIFFERENT
    // owner B must return `None` — no cross-pairing of A's cached geometry with
    // B's live messages — and must not hash. Reading under A still sees it.
    use crate::widgets::ai_chat_transcript_cache::{
        cached_canonical_transcript_owned, transcript_fingerprint_count, with_current_canonical,
    };
    let owner_a = crate::widgets::AIChatPlaceholder::next_owner();
    let owner_b = crate::widgets::AIChatPlaceholder::next_owner();
    assert_ne!(
        owner_a, owner_b,
        "each panel instance gets a distinct owner"
    );

    let msgs = [ChatMessage::assistant("owner-scope probe — unique content")];
    let b = body();
    let loc = op_editor_core::Locale::EnUs;

    // Owner A paints/resolves → stores a build tagged with owner A.
    let stored = cached_canonical_transcript_owned(owner_a, &msgs, b, loc);

    // Owner B's display-frame hint read: None, and it hashes nothing.
    let fp_before = transcript_fingerprint_count();
    let b_saw = with_current_canonical(owner_b, |current| current.map(|(_, c)| c.items.len()));
    assert_eq!(
        b_saw, None,
        "a different owner must not read the stored build"
    );
    assert_eq!(
        transcript_fingerprint_count(),
        fp_before,
        "the owner-mismatch read must not fingerprint the transcript"
    );

    // Owner A still sees its own build.
    let a_saw = with_current_canonical(owner_a, |current| current.map(|(_, c)| c.items.len()));
    assert_eq!(
        a_saw,
        Some(stored.items.len()),
        "the resolving owner reads back its own build"
    );
}

#[test]
fn identical_key_resolve_by_a_second_owner_claims_the_slot() {
    // Finding 1: owner A builds key K; owner B then RESOLVES byte-identical inputs
    // (a cache HIT — no rebuild). Under "last resolver owns", B's resolve re-stamps
    // the slot, so B's later hint read returns the build and A's now returns None.
    // Without the re-tag-on-hit rule B could hit the cache yet never own it, and
    // its hint would stay None forever (the bug this fixes).
    use crate::widgets::ai_chat_transcript_cache::{
        cached_canonical_transcript_owned, transcript_build_count, with_current_canonical,
    };
    let owner_a = crate::widgets::AIChatPlaceholder::next_owner();
    let owner_b = crate::widgets::AIChatPlaceholder::next_owner();

    let msgs = [ChatMessage::assistant(
        "identical-key claim probe — unique content",
    )];
    let b = body();
    let loc = op_editor_core::Locale::EnUs;

    // A resolves/stores under owner A.
    let built_a = cached_canonical_transcript_owned(owner_a, &msgs, b, loc);
    let builds_after_a = transcript_build_count();

    // B resolves the SAME inputs → a cache hit (no rebuild) that claims the slot.
    let built_b = cached_canonical_transcript_owned(owner_b, &msgs, b, loc);
    assert_eq!(
        transcript_build_count(),
        builds_after_a,
        "B's identical-key resolve is a cache HIT, not a rebuild"
    );
    assert_eq!(
        built_b.items.len(),
        built_a.items.len(),
        "B is served the same build A produced"
    );

    // B now owns the slot: B reads it back, A no longer can.
    let b_saw = with_current_canonical(owner_b, |current| current.map(|(_, c)| c.items.len()));
    assert_eq!(
        b_saw,
        Some(built_b.items.len()),
        "the last resolver (B) owns and reads the build"
    );
    let a_saw = with_current_canonical(owner_a, |current| current.map(|(_, c)| c.items.len()));
    assert_eq!(
        a_saw, None,
        "A's read returns None after B claimed the slot"
    );
}

#[test]
fn stale_build_with_more_items_than_live_messages_is_no_hit_not_a_panic() {
    // (b) A cached build laid out for a LONGER transcript is probed against a
    // SHRUNKEN live `messages` slice (messages removed between the resolve and
    // this probe). Every `msg_index` consumer must stay safe: the message-indexing
    // selection path returns None via `.get` (never index-and-panic), and the
    // canonical-only hit path returns without panicking. Coverage spans EVERY item
    // kind the layout can carry — user bubble, thinking, tool, design, action —
    // each placed at an index the shrunken live slice no longer contains.
    let selection_at = crate::widgets::ai_chat_transcript_selection::transcript_text_offset_at;

    // A 6-message transcript, one per kind, so every kind's item lands at a
    // distinct msg_index (1..=5) beyond the shrunken live slice (len 1).
    let mut thinking = ChatMessage::assistant("thinking answer — unique");
    thinking.thinking = "reasoning long enough to render a clickable header".into();
    let mut tool = ChatMessage::assistant("tool answer — unique");
    tool.tool_calls = vec![ChatToolCall {
        name: "insert_node".into(),
        args: "{}".into(),
        content_offset: None,
    }];
    let design = ChatMessage::assistant(
        "design answer — unique\n```json\n[{\"id\":\"frame-1\",\"type\":\"Frame\"}]\n```",
    );
    let action =
        ChatMessage::assistant("<step title=\"Sketch\">did some work</step> done — unique");
    let long: Vec<ChatMessage> = vec![
        ChatMessage::user("bounds-defense kept message 0 — unique"),
        thinking,
        tool,
        design,
        action,
        ChatMessage::user("bounds-defense selectable bubble — unique"),
    ];
    let build = canonical_at_body(&long);

    // Live messages shrank to a single entry; indices 1..=5 are now out of range.
    let short = std::slice::from_ref(&long[0]);
    let b = body();
    let center = |r: Rect| Point2D::new(r.origin.x + r.size.x / 2.0, r.origin.y + r.size.y / 2.0);

    // Locate each kind's interactive rect in the stale build by its msg_index.
    let mut thinking_pt = None;
    let mut tool_pt = None;
    let mut design_pt = None;
    let mut action_pt = None;
    let mut bubble_pt = None;
    for it in &build.items {
        if let Some(t) = &it.thinking {
            thinking_pt = Some((it.msg_index, center(t.header)));
        }
        if let Some(t) = &it.tools {
            tool_pt = Some((it.msg_index, center(t.header)));
        }
        if let Some(block) = it.design_blocks.first() {
            design_pt = Some((it.msg_index, center(block.header)));
        }
        if let Some(step) = it.steps.first() {
            let header = Rect::xywh(
                step.rect.origin.x,
                step.rect.origin.y,
                step.rect.size.x,
                ACTION_STEP_H,
            );
            action_pt = Some((it.msg_index, center(header)));
        }
        if it.role == op_editor_core::chat::ChatRole::User {
            if let Some(bub) = &it.bubble {
                // The LAST user bubble (msg_index 5) is the message-indexing target.
                bubble_pt = Some((it.msg_index, center(bub.rect)));
            }
        }
    }

    let (t_idx, t_pt) = thinking_pt.expect("thinking item laid out");
    let (tool_idx, tool_p) = tool_pt.expect("tool item laid out");
    let (d_idx, d_pt) = design_pt.expect("design item laid out");
    let (a_idx, a_pt) = action_pt.expect("action item laid out");
    let (b_idx, b_pt) = bubble_pt.expect("user bubble laid out");
    assert_eq!(b_idx, 5, "the selectable bubble indexes messages[5]");

    // Assistant kinds: the canonical-only hit path returns the toggle carrying the
    // OOB index WITHOUT panicking (it never indexes the live slice), and the
    // message-indexing selection path yields no false hit.
    assert_eq!(
        transcript_hit(&build, b, t_pt.x, t_pt.y, 0.0),
        Some(TranscriptHit::ToggleThinking(t_idx)),
        "thinking header still hits safely against the stale build"
    );
    assert_eq!(selection_at(short, &build, b, t_pt, 0.0), None);
    assert_eq!(
        transcript_hit(&build, b, tool_p.x, tool_p.y, 0.0),
        Some(TranscriptHit::ToggleToolCalls(tool_idx)),
        "tool header still hits safely"
    );
    assert_eq!(selection_at(short, &build, b, tool_p, 0.0), None);
    assert_eq!(
        transcript_hit(&build, b, d_pt.x, d_pt.y, 0.0),
        Some(TranscriptHit::SetDesignBlockExpanded(d_idx, 0, true)),
        "design header still hits safely"
    );
    assert_eq!(selection_at(short, &build, b, d_pt, 0.0), None);
    assert_eq!(
        transcript_hit(&build, b, a_pt.x, a_pt.y, 0.0),
        Some(TranscriptHit::SetActionStepExpanded(a_idx, 0, true)),
        "action step header still hits safely"
    );
    assert_eq!(selection_at(short, &build, b, a_pt, 0.0), None);

    // User bubble at the OOB index: the selection path indexes messages[5] via
    // `.get` → None (the core bounds-defense — an OOB index would panic without
    // the guard); the header hit path stays None and does not panic.
    assert_eq!(
        selection_at(short, &build, b, b_pt, 0.0),
        None,
        "out-of-range selection index yields no hit, not a panic"
    );
    assert_eq!(transcript_hit(&build, b, b_pt.x, b_pt.y, 0.0), None);
}
