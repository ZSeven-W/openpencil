use crate::theme::Theme;
use crate::widgets::ai_chat_checklist::{
    fixed_checklist_height, fixed_checklist_rect, paint_fixed_checklist,
};
use crate::widgets::ai_chat_panel_controls::{paint_attachment_row, ATTACHMENT_ROW_HEIGHT};
use crate::widgets::ai_chat_panel_paint::{
    paint_examples, paint_panel_body_chrome, paint_panel_surface,
};
use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::chat::ChatState;
use op_editor_core::EditorState;

pub const AI_CHAT_WIDTH: f32 = op_editor_core::chat::DEFAULT_CHAT_PANEL_WIDTH;
pub const AI_CHAT_HEIGHT: f32 = op_editor_core::chat::DEFAULT_CHAT_PANEL_HEIGHT;
pub const AI_CHAT_MIN_WIDTH: f32 = 280.0;
pub const AI_CHAT_MIN_HEIGHT: f32 = 250.0;
pub const AI_CHAT_MAX_RATIO: f32 = 0.8;
pub const AI_CHAT_COLLAPSED_WIDTH: f32 = 150.0;
pub const AI_CHAT_COLLAPSED_HEIGHT: f32 = 32.0;
pub(crate) const PAD: f32 = 16.0;
pub(crate) const HEADER_HEIGHT: f32 = 36.0;
const COLLAPSED_RADIUS: f32 = 8.0;
const COLLAPSED_X_PAD: f32 = 12.0;
const COLLAPSED_GAP: f32 = 6.0;
const COLLAPSED_MESSAGE_ICON: f32 = 13.0;
const COLLAPSED_CHEVRON_ICON: f32 = 12.0;
pub(crate) const RESIZE_GUTTER: f32 = 4.0;
pub(crate) const RESIZE_CORNER: f32 = 12.0;
pub(crate) const INPUT_AREA_HEIGHT: f32 = 56.0;
const INPUT_TOOLBAR_HEIGHT: f32 = 40.0;
const INPUT_BASE_HEIGHT: f32 = INPUT_AREA_HEIGHT + INPUT_TOOLBAR_HEIGHT;

#[derive(Debug, Clone)]
pub(crate) struct ExampleCard {
    pub(crate) title: String,
    pub(crate) subtitle: String,
    pub(crate) prompt: String,
    pub(crate) emoji: &'static str,
}

pub(crate) fn example_cards(locale: op_editor_core::Locale) -> [ExampleCard; 4] {
    let t = |key: &'static str| op_i18n::translate(locale, key).to_string();
    [
        ExampleCard {
            title: t("ai.quickAction.loginScreen"),
            subtitle: t("ai.quickAction.loginScreenDesc"),
            prompt: t("ai.quickAction.loginScreenPrompt"),
            emoji: "📱",
        },
        ExampleCard {
            title: t("ai.quickAction.foodApp"),
            subtitle: t("ai.quickAction.foodAppDesc"),
            prompt: t("ai.quickAction.foodAppPrompt"),
            emoji: "🍕",
        },
        ExampleCard {
            title: t("ai.quickAction.bottomNav"),
            subtitle: t("ai.quickAction.bottomNavDesc"),
            prompt: t("ai.quickAction.bottomNavPrompt"),
            emoji: "⬇️",
        },
        ExampleCard {
            title: t("ai.quickAction.colorPalette"),
            subtitle: t("ai.quickAction.colorPaletteDesc"),
            prompt: t("ai.quickAction.colorPalettePrompt"),
            emoji: "🎨",
        },
    ]
}

pub struct AIChatPlaceholder<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: &'a ChatState,
    pub now_ms: u64,
    pub label_new_chat: String,
    pub label_start_with_ai: String,
    pub label_input_placeholder: String,
    pub label_tip_select_elements: String,
    pub label_no_models: String,
    /// Number of currently selected canvas nodes, shown in the
    /// bottom toolbar like the TS panel.
    pub(crate) selected_count: usize,
    /// Whether the model-picker dropdown is open.
    pub model_picker_open: bool,
    /// Vertical scroll offset of the open model-picker dropdown.
    pub model_picker_scroll: f32,
    /// Index into `state.available_models` of the picker row under the cursor.
    pub model_picker_hover: Option<usize>,
    /// Live model-picker search query.
    pub model_picker_search: String,
    /// Byte caret for the model-picker search query.
    pub model_picker_caret: Option<usize>,
    /// Whether Ctrl/Cmd+A selected the full model-picker query.
    pub model_picker_select_all: bool,
    /// Last focus / edit timestamp for the model-picker search caret.
    pub model_picker_caret_anchor_ms: u64,
    pub design_hover: Option<(usize, usize)>,
    /// Empty-state quick action card under the cursor.
    pub example_hover: Option<usize>,
    /// Which bare header button the cursor is over (chevron / maximize
    /// / new chat) — drives their `theme.button_hover` wash.
    pub header_hover: Option<op_editor_core::ChatHeaderButton>,
    /// Which bottom-toolbar chat control the cursor is over.
    pub footer_hover: Option<op_editor_core::ChatFooterButton>,
    /// Localised empty-state example cards.
    pub(crate) examples: [ExampleCard; 4],
    /// Active UI locale.
    pub(crate) locale: op_editor_core::Locale,
}

impl<'a> AIChatPlaceholder<'a> {
    pub fn from_editor(state: &'a EditorState) -> Self {
        Self::from_editor_at(state, 0)
    }

    pub fn from_editor_at(state: &'a EditorState, now_ms: u64) -> Self {
        let ui = &state.editor_ui;
        Self {
            id: WidgetId::new(7000),
            theme: theme_for(ui),
            state: &state.chat,
            now_ms,
            // TS stores the chat title as UI state and defaults it to
            // this English title even under a Chinese locale.
            label_new_chat: "New Chat".to_string(),
            label_start_with_ai: translate(ui, "ai.startDesigning").to_string(),
            label_input_placeholder: translate(ui, "ai.designWithAgent").to_string(),
            label_tip_select_elements: translate(ui, "ai.tipSelectElements").to_string(),
            label_no_models: translate(ui, "ai.noModelsConnected").to_string(),
            selected_count: state.selection_count(),
            model_picker_open: ui.chat_model_picker_open,
            model_picker_scroll: ui.chat_model_picker_scroll,
            model_picker_hover: ui.chat_model_picker_hover,
            model_picker_search: ui.chat_model_picker_search.clone(),
            model_picker_caret: ui.chat_model_picker_caret,
            model_picker_select_all: ui.chat_model_picker_select_all,
            model_picker_caret_anchor_ms: ui.chat_model_picker_caret_anchor_ms,
            design_hover: ui.chat_design_block_hover,
            example_hover: ui.chat_example_hover,
            header_hover: ui.chat_header_hover,
            footer_hover: ui.chat_footer_hover,
            examples: example_cards(ui.locale),
            locale: ui.locale,
        }
    }

    /// Height of the staged-attachment row — `0` when none is staged.
    pub(crate) fn attachment_row_h(&self) -> f32 {
        if self.state.pending_attachments.is_empty() {
            0.0
        } else {
            ATTACHMENT_ROW_HEIGHT
        }
    }

    /// Total input-block height, including the attachment row when
    /// attachments are staged.
    pub(crate) fn input_height(&self) -> f32 {
        INPUT_BASE_HEIGHT + self.attachment_row_h()
    }

    fn maximize_icon(&self) -> Icon {
        [Icon::Maximize, Icon::Minimize][self.state.maximized as usize]
    }

    pub(crate) fn is_streaming(&self) -> bool {
        self.state.messages.iter().any(|message| message.streaming)
    }

    pub(crate) fn body_rect(&self, rect: Rect) -> Rect {
        let body_top = rect.origin.y + HEADER_HEIGHT + 14.0; // gap before first bubble
        let body_bottom = rect.origin.y + rect.size.y
            - self.input_height()
            - PAD
            - 8.0
            - fixed_checklist_height(&self.state.messages, self.state.checklist_collapsed);
        Rect {
            origin: Point2D::new(rect.origin.x + PAD, body_top),
            size: Point2D::new(rect.size.x - PAD * 2.0, (body_bottom - body_top).max(0.0)),
        }
    }

    pub(crate) fn model_picker_rect(&self, rect: Rect, input_rect: Rect) -> Rect {
        let height = crate::widgets::ai_chat_model_picker::picker_view_height(
            &self.state.available_models,
            &self.model_picker_search,
        );
        let toolbar_top = input_rect.origin.y + INPUT_AREA_HEIGHT + self.attachment_row_h();
        let bottom = toolbar_top - 4.0;
        Rect {
            origin: Point2D::new(rect.origin.x + PAD, bottom - height),
            size: Point2D::new(rect.size.x - PAD * 2.0, height),
        }
    }

    pub(crate) fn expanded_header_title_rect(&self, rect: Rect) -> Rect {
        let x = rect.origin.x + PAD - 8.0;
        let right_limit = rect.origin.x + rect.size.x - PAD - 58.0;
        let w = (28.0 + footer_label_width(&self.label_new_chat, 14.0) + 22.0)
            .max(112.0)
            .min((right_limit - x).max(36.0));
        Rect {
            origin: Point2D::new(x, rect.origin.y + 3.0),
            size: Point2D::new(w, 30.0),
        }
    }

    pub fn model_picker_bounds(&self, rect: Rect) -> Option<Rect> {
        if !self.model_picker_open {
            return None;
        }
        let input_rect = self.input_rect(rect);
        Some(self.model_picker_rect(rect, input_rect))
    }

    pub fn input_rect(&self, rect: Rect) -> Rect {
        let input_h = self.input_height();
        Rect {
            origin: Point2D::new(
                rect.origin.x + PAD,
                rect.origin.y + rect.size.y - input_h + 1.0,
            ),
            size: Point2D::new(rect.size.x - PAD * 2.0, input_h),
        }
    }

    pub(crate) fn footer_layout(
        &self,
        rect: Rect,
        input_rect: Rect,
        toolbar_top: f32,
    ) -> FooterLayout {
        let toolbar_center_y = toolbar_top + INPUT_TOOLBAR_HEIGHT / 2.0;
        let selected = self.state.selected_model_entry();
        let model_name: &str = selected
            .map(|m| m.display_name.as_str())
            .unwrap_or(self.label_no_models.as_str());
        let model_w = footer_label_width(model_name, 12.0);
        let agent_team_x = input_rect.origin.x + 20.0 + model_w + 4.0 + 18.0;
        let model = Rect {
            origin: Point2D::new(input_rect.origin.x - 6.0, toolbar_center_y - 14.0),
            size: Point2D::new((agent_team_x - input_rect.origin.x).max(44.0), 28.0),
        };
        let agent_team = Rect {
            origin: Point2D::new(agent_team_x, toolbar_center_y - 10.0),
            size: Point2D::new(28.0, 20.0),
        };
        let rx = rect.origin.x + rect.size.x - PAD;
        let attach = Rect {
            origin: Point2D::new(rx - 58.0, toolbar_center_y - 12.0),
            size: Point2D::new(24.0, 24.0),
        };
        let send = Rect {
            origin: Point2D::new(rx - 24.0, toolbar_center_y - 12.0),
            size: Point2D::new(24.0, 24.0),
        };
        FooterLayout {
            model,
            agent_team,
            attach,
            send,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FooterLayout {
    pub(crate) model: Rect,
    pub(crate) agent_team: Rect,
    pub(crate) attach: Rect,
    pub(crate) send: Rect,
}

fn footer_label_width(label: &str, size: f32) -> f32 {
    label.chars().fold(0.0, |w, c| {
        w + if c.is_ascii() { size * 0.55 } else { size }
    })
}

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
        // Collapsed TS-style pill.
        if self.state.collapsed {
            cx.backend
                .fill_round_rect(rect, COLLAPSED_RADIUS, self.theme.card);
            if self.header_hover == Some(op_editor_core::ChatHeaderButton::ToggleCollapse) {
                cx.backend.fill_round_rect(
                    rect,
                    COLLAPSED_RADIUS,
                    chat_neutral_hover_color(&self.theme),
                );
            }
            cx.backend
                .stroke_round_rect(rect, COLLAPSED_RADIUS, self.theme.border, 1.0);
            let center_y = rect.origin.y + rect.size.y / 2.0;
            // Bubble at the left.
            draw_icon(
                cx.backend,
                Icon::MessageSquare,
                Point2D::new(
                    rect.origin.x + COLLAPSED_X_PAD,
                    center_y - COLLAPSED_MESSAGE_ICON / 2.0,
                ),
                COLLAPSED_MESSAGE_ICON,
                self.theme.muted_foreground,
                1.4,
            );
            // "New Chat" label.
            let title = TextLayout::single_run(
                &self.label_new_chat,
                "system-ui",
                12.0,
                to_jian_color(self.theme.muted_foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &title,
                Point2D::new(
                    rect.origin.x + COLLAPSED_X_PAD + COLLAPSED_MESSAGE_ICON + COLLAPSED_GAP,
                    center_y + 4.0,
                ),
            );
            // Chevron-up at the right (click to expand).
            draw_icon(
                cx.backend,
                Icon::ChevronUp,
                Point2D::new(
                    rect.origin.x + rect.size.x - COLLAPSED_X_PAD - COLLAPSED_CHEVRON_ICON,
                    center_y - COLLAPSED_CHEVRON_ICON / 2.0,
                ),
                COLLAPSED_CHEVRON_ICON,
                self.theme.muted_foreground,
                1.4,
            );
            return;
        }

        paint_panel_surface(cx, &self.theme, rect);
        let can_use_model = !self.state.available_models.is_empty();
        let input_h = self.input_height();
        let sep_y = rect.origin.y + rect.size.y - input_h;
        paint_panel_body_chrome(cx, &self.theme, rect, sep_y);

        // Expanded header.
        use op_editor_core::ChatHeaderButton;
        let header_y = rect.origin.y + 8.0;
        let chevron_x = rect.origin.x + PAD;
        let title_hovered = self.header_hover == Some(ChatHeaderButton::ToggleCollapse);
        if title_hovered {
            cx.backend.fill_round_rect(
                self.expanded_header_title_rect(rect),
                8.0,
                chat_neutral_hover_color(&self.theme),
            );
        }
        let chevron_color = if title_hovered {
            self.theme.foreground
        } else {
            self.theme.muted_foreground
        };
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(chevron_x, header_y),
            18.0,
            chevron_color,
            1.4,
        );
        let title = TextLayout::single_run(
            &self.label_new_chat,
            "system-ui",
            14.0,
            to_jian_color(self.theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title,
            Point2D::new(rect.origin.x + PAD + 28.0, header_y + 14.0),
        );
        let maximize_x = rect.origin.x + rect.size.x - PAD - 50.0;
        let maximize_color = paint_header_btn_bg(
            cx,
            &self.theme,
            maximize_x,
            header_y,
            self.header_hover == Some(ChatHeaderButton::ToggleMaximize),
        );
        draw_icon(
            cx.backend,
            self.maximize_icon(),
            Point2D::new(maximize_x, header_y),
            18.0,
            maximize_color,
            1.4,
        );
        let new_chat_x = rect.origin.x + rect.size.x - PAD - 22.0;
        let new_chat_color = paint_header_btn_bg(
            cx,
            &self.theme,
            new_chat_x,
            header_y,
            self.header_hover == Some(ChatHeaderButton::NewChat),
        );
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(new_chat_x, header_y),
            18.0,
            new_chat_color,
            1.4,
        );

        // Body — either messages or examples.
        let checklist_h =
            fixed_checklist_height(&self.state.messages, self.state.checklist_collapsed);
        if self.state.messages.is_empty() {
            paint_examples(
                cx,
                &self.theme,
                rect,
                &self.label_start_with_ai,
                &self.label_tip_select_elements,
                &self.examples,
                !can_use_model || self.is_streaming(),
                self.example_hover,
            );
        } else {
            crate::widgets::ai_chat_transcript::paint_transcript_with_selection(
                cx,
                &self.theme,
                self.body_rect(rect),
                &self.state.messages,
                self.now_ms,
                self.locale,
                self.design_hover,
                self.state.transcript_selection,
            );
        }
        if checklist_h > 0.0 {
            paint_fixed_checklist(
                cx,
                &self.theme,
                fixed_checklist_rect(rect, input_h, checklist_h),
                &self.state.messages,
                self.state.checklist_collapsed,
                self.state.checklist_scroll,
            );
        }
        let input_rect = Rect {
            origin: Point2D::new(rect.origin.x + PAD, sep_y + 1.0),
            size: Point2D::new(rect.size.x - PAD * 2.0, INPUT_AREA_HEIGHT),
        };
        crate::widgets::ai_chat_input_text::paint_input_text_area(
            cx,
            &self.theme,
            self.state,
            input_rect,
            INPUT_AREA_HEIGHT,
            self.now_ms,
            &self.label_input_placeholder,
        );

        // Staged-attachment strip — between the textarea and the
        // controls row, present only when attachments are staged.
        let attach_h = self.attachment_row_h();
        if attach_h > 0.0 {
            let attach_rect = Rect {
                origin: Point2D::new(input_rect.origin.x, input_rect.origin.y + INPUT_AREA_HEIGHT),
                size: Point2D::new(input_rect.size.x, attach_h),
            };
            paint_attachment_row(cx, &self.theme, attach_rect, self.state);
        }

        // Bottom toolbar — model picker on the left, send on the
        // right (mirrors the TS panel's bottom row).
        let toolbar_y = input_rect.origin.y + INPUT_AREA_HEIGHT + attach_h;
        let toolbar_center_y = toolbar_y + INPUT_TOOLBAR_HEIGHT / 2.0;
        let footer = self.footer_layout(rect, self.input_rect(rect), toolbar_y);
        use op_editor_core::ChatFooterButton;
        // Model chip — brand logo of the selected model's provider
        // + its display name + a chevron. Click toggles the picker.
        let mut model_x = rect.origin.x + PAD;
        if self.footer_hover == Some(ChatFooterButton::ModelPicker) {
            cx.backend
                .fill_round_rect(footer.model, 6.0, chat_neutral_hover_color(&self.theme));
        }
        let selected = self.state.selected_model_entry();
        let chip_color = self.theme.muted_foreground;
        match selected {
            Some(entry)
                if entry.builtin_provider_id.is_some() || entry.value.starts_with("builtin:") =>
            {
                crate::widgets::ai_chat_model_picker::paint_key_glyph(
                    cx,
                    Point2D::new(model_x, toolbar_center_y - 7.0),
                    14.0,
                    chip_color,
                )
            }
            Some(entry) => crate::widgets::ai_chat_model_picker::paint_provider_logo(
                cx,
                entry.provider,
                Point2D::new(model_x, toolbar_center_y - 7.0),
                14.0,
                chip_color,
            ),
            // No model discovered yet — generic sparkles glyph.
            None => draw_icon(
                cx.backend,
                Icon::Sparkles,
                Point2D::new(model_x, toolbar_center_y - 7.0),
                14.0,
                chip_color,
                1.4,
            ),
        }
        model_x += 20.0;
        let model_name: &str = selected
            .map(|m| m.display_name.as_str())
            .unwrap_or(self.label_no_models.as_str());
        let model_label = TextLayout::single_run(
            model_name,
            "system-ui",
            12.0,
            to_jian_color(chip_color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&model_label, Point2D::new(model_x, toolbar_center_y + 4.0));
        let model_w = cx.backend.measure_text(model_name, 12.0);
        model_x += model_w + 4.0;
        draw_icon(
            cx.backend,
            Icon::ChevronUp,
            Point2D::new(model_x, toolbar_center_y - 5.0),
            10.0,
            self.theme.muted_foreground,
            1.4,
        );
<<<<<<< HEAD
        model_x += 18.0;
        // Concurrency chip — `Zap` + `{n}x`. A solo 1x team rests as a
        // faint ghost; a staffed team (>1) gets a primary-tinted chip so
        // the parallel mode reads as active.
=======
>>>>>>> 926f84b6 (feat(editor-ui): canvas + panel widget batch for TS parity)
        let chip = footer.agent_team;
        let team_size = self.state.agent_team_size;
        let team_active = team_size > 1;
        let conc_color = if team_active {
            self.theme.primary
        } else {
            Color {
                a: 0.5,
                ..self.theme.muted_foreground
            }
        };
        let primary_wash = Color {
            a: 0.1,
            ..self.theme.primary
        };
        if team_active {
            cx.backend.fill_round_rect(chip, 6.0, primary_wash);
        }
        if self.footer_hover == Some(ChatFooterButton::AgentTeam) {
            let wash = if team_active {
                primary_wash
            } else {
                chat_neutral_hover_color(&self.theme)
            };
            cx.backend.fill_round_rect(chip, 6.0, wash);
        }
        draw_icon(
            cx.backend,
            Icon::Zap,
            Point2D::new(chip.origin.x + 4.0, chip.origin.y + 5.0),
            10.0,
            conc_color,
            1.4,
        );
        let team_label = format!("{}x", team_size);
        draw_label(
            cx,
            &team_label,
            11.0,
            conc_color,
            chip.origin.x + 16.0,
            chip.origin.y + 14.0,
        );
        model_x = chip.origin.x + chip.size.x + 4.0;
        let count = self.selected_count.to_string();
        let selected_label =
            op_i18n::translate(self.locale, "common.selected").replace("{{count}}", &count);
        draw_label(
            cx,
            &selected_label,
            10.0,
            self.theme.muted_foreground,
            model_x,
            toolbar_center_y + 4.0,
        );

        // Right cluster — attach + send (TS ghost buttons: bare icons
        // that only get a wash while hovered).
        let attach_rect = footer.attach;
        if self.footer_hover == Some(ChatFooterButton::AddAttachment) {
            cx.backend
                .fill_round_rect(attach_rect, 6.0, chat_neutral_hover_color(&self.theme));
        }
        draw_icon(
            cx.backend,
            Icon::Paperclip,
            Point2D::new(attach_rect.origin.x + 6.0, attach_rect.origin.y + 6.0),
            12.0,
            self.theme.muted_foreground,
            1.4,
        );
        let send_rect = footer.send;
        // A turn is sendable with text, with staged attachments, or
        // both (TS parity: an attachment-only message is valid).
        let send_active = can_use_model
            && (!self.state.input.trim().is_empty() || !self.state.pending_attachments.is_empty());
        let streaming = self.is_streaming();
        // TS: streaming → red stop; sendable → primary; otherwise a faded
        // muted-foreground/30 icon so the send button reads as disabled.
        let (icon_color, send_icon) = if streaming {
            (self.theme.destructive, Icon::Square)
        } else if send_active {
            (self.theme.primary, Icon::Send)
        } else {
            (
                Color {
                    a: 0.3,
                    ..self.theme.muted_foreground
                },
                Icon::Send,
            )
        };
        if self.footer_hover == Some(ChatFooterButton::Send)
            || self.footer_hover == Some(ChatFooterButton::Stop)
        {
            cx.backend
                .fill_round_rect(send_rect, 6.0, chat_neutral_hover_color(&self.theme));
        }
        draw_icon(
            cx.backend,
            send_icon,
            Point2D::new(send_rect.origin.x + 6.0, send_rect.origin.y + 6.0),
            12.0,
            icon_color,
            1.4,
        );

        // Model-picker dropdown paints last so it sits above the
        // message list / examples / input.
        if self.model_picker_open {
            let picker = self.model_picker_rect(rect, input_rect);
            crate::widgets::ai_chat_model_picker::paint_model_picker(
                cx,
                &self.theme,
                picker,
                &self.state.available_models,
                self.state.selected_model,
                self.model_picker_scroll,
                self.model_picker_hover,
                &self.model_picker_search,
                self.model_picker_caret,
                self.model_picker_select_all,
                self.now_ms,
                self.model_picker_caret_anchor_ms,
                self.locale,
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(op_i18n::translate(self.locale, "a11y.aiChat"));
        node
    }
}

fn draw_label(cx: &mut PaintCx<'_>, text: &str, size: f32, color: Color, x: f32, y: f32) {
    let label = TextLayout::single_run(
        text,
        "system-ui",
        size,
        to_jian_color(color),
        Point2D::new(0.0, 0.0),
    );
    cx.backend.draw_text(&label, Point2D::new(x, y));
}

fn chat_neutral_hover_color(theme: &Theme) -> Color {
    Color {
        r: theme.foreground.r,
        g: theme.foreground.g,
        b: theme.foreground.b,
        a: 0.12,
    }
}

/// Paint the `theme.button_hover` wash behind a bare header glyph
/// (18 px, drawn at `(icon_x, header_y)`) when the cursor rests on it,
/// returning the glyph color: foreground while hovered, muted
/// otherwise. The wash is a 24 px square centred on the glyph.
fn paint_header_btn_bg(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    icon_x: f32,
    header_y: f32,
    hovered: bool,
) -> Color {
    if hovered {
        let center = Point2D::new(icon_x + 9.0, header_y + 9.0);
        let r = Rect {
            origin: Point2D::new(center.x - 12.0, center.y - 12.0),
            size: Point2D::new(24.0, 24.0),
        };
        cx.backend.fill_round_rect(r, 6.0, theme.button_hover);
        theme.foreground
    } else {
        theme.muted_foreground
    }
}

pub(crate) fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
#[path = "ai_chat_panel/tests.rs"]
mod tests;
<<<<<<< HEAD
#[cfg(test)]
#[path = "ai_chat_panel/tests_paint.rs"]
mod tests_paint;
=======

#[cfg(test)]
#[path = "ai_chat_panel/tests_transcript.rs"]
mod tests_transcript;
>>>>>>> 926f84b6 (feat(editor-ui): canvas + panel widget batch for TS parity)
