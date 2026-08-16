//! Click resolution for the web `WidgetHost` — `apply_click` routes a
//! press that no higher overlay consumed onto the chat panel, the
//! toolbar, and the layer panel. Split out of `press.rs` to keep that
//! file under the repo's 800-line cap (mirrors the native host's
//! `click.rs` sibling).
//!
//! The chat-panel and LayerPanel dispatches themselves live in
//! `op_editor_ui::widgets::chat_click_flow` / `press_flow` and are
//! shared verbatim with the native host; only the platform tail
//! (`mark_dirty`, transcript-cache owner rotation, the daemon chat
//! transport) stays here.

use op_editor_core::host_press_transitions as core_press;
use op_editor_ui::widgets::chat_click_flow::{self, ChatClickStep, ChatHostAction};
use op_editor_ui::widgets::press_flow::{self, LayerPanelClick};
use op_editor_ui::widgets::{AIChatHit, AIChatPlaceholder, Toolbar};
use op_editor_ui::Point2D;

use super::WidgetHost;

impl WidgetHost {
    /// Route a press inside the model picker before lower rail/panel widgets.
    /// The popup can extend outside the chat rect; dispatching through
    /// `apply_click` keeps web and native on the shared chat hit-test path.
    pub(in crate::widget_host) fn apply_chat_model_picker_overlay_press(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        if !self.editor_state.editor_ui.chat_model_picker.open {
            return false;
        }
        let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) else {
            self.editor_state.editor_ui.close_chat_model_picker();
            self.mark_dirty();
            return true;
        };
        let over_picker = AIChatPlaceholder::from_editor(&self.editor_state)
            .model_picker_bounds(chat_rect)
            .is_some_and(|picker| picker.contains(Point2D::new(x, y)));
        if !over_picker {
            return false;
        }
        let handled = self.apply_click(x, y, viewport_width, viewport_height);
        debug_assert!(handled, "a visible model-picker press must be handled");
        true
    }

    pub fn apply_click(&mut self, x: f32, y: f32, viewport_w: f32, viewport_h: f32) -> bool {
        // glue:
        // Floating chat panel sits on top — check first so its
        // clicks don't fall through to the canvas.
        self.refresh_layout_scene();
        if let Some(chat_rect) = self.ai_chat_rect(viewport_w, viewport_h) {
            let panel =
                AIChatPlaceholder::from_editor(&self.editor_state).owned_by(self.chat_panel_owner);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                return self.dispatch_chat_click(hit);
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
                    self.apply_set_tool(tool);
                    return true;
                }
                op_editor_ui::widgets::ToolbarHit::Action(action) => {
                    return self.dispatch_toolbar_action(action);
                }
                op_editor_ui::widgets::ToolbarHit::ToggleShapePicker => {
                    core_press::toggle_shape_picker(&mut self.editor_state.editor_ui);
                    self.mark_dirty();
                    return true;
                }
            }
        }
        if !self.editor_state.editor_ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = self.layer_panel_rect(viewport_h);
        let panel = self.layer_panel();
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            return match press_flow::apply_layer_panel_click(
                &mut self.editor_state,
                hit,
                self.now_ms,
                self.shift_held,
            ) {
                LayerPanelClick::Consumed => true,
                LayerPanelClick::Dirty => {
                    self.mark_dirty();
                    true
                }
                LayerPanelClick::Refit => {
                    self.fit_active_page_after_switch(viewport_w, viewport_h);
                    true
                }
                LayerPanelClick::SelectionChanged => {
                    // Web repaints only off an explicit dirty flag (the
                    // native host repaints off the consumed press itself).
                    self.mark_dirty();
                    true
                }
            };
        }
        // Defocusing the chat input itself is a visible change —
        // the caller should still repaint to drop the caret.
        was_focused
    }

    /// Platform tail for the shared chat-panel click dispatch.
    fn dispatch_chat_click(&mut self, hit: AIChatHit) -> bool {
        match chat_click_flow::apply_chat_hit(&mut self.editor_state, hit, self.now_ms) {
            // Drag handle / resize handles are press-drag only.
            ChatClickStep::Unhandled => false,
            ChatClickStep::Clean => true,
            ChatClickStep::Dirty => {
                self.mark_dirty();
                true
            }
            ChatClickStep::BlankPress => {
                self.blur_text_inputs_on_blank_press();
                self.mark_dirty();
                true
            }
            ChatClickStep::RotateChatOwner => {
                // Rotate the transcript-cache owner synchronously so a
                // pointer move before the next paint can't cross-pair the
                // previous tab's geometry with this tab's messages.
                self.force_rotate_chat_owner();
                self.mark_dirty();
                true
            }
            ChatClickStep::ModelPickerToggled { opening } => {
                if opening {
                    // Clear covered hover state synchronously with opening;
                    // a repaint may happen before the next pointer event.
                    self.clear_hover_below_chat_model_picker();
                }
                self.mark_dirty();
                true
            }
            ChatClickStep::Host(ChatHostAction::Send) => {
                if self.editor_state.chat.available_models.is_empty() {
                    return true;
                }
                let sent = self.begin_chat_send();
                if sent {
                    self.mark_dirty();
                }
                sent
            }
            ChatClickStep::Host(ChatHostAction::ApplyDesignBlock {
                message_index,
                text,
            }) => self.apply_chat_design_block(message_index, &text),
            // Inert on web: the failed-subtask retry pipeline (spec
            // retention + single-shot rerun) is desktop host machinery;
            // the web design route streams straight to the browser with no
            // ChatMessage to retain a spec on, so the transcript never
            // paints the retry icon here. Unreachable until web grows its
            // own retry session storage.
            ChatClickStep::Host(ChatHostAction::RetrySubtask { .. }) => true,
        }
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
        // Both the codegen (skia) and canvaskit builds compile a real daemon
        // transport (`web_chat`), so queue the send for the DOM drain. Only a
        // transport-less stub build falls back to the honest per-send error.
        #[cfg(feature = "canvaskit")]
        {
            self.editor_state.chat.begin_send()
        }
        #[cfg(not(feature = "canvaskit"))]
        {
            apply_offline_chat_error(&mut self.editor_state.chat)
        }
    }
}

/// Honest assistant-side error a `codegen`-less build shows per send —
/// no transport exists, so no fake reply may pretend one does.
// In `codegen` builds this pair is exercised by tests only — the real
// send path streams through `web_chat` instead.
#[cfg_attr(feature = "canvaskit", allow(dead_code))]
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
#[cfg_attr(feature = "canvaskit", allow(dead_code))]
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
