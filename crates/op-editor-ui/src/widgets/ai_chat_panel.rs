use crate::theme::Theme;
use crate::widgets::ai_chat_chip_row::ChipRowLayout;
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
use crate::{Point2D, Rect};
use jian_core::text_input::TextInputState;
use jian_widgets::components::select::SelectState;
use op_editor_core::chat::ChatState;
use op_editor_core::{EditorState, PenNodeExt};

pub const AI_CHAT_WIDTH: f32 = op_editor_core::chat::DEFAULT_CHAT_PANEL_WIDTH;
pub const AI_CHAT_HEIGHT: f32 = op_editor_core::chat::DEFAULT_CHAT_PANEL_HEIGHT;
pub const AI_CHAT_MIN_WIDTH: f32 = 280.0;
pub const AI_CHAT_MIN_HEIGHT: f32 = 250.0;
pub const AI_CHAT_MAX_RATIO: f32 = 0.8;
pub(crate) const PAD: f32 = 16.0;
pub(crate) const HEADER_HEIGHT: f32 = 36.0;
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
            title: t("ai.quickAction.travelApp"),
            subtitle: String::new(),
            prompt: t("ai.quickAction.travelAppPrompt"),
            emoji: "",
        },
        ExampleCard {
            title: t("ai.quickAction.dashboard"),
            subtitle: String::new(),
            prompt: t("ai.quickAction.dashboardPrompt"),
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
    /// Text painted in the selection chip: selected node name for a
    /// single resolved selection, count label for multi-selection.
    pub(crate) selected_label: Option<String>,
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
    /// What the next generation will style itself with, or `None` when there
    /// is nothing true to report. See `ai_chat_style_receipt`.
    pub(crate) style_receipt: Option<crate::widgets::ai_chat_style_receipt::StyleReceipt>,
    /// The receipt's hover card, present only once its dwell has elapsed.
    /// Resolving it is gated on that so an un-hovered panel does no work for
    /// it at all — see `ai_chat_style_card`.
    pub(crate) style_card: Option<crate::widgets::ai_chat_style_card::StyleCard>,
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
    /// Opaque, stable per-panel-INSTANCE id used to scope the thread-local
    /// transcript cache. A host allocates ONE id (`Self::next_owner`) for its
    /// persistent widget-host state and stamps every constructed panel with it
    /// via [`Self::owned_by`], so the display-frame hint
    /// (`hit_test_current_build`) only ever reads a build THIS panel resolved —
    /// never one left behind by a different panel after a tab/host switch.
    /// Defaults to `UNOWNED` (0) for plain `from_editor*` constructions (unit
    /// tests / paths that never touch the hint).
    pub(crate) owner: u64,
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
            selected_label: selection_chip_label_for_state(state),
            style_receipt: crate::widgets::ai_chat_style_receipt::StyleReceipt::for_state(state),
            style_card: crate::widgets::ai_chat_style_card::StyleCard::for_state(state, now_ms),
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
            examples: example_cards(ui.effective_locale()),
            locale: ui.effective_locale(),
            tabs_snapshot,
            active_tab_index,
            tab_hover: ui.chat_tab_hover,
            parallel_agents_picker_open: ui.parallel_agents_picker_open,
            parallel_agents_picker_hover: ui.parallel_agents_picker_hover,
            owner: crate::widgets::ai_chat_transcript_cache::UNOWNED,
        }
    }

    /// Allocate a fresh, process-unique panel-owner id. A host calls this ONCE
    /// for its persistent widget-host state; the id is then handed to every
    /// panel it builds via [`Self::owned_by`].
    pub fn next_owner() -> u64 {
        crate::widgets::ai_chat_transcript_cache::next_panel_owner()
    }

    /// Stamp this panel with its host's stable owner id so its transcript-cache
    /// resolves and display-frame hint reads are scoped to this panel instance.
    /// A no-op-shaped builder: hosts append `.owned_by(self.chat_panel_owner)`
    /// at the paint / cursor-probe / cursor-hint construction sites.
    pub fn owned_by(mut self, owner: u64) -> Self {
        self.owner = owner;
        self
    }

    /// Height of the staged-attachment row — `0` when none is staged.
    pub(crate) fn attachment_row_h(&self) -> f32 {
        if self.state.pending_attachments.is_empty() {
            0.0
        } else {
            ATTACHMENT_ROW_HEIGHT
        }
    }

    /// Height of the context-chip row — `0` when neither chip is live, so a
    /// user with nothing pinned and nothing selected gets no empty band
    /// above the input at all.
    pub(crate) fn chip_row_h(&self) -> f32 {
        if self.style_receipt.is_some() || self.selected_count > 0 {
            crate::widgets::ai_chat_chip_row::CHIP_ROW_HEIGHT
        } else {
            0.0
        }
    }

    /// Where the live chips sit. The single rect source paint and hit-test
    /// both read — see `ai_chat_chip_row`.
    pub(crate) fn chip_row(&self, input_rect: Rect) -> ChipRowLayout {
        let style_w = self
            .style_receipt
            .as_ref()
            .zip(self.style_receipt_label())
            .map(|(receipt, label)| {
                crate::widgets::ai_chat_style_receipt::chip_width(
                    &label,
                    receipt.swatches.len(),
                    receipt.clearable,
                )
            });
        let selection_w = (self.selected_count > 0).then(|| {
            crate::widgets::ai_chat_chip_row::label_chip_width(&self.selection_chip_label())
        });
        crate::widgets::ai_chat_chip_row::chip_row_layout(style_w, selection_w, input_rect)
    }

    /// The receipt's label, already formatted for the active locale.
    pub(crate) fn style_receipt_label(&self) -> Option<String> {
        let receipt = self.style_receipt.as_ref()?;
        Some(op_i18n::translate(self.locale, "ai.pinnedStyle").replace("{{name}}", &receipt.name))
    }

    /// Where the pinned-style chip sits, or `None` when no style is in force.
    /// The same rect paint and hit-test read — see `ai_chat_chip_row`.
    pub fn style_chip_rect(&self, rect: Rect) -> Option<Rect> {
        self.chip_row(self.input_rect(rect)).style
    }

    /// Whether the chip's detail card is on screen at this panel's `now_ms`.
    /// Hosts read this to keep a dwell wake alive past the instant it retires.
    pub fn style_card_showing(&self) -> bool {
        self.style_card.is_some()
    }

    /// Whether the cursor rests on the pinned-style chip — the hover the
    /// detail card's dwell clock is started and stopped by.
    ///
    /// The whole chip counts, ✕ included: the card hangs above the row, so it
    /// never covers the clear button it would be competing with, and a card
    /// that blinked out as the cursor crossed onto the ✕ would read as a bug.
    /// An open model picker suppresses it for the same reason the footer's
    /// hover does — that dropdown is painted over this row.
    pub fn style_chip_hover_at(&self, rect: Rect, point: Point2D) -> bool {
        if self.state.is_minimized() || self.model_picker.open || self.style_receipt.is_none() {
            return false;
        }
        self.chip_row(self.input_rect(rect))
            .style
            .is_some_and(|chip| chip.contains(point))
    }

    pub(crate) fn style_receipt_clear_rect(&self, input_rect: Rect) -> Option<Rect> {
        let receipt = self.style_receipt.as_ref()?;
        let chip = self.chip_row(input_rect).style?;
        crate::widgets::ai_chat_style_receipt::clear_rect(receipt, chip)
    }

    pub(crate) fn selection_chip_label(&self) -> String {
        self.selected_label.clone().unwrap_or_else(|| {
            op_i18n::translate(self.locale, "common.selected")
                .replace("{{count}}", &self.selected_count.to_string())
        })
    }

    pub(crate) fn selection_chip_rect(&self, input_rect: Rect) -> Option<Rect> {
        self.chip_row(input_rect).selection
    }

    pub(crate) fn selection_chip_clear_rect(&self, input_rect: Rect) -> Option<Rect> {
        self.selection_chip_rect(input_rect)
            .map(crate::widgets::ai_chat_chip_row::chip_clear_rect)
    }

    /// Total input-block height, including the attachment row when
    /// attachments are staged.
    #[cfg(test)]
    pub(crate) fn input_height(&self) -> f32 {
        self.input_height_for_width(self.state.panel_width, self.state.panel_height)
    }

    /// Text-area height for an input `input_w` wide inside a panel `panel_h`
    /// tall. The panel height is what caps the growth — see
    /// [`max_input_lines`].
    ///
    /// [`max_input_lines`]: crate::widgets::ai_chat_input_text::max_input_lines
    pub(crate) fn input_area_height_for_input_width(&self, input_w: f32, panel_h: f32) -> f32 {
        let lines = crate::widgets::ai_chat_input_text::visible_input_line_count(
            self.state.input.text(),
            input_w,
            panel_h,
        );
        crate::widgets::ai_chat_input_text::input_area_height(lines)
    }

    pub(crate) fn input_area_height_for_width(&self, panel_w: f32, panel_h: f32) -> f32 {
        let input_w = (panel_w - PAD * 2.0).max(0.0);
        self.input_area_height_for_input_width(input_w, panel_h)
    }

    pub(crate) fn input_height_for_width(&self, panel_w: f32, panel_h: f32) -> f32 {
        self.chip_row_h()
            + self.input_area_height_for_width(panel_w, panel_h)
            + INPUT_TOOLBAR_HEIGHT
            + self.attachment_row_h()
    }

    pub(crate) fn input_area_height_for_rect(&self, rect: Rect) -> f32 {
        self.input_area_height_for_width(rect.size.x, rect.size.y)
    }

    pub(crate) fn input_height_for_rect(&self, rect: Rect) -> f32 {
        self.input_height_for_width(rect.size.x, rect.size.y)
    }

    fn maximize_icon(&self) -> Icon {
        [Icon::Maximize, Icon::Minimize][self.state.maximized as usize]
    }

    pub(crate) fn is_streaming(&self) -> bool {
        self.state.messages.iter().any(|message| message.streaming)
    }

    pub fn body_rect(&self, rect: Rect) -> Rect {
        let body_top = rect.origin.y + HEADER_HEIGHT + 14.0; // gap before first bubble
        let body_bottom =
            rect.origin.y + rect.size.y - self.input_height_for_rect(rect) - PAD - 8.0;
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
        // Resolve owner-scoped so a rebuild here tags the slot with this panel's
        // owner (rather than clobbering it to UNOWNED), keeping the native
        // cursor-shape hint matching between wheel events and paints.
        (crate::widgets::ai_chat_transcript_cache::cached_canonical_transcript_owned(
            self.owner,
            &self.state.messages,
            body,
            self.locale,
        )
        .total_height
            - body.size.y)
            .max(0.0)
    }

    pub(crate) fn model_picker_rect(&self, rect: Rect, input_rect: Rect) -> Rect {
        let height = crate::widgets::ai_chat_model_picker::picker_view_height(
            &self.state.available_models,
            self.model_picker_input.text(),
        );
        let toolbar_top = input_rect.origin.y
            + self.chip_row_h()
            + self.input_area_height_for_rect(rect)
            + self.attachment_row_h();
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
        if self.state.is_minimized() || !self.model_picker.open {
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

    pub fn input_text_rect(&self, rect: Rect) -> Rect {
        let input_block = self.input_rect(rect);
        Rect {
            origin: Point2D::new(
                input_block.origin.x,
                input_block.origin.y + self.chip_row_h(),
            ),
            size: Point2D::new(input_block.size.x, self.input_area_height_for_rect(rect)),
        }
    }

    pub fn input_caret_rect(&self, rect: Rect) -> Rect {
        let input_text = self.input_text_rect(rect);
        crate::widgets::ai_chat_input_text::input_caret_rect(
            self.state,
            input_text,
            input_text.size.y,
        )
    }

    /// Resolved `(current, max)` scroll offset of the draft input for the
    /// panel laid out at `rect`. `max == 0.0` while the text still fits.
    ///
    /// `current` is what is on screen right now, which is not always the
    /// stored offset — the caret overrides it after any caret motion — so a
    /// wheel must apply its delta to this, not to `chat.input_scroll`.
    pub fn input_scroll_state(&self, rect: Rect) -> (f32, f32) {
        let input_text = self.input_text_rect(rect);
        let view = crate::widgets::ai_chat_input_text::measured_input_text_view(
            self.state,
            input_text,
            input_text.size.y,
        );
        (view.scroll, view.max_scroll)
    }

    /// Byte offset the caret moves to when it steps one visual line
    /// up / down inside the draft input.
    pub fn input_vertical_caret_offset(&self, rect: Rect, down: bool) -> Option<usize> {
        let input_text = self.input_text_rect(rect);
        crate::widgets::ai_chat_input_text::vertical_caret_offset(
            self.state,
            input_text,
            input_text.size.y,
            down,
        )
    }
}

fn selection_chip_label_for_state(state: &EditorState) -> Option<String> {
    match state.selection_count() {
        0 => None,
        1 => {
            let id = state.selection.set.first()?;
            if !id.is_real() {
                return None;
            }
            let node = op_editor_core::walkers::find_node(state.active_children(), id);
            Some(
                node.and_then(|node| {
                    node.base()
                        .name
                        .as_deref()
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .map(str::to_string)
                })
                .unwrap_or_else(|| id.as_str().to_string()),
            )
        }
        count => Some(
            op_i18n::translate(state.editor_ui.effective_locale(), "common.selected")
                .replace("{{count}}", &count.to_string()),
        ),
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FooterLayout {
    /// Model picker pill — left anchor of the toolbar.
    pub(crate) model: Rect,
    /// Prompt-library button — opens the Prompt Center.
    pub(crate) prompt_center: Rect,
    /// Thinking-mode toggle — 🧠 icon, cycles Adaptive/Disabled/Enabled.
    /// Zero-width when the row is too narrow to hold it (see
    /// `footer_layout`); `contains()` is then always false.
    pub(crate) thinking: Rect,
    /// Speed/effort chip — ⚡ icon + effort label in gold, no bg.
    /// Retained next to the model pill; clicking cycles effort level.
    pub(crate) speed: Rect,
    /// Agent-team size chip — kept for backward compat / future use.
    pub(crate) agent_team: Rect,
    /// Paperclip attach button — bare icon, muted.
    pub(crate) attach: Rect,
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

#[path = "ai_chat_panel/widget_paint.rs"]
mod widget_paint;

#[cfg(test)]
#[path = "ai_chat_panel/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "ai_chat_panel/tests_paint.rs"]
mod tests_paint;

#[cfg(test)]
#[path = "ai_chat_panel/tests_minimized_paint.rs"]
mod tests_minimized_paint;

#[cfg(test)]
#[path = "ai_chat_panel/tests_transcript.rs"]
mod tests_transcript;

#[cfg(test)]
#[path = "ai_chat_panel/tests_selected_chip.rs"]
mod tests_selected_chip;
