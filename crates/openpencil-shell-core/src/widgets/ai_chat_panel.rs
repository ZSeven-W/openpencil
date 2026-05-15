//! `AIChatPlaceholder` — floating "用 AI 开始设计" / chat panel
//! pinned to the bottom-center of the canvas (Step 4 visual lift +
//! Step 5 P2 dynamic state).
//!
//! Two render modes driven by `ChatState`:
//!  - **Empty** (no messages): the original "用 AI 开始设计" hint
//!    + 2×2 example cards — clicking a card fills the input.
//!  - **Active** (≥1 message): renders the message list above the
//!    input, hides the example grid.
//!
//! Hit-test exposes [`AIChatHit`] so the host can route a click to
//! Send / focus input / pick example. Full keyboard plumbing lives
//! on the host (`apply_text` / `apply_send`).

use crate::document::{ChatRole, ChatState, Document};
use crate::theme::Theme;
use crate::widgets::icons::{draw_icon, Icon};
use crate::widgets::{LayoutBox, LayoutCx, PaintCx, Widget, WidgetId};
use crate::{Color, Point2D, Rect, TextLayout};

pub const AI_CHAT_WIDTH: f32 = 380.0;
pub const AI_CHAT_HEIGHT: f32 = 460.0;
/// Pill width when [`ChatState::collapsed`] is true — sized to
/// fit "[bubble] New Chat [chevron-up]" with comfortable padding,
/// matching the TS app reference screenshot.
pub const AI_CHAT_COLLAPSED_WIDTH: f32 = 150.0;
/// Height of the panel when collapsed — short pill, just enough
/// for the row to read.
pub const AI_CHAT_COLLAPSED_HEIGHT: f32 = 36.0;
const PAD: f32 = 16.0;
const HEADER_HEIGHT: f32 = 36.0;
/// Tall textarea-style input region (placeholder / typed buffer).
const INPUT_AREA_HEIGHT: f32 = 56.0;
/// Toolbar below the textarea — model picker on left, attach +
/// send on right. Mirrors the TS panel's bottom row.
const INPUT_TOOLBAR_HEIGHT: f32 = 40.0;
/// Click-width of the bottom-toolbar model chip (sparkles + agent
/// name + chevron). Fixed so hit-test needs no text measurement.
const MODEL_CHIP_W: f32 = 150.0;
/// Total reserved space for the input + toolbar block.
const INPUT_HEIGHT: f32 = INPUT_AREA_HEIGHT + INPUT_TOOLBAR_HEIGHT;

#[derive(Debug, Clone)]
struct ExampleCard {
    title: &'static str,
    subtitle: &'static str,
    emoji: &'static str,
}

const EXAMPLES: [ExampleCard; 4] = [
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
    /// host advances `chat_selected_agent` to the next connected
    /// CLI agent (`Document::cycle_chat_agent`).
    CycleModel,
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
    /// "提示：在对话前选中画布上的元素以提供上下文。" — bottom of
    /// the empty-state body, between the example cards and the
    /// separator above the input.
    pub label_tip_select_elements: String,
    /// Name of the AI-chat agent shown in the bottom toolbar's
    /// model chip — the connected CLI selected via `chat_selected_agent`
    /// (`AgentProvider::label`). Falls back to "Default" only when
    /// the stored index is somehow out of range.
    pub model_label: String,
}

impl<'a> AIChatPlaceholder<'a> {
    pub fn from_document(doc: &'a Document) -> Self {
        Self::from_document_at(doc, 0)
    }

    /// Same as `from_document` but threads through the host's
    /// current millisecond timestamp so the caret can blink.
    pub fn from_document_at(doc: &'a Document, now_ms: u64) -> Self {
        Self {
            id: WidgetId::new(7000),
            theme: doc.theme(),
            state: &doc.chat,
            now_ms,
            label_new_chat: doc.t("ai.newChat").to_string(),
            label_start_with_ai: doc.t("ai.tryExample").to_string(),
            label_input_placeholder: doc.t("ai.designWithAgent").to_string(),
            label_tip_select_elements: doc.t("ai.tipSelectElements").to_string(),
            model_label: crate::agent_settings_state::AgentProvider::ALL
                .get(doc.ui.chat_selected_agent)
                .map(|a| a.name().to_string())
                .unwrap_or_else(|| "Default".to_string()),
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
        let input_rect = Rect {
            origin: Point2D::new(
                rect.origin.x + PAD,
                rect.origin.y + rect.size.y - INPUT_HEIGHT - PAD,
            ),
            size: Point2D::new(rect.size.x - PAD * 2.0, INPUT_HEIGHT),
        };
        if rect_contains(input_rect, point) {
            // Bottom toolbar strip = the lower `INPUT_TOOLBAR_HEIGHT`
            // of the input box; its left `MODEL_CHIP_W` is the model
            // chip (advances the connected-CLI selection on click).
            let toolbar_top = input_rect.origin.y + INPUT_AREA_HEIGHT;
            if point.y >= toolbar_top
                && point.x <= input_rect.origin.x + MODEL_CHIP_W
            {
                return Some(AIChatHit::CycleModel);
            }
            // Send chip is the rightmost ~40px of the input area.
            let send_x = input_rect.origin.x + input_rect.size.x - 40.0;
            if point.x >= send_x {
                return Some(AIChatHit::Send);
            }
            return Some(AIChatHit::FocusInput);
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
        let body_top = rect.origin.y + HEADER_HEIGHT;
        let body_bottom = rect.origin.y + rect.size.y - INPUT_HEIGHT - PAD - 8.0;
        let body_rect = Rect {
            origin: Point2D::new(rect.origin.x + PAD, body_top),
            size: Point2D::new(rect.size.x - PAD * 2.0, (body_bottom - body_top).max(0.0)),
        };

        if self.state.messages.is_empty() {
            paint_examples(cx, &self.theme, rect, &self.label_start_with_ai);
        } else {
            paint_messages(cx, &self.theme, body_rect, &self.state.messages);
        }

        // Separator hairline between body and input area
        // (matches the TS panel's bottom-bordered body region).
        let sep_y = rect.origin.y + rect.size.y - INPUT_HEIGHT;
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

        // Bottom toolbar — model picker on the left, send + attach
        // on the right (mirrors the TS panel's bottom row).
        let toolbar_y = input_rect.origin.y + INPUT_AREA_HEIGHT;
        let toolbar_center_y = toolbar_y + INPUT_TOOLBAR_HEIGHT / 2.0;
        // Sparkles glyph + "Default" + chevron — model picker.
        let mut model_x = rect.origin.x + PAD;
        draw_icon(
            cx.backend,
            Icon::Sparkles,
            Point2D::new(model_x, toolbar_center_y - 7.0),
            14.0,
            self.theme.muted_foreground,
            1.4,
        );
        model_x += 20.0;
        let model_label = TextLayout::single_run(
            &self.model_label,
            "system-ui",
            12.0,
            to_jian_color(self.theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend
            .draw_text(&model_label, Point2D::new(model_x, toolbar_center_y + 4.0));
        let model_w = cx.backend.measure_text(&self.model_label, 12.0);
        model_x += model_w + 4.0;
        draw_icon(
            cx.backend,
            Icon::ChevronUp,
            Point2D::new(model_x, toolbar_center_y - 5.0),
            10.0,
            self.theme.muted_foreground,
            1.4,
        );

        // Right cluster — send button (and attach plus icon).
        let mut rx = rect.origin.x + rect.size.x - PAD;
        let send_size = 24.0;
        let send_rect = Rect {
            origin: Point2D::new(rx - send_size, toolbar_center_y - send_size / 2.0),
            size: Point2D::new(send_size, send_size),
        };
        let send_active = !self.state.input.trim().is_empty();
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
        rx -= send_size + 8.0;
        // Attach (paperclip — not yet wired). Use Plus glyph as a
        // placeholder; lucide-paperclip would be a follow-up.
        draw_icon(
            cx.backend,
            Icon::Plus,
            Point2D::new(rx - 16.0, toolbar_center_y - 8.0),
            16.0,
            self.theme.muted_foreground,
            1.4,
        );
    }

    fn access_node(&self) -> accesskit::Node {
        let mut node = accesskit::Node::new(accesskit::Role::Group);
        node.set_label("AI chat");
        node
    }
}

fn paint_examples(cx: &mut PaintCx<'_>, theme: &Theme, rect: Rect, hint_label: &str) {
    let hint = TextLayout::single_run(
        hint_label,
        "system-ui",
        12.0,
        to_jian_color(theme.muted_foreground),
        Point2D::new(0.0, 0.0),
    );
    let hint_y = rect.origin.y + HEADER_HEIGHT + 16.0;
    cx.backend.draw_text(
        &hint,
        Point2D::new(rect.origin.x + rect.size.x / 2.0 - 40.0, hint_y),
    );

    let grid_origin_y = hint_y + 16.0;
    let card_w = (rect.size.x - PAD * 2.0 - 8.0) / 2.0;
    let card_h = 70.0;
    for (i, ex) in EXAMPLES.iter().enumerate() {
        let col = (i % 2) as f32;
        let row = (i / 2) as f32;
        let card = Rect {
            origin: Point2D::new(
                rect.origin.x + PAD + col * (card_w + 8.0),
                grid_origin_y + row * (card_h + 8.0),
            ),
            size: Point2D::new(card_w, card_h),
        };
        cx.backend.fill_round_rect(card, 8.0, theme.muted);
        cx.backend.stroke_round_rect(card, 8.0, theme.border, 1.0);
        let emoji_layout = TextLayout::single_run(
            ex.emoji,
            "system-ui",
            14.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &emoji_layout,
            Point2D::new(card.origin.x + 12.0, card.origin.y + 22.0),
        );
        let title_layout = TextLayout::single_run(
            ex.title,
            "system-ui",
            12.0,
            to_jian_color(theme.foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &title_layout,
            Point2D::new(card.origin.x + 36.0, card.origin.y + 22.0),
        );
        let subtitle_layout = TextLayout::single_run(
            ex.subtitle,
            "system-ui",
            11.0,
            to_jian_color(theme.muted_foreground),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &subtitle_layout,
            Point2D::new(card.origin.x + 36.0, card.origin.y + 42.0),
        );
    }
}

fn paint_messages(
    cx: &mut PaintCx<'_>,
    theme: &Theme,
    body_rect: Rect,
    messages: &[crate::document::ChatMessage],
) {
    cx.backend.save();
    cx.backend.clip_rect(body_rect);
    let row_h = 38.0;
    let max_visible = (body_rect.size.y / row_h).floor() as usize;
    let start = messages.len().saturating_sub(max_visible.max(1));
    let mut y = body_rect.origin.y + 4.0;
    for msg in &messages[start..] {
        let bubble_rect = match msg.role {
            ChatRole::User => Rect {
                origin: Point2D::new(body_rect.origin.x + body_rect.size.x * 0.25, y),
                size: Point2D::new(body_rect.size.x * 0.75 - 4.0, row_h - 6.0),
            },
            ChatRole::Assistant => Rect {
                origin: Point2D::new(body_rect.origin.x, y),
                size: Point2D::new(body_rect.size.x * 0.75, row_h - 6.0),
            },
        };
        let bg = match msg.role {
            ChatRole::User => theme.primary,
            ChatRole::Assistant => theme.muted,
        };
        let fg = match msg.role {
            ChatRole::User => theme.primary_foreground,
            ChatRole::Assistant => theme.foreground,
        };
        cx.backend.fill_round_rect(bubble_rect, 8.0, bg);
        let layout = TextLayout::single_run(
            &msg.content,
            "system-ui",
            12.0,
            to_jian_color(fg),
            Point2D::new(0.0, 0.0),
        );
        cx.backend.draw_text(
            &layout,
            Point2D::new(bubble_rect.origin.x + 10.0, bubble_rect.origin.y + 21.0),
        );
        y += row_h;
    }
    cx.backend.restore();
}

fn to_jian_color(c: Color) -> jian_core::scene::Color {
    fn ch(v: f32) -> u8 {
        (v.clamp(0.0, 1.0) * 255.0).round() as u8
    }
    jian_core::scene::Color::rgba(ch(c.r), ch(c.g), ch(c.b), ch(c.a))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_reports_fixed_size() {
        let doc = Document::sample();
        let p = AIChatPlaceholder::from_document(&doc);
        let cx = LayoutCx {
            available_width: 9999.0,
            dpi: 1.0,
        };
        let lb = p.layout(&cx);
        assert_eq!(lb.rect.size.x, AI_CHAT_WIDTH);
        assert_eq!(lb.rect.size.y, AI_CHAT_HEIGHT);
    }

    #[test]
    fn examples_grid_has_four_cards() {
        assert_eq!(EXAMPLES.len(), 4);
    }

    #[test]
    fn hit_test_resolves_input_focus() {
        let doc = Document::sample();
        let panel = AIChatPlaceholder::from_document(&doc);
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        // Click near the input center → FocusInput.
        let p = Point2D::new(60.0, AI_CHAT_HEIGHT - PAD - INPUT_HEIGHT / 2.0);
        assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::FocusInput));
    }

    #[test]
    fn hit_test_resolves_send_at_right() {
        let doc = Document::sample();
        let panel = AIChatPlaceholder::from_document(&doc);
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        let send_x = AI_CHAT_WIDTH - PAD - 20.0;
        let p = Point2D::new(send_x, AI_CHAT_HEIGHT - PAD - INPUT_HEIGHT / 2.0);
        assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::Send));
    }

    #[test]
    fn hit_test_resolves_first_example_when_empty() {
        let doc = Document::sample(); // chat empty by default
        let panel = AIChatPlaceholder::from_document(&doc);
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        // First example card: top-left of grid.
        let card_w = (AI_CHAT_WIDTH - PAD * 2.0 - 8.0) / 2.0;
        let p = Point2D::new(PAD + card_w / 2.0, HEADER_HEIGHT + 32.0 + 35.0);
        match panel.hit_test(rect, p) {
            Some(AIChatHit::Example(s)) => {
                assert_eq!(s, EXAMPLES[0].title);
            }
            other => panic!("expected first example hit, got {:?}", other),
        }
    }

    #[test]
    fn hit_test_header_returns_drag_handle() {
        let doc = Document::sample();
        let panel = AIChatPlaceholder::from_document(&doc);
        let rect = Rect::xywh(0.0, 0.0, AI_CHAT_WIDTH, AI_CHAT_HEIGHT);
        // Click in the empty header band (between title and icons).
        let p = Point2D::new(AI_CHAT_WIDTH / 2.0, 16.0);
        assert_eq!(panel.hit_test(rect, p), Some(AIChatHit::DragHandle));
    }
}
