//! Primary-button click routing + marquee / layer-drag commit on
//! `WidgetHostNative`. Split out of `keyboard.rs` to honor the
//! 800-line cap.
//!
//! Widget hit-tests run against `EditorState`; canvas marquee
//! hit-tests query the layout-resolved `LayoutScene`. Resolved-scene
//! node ids are wrapped into op-editor-core `NodeId`s before feeding
//! `EditorState` mutators.

use super::helpers::{TOOLBAR_INSET_X, TOOLBAR_INSET_Y};
use super::WidgetHostNative;
use op_editor_ui::widgets::{
    AIChatHit, AIChatPlaceholder, LayerPanel, LayoutCx, Toolbar, Widget, TOOLBAR_WIDTH,
    TOP_BAR_HEIGHT,
};
use op_editor_ui::{Point2D, Rect};

impl WidgetHostNative {
    /// Layer drag release → reorder_before/after/into.
    pub(in crate::widget_host) fn commit_layer_drag(
        &mut self,
        d: super::LayerDragState,
        viewport_h: f32,
    ) -> bool {
        if !d.active {
            // Never moved past threshold — selection on press is the
            // only effect, nothing more to do.
            return false;
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
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.editor_state.editor_ui.layer_panel_width,
                (viewport_h - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
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
        viewport_w: f32,
        viewport_h: f32,
    ) {
        // 2 screen-px marquee threshold (TS `useMarqueeStart`).
        let screen_dx = (m.current_screen_x - m.start_screen_x).abs();
        let screen_dy = (m.current_screen_y - m.start_screen_y).abs();
        if screen_dx < 2.0 && screen_dy < 2.0 {
            return;
        }
        self.refresh_layout_scene();
        let (cx0, cy0, _cw, _ch) = self.canvas_region(viewport_w, viewport_h);
        let to_doc = |sx: f32, sy: f32| -> Point2D {
            let local = Point2D::new(sx - cx0, sy - cy0);
            self.editor_state.viewport.to_document(local)
        };
        let p0 = to_doc(m.start_screen_x, m.start_screen_y);
        let p1 = to_doc(m.current_screen_x, m.current_screen_y);
        let x = p0.x.min(p1.x);
        let y = p0.y.min(p1.y);
        let w = (p1.x - p0.x).abs();
        let h = (p1.y - p0.y).abs();
        let rect = Rect::xywh(x, y, w, h);
        // `nodes_intersecting_doc_rect` queries the `LayoutScene` —
        // it returns the resolved-scene node id strings.
        let ids = self.layout_scene.nodes_intersecting_doc_rect(rect);
        if m.additive {
            // ADD-only: every hit joins the set; already-selected
            // hits stay selected (TS shift-marquee parity).
            for id in ids {
                let ec_id = op_editor_core::NodeId::new(&id);
                if !self.editor_state.is_selected(&ec_id) {
                    self.editor_state.toggle_selection(ec_id);
                }
            }
            self.mark_dirty();
        } else if !ids.is_empty() {
            // Replace with the hit set; anchor = last hit.
            let ec_ids: Vec<op_editor_core::NodeId> =
                ids.iter().map(op_editor_core::NodeId::new).collect();
            let anchor = ec_ids.last().unwrap().clone();
            self.editor_state.selection.set = ec_ids;
            self.editor_state.selection.anchor = anchor;
            self.mark_dirty();
        }
        // Empty marquee on plain press already cleared at start.
        // A marquee selection landing outside the entered container
        // steps out of it (selection-outside-exits rule).
        self.editor_state.sync_entered_container_with_selection();
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
                        self.editor_state.chat.begin_send();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::Stop => {
                        self.editor_state.chat.stop_streaming();
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::Example { prompt, .. } => {
                        self.editor_state.chat.set_input_text(prompt);
                        self.editor_state.chat.focus_input_at_end(self.now_ms);
                        self.editor_state.chat.transcript_selection = None;
                        self.mark_dirty();
                        return true;
                    }
                    AIChatHit::DragHandle => {
                        // Drag handle handled in apply_press ahead of
                        // this; reaching here is a path bypass.
                        return false;
                    }
                    AIChatHit::Resize(_) => {
                        // Resize handles are press-drag only.
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
                        self.editor_state.editor_ui.chat_model_picker.pressed = Some(idx);
                        self.editor_state.editor_ui.chat_model_picker.hover = Some(idx);
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
                        // The desktop event loop drains this flag,
                        // opens a native file picker, and stages the
                        // chosen file via `ChatState::add_attachment`.
                        self.editor_state.chat.pending_attachment_pick = true;
                        self.mark_dirty();
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
        let (cx0, _cy0, _cw, _ch) = self.canvas_region(viewport_width, viewport_height);
        let toolbar = Toolbar::for_editor(&self.editor_state);
        let toolbar_h = toolbar
            .layout(&LayoutCx {
                available_width: TOOLBAR_WIDTH,
                dpi: 1.0,
            })
            .rect
            .size
            .y;
        let toolbar_rect = Rect {
            origin: Point2D::new(cx0 + TOOLBAR_INSET_X, TOP_BAR_HEIGHT + TOOLBAR_INSET_Y),
            size: Point2D::new(TOOLBAR_WIDTH, toolbar_h),
        };
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
        // Panel hits only when sidebar is open.
        if !self.editor_state.editor_ui.sidebar_open {
            return was_focused;
        }
        let layer_rect = Rect {
            origin: Point2D::new(0.0, TOP_BAR_HEIGHT),
            size: Point2D::new(
                self.editor_state.editor_ui.layer_panel_width,
                (viewport_height - TOP_BAR_HEIGHT).max(0.0),
            ),
        };
        let panel = LayerPanel::from_editor(&self.editor_state);
        if let Some(hit) = panel.hit_test(layer_rect, Point2D::new(x, y)) {
            use op_editor_core::ui_draft::LayerContextTarget;
            use op_editor_ui::widgets::LayerPanelHit as H;
            // Build the op-editor-core context target for the
            // double-click rename detection.
            let target_for_dbl = match &hit {
                H::Layer(id) => Some(LayerContextTarget::Layer(id.clone())),
                H::Page(idx) => Some(LayerContextTarget::Page(*idx)),
                _ => None,
            };
            if let Some(target) = target_for_dbl {
                if let Some((prev, prev_ms)) = self.editor_state.editor_ui.last_layer_click.clone()
                {
                    if prev == target && self.now_ms.saturating_sub(prev_ms) < 400 {
                        let started = match &target {
                            LayerContextTarget::Layer(id) => {
                                self.editor_state.start_rename_layer(id.clone())
                            }
                            LayerContextTarget::Page(idx) => {
                                self.editor_state.start_rename_page(*idx)
                            }
                        };
                        if started {
                            if let Some(rename) = self.editor_state.ui.layer_rename.as_mut() {
                                rename.input.touch(self.now_ms);
                            }
                        }
                        self.editor_state.editor_ui.last_layer_click = None;
                        self.mark_dirty();
                        return true;
                    }
                }
                self.editor_state.editor_ui.last_layer_click = Some((target, self.now_ms));
            }
            match hit {
                H::Page(idx) => {
                    let _ = self.editor_state.set_active_page(idx);
                    self.editor_state.clear_selection();
                    self.mark_dirty();
                    return true;
                }
                H::Layer(node_id) => {
                    let ec_id = node_id.clone();
                    if self.shift_held {
                        self.editor_state.toggle_selection(ec_id);
                    } else {
                        self.editor_state.set_single_selection(ec_id);
                    }
                    return true;
                }
                H::ToggleHidden(node_id) => {
                    // TS toggleVisibility → mutateWithHistory
                    // (document-store-node-actions.ts:162-174).
                    self.with_doc_history(|s| s.toggle_node_hidden(&node_id.clone()));
                    self.mark_dirty();
                    return true;
                }
                H::ToggleLocked(node_id) => {
                    // TS toggleLock → mutateWithHistory
                    // (document-store-node-actions.ts:176-188).
                    self.with_doc_history(|s| s.toggle_node_locked(&node_id.clone()));
                    self.mark_dirty();
                    return true;
                }
                H::ToggleCollapsed(node_id) => {
                    self.editor_state.toggle_node_collapsed(&node_id.clone());
                    self.mark_dirty();
                    return true;
                }
                H::AddPage => {
                    // TS addPage pushes history before the insert
                    // (document-store-pages.ts:19-49).
                    self.with_doc_history(|s| s.add_page().is_some());
                    self.mark_dirty();
                    return true;
                }
                H::DeletePage(idx) => {
                    // TS removePage pushes history after its
                    // last-page guard (document-store-pages.ts:51-63)
                    // — `with_doc_history` skips the push when the
                    // guard rejects the delete.
                    self.with_doc_history(|s| s.remove_page(idx));
                    self.mark_dirty();
                    return true;
                }
            }
        }
        // Click hit no chrome — repaint if focus changed.
        was_focused
    }
}

fn chat_button_press_target(hit: &AIChatHit) -> Option<op_editor_core::ButtonPressTarget> {
    if let Some(header) = op_editor_ui::widgets::editor_state_ext::chat_header_hover(hit) {
        return Some(op_editor_core::ButtonPressTarget::ChatHeader(header));
    }
    if let AIChatHit::Example { index, .. } = hit {
        return Some(op_editor_core::ButtonPressTarget::ChatExample(*index));
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
