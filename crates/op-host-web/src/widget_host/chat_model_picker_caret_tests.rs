use super::WidgetHost;
use op_editor_ui::widgets::AIChatPlaceholder;

#[test]
fn chat_model_picker_arrows_move_caret_for_insert_and_backspace() {
    let mut host = WidgetHost::new();
    {
        let ui = &mut host.editor_state.editor_ui;
        ui.chat_model_picker.open = true;
        ui.chat_model_picker_input.set_text("abcd");
    }

    assert!(host.apply_chat_model_picker_caret(false));
    assert!(host.apply_chat_model_picker_caret(false));
    assert_eq!(
        host.editor_state.editor_ui.chat_model_picker_input.caret(),
        2
    );

    assert!(host.apply_text('X'));
    assert_eq!(
        host.editor_state.editor_ui.chat_model_picker_input.text(),
        "abXcd"
    );
    assert_eq!(
        host.editor_state.editor_ui.chat_model_picker_input.caret(),
        3
    );

    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state.editor_ui.chat_model_picker_input.text(),
        "abcd"
    );
    assert_eq!(
        host.editor_state.editor_ui.chat_model_picker_input.caret(),
        2
    );
}

#[test]
fn chat_model_picker_clear_button_empties_search() {
    let mut host = WidgetHost::new();
    host.set_now_ms(456);
    {
        let ui = &mut host.editor_state.editor_ui;
        ui.chat_model_picker.open = true;
        ui.chat_model_picker_input.set_text("231");
        ui.chat_model_picker.scroll.offset = 10.0;
        ui.chat_model_picker.hover = Some(0);
    }
    let chat_rect = host.ai_chat_rect(1200.0, 800.0).unwrap();
    let panel = AIChatPlaceholder::from_editor_at(&host.editor_state, 456);
    let picker = panel.model_picker_bounds(chat_rect).unwrap();
    let x = picker.origin.x + picker.size.x - 24.0;
    let y = picker.origin.y + 19.0;

    assert!(host.apply_click(x, y, 1200.0, 800.0));

    let ui = &host.editor_state.editor_ui;
    assert!(ui.chat_model_picker_input.text().is_empty());
    assert_eq!(ui.chat_model_picker_input.caret(), 0);
    assert_eq!(ui.chat_model_picker.scroll.offset, 0.0);
    assert_eq!(ui.chat_model_picker.hover, None);
    assert!(ui.chat_model_picker.open);
    assert_eq!(ui.chat_model_picker_input.next_blink_flip_ms(456), 956);
}
