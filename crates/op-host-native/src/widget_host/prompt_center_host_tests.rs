//! Native-host twins for Prompt Center overlay routing.

use super::{CursorHint, WidgetHostNative};
use op_editor_core::chat::ChatMessage;
use op_editor_core::component_browser_state::ComponentBrowserButton;
use op_editor_core::{NodeId, PromptCenterFocus, PropertyFocus, Tool};
use op_editor_ui::widgets::PromptCenterPanel;
use op_editor_ui::Point2D;

const VIEWPORT_W: f32 = 1_200.0;
const VIEWPORT_H: f32 = 800.0;

fn open_host() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.last_viewport_w = VIEWPORT_W;
    host.last_viewport_h = VIEWPORT_H;
    host.editor_state_mut().editor_ui.open_prompt_center(1);
    host
}

#[test]
fn prompt_center_padding_owns_cursor_and_clears_lower_hover() {
    let mut host = open_host();
    host.editor_state_mut().tool = Tool::Hand;
    let panel = host
        .prompt_center_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("prompt center rect");
    // Bottom-right padding: inside the panel, below the card grid, and —
    // unlike the panel's bottom-centre, which the composer card's east
    // resize gutter now owns at the canvas floor — over bare canvas when
    // the panel is closed, which is what makes the control below
    // meaningful.
    let point = Point2D::new(
        panel.origin.x + panel.size.x - 24.0,
        panel.origin.y + panel.size.y - 4.0,
    );

    assert!(host.editor_state_mut().editor_ui.close_prompt_center());
    assert_eq!(
        host.cursor_hint(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Grab,
        "fixture point must expose the canvas cursor without the overlay"
    );

    host.editor_state_mut().editor_ui.open_prompt_center(2);
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.canvas_hover_node = Some(NodeId::new("covered-node"));
        ui.property_action_hover = Some(0);
    }
    assert_eq!(
        host.cursor_hint(point.x, point.y, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Default,
        "blank panel padding must suppress the canvas cursor"
    );

    assert!(host.apply_cursor_move(point.x, point.y));
    let ui = &host.editor_state().editor_ui;
    assert_eq!(
        ui.prompt_center.hover, None,
        "padding is owned chrome, not an interactive panel control"
    );
    assert_eq!(
        ui.canvas_hover_node, None,
        "canvas hover must not survive beneath Prompt Center padding"
    );
    assert_eq!(
        ui.property_action_hover, None,
        "covered right-rail hover must be cleared by the overlay"
    );
}

#[test]
fn prompt_inputs_use_text_cursor_but_higher_floating_panels_win() {
    let mut host = open_host();
    let panel = host
        .prompt_center_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("prompt center rect");
    let search = PromptCenterPanel::search_rect(panel);
    let search_point = Point2D::new(
        search.origin.x + search.size.x / 2.0,
        search.origin.y + search.size.y / 2.0,
    );
    assert_eq!(
        host.cursor_hint(search_point.x, search_point.y, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Text
    );

    {
        let prompt = &mut host.editor_state_mut().editor_ui.prompt_center;
        prompt.save_open = true;
        prompt.focus = PromptCenterFocus::SaveTitle;
    }
    let save_title = PromptCenterPanel::save_title_rect(panel);
    let save_point = Point2D::new(
        save_title.origin.x + save_title.size.x / 2.0,
        save_title.origin.y + save_title.size.y / 2.0,
    );
    assert_eq!(
        host.cursor_hint(save_point.x, save_point.y, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Text
    );

    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.icon_picker.open = true;
        ui.icon_picker_panel_pos = Some((search_point.x - 20.0, search_point.y - 20.0));
    }
    assert!(host
        .icon_picker_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("icon picker rect")
        .contains(search_point));
    assert_eq!(
        host.cursor_hint(search_point.x, search_point.y, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Default,
        "Icon Picker paints above Prompt Center"
    );

    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.icon_picker.open = false;
        ui.design_md_panel.open = true;
        ui.design_md_panel.pos = Some((search_point.x - 20.0, search_point.y - 20.0));
    }
    assert!(host
        .design_md_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("design panel rect")
        .contains(search_point));
    assert_eq!(
        host.cursor_hint(search_point.x, search_point.y, VIEWPORT_W, VIEWPORT_H),
        CursorHint::Default,
        "Design-MD paints above Prompt Center"
    );
}

#[test]
fn prompt_hover_clears_covered_component_browser_hover() {
    let mut host = open_host();
    let prompt_rect = host
        .prompt_center_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("prompt center rect");
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.component_browser_open = true;
        ui.component_browser_pos = Some((prompt_rect.origin.x, prompt_rect.origin.y));
        ui.component_browser_hover = Some(ComponentBrowserButton::Card(0));
    }
    let component_rect = host
        .component_browser_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("component browser rect");
    let point = Point2D::new(
        component_rect.origin.x + component_rect.size.x / 2.0,
        component_rect.origin.y + component_rect.size.y / 2.0,
    );
    assert!(prompt_rect.contains(point));

    assert!(host.apply_cursor_move(point.x, point.y));
    assert_eq!(
        host.editor_state().editor_ui.component_browser_hover,
        None,
        "Prompt Center must clear the hover wash painted underneath it"
    );
}

#[test]
fn middle_press_inside_prompt_is_swallowed_without_starting_canvas_pan() {
    let mut host = open_host();
    let panel = host
        .prompt_center_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("prompt center rect");
    let point = Point2D::new(
        panel.origin.x + panel.size.x / 2.0,
        panel.origin.y + panel.size.y / 2.0,
    );

    assert!(host.apply_pan_press(point.x, point.y));
    assert!(
        host.drag.is_none(),
        "middle press over Prompt Center must not capture a canvas pan"
    );

    assert!(host.apply_pan_press(8.0, VIEWPORT_H - 8.0));
    assert!(
        host.drag.is_some(),
        "middle press outside the floating panel must still start a pan"
    );
}

#[test]
fn stale_property_focus_cannot_take_prompt_keyboard_edits() {
    let mut host = open_host();
    {
        let state = host.editor_state_mut();
        state.ui.property_focus = Some(PropertyFocus::SizeW);
        state.ui.property_input.set_text("42");
        state.editor_ui.prompt_center.search.set_text("ab");
        state.editor_ui.prompt_center.search.set_caret(2, 0);
    }

    assert!(host.apply_text('c'));
    assert_eq!(
        host.editor_state().editor_ui.prompt_center.search.text(),
        "abc"
    );
    assert!(host.apply_backspace());
    assert_eq!(
        host.editor_state().editor_ui.prompt_center.search.text(),
        "ab"
    );
    assert!(host.apply_select_all());
    assert!(host
        .editor_state()
        .editor_ui
        .prompt_center
        .search
        .is_select_all());
    assert!(host.apply_text('旅'));
    assert_eq!(
        host.editor_state().editor_ui.prompt_center.search.text(),
        "旅"
    );
    assert_eq!(host.editor_state().ui.property_input.text(), "42");
    assert_eq!(
        host.editor_state().ui.property_focus,
        Some(PropertyFocus::SizeW),
        "covered lower focus may remain stale but must never win routing"
    );
}

#[test]
fn escape_closes_only_prompt_center_before_chat_or_selection() {
    let mut host = open_host();
    let selected = NodeId::new("selection-under-prompt-center");
    host.editor_state_mut()
        .set_single_selection(selected.clone());
    host.editor_state_mut().chat.focused = true;
    host.editor_state_mut().ui.property_focus = Some(PropertyFocus::SizeW);

    assert!(host.apply_escape());

    let state = host.editor_state();
    assert!(!state.editor_ui.prompt_center.open);
    assert!(
        state.chat.focused,
        "the same Escape must not continue into chat blur"
    );
    assert_eq!(
        state.selection.anchor, selected,
        "the same Escape must not continue into selection clearing"
    );
    assert_eq!(
        state.ui.property_focus,
        Some(PropertyFocus::SizeW),
        "the same Escape must not continue into lower property focus"
    );
}

#[test]
fn prompt_center_wheel_and_trackpad_scroll_without_moving_viewport() {
    let mut host = open_host();
    let panel_rect = host
        .prompt_center_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("prompt center rect");
    let point = Point2D::new(
        panel_rect.origin.x + panel_rect.size.x / 2.0,
        panel_rect.origin.y + panel_rect.size.y / 2.0,
    );
    assert!(
        PromptCenterPanel::for_editor(host.editor_state())
            .expect("open prompt center")
            .max_scroll(panel_rect)
            > 0.0,
        "catalogue fixture must overflow the two-column grid"
    );
    let viewport = host.editor_state().viewport;

    assert!(host.apply_wheel(point.x, point.y, -120.0, VIEWPORT_W, VIEWPORT_H));
    let wheel_offset = host.editor_state().editor_ui.prompt_center.scroll.offset;
    assert!(wheel_offset > 0.0);
    assert_eq!(host.editor_state().viewport, viewport);

    assert!(host.apply_pan_gesture(point.x, point.y, 35.0, -80.0, VIEWPORT_W, VIEWPORT_H));
    assert!(
        host.editor_state().editor_ui.prompt_center.scroll.offset > wheel_offset,
        "trackpad vertical delta must continue scrolling the card grid"
    );
    assert_eq!(
        host.editor_state().viewport,
        viewport,
        "neither wheel zoom nor trackpad pan may leak to the canvas"
    );
}

#[test]
fn built_in_card_click_only_fills_chat_and_closes_center() {
    let mut host = open_host();
    host.set_now_ms(77);
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant("existing transcript"));
    host.editor_state_mut().chat.pending_send = Some("in-flight request".to_owned());

    let panel_rect = host
        .prompt_center_panel_rect(VIEWPORT_W, VIEWPORT_H)
        .expect("prompt center rect");
    let (card_rect, expected_body) = {
        let panel = PromptCenterPanel::for_editor(host.editor_state()).expect("open prompt center");
        let cards = panel.filtered();
        assert!(!cards.is_empty());
        assert!(!cards[0].custom, "fixture must click a built-in card");
        (panel.card_rects(panel_rect)[0].1, cards[0].body.to_owned())
    };
    let messages = host.editor_state().chat.messages.clone();
    let pending_send = host.editor_state().chat.pending_send.clone();

    assert!(host.apply_press(
        card_rect.origin.x + 12.0,
        card_rect.origin.y + 12.0,
        VIEWPORT_W,
        VIEWPORT_H
    ));

    let state = host.editor_state();
    assert!(!state.editor_ui.prompt_center.open);
    assert_eq!(state.chat.input.text(), expected_body);
    assert!(state.chat.focused);
    assert_eq!(state.chat.input_caret(), expected_body.len());
    assert_eq!(
        state.chat.pending_send, pending_send,
        "selecting a prompt must not queue another send"
    );
    assert_eq!(
        state.chat.messages, messages,
        "selecting a prompt must not append transcript messages"
    );
}
