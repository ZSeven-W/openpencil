//! Tests for the chat sub-state.
//!
//! Split out of the `chat` spine (800-line file ceiling).

use super::*;

#[test]
fn begin_send_pushes_user_plus_empty_assistant_and_raises_flag() {
    let mut chat = ChatState::default();
    chat.set_input_text("  design a login page  ");
    assert!(chat.begin_send());
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.messages[0].role, ChatRole::User);
    assert_eq!(chat.messages[0].content, "design a login page");
    assert_eq!(chat.messages[1].role, ChatRole::Assistant);
    assert!(chat.messages[1].content.is_empty());
    assert!(chat.input.text().is_empty());
    assert_eq!(chat.pending_send.as_deref(), Some("design a login page"));
}

#[test]
fn begin_send_auto_titles_new_chat_from_first_prompt() {
    let mut chat = ChatState::default();
    assert_eq!(chat.title, "New Chat");

    chat.set_input_text(
        "设计一个现代的移动端登录页面，包含邮箱输入框、密码输入框、登录按钮和社交登录选项",
    );
    assert!(chat.begin_send());
    assert_eq!(chat.title, "现代移动端登录页面");

    chat.set_input_text("设计一个新的设置页面");
    assert!(chat.begin_send());
    assert_eq!(
        chat.title, "现代移动端登录页面",
        "a later turn must not overwrite the existing conversation title"
    );

    chat.new_chat();
    assert_eq!(chat.title, "New Chat");
}

#[test]
fn begin_send_expands_minimized_panel_for_streaming_turn() {
    let mut chat = ChatState::default();
    chat.minimize();
    chat.set_input_text("design a pricing page");

    assert!(chat.begin_send());

    assert!(
        !chat.is_minimized(),
        "streaming output should reopen the chat panel like the TS isStreaming effect"
    );
}

#[test]
fn legacy_collapsed_state_resolves_to_the_minimized_bar() {
    // The header-only middle state is retired: state carrying the old
    // flag must land on the compact bar, not on a form that no longer
    // paints.
    let chat = ChatState {
        collapsed: true,
        ..Default::default()
    };
    assert!(chat.is_minimized());
}

#[test]
fn toggling_walks_between_exactly_two_forms_and_clears_the_legacy_flag() {
    let mut chat = ChatState {
        collapsed: true,
        ..Default::default()
    };

    // Legacy-collapsed → expanded, and the legacy flag is gone for good.
    chat.toggle_minimized();
    assert!(!chat.is_minimized());
    assert!(!chat.collapsed);
    assert!(!chat.minimized);

    // Expanded → minimized bar → expanded again.
    chat.toggle_minimized();
    assert!(chat.minimized && !chat.collapsed);
    chat.toggle_minimized();
    assert!(!chat.is_minimized());
}

#[test]
fn a_streaming_turn_refuses_to_minimize() {
    let mut chat = ChatState::default();
    chat.set_input_text("design a pricing page");
    assert!(chat.begin_send());

    chat.toggle_minimized();

    assert!(
        !chat.is_minimized(),
        "a reply arriving must not be hidden behind the bar"
    );
}

#[test]
fn begin_send_clears_a_prior_interrupted_streaming_bubble() {
    let mut chat = ChatState::default();
    chat.set_input_text("first question");
    assert!(chat.begin_send());
    // messages[1] is the in-flight assistant bubble.
    assert!(chat.messages[1].streaming);
    // The user sends again before the first turn finished — the
    // first turn is now interrupted and will never reach `Done`.
    chat.set_input_text("second question");
    assert!(chat.begin_send());
    assert!(
        !chat.messages[1].streaming,
        "the interrupted turn's bubble must stop streaming"
    );
    assert!(
        chat.messages[3].streaming,
        "only the newest assistant bubble streams"
    );
}

#[test]
fn begin_send_empty_input_no_ops() {
    let mut chat = ChatState::default();
    chat.set_input_text("   ");
    assert!(!chat.begin_send());
    assert!(chat.messages.is_empty());
    assert!(chat.pending_send.is_none());
}

#[test]
fn input_selection_replaces_only_selected_range() {
    let mut chat = ChatState::default();
    chat.input.set_text("abcdef");
    chat.input.set_caret(1, 0);
    chat.input.drag_to(3, 0);

    assert_eq!(chat.selected_input_text(), Some("bc"));
    assert!(chat.insert_input_text("X", 10));

    assert_eq!(chat.input.text(), "aXdef");
    assert_eq!(
        chat.input.selection(),
        jian_core::text_input::Selection::caret(2)
    );
}

#[test]
fn send_echo_appends_user_and_assistant() {
    let mut chat = ChatState::default();
    chat.set_input_text("hi");
    chat.send();
    assert_eq!(chat.messages.len(), 2);
    assert_eq!(chat.messages[1].role, ChatRole::Assistant);
    assert!(chat.input.text().is_empty());
}

#[test]
fn cycle_thinking_mode_wraps() {
    let mut chat = ChatState::default();
    assert_eq!(chat.thinking_mode, ThinkingMode::Adaptive);
    chat.cycle_thinking_mode();
    assert_eq!(chat.thinking_mode, ThinkingMode::Disabled);
    chat.cycle_thinking_mode();
    assert_eq!(chat.thinking_mode, ThinkingMode::Enabled);
    chat.cycle_thinking_mode();
    assert_eq!(chat.thinking_mode, ThinkingMode::Adaptive);
}

#[test]
fn cycle_effort_level_wraps() {
    let mut chat = ChatState::default();
    assert_eq!(chat.effort_level, EffortLevel::Low);
    chat.cycle_effort_level();
    assert_eq!(chat.effort_level, EffortLevel::Medium);
    chat.cycle_effort_level();
    assert_eq!(chat.effort_level, EffortLevel::High);
    chat.cycle_effort_level();
    assert_eq!(chat.effort_level, EffortLevel::Max);
    chat.cycle_effort_level();
    assert_eq!(chat.effort_level, EffortLevel::Low);
}

#[test]
fn cycle_agent_team_size_wraps_one_through_six() {
    let mut chat = ChatState::default();
    assert_eq!(chat.agent_team_size, 1);
    chat.cycle_agent_team_size();
    assert_eq!(chat.agent_team_size, 2);
    chat.agent_team_size = 6;
    chat.cycle_agent_team_size();
    assert_eq!(chat.agent_team_size, 1);
}

#[test]
fn add_and_remove_attachment() {
    let mut chat = ChatState::default();
    assert!(chat.pending_attachments.is_empty());
    chat.add_attachment(ChatAttachment {
        name: "a.png".into(),
        media_type: "image/png".into(),
        data: vec![1],
    });
    chat.add_attachment(ChatAttachment {
        name: "b.png".into(),
        media_type: "image/png".into(),
        data: vec![2],
    });
    assert_eq!(chat.pending_attachments.len(), 2);
    chat.remove_attachment(0);
    assert_eq!(chat.pending_attachments.len(), 1);
    assert_eq!(chat.pending_attachments[0].name, "b.png");
    // Out-of-range remove is a no-op.
    chat.remove_attachment(9);
    assert_eq!(chat.pending_attachments.len(), 1);
}

#[test]
fn begin_send_leaves_pending_attachments_for_host_to_drain() {
    let mut chat = ChatState::default();
    chat.set_input_text("design with this");
    chat.add_attachment(ChatAttachment {
        name: "ref.png".into(),
        media_type: "image/png".into(),
        data: vec![9],
    });
    assert!(chat.begin_send());
    // begin_send clears the input but NOT the attachments — the
    // host copies them into the ChatRequest, then clears.
    assert_eq!(chat.pending_attachments.len(), 1);
}

#[test]
fn add_attachment_enforces_count_cap() {
    let mut chat = ChatState::default();
    for i in 0..MAX_ATTACHMENTS {
        assert!(chat.add_attachment(ChatAttachment {
            name: format!("{i}.png"),
            media_type: "image/png".into(),
            data: vec![1],
        }));
    }
    // The cap is reached — a further attachment is rejected.
    assert!(!chat.add_attachment(ChatAttachment {
        name: "extra.png".into(),
        media_type: "image/png".into(),
        data: vec![1],
    }));
    assert_eq!(chat.pending_attachments.len(), MAX_ATTACHMENTS);
}

#[test]
fn add_attachment_rejects_oversized_file() {
    let mut chat = ChatState::default();
    let huge = ChatAttachment {
        name: "big.png".into(),
        media_type: "image/png".into(),
        data: vec![0u8; MAX_ATTACHMENT_BYTES + 1],
    };
    assert!(!chat.add_attachment(huge));
    assert!(chat.pending_attachments.is_empty());
}

#[test]
fn begin_send_allows_attachment_only_message() {
    let mut chat = ChatState::default();
    chat.add_attachment(ChatAttachment {
        name: "ref.png".into(),
        media_type: "image/png".into(),
        data: vec![9],
    });
    // Empty text but a staged attachment — still sendable.
    assert!(chat.begin_send());
    assert_eq!(chat.pending_attachments.len(), 1);
}

#[test]
fn chat_message_user_constructor_has_empty_structured_fields() {
    let m = ChatMessage::user("hello");
    assert_eq!(m.role, ChatRole::User);
    assert_eq!(m.content, "hello");
    assert!(m.thinking.is_empty());
    assert!(m.tool_calls.is_empty());
    assert!(m.images.is_empty());
    assert!(!m.streaming);
}

#[test]
fn begin_send_marks_only_the_assistant_message_streaming() {
    let mut chat = ChatState::default();
    chat.set_input_text("design something");
    assert!(chat.begin_send());
    assert!(!chat.messages[0].streaming, "user message is not streaming");
    assert!(
        chat.messages[1].streaming,
        "the empty assistant bubble is streaming until the turn ends"
    );
}

#[test]
fn begin_send_copies_image_attachments_into_user_message_with_unique_ids() {
    let mut chat = ChatState::default();
    chat.set_input_text("look at these");
    chat.add_attachment(ChatAttachment {
        name: "a.png".into(),
        media_type: "image/png".into(),
        data: vec![1],
    });
    chat.add_attachment(ChatAttachment {
        name: "b.png".into(),
        media_type: "image/png".into(),
        data: vec![2],
    });
    assert!(chat.begin_send());
    let user = &chat.messages[0];
    assert_eq!(user.images.len(), 2, "both images shown in the bubble");
    assert_eq!(user.images[0].name, "a.png");
    assert_eq!(user.images[0].data, vec![1]);
    assert_ne!(
        user.images[0].id, user.images[1].id,
        "each image gets a distinct decode-cache id"
    );
    // The host still drains pending_attachments into the request.
    assert_eq!(chat.pending_attachments.len(), 2);
}

#[test]
fn image_ids_never_collide_across_fresh_chat_states() {
    // A "New Chat" makes a fresh ChatState — its image ids must
    // not restart at 0 and collide with a still-cached decode.
    let mut a = ChatState::default();
    a.set_input_text("x");
    a.add_attachment(ChatAttachment {
        name: "a.png".into(),
        media_type: "image/png".into(),
        data: vec![1],
    });
    a.begin_send();
    let first_id = a.messages[0].images[0].id;

    let mut b = ChatState::default();
    b.set_input_text("y");
    b.add_attachment(ChatAttachment {
        name: "b.png".into(),
        media_type: "image/png".into(),
        data: vec![2],
    });
    b.begin_send();
    assert_ne!(
        first_id, b.messages[0].images[0].id,
        "a fresh ChatState must not reuse image ids"
    );
}

#[test]
fn begin_send_skips_non_image_attachments_for_the_bubble() {
    let mut chat = ChatState::default();
    chat.set_input_text("and a doc");
    chat.add_attachment(ChatAttachment {
        name: "notes.txt".into(),
        media_type: "text/plain".into(),
        data: vec![7],
    });
    assert!(chat.begin_send());
    // A non-image attachment can't be drawn — keep it out of the
    // bubble's image strip (the host still sends it).
    assert!(chat.messages[0].images.is_empty());
}

#[test]
fn toggle_message_thinking_flips_collapsed_flag() {
    let mut chat = ChatState::default();
    chat.messages.push(ChatMessage::assistant("hi"));
    let before = chat.messages[0].thinking_collapsed;
    chat.toggle_message_thinking(0);
    assert_eq!(chat.messages[0].thinking_collapsed, !before);
    // Out-of-range index is a no-op (must not panic).
    chat.toggle_message_thinking(99);
}

#[test]
fn toggle_message_tool_calls_flips_collapsed_flag() {
    let mut chat = ChatState::default();
    chat.messages.push(ChatMessage::assistant("hi"));
    let before = chat.messages[0].tools_collapsed;
    chat.toggle_message_tool_calls(0);
    assert_eq!(chat.messages[0].tools_collapsed, !before);
    chat.toggle_message_tool_calls(99);
}

#[test]
fn set_message_tool_call_expanded_records_per_card_override() {
    let mut chat = ChatState::default();
    let mut msg = ChatMessage::assistant("hi");
    msg.tool_calls.push(ChatToolCall {
        name: "snapshot_layout".into(),
        args: "{}".into(),
        content_offset: None,
    });
    chat.messages.push(msg);

    chat.set_message_tool_call_expanded(0, 0, true);
    assert_eq!(
        chat.messages[0].tool_call_expanded_overrides,
        vec![Some(true)]
    );

    chat.set_message_tool_call_expanded(0, 99, false);
    chat.set_message_tool_call_expanded(99, 0, false);
    assert_eq!(
        chat.messages[0].tool_call_expanded_overrides,
        vec![Some(true)]
    );
}

#[test]
fn set_message_action_step_expanded_records_per_card_override() {
    let mut chat = ChatState::default();
    chat.messages.push(ChatMessage::assistant("hi"));

    chat.set_message_action_step_expanded(0, 1, true);
    assert_eq!(
        chat.messages[0].action_step_expanded_overrides,
        vec![None, Some(true)]
    );

    // Out-of-range message index is a no-op.
    chat.set_message_action_step_expanded(99, 0, false);
    assert_eq!(
        chat.messages[0].action_step_expanded_overrides,
        vec![None, Some(true)]
    );
}

/// End-to-end proof of the failed-subtask remediation data model, from
/// the click handler's own perspective: a message carrying BOTH a
/// persisted request (`design_request_json_for_retry`, stashed at
/// launch — either desktop route) AND a persisted failed-subtask spec
/// (`failed_subtasks`, captured by `pump_progress` from the RunSummary)
/// must let `begin_subtask_retry` find it, flip the row to `Running`,
/// clear its stale detail, and raise `pending_subtask_retry`.
#[test]
fn begin_subtask_retry_finds_a_fully_persisted_row_and_raises_the_pending_flag() {
    let mut chat = ChatState::default();
    let mut msg = ChatMessage::assistant("designing");
    msg.design_request_json_for_retry = Some("{\"prompt\":\"p\"}".into());
    msg.activities.push(ChatActivity {
        id: "hero".into(),
        title: "Hero".into(),
        detail: Some("Needs attention".into()),
        status: ChatActivityStatus::Error,
        content_offset: Some(0),
    });
    msg.failed_subtasks
        .push(crate::chat_activity::PendingSubtaskRetry {
            subtask_id: "hero".into(),
            subtask_json: "{\"id\":\"hero\"}".into(),
        });
    chat.messages.push(msg);

    chat.begin_subtask_retry(0, 0);

    assert_eq!(
        chat.messages[0].activities[0].status,
        ChatActivityStatus::Running
    );
    assert_eq!(chat.messages[0].activities[0].detail, None);
    assert_eq!(
        chat.pending_subtask_retry,
        Some((0, "hero".into())),
        "the desktop host drains this to launch the retry worker"
    );
}

#[test]
fn begin_subtask_retry_is_a_noop_without_a_persisted_spec() {
    // Mirrors a whole-run catastrophic failure: every activity flips to
    // Error but no RunSummary ever landed, so nothing is in
    // `failed_subtasks` — clicking must not raise a phantom retry.
    let mut chat = ChatState::default();
    let mut msg = ChatMessage::assistant("designing");
    msg.design_request_json_for_retry = Some("{\"prompt\":\"p\"}".into());
    msg.activities.push(ChatActivity {
        id: "hero".into(),
        title: "Hero".into(),
        detail: Some("Needs attention".into()),
        status: ChatActivityStatus::Error,
        content_offset: Some(0),
    });
    chat.messages.push(msg);

    chat.begin_subtask_retry(0, 0);

    assert_eq!(
        chat.messages[0].activities[0].status,
        ChatActivityStatus::Error,
        "the row must stay Error, not flip to a phantom Running"
    );
    assert_eq!(chat.pending_subtask_retry, None);
}

#[test]
fn set_message_design_block_expanded_records_per_card_override() {
    let mut chat = ChatState::default();
    chat.messages.push(ChatMessage::assistant("hi"));

    chat.set_message_design_block_expanded(0, 1, true);
    assert_eq!(
        chat.messages[0].design_block_expanded_overrides,
        vec![None, Some(true)]
    );

    chat.set_message_design_block_expanded(99, 0, false);
    assert_eq!(
        chat.messages[0].design_block_expanded_overrides,
        vec![None, Some(true)]
    );
}

#[test]
fn queue_copy_text_records_pending_clipboard_payload() {
    let mut chat = ChatState::default();

    chat.queue_copy_text("json");

    assert_eq!(chat.pending_copy_text.as_deref(), Some("json"));
}

#[test]
fn nearest_anchor_picks_corner() {
    let p = crate::render_backend::Point2D::new(10.0, 10.0);
    assert_eq!(
        ChatAnchor::nearest(p, 0.0, 0.0, 100.0, 100.0),
        ChatAnchor::TopLeft
    );
    let p2 = crate::render_backend::Point2D::new(90.0, 90.0);
    assert_eq!(
        ChatAnchor::nearest(p2, 0.0, 0.0, 100.0, 100.0),
        ChatAnchor::BottomRight
    );
}

#[test]
fn rebuild_available_models_keeps_only_connected_providers() {
    let mut chat = ChatState {
        discovered_models: vec![
            ModelEntry::new(AgentProvider::ClaudeCode, "opus", "Opus"),
            ModelEntry::new(AgentProvider::ClaudeCode, "sonnet", "Sonnet"),
            ModelEntry::new(AgentProvider::CodexCli, "gpt-5.5", "GPT-5.5"),
            ModelEntry::new(AgentProvider::OpenCode, "oc/x", "oc/x"),
        ],
        ..Default::default()
    };
    // Only Claude Code (index 0 of AgentProvider::ALL) connected.
    let mut connected = [false; 7];
    connected[0] = true;
    chat.rebuild_available_models(&connected);
    assert_eq!(chat.available_models.len(), 2);
    assert!(chat
        .available_models
        .iter()
        .all(|m| m.provider == AgentProvider::ClaudeCode));
}

#[test]
fn rebuild_available_models_preserves_selection_by_identity() {
    let mut chat = ChatState {
        discovered_models: vec![
            ModelEntry::new(AgentProvider::ClaudeCode, "opus", "Opus"),
            ModelEntry::new(AgentProvider::CodexCli, "gpt-5.5", "GPT-5.5"),
        ],
        ..Default::default()
    };
    let mut connected = [false; 7];
    connected[0] = true; // Claude
    connected[1] = true; // Codex
    chat.rebuild_available_models(&connected);
    // Select Codex's GPT-5.5 (index 1).
    chat.selected_model = 1;
    // Disconnecting Claude drops index 0 — the selection must
    // follow GPT-5.5 to its new index rather than dangle.
    connected[0] = false;
    chat.rebuild_available_models(&connected);
    assert_eq!(chat.available_models.len(), 1);
    assert_eq!(chat.selected_model, 0);
    assert_eq!(chat.available_models[0].value, "gpt-5.5");
    // Disconnecting the last provider empties the list and the
    // selection clamps back to 0.
    connected[1] = false;
    chat.rebuild_available_models(&connected);
    assert!(chat.available_models.is_empty());
    assert_eq!(chat.selected_model, 0);
}

#[test]
fn builtin_model_id_strips_the_exact_structured_provider_prefix() {
    let entry = ModelEntry::builtin_with_display_name(
        AgentProvider::CodexCli,
        "web-credential:builtin:account:7",
        "Provider",
        "builtin:web-credential:builtin:account:7:deployment:blue",
        "Blue",
    );

    assert_eq!(entry.builtin_model_id(), Some("deployment:blue"));

    let malformed = ModelEntry::builtin(
        AgentProvider::CodexCli,
        "expected:id",
        "builtin:different:id:model",
        "Model",
    );
    assert_eq!(malformed.builtin_model_id(), None);
}
