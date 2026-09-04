//! `impl Widget for AIChatPlaceholder` — the panel's full composition
//! pass (collapsed pill, header, transcript, empty-state examples,
//! input block, bottom toolbar, overlays) plus its accessibility node.
//! Carved off `ai_chat_panel.rs` to keep every file under the 800-line
//! cap; layout + geometry stay on the spine.

use super::*;

impl<'a> Widget for AIChatPlaceholder<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(self.state.panel_width, self.state.panel_height),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        // Minimized: the compact input bar replaces the whole panel.
        if self.state.is_minimized() {
            crate::widgets::ai_chat_panel_minimized::paint_minimized_bar(
                cx,
                &self.theme,
                self,
                rect,
            );
            return;
        }

        paint_panel_surface(cx, &self.theme, rect);
        let can_use_model = !self.state.available_models.is_empty();
        let input_h = self.input_height_for_rect(rect);
        let sep_y = rect.origin.y + rect.size.y - input_h;
        paint_panel_body_chrome(cx, &self.theme, rect, sep_y);

        // Expanded header (MT.2 tab-row restyle):
        //   [chevron ⌄] [tab 0] [tab 1 (active + spinner)] … [maximize] [new-chat ⊕]
        use op_editor_core::ChatHeaderButton;
        // Vertical center of all header icons (18px icons in 36px header).
        let header_icon_y = rect.origin.y + (HEADER_HEIGHT - 18.0) / 2.0;
        let right_edge = rect.origin.x + rect.size.x - PAD;
        let chevron_x = rect.origin.x + PAD;
        let tokens = crate::widgets::button::tokens_from_theme(&self.theme);

        // --- Collapse chevron (far left) ---
        // Rendered through IconButton (like the maximize button beside the
        // new-chat "+") so hovering/pressing it shows the same ghost
        // button-hover wash instead of only swapping the icon color (#41).
        jian_widgets::components::icon_button::IconButton {
            icon_paths: Icon::ChevronDown.paths(),
            hovered: self.header_hover == Some(ChatHeaderButton::ToggleCollapse),
            pressed: self.header_pressed == Some(ChatHeaderButton::ToggleCollapse),
            active: false,
            enabled: true,
            icon_size: 18.0,
            stroke_width: 1.4,
        }
        .paint(
            cx.backend,
            Rect {
                origin: Point2D::new(chevron_x, header_icon_y),
                size: Point2D::new(18.0, 18.0),
            },
            &tokens,
        );

        // --- New-chat "+" circular button (far right, 28px circle) ---
        const NEW_CHAT_R: f32 = NEW_CHAT_D / 2.0;
        let new_chat_rect = Rect {
            origin: Point2D::new(
                right_edge - NEW_CHAT_D,
                rect.origin.y + (HEADER_HEIGHT - NEW_CHAT_D) / 2.0,
            ),
            size: Point2D::new(NEW_CHAT_D, NEW_CHAT_D),
        };
        let new_chat_hovered = self.header_hover == Some(ChatHeaderButton::NewChat);
        let new_chat_pressed = self.header_pressed == Some(ChatHeaderButton::NewChat);
        let new_chat_fill = if new_chat_pressed {
            chat_neutral_feedback_color(&self.theme, true)
        } else if new_chat_hovered {
            chat_neutral_feedback_color(&self.theme, false)
        } else {
            self.theme.secondary
        };
        cx.backend
            .fill_round_rect(new_chat_rect, NEW_CHAT_R, new_chat_fill);
        cx.backend
            .stroke_round_rect(new_chat_rect, NEW_CHAT_R, self.theme.border, 1.0);
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(
                new_chat_rect.origin.x + (NEW_CHAT_D - 14.0) / 2.0,
                new_chat_rect.origin.y + (NEW_CHAT_D - 14.0) / 2.0,
            ),
            14.0,
            self.theme.muted_foreground,
            1.4,
        );
        // --- Maximize / minimize icon (just left of new-chat) ---
        let maximize_x = right_edge - NEW_CHAT_D - MAXIMIZE_GAP - MAXIMIZE_W;
        jian_widgets::components::icon_button::IconButton {
            icon_paths: self.maximize_icon().paths(),
            hovered: self.header_hover == Some(ChatHeaderButton::ToggleMaximize),
            pressed: self.header_pressed == Some(ChatHeaderButton::ToggleMaximize),
            active: false,
            enabled: true,
            icon_size: MAXIMIZE_W,
            stroke_width: 1.4,
        }
        .paint(
            cx.backend,
            Rect {
                origin: Point2D::new(maximize_x, header_icon_y),
                size: Point2D::new(MAXIMIZE_W, MAXIMIZE_W),
            },
            &tokens,
        );

        // --- Tab row (between chevron and maximize) ---
        // Replaces the single active-chat pill from 5.3.
        let is_running = self.state.agents_running.0 > 0;
        paint_header_tabs(
            cx,
            &self.theme,
            rect,
            &self.tabs_snapshot,
            self.active_tab_index,
            self.tab_hover,
            is_running,
            self.now_ms,
        );

        // Body — either messages or examples.
        if self.state.messages.is_empty() {
            // The empty-state stack is laid out from the top while the
            // composer block is bottom-anchored; on a keyboard-shrunk
            // compact sheet the two can meet. Clip to the region between
            // them and hand the painter its bottom so pills / tips that
            // don't fit are dropped instead of overlapping the composer.
            let region = self.empty_state_region(rect);
            cx.backend.save();
            cx.backend.clip_rect(region);
            paint_examples(
                cx,
                &self.theme,
                rect,
                &self.label_start_with_ai,
                &self.label_tip_select_elements,
                &self.examples,
                // Examples stay enabled without a connected model (#43) — clicking
                // one fills the input; only streaming disables them.
                self.is_streaming(),
                self.example_hover,
                self.example_pressed,
                region.origin.y + region.size.y,
            );
            cx.backend.restore();
        } else {
            let body = self.body_rect(rect);
            // Fingerprint + resolve the transcript ONCE for the whole paint
            // pass, then thread the build into both the scroll clamp and the
            // painter so neither re-hashes.
            let canonical =
                crate::widgets::ai_chat_transcript_cache::cached_canonical_transcript_owned(
                    self.owner,
                    &self.state.messages,
                    body,
                    self.locale,
                );
            let scroll_offset = crate::widgets::ai_chat_transcript::effective_offset_of(
                &canonical,
                body,
                self.state.transcript_scroll.offset,
                self.state.transcript_pinned,
            );
            crate::widgets::ai_chat_transcript::paint_transcript_with_selection(
                cx,
                &self.theme,
                body,
                &self.state.messages,
                &canonical,
                self.now_ms,
                self.design_hover,
                self.state.transcript_selection,
                scroll_offset,
            );
        }
        let input_block_rect = self.input_rect(rect);
        // Above even the chips: a turn that cannot start at all outranks
        // anything describing what the turn would do.
        let notice_h = self.mcp_notice_row_h();
        if let (Some(row), Some(label)) = (self.mcp_notice_row(rect), self.mcp_notice.as_deref()) {
            crate::widgets::ai_chat_mcp_notice::paint_mcp_notice(cx, row, self.theme, label);
        }
        let rows_rect = Rect {
            origin: Point2D::new(
                input_block_rect.origin.x,
                input_block_rect.origin.y + notice_h,
            ),
            size: Point2D::new(
                input_block_rect.size.x,
                (input_block_rect.size.y - notice_h).max(0.0),
            ),
        };
        // Above everything else in the input block: the chips describe the
        // next turn and what it is aimed at, not the turn's content.
        crate::widgets::ai_chat_chip_row::paint_chip_row(cx, &self.theme, self, rows_rect);
        let chip_row_h = self.chip_row_h();
        let input_rect = Rect {
            origin: Point2D::new(rect.origin.x + PAD, sep_y + 1.0 + notice_h + chip_row_h),
            size: Point2D::new(
                rect.size.x - PAD * 2.0,
                self.input_area_height_for_rect(rect),
            ),
        };
        let input_area_h = input_rect.size.y;
        crate::widgets::ai_chat_input_text::paint_input_text_area(
            cx,
            &self.theme,
            self.state,
            input_rect,
            input_area_h,
            self.now_ms,
            &self.label_input_placeholder,
        );

        // Staged-attachment strip — between the textarea and the
        // controls row, present only when attachments are staged.
        let attach_h = self.attachment_row_h();
        if attach_h > 0.0 {
            let attach_rect = Rect {
                origin: Point2D::new(input_rect.origin.x, input_rect.origin.y + input_area_h),
                size: Point2D::new(input_rect.size.x, attach_h),
            };
            paint_attachment_row(cx, &self.theme, attach_rect, self.state);
        }

        // Bottom toolbar (#27) — single row:
        //   model pill | ⚡ speed chip | 📎 attach | [gap] | ◻ stop | ↑ send
        let toolbar_y = input_rect.origin.y + input_area_h + attach_h;
        let toolbar_center_y = toolbar_y + INPUT_TOOLBAR_HEIGHT / 2.0;
        let footer = self.footer_layout(rect, self.input_rect(rect), toolbar_y);
        let streaming = self.is_streaming();
        let send_active = can_use_model
            && (!self.state.input.text().trim().is_empty()
                || !self.state.pending_attachments.is_empty());
        paint_bottom_toolbar(cx, self, &footer, toolbar_center_y, streaming, send_active);

        // Model-picker dropdown paints above other panel content.
        if let Some(picker) = self.model_picker_bounds(rect) {
            crate::widgets::ai_chat_model_picker::paint_model_picker(
                cx,
                &self.theme,
                picker,
                &self.state.available_models,
                self.state.selected_model,
                self.model_picker,
                self.model_picker_input,
                self.now_ms,
                self.locale,
            );
        }

        // Parallel-agents picker paints last (top-most overlay).
        if self.parallel_agents_picker_open {
            crate::widgets::ai_chat_panel_footer::paint_parallel_agents_picker(
                cx,
                &self.theme,
                &footer,
                self.state.agent_team_size,
                self.parallel_agents_picker_hover,
            );
        }

        // "New Chat Cmd+T" tooltip paints after transcript and overlays so it
        // cannot be covered by message bubbles below the header.
        if new_chat_hovered {
            paint_new_chat_tooltip(cx, &self.theme, rect);
        }

        // Thinking-toggle tooltip — the button is a bare glyph with three
        // states, so the tooltip is where "what does this do, and what is it
        // set to right now" is actually answered. Painted last, above the
        // input block it hangs over, and clamped to the panel so a narrow
        // panel doesn't push the label off its own edge.
        if self.footer_hover == Some(op_editor_core::ChatFooterButton::ThinkingMode)
            && footer.thinking.size.x > 0.0
        {
            let label = op_i18n::translate(
                self.locale,
                crate::widgets::ai_chat_thinking_toggle::thinking_tooltip_key(
                    self.state.thinking_mode,
                ),
            );
            crate::widgets::tooltip::paint_tooltip(
                cx,
                &self.theme,
                footer.thinking,
                label,
                None,
                crate::widgets::tooltip::TooltipPlacement::Above,
                Some((rect.origin.x + 4.0, rect.origin.x + rect.size.x - 4.0)),
            );
        }

        // Pinned-style detail card — last of all, so it hangs over the input
        // block, and anchored ABOVE its chip so the ✕ on that row stays both
        // visible and clickable. Present only once the dwell has elapsed; the
        // resolve that builds it is gated on the same clock.
        if let Some(card) = self.style_card.as_ref() {
            if let Some(chip) = self.chip_row(input_block_rect).style {
                crate::widgets::ai_chat_style_card::paint_style_card(
                    cx,
                    &self.theme,
                    card,
                    chip,
                    rect,
                    self.locale,
                );
            }
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(op_i18n::translate(self.locale, "a11y.aiChat"));
        node
    }
}
