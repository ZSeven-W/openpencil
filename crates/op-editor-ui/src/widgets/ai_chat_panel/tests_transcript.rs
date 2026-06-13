//! Transcript-area tests for [`super::AIChatPlaceholder`] — fixed
//! checklist geometry, tool-card / design-block toggles, and paint
//! assertions. Split out of `tests.rs` at the 800-line cap.

use super::tests::{has_fill_rect, PanelPaintBackend};
use super::*;
use crate::widgets::ai_chat_hit::AIChatHit;

#[test]
fn body_rect_reserves_space_for_fixed_step_checklist() {
    let mut s = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    message.content =
        r#"<step title="Checking guidelines" status="streaming">Analyzing request...</step>"#
            .into();
    s.chat.messages.push(message);

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let body = panel.body_rect(rect);
    let legacy_bottom = rect.origin.y + rect.size.y - INPUT_BASE_HEIGHT - PAD - 8.0;

    assert!(
        body.origin.y + body.size.y < legacy_bottom - 1.0,
        "fixed step checklist should reserve bottom space outside transcript"
    );
}

#[test]
fn default_height_preserves_transcript_space_above_full_checklist() {
    let mut s = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    message.content = (0..17)
        .map(|idx| {
            format!(r#"<step title="Subtask {idx}" status="done">Generated section {idx}</step>"#)
        })
        .collect::<Vec<_>>()
        .join("\n");
    s.chat.messages.push(message);

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let body = panel.body_rect(rect);

    assert!(
        body.size.y >= 150.0,
        "default chat panel height should leave room for prior chat above the pinned checklist, got {}",
        body.size.y
    );
}

#[test]
fn body_rect_reserves_less_space_when_fixed_step_checklist_collapsed() {
    let mut expanded_state = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    message.content = r#"<step title="Plan" status="done"></step>
<step title="Draw" status="streaming"></step>"#
        .into();
    expanded_state.chat.messages.push(message.clone());

    let mut collapsed_state = EditorState::new();
    collapsed_state.chat.messages.push(message);
    collapsed_state.chat.checklist_collapsed = true;

    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let expanded = AIChatPlaceholder::from_editor(&expanded_state).body_rect(rect);
    let collapsed = AIChatPlaceholder::from_editor(&collapsed_state).body_rect(rect);

    assert!(collapsed.size.y > expanded.size.y);
}

#[test]
fn hit_test_resolves_fixed_checklist_header_toggle() {
    let mut s = EditorState::new();
    let mut message = op_editor_core::ChatMessage::assistant_streaming();
    message.content = r#"<step title="Plan" status="done"></step>
<step title="Draw" status="streaming"></step>"#
        .into();
    s.chat.messages.push(message);

    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let checklist_h = fixed_checklist_height(&s.chat.messages, s.chat.checklist_collapsed);
    let checklist = fixed_checklist_rect(rect, INPUT_BASE_HEIGHT, checklist_h);
    let p = Point2D::new(
        checklist.origin.x + checklist.size.x / 2.0,
        checklist.origin.y + 2.0 + 32.0 / 2.0,
    );

    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::ToggleChecklist));
}

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

    let key_paths = crate::widgets::icons::Icon::Key.paths();
    assert!(
        key_paths
            .iter()
            .all(|kp| backend.svg_paths.iter().any(|p| p == kp)),
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
