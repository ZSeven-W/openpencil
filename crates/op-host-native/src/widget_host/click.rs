//! Primary-button click routing + marquee / layer-drag commit on
//! `WidgetHostNative`. Split out of `keyboard.rs` to honor the
//! 800-line cap.
//!
//! Widget hit-tests run against `EditorState`; canvas marquee
//! hit-tests query the layout-resolved `LayoutScene`. Resolved-scene
//! node ids are wrapped into op-editor-core `NodeId`s before feeding
//! `EditorState` mutators.
//!
//! The chat-panel and LayerPanel click dispatches themselves are
//! host-independent — they live in `op_editor_ui::widgets::
//! chat_click_flow` / `press_flow` and are shared verbatim with the web
//! host; only the platform tail (`mark_dirty`, transcript-cache owner
//! rotation, viewport fits, chat transport) stays here.

use super::WidgetHostNative;
use op_editor_core::host_press_transitions as core_press;
use op_editor_ui::widgets::chat_click_flow::{self, ChatClickStep, ChatHostAction};
use op_editor_ui::widgets::press_flow::{self, LayerPanelClick};
use op_editor_ui::widgets::{AIChatPlaceholder, Toolbar};
use op_editor_ui::Point2D;

impl WidgetHostNative {
    /// Route a press inside the model picker before lower rail/panel widgets.
    /// The picker can extend outside the chat rect, so checking only the
    /// panel's normal slot lets the underlying Property/Layer panel consume
    /// the visible popup. The action itself stays centralized in
    /// [`Self::apply_click`], which reuses `AIChatPlaceholder::hit_test`.
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

    /// Layer drag release → reorder_before/after/into.
    pub(in crate::widget_host) fn commit_layer_drag(
        &mut self,
        d: super::LayerDragState,
        viewport_w: f32,
        viewport_h: f32,
    ) -> bool {
        if !d.active {
            // Never moved past threshold — selection on press is the
            // only effect, nothing more to do.
            return false;
        }
        // Activation happened on cursor move, but permissions can change
        // before release. The commit sink must fail closed as well.
        if !self.collab_allows_document_mutation(op_editor_core::CollabDocumentMutation::NodeMove) {
            return true;
        }
        self.refresh_layout_scene();
        if self
            .layout_scene
            .active_page()
            .map(|p| p.find(d.source.as_str()).is_none())
            .unwrap_or(true)
        {
            return false;
        }
        use op_editor_ui::widgets::{DropPosition, LayerPanel};
        let layer_rect = self.layers_content_rect(viewport_w, viewport_h);
        // Build with source excluded so indicator y matches post-commit.
        let panel = LayerPanel::from_editor_with_drag_source(&self.editor_state, &d.source);
        let cursor = Point2D::new(d.current_x, d.current_y);
        let Some(drop) = panel.drop_target_at(layer_rect, cursor) else {
            return true;
        };
        if drop.anchor == d.source {
            return true; // self-drop no-op
        }
        let source = d.source.clone();
        let anchor = drop.anchor.clone();
        // TS moveNode wraps the reorder/reparent in mutateWithHistory
        // (document-store-node-actions.ts:106-132) — push only when
        // the mutator actually moved something (a rejected cycle /
        // missing anchor must not pollute the undo stack).
        self.with_doc_history(|s| match drop.position {
            DropPosition::Before => s.reorder_before(source, anchor),
            DropPosition::After => s.reorder_after(source, anchor),
            DropPosition::Into => s.reorder_into(source, anchor),
        });
        self.mark_dirty();
        true
    }

    pub(in crate::widget_host) fn commit_marquee_selection(
        &mut self,
        m: super::MarqueeDragState,
        _viewport_w: f32,
        _viewport_h: f32,
    ) {
        use op_editor_ui::widgets::marquee_flow;
        if !marquee_flow::marquee_dragged(&m) {
            return;
        }
        self.refresh_layout_scene();
        if marquee_flow::commit_marquee_selection(&mut self.editor_state, &self.layout_scene, &m) {
            self.mark_dirty();
        }
    }

    /// Primary-button click — routes to AI chat / Toolbar / Layer.
    pub fn apply_click(
        &mut self,
        x: f32,
        y: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> bool {
        self.refresh_layout_scene();
        // AI chat panel sits above canvas — check first.
        if let Some(chat_rect) = self.ai_chat_rect(viewport_width, viewport_height) {
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
        let toolbar_rect = self.toolbar_rect(viewport_width, viewport_height);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        if !self.editor_state.editor_ui.touch_chrome() {
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
        }
        // Panel hits only when sidebar is open.
        if !self.layers_panel_visible() {
            return was_focused;
        }
        let layer_rect = self.layers_content_rect(viewport_width, viewport_height);
        let panel = self.layer_panel();
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            let mutation = match &hit {
                op_editor_ui::widgets::LayerPanelHit::ToggleHidden(_)
                | op_editor_ui::widgets::LayerPanelHit::ToggleLocked(_) => {
                    Some(op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::VisibilityAndLocking,
                    ))
                }
                op_editor_ui::widgets::LayerPanelHit::AddPage
                | op_editor_ui::widgets::LayerPanelHit::DeletePage(_) => {
                    Some(op_editor_core::CollabDocumentMutation::Unsupported(
                        op_editor_core::CollabUnsupportedFeature::PageStructure,
                    ))
                }
                op_editor_ui::widgets::LayerPanelHit::Page(_)
                | op_editor_ui::widgets::LayerPanelHit::Layer(_)
                | op_editor_ui::widgets::LayerPanelHit::ToggleCollapsed(_) => None,
            };
            if mutation.is_some_and(|mutation| !self.collab_allows_document_mutation(mutation)) {
                return true;
            }
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
                    self.fit_active_page_after_switch(viewport_width, viewport_height);
                    true
                }
                // Native repaints off the consumed press alone (the web
                // host additionally marks dirty here — see its wrapper).
                LayerPanelClick::SelectionChanged => true,
            };
        }
        // Click hit no chrome — repaint if focus changed.
        was_focused
    }

    /// Platform tail for the shared chat-panel click dispatch.
    fn dispatch_chat_click(&mut self, hit: op_editor_ui::widgets::AIChatHit) -> bool {
        match chat_click_flow::apply_chat_hit(&mut self.editor_state, hit, self.now_ms) {
            // Drag handle / resize handles are press-drag only; the
            // press path claimed them before reaching apply_click.
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
                // Rotate the transcript-cache owner NOW so a pre-paint
                // CursorMoved hint can't cross-pair the old tab's
                // geometry with the new tab's messages.
                self.force_rotate_chat_owner();
                self.mark_dirty();
                true
            }
            ChatClickStep::ModelPickerToggled { opening } => {
                if opening {
                    // Opening can be painted before the deferred cursor-move
                    // queue runs. Clear covered hover state now so the first
                    // picker frame never carries a stale canvas/panel wash.
                    self.clear_hover_below_chat_model_picker();
                }
                self.mark_dirty();
                true
            }
            ChatClickStep::Host(ChatHostAction::Send) => {
                self.editor_state.chat.begin_send();
                self.mark_dirty();
                true
            }
            ChatClickStep::Host(ChatHostAction::ApplyDesignBlock {
                message_index,
                text,
            }) => self.apply_chat_design_block(message_index, &text),
            ChatClickStep::Host(ChatHostAction::RetrySubtask {
                message_index,
                source_index,
            }) => {
                // Flips the row to Running + raises `pending_subtask_retry`;
                // the desktop host drains it next frame (see
                // `design_session::launch_subtask_retry_if_pending`). No-ops
                // when the row has no persisted spec.
                self.editor_state
                    .chat
                    .begin_subtask_retry(message_index, source_index);
                self.mark_dirty();
                true
            }
        }
    }
}
