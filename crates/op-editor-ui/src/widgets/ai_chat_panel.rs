use crate::theme::Theme;
use crate::widgets::ai_chat_checklist::{
    fixed_checklist_height, fixed_checklist_rect, paint_fixed_checklist,
};
pub(crate) use crate::widgets::ai_chat_panel_controls::chat_neutral_feedback_color;
// Re-exported for paint tests that verify hover tint colours.
#[cfg(test)]
pub(crate) use crate::widgets::ai_chat_panel_controls::chat_neutral_hover_color;
use crate::widgets::ai_chat_panel_controls::{paint_attachment_row, ATTACHMENT_ROW_HEIGHT};
use crate::widgets::ai_chat_panel_footer::paint_bottom_toolbar;
use crate::widgets::ai_chat_panel_header::{
    paint_header_tabs, paint_new_chat_tooltip, MAXIMIZE_GAP, MAXIMIZE_W, NEW_CHAT_D,
};
use crate::widgets::ai_chat_panel_paint::{
    paint_examples, paint_panel_body_chrome, paint_panel_surface,
};
use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Point2D, Rect, TextLayout};
use jian_core::text_input::TextInputState;
use jian_widgets::components::select::SelectState;
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
pub(crate) const INPUT_TOOLBAR_HEIGHT: f32 = 40.0;
#[cfg(test)]
const INPUT_BASE_HEIGHT: f32 = INPUT_AREA_HEIGHT + INPUT_TOOLBAR_HEIGHT;

#[derive(Debug, Clone)]
#[allow(dead_code)] // subtitle/emoji retained for parity with card schema; not painted yet
pub(crate) struct ExampleCard {
    pub(crate) title: String,
    pub(crate) subtitle: String,
    pub(crate) prompt: String,
    pub(crate) emoji: &'static str,
}

pub(crate) fn example_cards(locale: op_editor_core::Locale) -> [ExampleCard; 4] {
    let t = |key: &'static str| op_i18n::translate(locale, key).to_string();
    // Pills show only the title as a single-line label; subtitle and emoji are
    // retained in the struct for potential future use but are not painted (#33 restyle).
    [
        ExampleCard {
            title: t("ai.quickAction.dashboard"),
            subtitle: String::new(),
            prompt: t("ai.quickAction.dashboardPrompt"),
            emoji: "",
        },
        ExampleCard {
            title: t("ai.quickAction.terminalDashboard"),
            subtitle: String::new(),
            prompt: t("ai.quickAction.terminalDashboardPrompt"),
            emoji: "",
        },
        ExampleCard {
            title: t("ai.quickAction.coffeeShop"),
            subtitle: String::new(),
            prompt: t("ai.quickAction.coffeeShopPrompt"),
            emoji: "",
        },
        ExampleCard {
            title: t("ai.quickAction.barbershop"),
            subtitle: String::new(),
            prompt: t("ai.quickAction.barbershopPrompt"),
            emoji: "",
        },
    ]
}

pub struct AIChatPlaceholder<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: &'a ChatState,
    pub now_ms: u64,
    pub label_start_with_ai: String,
    pub label_input_placeholder: String,
    pub label_tip_select_elements: String,
    pub label_no_models: String,
    /// Number of currently selected canvas nodes.
    /// Kept for future affordances; paint tests verify it is seeded correctly.
    #[allow(dead_code)]
    pub(crate) selected_count: usize,
    /// Model-picker dropdown interaction state.
    pub model_picker: &'a SelectState,
    /// Text state for the model-picker search query.
    pub model_picker_input: &'a TextInputState,
    pub design_hover: Option<(usize, usize)>,
    /// Empty-state quick action card under the cursor.
    pub example_hover: Option<usize>,
    pub example_pressed: Option<usize>,
    /// Which bare header button the cursor is over (chevron / maximize
    /// / new chat) — drives their `theme.button_hover` wash.
    pub header_hover: Option<op_editor_core::ChatHeaderButton>,
    /// Which bottom-toolbar chat control the cursor is over.
    pub footer_hover: Option<op_editor_core::ChatFooterButton>,
    pub header_pressed: Option<op_editor_core::ChatHeaderButton>,
    pub footer_pressed: Option<op_editor_core::ChatFooterButton>,
    /// Localised empty-state example cards.
    pub(crate) examples: [ExampleCard; 4],
    /// Active UI locale.
    pub(crate) locale: op_editor_core::Locale,
    /// All open chat tabs (from `state.chat.tabs()`). Kept as an owned
    /// snapshot so the tab row can paint all titles in one pass without
    /// holding a borrow on `state.chat` that conflicts with `Deref`.
    ///
    /// `state` already points to the active tab via `Deref`; this vec
    /// holds the full collection for the tab-row renderer.
    pub(crate) tabs_snapshot: Vec<ChatTabInfo>,
    /// Active tab index (from `state.chat.active_index()`).
    pub(crate) active_tab_index: usize,
    /// Which tab (if any) the cursor is over — drives × close glyph
    /// and hover wash on inactive tabs.
    pub(crate) tab_hover: Option<usize>,
    /// Whether the Parallel Agents picker dropdown is open.
    pub(crate) parallel_agents_picker_open: bool,
    /// Which row (1–6) the cursor is over inside the Parallel Agents picker.
    pub(crate) parallel_agents_picker_hover: Option<u32>,
}

/// Minimal per-tab snapshot used by the tab-row painter.
///
/// Only the fields the painter actually reads — avoids a full `ChatState`
/// clone (which includes message histories and attachments).
#[derive(Debug, Clone)]
pub(crate) struct ChatTabInfo {
    /// Tab display title (same as `ChatState::title`).
    pub(crate) title: String,
}

impl<'a> AIChatPlaceholder<'a> {
    pub fn from_editor(state: &'a EditorState) -> Self {
        Self::from_editor_at(state, 0)
    }

    pub fn from_editor_at(state: &'a EditorState, now_ms: u64) -> Self {
        let ui = &state.editor_ui;
        // Build a lightweight snapshot of all tabs so the tab-row painter
        // can iterate titles without holding a borrow on the ChatSessions.
        let tabs_snapshot: Vec<ChatTabInfo> = state
            .chat
            .tabs()
            .iter()
            .map(|tab| ChatTabInfo {
                title: tab.title.clone(),
            })
            .collect();
        let active_tab_index = state.chat.active_index();
        Self {
            id: WidgetId::new(7000),
            theme: theme_for(ui),
            state: &state.chat,
            now_ms,
            label_start_with_ai: translate(ui, "ai.tryExample").to_string(),
            label_input_placeholder: translate(ui, "ai.designWithAgent").to_string(),
            label_tip_select_elements: translate(ui, "ai.tipSelectElements").to_string(),
            label_no_models: translate(ui, "ai.noModelsConnected").to_string(),
            selected_count: state.selection_count(),
            model_picker: &ui.chat_model_picker,
            model_picker_input: &ui.chat_model_picker_input,
            design_hover: ui.chat_design_block_hover,
            example_hover: ui.chat_example_hover,
            example_pressed: match ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::ChatExample(index)) => Some(index),
                _ => None,
            },
            header_hover: ui.chat_header_hover,
            footer_hover: ui.chat_footer_hover,
            header_pressed: match ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::ChatHeader(button)) => Some(button),
                _ => None,
            },
            footer_pressed: match ui.pressed_button {
                Some(op_editor_core::ButtonPressTarget::ChatFooter(button)) => Some(button),
                _ => None,
            },
            examples: example_cards(ui.locale),
            locale: ui.locale,
            tabs_snapshot,
            active_tab_index,
            tab_hover: ui.chat_tab_hover,
            parallel_agents_picker_open: ui.parallel_agents_picker_open,
            parallel_agents_picker_hover: ui.parallel_agents_picker_hover,
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
    #[cfg(test)]
    pub(crate) fn input_height(&self) -> f32 {
        self.input_height_for_width(self.state.panel_width)
    }

    pub(crate) fn input_area_height_for_input_width(&self, input_w: f32) -> f32 {
        let lines = crate::widgets::ai_chat_input_text::visible_input_line_count(
            self.state.input.text(),
            input_w,
        );
        INPUT_AREA_HEIGHT
            + (lines.saturating_sub(1) as f32) * crate::widgets::ai_chat_input_text::INPUT_LINE_H
    }

    pub(crate) fn input_area_height_for_width(&self, panel_w: f32) -> f32 {
        let input_w = (panel_w - PAD * 2.0).max(0.0);
        self.input_area_height_for_input_width(input_w)
    }

    pub(crate) fn input_height_for_width(&self, panel_w: f32) -> f32 {
        self.input_area_height_for_width(panel_w) + INPUT_TOOLBAR_HEIGHT + self.attachment_row_h()
    }

    pub(crate) fn input_area_height_for_rect(&self, rect: Rect) -> f32 {
        self.input_area_height_for_width(rect.size.x)
    }

    pub(crate) fn input_height_for_rect(&self, rect: Rect) -> f32 {
        self.input_height_for_width(rect.size.x)
    }

    fn maximize_icon(&self) -> Icon {
        [Icon::Maximize, Icon::Minimize][self.state.maximized as usize]
    }

    pub(crate) fn is_streaming(&self) -> bool {
        self.state.messages.iter().any(|message| message.streaming)
    }

    pub fn body_rect(&self, rect: Rect) -> Rect {
        let body_top = rect.origin.y + HEADER_HEIGHT + 14.0; // gap before first bubble
        let body_bottom = rect.origin.y + rect.size.y
            - self.input_height_for_rect(rect)
            - PAD
            - 8.0
            - fixed_checklist_height(self.state, self.state.checklist_collapsed);
        Rect {
            origin: Point2D::new(rect.origin.x + PAD, body_top),
            size: Point2D::new(rect.size.x - PAD * 2.0, (body_bottom - body_top).max(0.0)),
        }
    }

    /// Maximum transcript scroll offset (px) for the panel laid out at
    /// `rect`: `content_height - body_height`, clamped at 0. The host's
    /// wheel handler clamps the stored offset to this and re-pins to the
    /// bottom once it is reached.
    pub fn transcript_scroll_max(&self, rect: Rect) -> f32 {
        let body = self.body_rect(rect);
        (crate::widgets::ai_chat_transcript::transcript_content_height(
            &self.state.messages,
            body,
            self.locale,
        ) - body.size.y)
            .max(0.0)
    }

    pub(crate) fn model_picker_rect(&self, rect: Rect, input_rect: Rect) -> Rect {
        let height = crate::widgets::ai_chat_model_picker::picker_view_height(
            &self.state.available_models,
            self.model_picker_input.text(),
        );
        let toolbar_top =
            input_rect.origin.y + self.input_area_height_for_rect(rect) + self.attachment_row_h();
        let bottom = toolbar_top - 4.0;
        Rect {
            origin: Point2D::new(rect.origin.x + PAD, bottom - height),
            size: Point2D::new(rect.size.x - PAD * 2.0, height),
        }
    }

    /// Collapse-toggle hit area in the expanded header.
    ///
    /// With multi-tab layout: the tab bodies produce `SwitchTab` hits, so
    /// ToggleCollapse is now scoped to just the chevron icon (18×18 px at
    /// the far left). The hit rect is slightly generous (18×26 to match the
    /// row height) so it is easy to click.
    ///
    /// In single-tab mode the user can still click the single active tab to
    /// collapse via the tab's `SwitchTab(0)` being harmless (already active),
    /// but the chevron is always the canonical collapse affordance.
    pub(crate) fn expanded_header_title_rect(&self, rect: Rect) -> Rect {
        use crate::widgets::ai_chat_panel_header::CHEVRON_W;
        let chevron_h = 26.0; // generous hit target matching the pill height
        Rect {
            origin: Point2D::new(
                rect.origin.x + PAD,
                rect.origin.y + (HEADER_HEIGHT - chevron_h) / 2.0,
            ),
            size: Point2D::new(CHEVRON_W, chevron_h),
        }
    }

    pub fn model_picker_bounds(&self, rect: Rect) -> Option<Rect> {
        if !self.model_picker.open {
            return None;
        }
        let input_rect = self.input_rect(rect);
        Some(self.model_picker_rect(rect, input_rect))
    }

    /// Bounding rect of the Parallel Agents picker dropdown overlay.
    /// The picker lists 6 rows of "Nx" (N=1..=6) above the speed chip.
    /// `None` when the picker is closed.
    pub(crate) fn parallel_agents_picker_rect(
        &self,
        _rect: Rect,
        footer: &FooterLayout,
    ) -> Option<Rect> {
        if !self.parallel_agents_picker_open {
            return None;
        }
        Some(crate::widgets::ai_chat_panel_footer::parallel_agents_picker_rect(footer))
    }

    pub fn input_rect(&self, rect: Rect) -> Rect {
        let input_h = self.input_height_for_rect(rect);
        Rect {
            origin: Point2D::new(
                rect.origin.x + PAD,
                rect.origin.y + rect.size.y - input_h + 1.0,
            ),
            size: Point2D::new(rect.size.x - PAD * 2.0, input_h),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FooterLayout {
    /// Model picker pill — left anchor of the toolbar.
    pub(crate) model: Rect,
    /// Speed/effort chip — ⚡ icon + effort label in gold, no bg.
    /// Retained next to the model pill; clicking cycles effort level.
    pub(crate) speed: Rect,
    /// Agent-team size chip — kept for backward compat / future use.
    pub(crate) agent_team: Rect,
    /// Paperclip attach button — bare icon, muted.
    pub(crate) attach: Rect,
    /// Palette button — bare icon, muted, currently inert (#27 spec).
    pub(crate) palette: Rect,
    /// Stop circle — shown only while a turn streams.
    pub(crate) stop: Rect,
    /// Send/stop circle — the primary action button.
    pub(crate) send: Rect,
}

pub(crate) fn footer_label_width(label: &str, size: f32) -> f32 {
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
            if self.header_hover == Some(op_editor_core::ChatHeaderButton::ToggleCollapse)
                || self.header_pressed == Some(op_editor_core::ChatHeaderButton::ToggleCollapse)
            {
                cx.backend.fill_round_rect(
                    rect,
                    COLLAPSED_RADIUS,
                    chat_neutral_feedback_color(
                        &self.theme,
                        self.header_pressed
                            == Some(op_editor_core::ChatHeaderButton::ToggleCollapse),
                    ),
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
            // Title label — ellipsized to the space between the bubble icon
            // and the chevron so a long chat title can't overflow the pill.
            let title_x = rect.origin.x + COLLAPSED_X_PAD + COLLAPSED_MESSAGE_ICON + COLLAPSED_GAP;
            let chevron_x = rect.origin.x + rect.size.x - COLLAPSED_X_PAD - COLLAPSED_CHEVRON_ICON;
            let max_title_w = (chevron_x - COLLAPSED_GAP - title_x).max(0.0);
            let title_text = crate::util::ellipsize_to_width(&self.state.title, max_title_w, |s| {
                cx.backend.measure_text(s, 12.0)
            });
            let title = TextLayout::single_run(
                &title_text,
                "system-ui",
                12.0,
                (self.theme.muted_foreground).to_jian(),
                Point2D::new(0.0, 0.0),
            );
            cx.backend
                .draw_text(&title, Point2D::new(title_x, center_y + 4.0));
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
        let checklist_h = fixed_checklist_height(self.state, self.state.checklist_collapsed);
        if self.state.messages.is_empty() {
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
            );
        } else {
            let body = self.body_rect(rect);
            let scroll_offset = crate::widgets::ai_chat_transcript::transcript_effective_offset(
                &self.state.messages,
                body,
                self.locale,
                self.state.transcript_scroll.offset,
                self.state.transcript_pinned,
            );
            crate::widgets::ai_chat_transcript::paint_transcript_with_selection(
                cx,
                &self.theme,
                body,
                &self.state.messages,
                self.now_ms,
                self.locale,
                self.design_hover,
                self.state.transcript_selection,
                scroll_offset,
            );
        }
        if checklist_h > 0.0 {
            paint_fixed_checklist(
                cx,
                &self.theme,
                fixed_checklist_rect(rect, input_h, checklist_h),
                self.state,
                self.state.checklist_collapsed,
                self.state.checklist_scroll.offset,
            );
        }
        let input_rect = Rect {
            origin: Point2D::new(rect.origin.x + PAD, sep_y + 1.0),
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
        //   model pill | ⚡ speed chip | 📎 attach | 🎨 palette | [gap] | ◻ stop | ↑ send
        let toolbar_y = input_rect.origin.y + input_area_h + attach_h;
        let toolbar_center_y = toolbar_y + INPUT_TOOLBAR_HEIGHT / 2.0;
        let footer = self.footer_layout(rect, self.input_rect(rect), toolbar_y);
        let streaming = self.is_streaming();
        let send_active = can_use_model
            && (!self.state.input.text().trim().is_empty()
                || !self.state.pending_attachments.is_empty());
        paint_bottom_toolbar(cx, self, &footer, toolbar_center_y, streaming, send_active);

        // Model-picker dropdown paints above other panel content.
        if self.model_picker.open {
            let picker = self.model_picker_rect(rect, input_rect);
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
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label(op_i18n::translate(self.locale, "a11y.aiChat"));
        node
    }
}

#[cfg(test)]
#[path = "ai_chat_panel/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "ai_chat_panel/tests_paint.rs"]
mod tests_paint;

#[cfg(test)]
#[path = "ai_chat_panel/tests_transcript.rs"]
mod tests_transcript;
