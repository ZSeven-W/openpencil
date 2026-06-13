//! Click resolution for the web `WidgetHost` — `apply_click` routes a
//! press that no higher overlay consumed onto the chat panel, the
//! toolbar, and the layer panel. Split out of `press.rs` to keep that
//! file under the repo's 800-line cap (mirrors the native host's
//! `click.rs` sibling).

use op_editor_ui::widgets::{AIChatHit, AIChatPlaceholder, LayerPanel, LayerPanelHit, Toolbar};
use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
    pub fn apply_click(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        // glue:
        // Floating chat panel sits on top — check first so its
        // clicks don't fall through to the canvas.
        self.refresh_layout_scene();
        if let Some(chat_rect) = self.ai_chat_rect(viewport_w, viewport_h) {
            let panel = AIChatPlaceholder::from_editor(&self.editor_state);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                if let Some(target) = chat_button_press_target(&hit) {
                    self.editor_state.editor_ui.pressed_button = Some(target);
                }
                match hit {
                    AIChatHit::Inside => {
                        // Panel chrome that hit no control — blank
                        // press: blur every input (the chat's own
                        // textarea included, DOM parity).
                        self.blur_text_inputs_on_blank_press();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::FocusInput => {
                        self.editor_state.chat.focus_input_at_end(self.now_ms);
                        self.editor_state.chat.transcript_selection = None;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::SelectInputText(offset) => {
                        self.editor_state.chat.focused = true;
                        self.editor_state.chat.set_input_caret(offset, self.now_ms);
                        self.editor_state.chat.transcript_selection = None;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::Send => {
                        if self.editor_state.chat.available_models.is_empty() {
                            return true;
                        }
                        let sent = self.begin_chat_send();
                        if sent {
                            self.mark_dirty();
                        }
                        return sent;
                    }
                    AIChatHit::Stop => {
                        self.editor_state.chat.stop_streaming();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::Example(text) => {
                        self.editor_state.chat.set_input_text(text);
                        self.editor_state.chat.focus_input_at_end(self.now_ms);
                        self.editor_state.chat.transcript_selection = None;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        return false;
                    }
                    AIChatHit::Resize(_) => {
                        return false;
                    }
                    AIChatHit::ToggleCollapse => {
                        self.editor_state.chat.toggle_collapsed();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleMaximize => {
                        self.editor_state.chat.maximized = !self.editor_state.chat.maximized;
                        self.editor_state.chat.collapsed = false;
                        self.editor_state.editor_ui.close_chat_model_picker();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::NewChat => {
                        self.editor_state.chat.new_chat();
                        self.editor_state.editor_ui.close_chat_model_picker();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleModelPicker => {
                        let opening = self.editor_state.editor_ui.toggle_chat_model_picker();
                        if opening {
                            self.editor_state
                                .editor_ui
                                .chat_model_picker_input
                                .touch(self.now_ms);
                        }
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::FocusModelSearch => {
                        self.editor_state
                            .editor_ui
                            .chat_model_picker_input
                            .touch(self.now_ms);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ClearModelSearch => {
                        self.editor_state
                            .editor_ui
                            .chat_model_picker_input
                            .set_text("");
                        self.editor_state.editor_ui.chat_model_picker.scroll.offset = 0.0;
                        self.editor_state.editor_ui.chat_model_picker.hover = None;
                        self.editor_state.editor_ui.chat_model_picker.pressed = None;
                        self.editor_state
                            .editor_ui
                            .chat_model_picker_input
                            .touch(self.now_ms);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::SelectModel(idx) => {
                        self.editor_state.select_chat_model(idx);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::CycleThinking => {
                        self.editor_state.chat.cycle_thinking_mode();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::CycleEffort => {
                        self.editor_state.chat.cycle_effort_level();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::CycleAgentTeam => {
                        self.editor_state.chat.cycle_agent_team_size();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::AddAttachment => {
                        // The web shell has no native file picker wired
                        // yet — staging an attachment is a desktop-only
                        // path for now. No-op so the click is consumed.
                        return true;
                    }
                    AIChatHit::RemoveAttachment(idx) => {
                        self.editor_state.chat.remove_attachment(idx);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleThinking(idx) => {
                        self.editor_state.chat.toggle_message_thinking(idx);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleToolCalls(idx) => {
                        self.editor_state.chat.toggle_message_tool_calls(idx);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::SetToolCallCardExpanded(msg_idx, tool_idx, expanded) => {
                        self.editor_state
                            .chat
                            .set_message_tool_call_expanded(msg_idx, tool_idx, expanded);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::SetDesignBlockExpanded(msg_idx, block_idx, expanded) => {
                        self.editor_state
                            .chat
                            .set_message_design_block_expanded(msg_idx, block_idx, expanded);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::CopyDesignBlock(text) => {
                        self.editor_state.chat.queue_copy_text(text);
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ApplyDesignBlock(msg_idx, text) => {
                        return self.apply_chat_design_block(msg_idx, &text);
                    }
                    AIChatHit::SelectTranscriptText(message_index, offset) => {
                        self.editor_state.chat.transcript_selection =
                            Some(op_editor_core::chat::ChatTranscriptSelection {
                                message_index,
                                anchor: offset,
                                focus: offset,
                            });
                        self.editor_state.codegen.code_selection = None;
                        self.editor_state.chat.focused = false;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::ToggleChecklist => {
                        self.editor_state.chat.toggle_checklist_collapsed();
                        self.mark_dirty();
                        return true;
                    }
                }
            }
        }
        // Click outside the chat panel — blank press for the chat
        // (and every other text input): blur + commit through the
        // central helper so a panel-gap click can't strand a focused
        // input behind this block's early-consume return.
        let was_focused = self.blur_text_inputs_on_blank_press();
        self.mark_dirty();

        let toolbar_rect = self.toolbar_rect(viewport_w);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
            self.editor_state.editor_ui.pressed_button =
                Some(op_editor_core::ButtonPressTarget::Toolbar(
                    op_editor_ui::widgets::editor_state_ext::toolbar_hover(hit),
                ));
            match hit {
                op_editor_ui::widgets::ToolbarHit::Tool(tool) => {
                    self.editor_state.tool = tool;
                    self.mark_dirty();
                    return true;
                }
                op_editor_ui::widgets::ToolbarHit::Action(action) => {
                    return self.dispatch_toolbar_action(action);
                }
                op_editor_ui::widgets::ToolbarHit::ToggleShapePicker => {
                    let picker = &mut self.editor_state.editor_ui.shape_picker;
                    picker.open = !picker.open;
                    picker.hover = None;
                    picker.pressed = None;
                    if picker.open {
                        picker.scroll.offset = 0.0;
                    }
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if !self.editor_state.editor_ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = LayerPanel::from_editor(&self.editor_state);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            match hit {
                LayerPanelHit::Page(idx) => {
                    let _ = self.editor_state.set_active_page(idx);
                    self.editor_state.clear_selection();
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::Layer(node_id) => {
                    let ec_id = node_id.clone();
                    if self.shift_held {
                        self.editor_state.toggle_selection(ec_id);
                    } else {
                        self.editor_state.set_single_selection(ec_id);
                    }
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::ToggleHidden(node_id) => {
                    self.editor_state.toggle_node_hidden(&node_id.clone());
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::ToggleLocked(node_id) => {
                    self.editor_state.toggle_node_locked(&node_id.clone());
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::ToggleCollapsed(node_id) => {
                    self.editor_state.toggle_node_collapsed(&node_id.clone());
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::AddPage => {
                    let _ = self.editor_state.add_page();
                    self.mark_dirty();
                    return true;
                }
                LayerPanelHit::DeletePage(idx) => {
                    let _ = self.editor_state.remove_page(idx);
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // Defocusing the chat input itself is a visible change —
        // the caller should still repaint to drop the caret.
        was_focused
    }

    /// Chat send dispatch, shared by the Send button (above) and the Enter
    /// key (`keyboard.rs::apply_send`). With the AI transport build
    /// (`codegen`), `begin_send` pushes the user message + a streaming
    /// assistant bubble and raises `chat.pending_send`; the DOM listeners
    /// drain it into `crate::web_chat` once their borrow is released.
    ///
    /// A build WITHOUT `codegen` has no daemon transport compiled in, so
    /// it reports an honest per-send error instead of the retired
    /// `ChatState::send()` echo stub ("(stub) Got it — …" faked an
    /// assistant reply; campaign residual "web echo→真流式"). TS parity:
    /// the TS web app never shipped an offline echo — chat errored when
    /// `/api/ai/stream` was unreachable.
    pub(in crate::widget_host) fn begin_chat_send(&mut self) -> bool {
        #[cfg(feature = "codegen")]
        {
            self.editor_state.chat.begin_send()
        }
        #[cfg(not(feature = "codegen"))]
        {
            apply_offline_chat_error(&mut self.editor_state.chat)
        }
    }
}

/// Honest assistant-side error a `codegen`-less build shows per send —
/// no transport exists, so no fake reply may pretend one does.
// In `codegen` builds this pair is exercised by tests only — the real
// send path streams through `web_chat` instead.
#[cfg_attr(feature = "codegen", allow(dead_code))]
pub(crate) const CHAT_OFFLINE_ERROR: &str =
    "error: AI chat is not available in this build — the daemon streaming \
     transport is not compiled in. Rebuild the web bundle with the `codegen` \
     feature (tools/check-wasm-bundle.sh).";

/// Push the user message and resolve its assistant bubble to
/// [`CHAT_OFFLINE_ERROR`] immediately. Used by the non-`codegen`
/// `begin_chat_send` branch (no `web_chat` drain exists there, so the
/// raised `pending_send` is consumed inline and the bubble must not be
/// left streaming forever). Compiled in every build so the codegen test
/// gate covers it; returns true when a send was actually queued.
#[cfg_attr(feature = "codegen", allow(dead_code))]
pub(crate) fn apply_offline_chat_error(chat: &mut op_editor_core::ChatState) -> bool {
    if !chat.begin_send() {
        return false;
    }
    chat.pending_send = None;
    if let Some(msg) = chat.messages.iter_mut().rev().find(|m| m.streaming) {
        msg.content = CHAT_OFFLINE_ERROR.to_string();
        msg.streaming = false;
    }
    true
}

fn chat_button_press_target(hit: &AIChatHit) -> Option<op_editor_core::ButtonPressTarget> {
    if let Some(header) = op_editor_ui::widgets::editor_state_ext::chat_header_hover(hit) {
        return Some(op_editor_core::ButtonPressTarget::ChatHeader(header));
    }
    let footer = match hit {
        AIChatHit::ToggleModelPicker => op_editor_core::ChatFooterButton::ModelPicker,
        AIChatHit::CycleAgentTeam => op_editor_core::ChatFooterButton::AgentTeam,
        AIChatHit::AddAttachment => op_editor_core::ChatFooterButton::AddAttachment,
        AIChatHit::Send => op_editor_core::ChatFooterButton::Send,
        AIChatHit::Stop => op_editor_core::ChatFooterButton::Stop,
        _ => return None,
    };
    Some(op_editor_core::ButtonPressTarget::ChatFooter(footer))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_chat_error_replaces_echo_stub_with_honest_error() {
        let mut chat = op_editor_core::ChatState::default();
        chat.set_input_text("design a login page");
        assert!(apply_offline_chat_error(&mut chat));
        // The user message is preserved; the assistant bubble carries
        // the honest unavailability error — never the retired
        // "(stub) Got it — …" echo.
        assert_eq!(chat.messages.len(), 2);
        assert_eq!(chat.messages[0].content, "design a login page");
        assert_eq!(chat.messages[1].content, CHAT_OFFLINE_ERROR);
        assert!(
            !chat.messages[1].streaming,
            "the bubble must not stream forever in a transport-less build"
        );
        assert!(
            chat.pending_send.is_none(),
            "no web_chat drain exists without codegen — the flag is consumed inline"
        );
        assert!(!chat.messages[1].content.contains("(stub)"));
    }

    #[test]
    fn offline_chat_error_ignores_empty_input() {
        let mut chat = op_editor_core::ChatState::default();
        chat.set_input_text("   ");
        assert!(!apply_offline_chat_error(&mut chat));
        assert!(chat.messages.is_empty());
    }
}
