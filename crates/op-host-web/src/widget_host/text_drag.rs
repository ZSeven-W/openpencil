//! Text-offset probes and the text-selection drag moves they feed —
//! code preview, chat transcript, chat input.
//!
//! Split out of the `widget_host.rs` spine to keep it under the repo's
//! 800-line cap.

use super::*;

impl WidgetHost {
    pub(in crate::widget_host) fn code_text_offset_at_screen(
        &self,
        x: f32,
        y: f32,
    ) -> Option<usize> {
        if !self.editor_state.property_panel_visible()
            || !matches!(
                self.editor_state.editor_ui.property_tab,
                op_editor_core::PropertyTab::Code
            )
        {
            return None;
        }
        let pw = self.editor_state.editor_ui.property_panel_width;
        let panel_x = self.last_viewport_w - pw;
        if x < panel_x || x > self.last_viewport_w {
            return None;
        }
        let panel_rect = Rect {
            origin: Point2D::new(panel_x, TOP_BAR_HEIGHT),
            size: Point2D::new(pw, (self.last_viewport_h - TOP_BAR_HEIGHT).max(0.0)),
        };
        op_editor_ui::widgets::PropertyPanel::for_selection(&self.editor_state)?
            .code_text_offset_at(panel_rect, Point2D::new(x, y))
    }

    pub(in crate::widget_host) fn apply_code_selection_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(anchor) = self.code_selection_drag.map(|drag| drag.anchor) else {
            return false;
        };
        if let Some(focus) = self.code_text_offset_at_screen(x, y) {
            let next = Some(op_editor_core::codegen::CodeSelection { anchor, focus });
            if self.editor_state.codegen.code_selection != next {
                self.editor_state.codegen.code_selection = next;
                self.mark_dirty();
            }
        }
        true
    }

    fn chat_transcript_text_offset_at_screen(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let chat_rect = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h)?;
        // Selection probe resolves the transcript cache; owner-stamp it so the
        // slot stays tagged with this host's panel (mirrors native).
        match op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
            &self.editor_state,
            self.now_ms,
        )
        .owned_by(self.chat_panel_owner)
        .hit_test(chat_rect, Point2D::new(x, y))
        {
            Some(op_editor_ui::widgets::AIChatHit::SelectTranscriptText(message_index, offset)) => {
                Some((message_index, offset))
            }
            _ => None,
        }
    }

    fn chat_input_text_offset_at_screen(&self, x: f32, y: f32) -> Option<usize> {
        let chat_rect = self.ai_chat_rect(self.last_viewport_w, self.last_viewport_h)?;
        match op_editor_ui::widgets::AIChatPlaceholder::from_editor_at(
            &self.editor_state,
            self.now_ms,
        )
        .owned_by(self.chat_panel_owner)
        .hit_test(chat_rect, Point2D::new(x, y))
        {
            Some(op_editor_ui::widgets::AIChatHit::SelectInputText(offset)) => Some(offset),
            _ => None,
        }
    }

    pub(in crate::widget_host) fn apply_chat_input_selection_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(drag) = self.chat_input_selection_drag else {
            return false;
        };
        if let Some(focus) = self.chat_input_text_offset_at_screen(x, y) {
            if self
                .editor_state
                .chat
                .drag_input_selection(drag.anchor, focus, self.now_ms)
            {
                self.editor_state.chat.focused = true;
                self.mark_dirty();
            }
        }
        true
    }

    pub(in crate::widget_host) fn apply_chat_text_selection_drag_cursor_move(
        &mut self,
        x: f32,
        y: f32,
    ) -> bool {
        let Some(drag) = self.chat_text_selection_drag else {
            return false;
        };
        if let Some((message_index, focus)) = self.chat_transcript_text_offset_at_screen(x, y) {
            if message_index == drag.message_index {
                let next = Some(op_editor_core::chat::ChatTranscriptSelection {
                    message_index,
                    anchor: drag.anchor,
                    focus,
                });
                if self.editor_state.chat.transcript_selection != next {
                    self.editor_state.chat.transcript_selection = next;
                    self.mark_dirty();
                }
            }
        }
        true
    }
}
