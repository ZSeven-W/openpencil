//! Transcript-area tests for [`super::AIChatPlaceholder`] — fixed
//! checklist geometry, tool-card / design-block toggles, and paint
//! assertions. Split out of `tests.rs` at the 800-line cap.

use super::tests::{has_fill_rect, toolbar_center_y, PanelPaintBackend};
use super::*;
use crate::widgets::ai_chat_hit::AIChatHit;

#[test]
fn hit_test_resolves_individual_tool_card_header_toggle() {
    let mut s = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant("answer");
    message.tools_collapsed = false;
    message.tool_calls.push(op_editor_core::ChatToolCall {
        name: "snapshot_layout".into(),
        args: r#"{"args":{"pageId":"page-1"}}"#.into(),
    });
    s.chat.messages.push(message);

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let card_header = crate::widgets::ai_chat_transcript::build_transcript(
        &s.chat.messages,
        panel.body_rect(rect),
        panel.locale,
    )[0]
    .tools
    .as_ref()
    .unwrap()
    .cards[0]
        .header;
    let p = Point2D::new(
        card_header.origin.x + card_header.size.x / 2.0,
        card_header.origin.y + card_header.size.y / 2.0,
    );

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::SetToolCallCardExpanded(0, 0, true))
    );
}

#[test]
fn hit_test_resolves_design_block_header_toggle() {
    let mut s = EditorState::new();
    s.chat.messages.push(op_editor_core::ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    ));

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let header = crate::widgets::ai_chat_transcript::build_transcript(
        &s.chat.messages,
        panel.body_rect(rect),
        panel.locale,
    )[0]
    .design_blocks[0]
        .header;
    let p = Point2D::new(
        header.origin.x + header.size.x / 2.0,
        header.origin.y + header.size.y / 2.0,
    );

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::SetDesignBlockExpanded(0, 0, true))
    );
}

#[test]
fn hit_test_resolves_design_block_copy_button() {
    let code = r#"[{"id":"frame-1","type":"Frame"}]"#;
    let mut s = EditorState::new();
    s.chat
        .messages
        .push(op_editor_core::ChatMessage::assistant(format!(
            r#"```json
{code}
```"#
        )));

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let block = &crate::widgets::ai_chat_transcript::build_transcript(
        &s.chat.messages,
        panel.body_rect(rect),
        panel.locale,
    )[0]
    .design_blocks[0];
    let p = Point2D::new(
        block.header.origin.x + block.header.size.x - 38.0,
        block.header.origin.y + block.header.size.y / 2.0,
    );

    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::CopyDesignBlock(code.to_string()))
    );
}

#[test]
fn paint_model_chip_uses_key_glyph_for_builtin_model() {
    let mut s = EditorState::new();
    s.chat
        .available_models
        .push(op_editor_core::chat::ModelEntry::builtin_with_display_name(
            op_editor_core::chat::AgentProvider::CodexCli,
            "builtin-minimax",
            "MiniMax",
            "builtin:builtin-minimax:MiniMax-M2.7",
            "MiniMax-M2.7",
        ));
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    // In #27 layout the logo sits 8px inside the pill left edge (pill starts at PAD).
    let key_top_left = Point2D::new(
        rect.origin.x + PAD + 8.0,
        rect.origin.y + toolbar_center_y() - 7.0,
    );
    let key_strokes = backend
        .svg_strokes
        .iter()
        .filter(|(top_left, size, _, _)| {
            (top_left.x - key_top_left.x).abs() < 0.01
                && (top_left.y - key_top_left.y).abs() < 0.01
                && (*size - 14.0).abs() < 0.01
        })
        .count();

    assert_eq!(
        key_strokes,
        crate::widgets::icons::Icon::Key.paths().len(),
        "built-in selected model chip should paint the TS-style Key glyph"
    );
}

#[test]
fn paint_draws_header_divider_and_message_body_background() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(10.0, 20.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let input_h = INPUT_BASE_HEIGHT;
    let sep_y = rect.origin.y + rect.size.y - input_h;
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    panel.paint(&mut cx, rect);

    assert!(has_fill_rect(
        &backend.fills,
        Rect::xywh(
            rect.origin.x + 1.0,
            rect.origin.y + HEADER_HEIGHT,
            rect.size.x - 2.0,
            1.0
        )
    ));
    assert!(has_fill_rect(
        &backend.fills,
        Rect::xywh(
            rect.origin.x + 1.0,
            rect.origin.y + HEADER_HEIGHT + 1.0,
            rect.size.x - 2.0,
            sep_y - (rect.origin.y + HEADER_HEIGHT + 1.0),
        )
    ));
}

#[test]
fn paint_pass_fingerprints_transcript_at_most_once() {
    // Concern 1: the paint path once resolved the transcript twice per frame
    // (height + paint) and each lookup re-fingerprinted the whole transcript.
    // The entry point now fingerprints once and threads the build to the scroll
    // clamp and the painter, so a full paint pass hashes the transcript exactly
    // once — the accepted floor.
    use crate::widgets::ai_chat_transcript_cache::transcript_fingerprint_count;
    let mut s = EditorState::new();
    s.chat.messages.push(op_editor_core::ChatMessage::user(
        "fingerprint paint probe — unique",
    ));
    s.chat.messages.push(op_editor_core::ChatMessage::assistant(
        "answer body content",
    ));
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let mut backend = PanelPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    let before = transcript_fingerprint_count();
    panel.paint(&mut cx, rect);
    assert_eq!(
        transcript_fingerprint_count() - before,
        1,
        "a whole paint pass fingerprints the transcript exactly once"
    );
}

#[test]
fn hit_test_fingerprints_transcript_at_most_once() {
    // Concern 1: the hit path once called transcript_effective_offset,
    // transcript_text_offset_at, and transcript_hit, each re-fingerprinting the
    // transcript (three hashes per event). The entry point now resolves once and
    // hands the build to every probe, so one input event hashes at most once.
    use crate::widgets::ai_chat_transcript_cache::transcript_fingerprint_count;
    let mut s = EditorState::new();
    s.chat.messages.push(op_editor_core::ChatMessage::user(
        "fingerprint hit probe — unique",
    ));
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let body = panel.body_rect(rect);
    // A point inside the transcript body (not on any interactive header) still
    // enters the transcript branch, which resolves the build once.
    let point = Point2D::new(body.origin.x + 4.0, body.origin.y + 4.0);

    let before = transcript_fingerprint_count();
    let _ = panel.hit_test(rect, point);
    assert_eq!(
        transcript_fingerprint_count() - before,
        1,
        "one input event fingerprints the transcript at most once"
    );
}

#[test]
fn cursor_probe_fingerprints_transcript_once_for_the_whole_event() {
    // Finding 1: a physical cursor move used to fingerprint the transcript
    // two or three times — the header-hover hit-test and the design-block hover
    // each resolved independently. `cursor_probe` resolves the canonical build
    // once and returns both the hit and the design-block hover; the host feeds
    // `hit` to the header-hover update and `design_block_hover` to the
    // design-hover update. The redraw-time `cursor_probe` is the single hash per
    // cursor move; the separate native cursor-hint pass reads the stored build
    // with zero hashes (see `hit_test_current_build`). The whole event
    // fingerprints the transcript exactly once here.
    use crate::widgets::ai_chat_transcript_cache::transcript_fingerprint_count;
    let mut s = EditorState::new();
    s.chat.messages.push(op_editor_core::ChatMessage::user(
        "cursor probe fingerprint — unique",
    ));
    s.chat.messages.push(op_editor_core::ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    ));
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let body = panel.body_rect(rect);
    // A point inside the transcript body (not on any interactive header) still
    // enters the transcript branch of hit_test AND the design-block probe.
    let point = Point2D::new(body.origin.x + 4.0, body.origin.y + 4.0);

    let before = transcript_fingerprint_count();
    let probe = panel.cursor_probe(rect, point);
    // The host consumes both fields (hit → header hover + cursor-hint reuse,
    // design_block_hover → design hover) without any further panel call.
    let _ = probe.hit;
    let _ = probe.design_block_hover;
    assert_eq!(
        transcript_fingerprint_count() - before,
        1,
        "a full cursor event (hit + hover + reused cursor-hint) fingerprints the transcript once"
    );
}

#[test]
fn cursor_probe_matches_standalone_hit_and_design_hover() {
    // The combined probe must be behavior-identical to calling the two methods
    // separately — same hit, same design-block hover.
    let mut s = EditorState::new();
    s.chat.messages.push(op_editor_core::ChatMessage::assistant(
        r#"```json
[{"id":"frame-1","type":"Frame"}]
```"#,
    ));
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Aim at the design block so the design-block hover is populated.
    let block = &crate::widgets::ai_chat_transcript::build_transcript(
        &s.chat.messages,
        panel.body_rect(rect),
        panel.locale,
    )[0]
    .design_blocks[0];
    let point = Point2D::new(
        block.rect.origin.x + block.rect.size.x / 2.0,
        block.rect.origin.y + block.rect.size.y / 2.0,
    );

    let probe = panel.cursor_probe(rect, point);
    assert_eq!(probe.hit, panel.hit_test(rect, point));
    assert_eq!(
        probe.design_block_hover,
        panel.design_block_hover_at(rect, point)
    );
    assert_eq!(probe.design_block_hover, Some((0, 0)));
}

#[test]
fn cursor_hint_reads_last_painted_build_with_zero_hashes_and_reflects_mutations() {
    // F3(b): simulate the real native cursor ordering. Paint resolves the
    // canonical build (the event-time `cursor_hint` runs BEFORE the deferred
    // `apply_cursor_move` re-resolves), then the cursor-hint path hit-tests the
    // STORED build: zero new fingerprints, correct hit. After a message
    // mutation + the next paint's re-resolve, the hint reflects the new layout.
    use crate::widgets::ai_chat_transcript_cache::{
        cached_canonical_transcript_owned, transcript_fingerprint_count,
    };
    let mut m = op_editor_core::ChatMessage::assistant("hint build probe — unique base");
    m.thinking = "reasoning long enough to render a clickable thinking header".into();
    let mut s = EditorState::new();
    s.chat.messages = vec![m];

    // A real owner is required: the display-frame hint refuses the UNOWNED
    // sentinel, so both the store and the panel must carry this host's owner.
    let owner = AIChatPlaceholder::next_owner();
    let panel = AIChatPlaceholder::from_editor(&s).owned_by(owner);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let body = panel.body_rect(rect);

    // Paint stores the canonical build under the owner (one hash for the frame).
    let build = cached_canonical_transcript_owned(owner, &s.chat.messages, body, panel.locale);
    let header = build.items[0].thinking.as_ref().unwrap().header;
    let point = Point2D::new(
        header.origin.x + header.size.x / 2.0,
        header.origin.y + header.size.y / 2.0,
    );

    // Cursor-hint ordering: hit-test against the STORED build — no fingerprint.
    let before = transcript_fingerprint_count();
    let hit = panel.hit_test_current_build(rect, point);
    assert_eq!(
        transcript_fingerprint_count() - before,
        0,
        "cursor-hint hit-test against the stored build must not fingerprint"
    );
    assert_eq!(
        hit,
        Some(AIChatHit::ToggleThinking(0)),
        "hit matches the painted layout"
    );

    // Streaming/edit mutation: prepend a message so the transcript re-flows and
    // the thinking header (now message index 1) moves down.
    s.chat.messages.insert(
        0,
        op_editor_core::ChatMessage::user("prepended message — pushes layout down"),
    );
    // The next paint re-resolves and re-stores the canonical for the new state,
    // under the SAME host owner (a real host keeps one owner across paints).
    let panel2 = AIChatPlaceholder::from_editor(&s).owned_by(owner);
    let build2 = cached_canonical_transcript_owned(
        owner,
        &s.chat.messages,
        panel2.body_rect(rect),
        panel2.locale,
    );
    let header2 = build2.items[1].thinking.as_ref().unwrap().header;
    assert!(
        header2.origin.y > header.origin.y,
        "prepended message pushes the thinking header lower"
    );
    let point2 = Point2D::new(
        header2.origin.x + header2.size.x / 2.0,
        header2.origin.y + header2.size.y / 2.0,
    );
    let hit2 = panel2.hit_test_current_build(rect, point2);
    assert_eq!(
        hit2,
        Some(AIChatHit::ToggleThinking(1)),
        "cursor-hint hit reflects the newly painted layout after mutation"
    );
}

#[test]
fn forced_rotation_on_same_index_session_replacement_yields_none_until_repaint() {
    // Review-6 F1/F2 (a): closing active tab 0 installs the next session AT index
    // 0, and closing the sole tab replaces it in place — both leave
    // `active_index()` unchanged, so the index-only poll would MISS the switch.
    // The host instead FORCE-rotates the owner unconditionally at the mutation
    // site (`force_rotate_chat_owner`). This mirrors that host sequence at the
    // cache seam: paint under owner A, then a forced rotation to owner B WITHOUT
    // any index change, and asserts the display-frame hint reads None for B until
    // B's own first paint re-stamps the slot.
    use crate::widgets::ai_chat_transcript_cache::{
        cached_canonical_transcript_owned, next_panel_owner, with_current_canonical,
    };
    let mut s = EditorState::new();
    s.chat.messages = vec![op_editor_core::ChatMessage::assistant(
        "tab-0 transcript — session replaced in place at the same index",
    )];
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);

    // Tab 0 paints under owner A: the slot now belongs to A.
    let owner_a = next_panel_owner();
    let panel_a = AIChatPlaceholder::from_editor(&s).owned_by(owner_a);
    let body = panel_a.body_rect(rect);
    let _ = cached_canonical_transcript_owned(owner_a, &s.chat.messages, body, panel_a.locale);
    assert!(
        with_current_canonical(owner_a, |c| c.is_some()),
        "owner A reads the slot it just painted"
    );

    // Same-index session replacement: the host force-rotates the owner NOW even
    // though `active_index()` is unchanged.
    let owner_b = next_panel_owner();
    // Before B's first paint the hint must read None — the slot still belongs to
    // A, so B never cross-pairs A's cached geometry with B's live messages.
    assert!(
        with_current_canonical(owner_b, |c| c.is_none()),
        "forced-rotated owner B reads None before its own first paint"
    );

    // B's first paint re-resolves + re-stamps the slot; the hint now serves B.
    s.chat.messages = vec![op_editor_core::ChatMessage::assistant(
        "replacement session transcript — freshly installed at index 0",
    )];
    let panel_b = AIChatPlaceholder::from_editor(&s).owned_by(owner_b);
    let _ = cached_canonical_transcript_owned(
        owner_b,
        &s.chat.messages,
        panel_b.body_rect(rect),
        panel_b.locale,
    );
    assert!(
        with_current_canonical(owner_b, |c| c.is_some()),
        "after B's first paint the hint serves B's build"
    );
    assert!(
        with_current_canonical(owner_a, |c| c.is_none()),
        "the old owner A no longer reads the slot"
    );
}

#[test]
fn tab_switch_before_paint_yields_no_cursor_hint_hit() {
    // Review-6 F1 (b): a tab-switch click rotates the owner synchronously; a
    // CursorMoved arriving before the next paint runs the event-time cursor hint
    // (`hit_test_current_build`) against the STORED build. Because the new owner
    // does not yet own the slot, the hint reads None (default arrow) instead of
    // pairing the previous tab's geometry with the new session's messages. This
    // mirrors the native ordering at the reachable op-editor-ui seam.
    use crate::widgets::ai_chat_transcript_cache::{
        cached_canonical_transcript_owned, next_panel_owner,
    };
    let mut m = op_editor_core::ChatMessage::assistant("switch-source transcript — unique base");
    m.thinking = "reasoning long enough to render a clickable thinking header".into();
    let mut s = EditorState::new();
    s.chat.messages = vec![m];
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);

    // Source tab paints under owner A and stores a build with a hittable header.
    let owner_a = next_panel_owner();
    let panel_a = AIChatPlaceholder::from_editor(&s).owned_by(owner_a);
    let body = panel_a.body_rect(rect);
    let build = cached_canonical_transcript_owned(owner_a, &s.chat.messages, body, panel_a.locale);
    let header = build.items[0].thinking.as_ref().unwrap().header;
    let point = Point2D::new(
        header.origin.x + header.size.x / 2.0,
        header.origin.y + header.size.y / 2.0,
    );
    // Sanity: owner A's hint DOES hit its own painted header.
    assert_eq!(
        panel_a.hit_test_current_build(rect, point),
        Some(AIChatHit::ToggleThinking(0)),
        "source tab's own hint hits its painted header"
    );

    // Tab-switch click rotates the owner (host `force_rotate_chat_owner`). The
    // destination panel carries owner B and has NOT painted yet.
    let owner_b = next_panel_owner();
    let panel_b = AIChatPlaceholder::from_editor(&s).owned_by(owner_b);
    // The very next event-time cursor hint (before any B paint) must read None,
    // never the stale A-owned geometry.
    assert_eq!(
        panel_b.hit_test_current_build(rect, point),
        None,
        "post-switch hint reads None until the new tab's first paint"
    );
}
