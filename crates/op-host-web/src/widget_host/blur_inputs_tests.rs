//! Blank-press blur coverage — `#[cfg(test)]` companion to the web
//! host's `blur_inputs.rs`. Mirrors the native suite for the inputs
//! the web shell wires today (chat, model picker, settings modal).

use super::WidgetHost;
use op_editor_core::agent_settings::SettingsFocus;

const VW: f32 = 1200.0;
const VH: f32 = 800.0;

#[test]
fn top_bar_gap_press_blurs_chat_and_model_picker() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    host.editor_state.editor_ui.chat_model_picker_open = true;
    host.editor_state
        .editor_ui
        .chat_model_picker_input
        .set_text("gpt");

    // Dead centre of the top bar — between the left file controls and
    // the right chrome chips.
    assert!(host.apply_press(VW / 2.0, 20.0, VW, VH));

    assert!(
        !host.editor_state.chat.focused,
        "top-bar gap press must blur the chat input"
    );
    assert!(!host.editor_state.editor_ui.chat_model_picker_open);
    assert!(host
        .editor_state
        .editor_ui
        .chat_model_picker_input
        .text()
        .is_empty());
}

#[test]
fn toolbar_gap_press_blurs_chat() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;

    // Bottom inset of the toolbar's bounding rect — consumed by the
    // toolbar block but hits no button.
    let rect = host.toolbar_rect(VW);
    let x = rect.origin.x + rect.size.x / 2.0;
    let y = rect.origin.y + rect.size.y - 2.0;
    assert!(host.apply_press(x, y, VW, VH));

    assert!(
        !host.editor_state.chat.focused,
        "toolbar gap press must blur the chat input"
    );
}

#[test]
fn settings_modal_blank_press_commits_mcp_port_draft() {
    let mut host = WidgetHost::new();
    {
        let eui = &mut host.editor_state.editor_ui;
        eui.agent_settings_open = true;
        eui.agent_settings.focus = Some(SettingsFocus::McpPort);
        eui.settings_input.set_text("4321");
    }

    // Bottom of the modal's sidebar column — below the nav tabs, no
    // control under it. The 720×720 modal spans x ∈ [240, 960] at
    // this viewport, so x=260 sits just inside its left edge.
    assert!(host.apply_press(260.0, 700.0, VW, VH));

    let eui = &host.editor_state.editor_ui;
    assert!(
        eui.agent_settings.focus.is_none(),
        "blank press inside the modal must commit + blur the port input"
    );
    assert_eq!(eui.agent_settings.mcp_server.port, 4321);
    assert!(
        eui.agent_settings_open,
        "blank press inside the modal must not close it"
    );
}
