//! Tests for the pinned in-place refine route (Home's template draft).

use super::super::launch_if_pending;
use op_editor_core::{
    AgentProvider, BuiltinAgentConfig, BuiltinAgentKind, BuiltinAgentPresetKey, LaunchRoute,
    ModelEntry, NodeId, SelectionState,
};
use op_host_native::WidgetHostNative;

fn host_with_builtin_model() -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .builtin_agents
        .push(BuiltinAgentConfig {
            id: "builtin-1".into(),
            preset: BuiltinAgentPresetKey::Custom,
            display_name: "MiniMax".into(),
            kind: BuiltinAgentKind::OpenAiCompat,
            api_key: "sk-test".into(),
            models: vec!["MiniMax-M3".into()],
            base_url: "http://localhost:9".into(),
            enabled: true,
        });
    host.editor_state_mut().chat.available_models = vec![ModelEntry::builtin(
        AgentProvider::ClaudeCode,
        "builtin-1",
        "builtin:builtin-1:MiniMax-M3",
        "MiniMax M3",
    )];
    host.editor_state_mut().chat.selected_model = 0;
    host
}

/// Two slide boards standing in for a loaded template draft.
fn with_draft_boards(host: &mut WidgetHostNative) -> Vec<String> {
    let state = host.editor_state_mut();
    state.active_children_mut().clear();
    for id in ["slide-1", "slide-2"] {
        let board: jian_ops_schema::node::PenNode = serde_json::from_value(serde_json::json!({
            "type": "frame", "id": id, "name": id,
            "width": 1920, "height": 1080,
            "children": [{ "type": "text", "id": format!("{id}-title"), "content": "Title" }]
        }))
        .expect("board fixture");
        state.active_children_mut().push(board);
    }
    vec!["slide-1".into(), "slide-2".into()]
}

fn queue_refine(host: &mut WidgetHostNative) {
    // The refine brief quotes the example, which reads as a NEW design
    // to every keyword classifier ("做一份 5 页 … PPT").
    let prompt = op_editor_core::refine_prompt(
        op_editor_core::HomeFamily::Presentation,
        "为 OpenPencil 做一份 5 页产品介绍 PPT，包含封面和结束页。",
    );
    let chat = &mut host.editor_state_mut().chat;
    chat.set_input_text(prompt);
    chat.launch_route = LaunchRoute::Refine;
    assert!(chat.begin_send());
}

#[test]
fn a_pinned_refine_edits_the_selected_boards_instead_of_designing_anew() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();

    let mut host = host_with_builtin_model();
    let boards = with_draft_boards(&mut host);
    host.editor_state_mut().selection = SelectionState {
        anchor: NodeId::new("slide-2"),
        set: boards.iter().map(NodeId::new).collect(),
    };
    // The loop gate would say yes too — the pinned route must still win.
    host.editor_state_mut()
        .editor_ui
        .agent_settings
        .experimental_features_enabled = true;
    queue_refine(&mut host);

    let mut current_chat = None;
    let mut current_design = None;
    assert!(launch_if_pending(
        &mut host,
        &mut current_chat,
        &mut current_design
    ));
    assert!(
        current_chat.is_some(),
        "the modify route runs as a chat-channel session"
    );
    assert!(
        current_design.is_none(),
        "no orchestrator / design-loop run may draw a second design"
    );
    assert_eq!(host.editor_state().chat.launch_route, LaunchRoute::Auto);
    assert!(
        host.editor_state().selection.set.is_empty(),
        "the selection was only the hand-off; the plan captured the scope"
    );
    let ids: Vec<String> =
        op_editor_core::preview_slideshow::active_page_boards(host.editor_state());
    assert_eq!(ids, boards, "the draft is untouched until the edit applies");

    op_editor_core::agent_indicators::clear();
}

#[test]
fn a_refine_with_no_board_scope_ends_honestly_and_rearms_the_draft_banner() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();

    let mut host = host_with_builtin_model();
    with_draft_boards(&mut host);
    host.editor_state_mut().editor_ui.workspace.active = true;
    host.editor_state_mut().editor_ui.workspace.draft_template = Some("slide-deck");
    // Nothing selected: the modify route has no frames to edit.
    queue_refine(&mut host);

    let mut current_chat = None;
    let mut current_design = None;
    assert!(launch_if_pending(
        &mut host,
        &mut current_chat,
        &mut current_design
    ));
    assert!(current_chat.is_none() && current_design.is_none());
    let state = host.editor_state();
    let last = state.chat.messages.last().expect("the turn's bubble");
    assert!(!last.streaming);
    assert_eq!(
        last.content,
        op_i18n::translate(state.editor_ui.locale, "workspace.draft.refineUnavailable")
    );
    assert!(state.editor_ui.workspace.draft_awaiting_refine);
    assert_eq!(state.active_children().len(), 2, "no design was drawn");

    op_editor_core::agent_indicators::clear();
}
