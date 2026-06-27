//! Hit-testing + resize-edge geometry for the AI chat panel — split
//! out of `ai_chat_panel.rs` to keep that file under the 800-line cap.
//! Pure geometry; the painting half stays in `ai_chat_panel.rs`.

use super::ai_chat_panel::{AIChatPlaceholder, HEADER_HEIGHT, PAD, RESIZE_CORNER, RESIZE_GUTTER};
use crate::widgets::ai_chat_checklist::{
    checklist_item_chevron_rect, checklist_item_height, fixed_checklist_height,
    fixed_checklist_items, fixed_checklist_list_rect, fixed_checklist_max_scroll,
    fixed_checklist_rect, HEADER_H, ITEM_GAP, PROGRESS_H,
};
use crate::widgets::ai_chat_hit::{AIChatHit, ChatResizeEdge};
use crate::widgets::ai_chat_panel_controls::attachment_row_hit;
use crate::widgets::ai_chat_panel_header::{
    tab_hit_at, tab_row_rects, MAXIMIZE_GAP, MAXIMIZE_W, NEW_CHAT_D,
};
use crate::widgets::ai_chat_panel_paint::example_card_rects;
use crate::{Point2D, Rect};

impl<'a> AIChatPlaceholder<'a> {
    pub fn fixed_checklist_bounds(&self, rect: Rect) -> Option<Rect> {
        let checklist_h = fixed_checklist_height(self.state, self.state.checklist_collapsed);
        (checklist_h > 0.0)
            .then(|| fixed_checklist_rect(rect, self.input_height_for_rect(rect), checklist_h))
    }

    pub fn fixed_checklist_scroll_max(&self) -> f32 {
        fixed_checklist_max_scroll(self.state, self.state.checklist_collapsed)
    }

    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<AIChatHit> {
        if let Some(edge) = self.resize_edge_at(rect, point) {
            return Some(AIChatHit::Resize(edge));
        }
        if !(rect).contains(point) {
            return None;
        }
        // When collapsed: anywhere on the pill expands it. Drag
        // is only available in expanded mode (by-design — the
        // pill is too small to reliably distinguish drag intent
        // from click intent, so we treat any pill click as the
        // single intended action: re-open).
        if self.state.collapsed {
            return Some(AIChatHit::ToggleCollapse);
        }
        let can_use_model = !self.state.available_models.is_empty();
        // Expanded: chevron + "New Chat" title group toggles collapse.
        if (self.expanded_header_title_rect(rect)).contains(point) {
            return Some(AIChatHit::ToggleCollapse);
        }
        // Hit rects must mirror the paint geometry exactly.
        // Constants live in `ai_chat_panel_header` (imported above):
        //   NEW_CHAT_D = 28, MAXIMIZE_GAP = 6, MAXIMIZE_W = 18, HEADER_HEIGHT = 36
        let right_edge = rect.origin.x + rect.size.x - PAD;
        let header_icon_y = rect.origin.y + (HEADER_HEIGHT - MAXIMIZE_W) / 2.0;
        // New-chat circle (far right).
        let new_chat_rect = Rect {
            origin: Point2D::new(
                right_edge - NEW_CHAT_D,
                rect.origin.y + (HEADER_HEIGHT - NEW_CHAT_D) / 2.0,
            ),
            size: Point2D::new(NEW_CHAT_D, NEW_CHAT_D),
        };
        if (new_chat_rect).contains(point) {
            return Some(AIChatHit::NewChat);
        }
        // Maximize / minimize icon (just left of new-chat).
        let maximize_rect = Rect {
            origin: Point2D::new(
                right_edge - NEW_CHAT_D - MAXIMIZE_GAP - MAXIMIZE_W,
                header_icon_y,
            ),
            size: Point2D::new(MAXIMIZE_W, MAXIMIZE_W),
        };
        if (maximize_rect).contains(point) {
            return Some(AIChatHit::ToggleMaximize);
        }
        // Tab row — between chevron and maximize button.
        // Returns SwitchTab(i) for a tab body click; CloseTab(i) for the × glyph.
        let tab_count = self.tabs_snapshot.len();
        if tab_count > 0 {
            if let Some((tab_idx, over_close)) = tab_hit_at(rect, tab_count, point, self.tab_hover)
            {
                return Some(if over_close {
                    AIChatHit::CloseTab(tab_idx)
                } else {
                    AIChatHit::SwitchTab(tab_idx)
                });
            }
        }
        // Must match `paint` exactly: paint draws the separator at
        // `bottom - input_h` and the input block one pixel below it
        // (`sep_y + 1`). An earlier `- PAD` here put the hit targets
        // ~17 px above where they are painted.
        let input_rect = self.input_rect(rect);
        let input_area_h = self.input_area_height_for_rect(rect);
        // Model-picker dropdown — an overlay above the chip. When
        // open it behaves modally: a row click selects, any other
        // click dismisses it. Hit-tested before the input so a row
        // click isn't eaten by the message list beneath.
        if self.model_picker.open {
            let picker = self.model_picker_rect(rect, input_rect);
            if crate::widgets::ai_chat_model_picker::search_clear_hit(
                picker,
                point,
                self.model_picker_input.text(),
            ) {
                return Some(AIChatHit::ClearModelSearch);
            }
            match crate::widgets::ai_chat_model_picker::model_picker_hit(
                self.model_picker,
                picker,
                point,
                &self.state.available_models,
                self.model_picker_input.text(),
            ) {
                jian_widgets::components::select::SelectHit::Row(idx) => {
                    return Some(AIChatHit::SelectModel(idx));
                }
                jian_widgets::components::select::SelectHit::Inside => {
                    return Some(AIChatHit::FocusModelSearch);
                }
                jian_widgets::components::select::SelectHit::Outside => {}
            }
            return Some(AIChatHit::ToggleModelPicker);
        }
        // Parallel-agents picker — when open, behaves modally: a row click
        // sets the multiplier; any other click closes the picker.
        if self.parallel_agents_picker_open {
            let footer = self.footer_layout(rect, input_rect, {
                let attach_h = self.attachment_row_h();
                input_rect.origin.y + self.input_area_height_for_rect(rect) + attach_h
            });
            if let Some(picker) = self.parallel_agents_picker_rect(rect, &footer) {
                if picker.contains(point) {
                    // Hit inside picker — check which row.
                    let rows_top = picker.origin.y + 32.0;
                    for i in 1..=crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_COUNT {
                        let row_y = rows_top
                            + (i - 1) as f32
                                * crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_ROW_H_PUB;
                        if point.y >= row_y
                            && point.y < row_y + crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_ROW_H_PUB
                        {
                            return Some(AIChatHit::SetParallelAgents(i));
                        }
                    }
                    return Some(AIChatHit::Inside);
                }
            }
            // Click outside the picker — close it.
            return Some(AIChatHit::ToggleParallelAgentsPicker);
        }
        if (input_rect).contains(point) {
            let attach_top = input_rect.origin.y + input_area_h;
            let attach_h = self.attachment_row_h();
            let toolbar_top = attach_top + attach_h;
            if point.y < attach_top {
                if self.is_streaming() {
                    return Some(AIChatHit::Inside);
                }
                if self.state.input.text().is_empty() {
                    return Some(AIChatHit::FocusInput);
                }
                let text_area = Rect {
                    origin: input_rect.origin,
                    size: Point2D::new(input_rect.size.x, input_area_h),
                };
                let offset = crate::widgets::ai_chat_input_text::input_text_offset_at(
                    &self.state.input,
                    text_area,
                    point,
                )
                .unwrap_or(self.state.input.text().len());
                return Some(AIChatHit::SelectInputText(offset));
            }
            // Staged-attachment strip — present only when attachments
            // are staged; a chip click removes that attachment.
            if attach_h > 0.0 && point.y >= attach_top && point.y < toolbar_top {
                let row = Rect {
                    origin: Point2D::new(input_rect.origin.x, attach_top),
                    size: Point2D::new(input_rect.size.x, attach_h),
                };
                if let Some(hit) =
                    attachment_row_hit(row, point, self.state.pending_attachments.len())
                {
                    return Some(hit);
                }
                return Some(AIChatHit::FocusInput);
            }
            // Bottom toolbar strip (#27 layout):
            //   model pill | ⚡ speed chip | 📎 attach | 🎨 palette | [gap] | ◻ stop | ↑ send
            if point.y >= toolbar_top {
                let footer = self.footer_layout(rect, input_rect, toolbar_top);
                let streaming = self.is_streaming();
                if (footer.model).contains(point) {
                    return Some(if can_use_model {
                        AIChatHit::ToggleModelPicker
                    } else {
                        AIChatHit::FocusInput
                    });
                }
                if (footer.speed).contains(point) {
                    // The ⚡ chip is now the Parallel Agents chip (#32).
                    // While streaming the chip is inert (parity with old effort chip).
                    return Some(if streaming {
                        AIChatHit::Inside
                    } else {
                        AIChatHit::ToggleParallelAgentsPicker
                    });
                }
                // agent_team is zero-width — contains() never fires; kept for
                // schema compat.
                if (footer.agent_team).contains(point) {
                    return Some(AIChatHit::CycleAgentTeam);
                }
                if (footer.attach).contains(point) {
                    return Some(if streaming {
                        AIChatHit::Inside
                    } else {
                        AIChatHit::AddAttachment
                    });
                }
                // Palette is inert in #27 — consume the click so canvas is not affected.
                if (footer.palette).contains(point) {
                    return Some(AIChatHit::Inside);
                }
                // Stop circle — only a live target while streaming.
                if streaming && (footer.stop).contains(point) {
                    return Some(AIChatHit::Stop);
                }
                if (footer.send).contains(point) {
                    return Some(
                        if can_use_model
                            && !streaming
                            && (!self.state.input.text().trim().is_empty()
                                || !self.state.pending_attachments.is_empty())
                        {
                            AIChatHit::Send
                        } else {
                            AIChatHit::FocusInput
                        },
                    );
                }
            }
            return Some(if self.is_streaming() {
                AIChatHit::Inside
            } else {
                AIChatHit::FocusInput
            });
        }
        let checklist_h = fixed_checklist_height(self.state, self.state.checklist_collapsed);
        if checklist_h > 0.0 {
            let input_h = self.input_height_for_rect(rect);
            let checklist = fixed_checklist_rect(rect, input_h, checklist_h);
            if (checklist).contains(point) {
                let header = Rect::xywh(
                    checklist.origin.x,
                    checklist.origin.y + PROGRESS_H,
                    checklist.size.x,
                    HEADER_H,
                );
                if (header).contains(point) {
                    return Some(AIChatHit::ToggleChecklist);
                }
                if !self.state.checklist_collapsed {
                    let list = fixed_checklist_list_rect(checklist);
                    let items = fixed_checklist_items(self.state);
                    let scroll = self
                        .fixed_checklist_scroll_max()
                        .min(self.state.checklist_scroll.offset.max(0.0));
                    let mut row_y = list.origin.y - scroll;
                    for (idx, item) in items.iter().enumerate() {
                        let h = checklist_item_height(item);
                        if !item.details.is_empty() {
                            let chevron = checklist_item_chevron_rect(
                                list.origin.x + PAD,
                                row_y,
                                list.size.x - PAD * 2.0,
                            );
                            if (chevron).contains(point) {
                                return Some(AIChatHit::ToggleChecklistItem(idx));
                            }
                        }
                        row_y += h + ITEM_GAP;
                    }
                }
                return Some(AIChatHit::FocusInput);
            }
        }
        // Transcript hit-test — a click on a message's thinking /
        // tool-call collapsible header toggles it. Checked before the
        // drag-handle fallback so the headers are interactive.
        if !self.state.messages.is_empty() {
            let body = self.body_rect(rect);
            let scroll_offset = crate::widgets::ai_chat_transcript::transcript_effective_offset(
                &self.state.messages,
                body,
                self.locale,
                self.state.transcript_scroll.offset,
                self.state.transcript_pinned,
            );
            if let Some(hit) = crate::widgets::ai_chat_transcript::transcript_text_offset_at(
                &self.state.messages,
                body,
                point,
                self.locale,
                scroll_offset,
            ) {
                return Some(AIChatHit::SelectTranscriptText(
                    hit.message_index,
                    hit.offset,
                ));
            }
            if let Some(hit) = crate::widgets::ai_chat_transcript::transcript_hit(
                &self.state.messages,
                body,
                point.x,
                point.y,
                self.locale,
                scroll_offset,
            ) {
                return Some(hit.into());
            }
        }
        if self.state.messages.is_empty() && !self.is_streaming() {
            // Examples grid hit-test (only rendered when no messages).
            // Clickable regardless of model connection — clicking an example
            // fills the input (sending separately requires a model) (#43).
            for (index, (card, ex)) in example_card_rects(rect)
                .iter()
                .zip(self.examples.iter())
                .enumerate()
            {
                if (*card).contains(point) {
                    return Some(AIChatHit::Example {
                        index,
                        prompt: ex.prompt.clone(),
                    });
                }
            }
        }
        if self.state.maximized {
            return Some(AIChatHit::FocusInput);
        }
        Some(AIChatHit::DragHandle)
    }

    pub fn resize_edge_at(&self, rect: Rect, point: Point2D) -> Option<ChatResizeEdge> {
        if self.state.collapsed || self.state.maximized {
            return None;
        }
        let left = rect.origin.x;
        let right = rect.origin.x + rect.size.x;
        let top = rect.origin.y;
        let bottom = rect.origin.y + rect.size.y;
        let outer = Rect::xywh(
            left - RESIZE_GUTTER,
            top - RESIZE_GUTTER,
            rect.size.x + RESIZE_GUTTER * 2.0,
            rect.size.y + RESIZE_GUTTER * 2.0,
        );
        if !(outer).contains(point) {
            return None;
        }

        let near_top = (point.y - top).abs() <= RESIZE_GUTTER;
        let near_bottom = (point.y - bottom).abs() <= RESIZE_GUTTER;
        let near_left = (point.x - left).abs() <= RESIZE_GUTTER;
        let near_right = (point.x - right).abs() <= RESIZE_GUTTER;
        let in_left_corner = point.x <= left + RESIZE_CORNER;
        let in_right_corner = point.x >= right - RESIZE_CORNER;
        let in_top_corner = point.y <= top + RESIZE_CORNER;
        let in_bottom_corner = point.y >= bottom - RESIZE_CORNER;

        match (
            near_top && in_left_corner,
            near_top && in_right_corner,
            near_bottom && in_left_corner,
            near_bottom && in_right_corner,
        ) {
            (true, _, _, _) => return Some(ChatResizeEdge::Nw),
            (_, true, _, _) => return Some(ChatResizeEdge::Ne),
            (_, _, true, _) => return Some(ChatResizeEdge::Sw),
            (_, _, _, true) => return Some(ChatResizeEdge::Se),
            _ => {}
        }
        if near_top {
            Some(ChatResizeEdge::N)
        } else if near_bottom {
            Some(ChatResizeEdge::S)
        } else if near_left && !in_top_corner && !in_bottom_corner {
            Some(ChatResizeEdge::W)
        } else if near_right && !in_top_corner && !in_bottom_corner {
            Some(ChatResizeEdge::E)
        } else {
            None
        }
    }

    pub fn design_block_hover_at(&self, rect: Rect, point: Point2D) -> Option<(usize, usize)> {
        if self.state.messages.is_empty() {
            return None;
        }
        let body = self.body_rect(rect);
        let scroll_offset = crate::widgets::ai_chat_transcript::transcript_effective_offset(
            &self.state.messages,
            body,
            self.locale,
            self.state.transcript_scroll.offset,
            self.state.transcript_pinned,
        );
        crate::widgets::ai_chat_transcript_hit::design_block_at(
            &self.state.messages,
            body,
            point.x,
            point.y,
            self.locale,
            scroll_offset,
        )
    }

    pub fn footer_hover_at(
        &self,
        rect: Rect,
        point: Point2D,
    ) -> Option<op_editor_core::ChatFooterButton> {
        if self.state.collapsed || self.model_picker.open {
            return None;
        }
        let input_rect = self.input_rect(rect);
        if !(input_rect).contains(point) {
            return None;
        }
        let attach_h = self.attachment_row_h();
        let toolbar_top = input_rect.origin.y + self.input_area_height_for_rect(rect) + attach_h;
        if point.y < toolbar_top {
            return None;
        }
        let footer = self.footer_layout(rect, input_rect, toolbar_top);
        let streaming = self.is_streaming();
        if !self.state.available_models.is_empty() && (footer.model).contains(point) {
            return Some(op_editor_core::ChatFooterButton::ModelPicker);
        }
        if (footer.speed).contains(point) {
            return Some(op_editor_core::ChatFooterButton::SpeedChip);
        }
        // agent_team zero-width — contains() never fires, kept for future use.
        if (footer.agent_team).contains(point) {
            return Some(op_editor_core::ChatFooterButton::AgentTeam);
        }
        if !streaming && (footer.attach).contains(point) {
            return Some(op_editor_core::ChatFooterButton::AddAttachment);
        }
        if (footer.palette).contains(point) {
            return Some(op_editor_core::ChatFooterButton::Palette);
        }
        if streaming && (footer.stop).contains(point) {
            return Some(op_editor_core::ChatFooterButton::Stop);
        }
        if (footer.send).contains(point) {
            // #42: stop shares this slot and is matched above while streaming, so
            // reaching here means we're idle — the circle is the Send button.
            return if !self.state.available_models.is_empty() {
                Some(op_editor_core::ChatFooterButton::Send)
            } else {
                None
            };
        }
        None
    }

    pub fn example_hover_at(&self, rect: Rect, point: Point2D) -> Option<usize> {
        // Examples are hoverable/clickable regardless of model connection (#43);
        // gate only on messages-empty / not-streaming / not-collapsed.
        if !self.state.messages.is_empty() || self.is_streaming() || self.state.collapsed {
            return None;
        }
        example_card_rects(rect)
            .iter()
            .position(|card| (*card).contains(point))
    }

    /// Return the index of the tab the cursor is over (for the host to
    /// store in `EditorUiState.chat_tab_hover`). Returns `None` when the
    /// panel is collapsed or the cursor is not in the tab row zone.
    pub fn tab_hover_at(&self, rect: Rect, point: Point2D) -> Option<usize> {
        if self.state.collapsed {
            return None;
        }
        let tab_count = self.tabs_snapshot.len();
        if tab_count == 0 {
            return None;
        }
        // Iterate tab rects and check containment — same layout as `tab_hit_at`
        // but ignores the × sub-rect (hover is per-tab-body, not sub-element).
        let rects = tab_row_rects(rect, tab_count);
        rects.iter().position(|tr| tr.body.contains(point))
    }

    /// Return which row (1–6) of the open Parallel Agents picker the cursor
    /// is over. Returns `None` when the picker is closed or the cursor is
    /// outside the picker rect. Used by the host to update
    /// `EditorUiState::parallel_agents_picker_hover`.
    pub fn parallel_agents_picker_hover_at(&self, rect: Rect, point: Point2D) -> Option<u32> {
        if !self.parallel_agents_picker_open || self.state.collapsed {
            return None;
        }
        let input_rect = self.input_rect(rect);
        let attach_h = self.attachment_row_h();
        let toolbar_top = input_rect.origin.y + self.input_area_height_for_rect(rect) + attach_h;
        let footer = self.footer_layout(rect, input_rect, toolbar_top);
        let picker = crate::widgets::ai_chat_panel_footer::parallel_agents_picker_rect(&footer);
        if !picker.contains(point) {
            return None;
        }
        let rows_top = picker.origin.y + 32.0;
        for i in 1..=crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_COUNT {
            let row_y = rows_top
                + (i - 1) as f32 * crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_ROW_H_PUB;
            if point.y >= row_y
                && point.y < row_y + crate::widgets::ai_chat_panel_footer::PARALLEL_AGENTS_ROW_H_PUB
            {
                return Some(i);
            }
        }
        None
    }
}
