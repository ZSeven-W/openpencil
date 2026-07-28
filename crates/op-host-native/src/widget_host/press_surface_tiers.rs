//! `apply_press` tiers 9-11 — the Git panel + AI chat band, the toolbar
//! band, and the LayerPanel drag peek that precedes `apply_click`.
//!
//! The chat sits above the toolbar in paint order, so its rect gets first
//! refusal even where the two overlap. The layer-drag peek deliberately
//! only *seeds* a candidate: promotion to a real reorder drag happens on
//! the first cursor move past the threshold.

use super::press_ctx::PressCtx;
use super::{
    ChatDragState, ChatInputSelectionDragState, ChatResizeState, ChatTextSelectionDragState,
    WidgetHostNative,
};
use op_editor_core::host_press_transitions as core_press;
use op_editor_ui::widgets::chat_click_flow;
use op_editor_ui::widgets::{AIChatHit, AIChatPlaceholder, Toolbar, TOP_BAR_HEIGHT};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// `None` — neither the Git panel nor the chat claimed the press.
    pub(in crate::widget_host) fn press_git_and_chat_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        // 0.9. Floating Git panel — status + interactive actions
        //      (dispatch in `git_press.rs`).
        if self.dispatch_git_panel_press(x, y, viewport_width, viewport_height) {
            self.close_image_popovers_for_higher_overlay();
            return Some(true);
        }

        // 1. AI chat panel — sits on top of the toolbar in paint
        //    order. DragHandle starts a chat drag; other AI hits
        //    defer to apply_click.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
            let panel =
                AIChatPlaceholder::from_editor(&self.editor_state).owned_by(self.chat_panel_owner);
            if let Some(hit) = panel.hit_test(chat_rect, Point2D::new(x, y)) {
                if let AIChatHit::Resize(edge) = hit {
                    self.chat_resize = Some(ChatResizeState {
                        edge,
                        start_x: x,
                        start_y: y,
                        start_rect: chat_rect,
                    });
                    self.editor_state.chat.focused = false;
                    self.mark_dirty();
                    return Some(true);
                }
                if let AIChatHit::SelectInputText(anchor) = hit {
                    self.chat_input_selection_drag = Some(ChatInputSelectionDragState { anchor });
                    chat_click_flow::begin_chat_input_selection(
                        &mut self.editor_state,
                        anchor,
                        self.now_ms,
                    );
                    self.mark_dirty();
                    return Some(true);
                }
                if let AIChatHit::SelectTranscriptText(message_index, anchor) = hit {
                    self.chat_text_selection_drag = Some(ChatTextSelectionDragState {
                        message_index,
                        anchor,
                    });
                    chat_click_flow::begin_chat_transcript_selection(
                        &mut self.editor_state,
                        message_index,
                        anchor,
                    );
                    self.mark_dirty();
                    return Some(true);
                }
                if matches!(hit, AIChatHit::DragHandle) {
                    self.chat_drag = Some(ChatDragState {
                        grab_dx: x - chat_rect.origin.x,
                        grab_dy: y - chat_rect.origin.y,
                        pos_x: chat_rect.origin.x,
                        pos_y: chat_rect.origin.y,
                    });
                    self.editor_state.chat.focused = false;
                    self.mark_dirty();
                    return Some(true);
                }
                let _ = self.apply_click(x, y, viewport_width, viewport_height);
                return Some(true);
            }
        }
        None
    }

    /// Toolbar + floating selection/align toolbar.
    /// `None` — neither claimed the press.
    pub(in crate::widget_host) fn press_toolbar_tiers(&mut self, ctx: &PressCtx) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        let rename_committed = ctx.rename_committed;
        let text_edit_committed = ctx.text_edit_committed;
        // 2. Toolbar — second-highest overlay.
        let toolbar_rect = self.toolbar_rect(viewport_width, viewport_height);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        if (toolbar_rect).contains(Point2D::new(x, y)) {
            if let Some(hit) = toolbar.hit_test(toolbar_rect, Point2D::new(x, y)) {
                match hit {
                    op_editor_ui::widgets::ToolbarHit::Tool(tool) => {
                        self.apply_set_tool(tool);
                        core_press::close_shape_picker(&mut self.editor_state.editor_ui);
                        return Some(true);
                    }
                    op_editor_ui::widgets::ToolbarHit::Action(action) => {
                        core_press::close_shape_picker(&mut self.editor_state.editor_ui);
                        let acted = self.dispatch_toolbar_action(action);
                        return Some(acted || rename_committed || text_edit_committed);
                    }
                    op_editor_ui::widgets::ToolbarHit::ToggleShapePicker => {
                        core_press::toggle_shape_picker(&mut self.editor_state.editor_ui);
                        self.mark_dirty();
                        return Some(true);
                    }
                }
            }
            // Toolbar padding / gaps eat the click — blank press.
            let blurred = self.blur_text_inputs_on_blank_press();
            return Some(blurred || rename_committed || text_edit_committed);
        }

        if let Some(hit) = self.selection_toolbar_hit(x, y, viewport_width, viewport_height) {
            match hit {
                op_editor_ui::widgets::AlignToolbarHit::Align(action) => {
                    if !self.collab_allows_document_mutation(
                        op_editor_core::CollabDocumentMutation::NodePropertyBatch,
                    ) {
                        return Some(true);
                    }
                    self.editor_state.align_selected(action);
                    self.mark_dirty();
                }
                op_editor_ui::widgets::AlignToolbarHit::Boolean(op) => {
                    let _ = self.apply_boolean_op(op);
                }
            }
            return Some(true);
        }
        None
    }

    /// Seed a LayerPanel drag candidate, then run `apply_click`.
    /// `None` — `apply_click` did not consume the press.
    pub(in crate::widget_host) fn press_layer_and_click_tiers(
        &mut self,
        ctx: &PressCtx,
    ) -> Option<bool> {
        let (x, y) = (ctx.x, ctx.y);
        let viewport_width = ctx.viewport_width;
        let viewport_height = ctx.viewport_height;
        // 3. apply_click — LayerPanel + chat-defocus. Peek the
        //    LayerPanel hit-test for a drag-to-reorder candidate.
        if self.editor_state.editor_ui.sidebar_open {
            use op_editor_ui::widgets::LayerPanelHit;
            let layer_rect = Rect {
                origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
                size: Point2D::new(
                    self.editor_state.editor_ui.layer_panel_width,
                    (viewport_height - TOP_BAR_HEIGHT).max(0.0),
                ),
            };
            let panel = self.layer_panel();
            if let Some(LayerPanelHit::Layer(node_id)) =
                panel.hit_test(layer_rect, Point2D::new(x, y))
            {
                self.layer_drag = Some(crate::widget_host::LayerDragState {
                    source: node_id,
                    start_y: y,
                    current_x: x,
                    current_y: y,
                    active: false,
                });
            }
        }
        let consumed = self.apply_click(x, y, viewport_width, viewport_height);
        if consumed {
            return Some(true);
        }
        None
    }
}
