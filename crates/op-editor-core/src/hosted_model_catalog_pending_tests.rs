use super::*;

fn route(id: &str) -> HostedModelRoute {
    HostedModelRoute::new(
        id,
        "shared-model",
        "Hosted Model",
        HostedProtocol::OpenAiCompatible,
        HostedModelCapabilities::default(),
    )
    .unwrap()
}

fn state() -> EditorState {
    let mut state = EditorState::new();
    state.editor_ui.account = crate::AccountState::signed_in_profile("test".into(), None);
    state.editor_ui.agent_settings.add_builtin_agent_config(
        "BYOK",
        "test-api-key",
        "shared-model",
        crate::BuiltinAgentKind::OpenAiCompat,
        "https://example.test/v1",
    );
    state
        .replace_hosted_model_catalog(vec![route("route-a"), route("route-b")])
        .unwrap();
    state
}

fn queue(state: &mut EditorState, route_id: Option<&str>) {
    state.chat.selected_model = state
        .chat
        .available_models
        .iter()
        .position(|entry| match route_id {
            Some(id) => entry.hosted_route_id() == Some(id),
            None => entry.builtin_provider_id.is_some(),
        })
        .unwrap();
    state
        .chat
        .messages
        .push(crate::ChatMessage::assistant("completed transcript"));
    state
        .chat
        .pending_attachments
        .push(crate::chat::ChatAttachment {
            name: "image.png".into(),
            media_type: "image/png".into(),
            data: vec![1],
        });
    state.chat.set_input_text("queued turn");
    assert!(state.chat.begin_send());
}

#[test]
fn clear_consumes_queued_hosted_turns_in_all_tabs_without_touching_byok() {
    let mut state = state();
    queue(&mut state, Some("route-a"));
    state.chat.new_tab();
    queue(&mut state, Some("route-b"));
    state.chat.new_tab();
    queue(&mut state, None);
    let byok_before = state.chat.clone();

    state.clear_hosted_model_catalog();

    for tab in &state.chat.tabs()[..2] {
        assert!(tab.pending_send.is_none());
        assert!(tab.pending_attachments.is_empty());
        assert!(!tab.messages.last().unwrap().streaming);
        assert_eq!(tab.messages[0].content, "completed transcript");
    }
    assert_eq!(state.chat.active_index(), 2);
    assert_eq!(state.chat.pending_send, byok_before.pending_send);
    assert_eq!(
        state.chat.pending_attachments,
        byok_before.pending_attachments
    );
    assert_eq!(state.chat.messages, byok_before.messages);
    assert!(state
        .chat
        .selected_model_entry()
        .unwrap()
        .builtin_provider_id
        .is_some());
}

#[test]
fn replacement_cancels_only_removed_routes_and_preserves_retained_pending_turn() {
    let mut state = state();
    queue(&mut state, Some("route-a"));
    state.chat.new_tab();
    queue(&mut state, Some("route-b"));

    state
        .replace_hosted_model_catalog(vec![route("route-b")])
        .unwrap();

    let removed = &state.chat.tabs()[0];
    assert!(removed.pending_send.is_none());
    assert!(removed.pending_attachments.is_empty());
    assert!(!removed.messages.last().unwrap().streaming);
    assert_eq!(state.chat.pending_send.as_deref(), Some("queued turn"));
    assert_eq!(state.chat.pending_attachments.len(), 1);
    assert!(state.chat.messages.last().unwrap().streaming);
    assert_eq!(
        state.chat.selected_model_entry().unwrap().hosted_route_id(),
        Some("route-b")
    );
}

#[test]
fn invalid_replacement_leaves_existing_queued_route_untouched() {
    let mut state = state();
    queue(&mut state, Some("route-a"));
    assert!(state
        .replace_hosted_model_catalog(vec![route("duplicate"), route("duplicate")])
        .is_err());
    assert_eq!(state.chat.pending_send.as_deref(), Some("queued turn"));
    assert!(state.chat.messages.last().unwrap().streaming);
    assert_eq!(
        state.chat.selected_model_entry().unwrap().hosted_route_id(),
        Some("route-a")
    );
}

fn queue_codegen_and_retry(state: &mut EditorState) {
    state.codegen.pending_generate = true;
    state.codegen.pending_regenerate = true;
    let mut message = crate::ChatMessage::assistant("completed design");
    message.activities.push(crate::ChatActivity {
        id: "section".into(),
        title: "Section".into(),
        detail: None,
        status: crate::ChatActivityStatus::Error,
        content_offset: Some(0),
    });
    message
        .failed_subtasks
        .push(crate::chat_activity::PendingSubtaskRetry {
            subtask_id: "section".into(),
            subtask_json: "{}".into(),
            insert_after_sibling_id: None,
        });
    state.chat.messages.push(message);
    let index = state.chat.messages.len() - 1;
    state.chat.begin_subtask_retry(index, 0);
    assert!(state.chat.pending_subtask_retry.is_some());
}

#[test]
fn removing_selected_hosted_cancels_codegen_and_retry_without_a_pending_send() {
    for clear in [false, true] {
        let mut state = state();
        state.chat.selected_model = state
            .chat
            .available_models
            .iter()
            .position(|entry| entry.hosted_route_id() == Some("route-a"))
            .unwrap();
        queue_codegen_and_retry(&mut state);
        assert!(state.chat.pending_send.is_none());
        if clear {
            state.clear_hosted_model_catalog();
        } else {
            state
                .replace_hosted_model_catalog(vec![route("route-b")])
                .unwrap();
        }
        assert!(!state.codegen.pending_generate);
        assert!(!state.codegen.pending_regenerate);
        assert!(state.chat.pending_subtask_retry.is_none());
        assert_eq!(
            state.chat.messages.last().unwrap().content,
            "completed design"
        );
        assert_eq!(
            state.chat.messages.last().unwrap().activities[0].status,
            crate::ChatActivityStatus::Error
        );
    }
}

#[test]
fn removing_background_hosted_keeps_active_byok_codegen_and_retry_flags() {
    let mut state = state();
    state.chat.selected_model = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.hosted_route_id() == Some("route-a"))
        .unwrap();
    queue_codegen_and_retry(&mut state);
    state.chat.new_tab();
    state.chat.selected_model = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.is_some())
        .unwrap();
    queue_codegen_and_retry(&mut state);
    let byok_retry = state.chat.pending_subtask_retry.clone();
    state.clear_hosted_model_catalog();
    assert!(state.chat.tabs()[0].pending_subtask_retry.is_none());
    assert!(state.codegen.pending_generate);
    assert!(state.codegen.pending_regenerate);
    assert_eq!(state.chat.pending_subtask_retry, byok_retry);
    assert_eq!(
        state.chat.messages.last().unwrap().activities[0].status,
        crate::ChatActivityStatus::Running
    );
}

#[test]
fn retained_hosted_and_invalid_replacement_keep_codegen_and_retry_flags() {
    let mut state = state();
    state.chat.selected_model = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.hosted_route_id() == Some("route-b"))
        .unwrap();
    queue_codegen_and_retry(&mut state);
    let retry = state.chat.pending_subtask_retry.clone();
    state
        .replace_hosted_model_catalog(vec![route("route-b")])
        .unwrap();
    assert!(state.codegen.pending_generate && state.codegen.pending_regenerate);
    assert_eq!(state.chat.pending_subtask_retry, retry);
    assert!(state
        .replace_hosted_model_catalog(vec![route("duplicate"), route("duplicate")])
        .is_err());
    assert!(state.codegen.pending_generate && state.codegen.pending_regenerate);
    assert_eq!(state.chat.pending_subtask_retry, retry);
}

#[test]
fn changed_route_payload_consumes_queued_turn_before_fallback_selection() {
    let mut state = state();
    queue(&mut state, Some("route-a"));
    queue_codegen_and_retry(&mut state);
    let changed = HostedModelRoute::new(
        "route-a",
        "replacement-model",
        "Replacement",
        HostedProtocol::OpenAiCompatible,
        HostedModelCapabilities::default(),
    )
    .unwrap();
    state.replace_hosted_model_catalog(vec![changed]).unwrap();
    assert!(state.chat.pending_send.is_none());
    assert!(state.chat.pending_subtask_retry.is_none());
    assert!(!state.codegen.pending_generate);
    assert!(!state.codegen.pending_regenerate);
}

#[test]
fn logout_rebuild_clears_routes_and_queued_turns_in_background_tabs() {
    let mut state = state();
    queue(&mut state, Some("route-a"));
    state.chat.new_tab();
    queue(&mut state, Some("route-b"));
    state.editor_ui.account = crate::AccountState::default();
    state.rebuild_chat_models();
    assert!(state.editor_ui.hosted_model_routes().is_empty());
    for tab in state.chat.tabs() {
        assert!(tab.pending_send.is_none());
        assert!(tab
            .available_models
            .iter()
            .all(|entry| entry.hosted.is_none()));
    }
}

#[test]
fn hosted_selection_is_independent_of_cli_and_saved_byok_identity() {
    let mut state = state();
    state.editor_ui.chat_selected_agent = AgentProvider::Antigravity.index();
    let saved = state.editor_ui.agent_settings.builtin_agents.clone();
    let selected = state
        .chat
        .available_models
        .iter()
        .position(|entry| entry.hosted_route_id() == Some("route-a"))
        .unwrap();
    state.select_chat_model(selected);
    state.rebuild_chat_models();
    assert_eq!(
        state.editor_ui.chat_selected_agent,
        AgentProvider::Antigravity.index()
    );
    assert_eq!(state.editor_ui.agent_settings.builtin_agents, saved);
    let entry = state.chat.selected_model_entry().unwrap();
    assert_eq!(entry.hosted_route_id(), Some("route-a"));
    assert!(entry.builtin_model_id().is_none());
    assert!(entry.acp_agent_id().is_none());
    state.chat.set_input_text("hello");
    assert!(state.chat.begin_send());
    assert_eq!(
        state.chat.messages.last().unwrap().agent_name.as_deref(),
        Some("OpenPencil")
    );
}
