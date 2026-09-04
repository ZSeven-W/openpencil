//! The pre-flight MCP notice: it shows exactly when the selected agent cannot
//! start a canvas turn, it costs no layout when it is silent, and it is the
//! button that opens the toggle it is complaining about.

use crate::widgets::ai_chat_hit::AIChatHit;
use crate::widgets::ai_chat_mcp_notice::MCP_NOTICE_ROW_HEIGHT;
use crate::widgets::AIChatPlaceholder;
use crate::{Point2D, Rect};
use op_editor_core::agent_settings::McpCli;
use op_editor_core::chat::models::AgentProvider;
use op_editor_core::EditorState;

const PANEL: Rect = Rect {
    origin: Point2D { x: 40.0, y: 60.0 },
    size: Point2D { x: 360.0, y: 420.0 },
};

fn state_with_antigravity(mcp_on: bool) -> EditorState {
    let mut state = EditorState::default();
    state.editor_ui.chat_selected_agent = AgentProvider::ALL
        .iter()
        .position(|candidate| *candidate == AgentProvider::Antigravity)
        .expect("Antigravity is registered");
    state.editor_ui.agent_settings.mcp_cli_enabled[McpCli::Antigravity.index()] = mcp_on;
    state
}

#[test]
fn a_gated_agent_gets_a_notice_row_naming_it() {
    let state = state_with_antigravity(false);
    let panel = AIChatPlaceholder::from_editor(&state);
    let label = panel.mcp_notice.as_deref().expect("the notice shows");
    assert!(
        label.contains("Antigravity"),
        "the notice must name the agent it is about: {label}"
    );
    assert!(
        !label.contains("{cli}"),
        "the placeholder must be substituted, not painted: {label}"
    );
    assert_eq!(panel.mcp_notice_row_h(), MCP_NOTICE_ROW_HEIGHT);
    assert!(panel.mcp_notice_row(PANEL).is_some());
}

#[test]
fn a_ready_agent_pays_no_row_at_all() {
    let state = state_with_antigravity(true);
    let panel = AIChatPlaceholder::from_editor(&state);
    assert!(panel.mcp_notice.is_none());
    assert_eq!(panel.mcp_notice_row_h(), 0.0);
    assert!(panel.mcp_notice_row(PANEL).is_none());
}

#[test]
fn the_notice_pushes_the_input_rows_down_by_exactly_its_height() {
    // The rows below must not shift by anything other than the notice — a
    // drift here shows up as a caret that no longer lands where it is drawn.
    let ready = state_with_antigravity(true);
    let gated = state_with_antigravity(false);
    let ready_panel = AIChatPlaceholder::from_editor(&ready);
    let gated_panel = AIChatPlaceholder::from_editor(&gated);

    let ready_text = ready_panel.input_text_rect(PANEL);
    let gated_text = gated_panel.input_text_rect(PANEL);
    assert_eq!(
        gated_text.size, ready_text.size,
        "the textarea keeps its size; only the block above it grows"
    );
    // The input block grows upward by the notice height, so the text row
    // itself stays anchored to the same distance from the panel bottom.
    let ready_bottom = ready_text.origin.y + ready_text.size.y;
    let gated_bottom = gated_text.origin.y + gated_text.size.y;
    assert_eq!(gated_bottom, ready_bottom);
}

#[test]
fn clicking_the_notice_opens_the_settings_tab_that_carries_the_toggle() {
    let state = state_with_antigravity(false);
    let panel = AIChatPlaceholder::from_editor(&state);
    let row = panel.mcp_notice_row(PANEL).expect("the notice shows");
    let point = Point2D::new(
        row.origin.x + row.size.x / 2.0,
        row.origin.y + row.size.y / 2.0,
    );
    assert_eq!(
        panel.hit_test(PANEL, point),
        Some(AIChatHit::OpenMcpSettings),
        "the whole row is the button"
    );
}

#[test]
fn the_row_below_the_notice_still_focuses_the_input() {
    // Guards the rebased hit-test: everything under the notice must keep
    // resolving as it did before, not shift into the wrong band.
    let state = state_with_antigravity(false);
    let panel = AIChatPlaceholder::from_editor(&state);
    let row = panel.mcp_notice_row(PANEL).expect("the notice shows");
    let text = panel.input_text_rect(PANEL);
    let point = Point2D::new(
        text.origin.x + text.size.x / 2.0,
        text.origin.y + text.size.y / 2.0,
    );
    assert!(point.y > row.origin.y + row.size.y);
    assert!(
        matches!(
            panel.hit_test(PANEL, point),
            Some(AIChatHit::FocusInput) | Some(AIChatHit::SelectInputText(_))
        ),
        "a click in the textarea must still reach the textarea"
    );
}
