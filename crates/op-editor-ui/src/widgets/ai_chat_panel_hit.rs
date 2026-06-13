//! Hit-testing + resize-edge geometry for the AI chat panel — split
//! out of `ai_chat_panel.rs` to keep that file under the 800-line cap.
//! Pure geometry; the painting half stays in `ai_chat_panel.rs`.

use super::ai_chat_panel::{AIChatPlaceholder, PAD, RESIZE_CORNER, RESIZE_GUTTER};
use crate::widgets::ai_chat_checklist::{
    fixed_checklist_height, fixed_checklist_max_scroll, fixed_checklist_rect, HEADER_H, PROGRESS_H,
};
use crate::widgets::ai_chat_hit::{AIChatHit, ChatResizeEdge};
use crate::widgets::ai_chat_panel_controls::attachment_row_hit;
use crate::widgets::ai_chat_panel_paint::example_card_rects;
use crate::{Point2D, Rect};

impl<'a> AIChatPlaceholder<'a> {
    pub fn fixed_checklist_bounds(&self, rect: Rect) -> Option<Rect> {
        let checklist_h =
            fixed_checklist_height(&self.state.messages, self.state.checklist_collapsed);
        (checklist_h > 0.0)
            .then(|| fixed_checklist_rect(rect, self.input_height_for_rect(rect), checklist_h))
    }

    pub fn fixed_checklist_scroll_max(&self) -> f32 {
        fixed_checklist_max_scroll(&self.state.messages, self.state.checklist_collapsed)
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
        let header_y = rect.origin.y + 8.0;
        let maximize_rect = Rect {
            origin: Point2D::new(rect.origin.x + rect.size.x - PAD - 50.0, header_y),
            size: Point2D::new(22.0, 22.0),
        };
        if (maximize_rect).contains(point) {
            return Some(AIChatHit::ToggleMaximize);
        }
        let new_chat_rect = Rect {
            origin: Point2D::new(rect.origin.x + rect.size.x - PAD - 22.0, header_y),
            size: Point2D::new(22.0, 22.0),
        };
        if (new_chat_rect).contains(point) {
            return Some(AIChatHit::NewChat);
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
            // Bottom toolbar strip — model picker and Agent Team chip
            // on the left, with attach + send icon buttons on the right.
            if point.y >= toolbar_top {
                let footer = self.footer_layout(rect, input_rect, toolbar_top);
                if (footer.model).contains(point) {
                    return Some(if can_use_model {
                        AIChatHit::ToggleModelPicker
                    } else {
                        AIChatHit::FocusInput
                    });
                }
                if (footer.agent_team).contains(point) {
                    return Some(AIChatHit::CycleAgentTeam);
                }
                if (footer.attach).contains(point) {
                    return Some(if self.is_streaming() {
                        AIChatHit::Inside
                    } else {
                        AIChatHit::AddAttachment
                    });
                }
                if (footer.send).contains(point) {
                    return Some(if self.is_streaming() {
                        AIChatHit::Stop
                    } else if can_use_model
                        && (!self.state.input.text().trim().is_empty()
                            || !self.state.pending_attachments.is_empty())
                    {
                        AIChatHit::Send
                    } else {
                        AIChatHit::FocusInput
                    });
                }
            }
            return Some(if self.is_streaming() {
                AIChatHit::Inside
            } else {
                AIChatHit::FocusInput
            });
        }
        let checklist_h =
            fixed_checklist_height(&self.state.messages, self.state.checklist_collapsed);
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
                return Some(AIChatHit::FocusInput);
            }
        }
        // Transcript hit-test — a click on a message's thinking /
        // tool-call collapsible header toggles it. Checked before the
        // drag-handle fallback so the headers are interactive.
        if !self.state.messages.is_empty() {
            if let Some(hit) = crate::widgets::ai_chat_transcript::transcript_text_offset_at(
                &self.state.messages,
                self.body_rect(rect),
                point,
                self.locale,
            ) {
                return Some(AIChatHit::SelectTranscriptText(
                    hit.message_index,
                    hit.offset,
                ));
            }
            if let Some(hit) = crate::widgets::ai_chat_transcript::transcript_hit(
                &self.state.messages,
                self.body_rect(rect),
                point.x,
                point.y,
                self.locale,
            ) {
                return Some(hit.into());
            }
        }
        if self.state.messages.is_empty() && can_use_model && !self.is_streaming() {
            // Examples grid hit-test (only rendered when no messages).
            for (card, ex) in example_card_rects(rect).iter().zip(self.examples.iter()) {
                if (*card).contains(point) {
                    return Some(AIChatHit::Example(ex.prompt.clone()));
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
        crate::widgets::ai_chat_transcript_hit::design_block_at(
            &self.state.messages,
            self.body_rect(rect),
            point.x,
            point.y,
            self.locale,
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
        if !self.state.available_models.is_empty() && (footer.model).contains(point) {
            return Some(op_editor_core::ChatFooterButton::ModelPicker);
        }
        if (footer.agent_team).contains(point) {
            return Some(op_editor_core::ChatFooterButton::AgentTeam);
        }
        if !self.is_streaming() && (footer.attach).contains(point) {
            return Some(op_editor_core::ChatFooterButton::AddAttachment);
        }
        if (footer.send).contains(point) {
            return Some(if self.is_streaming() {
                op_editor_core::ChatFooterButton::Stop
            } else if !self.state.available_models.is_empty() {
                op_editor_core::ChatFooterButton::Send
            } else {
                return None;
            });
        }
        None
    }

    pub fn example_hover_at(&self, rect: Rect, point: Point2D) -> Option<usize> {
        if !self.state.messages.is_empty()
            || self.state.available_models.is_empty()
            || self.is_streaming()
            || self.state.collapsed
        {
            return None;
        }
        example_card_rects(rect)
            .iter()
            .position(|card| (*card).contains(point))
    }
}
