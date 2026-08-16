//! Chat-panel click dispatch shared by the native and web widget
//! hosts. Both `widget_host/click.rs` twins used to carry this ~250
//! line `match` verbatim; they now map their hit-test result through
//! [`apply_chat_hit`] and only run the platform tail each step asks
//! for (`mark_dirty`, transcript-cache owner rotation, chat transport,
//! …).
//!
//! Everything here is pure `EditorState` mutation plus the widget-layer
//! [`AIChatHit`] type, so it lives in op-editor-ui (wasm32-clean) rather
//! than op-editor-core.

use op_editor_core::{ButtonPressTarget, ChatFooterButton, EditorState};

use crate::widgets::{editor_state_ext, AIChatHit};

/// Residual host work after [`apply_chat_hit`] applied the shared
/// editor-state transition for a chat hit.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatClickStep {
    /// Press-drag only hit (drag handle / resize edge) — the click
    /// path does not consume it; the caller returns `false`.
    Unhandled,
    /// Consumed; the caller returns `true` WITHOUT marking dirty.
    Clean,
    /// Consumed; the caller marks dirty and returns `true`.
    Dirty,
    /// Blank press inside the panel chrome: the caller blurs every
    /// text input, marks dirty and returns `true`.
    BlankPress,
    /// The active chat session changed (new tab / tab switch): the
    /// caller rotates the transcript-cache owner, marks dirty and
    /// returns `true`.
    RotateChatOwner,
    /// The model picker was toggled. When `opening`, the caller also
    /// clears the hover state covered by the popup. Marks dirty and
    /// returns `true`.
    ModelPickerToggled { opening: bool },
    /// Host-specific follow-up — see [`ChatHostAction`].
    Host(ChatHostAction),
}

/// Chat actions whose implementation is genuinely per-host: the send
/// transport, the design-block applier, and the failed-subtask retry
/// pipeline (desktop-only machinery).
#[derive(Debug, Clone, PartialEq)]
pub enum ChatHostAction {
    /// Send affordance pressed.
    Send,
    /// Apply a transcript design block onto the document.
    ApplyDesignBlock { message_index: usize, text: String },
    /// Retry a failed subtask row.
    RetrySubtask {
        message_index: usize,
        source_index: usize,
    },
}

/// Map a chat hit onto the pressed-button wash target (footer chips,
/// header buttons, example cards).
pub fn chat_button_press_target(hit: &AIChatHit) -> Option<ButtonPressTarget> {
    if let Some(header) = editor_state_ext::chat_header_hover(hit) {
        return Some(ButtonPressTarget::ChatHeader(header));
    }
    if let AIChatHit::Example { index, .. } = hit {
        return Some(ButtonPressTarget::ChatExample(*index));
    }
    let footer = match hit {
        AIChatHit::OpenPromptCenter => ChatFooterButton::PromptCenter,
        AIChatHit::ToggleModelPicker => ChatFooterButton::ModelPicker,
        // The ⚡ chip is now the Parallel Agents chip — pressed state uses SpeedChip.
        AIChatHit::ToggleParallelAgentsPicker => ChatFooterButton::SpeedChip,
        AIChatHit::CycleAgentTeam => ChatFooterButton::AgentTeam,
        AIChatHit::CycleThinking => ChatFooterButton::ThinkingMode,
        AIChatHit::AddAttachment => ChatFooterButton::AddAttachment,
        AIChatHit::Send => ChatFooterButton::Send,
        AIChatHit::Stop => ChatFooterButton::Stop,
        _ => return None,
    };
    Some(ButtonPressTarget::ChatFooter(footer))
}

/// Apply the shared editor-state transition for a chat-panel click and
/// report what the host still has to do.
pub fn apply_chat_hit(state: &mut EditorState, hit: AIChatHit, now_ms: u64) -> ChatClickStep {
    if let Some(target) = chat_button_press_target(&hit) {
        state.editor_ui.pressed_button = Some(target);
    }
    match hit {
        // Panel chrome that hit no control — blank press: blur every
        // input (the chat's own textarea included, DOM parity).
        AIChatHit::Inside => ChatClickStep::BlankPress,
        // Same shared entry point the Asset Center card uses.
        AIChatHit::ClearPinnedStyle => {
            if state.editor_ui.clear_pinned_style_guide() {
                ChatClickStep::Dirty
            } else {
                ChatClickStep::Clean
            }
        }
        AIChatHit::FocusInput => {
            state.chat.focus_input_at_end(now_ms);
            state.chat.transcript_selection = None;
            ChatClickStep::Dirty
        }
        AIChatHit::SelectInputText(offset) => {
            state.chat.focused = true;
            state.chat.set_input_caret(offset, now_ms);
            state.chat.transcript_selection = None;
            ChatClickStep::Dirty
        }
        AIChatHit::Send => ChatClickStep::Host(ChatHostAction::Send),
        AIChatHit::Stop => {
            state.chat.stop_streaming();
            ChatClickStep::Dirty
        }
        AIChatHit::Example { prompt, .. } => {
            state.chat.set_input_text(prompt);
            state.chat.focus_input_at_end(now_ms);
            state.chat.transcript_selection = None;
            ChatClickStep::Dirty
        }
        // Drag handle + resize edges are press-drag gestures handled in
        // `apply_press` ahead of the click path; reaching here is a
        // path bypass.
        AIChatHit::DragHandle | AIChatHit::Resize(_) => ChatClickStep::Unhandled,
        AIChatHit::ToggleCollapse => {
            let expanding = state.chat.is_minimized();
            state.chat.toggle_minimized();
            // Expanding from the bar hands the user straight to the
            // textarea — the bar reads as an input, so a click on it is
            // an intent to type.
            if expanding {
                state.chat.focus_input_at_end(now_ms);
            }
            state.editor_ui.close_chat_model_picker();
            ChatClickStep::Dirty
        }
        AIChatHit::ToggleMaximize => {
            state.chat.maximized = !state.chat.maximized;
            state.chat.expand();
            state.editor_ui.close_chat_model_picker();
            ChatClickStep::Dirty
        }
        AIChatHit::NewChat => {
            // "+" opens a fresh, PRESERVED tab — the old tab keeps its
            // transcript intact (MT.1-review regression fix: `new_chat()`
            // reset/wiped the active tab in place, and the host drain then
            // pushed a SECOND tab → one "+" click double-created + wiped
            // history). `new_tab()` pushes one blank tab and activates it;
            // `pending_new_chat` rides to the host drain so it aborts the
            // in-flight turn WITHOUT creating another tab.
            state.chat.new_tab();
            state.chat.pending_new_chat = true;
            state.editor_ui.close_chat_model_picker();
            // Session set mutated: the host rotates the transcript-cache
            // owner NOW so a pre-paint pointer move can't cross-pair the
            // old tab's geometry with the new tab's messages.
            ChatClickStep::RotateChatOwner
        }
        AIChatHit::ToggleModelPicker => {
            let opening = state.editor_ui.toggle_chat_model_picker();
            if opening {
                // The chat picker is a read-only view of models already saved
                // in provider settings. Rebuild locally when it opens; model
                // discovery belongs exclusively to the settings workflow.
                state.rebuild_chat_models();
                // Close the parallel-agents picker when model picker opens.
                state.editor_ui.close_parallel_agents_picker();
                state.editor_ui.chat_model_picker_input.touch(now_ms);
            }
            ChatClickStep::ModelPickerToggled { opening }
        }
        AIChatHit::FocusModelSearch => {
            state.editor_ui.chat_model_picker_input.touch(now_ms);
            ChatClickStep::Dirty
        }
        AIChatHit::ClearModelSearch => {
            state.editor_ui.chat_model_picker_input.set_text("");
            state.editor_ui.chat_model_picker.scroll.offset = 0.0;
            state.editor_ui.chat_model_picker.hover = None;
            state.editor_ui.chat_model_picker.pressed = None;
            state.editor_ui.chat_model_picker_input.touch(now_ms);
            ChatClickStep::Dirty
        }
        AIChatHit::SelectModel(idx) => {
            // Commit on press so the selected model and closed picker
            // appear in the first repaint. A deferred bare index could
            // also target the wrong row if saved provider settings changed
            // before pointer release.
            state.select_chat_model(idx);
            // The consumed press schedules the repaint; the document
            // layout scene is NOT stale for this UI-only preference
            // change, so no `mark_dirty`.
            ChatClickStep::Clean
        }
        AIChatHit::CycleThinking => {
            state.chat.cycle_thinking_mode();
            ChatClickStep::Dirty
        }
        AIChatHit::CycleEffort => {
            state.chat.cycle_effort_level();
            ChatClickStep::Dirty
        }
        AIChatHit::OpenPromptCenter => {
            state.editor_ui.open_prompt_center(now_ms);
            ChatClickStep::Dirty
        }
        AIChatHit::CycleAgentTeam => {
            state.cycle_agent_team_size();
            ChatClickStep::Dirty
        }
        AIChatHit::ToggleParallelAgentsPicker => {
            // Toggle the Parallel Agents picker open/closed.
            // Also closes the model picker when opening this one.
            if !state.editor_ui.parallel_agents_picker_open {
                state.editor_ui.close_chat_model_picker();
            }
            state.editor_ui.toggle_parallel_agents_picker();
            ChatClickStep::Dirty
        }
        AIChatHit::SetParallelAgents(n) => {
            // Set the agent_team_size (mirrors into the sticky
            // `preferred_agent_team_size` preference) and close the picker.
            state.set_agent_team_size(n);
            state.editor_ui.close_parallel_agents_picker();
            ChatClickStep::Dirty
        }
        AIChatHit::AddAttachment => {
            // The host event loop drains this flag, opens its platform
            // file picker, and stages the chosen file via
            // `ChatState::add_attachment`.
            state.chat.pending_attachment_pick = true;
            ChatClickStep::Dirty
        }
        AIChatHit::RemoveAttachment(idx) => {
            state.chat.remove_attachment(idx);
            ChatClickStep::Dirty
        }
        AIChatHit::ClearSelection => {
            state.clear_selection();
            ChatClickStep::Dirty
        }
        AIChatHit::ToggleThinking(idx) => {
            state.chat.toggle_message_thinking(idx);
            ChatClickStep::Dirty
        }
        AIChatHit::ToggleToolCalls(idx) => {
            state.chat.toggle_message_tool_calls(idx);
            ChatClickStep::Dirty
        }
        AIChatHit::SetToolCallCardExpanded(msg_idx, tool_idx, expanded) => {
            state
                .chat
                .set_message_tool_call_expanded(msg_idx, tool_idx, expanded);
            ChatClickStep::Dirty
        }
        AIChatHit::SetDesignBlockExpanded(msg_idx, block_idx, expanded) => {
            state
                .chat
                .set_message_design_block_expanded(msg_idx, block_idx, expanded);
            ChatClickStep::Dirty
        }
        AIChatHit::SetActionStepExpanded(msg_idx, step_idx, expanded) => {
            state
                .chat
                .set_message_action_step_expanded(msg_idx, step_idx, expanded);
            ChatClickStep::Dirty
        }
        AIChatHit::RetrySubtask(message_index, source_index) => {
            ChatClickStep::Host(ChatHostAction::RetrySubtask {
                message_index,
                source_index,
            })
        }
        AIChatHit::CopyDesignBlock(text) => {
            state.chat.queue_copy_text(text);
            ChatClickStep::Dirty
        }
        AIChatHit::ApplyDesignBlock(message_index, text) => {
            ChatClickStep::Host(ChatHostAction::ApplyDesignBlock {
                message_index,
                text,
            })
        }
        AIChatHit::SelectTranscriptText(message_index, offset) => {
            state.chat.transcript_selection = Some(op_editor_core::chat::ChatTranscriptSelection {
                message_index,
                anchor: offset,
                focus: offset,
            });
            state.codegen.code_selection = None;
            state.chat.focused = false;
            ChatClickStep::Dirty
        }
        AIChatHit::SwitchTab(idx) => {
            // Pure editor-state change — a switch does NOT touch the run
            // binding (a run keeps streaming into its own bound tab).
            state.chat.switch_to(idx);
            state.rebuild_chat_models();
            state.editor_ui.close_chat_model_picker();
            // Active session changed: the host rotates the transcript-cache
            // owner synchronously so a pointer move arriving before the next
            // paint reads None instead of pairing the previous tab's cached
            // geometry with this tab's messages.
            ChatClickStep::RotateChatOwner
        }
        AIChatHit::CloseTab(idx) => {
            // Closing a tab can need to abort an in-flight run bound to it
            // (a session the widget layer can't reach), so defer to the
            // host runner: raise the request and let it do the close +
            // run-binding fix-up.
            state.editor_ui.pending_close_chat_tab = Some(idx);
            ChatClickStep::Dirty
        }
    }
}

/// Start an input-text selection drag from a chat press (host stores
/// its own drag-state struct around this).
pub fn begin_chat_input_selection(state: &mut EditorState, anchor: usize, now_ms: u64) {
    state.chat.focused = true;
    state.chat.set_input_caret(anchor, now_ms);
    state.chat.transcript_selection = None;
    state.codegen.code_selection = None;
}

/// Start a transcript-text selection drag from a chat press.
pub fn begin_chat_transcript_selection(
    state: &mut EditorState,
    message_index: usize,
    anchor: usize,
) {
    state.chat.transcript_selection = Some(op_editor_core::chat::ChatTranscriptSelection {
        message_index,
        anchor,
        focus: anchor,
    });
    state.codegen.code_selection = None;
    state.chat.focused = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_and_searching_model_picker_never_queue_builtin_discovery() {
        let mut state = EditorState::new();
        state.editor_ui.agent_settings.add_builtin_agent_config(
            "Provider",
            "sk-test",
            "fallback",
            op_editor_core::BuiltinAgentKind::OpenAiCompat,
            "https://example.test/v1",
        );

        assert_eq!(
            apply_chat_hit(&mut state, AIChatHit::ToggleModelPicker, 42),
            ChatClickStep::ModelPickerToggled { opening: true }
        );
        assert_eq!(
            state
                .editor_ui
                .agent_settings
                .pending_builtin_model_catalog_refreshes
                .len(),
            0
        );

        state.editor_ui.chat_model_picker_input.set_text("fall");
        assert_eq!(
            apply_chat_hit(&mut state, AIChatHit::FocusModelSearch, 43),
            ChatClickStep::Dirty
        );
        assert_eq!(
            apply_chat_hit(&mut state, AIChatHit::ClearModelSearch, 44),
            ChatClickStep::Dirty
        );
        apply_chat_hit(&mut state, AIChatHit::ToggleModelPicker, 43);
        apply_chat_hit(&mut state, AIChatHit::ToggleModelPicker, 44);
        assert_eq!(
            state
                .editor_ui
                .agent_settings
                .pending_builtin_model_catalog_refreshes
                .len(),
            0,
            "chat picker interactions must never enter the provider discovery queue"
        );
    }

    #[test]
    fn switching_tabs_uses_saved_models_and_ignores_runtime_catalogs() {
        let mut state = EditorState::new();
        let id = state.editor_ui.agent_settings.add_builtin_agent_config(
            "Provider",
            "sk-test",
            "fallback",
            op_editor_core::BuiltinAgentKind::OpenAiCompat,
            "https://example.test/v1",
        );
        state.chat.new_tab();
        state.chat.switch_to(0);
        let request = state
            .editor_ui
            .agent_settings
            .begin_builtin_model_catalog_refresh(
                op_editor_core::BuiltinModelCatalogTarget::Agent(id.clone()),
                1,
            )
            .expect("catalog request");
        state
            .editor_ui
            .agent_settings
            .take_pending_builtin_model_catalog_refresh();
        let expected = state
            .editor_ui
            .agent_settings
            .builtin_model_catalog_config_for_request(&request)
            .expect("provider snapshot");
        assert!(state
            .editor_ui
            .agent_settings
            .apply_builtin_model_catalog_refresh_outcome_if_current(
                &expected,
                &request,
                op_editor_core::BuiltinModelCatalogRefreshOutcome::Success {
                    models: vec![op_editor_core::BuiltinModelOption::new(
                        "dynamic", "Dynamic",
                    )],
                },
            ));
        state.rebuild_chat_models();
        assert!(state.chat.tabs()[0]
            .available_models
            .iter()
            .any(|entry| entry.builtin_model_id() == Some("fallback")));
        assert!(!state.chat.tabs()[1]
            .available_models
            .iter()
            .any(|entry| entry.builtin_model_id() == Some("dynamic")));

        assert_eq!(
            apply_chat_hit(&mut state, AIChatHit::SwitchTab(1), 2),
            ChatClickStep::RotateChatOwner
        );
        assert!(state
            .chat
            .available_models
            .iter()
            .any(|entry| entry.builtin_model_id() == Some("fallback")));
        assert!(!state
            .chat
            .available_models
            .iter()
            .any(|entry| entry.builtin_model_id() == Some("dynamic")));
        assert!(state
            .editor_ui
            .agent_settings
            .pending_builtin_model_catalog_refreshes
            .is_empty());
    }
}
