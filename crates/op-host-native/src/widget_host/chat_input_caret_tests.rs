//! Arrow-key caret motion and wheel scrolling inside the AI chat draft
//! input.
//!
//! The reported bug had two halves: arrows did nothing visible in a long
//! prompt, and a prompt taller than the box could not be scrolled at all.
//! The tests below pin both, plus the thing that must NOT happen — an
//! arrow reaching `apply_nudge` and dragging the selected canvas node.

use super::WidgetHostNative;
use op_editor_core::{EditorState, NodeId, PenNodeExt};
use op_editor_ui::widgets::AIChatPlaceholder;

const VIEWPORT: (f32, f32) = (1400.0, 900.0);
/// Seven authored rows — more than the input's row cap, so it scrolls.
const LONG_PROMPT: &str = "line one of the prompt\nline two of the prompt\nline three of the prompt\nline four of the prompt\nline five of the prompt\nline six of the prompt\nline seven of the prompt";

const TWO_RECTS: &str = r#"{"version":"1.0.0","children":[
    {"type":"rectangle","id":"n1","name":"One","x":40,"y":40,"width":80,"height":40},
    {"type":"rectangle","id":"n2","name":"Two","x":200,"y":40,"width":80,"height":40}
]}"#;

fn host_with_focused_chat(text: &str) -> WidgetHostNative {
    let document = jian_ops_schema::load_str(TWO_RECTS)
        .expect("fixture JSON parses")
        .value;
    let mut host = WidgetHostNative::new();
    // The expanded chat panel lives in the rail's Agent tab; anywhere
    // else the chat is composer-only, so put the rail on its home.
    host.editor_state_mut().editor_ui.enter_chat_tab();
    *host.editor_state_mut() = EditorState::from_document(document);
    host.last_viewport_w = VIEWPORT.0;
    host.last_viewport_h = VIEWPORT.1;
    let state = host.editor_state_mut();
    state.set_single_selection(NodeId::new("n1"));
    state.chat.focused = true;
    state.chat.set_input_text(text);
    host
}

fn selected_node_x(host: &WidgetHostNative) -> f64 {
    host.editor_state()
        .selected_node()
        .expect("selected node")
        .base()
        .x
        .expect("fixture authors an explicit x")
}

/// Screen point inside the chat panel's draft input.
fn input_point(host: &WidgetHostNative) -> op_editor_ui::Point2D {
    let chat_rect = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("chat panel placed");
    let input = AIChatPlaceholder::from_editor_at(host.editor_state(), host.now_ms)
        .input_text_rect(chat_rect);
    op_editor_ui::Point2D::new(
        input.origin.x + input.size.x / 2.0,
        input.origin.y + input.size.y / 2.0,
    )
}

/// ① The chat input owns Left/Right, and the selected node stays put.
#[test]
fn horizontal_arrows_move_the_chat_caret_and_never_nudge_the_selection() {
    let mut host = host_with_focused_chat("abcd");
    let node_x = selected_node_x(&host);

    assert!(host.apply_chat_input_caret(false, false));
    assert!(host.apply_chat_input_caret(false, false));
    assert_eq!(host.editor_state().chat.input.caret(), 2);

    assert!(host.apply_chat_input_caret(true, false));
    assert_eq!(host.editor_state().chat.input.caret(), 3);

    // Second line of defence: even if a host ladder ever calls it anyway,
    // nudge must decline while the chat owns the keyboard.
    assert!(
        !host.apply_nudge(1.0, 0.0),
        "a focused chat input must make nudge decline the key"
    );
    assert_eq!(selected_node_x(&host), node_x, "the node must not move");
}

/// ② Up / Down walk VISUAL rows, and hold a sane column.
#[test]
fn vertical_arrows_walk_visual_rows_of_the_chat_input() {
    let mut host = host_with_focused_chat("abcdefgh\nijklmnop\nqrstuvwx");
    let text = host.editor_state().chat.input.text().to_owned();
    let middle_row = text.find("ijkl").expect("fixture");
    let last_row = text.find("qrst").expect("fixture");
    host.editor_state_mut()
        .chat
        .input
        .set_caret(middle_row + 3, 0);

    assert!(host.apply_chat_input_vertical_caret(false, false));
    assert_eq!(host.editor_state().chat.input.caret(), 3, "row 0, column 3");

    assert!(host.apply_chat_input_vertical_caret(true, false));
    assert_eq!(host.editor_state().chat.input.caret(), middle_row + 3);

    assert!(host.apply_chat_input_vertical_caret(true, false));
    assert_eq!(host.editor_state().chat.input.caret(), last_row + 3);
}

/// ② (edges) Up on the first row goes home, Down on the last goes to the
/// end — and both still consume the key.
#[test]
fn vertical_arrows_collapse_at_the_first_and_last_row() {
    let mut host = host_with_focused_chat("abcdefgh\nijklmnop");
    let len = host.editor_state().chat.input.text().len();
    let node_x = selected_node_x(&host);

    host.editor_state_mut().chat.input.set_caret(3, 0);
    assert!(host.apply_chat_input_vertical_caret(false, false));
    assert_eq!(host.editor_state().chat.input.caret(), 0);

    host.editor_state_mut().chat.input.set_caret(len - 3, 0);
    assert!(host.apply_chat_input_vertical_caret(true, false));
    assert_eq!(host.editor_state().chat.input.caret(), len);

    assert_eq!(selected_node_x(&host), node_x);
}

/// Shift+arrow grows the selection instead of collapsing it.
#[test]
fn shift_arrows_extend_the_chat_input_selection() {
    let mut host = host_with_focused_chat("abcdefgh\nijklmnop");
    host.editor_state_mut().chat.input.set_caret(4, 0);

    assert!(host.apply_chat_input_caret(false, true));
    assert_eq!(host.editor_state().chat.selected_input_text(), Some("d"));

    assert!(host.apply_chat_input_vertical_caret(true, true));
    let selected = host
        .editor_state()
        .chat
        .selected_input_text()
        .expect("vertical shift-arrow keeps a selection");
    // Anchor stayed at the Shift+Left origin (byte 4) while the focus
    // stepped a visual row down to the same column.
    assert_eq!(selected, "efgh\nijk");
}

/// ⑤ A live IME preedit owns the arrows: the key is swallowed, the caret
/// does not move, and the composition survives.
#[test]
fn arrows_leave_the_caret_alone_while_composing() {
    let mut host = host_with_focused_chat("abcd");
    {
        let input = &mut host.editor_state_mut().chat.input;
        input.set_caret(2, 0);
        input.set_composition("ni", 2, 0);
    }
    let before = host.editor_state().chat.input.caret();

    // Two presses in the SAME direction: a left+right pair would net out to
    // the starting offset and hide an unguarded move.
    assert!(host.apply_chat_input_caret(false, false), "key is consumed");
    assert!(host.apply_chat_input_caret(false, false));
    assert!(host.apply_chat_input_vertical_caret(true, false));
    assert!(host.apply_chat_input_vertical_caret(true, false));

    assert_eq!(host.editor_state().chat.input.caret(), before);
    assert!(
        host.editor_state().chat.input.composition().is_some(),
        "the preedit must survive an arrow"
    );
}

/// ④ A wheel over the input scrolls the input, not the canvas.
#[test]
fn wheel_over_the_chat_input_scrolls_it_and_leaves_the_canvas_alone() {
    let mut host = host_with_focused_chat(LONG_PROMPT);
    let point = input_point(&host);
    let viewport_before = host.editor_state().viewport;
    // The caret sits at the end, so the box starts scrolled to the bottom.
    let chat_rect = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("chat panel placed");
    let (start, max) = AIChatPlaceholder::from_editor_at(host.editor_state(), host.now_ms)
        .input_scroll_state(chat_rect);
    assert!(max > 0.0, "fixture must overflow the input box");
    assert!((start - max).abs() < 0.01, "starts pinned to the caret row");

    assert!(
        host.apply_wheel(point.x, point.y, 60.0, VIEWPORT.0, VIEWPORT.1),
        "the input must swallow the wheel"
    );

    let (after, _) = AIChatPlaceholder::from_editor_at(host.editor_state(), host.now_ms)
        .input_scroll_state(chat_rect);
    assert!(
        after < start,
        "the input should have scrolled up ({after} vs {start})"
    );
    assert_eq!(
        host.editor_state().viewport,
        viewport_before,
        "the canvas viewport must not zoom or pan"
    );
}

/// ③ Moving the caret out of the scrolled-away region pulls the view back
/// to it, discarding the stale wheel offset.
#[test]
fn moving_the_caret_scrolls_the_input_back_to_the_caret_row() {
    let mut host = host_with_focused_chat(LONG_PROMPT);
    let point = input_point(&host);
    let chat_rect = host
        .ai_chat_rect(VIEWPORT.0, VIEWPORT.1)
        .expect("chat panel placed");

    // Wheel all the way to the top while the caret is still at the end.
    for _ in 0..20 {
        host.apply_wheel(point.x, point.y, 60.0, VIEWPORT.0, VIEWPORT.1);
    }
    let (scrolled, max) = AIChatPlaceholder::from_editor_at(host.editor_state(), host.now_ms)
        .input_scroll_state(chat_rect);
    assert_eq!(scrolled, 0.0);

    // One arrow press and the caret's row is on screen again.
    assert!(host.apply_chat_input_caret(false, false));
    let (after, _) = AIChatPlaceholder::from_editor_at(host.editor_state(), host.now_ms)
        .input_scroll_state(chat_rect);
    assert!(
        (after - max).abs() < 0.01,
        "the caret row must be revealed ({after} vs {max})"
    );
}

/// A wheel over the input is swallowed even when nothing overflows, so it
/// can never zoom the canvas showing through the panel.
#[test]
fn wheel_over_a_short_chat_input_still_never_zooms_the_canvas() {
    let mut host = host_with_focused_chat("short");
    let point = input_point(&host);
    let viewport_before = host.editor_state().viewport;

    assert!(host.apply_wheel(point.x, point.y, 60.0, VIEWPORT.0, VIEWPORT.1));
    assert_eq!(host.editor_state().viewport, viewport_before);
}
