//! Layout + hit-test unit tests for [`super::AIChatPlaceholder`].
//! Split into a sibling file to keep `ai_chat_panel.rs` under the
//! 800-line cap.

use super::*;

#[test]
fn layout_reports_fixed_size() {
    let s = EditorState::new();
    let p = AIChatPlaceholder::from_editor(&s);
    let cx = LayoutCx {
        available_width: 9999.0,
        dpi: 1.0,
    };
    let lb = p.layout(&cx);
    assert_eq!(lb.rect.size.x, AI_CHAT_WIDTH);
    assert_eq!(lb.rect.size.y, AI_CHAT_HEIGHT);
}

#[test]
fn examples_grid_has_four_cards() {
    assert_eq!(EXAMPLES.len(), 4);
}

/// Y-coordinate of the textarea's vertical center.
fn textarea_center_y() -> f32 {
    AI_CHAT_HEIGHT - INPUT_BASE_HEIGHT + 1.0 + INPUT_AREA_HEIGHT / 2.0
}

/// Y-coordinate of the per-turn controls strip's vertical center.
fn controls_center_y() -> f32 {
    AI_CHAT_HEIGHT - INPUT_BASE_HEIGHT + 1.0 + INPUT_AREA_HEIGHT + CONTROLS_ROW_HEIGHT / 2.0
}

/// Y-coordinate of the bottom toolbar's vertical center.
fn toolbar_center_y() -> f32 {
    AI_CHAT_HEIGHT - INPUT_BASE_HEIGHT
        + 1.0
        + INPUT_AREA_HEIGHT
        + CONTROLS_ROW_HEIGHT
        + INPUT_TOOLBAR_HEIGHT / 2.0
}

#[test]
fn hit_test_resolves_input_focus() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Click near the textarea center → FocusInput.
    let p = Point2D::new(120.0, textarea_center_y());
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::FocusInput));
}

#[test]
fn hit_test_resolves_send_at_right() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let send_x = AI_CHAT_WIDTH - PAD - 20.0;
    let p = Point2D::new(send_x, toolbar_center_y());
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::Send));
}

#[test]
fn hit_test_resolves_controls_strip() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    let y = controls_center_y();
    // Thinking chip sits at the left edge of the input box (PAD).
    assert_eq!(
        panel.hit_test(rect, Point2D::new(PAD + 8.0, y)),
        Some(AIChatHit::CycleThinking)
    );
    // Model chip still resolves in the toolbar below.
    assert_eq!(
        panel.hit_test(rect, Point2D::new(PAD + 8.0, toolbar_center_y())),
        Some(AIChatHit::ToggleModelPicker)
    );
}

#[test]
fn hit_test_resolves_attachment_chip_at_painted_position() {
    // With an attachment staged, the input block grows by the
    // attachment row. The click must land where `paint` draws the
    // chip — a regression guard for hit-test / paint y-alignment.
    let mut s = EditorState::new();
    s.chat.add_attachment(op_editor_core::chat::ChatAttachment {
        name: "ref.png".into(),
        media_type: "image/png".into(),
        data: vec![1],
    });
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // paint: input block top = bottom - input_h + 1; the
    // attachment row sits right below the textarea.
    let input_h = INPUT_BASE_HEIGHT + ATTACHMENT_ROW_HEIGHT;
    let input_top = AI_CHAT_HEIGHT - input_h + 1.0;
    let attach_row_center = input_top + INPUT_AREA_HEIGHT + ATTACHMENT_ROW_HEIGHT / 2.0;
    let p = Point2D::new(PAD + 30.0, attach_row_center);
    assert_eq!(
        panel.hit_test(rect, p),
        Some(AIChatHit::RemoveAttachment(0))
    );
}

#[test]
fn hit_test_resolves_first_example_when_empty() {
    let s = EditorState::new(); // chat empty by default
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // First example card: top-left of grid.
    let card_w = (AI_CHAT_WIDTH - PAD * 2.0 - 8.0) / 2.0;
    let p = Point2D::new(PAD + card_w / 2.0, HEADER_HEIGHT + 32.0 + 35.0);
    match panel.hit_test(rect, p) {
        Some(AIChatHit::Example(s)) => {
            assert_eq!(s, EXAMPLES[0].title);
        }
        other => panic!("expected first example hit, got {:?}", other),
    }
}

#[test]
fn hit_test_header_returns_drag_handle() {
    let s = EditorState::new();
    let panel = AIChatPlaceholder::from_editor(&s);
    let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
    // Click in the empty header band (between title and icons).
    let p = Point2D::new(AI_CHAT_WIDTH / 2.0, 16.0);
    assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::DragHandle));
}
