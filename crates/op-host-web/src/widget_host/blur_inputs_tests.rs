//! Blank-press blur coverage — `#[cfg(test)]` companion to the web
//! host's `blur_inputs.rs`. Mirrors the native suite for the inputs
//! the web shell wires today (chat, model picker, settings modal).

use super::WidgetHost;
use op_editor_core::agent_settings::SettingsFocus;
use op_editor_core::{CloneField, CloneFormState};

const VW: f32 = 1200.0;
const VH: f32 = 800.0;

#[test]
fn top_bar_gap_press_blurs_chat_and_model_picker() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    host.editor_state.editor_ui.chat_model_picker.open = true;
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
    assert!(!host.editor_state.editor_ui.chat_model_picker.open);
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
    // The default chat panel is anchored bottom-left and is painted ON TOP of
    // the toolbar (which shares the same left edge). Since the toolbar grew
    // (Undo/Redo + Variables/Design actions), its bottom inset now falls under
    // that panel, so the press would be claimed by the chat instead of the
    // toolbar gap. Anchor the chat to the right so the toolbar's own blank-press
    // handling actually receives the press.
    host.editor_state.chat.anchor = op_editor_core::ChatAnchor::BottomRight;

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
fn blank_right_press_blurs_chat_like_native() {
    let mut host = WidgetHost::new();
    host.editor_state.chat.focused = true;
    host.editor_state.editor_ui.chat_model_picker.open = true;
    host.editor_state
        .editor_ui
        .chat_model_picker_input
        .set_text("gpt");

    assert!(host.apply_right_press(500.0, 120.0, VW, VH));

    assert!(
        !host.editor_state.chat.focused,
        "blank right press must blur the chat input"
    );
    assert!(!host.editor_state.editor_ui.chat_model_picker.open);
    assert!(host
        .editor_state
        .editor_ui
        .chat_model_picker_input
        .text()
        .is_empty());
}

#[test]
fn blank_right_press_with_sidebar_closed_blurs_chat_like_native() {
    let mut host = WidgetHost::new();
    host.editor_state.editor_ui.sidebar_open = false;
    host.editor_state.chat.focused = true;

    assert!(host.apply_right_press(500.0, 120.0, VW, VH));

    assert!(
        !host.editor_state.chat.focused,
        "sidebar-closed blank right press must still blur the chat input"
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

#[test]
fn blank_press_blur_reports_and_defocuses_git_inputs_like_native() {
    let mut host = WidgetHost::new();
    {
        let git = &mut host.editor_state_mut().editor_ui.git_panel;
        git.open = true;
        git.author_prompt = true;
        git.commit_focused = true;
        git.remote_focused = true;
        git.https_focused = true;
        git.branch_create_focused = true;
        git.author_name_focused = true;
        git.author_email_focused = true;
        git.clone_form = Some(CloneFormState {
            focus: Some(CloneField::Url),
            ..Default::default()
        });
    }

    assert!(
        host.blur_text_inputs_on_blank_press(),
        "git inputs must count as focused chrome for blank-press blur"
    );

    let git = &host.editor_state().editor_ui.git_panel;
    assert!(!git.commit_focused);
    assert!(!git.remote_focused);
    assert!(!git.https_focused);
    assert!(!git.branch_create_focused);
    assert!(!git.author_name_focused);
    assert!(!git.author_email_focused);
    assert_eq!(git.clone_form.as_ref().and_then(|form| form.focus), None);
}

#[test]
fn blank_press_blur_reports_and_defocuses_preset_name_like_native() {
    let mut host = WidgetHost::new();
    {
        let eui = &mut host.editor_state_mut().editor_ui;
        eui.variables_preset_menu_open = true;
        eui.variables_preset_name_focus = true;
    }

    assert!(
        host.blur_text_inputs_on_blank_press(),
        "preset name input must count as focused chrome for blank-press blur"
    );
    assert!(!host.editor_state().editor_ui.variables_preset_name_focus);
}
