//! `AIChatPlaceholder` — floating "start designing with AI" / chat panel
//! pinned to the bottom-center of the canvas (Step 4 visual lift +
//! Step 5 P2 dynamic state).
//!
//! Two render modes driven by `ChatState`:
//!  - **Empty** (no messages): the original "start designing with AI" hint
//!    + 2×2 example cards — clicking a card fills the input.
//!  - **Active** (≥1 message): renders the message list above the
//!    input, hides the example grid.
//!
//! Hit-test exposes [`AIChatHit`] so the host can route a click to
//! Send / focus input / pick example. Full keyboard plumbing lives
//! on the host (`apply_text` / `apply_send`).

use crate::theme::Theme;
use crate::widgets::ai_chat_panel_controls::{
    attachment_row_hit, controls_row_hit, paint_attachment_row, paint_controls_row,
    ATTACHMENT_ROW_HEIGHT, CONTROLS_ROW_HEIGHT,
};
use crate::widgets::ai_chat_panel_paint::paint_examples;
use crate::widgets::editor_state_ext::{theme_for, translate};
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};
use op_editor_core::chat::ChatState;
use op_editor_core::EditorState;

pub const AI_CHAT_WIDTH: f32 = 380.0;
pub const AI_CHAT_HEIGHT: f32 = 460.0;
/// Pill width when [`ChatState::collapsed`] is true — sized to
/// fit "[bubble] New Chat [chevron-up]" with comfortable padding,
/// matching the TS app reference screenshot.
pub const AI_CHAT_COLLAPSED_WIDTH: f32 = 150.0;
/// Height of the panel when collapsed — short pill, just enough
/// for the row to read.
pub const AI_CHAT_COLLAPSED_HEIGHT: f32 = 36.0;
pub(crate) const PAD: f32 = 16.0;
pub(crate) const HEADER_HEIGHT: f32 = 36.0;
/// Tall textarea-style input region (placeholder / typed buffer).
const INPUT_AREA_HEIGHT: f32 = 56.0;
/// Toolbar below the textarea — model picker on left, attach +
/// send on right. Mirrors the TS panel's bottom row.
const INPUT_TOOLBAR_HEIGHT: f32 = 40.0;
/// Click-width of the bottom-toolbar model chip (sparkles + agent
/// name + chevron). Fixed so hit-test needs no text measurement.
const MODEL_CHIP_W: f32 = 150.0;
/// Reserved height of the input block when no attachment is staged:
/// textarea + per-turn controls strip + bottom toolbar. The block
/// grows by [`ATTACHMENT_ROW_HEIGHT`] when attachments are staged —
/// see [`AIChatPlaceholder::input_height`].
const INPUT_BASE_HEIGHT: f32 = INPUT_AREA_HEIGHT + CONTROLS_ROW_HEIGHT + INPUT_TOOLBAR_HEIGHT;

#[derive(Debug, Clone)]
pub(crate) struct ExampleCard {
    pub(crate) title: &'static str,
    pub(crate) subtitle: &'static str,
    pub(crate) emoji: &'static str,
}

pub(crate) const EXAMPLES: [ExampleCard; 4] = [
    ExampleCard {
        title: "设计一个移动端登录页面",
        subtitle: "带社交登录的移动端页面",
        emoji: "📱",
    },
    ExampleCard {
        title: "美食 App 首页",
        subtitle: "App 首页设计",
        emoji: "🍕",
    },
    ExampleCard {
        title: "设计一个底部导航栏",
        subtitle: "5 个 Tab 导航栏",
        emoji: "⬇️",
    },
    ExampleCard {
        title: "为我的应用推荐配色方案",
        subtitle: "应用配色推荐",
        emoji: "🎨",
    },
];

/// What a click inside the panel resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AIChatHit {
    /// Click landed on the input area — host should focus chat.
    FocusInput,
    /// Click landed on the send affordance.
    Send,
    /// Click landed on an example card; payload is the example's
    /// title (host fills the input with this).
    Example(String),
    /// Click landed on the header / margin — host should start a
    /// drag so the user can move the panel between canvas corners.
    DragHandle,
    /// Click on the chevron at the top-left of the header — host
    /// flips the `ChatState::collapsed` flag.
    ToggleCollapse,
    /// Click on the model chip (bottom-left of the input toolbar) —
    /// host toggles `ui.chat_model_picker_open` to open / close the
    /// model dropdown.
    ToggleModelPicker,
    /// Click on a model row in the open picker dropdown — payload
    /// is the index into `chat.available_models`
    /// (`Document::select_chat_model`).
    SelectModel(usize),
    /// Click on the thinking-mode chip — host cycles
    /// `ChatState::thinking_mode`.
    CycleThinking,
    /// Click on the effort chip — host cycles
    /// `ChatState::effort_level`.
    CycleEffort,
    /// Click on the attach button — host opens a file picker and
    /// stages the chosen file via `ChatState::add_attachment`.
    AddAttachment,
    /// Click on a staged-attachment chip — payload is the index
    /// into `chat.pending_attachments` to drop.
    RemoveAttachment(usize),
    /// Click on a message's thinking-block header — host toggles
    /// `ChatMessage::thinking_collapsed` for that message index.
    ToggleThinking(usize),
    /// Click on a message's tool-calls panel header — host toggles
    /// `ChatMessage::tools_collapsed` for that message index.
    ToggleToolCalls(usize),
}

pub struct AIChatPlaceholder<'a> {
    pub id: WidgetId,
    pub theme: Theme,
    pub state: &'a ChatState,
    /// Host-supplied frame timestamp (milliseconds since the host
    /// started). Drives caret blink via
    /// [`jian_core::anim::blink_visible`]. `0` = host hasn't
    /// installed a clock yet (caret stays solid).
    pub now_ms: u64,
    /// Localised chrome strings — resolved at construction time
    /// from `Document::t` so the panel reflows when the user
    /// flips the TopBar Globe icon.
    pub label_new_chat: String,
    pub label_start_with_ai: String,
    pub label_input_placeholder: String,
    /// Tip line ("select canvas elements before chatting to provide
    /// context") — bottom of the empty-state body, between the example
    /// cards and the separator above the input.
    pub label_tip_select_elements: String,
    /// Chip label shown when no model is selected / discovered yet
    /// (`ai.noModelsConnected`).
    pub label_no_models: String,
    /// Whether the model-picker dropdown is open
    /// (`Document.ui.chat_model_picker_open`). The picker lists
    /// `state.available_models`; the active row is
    /// `state.selected_model`.
    pub model_picker_open: bool,
}

impl<'a> AIChatPlaceholder<'a> {
    pub fn from_editor(state: &'a EditorState) -> Self {
        Self::from_editor_at(state, 0)
    }

    /// Same as `from_editor` but threads through the host's
    /// current millisecond timestamp so the caret can blink.
    pub fn from_editor_at(state: &'a EditorState, now_ms: u64) -> Self {
        let ui = &state.editor_ui;
        Self {
            id: WidgetId::new(7000),
            theme: theme_for(ui),
            state: &state.chat,
            now_ms,
            label_new_chat: translate(ui, "ai.newChat").to_string(),
            label_start_with_ai: translate(ui, "ai.tryExample").to_string(),
            label_input_placeholder: translate(ui, "ai.designWithAgent").to_string(),
            label_tip_select_elements: translate(ui, "ai.tipSelectElements").to_string(),
            label_no_models: translate(ui, "ai.noModelsConnected").to_string(),
            model_picker_open: ui.chat_model_picker_open,
        }
    }

    /// Height of the staged-attachment row — `0` when none is staged.
    fn attachment_row_h(&self) -> f32 {
        if self.state.pending_attachments.is_empty() {
            0.0
        } else {
            ATTACHMENT_ROW_HEIGHT
        }
    }

    /// Total input-block height, including the attachment row when
    /// attachments are staged.
    fn input_height(&self) -> f32 {
        INPUT_BASE_HEIGHT + self.attachment_row_h()
    }

    /// Bounds of the transcript body region — between the header and
    /// the input block. Shared by `paint` and `hit_test` so the
    /// message layout (and its collapsible-header hit targets) line
    /// up exactly.
    fn body_rect(&self, rect: Rect) -> Rect {
        let body_top = rect.origin.y + HEADER_HEIGHT;
        let body_bottom = rect.origin.y + rect.size.y - self.input_height() - PAD - 8.0;
        Rect {
            origin: Point2D::new(rect.origin.x + PAD, body_top),
            size: Point2D::new(rect.size.x - PAD * 2.0, (body_bottom - body_top).max(0.0)),
        }
    }

    /// Bounds of the model-picker dropdown — anchored just above
    /// the bottom toolbar (the chip), growing upward over the
    /// message list. `input_rect` is the panel's input box.
    fn model_picker_rect(&self, rect: Rect, input_rect: Rect) -> Rect {
        let height = crate::widgets::ai_chat_model_picker::picker_content_height(
            &self.state.available_models,
        );
        let toolbar_top =
            input_rect.origin.y + INPUT_AREA_HEIGHT + self.attachment_row_h() + CONTROLS_ROW_HEIGHT;
        let bottom = toolbar_top - 4.0;
        Rect {
            origin: Point2D::new(rect.origin.x + PAD, bottom - height),
            size: Point2D::new(rect.size.x - PAD * 2.0, height),
        }
    }

    pub fn hit_test(&self, rect: Rect, point: Point2D) -> Option<AIChatHit> {
        if !rect_contains(rect, point) {
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
        // Expanded: chevron-down at top-left toggles collapse.
        let chevron_rect = Rect {
            origin: rect.origin,
            size: Point2D::new(36.0, 32.0),
        };
        if rect_contains(chevron_rect, point) {
            return Some(AIChatHit::ToggleCollapse);
        }
        let input_h = self.input_height();
        // Must match `paint` exactly: paint draws the separator at
        // `bottom - input_h` and the input block one pixel below it
        // (`sep_y + 1`). An earlier `- PAD` here put the hit targets
        // ~17 px above where they are painted.
        let input_rect = Rect {
            origin: Point2D::new(
                rect.origin.x + PAD,
                rect.origin.y + rect.size.y - input_h + 1.0,
            ),
            size: Point2D::new(rect.size.x - PAD * 2.0, input_h),
        };
        // Model-picker dropdown — an overlay above the chip. When
        // open it behaves modally: a row click selects, any other
        // click dismisses it. Hit-tested before the input so a row
        // click isn't eaten by the message list beneath.
        if self.model_picker_open {
            let picker = self.model_picker_rect(rect, input_rect);
            if let Some(idx) = crate::widgets::ai_chat_model_picker::model_at(
                picker,
                point,
                &self.state.available_models,
            ) {
                return Some(AIChatHit::SelectModel(idx));
            }
            return Some(AIChatHit::ToggleModelPicker);
        }
        if rect_contains(input_rect, point) {
            let attach_top = input_rect.origin.y + INPUT_AREA_HEIGHT;
            let attach_h = self.attachment_row_h();
            let controls_top = attach_top + attach_h;
            let toolbar_top = controls_top + CONTROLS_ROW_HEIGHT;
            // Staged-attachment strip — present only when attachments
            // are staged; a chip click removes that attachment.
            if attach_h > 0.0 && point.y >= attach_top && point.y < controls_top {
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
            // Per-turn controls strip — thinking / effort / attach.
            // Hit-tested before the toolbar so its chips aren't eaten
            // by the model-chip / send band.
            if point.y >= controls_top && point.y < toolbar_top {
                let controls_rect = Rect {
                    origin: Point2D::new(input_rect.origin.x, controls_top),
                    size: Point2D::new(input_rect.size.x, CONTROLS_ROW_HEIGHT),
                };
                if let Some(hit) = controls_row_hit(controls_rect, point) {
                    return Some(hit);
                }
                return Some(AIChatHit::FocusInput);
            }
            // Bottom toolbar strip — its left `MODEL_CHIP_W` is the
            // model chip (opens / closes the picker), its rightmost
            // ~40px is the send chip.
            if point.y >= toolbar_top {
                if point.x <= input_rect.origin.x + MODEL_CHIP_W {
                    return Some(AIChatHit::ToggleModelPicker);
                }
                let send_x = input_rect.origin.x + input_rect.size.x - 40.0;
                if point.x >= send_x {
                    return Some(AIChatHit::Send);
                }
            }
            return Some(AIChatHit::FocusInput);
        }
        // Transcript hit-test — a click on a message's thinking /
        // tool-call collapsible header toggles it. Checked before the
        // drag-handle fallback so the headers are interactive.
        if !self.state.messages.is_empty() {
            if let Some(hit) = crate::widgets::ai_chat_transcript::transcript_hit(
                &self.state.messages,
                self.body_rect(rect),
                point.x,
                point.y,
            ) {
                return Some(match hit {
                    crate::widgets::ai_chat_transcript::TranscriptHit::ToggleThinking(i) => {
                        AIChatHit::ToggleThinking(i)
                    }
                    crate::widgets::ai_chat_transcript::TranscriptHit::ToggleToolCalls(i) => {
                        AIChatHit::ToggleToolCalls(i)
                    }
                });
            }
        }
        if self.state.messages.is_empty() {
            // Examples grid hit-test (only rendered when no messages).
            let card_w = (rect.size.x - PAD * 2.0 - 8.0) / 2.0;
            let card_h = 70.0;
            let grid_y = rect.origin.y + HEADER_HEIGHT + 32.0;
            for (i, ex) in EXAMPLES.iter().enumerate() {
                let col = (i % 2) as f32;
                let row = (i / 2) as f32;
                let card = Rect {
                    origin: Point2D::new(
                        rect.origin.x + PAD + col * (card_w + 8.0),
                        grid_y + row * (card_h + 8.0),
                    ),
                    size: Point2D::new(card_w, card_h),
                };
                if rect_contains(card, point) {
                    return Some(AIChatHit::Example(ex.title.to_string()));
                }
            }
        }
        // Anywhere else in the panel = drag handle. Header strip,
        // margins around the examples grid, gaps between cards —
        // host starts a drag-to-move gesture.
        Some(AIChatHit::DragHandle)
    }
}

fn rect_contains(r: Rect, p: Point2D) -> bool {
    p.x >= r.origin.x
        && p.x <= r.origin.x + r.size.x
        && p.y >= r.origin.y
        && p.y <= r.origin.y + r.size.y
}

impl<'a> Widget for AIChatPlaceholder<'a> {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _cx: &LayoutCx) -> LayoutBox {
        LayoutBox {
            rect: Rect {
                origin: Point2D::new(0.0, 0.0),
                size: Point2D::new(AI_CHAT_WIDTH, AI_CHAT_HEIGHT),
            },
        }
    }

    fn paint(&self, cx: &mut PaintCx<'_>, rect: Rect) {
        // Collapsed mode = compact pill: bubble icon + "New Chat"
        // + chevron-up. No maximize/plus, narrow rounded radius.
        if self.state.collapsed {
            let radius = rect.size.y / 2.0;
            cx.backend.fill_round_rect(rect, radius, self.theme.popover);
            cx.backend
                .stroke_round_rect(rect, radius, self.theme.border, 1.0);
            let center_y = rect.origin.y + rect.size.y / 2.0;
            let icon_size = 16.0;
            // Bubble at the left.
            draw_icon(
                cx.backend,
                Icon::MessageSquare,
                Point2D::new(rect.origin.x + 12.0, center_y - icon_size / 2.0),
                icon_size,
                self.theme.muted_foreground,
                1.4,
            );
            // "New Chat" label.
            let title = TextLayout::single_run(
                &self.label_new_chat,
                "system-ui",
                13.0,
                to_jian_color(self.theme.foreground),
                Point2D::new(0.0, 0.0),
            );
            cx.backend.draw_text(
                &title,
                Point2D::new(rect.origin.x + 12.0 + icon_size + 8.0, center_y + 5.0),
            );
            // Chevron-up at the right (click to expand).
            draw_icon(
                cx.backend,
                Icon::ChevronUp,
                Point2D::new(
                    rect.origin.x + rect.size.x - 12.0 - icon_size,
                    center_y - icon_size / 2.0,
                ),
                icon_size,
                self.theme.muted_foreground,
                1.4,
            );
            return;
        }

        cx.backend.fill_round_rect(rect, 14.0, self.theme.popover);
        cx.backend
            .stroke_round_rect(rect, 14.0, self.theme.border, 1.0);

        // Expanded header.
        let header_y = rect.origin.y + 8.0;
        draw_icon(
            cx.backend,
            Icon::ChevronDown,
            Point2D::new(rect.origin.x + PAD, header_y),
            18.0,
            self.theme.muted_foreground,
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
        draw_icon(
            cx.backend,
            Icon::Maximize,
            Point2D::new(rect.origin.x + rect.size.x - PAD - 50.0, header_y),
            18.0,
            self.theme.muted_foreground,
            1.4,
        );
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(rect.origin.x + rect.size.x - PAD - 22.0, header_y),
            18.0,
            self.theme.muted_foreground,
            1.4,
        );

        // Body — either messages or examples.
        let input_h = self.input_height();
        if self.state.messages.is_empty() {
            paint_examples(cx, &self.theme, rect, &self.label_start_with_ai);
        } else {
            crate::widgets::ai_chat_transcript::paint_transcript(
                cx,
                &self.theme,
                self.body_rect(rect),
                &self.state.messages,
                self.now_ms,
            );
        }

        // Separator hairline between body and input area
        // (matches the TS panel's bottom-bordered body region).
        let sep_y = rect.origin.y + rect.size.y - input_h;
        cx.backend.fill_rect(
            Rect {
                origin: Point2D::new(rect.origin.x + PAD, sep_y),
                size: Point2D::new(rect.size.x - PAD * 2.0, 1.0),
            },
            self.theme.border,
        );

        // Textarea region — borderless, single line of placeholder /
        // typed text, 14 px to mirror the TS app's textarea style.
        let input_rect = Rect {
            origin: Point2D::new(rect.origin.x + PAD, sep_y + 1.0),
            size: Point2D::new(rect.size.x - PAD * 2.0, INPUT_AREA_HEIGHT),
        };
        let (text, color) = if self.state.input.is_empty() {
            (
                self.label_input_placeholder.as_str(),
                self.theme.muted_foreground,
            )
        } else {
            (self.state.input.as_str(), self.theme.foreground)
        };
        let input_label = TextLayout::single_run(
            text,
            "system-ui",
            14.0,
            to_jian_color(color),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &input_label,
            Point2D::new(input_rect.origin.x, input_rect.origin.y + 22.0),
        );
        let caret_visible = self.state.focused
            && jian_core::anim::blink_visible(self.now_ms, self.state.caret_anchor_ms, 500);
        if caret_visible {
            let text_w = cx.backend.measure_text(&self.state.input, 14.0);
            let caret_x = input_rect.origin.x + text_w;
            cx.backend.fill_rect(
                Rect {
                    origin: Point2D::new(caret_x, input_rect.origin.y + 8.0),
                    size: Point2D::new(1.5, 18.0),
                },
                self.theme.foreground,
            );
        }

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

        // Per-turn controls strip — thinking / effort / attach.
        let controls_rect = Rect {
            origin: Point2D::new(
                input_rect.origin.x,
                input_rect.origin.y + INPUT_AREA_HEIGHT + attach_h,
            ),
            size: Point2D::new(input_rect.size.x, CONTROLS_ROW_HEIGHT),
        };
        paint_controls_row(cx, &self.theme, controls_rect, self.state);

        // Bottom toolbar — model picker on the left, send on the
        // right (mirrors the TS panel's bottom row).
        let toolbar_y = input_rect.origin.y + INPUT_AREA_HEIGHT + attach_h + CONTROLS_ROW_HEIGHT;
        let toolbar_center_y = toolbar_y + INPUT_TOOLBAR_HEIGHT / 2.0;
        // Model chip — brand logo of the selected model's provider
        // + its display name + a chevron. Click toggles the picker.
        let mut model_x = rect.origin.x + PAD;
        let selected = self.state.selected_model_entry();
        let chip_color = self.theme.muted_foreground;
        match selected {
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

        // Right cluster — send button. (Attach moved to the
        // controls strip above.)
        let rx = rect.origin.x + rect.size.x - PAD;
        let send_size = 24.0;
        let send_rect = Rect {
            origin: Point2D::new(rx - send_size, toolbar_center_y - send_size / 2.0),
            size: Point2D::new(send_size, send_size),
        };
        // A turn is sendable with text, with staged attachments, or
        // both (TS parity: an attachment-only message is valid).
        let send_active =
            !self.state.input.trim().is_empty() || !self.state.pending_attachments.is_empty();
        let (send_bg, icon_color) = if send_active {
            (self.theme.primary, self.theme.primary_foreground)
        } else {
            (self.theme.muted, self.theme.muted_foreground)
        };
        cx.backend.fill_round_rect(send_rect, 6.0, send_bg);
        // Lucide "send" arrow drawn as 3 short strokes.
        cx.backend.stroke_line(
            Point2D::new(send_rect.origin.x + 7.0, send_rect.origin.y + 7.0),
            Point2D::new(send_rect.origin.x + 17.0, send_rect.origin.y + 12.0),
            icon_color,
            1.6,
        );
        cx.backend.stroke_line(
            Point2D::new(send_rect.origin.x + 17.0, send_rect.origin.y + 12.0),
            Point2D::new(send_rect.origin.x + 7.0, send_rect.origin.y + 17.0),
            icon_color,
            1.6,
        );
        cx.backend.stroke_line(
            Point2D::new(send_rect.origin.x + 7.0, send_rect.origin.y + 7.0),
            Point2D::new(send_rect.origin.x + 7.0, send_rect.origin.y + 17.0),
            icon_color,
            1.6,
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
            );
        }
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label("AI chat");
        node
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
